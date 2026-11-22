//! 子网计算纯逻辑。
//!
//! 契约见 docs/plans/tool-specs/batch1.md「SubnetCalculator」节：
//! - 版本检测：IP 部分含 `.` → IPv4，含 `:` → IPv6；
//! - IPv4 支持 IP/CIDR 与 IP/子网掩码 两种输入；IPv6 仅 CIDR；
//! - 数字解析沿用宽松整数解析语义（见 [`super::timestamp::parse_int`]）；
//! - IPv6 总地址数：主机位 ≤53 时输出千分位数值，否则输出 `2^n`；
//! - 全部错误文案与历史版本一字不差。

use super::timestamp::parse_int;

/// IP 版本选择（auto 默认 / ipv4 / ipv6）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IpVersion {
    /// 自动检测（默认）。
    #[default]
    Auto,
    /// 强制按 IPv4 校验。
    V4,
    /// 强制按 IPv6 校验。
    V6,
}

/// 计算结果，字段与输出卡各行一一对应（IPv6 专属字段在不适用时为空串）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SubnetResult {
    /// "IPv4" 或 "IPv6"。
    pub version: String,
    /// "/24" 形式。
    pub cidr: String,
    /// 子网掩码（仅 IPv4）。
    pub mask: String,
    /// 网络地址。
    pub network: String,
    /// 广播地址（仅 IPv4）。
    pub broadcast: String,
    /// IPv4 为第一个可用 IP（/31、/32 显示 N/A）；IPv6 为网络前缀（即网络地址）。
    pub first_ip: String,
    /// IPv4 为最后一个可用 IP（/31、/32 显示 N/A）；IPv6 固定文案「计算复杂」。
    pub last_ip: String,
    /// 总 IP 数 / 总地址数（千分位，或 `2^n`）。
    pub total_ips: String,
    /// 可用 IP 数 / 可用地址数。
    pub usable_ips: String,
    /// 前缀 "/64"（仅 IPv6）。
    pub prefix: String,
    /// 网络部分（展开后前 ceil(prefix/16) 组，仅 IPv6）。
    pub network_portion: String,
    /// 主机部分（其余组；可为空串，由 UI 显示 N/A，仅 IPv6）。
    pub host_portion: String,
}

/// 计算入口：解析 `IP/CIDR` 或 `IP/子网掩码`，按指定版本（或自动检测）计算子网信息。
/// 错误文案与历史版本一致。
pub fn calculate(input: &str, selected: IpVersion) -> Result<SubnetResult, String> {
    if input.is_empty() {
        return Err("请输入IP地址和CIDR或子网掩码".into());
    }
    let Some((ip_part, suffix)) = input.split_once('/') else {
        return Err("输入格式应为 IP/CIDR 或 IP/子网掩码".into());
    };

    let detected = detect_ip_version(ip_part);
    let Some(detected) = detected else {
        return Err("无法识别IP地址版本".into());
    };

    if selected != IpVersion::Auto && selected != detected {
        return Err(format!(
            "IP地址版本不匹配：检测到{}，但选择了{}",
            version_label(detected),
            version_label(selected)
        ));
    }

    match detected {
        IpVersion::V4 => calculate_ipv4(ip_part, suffix),
        IpVersion::V6 => calculate_ipv6(ip_part, suffix),
        IpVersion::Auto => unreachable!("detect_ip_version 不返回 Auto"),
    }
}

fn detect_ip_version(ip_part: &str) -> Option<IpVersion> {
    if ip_part.contains('.') {
        Some(IpVersion::V4)
    } else if ip_part.contains(':') {
        Some(IpVersion::V6)
    } else {
        None
    }
}

fn version_label(version: IpVersion) -> &'static str {
    match version {
        IpVersion::V4 => "IPv4",
        IpVersion::V6 => "IPv6",
        IpVersion::Auto => "auto",
    }
}

