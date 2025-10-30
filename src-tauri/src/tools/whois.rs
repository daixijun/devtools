use regex::Regex;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WhoisParsed {
    pub domain: String,
    pub source: String,
    pub registrar: Option<String>,
    pub registrant: Option<String>,
    pub created: Option<String>,
    pub expires: Option<String>,
    pub updated: Option<String>,
    pub status: Option<Vec<String>>,
    pub name_servers: Option<Vec<String>>,
    pub raw_text: Option<String>,
}

fn extract_tld(domain: &str) -> Option<String> {
    let parts: Vec<&str> = domain.split('.').collect();
    if parts.len() >= 2 {
        Some(parts.last().unwrap().to_string())
    } else {
        None
    }
}

fn parse_whois_text(domain: &str, source: &str, text: &str) -> WhoisParsed {
    let registrar_re = Regex::new(r"(?i)^\s*(Registrar|Sponsoring Registrar)\s*:\s*(.+)$").ok();
    let registrant_re =
        Regex::new(r"(?i)^\s*(Registrant Organization|Registrant Name)\s*:\s*(.+)$").ok();
    let created_re = Regex::new(
        r"(?i)^\s*(Creation Date|Created On|Domain Registration Date|Registered)\s*:\s*(.+)$",
    )
    .ok();
    let expires_re = Regex::new(
        r"(?i)^\s*(Registry Expiry Date|Expiration Date|Expires On|Expiry Date)\s*:\s*(.+)$",
    )
    .ok();
    let updated_re =
        Regex::new(r"(?i)^\s*(Updated Date|Last Updated On|Last Updated)\s*:\s*(.+)$").ok();
    let ns_re = Regex::new(r"(?i)^\s*Name Server\s*:\s*(.+)$").ok();

    let mut registrar = None;
    let mut registrant = None;
    let mut created = None;
    let mut expires = None;
    let mut updated = None;
    let mut name_servers: Vec<String> = Vec::new();

    for line in text.lines() {
        if let Some(re) = &registrar_re {
            if let Some(cap) = re.captures(line) {
                registrar = Some(
                    cap.get(2)
                        .map(|m| m.as_str().trim().to_string())
                        .unwrap_or_default(),
                );
                continue;
            }
        }
        if let Some(re) = &registrant_re {
            if let Some(cap) = re.captures(line) {
                registrant = Some(
                    cap.get(2)
                        .map(|m| m.as_str().trim().to_string())
                        .unwrap_or_default(),
                );
                continue;
            }
        }
        if let Some(re) = &created_re {
            if let Some(cap) = re.captures(line) {
                created = Some(
                    cap.get(2)
                        .map(|m| m.as_str().trim().to_string())
                        .unwrap_or_default(),
                );
                continue;
            }
        }
        if let Some(re) = &expires_re {
            if let Some(cap) = re.captures(line) {
                expires = Some(
                    cap.get(2)
                        .map(|m| m.as_str().trim().to_string())
                        .unwrap_or_default(),
                );
                continue;
            }
        }
        if let Some(re) = &updated_re {
            if let Some(cap) = re.captures(line) {
                updated = Some(
                    cap.get(2)
                        .map(|m| m.as_str().trim().to_string())
                        .unwrap_or_default(),
                );
                continue;
            }
        }
        if let Some(re) = &ns_re {
            if let Some(cap) = re.captures(line) {
                let ns = cap.get(1).map(|m| m.as_str().trim().to_string());
                if let Some(ns) = ns {
                    name_servers.push(ns);
                }
                continue;
            }
        }
    }

    WhoisParsed {
        domain: domain.to_string(),
        source: source.to_string(),
        registrar,
        registrant,
        created,
        expires,
        updated,
        status: None,
        name_servers: if name_servers.is_empty() {
            None
        } else {
            Some(name_servers)
        },
        raw_text: Some(text.to_string()),
    }
}

