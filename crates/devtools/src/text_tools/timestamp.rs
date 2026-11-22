//! 时间戳与日期时间互转纯逻辑。
//!
//! 契约见 docs/plans/tool-specs/batch1.md「TimestampConverter」节：
//! - ts→dt：前导数字解析 → 按单位换算毫秒 → 范围校验 [0, 9999999999999]
//!   → 输出 zh-CN 本地化格式（对齐 JS `toLocaleString('zh-CN', { timeZone })`，
//!   即 `YYYY/M/D HH:mm:ss`：月/日不补零、时分秒补两位）
//! - dt→ts：RFC3339 与 `2026-11-21 14:30:05`、`2026/11/21 14:30:05` 等常见格式
//!   多级 fallback；无时区信息时按本地时区解释（对齐 JS `new Date(input)`）
//!
//! 错误文案必须与历史版本一致。

use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, SecondsFormat, TimeZone as _, Utc};
use chrono_tz::Tz;

/// 历史版本判定「合理范围」的毫秒上限（13 位毫秒时间戳）。
const MAX_REASONABLE_MS: i64 = 9_999_999_999_999;

/// 当前时间，返回 `(秒, 毫秒)`。
pub fn now_ts() -> (i64, i64) {
    let now = Utc::now();
    (now.timestamp(), now.timestamp_millis())
}

/// 当前时间的 ISO8601 字符串（UTC、毫秒精度、`Z` 结尾），对齐 JS `new Date().toISOString()`。
pub fn now_iso8601() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

/// 全量时区名列表（chrono-tz tz 数据库，597 个），按字典序排序。
pub fn all_timezones() -> Vec<String> {
    let mut names: Vec<String> = chrono_tz::TZ_VARIANTS
        .iter()
        .map(|tz| tz.name().to_string())
        .collect();
    names.sort_unstable();
    names
}

/// 毫秒时间戳 → 目标时区的中文本地化字符串。
///
/// 输出格式 `YYYY/M/D HH:mm:ss`，对齐 JS `toLocaleString('zh-CN', { timeZone })`：
/// 月/日不补零（`2026/11/21`），时分秒补两位（`09:05:03`）。
pub fn ts_to_localized(ms_millisec: u64, timezone: &str) -> Result<String, String> {
    let tz: Tz = timezone
        .parse()
        .map_err(|_| format!("未知时区: {}", timezone))?;
    let secs = (ms_millisec / 1000) as i64;
    let nanos = ((ms_millisec % 1000) * 1_000_000) as u32;
    let dt =
        DateTime::from_timestamp(secs, nanos).ok_or_else(|| "时间戳超出合理范围".to_string())?;
    Ok(dt
        .with_timezone(&tz)
        .format("%Y/%-m/%-d %H:%M:%S")
        .to_string())
}

/// 日期时间字符串 → `(毫秒时间戳, 秒时间戳)`；秒为毫秒向负无穷取整（对齐 JS `Math.floor`）。
///
/// 支持 RFC3339（含时区偏移），以及 `2026-11-21 14:30:05`、`2026/11/21 14:30:05`、
/// `2026-11-21T14:30`、`2026-11-21` 等常见格式（多格式 fallback）；
/// 无时区信息时按本地时区解释（对齐 JS `new Date(input)`）。
pub fn dt_to_ts(input: &str) -> Result<(i64, i64), String> {
    let s = input.trim();
    if s.is_empty() {
        return Err("请输入有效的日期时间格式".into());
    }

    // 带时区偏移的 ISO8601（RFC3339）
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        let ms = dt.timestamp_millis();
        return Ok((ms, ms.div_euclid(1000)));
    }

    // 无时区信息：按本地时区解释（对齐 JS new Date(input)）
    const DATETIME_FORMATS: [&str; 6] = [
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y/%m/%d %H:%M:%S%.f",
        "%Y-%m-%dT%H:%M",
        "%Y-%m-%d %H:%M",
        "%Y/%m/%d %H:%M",
    ];
    for fmt in DATETIME_FORMATS {
        if let Ok(naive) = NaiveDateTime::parse_from_str(s, fmt) {
            let ms = naive_to_local_ms(naive);
            return Ok((ms, ms.div_euclid(1000)));
        }
    }

    // 纯日期：本地时区零点
    const DATE_FORMATS: [&str; 2] = ["%Y-%m-%d", "%Y/%m/%d"];
    for fmt in DATE_FORMATS {
        if let Ok(date) = NaiveDate::parse_from_str(s, fmt) {
            let midnight = date.and_hms_opt(0, 0, 0).expect("valid midnight");
            let ms = naive_to_local_ms(midnight);
            return Ok((ms, ms.div_euclid(1000)));
        }
    }

    Err("请输入有效的日期时间格式".into())
}

