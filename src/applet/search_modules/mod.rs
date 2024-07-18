use crate::{app::BoxedRuntime, preview_window::PreviewWindowShowing};
use async_trait::async_trait;
use glimpse::config::CONF;

pub struct SearchResult {
    pub render: Box<dyn Fn() -> gtk::Box>,
    pub relevance: f32,
    pub on_select: Option<Box<dyn Fn() + Sync + Send>>,
    pub id: u64,
    pub preview_window_data: PreviewWindowShowing,
}

unsafe impl Send for SearchResult {}
unsafe impl Sync for SearchResult {}

pub type BoxedSearchModule = Box<dyn SearchModule + Sync + Send>;

#[async_trait]
pub trait SearchModule {
    async fn search(&self, query: String, max_results: u32) -> Vec<SearchResult>;
    fn name(&self) -> String {
        std::any::type_name::<Self>().to_string()
    }
    fn uid(&self) -> u64 {
        crate::utils::simple_hash(&self.name())
    }
}

mod calculator;
mod commands;
mod dictionary;
mod files;
mod steam_games;
mod web_bookmarks;

pub fn load_standard_modules(rt: BoxedRuntime) -> Vec<BoxedSearchModule> {
    let mut ret = Vec::<BoxedSearchModule>::new();

    if CONF.modules.commands {
        ret.push(Box::new(commands::Commands::new(rt.clone())));
    }

    if CONF.modules.files {
        ret.push(Box::new(files::Files::new(rt.clone())));
    }

    if CONF.modules.calculator {
        ret.push(Box::new(calculator::Calculator::new()));
    }

    if CONF.modules.web_bookmarks {
        ret.push(Box::new(web_bookmarks::WebBookmarks::new(rt.clone())));
    }

    if CONF.modules.dictionary {
        ret.push(Box::new(dictionary::Dictionary::new()));
    }

    if CONF.modules.steam_games && steam_games::steam_folder_exists() {
        ret.push(Box::new(steam_games::SteamGames::new(rt.clone())));
    }

    ret
}
