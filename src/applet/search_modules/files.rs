use async_trait::async_trait;
use execute::Execute;
use prober::indexing::Index;

use crate::{result_templates::standard_entry, search::string_search, utils::simple_hash};

use super::{SearchModule, SearchResult};

pub struct Files {
    index: Option<Index>,
}

enum FileType {
    File,
    Dir,
}

#[async_trait]
impl SearchModule for Files {
    async fn search(&self, query: String, max_results: u32) -> Vec<SearchResult> {
        if let Some(index) = &self.index {
            let query = query.to_lowercase();

            let mut files = string_search(
                &query,
                &index.files,
                max_results,
                Box::new(id_hash),
                false,
            )
            .into_iter()
            .map(|(s, r)| self.create_result(s, r, FileType::File))
            .collect::<Vec<SearchResult>>();

            let mut dirs = string_search(
                &query,
                &index.dirs,
                max_results,
                Box::new(id_hash),
                false,
            )
            .into_iter()
            .map(|(s, r)| self.create_result(s, r, FileType::Dir))
            .collect::<Vec<SearchResult>>();

            // TODO: tokenize
            let mut file_contents_matches = index
                .tf_idf
                .get(&query)
                .unwrap_or(&vec![])
                .into_iter()
                .map(|(s, r)| {
                    let s = s.to_str().unwrap().to_string();
                    let r = *r;
                    self.create_result(s, r, FileType::File)
                })
                .collect::<Vec<SearchResult>>();

            files.append(&mut dirs);
            files.append(&mut file_contents_matches);
            files
        } else {
            vec![]
        }
    }
}

impl Files {
    fn create_result(&self, name: String, relevance: f32, kind: FileType) -> SearchResult {
        let name_cpy = name.clone();
        let render = move || {
            let name = match file_name(name_cpy.clone()) {
                Some(name) => name,
                None => name_cpy.clone(),
            };
            let desc = Some(name_cpy.clone());
            standard_entry(name, None, desc)
        };

        let name_cpy = name.clone();
        let on_select = move || {
            let mut command = std::process::Command::new("bash");
            command.arg("-c");
            command.arg(format!("xdg-open \"{}\" & disown", name_cpy.clone()));
            let _ = command.execute();
        };

        SearchResult {
            render: Box::new(render),
            relevance,
            id: id_hash(&name),
            on_select: Some(Box::new(on_select)),
        }
    }
}

fn file_name(path_str: String) -> Option<String> {
    let path = std::path::Path::new(&path_str);
    let file_name = path.file_name()?.to_str()?.to_string();
    Some(file_name)
}

fn id_hash(name: &String) -> u64 {
    simple_hash(name) + 0x12389
}

impl Files {
    pub fn new() -> Files {
        let index = Index::load("index");

        Files {
            index,
        }
    }
}
