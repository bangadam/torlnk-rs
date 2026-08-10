use crate::sources::magnet::build_magnet;
use crate::sources::types::{Source, SourceGroup, SourceId, SourceResult, TorrentResult};
use crate::util::net::fetch_json;
use serde::Deserialize;

const API: &str = "https://apibay.org";

const MOVIE_CATS: &[u64] = &[201, 202, 207, 209];
const TV_CATS: &[u64] = &[205, 208];

const TOP_MOVIES: &str = "https://apibay.org/precompiled/data_top100_207.json";
const TOP_TV: &str = "https://apibay.org/precompiled/data_top100_208.json";

const ZERO_HASH: &str = "0000000000000000000000000000000000000000";

#[derive(Debug, Deserialize)]
struct ApibayItem {
    id: Option<String>,
    name: Option<String>,
    info_hash: Option<String>,
    seeders: Option<String>,
    leechers: Option<String>,
    num_files: Option<String>,
    size: Option<String>,
    added: Option<String>,
    category: Option<String>,
}

fn to_result(it: &ApibayItem, source: SourceId) -> Option<TorrentResult> {
    let info_hash = it.info_hash.as_deref().unwrap_or("").to_lowercase();
    if info_hash.is_empty() || info_hash == ZERO_HASH || it.id.as_deref() == Some("0") {
        return None;
    }
    let name = it.name.clone().unwrap_or_else(|| "Unknown".to_string());
    let num_files: u64 = it.num_files.as_deref().unwrap_or("0").parse().unwrap_or(0);
    Some(TorrentResult {
        info_hash,
        name,
        size_bytes: it.size.as_deref().unwrap_or("0").parse().unwrap_or(0),
        seeders: it.seeders.as_deref().unwrap_or("0").parse().unwrap_or(0),
        leechers: it.leechers.as_deref().unwrap_or("0").parse().unwrap_or(0),
        num_files: if num_files > 0 { Some(num_files) } else { None },
        source,
        magnet: build_magnet(&it.info_hash.as_deref().unwrap_or(""), &it.name.as_deref().unwrap_or("")),
        added: it.added.as_deref().and_then(|s| s.parse::<i64>().ok()),
    })
}

async fn search_impl(
    query: &str,
    cats: &[u64],
    browse_url: &str,
    source: SourceId,
    client: &reqwest::Client,
) -> SourceResult<Vec<TorrentResult>> {
    let q = query.trim();
    let url = if q.is_empty() {
        browse_url.to_string()
    } else {
        format!("{}/q.php?q={}", API, urlencoding::encode(q))
    };

    let items: Vec<ApibayItem> = fetch_json(client, &url, 1).await?;

    let mut out = vec![];
    for it in &items {
        if !q.is_empty() {
            let cat: u64 = it.category.as_deref().unwrap_or("0").parse().unwrap_or(0);
            if !cats.contains(&cat) {
                continue;
            }
        }
        if let Some(r) = to_result(it, source) {
            out.push(r);
        }
    }
    Ok(out)
}

pub struct TpbMovies;

#[async_trait::async_trait]
impl Source for TpbMovies {
    fn id(&self) -> SourceId {
        SourceId::TpbMovies
    }
    fn groups(&self) -> Vec<SourceGroup> {
        vec![SourceGroup::Movies]
    }
    fn homepage(&self) -> &str {
        "https://thepiratebay.org"
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
        search_impl(query, MOVIE_CATS, TOP_MOVIES, SourceId::TpbMovies, client).await
    }
}

pub struct TpbTv;

#[async_trait::async_trait]
impl Source for TpbTv {
    fn id(&self) -> SourceId {
        SourceId::TpbTv
    }
    fn groups(&self) -> Vec<SourceGroup> {
        vec![SourceGroup::Tv]
    }
    fn homepage(&self) -> &str {
        "https://thepiratebay.org"
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
        search_impl(query, TV_CATS, TOP_TV, SourceId::TpbTv, client).await
    }
}
