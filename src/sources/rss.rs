use crate::sources::types::{SourceError, SourceId, SourceResult, TorrentResult};
use crate::util::net::{fetch_text, HttpError};

/// Unescape common HTML entities in RSS feed text.
pub fn unescape_entities(s: &str) -> String {
    s.replace("&#0?38;", "&")
        .replace("&amp;", "&")
        .replace("&#38;", "&")
        .replace("&#8211;", "-")
        .replace("&#8212;", "-")
        .replace("&#8217;", "'")
        .replace("&#0?39;", "'")
        .replace("&apos;", "'")
        .replace("&#8220;", "\"")
        .replace("&#8221;", "\"")
        .replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

/// Extract text content of an XML tag from an RSS item fragment.
/// Handles CDATA sections.
pub fn tag(item: &str, name: &str) -> String {
    let open = format!("<{}", name);
    let close = format!("</{}>", name);
    if let Some(start) = item.find(&open) {
        let after_open = &item[start..];
        // Skip to the end of the opening tag (past attributes and >)
        if let Some(gt) = after_open.find('>') {
            let inner = &after_open[gt + 1..];
            let inner = if inner.starts_with("<![CDATA[") {
                let end = inner.find("]]>").unwrap_or(inner.len());
                &inner[9..end]
            } else {
                let end = inner.find(&close).unwrap_or(inner.len());
                &inner[..end]
            };
            return unescape_entities(inner.trim());
        }
    }
    String::new()
}

struct ParsedRssItem {
    magnet: String,
    info_hash: String,
    name: String,
    added: Option<i64>,
}

fn parse_rss_items(xml: &str, _source: SourceId) -> Vec<ParsedRssItem> {
    let mut out = vec![];
    for item in xml.split("<item>").skip(1) {
        // Find magnet link in href="magnet:..."
        let magnet = extract_magnet_from_html(item);
        if magnet.is_empty() {
            continue;
        }
        let info_hash = extract_info_hash_from_magnet(&magnet);
        if info_hash.is_empty() {
            continue;
        }
        let name = tag(item, "title");
        let added_str = tag(item, "pubDate");
        let added = if added_str.is_empty() {
            None
        } else {
            chrono::DateTime::parse_from_rfc2822(&added_str)
                .ok()
                .map(|dt| dt.timestamp())
        };
        out.push(ParsedRssItem {
            magnet: unescape_entities(&magnet),
            info_hash,
            name: unescape_entities(&name),
            added,
        });
    }
    out
}

fn extract_magnet_from_html(html: &str) -> String {
    let lower = html.to_lowercase();
    if let Some(pos) = lower.find("href=\"magnet:?xt=urn:btih:") {
        let start = pos + 6;
        if let Some(end) = html[start..].find('"') {
            return html[start..start + end].to_string();
        }
    }
    String::new()
}

fn extract_info_hash_from_magnet(magnet: &str) -> String {
    let lower = magnet.to_lowercase();
    if let Some(pos) = lower.find("xt=urn:btih:") {
        let after = &magnet[pos + 12..];
        let end = after.find('&').unwrap_or(after.len());
        let hash = &after[..end];
        if !hash.is_empty() {
            return hash.to_lowercase();
        }
    }
    String::new()
}

fn feed_url(base: &str, query: &str, page: u32) -> String {
    let q = query.trim();
    let url = if q.is_empty() {
        format!("{}/feed/", base)
    } else {
        format!("{}/?s={}&feed=rss2", base, urlencoding::encode(q))
    };
    if page <= 1 {
        url
    } else {
        format!("{}{}paged={}", url, if q.is_empty() { "?" } else { "&" }, page)
    }
}

const WP_FEED_PAGE_SIZE: usize = 10;
const FEED_DEPTH: u32 = 3;

/// Fetch a WordPress-style RSS feed with pagination. Used by FitGirl and
/// similar sources that expose RSS via WordPress.
pub async fn fetch_wordpress_rss(
    client: &reqwest::Client,
    base: &str,
    source: SourceId,
    query: &str,
    cancel: Option<&tokio_util::sync::CancellationToken>,
) -> SourceResult<Vec<TorrentResult>> {
    let first_url = feed_url(base, query, 1);
    let first = fetch_text(client, &first_url, 2).await?;
    let first_items = parse_rss_items(&first, source);

    // Count raw <item> tags to decide if we need more pages
    let raw_count = first.matches("<item>").count();
    let mut results: Vec<TorrentResult> = first_items
        .iter()
        .map(|p| TorrentResult {
            info_hash: p.info_hash.clone(),
            name: p.name.clone(),
            size_bytes: 0,
            seeders: 0,
            leechers: 0,
            num_files: None,
            source,
            magnet: p.magnet.clone(),
            added: p.added,
        })
        .collect();

    if raw_count < WP_FEED_PAGE_SIZE {
        return Ok(results);
    }

    // Fetch deeper pages concurrently
    let mut handles = vec![];
    for i in 2..=FEED_DEPTH {
        let url = feed_url(base, query, i);
        let client = client.clone();
        handles.push(tokio::spawn(async move {
            fetch_text(&client, &url, 1).await
        }));
    }

    let mut seen: std::collections::HashSet<String> =
        results.iter().map(|r| r.info_hash.clone()).collect();
    for handle in handles {
        if let Ok(Ok(xml)) = handle.await {
            for p in parse_rss_items(&xml, source) {
                if seen.contains(&p.info_hash) {
                    continue;
                }
                seen.insert(p.info_hash.clone());
                results.push(TorrentResult {
                    info_hash: p.info_hash,
                    name: p.name,
                    size_bytes: 0,
                    seeders: 0,
                    leechers: 0,
                    num_files: None,
                    source,
                    magnet: p.magnet,
                    added: p.added,
                });
            }
        }
    }

    Ok(results)
}
