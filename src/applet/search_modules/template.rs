use async_trait::async_trait;

use crate::{search::string_search, CONF, result_templates::standard_entry, utils::simple_hash};

use super::{SearchModule, SearchResult};

pub struct SteamGames {}

#[async_trait]
impl SearchModule for Template {
    async fn search(&self, query: String, max_results: u32) -> Vec<SearchResult> {
        string_search(&query, _, max_results, Box::new(id_hash))
            .into_iter()
            .map(|(s, r)| self.create_result(s, r))
            .collect::<Vec<SearchResult>>()
    }
}

impl SteamGames {
    fn create_result(&self, name: String, relevance: f32) -> SearchResult {
        let render = move || {
            standard_entry(name.clone(), None, None)
        };

        let on_select = move || {
        };

        SearchResult {
            render: Box::new(render),
            relevance,
            id: id_hash(&name),
            on_select: Some(Box::new(on_select)),
        }
    }
}
fn id_hash(name: &String) -> u64 {
    simple_hash(name) * 0xa0b0c0d0e0f0
}

impl SteamGames {
    pub fn new() -> SteamGames {
        SteamGames {}
    }
}
