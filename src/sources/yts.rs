use crate::sources::magnet::build_magnet;
use crate::sources::types::{Source, SourceError, SourceGroup, SourceId, SourceResult, TorrentResult};
use crate::util::net::{fetch_json, HttpError};
use serde::Deserialize;

const HOSTS: &[&str] = &["yts.mx", "yts.am", "yts.rs"];

#[derive(Debug, Deserialize)]
struct YtsTorrent {
    hash: Option<String>,
    quality: Option<String>,
    #[serde(rename = "type")]
    torrent_type: Option<String>,
    size_bytes: Option<u64>,
    seeds: Option<u64>,
    peers: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct YtsMovie {
    title_long: Option<String>,
    title: Option<String>,
    date_uploaded_unix: Option<i64>,
    torrents: Option<Vec<YtsTorrent>>,
}

#[derive(Debug, Deserialize)]
struct YtsData {
    movies: Option<Vec<YtsMovie>>,
}

#[derive(Debug, Deserialize)]
struct YtsResponse {
    data: Option<YtsData>,
}

pub struct Yts;

#[async_trait::async_trait]
impl Source for Yts {
    fn id(&self) -> SourceId {
        SourceId::Yts
    }
    fn groups(&self) -> Vec<SourceGroup> {
        vec![SourceGroup::Movies]
    }
    fn homepage(&self) -> &str {
        "https://yts.mx"
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
        let mut params = vec![("limit", "50".to_string())];
        if !q.is_empty() {
            params.push(("query_term", q.to_string()));
        } else {
            params.push(("sort_by", "date_added".to_string()));
        }

        let mut last_error: Option<HttpError> = None;
        for host in HOSTS {
            let query_string: String = params
                .iter()
                .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
                .collect::<Vec<_>>()
                .join("&");
            let url = format!("https://{}/api/v2/list_movies.json?{}", host, query_string);
            match fetch_json::<YtsResponse>(client, &url, 1).await {
                Ok(json) => {
                    let mut out = vec![];
                    for movie in json.data.and_then(|d| d.movies).unwrap_or_default() {
                        let base = movie
                            .title_long
                            .or(movie.title)
                            .unwrap_or_else(|| "Unknown".to_string());
                        for t in movie.torrents.unwrap_or_default() {
                            let hash = match &t.hash {
                                Some(h) if !h.is_empty() => h.to_lowercase(),
                                _ => continue,
                            };
                            let tag = match (&t.quality, &t.torrent_type) {
                                (Some(q), Some(ty)) if !q.is_empty() && !ty.is_empty() => {
                                    format!("{} {}", q, ty)
                                }
                                (Some(q), _) if !q.is_empty() => q.clone(),
                                _ => String::new(),
                            };
                            let name = if tag.is_empty() {
                                base.clone()
                            } else {
                                format!("{} [{}]", base, tag)
                            };
                            out.push(TorrentResult {
                                info_hash: hash.clone(),
                                name,
                                size_bytes: t.size_bytes.unwrap_or(0),
                                seeders: t.seeds.unwrap_or(0),
                                leechers: t.peers.unwrap_or(0),
                                num_files: None,
                                source: SourceId::Yts,
                                magnet: build_magnet(&hash, &base),
                                added: movie.date_uploaded_unix,
                            });
                        }
                    }
                    return Ok(out);
                }
                Err(e) => {
                    last_error = Some(e);
                }
            }
        }
        Err(SourceError::Http(
            last_error.unwrap_or(HttpError::new(0, "YTS unreachable")),
        ))
    }
}
