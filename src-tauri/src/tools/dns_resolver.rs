use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::net::IpAddr;
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize, Clone)]
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

#[derive(Clone)]
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

fn parse_dig_output(
    output: &str,
    record_type: &str,
) -> Result<Vec<DnsRecord>, String> {
    let mut records = Vec::new();
    let lines: Vec<&str> = output.lines().collect();

    let mut in_answer_section = false;

    for line in lines {
        let line = line.trim();

        if line.starts_with(";; ANSWER SECTION:") {
            in_answer_section = true;
            continue;
        }

        if line.starts_with(";;") {
            in_answer_section = false;
            continue;
        }

        if in_answer_section && !line.is_empty() && !line.starts_with(";") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 5 {
                let name = parts[0];
                let ttl = parts[1].parse::<u32>().unwrap_or(0);
                let _class = parts[2];
                let rtype = parts[3];

                if rtype == record_type || record_type == "ALL" {
                    let value_parts: Vec<&str> = parts[4..].to_vec();
                    let value = value_parts.join(" ");

                    records.push(DnsRecord {
                        record_type: rtype.to_string(),
                        name: name.to_string(),
                        value,
                        ttl: Some(ttl),
                    });
                }
            }
        }
    }

    if records.is_empty() {
        return Err(format!("未找到 {} 记录", record_type));
    }

    Ok(records)
}

async fn run_dig_query(
    domain: &str,
    dns_server: &str,
    record_type: DnsRecordType,
) -> Result<Vec<DnsRecord>, String> {
    let record_type_str = record_type.to_string();
    let output = if record_type_str == "ALL" {
        // 查询所有记录
        let mut all_records = Vec::new();
        let types = ["A", "AAAA", "MX", "TXT", "CNAME", "NS", "SOA"];

        for rtype in &types {
            match run_single_dig_query(domain, dns_server, rtype).await {
                Ok(mut records) => all_records.append(&mut records),
                Err(_) => {}
            }
        }

        if all_records.is_empty() {
            return Err("未找到任何 DNS 记录".to_string());
        }

        return Ok(all_records);
    } else {
        run_single_dig_query(domain, dns_server, &record_type_str).await?
    };

    Ok(output)
}

async fn run_single_dig_query(
    domain: &str,
    dns_server: &str,
    record_type: &str,
) -> Result<Vec<DnsRecord>, String> {
    let output = tokio::process::Command::new("dig")
        .args(&[
            "+time=5",
            "+tries=2",
            "+nocmd",
            format!("@{}", dns_server).as_str(),
            domain,
            record_type,
        ])
        .output()
        .await
        .map_err(|e| format!("执行 dig 命令失败: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("DNS 查询失败: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_dig_output(&stdout, record_type)
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

#[tauri::command]
pub async fn lookup_dns(
    domain: String,
    dns_servers: Option<Vec<String>>,
    record_type: String,
) -> Result<DnsLookupResponse, String> {
    if domain.trim().is_empty() {
        return Ok(DnsLookupResponse {
            domain: "".to_string(),
            results: vec![],
            ip_info: None,
            error: Some("域名不能为空".to_string()),
        });
    }

    let mut servers = dns_servers.unwrap_or_default();
    servers.retain(|s| !s.trim().is_empty());
    let mut seen = HashSet::new();
    servers.retain(|s| seen.insert(s.clone()));

    if servers.len() > MAX_DNS_SERVERS {
        return Ok(DnsLookupResponse {
            domain,
            results: vec![],
            ip_info: None,
            error: Some(format!(
                "DNS 服务器数量不能超过 {} 个",
                MAX_DNS_SERVERS
            )),
        });
    }

    if servers.is_empty() {
        servers.push(FALLBACK_DNS_SERVER.to_string());
    }

    // 并发查询所有选中的服务器；指定了服务器时统一走 dig，保证所选服务器真实生效
    let record_type_enum = DnsRecordType::from(record_type.as_str());
    let server_count = servers.len();
    let mut join_set = tokio::task::JoinSet::new();

    for (idx, server) in servers.into_iter().enumerate() {
        let domain = domain.clone();
        let record_type = record_type_enum.clone();
        join_set.spawn(async move {
            let start = tokio::time::Instant::now();
            let result = run_dig_query(&domain, &server, record_type).await;
            let latency_ms = start.elapsed().as_millis() as u64;
            (idx, server, result, latency_ms)
        });
    }

    let mut slots: Vec<Option<DnsServerLookupResult>> =
        (0..server_count).map(|_| None).collect();
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

    // 取第一个成功服务器的首个 A/AAAA 记录查询 IP 地理位置
    let mut ip_info = None;
    if let Some(ip) = results
        .iter()
        .filter(|r| r.error.is_none())
        .flat_map(|r| r.records.iter())
        .find(|rec| rec.record_type == "A" || rec.record_type == "AAAA")
        .map(|rec| rec.value.clone())
    {
        if let Ok(info) = query_ip_geolocation(&ip).await {
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

#[tauri::command]
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

#[tauri::command]
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

#[tauri::command]
pub async fn get_dns_servers() -> Result<Vec<DnsServerEntry>, String> {
    Ok(default_dns_servers())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn multi_server_lookup_returns_grouped_results_in_order() {
        let response = lookup_dns(
            "example.com".to_string(),
            Some(vec![
                "223.5.5.5".to_string(),
                "8.8.8.8".to_string(),
                "223.5.5.5".to_string(), // 重复项应被去重
            ]),
            "A".to_string(),
        )
        .await
        .expect("lookup_dns 不应返回 Err");

        assert!(response.error.is_none());
        assert_eq!(response.results.len(), 2, "重复服务器应被去重");
        assert_eq!(response.results[0].dns_server, "223.5.5.5");
        assert_eq!(response.results[1].dns_server, "8.8.8.8");

        for result in &response.results {
            assert!(result.error.is_none(), "查询失败: {:?}", result.error);
            assert!(result.latency_ms.is_some());
            assert!(!result.records.is_empty());
            assert!(result
                .records
                .iter()
                .all(|r| r.record_type == "A" && r.ttl.is_some()));
        }

        // ip_info 应基于第一个成功服务器的 A 记录
        let ip = response.results[0].records[0].value.clone();
        assert_eq!(response.ip_info.expect("应有 ip_info").ip, ip);
    }

    #[tokio::test]
    async fn empty_server_list_falls_back_to_default() {
        let response = lookup_dns(
            "example.com".to_string(),
            Some(vec![]),
            "A".to_string(),
        )
        .await
        .expect("lookup_dns 不应返回 Err");

        assert!(response.error.is_none());
        assert_eq!(response.results.len(), 1);
        assert_eq!(response.results[0].dns_server, FALLBACK_DNS_SERVER);
    }

    #[test]
    fn default_dns_servers_are_ordered() {
        let servers = default_dns_servers();
        assert_eq!(servers.len(), 4);
        assert_eq!(servers[0].server, "223.5.5.5");
        assert_eq!(servers[3].server, "1.1.1.1");
    }
}
