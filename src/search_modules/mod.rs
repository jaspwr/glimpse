use std::{sync::{Mutex, Arc}, future::Future};

use crate::{CONF, SafeListBox};
use async_trait::async_trait;
use gdk::glib::idle_add_once;
use gtk::traits::{WidgetExt, ListBoxExt};

pub struct SearchResult {
    pub render: Box<dyn Fn() -> gtk::Box>,
    pub relevance: f32,
    pub on_select: Option<Box<dyn Fn() + Sync + Send>>,
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

pub fn search(
    loaded_modules: &Vec<BoxedSearchModule>,
    query: String,
    list: Arc<Mutex<gtk::ListBox>>,
) {


    // join_all(module_search_futures).await;
}

pub async fn append_results(
    results: Vec<SearchResult>,
    list: Arc<Mutex<SafeListBox>>,
) {
    let mut results = results;
    results.sort_by(|a, b| a.relevance.partial_cmp(&b.relevance).unwrap());

    // {
    //     let list = list.lock().unwrap();
    //     let mut actions = actions.lock().unwrap();
    //     actions.clear();

    //     for result in &results {
    //         actions.push(result.on_select);
    //     }
    // }

    idle_add_once(move || {
        let rendered_entries = results
            .into_iter()
            .map(|result| result.render.as_ref()())
            .collect::<Vec<gtk::Box>>();

        {
            let list = list.lock().unwrap();
            for entry in rendered_entries {
                // TODO: Order by relevance
                list.list.insert(&entry, 0);
                entry.show_all();
            }

            // if len > 0 {
            //     list.list.select_row(Some(&list.list.row_at_index(0).unwrap()));
            //     *fake_first_selected.lock().unwrap() = true;
            // }
        }
    });
}

pub fn load_standard_modules() -> Vec<BoxedSearchModule> {
    let mut ret = Vec::<BoxedSearchModule>::new();

    if CONF.modules.commands {
        ret.push(Box::new(commands::Commands::new()));
    }

    if CONF.modules.web_modules.dictionary {
        ret.push(Box::new(dictionary::Dictionary::new()));
    }

    ret
}
