use std::sync::Arc;

use crate::{CONF, SafeListBox};
use async_trait::async_trait;
use gtk::traits::{WidgetExt, ListBoxExt};

pub struct SearchResult {
    pub render: Box<dyn Fn() -> gtk::Box>,
    pub relevance: f32,
    pub on_select: Option<Box<dyn Fn() + Sync + Send>>,
    pub id: u64
}

unsafe impl Send for SearchResult {}
unsafe impl Sync for SearchResult {}

pub type BoxedSearchModule = Box<dyn SearchModule + Sync + Send>;

#[async_trait]
pub trait SearchModule {
    async fn search(&self, query: String, max_results: u32) -> Vec<SearchResult>;
}


mod commands;
mod dictionary;
mod steam_games;

pub fn load_standard_modules() -> Vec<BoxedSearchModule> {
    let mut ret = Vec::<BoxedSearchModule>::new();

    if CONF.modules.commands {
        ret.push(Box::new(commands::Commands::new()));
    }

    if CONF.modules.web_modules.dictionary {
        ret.push(Box::new(dictionary::Dictionary::new()));
    }

    if CONF.modules.steam_games {
        ret.push(Box::new(steam_games::SteamGames::new()));
    }

    ret
}