fn parse_rdap_json(domain: &str, source: &str, val: &serde_json::Value) -> WhoisParsed {
    let mut registrar: Option<String> = None;
    let mut registrant: Option<String> = None;
    let mut created: Option<String> = None;
    let mut expires: Option<String> = None;
    let mut updated: Option<String> = None;
    let mut status: Option<Vec<String>> = None;
    let mut name_servers: Option<Vec<String>> = None;

    if let Some(events) = val.get("events").and_then(|e| e.as_array()) {
        for ev in events {
            if let (Some(action), Some(date)) = (ev.get("eventAction"), ev.get("eventDate")) {
                match action.as_str().unwrap_or("") {
                    "registration" => created = date.as_str().map(|s| s.to_string()),
                    "expiration" => expires = date.as_str().map(|s| s.to_string()),
                    "last changed" | "last update of RDAP database" => {
                        updated = date.as_str().map(|s| s.to_string())
                    }
                    _ => {}
                }
            }
        }
    }

    if let Some(status_arr) = val.get("status").and_then(|s| s.as_array()) {
        let mut statuses = Vec::new();
        for s in status_arr {
            if let Some(st) = s.as_str() {
                statuses.push(st.to_string());
            }
        }
        if !statuses.is_empty() {
            status = Some(statuses);
        }
    }

    if let Some(ns_arr) = val.get("nameservers").and_then(|a| a.as_array()) {
        let mut ns_list = Vec::new();
        for ns in ns_arr {
            if let Some(lab) = ns.get("ldhName").and_then(|s| s.as_str()) {
                ns_list.push(lab.to_string());
            }
        }
        if !ns_list.is_empty() {
            name_servers = Some(ns_list);
        }
    }

    // Entities for registrar / registrant
    if let Some(entities) = val.get("entities").and_then(|e| e.as_array()) {
        for ent in entities {
            let role = ent
                .get("roles")
                .and_then(|r| r.as_array())
                .and_then(|arr: &Vec<serde_json::Value>| {
                    arr.iter().find_map(|v: &serde_json::Value| v.as_str())
                })
                .unwrap_or("");
            let name = ent.get("vcardArray").and_then(|vc| {
                // vcardArray: ["vcard", [["fn", {}, "text", "Name"], ...]]
                vc.get(1).and_then(|items| items.as_array()).and_then(
                    |items: &Vec<serde_json::Value>| {
                        for item in items {
                            if let Some(prop) = item.get(0).and_then(|v| v.as_str()) {
                                if prop == "fn" {
                                    if let Some(val) = item.get(3).and_then(|v| v.as_str()) {
                                        return Some(val.to_string());
                                    }
                                }
                            }
                        }
                        None
                    },
                )
            });
            match role {
                "registrant" => {
                    if name.is_some() {
                        registrant = name.clone();
                    }
                }
                "registrar" => {
                    if name.is_some() {
                        registrar = name.clone();
                    }
                }
                _ => {}
            }
        }
    }

    WhoisParsed {
        domain: domain.to_string(),
        source: source.to_string(),
        registrar,
        registrant,
        created,
        expires,
        updated,
        status,
        name_servers,
        raw_text: Some(val.to_string()),
    }
}

async fn rdap_org_query(domain: &str) -> Result<WhoisParsed, String> {
    let url = format!("https://rdap.org/domain/{}", domain);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .map_err(|e| format!("构建 HTTP 客户端失败: {}", e))?;
    let resp = client
        .get(&url)
        .header("Accept", "application/rdap+json")
        .send()
        .await
        .map_err(|e| format!("rdap.org 请求失败: {}", e))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(format!("rdap.org 响应状态异常: {}", status));
    }
    let val: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("解析 rdap.org 响应失败: {}", e))?;
    Ok(parse_rdap_json(domain, "rdap.org", &val))
}

async fn rdap_verisign_query(domain: &str) -> Result<WhoisParsed, String> {
    let tld = extract_tld(domain).unwrap_or_default();
    if tld != "com" && tld != "net" {
        return Err("Verisign RDAP 仅支持 .com/.net".to_string());
    }
    let url = format!("https://rdap.verisign.com/{}/v1/domain/{}", tld, domain);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .map_err(|e| format!("构建 HTTP 客户端失败: {}", e))?;
    let resp = client
        .get(&url)
        .header("Accept", "application/rdap+json")
        .send()
        .await
        .map_err(|e| format!("Verisign RDAP 请求失败: {}", e))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(format!("Verisign RDAP 响应状态异常: {}", status));
    }
    let val: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("解析 Verisign RDAP 响应失败: {}", e))?;
    Ok(parse_rdap_json(domain, "rdap.verisign.com", &val))
}

fn query_whois_server(server: &str, query: &str) -> Result<String, String> {
    let timeout = Duration::from_secs(5);
    // Resolve hostname to SocketAddr (supports DNS hostnames)
    let mut addrs = (server, 43)
        .to_socket_addrs()
        .map_err(|e| format!("解析 WHOIS 服务器地址失败: {}", e))?;
    let socket_addr = addrs
        .next()
        .ok_or_else(|| "解析 WHOIS 服务器地址失败: 无有效地址".to_string())?;
    let mut stream = TcpStream::connect_timeout(&socket_addr, timeout)
        .map_err(|e| format!("连接 WHOIS 服务器失败: {}", e))?;
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|e| format!("设置读取超时失败: {}", e))?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(|e| format!("设置写入超时失败: {}", e))?;

    let q = format!("{}\r\n", query);
    stream
        .write_all(q.as_bytes())
        .map_err(|e| format!("发送查询失败: {}", e))?;
    let mut buf = Vec::new();
    stream
        .read_to_end(&mut buf)
        .map_err(|e| format!("读取响应失败: {}", e))?;

    let text = String::from_utf8_lossy(&buf).to_string();
    Ok(text)
}

fn resolve_whois_server_for_tld(tld: &str) -> Option<String> {
    // Try IANA referral first
    if let Ok(resp) = query_whois_server("whois.iana.org", tld) {
        for line in resp.lines() {
            let l = line.trim();
            if l.to_ascii_lowercase().starts_with("refer:")
                || l.to_ascii_lowercase().starts_with("whois:")
            {
                let parts: Vec<&str> = l.split(':').collect();
                if parts.len() >= 2 {
                    let server = parts[1].trim().to_string();
                    if !server.is_empty() {
                        return Some(server);
                    }
                }
            }
        }
    }
    // Fallbacks
    match tld {
        "com" | "net" => Some("whois.verisign-grs.com".to_string()),
        _ => Some(format!("{}.whois-servers.net", tld)),
    }
}

