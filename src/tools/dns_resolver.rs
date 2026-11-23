use futures::FutureExt as _;
use hickory_resolver::config::{NameServerConfig, ResolveHosts, ResolverConfig};
use hickory_resolver::net::runtime::TokioRuntimeProvider;
use hickory_resolver::net::{DnsError, NetError};
use hickory_resolver::proto::op::ResponseCode;
use hickory_resolver::proto::rr::{RData, RecordType};
use hickory_resolver::TokioResolver;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::future::Future;
use std::net::IpAddr;
use std::panic;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DnsRecord {
    pub record_type: String,
    pub name: String,
    pub value: String,
    pub ttl: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsServerLookupResult {
    pub dns_server: String,
    pub records: Vec<DnsRecord>,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsLookupResponse {
    pub domain: String,
    pub results: Vec<DnsServerLookupResult>,
    pub ip_info: Option<IpGeolocationInfo>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DnsServerEntry {
    pub name: String,
    pub server: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IpGeolocationInfo {
    pub ip: String,
    pub country: Option<String>,
    pub city: Option<String>,
    pub org: Option<String>,
    pub region: Option<String>,
    pub timezone: Option<String>,
    pub source: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ReverseDnsResponse {
    pub ip: String,
    pub domains: Vec<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BatchReverseDnsResponse {
    pub results: Vec<ReverseDnsResponse>,
    pub error: Option<String>,
}

#[derive(Deserialize)]
struct IpInfoIoRaw {
    ip: Option<String>,
    country: Option<String>,
    city: Option<String>,
    org: Option<String>,
    region: Option<String>,
    timezone: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DnsRecordType {
    A,
    AAAA,
    MX,
    TXT,
    CNAME,
    NS,
    SOA,
    ALL,
}

impl std::fmt::Display for DnsRecordType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DnsRecordType::A => write!(f, "A"),
            DnsRecordType::AAAA => write!(f, "AAAA"),
            DnsRecordType::MX => write!(f, "MX"),
            DnsRecordType::TXT => write!(f, "TXT"),
            DnsRecordType::CNAME => write!(f, "CNAME"),
            DnsRecordType::NS => write!(f, "NS"),
            DnsRecordType::SOA => write!(f, "SOA"),
            DnsRecordType::ALL => write!(f, "ALL"),
        }
    }
}

impl From<&str> for DnsRecordType {
    fn from(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "A" => DnsRecordType::A,
            "AAAA" => DnsRecordType::AAAA,
            "MX" => DnsRecordType::MX,
            "TXT" => DnsRecordType::TXT,
            "CNAME" => DnsRecordType::CNAME,
            "NS" => DnsRecordType::NS,
            "SOA" => DnsRecordType::SOA,
            "ALL" => DnsRecordType::ALL,
            _ => DnsRecordType::A,
        }
    }
}

async fn query_ip_geolocation(ip: &str) -> Result<IpGeolocationInfo, String> {
    let client = reqwest::Client::new();
    let url = format!("https://ipinfo.io/{}/json", ip);

    match client
        .get(&url)
        .timeout(Duration::from_secs(5))
        .send()
        .await
    {
        Ok(response) => match response.error_for_status() {
            Ok(ok) => match ok.json::<serde_json::Value>().await {
                Ok(val) => {
                    let raw: Option<IpInfoIoRaw> = serde_json::from_value(val.clone()).ok();
                    if let Some(r) = raw {
                        Ok(IpGeolocationInfo {
                            ip: r.ip.unwrap_or_else(|| ip.to_string()),
                            country: r.country,
                            city: r.city,
                            org: r.org,
                            region: r.region,
                            timezone: r.timezone,
                            source: "ipinfo.io".to_string(),
                        })
                    } else {
                        Err("无法解析 ipinfo.io 返回的数据".to_string())
                    }
                }
                Err(e) => Err(format!("解析 ipinfo.io 响应失败: {}", e)),
            },
            Err(e) => Err(format!("ipinfo.io 请求失败: {}", e)),
        },
        Err(e) => Err(format!("请求 ipinfo.io 出错: {}", e)),
    }
}

// ---------------------------------------------------------------------------
// 查询实现：原生 DNS 解析（hickory-resolver，UDP 发送 + TCP 回退）
//
// 直接向所选服务器发查询，不依赖外部 dig 命令——Windows 与多数 Linux
// 发行版默认没有 dig。记录值的展示格式与 dig 保持一致（域名带根点、
// MX 为「优先级 主机」、TXT 每段字符串加引号），各平台输出统一。
// ---------------------------------------------------------------------------

/// 单类查询的等待上限与尝试次数（沿用原 dig 调用的 +time=5 +tries=2）。
const QUERY_TIMEOUT: Duration = Duration::from_secs(5);
const QUERY_ATTEMPTS: usize = 2;

/// 「ALL」逐类查询的记录类型。
const ALL_RECORD_TYPES: [DnsRecordType; 7] = [
    DnsRecordType::A,
    DnsRecordType::AAAA,
    DnsRecordType::MX,
    DnsRecordType::TXT,
    DnsRecordType::CNAME,
    DnsRecordType::NS,
    DnsRecordType::SOA,
];

/// 单服务器查询 future（生产实现为原生解析，测试注入确定性结果）。
type QueryFuture = Pin<Box<dyn Future<Output = Result<Vec<DnsRecord>, String>> + Send>>;

/// 把用户填写的 DNS 服务器解析为 IP：IP 字面量直接使用，主机名经系统
/// 解析器解析。同时有 IPv4/IPv6 时优先 IPv4（多数网络环境 v4 可达性更稳）。
fn resolve_server_ip(server: &str) -> Result<IpAddr, String> {
    let server = server.trim();
    if let Ok(ip) = server.parse::<IpAddr>() {
        return Ok(ip);
    }
    let invalid = || format!("无效的 DNS 服务器地址: {}", server);
    let mut addrs = dns_lookup::lookup_host(server).map_err(|_| invalid())?;
    // 稳定排序保留同族地址的先后，只把 IPv4 提前，不依赖各平台的返回顺序
    addrs.sort_by_key(|ip| ip.is_ipv6());
    addrs.into_iter().next().ok_or_else(invalid)
}

/// 面向单个服务器的解析器。
fn build_resolver(ip: IpAddr) -> Result<TokioResolver, String> {
    let config = ResolverConfig::from_name_servers(vec![NameServerConfig::udp_and_tcp(ip)]);
    let mut builder = TokioResolver::builder_with_config(config, TokioRuntimeProvider::default());
    let options = builder.options_mut();
    options.timeout = QUERY_TIMEOUT;
    options.attempts = QUERY_ATTEMPTS;
    // 关缓存：TTL 展示服务器原值，重复查询也反映实时结果
    options.cache_size = 0;
    // 与 dig 一致：只问指定服务器，不查 hosts 文件
    options.use_hosts_file = ResolveHosts::Never;
    builder
        .build()
        .map_err(|e| format!("DNS 解析器初始化失败: {}", e))
}

/// 常见响应码的 RFC 助记符（与 dig 的 status 一致，排查时更易识别）；
/// 少见响应码用库自带的可读描述。
fn response_code_name(code: ResponseCode) -> &'static str {
    match code {
        ResponseCode::NoError => "NOERROR",
        ResponseCode::FormErr => "FORMERR",
        ResponseCode::ServFail => "SERVFAIL",
        ResponseCode::NXDomain => "NXDOMAIN",
        ResponseCode::NotImp => "NOTIMP",
        ResponseCode::Refused => "REFUSED",
        other => other.to_str(),
    }
}

/// 把解析器错误转成界面可读的失败原因。
fn describe_query_error(error: &NetError, record_type: DnsRecordType) -> String {
    match error {
        NetError::Dns(DnsError::NoRecordsFound(_)) => format!("未找到 {} 记录", record_type),
        NetError::Timeout => "DNS 查询失败: 查询超时".to_string(),
        NetError::Dns(DnsError::ResponseCode(code)) => {
            format!("DNS 查询失败: 服务器返回 {}", response_code_name(*code))
        }
        other => format!("DNS 查询失败: {}", other),
    }
}

fn hickory_record_type(record_type: DnsRecordType) -> Option<RecordType> {
    match record_type {
        DnsRecordType::A => Some(RecordType::A),
        DnsRecordType::AAAA => Some(RecordType::AAAA),
        DnsRecordType::MX => Some(RecordType::MX),
        DnsRecordType::TXT => Some(RecordType::TXT),
        DnsRecordType::CNAME => Some(RecordType::CNAME),
        DnsRecordType::NS => Some(RecordType::NS),
        DnsRecordType::SOA => Some(RecordType::SOA),
        DnsRecordType::ALL => None,
    }
}

/// TXT 单段字符串加引号：转义引号与反斜杠，与 dig 的展示一致。
fn quote_txt_string(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() + 2);
    out.push('"');
    for c in String::from_utf8_lossy(bytes).chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// 记录值格式化为 dig 的展示格式；不支持的类型返回 None。
fn format_rdata(data: &RData) -> Option<String> {
    match data {
        RData::A(a) => Some(a.to_string()),
        RData::AAAA(aaaa) => Some(aaaa.to_string()),
        RData::CNAME(name) => Some(name.to_string()),
        RData::NS(name) => Some(name.to_string()),
        RData::MX(mx) => Some(format!("{} {}", mx.preference, mx.exchange)),
        RData::TXT(txt) => Some(
            txt.txt_data
                .iter()
                .map(|part| quote_txt_string(part))
                .collect::<Vec<_>>()
                .join(" "),
        ),
        RData::SOA(soa) => Some(format!(
            "{} {} {} {} {} {} {}",
            soa.mname, soa.rname, soa.serial, soa.refresh, soa.retry, soa.expire, soa.minimum
        )),
        _ => None,
    }
}

/// 查询单一记录类型；只保留所查类型的应答（CNAME 链不混入结果）。
async fn query_single_type(
    resolver: &TokioResolver,
    domain: &str,
    record_type: DnsRecordType,
) -> Result<Vec<DnsRecord>, String> {
    let rtype = hickory_record_type(record_type)
        .ok_or_else(|| format!("不支持的记录类型: {}", record_type))?;
    let lookup = resolver
        .lookup(domain.to_string(), rtype)
        .await
        .map_err(|e| describe_query_error(&e, record_type))?;

    let records: Vec<DnsRecord> = lookup
        .answers()
        .iter()
        .filter(|record| record.data.record_type() == rtype)
        .filter_map(|record| {
            format_rdata(&record.data).map(|value| DnsRecord {
                record_type: record_type.to_string(),
                name: record.name.to_string(),
                value,
                ttl: Some(record.ttl),
            })
        })
        .collect();

    if records.is_empty() {
        Err(format!("未找到 {} 记录", record_type))
    } else {
        Ok(records)
    }
}

/// 向单个服务器查询；「ALL」逐类查询后合并，单类无记录不算整体失败。
async fn query_server(
    domain: &str,
    server: &str,
    record_type: DnsRecordType,
) -> Result<Vec<DnsRecord>, String> {
    let resolver = build_resolver(resolve_server_ip(server)?)?;
    if record_type == DnsRecordType::ALL {
        let mut all_records = Vec::new();
        for rtype in ALL_RECORD_TYPES {
            if let Ok(mut records) = query_single_type(&resolver, domain, rtype).await {
                all_records.append(&mut records);
            }
        }
        return if all_records.is_empty() {
            Err("未找到任何 DNS 记录".to_string())
        } else {
            Ok(all_records)
        };
    }
    query_single_type(&resolver, domain, record_type).await
}

fn native_query(domain: String, server: String, record_type: DnsRecordType) -> QueryFuture {
    Box::pin(async move { query_server(&domain, &server, record_type).await })
}

async fn geolocate_ip(ip: String) -> Result<IpGeolocationInfo, String> {
    query_ip_geolocation(&ip).await
}

fn default_dns_servers() -> Vec<DnsServerEntry> {
    vec![
        DnsServerEntry {
            name: "阿里云公共DNS".to_string(),
            server: "223.5.5.5".to_string(),
        },
        DnsServerEntry {
            name: "114DNS".to_string(),
            server: "114.114.114.114".to_string(),
        },
        DnsServerEntry {
            name: "谷歌公共DNS".to_string(),
            server: "8.8.8.8".to_string(),
        },
        DnsServerEntry {
            name: "Cloudflare公共DNS".to_string(),
            server: "1.1.1.1".to_string(),
        },
    ]
}

const MAX_DNS_SERVERS: usize = 8;
const FALLBACK_DNS_SERVER: &str = "223.5.5.5";

/// 查询编排核心：参数校验 → 并发查询各服务器（结果按服务器顺序归位）→
/// 取首个成功服务器的 A/AAAA 记录做 IP 归属查询。
///
/// `query` 与 `geolocate` 由调用方注入：生产环境为原生解析 + ipinfo.io，
/// 测试注入确定性结果，不发网络请求。
async fn lookup_dns_with<Q, QF, G, GF>(
    domain: String,
    dns_servers: Option<Vec<String>>,
    record_type: String,
    query: Q,
    geolocate: G,
) -> Result<DnsLookupResponse, String>
where
    Q: Fn(String, String, DnsRecordType) -> QF + Send + Sync + 'static,
    QF: Future<Output = Result<Vec<DnsRecord>, String>> + Send + 'static,
    G: Fn(String) -> GF + Send + 'static,
    GF: Future<Output = Result<IpGeolocationInfo, String>> + Send + 'static,
{
    let rejected = |error: &str| {
        Ok(DnsLookupResponse {
            domain: domain.clone(),
            results: vec![],
            ip_info: None,
            error: Some(error.to_string()),
        })
    };

    if domain.trim().is_empty() {
        // 历史契约：空域名返回的 domain 为空串
        return Ok(DnsLookupResponse {
            domain: String::new(),
            results: vec![],
            ip_info: None,
            error: Some("域名不能为空".to_string()),
        });
    }

    let mut servers: Vec<String> = dns_servers
        .unwrap_or_default()
        .into_iter()
        .map(|server| server.trim().to_string())
        .filter(|server| !server.is_empty())
        .collect();
    let mut seen = HashSet::new();
    servers.retain(|server| seen.insert(server.clone()));

    if servers.len() > MAX_DNS_SERVERS {
        return rejected(&format!("DNS 服务器数量不能超过 {} 个", MAX_DNS_SERVERS));
    }

    if servers.is_empty() {
        servers.push(FALLBACK_DNS_SERVER.to_string());
    }

    // 并发查询所有选中的服务器；先完成的等在槽位里，展示顺序始终等于输入顺序
    let record_type_enum = DnsRecordType::from(record_type.as_str());
    let server_count = servers.len();
    let query = Arc::new(query);
    let mut join_set = tokio::task::JoinSet::new();

    for (idx, server) in servers.into_iter().enumerate() {
        let domain = domain.clone();
        let record_type = record_type_enum;
        let query = Arc::clone(&query);
        join_set.spawn(async move {
            let start = tokio::time::Instant::now();
            // 查询内部 panic 不让该服务器整行静默消失，转成它的失败原因
            let result = panic::AssertUnwindSafe(query(domain, server.clone(), record_type))
                .catch_unwind()
                .await
                .unwrap_or_else(|_| Err("DNS 查询失败: 查询任务异常退出".to_string()));
            let latency_ms = start.elapsed().as_millis() as u64;
            (idx, server, result, latency_ms)
        });
    }

    let mut slots: Vec<Option<DnsServerLookupResult>> = (0..server_count).map(|_| None).collect();
    while let Some(joined) = join_set.join_next().await {
        if let Ok((idx, server, result, latency_ms)) = joined {
            let (records, error) = match result {
                Ok(records) => (records, None),
                Err(e) => (vec![], Some(e)),
            };
            slots[idx] = Some(DnsServerLookupResult {
                dns_server: server,
                records,
                latency_ms: Some(latency_ms),
                error,
            });
        }
    }

    let results: Vec<DnsServerLookupResult> = slots.into_iter().flatten().collect();

    // 取第一个成功服务器的首个 A/AAAA 记录查询 IP 地理位置；归属查询失败不影响结果
    let mut ip_info = None;
    if let Some(ip) = results
        .iter()
        .filter(|r| r.error.is_none())
        .flat_map(|r| r.records.iter())
        .find(|rec| matches!(rec.record_type.as_str(), "A" | "AAAA"))
        .map(|rec| rec.value.clone())
    {
        if let Ok(info) = geolocate(ip).await {
            ip_info = Some(info);
        }
    }

    Ok(DnsLookupResponse {
        domain,
        results,
        ip_info,
        error: None,
    })
}

pub async fn lookup_dns(
    domain: String,
    dns_servers: Option<Vec<String>>,
    record_type: String,
) -> Result<DnsLookupResponse, String> {
    lookup_dns_with(domain, dns_servers, record_type, native_query, geolocate_ip).await
}

pub fn reverse_dns_lookup(ip: String) -> Result<ReverseDnsResponse, String> {
    if ip.trim().is_empty() {
        return Ok(ReverseDnsResponse {
            ip: "".to_string(),
            domains: vec![],
            error: Some("IP 地址不能为空".to_string()),
        });
    }

    let parsed_ip: IpAddr = ip.parse().map_err(|_| "无效的 IP 地址格式".to_string())?;

    match dns_lookup::lookup_addr(&parsed_ip) {
        Ok(hostname) => Ok(ReverseDnsResponse {
            ip,
            domains: vec![hostname],
            error: None,
        }),
        Err(e) => Err(format!("反向 DNS 查询失败: {}", e)),
    }
}

pub async fn batch_reverse_dns_lookup(ips: Vec<String>) -> Result<BatchReverseDnsResponse, String> {
    if ips.is_empty() {
        return Ok(BatchReverseDnsResponse {
            results: vec![],
            error: Some("IP 地址列表不能为空".to_string()),
        });
    }

    if ips.len() > 50 {
        return Ok(BatchReverseDnsResponse {
            results: vec![],
            error: Some("单次查询的 IP 地址数量不能超过 50 个".to_string()),
        });
    }

    let mut results = vec![];
    let mut has_error = false;
    let mut error_message = None;

    for ip in ips {
        match reverse_dns_lookup(ip.clone()) {
            Ok(response) => results.push(response),
            Err(e) => {
                has_error = true;
                if error_message.is_none() {
                    error_message = Some(e.clone());
                }
                results.push(ReverseDnsResponse {
                    ip,
                    domains: vec![],
                    error: Some(e),
                });
            }
        }
    }

    Ok(BatchReverseDnsResponse {
        results,
        error: if has_error { error_message } else { None },
    })
}

pub async fn get_dns_servers() -> Result<Vec<DnsServerEntry>, String> {
    Ok(default_dns_servers())
}

#[cfg(test)]
mod tests {
    use super::*;
    use hickory_resolver::proto::rr::rdata::{CNAME, MX, NS, PTR, SOA, TXT};
    use hickory_resolver::proto::rr::Name;
    use std::net::{Ipv4Addr, Ipv6Addr};
    use std::sync::Mutex;

    type GeolocateFuture = Pin<Box<dyn Future<Output = Result<IpGeolocationInfo, String>> + Send>>;

    fn a_record(value: &str, ttl: u32) -> DnsRecord {
        DnsRecord {
            record_type: "A".to_string(),
            name: "example.com.".to_string(),
            value: value.to_string(),
            ttl: Some(ttl),
        }
    }

    /// 查询桩：`fail` 中的服务器失败，`slow` 中的服务器延迟 40ms 应答；
    /// 其余返回一条值含服务器名的 A 记录（便于核对归属）。
    fn mock_query(
        fail: &'static [&'static str],
        slow: &'static [&'static str],
    ) -> impl Fn(String, String, DnsRecordType) -> QueryFuture + Send + Sync + Clone + 'static {
        move |_, server, _| -> QueryFuture {
            Box::pin(async move {
                if slow.contains(&server.as_str()) {
                    tokio::time::sleep(Duration::from_millis(40)).await;
                }
                if fail.contains(&server.as_str()) {
                    return Err(format!("查询失败: {}", server));
                }
                Ok(vec![a_record(&format!("{}.1", server), 60)])
            })
        }
    }

    /// 归属查询桩：记录被询问的 IP，返回回显它的归属信息。
    fn geolocate_recording(
        calls: Arc<Mutex<Vec<String>>>,
    ) -> impl Fn(String) -> GeolocateFuture + Send + 'static {
        move |ip| -> GeolocateFuture {
            let calls = Arc::clone(&calls);
            Box::pin(async move {
                calls.lock().unwrap().push(ip.clone());
                Ok(IpGeolocationInfo {
                    ip,
                    country: None,
                    city: None,
                    org: None,
                    region: None,
                    timezone: None,
                    source: "test".to_string(),
                })
            })
        }
    }

    async fn lookup_with_mock<Q>(
        domain: &str,
        servers: Option<Vec<String>>,
        record_type: &str,
        query: Q,
    ) -> DnsLookupResponse
    where
        Q: Fn(String, String, DnsRecordType) -> QueryFuture + Send + Sync + 'static,
    {
        // 编排用例不关心归属信息：归属查询给空实现（ipinfo 实现在单独的用例里覆盖）
        let no_geolocation = |_: String| -> GeolocateFuture {
            Box::pin(async { Err("归属查询未启用".to_string()) })
        };
        lookup_dns_with(
            domain.to_string(),
            servers,
            record_type.to_string(),
            query,
            no_geolocation,
        )
        .await
        .expect("lookup_dns_with 不应返回 Err")
    }

    fn server_names(response: &DnsLookupResponse) -> Vec<&str> {
        response
            .results
            .iter()
            .map(|r| r.dns_server.as_str())
            .collect()
    }

    // ---- 纯函数 ----

    #[test]
    fn format_rdata_matches_dig_presentation() {
        assert_eq!(
            format_rdata(&RData::A(Ipv4Addr::new(93, 184, 215, 14).into())).as_deref(),
            Some("93.184.215.14")
        );
        let ipv6: Ipv6Addr = "2606:2800:21f:cb07:6820:80da:af6b:8b2c".parse().unwrap();
        assert_eq!(
            format_rdata(&RData::AAAA(ipv6.into())).as_deref(),
            Some("2606:2800:21f:cb07:6820:80da:af6b:8b2c")
        );

        // 域名带根点；MX 为「优先级 主机」
        let name = Name::from_utf8("www.example.com.").unwrap();
        assert_eq!(
            format_rdata(&RData::CNAME(CNAME(name.clone()))).as_deref(),
            Some("www.example.com.")
        );
        assert_eq!(
            format_rdata(&RData::NS(NS(name.clone()))).as_deref(),
            Some("www.example.com.")
        );
        assert_eq!(
            format_rdata(&RData::MX(MX::new(10, name.clone()))).as_deref(),
            Some("10 www.example.com.")
        );

        // TXT：每段字符串各加引号、空格分隔，内部的引号与反斜杠转义
        assert_eq!(
            format_rdata(&RData::TXT(TXT::new(vec!["v=spf1 -all".to_string()]))).as_deref(),
            Some("\"v=spf1 -all\"")
        );
        assert_eq!(
            format_rdata(&RData::TXT(TXT::new(vec![
                "a\"b\\c".to_string(),
                "d e".to_string()
            ])))
            .as_deref(),
            Some("\"a\\\"b\\\\c\" \"d e\"")
        );

        // SOA：主名 管理员邮箱 序列号 刷新 重试 过期 最小 TTL
        let mname = Name::from_utf8("ns.example.com.").unwrap();
        let rname = Name::from_utf8("hostmaster.example.com.").unwrap();
        assert_eq!(
            format_rdata(&RData::SOA(SOA::new(
                mname, rname, 2024010101, 7200, 3600, 1209600, 3600
            )))
            .as_deref(),
            Some("ns.example.com. hostmaster.example.com. 2024010101 7200 3600 1209600 3600")
        );

        // 工具不支持的类型（如 PTR）不产出记录值
        assert_eq!(format_rdata(&RData::PTR(PTR(name))), None);
    }

    #[test]
    fn server_ip_accepts_literals_and_hostnames() {
        assert_eq!(
            resolve_server_ip("8.8.8.8").unwrap(),
            "8.8.8.8".parse::<IpAddr>().unwrap()
        );
        assert_eq!(
            resolve_server_ip(" 2001:4860:4860::8888 ").unwrap(),
            "2001:4860:4860::8888".parse::<IpAddr>().unwrap()
        );
        // 主机名走系统解析器；localhost 由 hosts 文件保证（离线可得），IPv4 优先
        assert_eq!(
            resolve_server_ip("localhost").unwrap(),
            "127.0.0.1".parse::<IpAddr>().unwrap()
        );
        // .invalid 顶级域保证解析不出结果（RFC 6761）
        let err = resolve_server_ip("no-such-host.invalid.").unwrap_err();
        assert!(err.starts_with("无效的 DNS 服务器地址"), "{}", err);
    }

    #[test]
    fn query_errors_are_described_per_cause() {
        let record_type = DnsRecordType::A;
        assert_eq!(
            describe_query_error(&NetError::Timeout, record_type),
            "DNS 查询失败: 查询超时"
        );
        assert_eq!(
            describe_query_error(&NetError::Message("boom"), record_type),
            "DNS 查询失败: boom"
        );
        let servfail = describe_query_error(
            &NetError::Dns(DnsError::ResponseCode(ResponseCode::ServFail)),
            record_type,
        );
        assert_eq!(servfail, "DNS 查询失败: 服务器返回 SERVFAIL");
        // 少见响应码回退为可读描述
        let unknown = describe_query_error(
            &NetError::Dns(DnsError::ResponseCode(ResponseCode::NotZone)),
            record_type,
        );
        assert_eq!(unknown, "DNS 查询失败: 服务器返回 Name not in zone");
    }

    #[test]
    fn default_dns_servers_are_ordered() {
        let servers = default_dns_servers();
        assert_eq!(servers.len(), 4);
        assert_eq!(servers[0].server, "223.5.5.5");
        assert_eq!(servers[3].server, "1.1.1.1");
    }

    // ---- 查询编排（注入查询桩，不发网络请求） ----

    #[tokio::test]
    async fn lookup_keeps_input_order_whatever_finishes_first() {
        // 第一个服务器慢、第二个快：若按完成顺序归集，结果会颠倒
        let response = lookup_with_mock(
            "example.com",
            Some(vec![
                "223.5.5.5".to_string(),
                "8.8.8.8".to_string(),
                "223.5.5.5".to_string(),
            ]),
            "A",
            mock_query(&[], &["223.5.5.5"]),
        )
        .await;

        assert!(response.error.is_none());
        assert_eq!(
            server_names(&response),
            vec!["223.5.5.5", "8.8.8.8"],
            "重复服务器去重，顺序等于输入顺序"
        );
        for result in &response.results {
            assert!(result.error.is_none(), "查询失败: {:?}", result.error);
            assert!(result.latency_ms.is_some());
            assert_eq!(
                result.records,
                vec![a_record(&format!("{}.1", result.dns_server), 60)]
            );
        }
    }

    #[tokio::test]
    async fn lookup_reports_each_server_failure_in_its_own_row() {
        let response = lookup_with_mock(
            "example.com",
            Some(vec!["dead.beef".to_string(), "8.8.8.8".to_string()]),
            "A",
            mock_query(&["dead.beef"], &[]),
        )
        .await;

        assert_eq!(response.results.len(), 2);
        assert_eq!(
            response.results[0].error.as_deref(),
            Some("查询失败: dead.beef")
        );
        assert!(response.results[0].records.is_empty());
        assert!(
            response.results[0].latency_ms.is_some(),
            "失败的查询同样带耗时"
        );
        assert!(response.results[1].error.is_none());
        assert!(response.error.is_none(), "单台服务器失败不是整体失败");
    }

    #[tokio::test]
    async fn lookup_rejects_empty_domain_and_too_many_servers() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let counting = {
            let calls = Arc::clone(&calls);
            move |_: String, server: String, _: DnsRecordType| -> QueryFuture {
                let calls = Arc::clone(&calls);
                Box::pin(async move {
                    calls.lock().unwrap().push(server);
                    Ok(vec![a_record("1.1.1.1", 1)])
                })
            }
        };

        let empty = lookup_with_mock(
            "   ",
            Some(vec!["8.8.8.8".to_string()]),
            "A",
            counting.clone(),
        )
        .await;
        assert_eq!(empty.domain, "");
        assert_eq!(empty.error.as_deref(), Some("域名不能为空"));
        assert!(empty.results.is_empty());
        assert!(calls.lock().unwrap().is_empty(), "空域名不应发起任何查询");

        let too_many: Vec<String> = (1..=9).map(|i| format!("10.0.0.{}", i)).collect();
        let rejected = lookup_with_mock("example.com", Some(too_many), "A", counting).await;
        assert_eq!(rejected.domain, "example.com");
        assert_eq!(
            rejected.error.as_deref(),
            Some("DNS 服务器数量不能超过 8 个")
        );
        assert!(rejected.results.is_empty());
        assert!(calls.lock().unwrap().is_empty(), "超限不应发起任何查询");
    }

    #[tokio::test]
    async fn lookup_drops_blank_and_duplicate_servers_and_falls_back() {
        let response = lookup_with_mock(
            "example.com",
            Some(vec![
                "".to_string(),
                "   ".to_string(),
                " 8.8.8.8 ".to_string(),
                "8.8.8.8".to_string(),
            ]),
            "A",
            mock_query(&[], &[]),
        )
        .await;
        assert_eq!(
            server_names(&response),
            vec!["8.8.8.8"],
            "空白项丢弃；首尾空格规整后重复的服务器合并"
        );

        let fallback =
            lookup_with_mock("example.com", Some(vec![]), "A", mock_query(&[], &[])).await;
        assert_eq!(server_names(&fallback), vec![FALLBACK_DNS_SERVER]);
    }

    #[tokio::test]
    async fn geolocation_uses_first_successful_server_address() {
        let asked = Arc::new(Mutex::new(Vec::new()));
        let response = lookup_dns_with(
            "example.com".to_string(),
            Some(vec!["dead.beef".to_string(), "8.8.8.8".to_string()]),
            "A".to_string(),
            mock_query(&["dead.beef"], &[]),
            geolocate_recording(Arc::clone(&asked)),
        )
        .await
        .expect("lookup_dns_with 不应返回 Err");

        assert_eq!(
            *asked.lock().unwrap(),
            vec!["8.8.8.8.1".to_string()],
            "取首个成功服务器的首条 A 记录"
        );
        assert_eq!(
            response.ip_info.map(|info| info.ip).as_deref(),
            Some("8.8.8.8.1")
        );
    }

    #[tokio::test]
    async fn geolocation_failure_does_not_fail_the_lookup() {
        let failing =
            |_: String| -> GeolocateFuture { Box::pin(async { Err("ipinfo 限流".to_string()) }) };
        let response = lookup_dns_with(
            "example.com".to_string(),
            Some(vec!["8.8.8.8".to_string()]),
            "A".to_string(),
            mock_query(&[], &[]),
            failing,
        )
        .await
        .expect("lookup_dns_with 不应返回 Err");

        assert!(response.error.is_none());
        assert!(response.ip_info.is_none());
        assert!(response.results[0].error.is_none());
    }

    #[tokio::test]
    async fn panicking_query_becomes_that_servers_error() {
        let query = |_: String, server: String, _: DnsRecordType| -> QueryFuture {
            Box::pin(async move {
                if server == "boom" {
                    panic!("查询内部崩溃");
                }
                Ok(vec![a_record("1.1.1.1", 1)])
            })
        };
        let response = lookup_with_mock(
            "example.com",
            Some(vec!["boom".to_string(), "8.8.8.8".to_string()]),
            "A",
            query,
        )
        .await;

        assert_eq!(
            response.results.len(),
            2,
            "崩溃的服务器仍占一行，不静默消失"
        );
        let error = response.results[0].error.clone().unwrap_or_default();
        assert!(error.contains("查询任务异常退出"), "{}", error);
        assert!(response.results[1].error.is_none());
    }

    // ---- 实网冒烟测试（默认忽略） ----

    /// 实网冒烟测试：需要能访问公共 DNS（8.8.8.8:53），手动运行
    /// `cargo test dns_resolver -- --ignored`。
    #[tokio::test]
    #[ignore = "依赖实网公共 DNS"]
    async fn smoke_lookup_real_domain() {
        let response = lookup_dns(
            "example.com".to_string(),
            Some(vec!["8.8.8.8".to_string()]),
            "A".to_string(),
        )
        .await
        .expect("lookup_dns 不应返回 Err");

        assert!(response.error.is_none());
        assert_eq!(server_names(&response), vec!["8.8.8.8"]);
        let result = &response.results[0];
        let error = result.error.clone().unwrap_or_default();
        assert!(result.error.is_none(), "查询失败: {}", error);
        assert!(result.latency_ms.is_some());
        assert!(!result.records.is_empty());
        for record in &result.records {
            assert_eq!(record.record_type, "A");
            assert!(record.name.ends_with('.'), "域名带根点: {}", record.name);
            assert!(
                record.value.parse::<IpAddr>().is_ok(),
                "A 记录值应为 IP: {}",
                record.value
            );
            assert!(record.ttl.is_some());
        }

        // ALL：逐类查询后合并
        let all = lookup_dns(
            "example.com".to_string(),
            Some(vec!["8.8.8.8".to_string()]),
            "ALL".to_string(),
        )
        .await
        .expect("lookup_dns 不应返回 Err");
        let error = all.results[0].error.clone().unwrap_or_default();
        assert!(all.results[0].error.is_none(), "查询失败: {}", error);
        let types: Vec<&str> = all.results[0]
            .records
            .iter()
            .map(|record| record.record_type.as_str())
            .collect();
        for expected in ["A", "MX", "NS", "SOA"] {
            assert!(
                types.contains(&expected),
                "ALL 结果缺 {}: {:?}",
                expected,
                types
            );
        }
    }
}
