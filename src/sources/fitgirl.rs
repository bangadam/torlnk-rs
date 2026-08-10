use crate::sources::magnet::build_magnet;
use crate::sources::rss::fetch_wordpress_rss;
use crate::sources::types::{Source, SourceGroup, SourceId, SourceResult, TorrentResult};
use crate::util::net::fetch_json;

pub struct Fitgirl;

const HOME: &str = "https://fitgirl-repacks.site";

#[async_trait::async_trait]
impl Source for Fitgirl {
    fn id(&self) -> SourceId {
        SourceId::Fitgirl
    }
    fn groups(&self) -> Vec<SourceGroup> {
        vec![SourceGroup::Games]
    }
    fn homepage(&self) -> &str {
        HOME
    }
    fn reports_health(&self) -> bool {
        false
    }
    async fn search(
        &self,
        query: &str,
        client: &reqwest::Client,
        cancel: Option<&tokio_util::sync::CancellationToken>,
    ) -> SourceResult<Vec<TorrentResult>> {
        fetch_wordpress_rss(client, HOME, SourceId::Fitgirl, query, cancel).await
    }
}
