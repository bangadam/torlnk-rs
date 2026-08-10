use crate::sources::magnet::build_magnet;
use crate::sources::types::{Source, SourceGroup, SourceId, SourceResult, TorrentResult};
use crate::util::net::fetch_json;
use serde::Deserialize;

const API: &str = "https://eztvx.to/api/get-torrents";

#[derive(Debug, Deserialize)]
struct EztvTorrent {
    title: Option<String>,
    filename: Option<String>,
    hash: Option<String>,
    magnet_url: Option<String>,
    seeds: Option<u64>,
    peers: Option<u64>,
    size_bytes: Option<serde_json::Value>,
    date_released_unix: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct EztvResponse {
    torrents: Option<Vec<EztvTorrent>>,
}

pub struct Eztv;

#[async_trait::async_trait]
impl Source for Eztv {
    fn id(&self) -> SourceId {
        SourceId::Eztv
    }
    fn groups(&self) -> Vec<SourceGroup> {
        vec![SourceGroup::Tv]
    }
    fn homepage(&self) -> &str {
        "https://eztvx.to"
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
        // EZTV API doesn't support search; only browse (empty query = latest)
        if !query.trim().is_empty() {
            return Ok(vec![]);
        }

        let url = format!("{}?limit=100&page=1", API);
        let json: EztvResponse = fetch_json(client, &url, 1).await?;

        let mut out = vec![];
        for t in json.torrents.unwrap_or_default() {
            let hash = t.hash.unwrap_or_default().to_lowercase();
            if hash.is_empty() {
                continue;
            }
            let name = t
                .title
                .or(t.filename)
                .unwrap_or_else(|| hash.clone());
            let magnet = t
                .magnet_url
                .filter(|m| !m.is_empty())
                .unwrap_or_else(|| build_magnet(&hash, &name));

            // size_bytes can be string or number in the API
            let size_bytes = t
                .size_bytes
                .as_ref()
                .and_then(|v| {
                    v.as_u64().or_else(|| {
                        v.as_str()
                            .and_then(|s| s.parse::<u64>().ok())
                    })
                })
                .unwrap_or(0);

            out.push(TorrentResult {
                info_hash: hash,
                name,
                size_bytes,
                seeders: t.seeds.unwrap_or(0),
                leechers: t.peers.unwrap_or(0),
                num_files: None,
                source: SourceId::Eztv,
                magnet,
                added: t.date_released_unix,
            });
        }
        Ok(out)
    }
}
