use crate::sources::magnet::build_magnet;
use crate::sources::types::{Source, SourceGroup, SourceId, SourceResult, TorrentResult};
use crate::util::net::fetch_json;
use serde::Deserialize;

const BASE: &str = "https://bittorrented.com";
const MIN_QUERY: usize = 3;

#[derive(Debug, Deserialize)]
struct BtResult {
    torrent_infohash: Option<String>,
    torrent_name: Option<String>,
    torrent_total_size: Option<u64>,
    torrent_seeders: Option<u64>,
    torrent_leechers: Option<u64>,
    torrent_file_count: Option<u64>,
    torrent_created_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BtResponse {
    results: Option<Vec<BtResult>>,
}

fn to_unix_seconds(iso: &Option<String>) -> Option<i64> {
    iso.as_deref().and_then(|s| {
        chrono::DateTime::parse_from_rfc3339(s)
            .ok()
            .map(|dt| dt.timestamp())
    })
}

pub struct Bittorrented;

#[async_trait::async_trait]
impl Source for Bittorrented {
    fn id(&self) -> SourceId {
        SourceId::Bittorrented
    }
    fn groups(&self) -> Vec<SourceGroup> {
        vec![SourceGroup::Movies, SourceGroup::Tv]
    }
    fn homepage(&self) -> &str {
        BASE
    }
    fn reports_health(&self) -> bool {
        true
    }
    async fn search(
        &self,
        query: &str,
        client: &reqwest::Client,
        _cancel: Option<&tokio_util::sync::CancellationToken>,
    ) -> SourceResult<Vec<TorrentResult>> {
        let q = query.trim();
        if q.len() < MIN_QUERY {
            return Ok(vec![]);
        }

        let params = format!(
            "q={}&type=video&limit=50&sortBy=seeders&sortOrder=desc",
            urlencoding::encode(q)
        );
        let url = format!("{}/api/search/torrents?{}", BASE, params);

        let json: BtResponse = fetch_json(client, &url, 1).await?;

        let mut out = vec![];
        for r in json.results.unwrap_or_default() {
            let info_hash = r.torrent_infohash.unwrap_or_default().to_lowercase();
            if info_hash.is_empty() || !info_hash.chars().all(|c| c.is_ascii_hexdigit()) || info_hash.len() != 40 {
                continue;
            }
            let name = r.torrent_name.clone().unwrap_or_else(|| info_hash.clone());
            let magnet_name = r.torrent_name.as_deref().unwrap_or("");
            out.push(TorrentResult {
                info_hash: info_hash.clone(),
                name,
                size_bytes: r.torrent_total_size.unwrap_or(0),
                seeders: r.torrent_seeders.unwrap_or(0),
                leechers: r.torrent_leechers.unwrap_or(0),
                num_files: r.torrent_file_count,
                source: SourceId::Bittorrented,
                magnet: build_magnet(&info_hash, magnet_name),
                added: to_unix_seconds(&r.torrent_created_at),
            });
        }
        Ok(out)
    }
}
