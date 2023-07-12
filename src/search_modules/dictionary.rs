use std::process::{Command, Stdio};


use async_trait::async_trait;
use execute::Execute;
use gtk::traits::ContainerExt;
use http::Request;

use crate::{result_templates::standard_entry, search::string_search};

use super::{SearchModule, SearchResult};

pub struct Dictionary {}

impl Dictionary {
    pub fn new() -> Dictionary  {
        Dictionary {}
    }
}

#[async_trait]
impl SearchModule for Dictionary {
    async fn search(&self, query: String, max_results: u32) -> Vec<SearchResult> {
        let body = reqwest::get(format!("https://api.dictionaryapi.dev/api/v2/entries/en/{}", query))
                .await.unwrap().text().await.unwrap();

        vec![create_result(body)]
    }
}

fn create_result(name: String) -> SearchResult {
    let render = move || {
        let label = gtk::Label::new(Some(&name[0..100]));
        let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
        container.add(&label);
        container
    };
    SearchResult {
        render: Box::new(render),
        relevance: 1.0,
        on_select: None,
    }
}
