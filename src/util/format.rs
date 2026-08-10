pub fn format_bytes(bytes: u64) -> String {
    if bytes == 0 {
        return "0 B".to_string();
    }
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut n = bytes as f64;
    let mut i = 0;
    while n >= 1024.0 && i < units.len() - 1 {
        n /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{} {}", bytes, units[0])
    } else {
        format!("{:.2} {}", n, units[i])
    }
}

pub fn parse_size(s: &str) -> u64 {
    let s = s.trim();
    // Match number + optional space + unit (KiB, MiB, GiB, TiB, KB, MB, GB, TB)
    let lower = s.to_lowercase();
    let chars: Vec<char> = lower.chars().collect();
    let mut num_end = 0;
    for (i, &c) in chars.iter().enumerate() {
        if c.is_ascii_digit() || c == '.' {
            num_end = i + 1;
        } else {
            break;
        }
    }
    if num_end == 0 {
        return 0;
    }
    let num: f64 = match s[..num_end].parse() {
        Ok(n) => n,
        Err(_) => return 0,
    };
    let unit = lower[num_end..].trim();
    let multiplier: u64 = match unit {
        "b" => 1,
        "kib" => 1024,
        "mib" => 1024_u64.pow(2),
        "gib" => 1024_u64.pow(3),
        "tib" => 1024_u64.pow(4),
        "kb" => 1000,
        "mb" => 1_000_000,
        "gb" => 1_000_000_000,
        "tb" => 1_000_000_000_000,
        _ => 1,
    };
    (num * multiplier as f64).round() as u64
}

pub fn format_bytes_per_sec(bytes: u64) -> String {
    if bytes == 0 {
        return String::new();
    }
    let units = ["B/s", "KB/s", "MB/s", "GB/s"];
    let mut n = bytes as f64;
    let mut i = 0;
    while n >= 1024.0 && i < units.len() - 1 {
        n /= 1024.0;
        i += 1;
    }
    if i > 0 && n < 10.0 {
        format!("{:.1} {}", n, units[i])
    } else {
        format!("{:.0} {}", n, units[i])
    }
}

pub fn format_count(n: u64) -> String {
    if n == 0 {
        return "0".to_string();
    }
    if n < 10_000 {
        return n.to_string();
    }
    let k = (n + 500) / 1000;
    if k < 1000 {
        return format!("{}k", k);
    }
    let m = n as f64 / 1_000_000.0;
    if m < 10.0 {
        let s = format!("{:.1}", m);
        return format!("{}m", s.trim_end_matches(".0"));
    }
    format!("{}m", m.round() as u64)
}

pub fn format_relative(unix_seconds: i64) -> String {
    if unix_seconds <= 0 {
        return String::new();
    }
    let now = chrono::Utc::now().timestamp();
    let diff = now - unix_seconds;
    if diff < 60 {
        return "now".to_string();
    }
    let m = diff / 60;
    if m < 60 {
        return format!("{}m ago", m);
    }
    let h = m / 60;
    if h < 24 {
        let rm = m % 60;
        if rm > 0 {
            return format!("{}hr {}m ago", h, rm);
        }
        return format!("{}hr ago", h);
    }
    let d = h / 24;
    if d < 30 {
        let rh = h % 24;
        if rh > 0 {
            return format!("{}d {}hr ago", d, rh);
        }
        return format!("{}d ago", d);
    }
    let mo = d / 30;
    if mo < 12 {
        return format!("{}mo ago", mo);
    }
    format!("{}y ago", mo / 12)
}

pub fn format_eta_short(sec: u64) -> String {
    let d = sec / 86400;
    let h = (sec % 86400) / 3600;
    let m = (sec % 3600) / 60;
    let s = sec % 60;
    if d > 0 {
        let mut parts = vec![format!("{}d", d)];
        if h > 0 {
            parts.push(format!("{}hr", h));
        }
        if m > 0 {
            parts.push(format!("{}m", m));
        }
        return parts.join(" ");
    }
    if h > 0 {
        if m > 0 {
            return format!("{}hr {}m", h, m);
        }
        return format!("{}hr", h);
    }
    if m > 0 {
        if s > 0 {
            return format!("{}m {}s", m, s);
        }
        return format!("{}m", m);
    }
    format!("{}s", s)
}

/// Strip junk code points (zero-width, emoji, control chars) and collapse whitespace.
pub fn clean_text(s: &str) -> String {
    let mut out = String::new();
    for ch in s.chars() {
        let cp = ch as u32;
        if is_junk_code_point(cp) {
            continue;
        }
        out.push(ch);
    }
    let collapsed: String = out.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        "Untitled".to_string()
    } else {
        collapsed
    }
}

fn is_junk_code_point(cp: u32) -> bool {
    cp < 0x20
        || cp == 0x7f
        || cp == 0xfffd
        || (0x200b..=0x200f).contains(&cp)
        || (0x2028..=0x202e).contains(&cp)
        || cp == 0x2060
        || cp == 0xfeff
        || cp == 0x200d
        || cp == 0xfe0f
        || cp == 0x20e3
        || (0x2600..=0x27bf).contains(&cp)
        || (0x2b00..=0x2bff).contains(&cp)
        || (0x1f000..=0x1ffff).contains(&cp)
}

/// Strip control/escape-capable characters from a string printed verbatim.
/// Preserves all other characters exactly (no whitespace collapsing).
pub fn strip_control(s: &str) -> String {
    s.chars()
        .filter(|ch| {
            let cp = *ch as u32;
            !(cp <= 0x1f || cp == 0x7f || (0x80..=0x9f).contains(&cp))
        })
        .collect()
}

pub fn truncate(s: &str, max: usize) -> String {
    if max <= 1 {
        return s.chars().take(max.saturating_sub(0)).collect();
    }
    if s.chars().count() <= max {
        return s.to_string();
    }
    let truncated: String = s.chars().take(max - 1).collect();
    format!("{}…", truncated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1_048_576), "1.00 MB");
    }

    #[test]
    fn test_parse_size() {
        assert_eq!(parse_size("1.5 GiB"), 1_610_612_736);
        assert_eq!(parse_size("500 MB"), 500_000_000);
        assert_eq!(parse_size("0"), 0);
        assert_eq!(parse_size("nonsense"), 0);
    }

    #[test]
    fn test_format_count() {
        assert_eq!(format_count(0), "0");
        assert_eq!(format_count(999), "999");
        assert_eq!(format_count(10_000), "10k");
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 8), "hello w…");
    }

    #[test]
    fn test_clean_text() {
        assert_eq!(clean_text("hello\u{200b} world"), "hello world");
        assert_eq!(clean_text(""), "Untitled");
    }
}
