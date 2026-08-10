/// Default public trackers appended to every magnet link.
pub const TRACKERS: &[&str] = &[
    "udp://tracker.opentrackr.org:1337/announce",
    "udp://open.demonii.com:1337/announce",
    "udp://tracker.openbittorrent.com:6969/announce",
    "udp://tracker.torrent.eu.org:451/announce",
    "udp://exodus.desync.com:6969/announce",
    "udp://open.stealth.si:80/announce",
    "udp://tracker.dler.org:6969/announce",
    "http://tracker.opentrackr.org:1337/announce",
    "http://tracker.openbittorrent.com:80/announce",
    "http://tracker.dler.org:6969/announce",
    "https://tracker.tamersunion.org:443/announce",
];

/// Build a magnet URI from an info hash and display name, with default trackers.
pub fn build_magnet(info_hash: &str, name: &str) -> String {
    let dn = urlencoding::encode(name);
    let tr: String = TRACKERS
        .iter()
        .map(|t| format!("&tr={}", urlencoding::encode(t)))
        .collect();
    format!("magnet:?xt=urn:btih:{}&dn={}{}", info_hash, dn, tr)
}

const BASE32_CHARS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

fn base32_to_hex(b32: &str) -> Option<String> {
    let mut bits = 0u32;
    let mut value = 0u32;
    let mut out = String::new();
    for c in b32.to_uppercase().chars() {
        let idx = BASE32_CHARS.find(c)?;
        value = (value << 5) | idx as u32;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            let byte = (value >> bits) & 0xFF;
            out.push_str(&format!("{:02x}", byte));
            value &= (1 << bits) - 1;
        }
    }
    if out.len() == 40 {
        Some(out)
    } else {
        None
    }
}

/// Normalize a 32-char base32 info hash to 40-char hex, or lowercase a hex hash.
pub fn normalize_info_hash(raw: &str) -> String {
    if raw.len() == 32 {
        base32_to_hex(raw).unwrap_or_else(|| raw.to_lowercase())
    } else {
        raw.to_lowercase()
    }
}

#[derive(Debug, Clone)]
pub struct ParsedMagnet {
    pub info_hash: String,
    pub name: String,
    pub magnet: String,
}

/// Parse a magnet URI. Returns None if it's not a valid magnet with an info hash.
pub fn parse_magnet(input: &str) -> Option<ParsedMagnet> {
    let s = input.trim();
    if !s.to_lowercase().starts_with("magnet:?") {
        return None;
    }

    // Extract info hash from xt=urn:btih:...
    let lower = s.to_lowercase();
    let xt_pos = lower.find("xt=urn:btih:")?;
    let after_xt = &s[xt_pos + 12..];
    // Find end of hash (next & or end of string)
    let hash_end = after_xt.find('&').unwrap_or(after_xt.len());
    let raw_hash = &after_xt[..hash_end];
    if raw_hash.is_empty() {
        return None;
    }
    // Validate: 40 hex chars or 32 base32 chars
    let valid = raw_hash.len() == 40
        && raw_hash.chars().all(|c| c.is_ascii_hexdigit())
        || raw_hash.len() == 32
            && raw_hash
                .chars()
                .all(|c| c.is_ascii_alphanumeric() && !c.is_ascii_digit());
    if !valid {
        return None;
    }
    let info_hash = normalize_info_hash(raw_hash);

    // Extract display name from dn=...
    let name = extract_param(s, "dn").unwrap_or_else(|| info_hash.clone());

    Some(ParsedMagnet {
        info_hash,
        name,
        magnet: s.to_string(),
    })
}

fn extract_param(magnet: &str, key: &str) -> Option<String> {
    let lower = magnet.to_lowercase();
    let prefix = format!("{}=", key);
    let pos = lower.find(&prefix)?;
    let after = &magnet[pos + prefix.len()..];
    let end = after.find('&').unwrap_or(after.len());
    let raw = &after[..end];
    Some(urlencoding::decode(raw).ok()?.to_string())
}

/// True if the input is nothing but a 40-char hex or 32-char base32 info hash.
pub fn is_info_hash(input: &str) -> bool {
    let s = input.trim();
    (s.len() == 40 && s.chars().all(|c| c.is_ascii_hexdigit()))
        || (s.len() == 32
            && s.chars().all(|c| {
                c.is_ascii_alphanumeric() && !c.is_ascii_digit()
            }))
}

/// Accept either a magnet URI or a bare info hash. A bare hash is normalized and
/// wrapped with default public trackers. Returns None for anything that is neither.
pub fn parse_input(input: &str) -> Option<ParsedMagnet> {
    let s = input.trim();
    if let Some(m) = parse_magnet(s) {
        return Some(m);
    }
    if !is_info_hash(s) {
        return None;
    }
    let info_hash = normalize_info_hash(s);
    Some(ParsedMagnet {
        info_hash: info_hash.clone(),
        name: info_hash.clone(),
        magnet: build_magnet(&info_hash, &info_hash),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_magnet() {
        let m = build_magnet("abc123", "test");
        assert!(m.starts_with("magnet:?xt=urn:btih:abc123&dn=test"));
        assert!(m.contains("&tr="));
    }

    #[test]
    fn test_parse_magnet() {
        let m = parse_magnet("magnet:?xt=urn:btih:abcdef1234567890abcdef1234567890abcdef12&dn=hello");
        assert!(m.is_some());
        let m = m.unwrap();
        assert_eq!(m.info_hash, "abcdef1234567890abcdef1234567890abcdef12");
        assert_eq!(m.name, "hello");
    }

    #[test]
    fn test_is_info_hash() {
        assert!(is_info_hash("abcdef1234567890abcdef1234567890abcdef12"));
        assert!(!is_info_hash("short"));
        assert!(!is_info_hash("magnet:?xt=..."));
    }

    #[test]
    fn test_parse_input_magnet() {
        let r = parse_input("magnet:?xt=urn:btih:abcdef1234567890abcdef1234567890abcdef12");
        assert!(r.is_some());
    }

    #[test]
    fn test_parse_input_bare_hash() {
        let r = parse_input("abcdef1234567890abcdef1234567890abcdef12");
        assert!(r.is_some());
        let r = r.unwrap();
        assert!(r.magnet.starts_with("magnet:?"));
    }

    #[test]
    fn test_parse_input_query() {
        let r = parse_input("some movie search");
        assert!(r.is_none());
    }
}
