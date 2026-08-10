/// Simple token-based auth helpers shared by serve/files modes.

pub fn authorized(provided: Option<&str>, expected: Option<&str>) -> bool {
    match expected {
        None => true,
        Some(exp) => provided == Some(exp),
    }
}

pub fn parse_bearer(header: Option<&str>) -> Option<&str> {
    header.and_then(|h| h.strip_prefix("Bearer "))
}

pub fn parse_token_query(q: Option<&str>) -> Option<&str> {
    q
}
