use crate::sources::magnet::parse_magnet;
use crate::sources::types::{Source, SourceGroup, SourceId, SourceResult, TorrentResult};
use crate::util::net::fetch_json;
use serde::Deserialize;

const API: &str = "https://subsplease.org/api/";
const RES_PREFERENCE: &[&str] = &["1080", "720", "480"];

#[derive(Debug, Deserialize)]
struct SpDownload {
    res: Option<String>,
    magnet: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SpEntry {
    show: Option<String>,
    episode: Option<String>,
    release_date: Option<String>,
    downloads: Option<Vec<SpDownload>>,
}

fn pick_best(downloads: &[SpDownload]) -> Option<&SpDownload> {
    for res in RES_PREFERENCE {
        if let Some(d) = downloads.iter().find(|d| {
            d.res.as_deref() == Some(*res) && d.magnet.as_deref().is_some_and(|m| !m.is_empty())
        }) {
            return Some(d);
        }
    }
    downloads
        .iter()
        .find(|d| d.magnet.as_deref().is_some_and(|m| !m.is_empty()))
}

pub struct Subsplease;

#[async_trait::async_trait]
impl Source for Subsplease {
    fn id(&self) -> SourceId {
        SourceId::Subsplease
    }
    fn groups(&self) -> Vec<SourceGroup> {
        vec![SourceGroup::Anime]
    }
    fn homepage(&self) -> &str {
        "https://subsplease.org"
    }
    fn reports_health(&self) -> bool {
        false
    }
    async fn search(
        &self,
        query: &str,
        client: &reqwest::Client,
        _cancel: Option<&tokio_util::sync::CancellationToken>,
    ) -> SourceResult<Vec<TorrentResult>> {
        let q = query.trim();
        let params = if q.is_empty() {
            "tz=UTC&f=latest".to_string()
        } else {
            format!("tz=UTC&f=search&s={}", urlencoding::encode(q))
        };
        let url = format!("{}?{}", API, params);

        // The API returns either a map of entries or an array (empty/error)
        let parsed: serde_json::Value = fetch_json(client, &url, 1)
            .await
            .map_err(crate::sources::types::SourceError::from)?;

        let obj = match parsed.as_object() {
            Some(o) => o,
            None => return Ok(vec![]),
        };

        let mut out = vec![];
        for (_key, val) in obj {
            let entry: SpEntry = match serde_json::from_value(val.clone()) {
                Ok(e) => e,
                Err(_) => continue,
            };
            let downloads = entry.downloads.unwrap_or_default();
            let dl = match pick_best(&downloads) {
                Some(d) => d,
                None => continue,
            };
            let magnet_str = match &dl.magnet {
                Some(m) if !m.is_empty() => m,
                _ => continue,
            };
            let parsed = match parse_magnet(magnet_str) {
                Some(p) => p,
                None => continue,
            };
            let show = entry.show.unwrap_or_else(|| "Unknown".to_string());
            let ep = entry
                .episode
                .map(|e| format!(" - {}", e))
                .unwrap_or_default();
            let res = dl.res.as_deref().unwrap_or("?");
            // Extract size from magnet xl= param
            let size_bytes = magnet_str
                .split("xl=")
                .nth(1)
                .and_then(|s| s.split('&').next())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0);
            let added = entry
                .release_date
                .as_deref()
                .and_then(|d| chrono::DateTime::parse_from_rfc2822(d).ok())
                .map(|dt| dt.timestamp());

            out.push(TorrentResult {
                info_hash: parsed.info_hash,
                name: format!("{}{} [{}p]", show, ep, res),
                size_bytes,
                seeders: 0,
                leechers: 0,
                num_files: None,
                source: SourceId::Subsplease,
                magnet: parsed.magnet,
                added,
            });
        }
        Ok(out)
    }
}
