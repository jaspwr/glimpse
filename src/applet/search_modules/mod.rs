use std::sync::Arc;

use crate::{BoxedRuntime, SafeListBox};
use async_trait::async_trait;
use gtk::traits::{ListBoxExt, WidgetExt};
use prober::config::CONF;

pub struct SearchResult {
    pub render: Box<dyn Fn() -> gtk::Box>,
    pub relevance: f32,
    pub on_select: Option<Box<dyn Fn() + Sync + Send>>,
    pub id: u64,
}

unsafe impl Send for SearchResult {}
unsafe impl Sync for SearchResult {}

pub type BoxedSearchModule = Box<dyn SearchModule + Sync + Send>;

#[async_trait]
pub trait SearchModule {
    fn is_ready(&self) -> bool;
    async fn search(&self, query: String, max_results: u32) -> Vec<SearchResult>;
}

mod calculator;
mod commands;
mod dictionary;
mod files;
mod steam_games;

pub fn load_standard_modules(rt: BoxedRuntime) -> Vec<BoxedSearchModule> {
    let mut ret = Vec::<BoxedSearchModule>::new();

    if CONF.modules.commands {
        ret.push(Box::new(commands::Commands::new(rt.clone())));
    }

    if CONF.modules.web_modules.dictionary {
        ret.push(Box::new(dictionary::Dictionary::new()));
    }

    if CONF.modules.steam_games {
        ret.push(Box::new(steam_games::SteamGames::new(rt.clone())));
    }

    //TODO conf
    ret.push(Box::new(files::Files::new(rt.clone())));

    ret.push(Box::new(calculator::Calculator::new()));

    ret
}
