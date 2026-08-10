use crate::sources::magnet::build_magnet;
use crate::sources::rss::unescape_entities;
use crate::sources::types::{Source, SourceGroup, SourceId, SourceResult, TorrentResult};
use crate::util::format::parse_size;
use crate::util::net::fetch_text;
use scraper::{Html, Selector};

const HOSTS: &[&str] = &["1337x.to", "1337x.st", "x1337x.ws", "1337xx.to"];
const MAX_DETAILS: usize = 4;

const STOP_WORDS: &[&str] = &["the", "a", "an", "of", "and", "or", "to"];

struct Row {
    name: String,
    path: String,
    seeders: u64,
    leechers: u64,
    size_bytes: u64,
}

fn parse_rows(html: &str) -> Vec<Row> {
    let document = Html::parse_document(html);
    let mut out = vec![];

    let tr_selector = match Selector::parse("tbody tr") {
        Ok(s) => s,
        Err(_) => return out,
    };

    for tr in document.select(&tr_selector) {
        // Extract link: /torrent/... 
        let link_sel = match Selector::parse("a[href*='/torrent/']") {
            Ok(s) => s,
            Err(_) => continue,
        };
        let link = match tr.select(&link_sel).next() {
            Some(l) => l,
            None => continue,
        };
        let href = link.value().attr("href").unwrap_or("");
        let name = link.text().collect::<String>().trim().to_string();
        if name.is_empty() || href.is_empty() {
            continue;
        }

        let text = tr.text().collect::<String>();
        let seeders = extract_number(&text, "seeds");
        let leechers = extract_number(&text, "leeches");
        let size_bytes = parse_size(&extract_size(&text));

        out.push(Row {
            name: unescape_entities(&name),
            path: href.to_string(),
            seeders,
            leechers,
            size_bytes,
        });
    }

    out
}

fn extract_number(text: &str, _kind: &str) -> u64 {
    // In the table, seeders and leechers are in coll-2/coll-3 cells.
    // We'll just find all numbers in the row and take the first two.
    let nums: Vec<u64> = text
        .split_whitespace()
        .filter_map(|s| s.parse::<u64>().ok())
        .collect();
    // Heuristic: first number after the name is seeders, second is leechers
    // This is fragile but the HTML structure makes it hard to do better
    // without proper CSS class selectors.
    if _kind == "seeds" {
        nums.first().copied().unwrap_or(0)
    } else {
        nums.get(1).copied().unwrap_or(0)
    }
}

fn extract_size(text: &str) -> String {
    // Look for pattern like "1.5 GiB" or "500 MB"
    let re = regex_lite(&text);
    re
}

fn regex_lite(text: &str) -> String {
    // Simple approach: find "N.NN UNIT" pattern
    let words: Vec<&str> = text.split_whitespace().collect();
    for i in 0..words.len().saturating_sub(1) {
        let w = words[i];
        let next = words[i + 1];
        if w.parse::<f64>().is_ok()
            && (next.eq_ignore_ascii_case("KiB")
                || next.eq_ignore_ascii_case("MiB")
                || next.eq_ignore_ascii_case("GiB")
                || next.eq_ignore_ascii_case("TiB")
                || next.eq_ignore_ascii_case("KB")
                || next.eq_ignore_ascii_case("MB")
                || next.eq_ignore_ascii_case("GB")
                || next.eq_ignore_ascii_case("TB"))
        {
            return format!("{} {}", w, next);
        }
    }
    String::new()
}

const MONTHS: &[(&str, u32)] = &[
    ("jan", 0), ("feb", 1), ("mar", 2), ("apr", 3), ("may", 4), ("jun", 5),
    ("jul", 6), ("aug", 7), ("sep", 8), ("oct", 9), ("nov", 10), ("dec", 11),
];

/// Parse "Jun. 26th '26" style dates from 1337x detail pages.
pub fn parse_upload_date(html: &str) -> Option<i64> {
    // Look for "Date uploaded" section and extract month/day/year
    let lower = html.to_lowercase();
    let pos = lower.find("date uploaded")?;
    let after = &html[pos..];
    // Find a month name
    for (month_name, month_num) in MONTHS {
        if let Some(m_pos) = after.to_lowercase().find(month_name) {
            let after_month = &after[m_pos + month_name.len()..];
            // Extract day number
            let day: u32 = after_month
                .chars()
                .skip_while(|c| !c.is_ascii_digit())
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>()
                .parse()
                .ok()?;
            // Extract year (2-digit, 20xx)
            let year_str: String = after_month
                .chars()
                .filter(|c| c.is_ascii_digit())
                .collect();
            if year_str.len() >= 2 {
                let year_suffix: u32 = year_str[..2].parse().ok()?;
                let year = 2000 + year_suffix;
                let dt = chrono::Utc
                    .with_ymd_and_hms(year as i32, month_num + 1, day, 0, 0, 0)
                    .single()?;
                return Some(dt.timestamp());
            }
        }
    }
    None
}

use chrono::TimeZone;