// ---------------------------------------------------------------------------
// IPv4
// ---------------------------------------------------------------------------

/// 校验点分十进制 IPv4（每段 0-255，允许前导零）。
fn is_valid_ipv4(ip: &str) -> bool {
    let parts: Vec<&str> = ip.split('.').collect();
    if parts.len() != 4 {
        return false;
    }
    parts.into_iter().all(|p| {
        !p.is_empty()
            && p.chars().all(|c| c.is_ascii_digit())
            && p.chars().count() <= 3
            && p.parse::<u16>().map(|v| v <= 255).unwrap_or(false)
    })
}

/// CIDR → 点分十进制掩码（每段按剩余前缀位数取高位 1）。
fn cidr_to_mask(cidr: u32) -> String {
    let octets: Vec<String> = (0..4)
        .map(|i| {
            let bits = (cidr.saturating_sub(i * 8)).min(8);
            ((0xffu16 << (8 - bits)) & 0xff) as u8
        })
        .map(|v| v.to_string())
        .collect();
    octets.join(".")
}

/// 点分十进制掩码 → CIDR（统计各段二进制中 1 的个数，不校验连续性）。
fn mask_to_cidr(mask: &str) -> u32 {
    mask.split('.')
        .filter_map(|p| p.parse::<u8>().ok())
        .map(|p| p.count_ones())
        .sum()
}

fn calculate_ipv4(ip: &str, suffix: &str) -> Result<SubnetResult, String> {
    if !is_valid_ipv4(ip) {
        return Err("无效的IPv4地址".into());
    }
    let (cidr, mask) = if suffix.contains('.') {
        // 子网掩码格式
        if !is_valid_ipv4(suffix) {
            return Err("无效的子网掩码".into());
        }
        let cidr = mask_to_cidr(suffix);
        (cidr, suffix.to_string())
    } else {
        // CIDR 格式
        let Some(cidr) = parse_int(suffix).filter(|c| (0..=32).contains(c)) else {
            return Err("无效的CIDR值 (IPv4: 0-32)".into());
        };
        (cidr as u32, cidr_to_mask(cidr as u32))
    };

    let ip_parts: Vec<u16> = ip.split('.').filter_map(|p| p.parse().ok()).collect();
    let mask_parts: Vec<u16> = mask.split('.').filter_map(|p| p.parse().ok()).collect();
    let network_parts: Vec<u16> = ip_parts
        .iter()
        .zip(&mask_parts)
        .map(|(a, m)| a & m)
        .collect();
    let broadcast_parts: Vec<u16> = network_parts
        .iter()
        .zip(&mask_parts)
        .map(|(n, m)| n | (255 - m))
        .collect();

    let total = 1u64 << (32 - cidr);
    let usable = if cidr == 32 {
        1
    } else {
        total.saturating_sub(2)
    };

    // 第一个/最后一个可用 IP（仅对末段 ±1；/31、/32 显示 N/A）
    let mut first_parts = network_parts.clone();
    let mut last_parts = broadcast_parts.clone();
    if cidr < 31 {
        let n = first_parts.len() - 1;
        first_parts[n] += 1;
        last_parts[n] -= 1;
    }
    let join_parts = |parts: &[u16]| {
        parts
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(".")
    };

    Ok(SubnetResult {
        version: "IPv4".into(),
        cidr: format!("/{}", cidr),
        mask,
        network: join_parts(&network_parts),
        broadcast: join_parts(&broadcast_parts),
        first_ip: if cidr < 31 {
            join_parts(&first_parts)
        } else {
            "N/A".into()
        },
        last_ip: if cidr < 31 {
            join_parts(&last_parts)
        } else {
            "N/A".into()
        },
        total_ips: with_thousands(total),
        usable_ips: with_thousands(usable),
        prefix: String::new(),
        network_portion: String::new(),
        host_portion: String::new(),
    })
}

// ---------------------------------------------------------------------------
// IPv6
// ---------------------------------------------------------------------------

