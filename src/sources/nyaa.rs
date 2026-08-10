use crate::sources::magnet::build_magnet;
use crate::sources::rss::{tag, unescape_entities};
use crate::sources::types::{Source, SourceGroup, SourceId, SourceResult, TorrentResult};
use crate::util::format::parse_size;
use crate::util::net::fetch_text;

const BASE: &str = "https://nyaa.si/";

pub struct Nyaa;

#[async_trait::async_trait]
impl Source for Nyaa {
    fn id(&self) -> SourceId {
        SourceId::Nyaa
    }
    fn groups(&self) -> Vec<SourceGroup> {
        vec![SourceGroup::Anime]
    }
    fn homepage(&self) -> &str {
        "https://nyaa.si"
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
        let params = format!(
            "page=rss&q={}&c=0_0&f=0",
            urlencoding::encode(query.trim())
        );
        let url = format!("{}?{}", BASE, params);
        let xml = fetch_text(client, &url, 2).await?;

        let mut out = vec![];
        for item in xml.split("<item>").skip(1) {
            let info_hash = tag(item, "nyaa:infoHash").to_lowercase();
            if info_hash.is_empty() {
                continue;
            }
            let name = unescape_entities(&tag(item, "title"));
            if name.is_empty() {
                continue;
            }
            let seeders: u64 = tag(item, "nyaa:seeders").parse().unwrap_or(0);
            let leechers: u64 = tag(item, "nyaa:leechers").parse().unwrap_or(0);
            let size_bytes = parse_size(&tag(item, "nyaa:size"));
            let date_str = tag(item, "pubDate");
            let added = if date_str.is_empty() {
                None
            } else {
                chrono::DateTime::parse_from_rfc2822(&date_str)
                    .ok()
                    .map(|dt| dt.timestamp())
            };
            out.push(TorrentResult {
                info_hash,
                name,
                size_bytes,
                seeders,
                leechers,
                num_files: None,
                source: SourceId::Nyaa,
                magnet: build_magnet(&tag(item, "nyaa:infoHash"), &tag(item, "title")),
                added,
            });
        }
        Ok(out)
    }
}