/// 把无时区的日期时间按本地时区解析为毫秒时间戳。
fn naive_to_local_ms(naive: NaiveDateTime) -> i64 {
    let local = Local.from_local_datetime(&naive);
    let dt = local.single().or_else(|| local.earliest());
    match dt {
        Some(dt) => dt.timestamp_millis(),
        // 本地时区里不存在的时刻（夏令时跳变）按 UTC 解释兜底
        None => naive.and_utc().timestamp_millis(),
    }
}

/// 宽松整数解析（十进制）语义：跳过前导空白，可选正负号，取前导数字，遇非数字停止。
pub fn parse_int(s: &str) -> Option<i64> {
    let t = s.trim_start();
    let negative = t.starts_with('-');
    let digits_part = t
        .strip_prefix('+')
        .or_else(|| t.strip_prefix('-'))
        .unwrap_or(t);
    let mut acc: i64 = 0;
    let mut any = false;
    for c in digits_part.chars() {
        match c.to_digit(10) {
            Some(d) => {
                any = true;
                acc = acc.saturating_mul(10).saturating_add(d as i64);
            }
            None => break,
        }
    }
    if any {
        Some(if negative { -acc } else { acc })
    } else {
        None
    }
}

/// 统一转换入口（对应历史版本 handleConvert）。
///
/// `to_datetime=true`：时间戳 → 日期时间（`is_millis` 决定输入单位为秒/毫秒）；
/// `to_datetime=false`：日期时间 → 时间戳（`is_millis` 决定输出毫秒/秒）。
/// `timezone` 仅在 ts→dt 方向生效。错误文案与历史版本一致。
pub fn convert(
    value: &str,
    to_datetime: bool,
    is_millis: bool,
    timezone: &str,
) -> Result<String, String> {
    if value.is_empty() {
        return Err("请输入值".into());
    }
    if to_datetime {
        let Some(ts) = parse_int(value) else {
            return Err("请输入有效数字".into());
        };
        let ms = if is_millis {
            ts
        } else {
            ts.saturating_mul(1000)
        };
        if !(0..=MAX_REASONABLE_MS).contains(&ms) {
            return Err("时间戳超出合理范围".into());
        }
        ts_to_localized(ms as u64, timezone)
    } else {
        let (ms, sec) = dt_to_ts(value)?;
        Ok(if is_millis {
            ms.to_string()
        } else {
            sec.to_string()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2024-01-01T00:00:00Z 的毫秒时间戳（测试锚点）。
    const ANCHOR_MS: i64 = 1_704_067_200_000;

    #[test]
    fn now_ts_seconds_and_millis_agree() {
        let (sec, ms) = now_ts();
        assert!(sec > 0);
        assert!(ms >= sec * 1000 && ms < (sec + 1) * 1000);
    }

    #[test]
    fn now_iso8601_has_z_suffix_and_millis() {
        let s = now_iso8601();
        assert!(s.ends_with('Z'), "actual: {s}");
        assert!(s.contains('.'));
    }

    #[test]
    fn ts_to_localized_utc_epoch() {
        assert_eq!(ts_to_localized(0, "UTC").unwrap(), "1970/1/1 00:00:00");
    }

    #[test]
    fn ts_to_localized_zone_offsets() {
        // 各时区对同一时刻的本地化结果（月/日不补零，时分秒补两位）
        assert_eq!(
            ts_to_localized(ANCHOR_MS as u64, "UTC").unwrap(),
            "2024/1/1 00:00:00"
        );
        assert_eq!(
            ts_to_localized(ANCHOR_MS as u64, "Asia/Shanghai").unwrap(),
            "2024/1/1 08:00:00"
        );
        assert_eq!(
            ts_to_localized(ANCHOR_MS as u64, "America/New_York").unwrap(),
            "2023/12/31 19:00:00"
        );
        assert_eq!(
            ts_to_localized(ANCHOR_MS as u64, "Australia/Sydney").unwrap(),
            "2024/1/1 11:00:00"
        );
    }

    #[test]
    fn ts_to_localized_unknown_timezone() {
        let err = ts_to_localized(0, "Mars/Olympus").unwrap_err();
        assert_eq!(err, "未知时区: Mars/Olympus");
    }

    #[test]
    fn dt_to_ts_rfc3339_utc() {
        assert_eq!(
            dt_to_ts("2024-01-01T00:00:00Z").unwrap(),
            (ANCHOR_MS, 1_704_067_200)
        );
    }

    #[test]
    fn dt_to_ts_rfc3339_with_offset() {
        assert_eq!(
            dt_to_ts("2024-01-01T08:00:00+08:00").unwrap(),
            (ANCHOR_MS, 1_704_067_200)
        );
    }

    #[test]
    fn dt_to_ts_fractional_seconds() {
        assert_eq!(
            dt_to_ts("2024-01-01T00:00:00.5Z").unwrap(),
            (ANCHOR_MS + 500, 1_704_067_200)
        );
    }

    #[test]
    fn dt_to_ts_floor_for_negative_seconds() {
        // -500ms → 秒按 Math.floor 取整为 -1
        assert_eq!(dt_to_ts("1969-12-31T23:59:59.5Z").unwrap(), (-500, -1));
    }

    #[test]
    fn dt_to_ts_naive_formats_agree_across_separators() {
        // 无时区格式按本地时区解释，同一时刻的三种写法必须一致（不依赖测试机器时区）
        let dash = dt_to_ts("2024-01-01 12:34:56").unwrap();
        let slash = dt_to_ts("2024/01/01 12:34:56").unwrap();
        let tee = dt_to_ts("2024-01-01T12:34:56").unwrap();
        assert_eq!(dash, slash);
        assert_eq!(dash, tee);
    }

    #[test]
    fn dt_to_ts_minute_precision_and_date_only_agree() {
        // 省略秒的格式与纯日期格式均可解析，同一天内写法互相一致
        let a = dt_to_ts("2024-01-01 00:00").unwrap();
        let b = dt_to_ts("2024/01/01 00:00").unwrap();
        assert_eq!(a, b);
        assert_eq!(a.1 % 60, 0); // 分钟精度
        assert_eq!(
            dt_to_ts("2024-01-01").unwrap(),
            dt_to_ts("2024/01/01").unwrap()
        );
    }

    #[test]
    fn dt_to_ts_invalid() {
        assert_eq!(
            dt_to_ts("not-a-date").unwrap_err(),
            "请输入有效的日期时间格式"
        );
        assert_eq!(dt_to_ts("").unwrap_err(), "请输入有效的日期时间格式");
        assert_eq!(
            dt_to_ts("2024-13-40").unwrap_err(),
            "请输入有效的日期时间格式"
        );
    }

    #[test]
    fn parse_int_leading_digits_semantics() {
        assert_eq!(parse_int(" 1700000000abc"), Some(1_700_000_000));
        assert_eq!(parse_int("12.9"), Some(12));
        assert_eq!(parse_int("-7x"), Some(-7));
        assert_eq!(parse_int("+7"), Some(7));
        assert_eq!(parse_int("007"), Some(7));
        assert_eq!(parse_int("abc"), None);
        assert_eq!(parse_int(""), None);
        assert_eq!(parse_int("   "), None);
    }

    #[test]
    fn parse_int_saturates_huge_input() {
        // 超长数字饱和到 i64 边界（交由范围校验报错）
        assert_eq!(parse_int("99999999999999999999999"), Some(i64::MAX));
    }

    #[test]
    fn convert_ts_to_datetime() {
        assert_eq!(
            convert("1704067200", true, false, "UTC").unwrap(),
            "2024/1/1 00:00:00"
        );
        assert_eq!(
            convert("1704067200000", true, true, "Asia/Shanghai").unwrap(),
            "2024/1/1 08:00:00"
        );
    }

    #[test]
    fn convert_dt_to_timestamp_ms_and_seconds() {
        assert_eq!(
            convert("2024-01-01T00:00:00Z", false, true, "UTC").unwrap(),
            "1704067200000"
        );
        assert_eq!(
            convert("2024-01-01T00:00:00Z", false, false, "UTC").unwrap(),
            "1704067200"
        );
    }

    #[test]
    fn convert_empty_value() {
        assert_eq!(convert("", true, true, "UTC").unwrap_err(), "请输入值");
    }

    #[test]
    fn convert_invalid_number() {
        assert_eq!(
            convert("abc", true, true, "UTC").unwrap_err(),
            "请输入有效数字"
        );
        assert_eq!(
            convert("  ", true, true, "UTC").unwrap_err(),
            "请输入有效数字"
        );
    }

    #[test]
    fn convert_out_of_range() {
        // 13 位上限之上 / 负数 / 秒数 ×1000 溢出
        assert_eq!(
            convert("10000000000000", true, true, "UTC").unwrap_err(),
            "时间戳超出合理范围"
        );
        assert_eq!(
            convert("100000000000000", true, false, "UTC").unwrap_err(),
            "时间戳超出合理范围"
        );
        assert_eq!(
            convert("-1", true, true, "UTC").unwrap_err(),
            "时间戳超出合理范围"
        );
    }

    #[test]
    fn convert_range_boundaries_accepted() {
        // 上边界 9999999999999（2286-11-20）与 0 均合法
        assert!(convert("9999999999999", true, true, "UTC").is_ok());
        assert!(convert("0", true, true, "UTC").is_ok());
    }

    #[test]
    fn all_timezones_contains_common_zones_and_sorted() {
        let zones = all_timezones();
        assert!(zones.len() >= 500);
        assert!(zones.iter().any(|z| z == "Asia/Shanghai"));
        assert!(zones.iter().any(|z| z == "UTC"));
        let mut sorted = zones.clone();
        sorted.sort_unstable();
        assert_eq!(zones, sorted);
    }
}
