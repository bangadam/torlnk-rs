use crate::sources::types::{Source, SourceGroup, SourceId};
use std::sync::Arc;

use super::bittorrented::Bittorrented;
use super::eztv::Eztv;
use super::fitgirl::Fitgirl;
use super::nyaa::Nyaa;
use super::piratebay::{TpbMovies, TpbTv};
use super::subsplease::Subsplease;
use super::x1337::{X1337Movies, X1337Tv};
use super::yts::Yts;

/// Build the full list of sources in canonical order.
pub fn all_sources() -> Vec<Arc<dyn Source>> {
    vec![
        Arc::new(Fitgirl),
        Arc::new(Yts),
        Arc::new(TpbMovies),
        Arc::new(X1337Movies),
        Arc::new(Eztv),
        Arc::new(TpbTv),
        Arc::new(X1337Tv),
        Arc::new(Nyaa),
        Arc::new(Subsplease),
        Arc::new(Bittorrented),
    ]
}

const GROUP_ORDER: &[SourceGroup] = &[
    SourceGroup::Games,
    SourceGroup::Movies,
    SourceGroup::Tv,
    SourceGroup::Anime,
];

pub struct SourceGroupList {
    pub group: SourceGroup,
    pub sources: Vec<Arc<dyn Source>>,
}

/// Group sources by their category tabs, in canonical group order.
pub fn sources_by_group(sources: &[Arc<dyn Source>]) -> Vec<SourceGroupList> {
    GROUP_ORDER
        .iter()
        .map(|&group| {
            let group_sources: Vec<Arc<dyn Source>> = sources
                .iter()
                .filter(|s| s.groups().contains(&group))
                .cloned()
                .collect();
            SourceGroupList {
                group,
                sources: group_sources,
            }
        })
        .filter(|g| !g.sources.is_empty())
        .collect()
}
