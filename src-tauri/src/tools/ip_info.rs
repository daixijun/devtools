use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SourceInfo {
    pub source: String,
    pub ip: Option<String>,
    pub country: Option<String>,
    pub city: Option<String>,
    pub org: Option<String>,
    pub region: Option<String>,
    pub timezone: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IpLookupResponse {
    pub infos: Vec<SourceInfo>,
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
    // Some responses include error object; we ignore other fields.
}

#[derive(Deserialize)]
struct IpApiComRaw {
    status: Option<String>,
    message: Option<String>,
    query: Option<String>,
    country: Option<String>,
    city: Option<String>,
    org: Option<String>,
    #[serde(rename = "regionName")]
    region_name: Option<String>,
    timezone: Option<String>,
}

#[tauri::command]
pub async fn query_ip_info(ip: Option<String>) -> Result<IpLookupResponse, String> {
    let client = reqwest::Client::new();

    // Build endpoints
    let ipinfo_url = match ip.as_deref() {
        Some(s) if !s.trim().is_empty() => format!("https://ipinfo.io/{}/json", s.trim()),
        _ => "https://ipinfo.io/json".to_string(),
    };

    let ipapi_url = match ip.as_deref() {
        Some(s) if !s.trim().is_empty() => {
            format!("http://ip-api.com/json/{}?lang=zh-CN", s.trim())
        }
        _ => "http://ip-api.com/json/?lang=zh-CN".to_string(),
    };

    let mut infos: Vec<SourceInfo> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    // Fetch from ipinfo.io
    match client.get(&ipinfo_url).send().await {
        Ok(resp) => match resp.error_for_status() {
            Ok(ok) => match ok.json::<serde_json::Value>().await {
                Ok(val) => {
                    // Try to parse into our raw struct first
                    let raw: Option<IpInfoIoRaw> = serde_json::from_value(val.clone()).ok();
                    if let Some(r) = raw {
                        // even if ip missing, still push the source with available fields
                        infos.push(SourceInfo {
                            source: "ipinfo.io".into(),
                            ip: r.ip,
                            country: r.country,
                            city: r.city,
                            org: r.org,
                            region: r.region,
                            timezone: r.timezone,
                        });
                    } else {
                        errors.push("无法解析 ipinfo.io 返回的数据".into());
                    }
                }
                Err(e) => errors.push(format!("解析 ipinfo.io 响应失败: {}", e)),
            },
            Err(e) => errors.push(format!("ipinfo.io 请求失败: {}", e)),
        },
        Err(e) => errors.push(format!("请求 ipinfo.io 出错: {}", e)),
    }

    // Fetch from ip-api.com
    match client.get(&ipapi_url).send().await {
        Ok(resp) => match resp.error_for_status() {
            Ok(ok) => match ok.json::<IpApiComRaw>().await {
                Ok(raw) => {
                    if matches!(raw.status.as_deref(), Some("fail")) {
                        errors.push(raw.message.unwrap_or_else(|| "ip-api.com 查询失败".into()));
                    } else {
                        infos.push(SourceInfo {
                            source: "ip-api.com".into(),
                            ip: raw.query,
                            country: raw.country,
                            city: raw.city,
                            org: raw.org,
                            region: raw.region_name,
                            timezone: raw.timezone,
                        });
                    }
                }
                Err(e) => errors.push(format!("解析 ip-api.com 响应失败: {}", e)),
            },
            Err(e) => errors.push(format!("ip-api.com 请求失败: {}", e)),
        },
        Err(e) => errors.push(format!("请求 ip-api.com 出错: {}", e)),
    }

    let error = if errors.is_empty() {
        None
    } else {
        Some(errors.join("；"))
    };

    Ok(IpLookupResponse { infos, error })
}