#[tauri::command]
pub async fn query_rdap(domain: String) -> Result<WhoisParsed, String> {
    let d = domain.trim();
    if d.is_empty() {
        return Err("域名不能为空".to_string());
    }
    // Try rdap.org, then Verisign RDAP if appropriate
    match rdap_org_query(d).await {
        Ok(p) => Ok(p),
        Err(e1) => {
            if matches!(extract_tld(d).as_deref(), Some("com") | Some("net")) {
                match rdap_verisign_query(d).await {
                    Ok(p) => Ok(p),
                    Err(e2) => Err(format!("RDAP 查询失败: {}; {}", e1, e2)),
                }
            } else {
                Err(e1)
            }
        }
    }
}

#[tauri::command]
pub async fn query_whois(domain: String) -> Result<WhoisParsed, String> {
    let d = domain.trim();
    if d.is_empty() {
        return Err("域名不能为空".to_string());
    }
    let tld = extract_tld(d).ok_or_else(|| "无法解析域名 TLD".to_string())?;
    let server =
        resolve_whois_server_for_tld(&tld).ok_or_else(|| "无法解析 WHOIS 服务器".to_string())?;

    // Use blocking network in a blocking task
    let d_owned = d.to_string();
    let s_owned = server.clone();
    let text = tokio::task::spawn_blocking(move || query_whois_server(&s_owned, &d_owned))
        .await
        .map_err(|_| "WHOIS 查询线程执行失败".to_string())??;

    Ok(parse_whois_text(d, &server, &text))
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MultiSourceResult {
    pub domain: String,
    pub results: Vec<WhoisParsed>,
    pub error: Option<String>,
}

#[tauri::command]
pub async fn query_domain_multi_source(domain: String) -> Result<MultiSourceResult, String> {
    let d = domain.trim();
    if d.is_empty() {
        return Err("域名不能为空".to_string());
    }
    let mut results = Vec::new();
    let mut errors = Vec::new();

    // Channel 1: RDAP.org
    match query_rdap(d.to_string()).await {
        Ok(p) => results.push(p),
        Err(e) => errors.push(format!("rdap.org: {}", e)),
    }
    // Channel 2: WHOIS via IANA referral
    match query_whois(d.to_string()).await {
        Ok(p) => results.push(p),
        Err(e) => errors.push(format!("whois-referral: {}", e)),
    }
    // Channel 3: Verisign RDAP (only for com/net)
    if matches!(extract_tld(d).as_deref(), Some("com") | Some("net")) {
        match rdap_verisign_query(d).await {
            Ok(p) => results.push(p),
            Err(e) => errors.push(format!("verisign-rdap: {}", e)),
        }
    }

    let error = if errors.is_empty() {
        None
    } else {
        Some(errors.join("；"))
    };
    Ok(MultiSourceResult {
        domain: d.to_string(),
        results,
        error,
    })
}

async fn whois_from_server(domain: String, server: String) -> Result<WhoisParsed, String> {
    let d = domain.trim();
    if d.is_empty() {
        return Err("域名不能为空".to_string());
    }
    let s = server.trim();
    if s.is_empty() {
        return Err("WHOIS 服务器不能为空".to_string());
    }
    let d_owned = d.to_string();
    let s_owned = s.to_string();
    let text = tokio::task::spawn_blocking(move || query_whois_server(&s_owned, &d_owned))
        .await
        .map_err(|_| "WHOIS 查询线程执行失败".to_string())??;
    Ok(parse_whois_text(d, s, &text))
}

#[tauri::command]
pub async fn query_whois_unified(
    domain: String,
    source: Option<String>,
) -> Result<WhoisParsed, String> {
    let d = domain.trim();
    if d.is_empty() {
        return Err("域名不能为空".to_string());
    }
    let mode = source.unwrap_or_else(|| "auto".to_string());
    if mode.eq_ignore_ascii_case("auto") {
        // Prefer TLD-specific server via IANA referral, then fallback list
        let mut servers: Vec<String> = Vec::new();
        if let Some(tld) = extract_tld(d) {
            if let Some(s) = resolve_whois_server_for_tld(&tld) {
                servers.push(s);
            }
        }
        // Append common fallbacks
        servers.push("whois.verisign-grs.com".to_string());
        servers.push("grs-whois.cndns.com".to_string());
        servers.push("grs-whois.hichina.com".to_string());
        let mut last_err: Option<String> = None;
        for s in servers.iter() {
            match whois_from_server(d.to_string(), s.to_string()).await {
                Ok(parsed) => return Ok(parsed),
                Err(e) => {
                    last_err = Some(e);
                    continue;
                }
            }
        }
        Err(last_err.unwrap_or_else(|| "自动 WHOIS 查询失败".to_string()))
    } else {
        whois_from_server(d.to_string(), mode).await
    }
}