/// 校验 IPv6（含历史契约的全部边界规则）。
fn is_valid_ipv6(ip: &str) -> bool {
    // 基本格式检查：仅十六进制字符和冒号
    if ip.is_empty() || !ip.chars().all(|c| c.is_ascii_hexdigit() || c == ':') {
        return false;
    }
    // 特殊情况 ::（全零地址）
    if ip == "::" {
        return true;
    }
    // 双冒号最多出现一次
    if ip.matches("::").count() > 1 {
        return false;
    }
    // 不能以单个冒号开始或结束（双冒号可以）
    if (ip.starts_with(':') && !ip.starts_with("::")) || (ip.ends_with(':') && !ip.ends_with("::"))
    {
        return false;
    }

    let parts: Vec<&str> = if let Some(ix) = ip.find("::") {
        let left = &ip[..ix];
        let right = &ip[ix + 2..];
        let mut left: Vec<&str> = if left.is_empty() {
            Vec::new()
        } else {
            left.split(':').filter(|p| !p.is_empty()).collect()
        };
        let right: Vec<&str> = if right.is_empty() {
            Vec::new()
        } else {
            right.split(':').filter(|p| !p.is_empty()).collect()
        };
        // 压缩格式：左右组数合计不能超过 8
        if left.len() + right.len() >= 8 {
            return false;
        }
        left.extend(right);
        left
    } else {
        // 完整格式：必须有 8 组且不能有空组
        let parts: Vec<&str> = ip.split(':').collect();
        if parts.len() != 8 || parts.iter().any(|p| p.is_empty()) {
            return false;
        }
        parts
    };

    parts
        .iter()
        .all(|p| p.len() <= 4 && p.chars().all(|c| c.is_ascii_hexdigit()))
}

/// 展开压缩形式为完整 8 组（每组补足 4 位）。
fn expand_ipv6(ip: &str) -> String {
    let mut groups: Vec<String> = Vec::with_capacity(8);
    if let Some(ix) = ip.find("::") {
        let left = &ip[..ix];
        let right = &ip[ix + 2..];
        if !left.is_empty() {
            groups.extend(left.split(':').map(String::from));
        }
        if !right.is_empty() {
            let right: Vec<String> = right.split(':').map(String::from).collect();
            let missing = 8 - groups.len() - right.len();
            groups.extend(std::iter::repeat_n("0000".to_string(), missing));
            groups.extend(right);
        } else {
            let missing = 8 - groups.len();
            groups.extend(std::iter::repeat_n("0000".to_string(), missing));
        }
    } else {
        groups.extend(ip.split(':').map(String::from));
    }
    groups
        .iter()
        .map(|g| format!("{:0>4}", g))
        .collect::<Vec<_>>()
        .join(":")
}

/// 按前缀长度计算网络地址：保留前 prefix 位，其余置 0，输出完整 8 组小写 hex。
fn ipv6_network(ip: &str, prefix: u32) -> String {
    let expanded = expand_ipv6(ip);
    let mut binary = String::with_capacity(128);
    for group in expanded.split(':') {
        let value = u16::from_str_radix(group, 16).unwrap_or(0);
        binary.push_str(&format!("{:016b}", value));
    }
    let prefix = (prefix as usize).min(128);
    let mut network_binary: String = binary.chars().take(prefix).collect();
    network_binary.extend(std::iter::repeat_n('0', 128 - prefix));

    let groups: Vec<String> = (0..8)
        .map(|i| {
            let chunk = &network_binary[i * 16..(i + 1) * 16];
            let value = u32::from_str_radix(chunk, 2).unwrap_or(0);
            format!("{:04x}", value)
        })
        .collect();
    groups.join(":")
}