async fn detail_info(
    base: &str,
    path: &str,
    client: &reqwest::Client,
) -> Option<(String, Option<i64>)> {
    let url = format!("{}{}", base, path);
    let html = fetch_text(client, &url, 1).await.ok()?;
    // Find magnet link
    let lower = html.to_lowercase();
    let magnet_start = lower.find("magnet:?xt=urn:btih:")?;
    let after = &html[magnet_start..];
    let magnet_end = after
        .find(|c: char| c == '"' || c == '\'' || c == '<' || c == ' ' || c == '\n')
        .unwrap_or(after.len());
    let magnet = unescape_entities(&after[..magnet_end]);
    let added = parse_upload_date(&html);
    Some((magnet, added))
}

async fn search_impl(
    query: &str,
    cat: &str,
    source: SourceId,
    client: &reqwest::Client,
    _cancel: Option<&tokio_util::sync::CancellationToken>,
) -> SourceResult<Vec<TorrentResult>> {
    let q = query.trim();
    let path = if q.is_empty() {
        format!("/popular-{}", cat.to_lowercase())
    } else {
        let encoded = urlencoding::encode(q).replace("%20", "+");
        format!("/category-search/{}/{}/1/", encoded, cat)
    };

    let mut base = String::new();
    let mut html = String::new();
    let mut last_error: Option<crate::util::net::HttpError> = None;

    for host in HOSTS {
        let candidate = format!("https://{}", host);
        let url = format!("{}{}", candidate, path);
        match fetch_text(client, &url, 1).await {
            Ok(text) => {
                html = text;
                base = candidate;
                break;
            }
            Err(e) => {
                last_error = Some(e);
            }
        }
    }

    if base.is_empty() {
        return Err(crate::sources::types::SourceError::Http(
            last_error.unwrap_or_else(|| crate::util::net::HttpError::new(0, "1337x unreachable")),
        ));
    }

    let all = parse_rows(&html);
    let tokens: Vec<String> = q.to_lowercase().split_whitespace().map(String::from).collect();
    let meaningful: Vec<&String> = tokens.iter().filter(|t| !STOP_WORDS.contains(&t.as_str())).collect();
    let need = if !meaningful.is_empty() { meaningful } else { tokens.iter().collect() };

    let matched: Vec<&Row> = if need.is_empty() {
        all.iter().collect()
    } else {
        all.iter()
            .filter(|r| {
                let n = r.name.to_lowercase();
                need.iter().all(|t| n.contains(*t))
            })
            .collect()
    };

    let mut sorted: Vec<&Row> = matched;
    sorted.sort_by(|a, b| b.seeders.cmp(&a.seeders));
    let rows: Vec<&Row> = sorted.into_iter().take(MAX_DETAILS).collect();

    let mut handles = vec![];
    for row in &rows {
        let base = base.clone();
        let path = row.path.clone();
        let client = client.clone();
        handles.push(tokio::spawn(async move { detail_info(&base, &path, &client).await }));
    }

    let mut out = vec![];
    for (i, handle) in handles.into_iter().enumerate() {
        if let Ok(Some((magnet, added))) = handle.await {
            let lower = magnet.to_lowercase();
            if let Some(pos) = lower.find("urn:btih:") {
                let after = &magnet[pos + 9..];
                let end = after.find('&').unwrap_or(after.len());
                let info_hash = after[..end].to_lowercase();
                if !info_hash.is_empty() {
                    let row = &rows[i];
                    out.push(TorrentResult {
                        info_hash,
                        name: row.name.clone(),
                        size_bytes: row.size_bytes,
                        seeders: row.seeders,
                        leechers: row.leechers,
                        num_files: None,
                        source,
                        magnet,
                        added,
                    });
                }
            }
        }
    }

    Ok(out)
}

pub struct X1337Movies;

#[async_trait::async_trait]
impl Source for X1337Movies {
    fn id(&self) -> SourceId {
        SourceId::X1337Movies
    }
    fn groups(&self) -> Vec<SourceGroup> {
        vec![SourceGroup::Movies]
    }
    fn homepage(&self) -> &str {
        "https://1337x.to"
    }
    fn reports_health(&self) -> bool {
        true
    }
    async fn search(
        &self,
        query: &str,
        client: &reqwest::Client,
        cancel: Option<&tokio_util::sync::CancellationToken>,
    ) -> SourceResult<Vec<TorrentResult>> {
        search_impl(query, "Movies", SourceId::X1337Movies, client, cancel).await
    }
}

pub struct X1337Tv;

#[async_trait::async_trait]
impl Source for X1337Tv {
    fn id(&self) -> SourceId {
        SourceId::X1337Tv
    }
    fn groups(&self) -> Vec<SourceGroup> {
        vec![SourceGroup::Tv]
    }
    fn homepage(&self) -> &str {
        "https://1337x.to"
    }
    fn reports_health(&self) -> bool {
        true
    }
    async fn search(
        &self,
        query: &str,
        client: &reqwest::Client,
        cancel: Option<&tokio_util::sync::CancellationToken>,
    ) -> SourceResult<Vec<TorrentResult>> {
        search_impl(query, "TV", SourceId::X1337Tv, client, cancel).await
    }
}
