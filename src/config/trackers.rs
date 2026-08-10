/// Parse a comma/space-separated list of tracker URLs. Only UDP/HTTP(S) URLs
/// are accepted; duplicates are dropped.
pub fn parse_trackers(input: &str) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = vec![];
    for raw in input.split(|c: char| c.is_whitespace() || c == ',') {
        let url = raw.trim();
        if url.is_empty() {
            continue;
        }
        let valid = url.starts_with("udp://")
            || url.starts_with("http://")
            || url.starts_with("https://")
            || url.starts_with("wss://");
        if !valid || seen.contains(url) {
            continue;
        }
        seen.insert(url.to_string());
        out.push(url.to_string());
    }
    out
}

pub fn format_trackers(trackers: &[String]) -> String {
    trackers.join(", ")
}