fn calculate_ipv6(ip: &str, suffix: &str) -> Result<SubnetResult, String> {
    if !is_valid_ipv6(ip) {
        return Err("无效的IPv6地址".into());
    }
    let Some(prefix) = parse_int(suffix) else {
        return Err("IPv6只支持CIDR格式 (例如: /64)".into());
    };
    if !(0..=128).contains(&prefix) {
        return Err("无效的前缀长度 (IPv6: 0-128)".into());
    }
    let prefix = prefix as u32;

    let expanded = expand_ipv6(ip);
    let network = ipv6_network(ip, prefix);

    let host_bits = 128 - prefix;
    // JS Number 可精确表示 ≤2^53 的整数；更大的主机位只显示 2^n
    let total_string = if host_bits <= 53 {
        with_thousands(1u64 << host_bits)
    } else {
        format!("2^{}", host_bits)
    };

    // 网络部分/主机部分：以展开后的组数按 ceil(prefix/16) 切分
    let groups: Vec<&str> = expanded.split(':').collect();
    let boundary = prefix.div_ceil(16) as usize;
    let network_portion = groups[..boundary.min(groups.len())].join(":");
    let host_portion = groups[boundary.min(groups.len())..].join(":");

    Ok(SubnetResult {
        version: "IPv6".into(),
        cidr: format!("/{}", prefix),
        mask: String::new(),
        network: network.clone(),
        broadcast: String::new(),
        first_ip: network,
        last_ip: "计算复杂".into(),
        total_ips: total_string.clone(),
        usable_ips: total_string,
        prefix: format!("/{}", prefix),
        network_portion,
        host_portion,
    })
}

// ---------------------------------------------------------------------------
// 通用
// ---------------------------------------------------------------------------

