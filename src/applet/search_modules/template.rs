// Glimpse - GNU/Linux launcher and file search utility.
// Copyright (C) 2024 https://github.com/jaspwr

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

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