/// 千分位（逗号每 3 位分组），对齐 JS `Number.prototype.toLocaleString` 的默认分组。
fn with_thousands(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    let len = digits.len();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (len - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn calc(input: &str) -> Result<SubnetResult, String> {
        calculate(input, IpVersion::Auto)
    }

    #[test]
    fn ipv4_cidr_24() {
        let r = calc("192.168.1.0/24").unwrap();
        assert_eq!(r.version, "IPv4");
        assert_eq!(r.cidr, "/24");
        assert_eq!(r.mask, "255.255.255.0");
        assert_eq!(r.network, "192.168.1.0");
        assert_eq!(r.broadcast, "192.168.1.255");
        assert_eq!(r.first_ip, "192.168.1.1");
        assert_eq!(r.last_ip, "192.168.1.254");
        assert_eq!(r.total_ips, "256");
        assert_eq!(r.usable_ips, "254");
        assert_eq!(r.prefix, "");
        assert_eq!(r.network_portion, "");
        assert_eq!(r.host_portion, "");
    }

    #[test]
    fn ipv4_subnet_mask_input() {
        let r = calc("10.0.0.0/255.0.0.0").unwrap();
        assert_eq!(r.cidr, "/8");
        assert_eq!(r.mask, "255.0.0.0");
        assert_eq!(r.network, "10.0.0.0");
        assert_eq!(r.broadcast, "10.255.255.255");
        assert_eq!(r.first_ip, "10.0.0.1");
        assert_eq!(r.last_ip, "10.255.255.254");
        assert_eq!(r.total_ips, "16,777,216");
        assert_eq!(r.usable_ips, "16,777,214");
    }

    #[test]
    fn ipv4_network_offset_within_supernet() {
        let r = calc("192.168.1.77/26").unwrap();
        assert_eq!(r.cidr, "/26");
        assert_eq!(r.mask, "255.255.255.192");
        assert_eq!(r.network, "192.168.1.64");
        assert_eq!(r.broadcast, "192.168.1.127");
        assert_eq!(r.first_ip, "192.168.1.65");
        assert_eq!(r.last_ip, "192.168.1.126");
        assert_eq!(r.total_ips, "64");
        assert_eq!(r.usable_ips, "62");
    }

    #[test]
    fn ipv4_slash_31_boundary() {
        let r = calc("1.2.3.4/31").unwrap();
        assert_eq!(r.network, "1.2.3.4");
        assert_eq!(r.broadcast, "1.2.3.5");
        assert_eq!(r.mask, "255.255.255.254");
        assert_eq!(r.total_ips, "2");
        assert_eq!(r.usable_ips, "0");
        assert_eq!(r.first_ip, "N/A");
        assert_eq!(r.last_ip, "N/A");
    }

    #[test]
    fn ipv4_slash_32_boundary() {
        let r = calc("1.2.3.4/32").unwrap();
        assert_eq!(r.cidr, "/32");
        assert_eq!(r.network, "1.2.3.4");
        assert_eq!(r.broadcast, "1.2.3.4");
        assert_eq!(r.mask, "255.255.255.255");
        assert_eq!(r.total_ips, "1");
        assert_eq!(r.usable_ips, "1");
        assert_eq!(r.first_ip, "N/A");
        assert_eq!(r.last_ip, "N/A");
    }

    #[test]
    fn ipv4_cidr_zero() {
        let r = calc("1.2.3.4/0").unwrap();
        assert_eq!(r.mask, "0.0.0.0");
        assert_eq!(r.network, "0.0.0.0");
        assert_eq!(r.broadcast, "255.255.255.255");
        assert_eq!(r.total_ips, "4,294,967,296");
        assert_eq!(r.usable_ips, "4,294,967,294");
    }

    #[test]
    fn ipv6_slash_64() {
        let r = calc("2001:db8::/64").unwrap();
        assert_eq!(r.version, "IPv6");
        assert_eq!(r.cidr, "/64");
        assert_eq!(r.prefix, "/64");
        assert_eq!(r.mask, "");
        assert_eq!(r.broadcast, "");
        assert_eq!(r.network, "2001:0db8:0000:0000:0000:0000:0000:0000");
        assert_eq!(r.first_ip, r.network);
        assert_eq!(r.last_ip, "计算复杂");
        assert_eq!(r.total_ips, "2^64");
        assert_eq!(r.usable_ips, "2^64");
        assert_eq!(r.network_portion, "2001:0db8:0000:0000");
        assert_eq!(r.host_portion, "0000:0000:0000:0000");
    }

    #[test]
    fn ipv6_loopback_slash_128() {
        let r = calc("::1/128").unwrap();
        assert_eq!(r.network, "0000:0000:0000:0000:0000:0000:0000:0001");
        assert_eq!(r.total_ips, "1");
        assert_eq!(r.usable_ips, "1");
        assert_eq!(r.network_portion, "0000:0000:0000:0000:0000:0000:0000:0001");
        assert_eq!(r.host_portion, "");
    }

    #[test]
    fn ipv6_fe80_slash_10() {
        let r = calc("fe80::/10").unwrap();
        assert_eq!(r.network, "fe80:0000:0000:0000:0000:0000:0000:0000");
        assert_eq!(r.total_ips, "2^118");
        assert_eq!(r.network_portion, "fe80");
        assert_eq!(r.host_portion, "0000:0000:0000:0000:0000:0000:0000");
    }

    #[test]
    fn ipv6_compressed_host_bits() {
        let r = calc("2001:db8::1/64").unwrap();
        assert_eq!(r.network, "2001:0db8:0000:0000:0000:0000:0000:0000");
        // ceil(64/16)=4 → 网络部分 4 组、主机部分其余 4 组
        assert_eq!(r.network_portion, "2001:0db8:0000:0000");
        assert_eq!(r.host_portion, "0000:0000:0000:0001");
    }

    #[test]
    fn ipv6_full_form_and_uppercase() {
        let r = calc("2001:0DB8:0000:0000:0000:0000:0000:0001/64").unwrap();
        assert_eq!(r.network, "2001:0db8:0000:0000:0000:0000:0000:0000");
        // 保留原始大小写：网络地址小写，网络/主机部分保留输入大小写
        assert_eq!(r.network_portion, "2001:0DB8:0000:0000");
        assert_eq!(r.host_portion, "0000:0000:0000:0001");
    }

    #[test]
    fn ipv6_all_zero_network() {
        let r = calc("::/0").unwrap();
        assert_eq!(r.cidr, "/0");
        assert_eq!(r.network, "0000:0000:0000:0000:0000:0000:0000:0000");
        assert_eq!(r.total_ips, "2^128");
        assert_eq!(r.network_portion, "");
        assert_eq!(r.host_portion, "0000:0000:0000:0000:0000:0000:0000:0000");
    }

    #[test]
    fn errors_verbatim() {
        assert_eq!(calc("").unwrap_err(), "请输入IP地址和CIDR或子网掩码");
        assert_eq!(
            calc("192.168.1.0").unwrap_err(),
            "输入格式应为 IP/CIDR 或 IP/子网掩码"
        );
        assert_eq!(calc("abc/24").unwrap_err(), "无法识别IP地址版本");
        assert_eq!(
            calculate("192.168.1.0/24", IpVersion::V6).unwrap_err(),
            "IP地址版本不匹配：检测到IPv4，但选择了IPv6"
        );
        assert_eq!(
            calculate("2001:db8::/64", IpVersion::V4).unwrap_err(),
            "IP地址版本不匹配：检测到IPv6，但选择了IPv4"
        );
        assert_eq!(calc("999.1.1.1/24").unwrap_err(), "无效的IPv4地址");
        assert_eq!(calc("1.2.3.4/255.0.0").unwrap_err(), "无效的子网掩码");
        assert_eq!(calc("1.2.3.4/33").unwrap_err(), "无效的CIDR值 (IPv4: 0-32)");
        assert_eq!(calc("1.2.3.4/").unwrap_err(), "无效的CIDR值 (IPv4: 0-32)");
        assert_eq!(calc("gg::1/64").unwrap_err(), "无效的IPv6地址");
        assert_eq!(calc("1:2:3:4:5:6:7:8::/64").unwrap_err(), "无效的IPv6地址");
        assert_eq!(
            calc("2001:db8::1/abc").unwrap_err(),
            "IPv6只支持CIDR格式 (例如: /64)"
        );
        assert_eq!(
            calc("2001:db8::/129").unwrap_err(),
            "无效的前缀长度 (IPv6: 0-128)"
        );
        assert_eq!(
            calc("2001:db8::/").unwrap_err(),
            "IPv6只支持CIDR格式 (例如: /64)"
        );
    }

    #[test]
    fn selected_version_passes_through() {
        // 选中版本与检测结果一致时正常计算
        let r = calculate("192.168.1.0/24", IpVersion::V4).unwrap();
        assert_eq!(r.cidr, "/24");
        let r = calculate("2001:db8::/64", IpVersion::V6).unwrap();
        assert_eq!(r.cidr, "/64");
    }

    #[test]
    fn thousands_grouping() {
        assert_eq!(with_thousands(0), "0");
        assert_eq!(with_thousands(999), "999");
        assert_eq!(with_thousands(1_000), "1,000");
        assert_eq!(
            with_thousands(9_007_199_254_740_992_u64),
            "9,007,199,254,740,992"
        );
    }

    #[test]
    fn cidr_mask_roundtrip_helpers() {
        assert_eq!(cidr_to_mask(24), "255.255.255.0");
        assert_eq!(cidr_to_mask(8), "255.0.0.0");
        assert_eq!(cidr_to_mask(0), "0.0.0.0");
        assert_eq!(cidr_to_mask(32), "255.255.255.255");
        assert_eq!(mask_to_cidr("255.255.255.0"), 24);
        assert_eq!(mask_to_cidr("255.0.0.0"), 8);
        assert_eq!(mask_to_cidr("0.0.0.0"), 0);
    }
}
