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

use std::{collections::HashMap, path::PathBuf, sync::Arc};

use async_trait::async_trait;
use glimpse::{
    config::CONF,
    file_index::{tokenize_string, FileIndex, FILE_DB_READ},
    prelude::*,
    tfidf::_tf_idf,
};

use crate::{
    app::BoxedRuntime,
    exec::{execute_detached, xdg_open},
    icon,
    result_templates::standard_entry,
    utils::{benchmark, needs_reindex, simple_hash_nonce},
};

use super::{SearchModule, SearchResult};

pub struct Files {
    index: Arc<tokio::sync::Mutex<Option<FileIndex>>>,
}

#[derive(Debug)]
enum FileType {
    File,
    Dir,
}

#[derive(Debug)]
struct FileResult {
    relevance: f32,
    kind: FileType,
}

#[async_trait]
impl SearchModule for Files {
    async fn search(&self, mut query: String, _: u32) -> Vec<SearchResult> {
        if query.is_empty() {
            return vec![];
        }

        query = query.to_lowercase();

        let mut index = self.index.lock().await;

        let hash_fn = simple_hash_nonce(std::any::type_name::<Self>());

        if let Some(index) = index.as_mut() {
            // let query = query.to_lowercase();

            // let mut files = string_search(&query, &index.files, max_results, &hash_fn, false)
            //     .into_iter()
            //     .map(|(s, r)| self.create_result(&s, r, FileType::File, hash_fn(&*s)))
            //     .collect::<Vec<SearchResult>>();

            // let mut dirs = string_search(&query, &index.dirs, max_results, &hash_fn, false)
            //     .into_iter()
            //     .map(|(s, r)| self.create_result(&s, r, FileType::Dir, hash_fn(&*s)))
            //     .collect::<Vec<SearchResult>>();
            let mut files: HashMap<String, FileResult> = HashMap::new();

            let push =
                |files: &mut HashMap<String, FileResult>, s: &String, r: f32, kind: FileType| {
                    let s = s.clone();

                    if let Some(res) = files.get_mut(&s) {
                        res.relevance += r;
                        return;
                    }

                    files.insert(s, FileResult { relevance: r, kind });
                };

            index
                .dirs
                .get(&query, &hash_fn)
                .into_iter()
                .for_each(|(s, r)| push(&mut files, &s, r * 1.5, FileType::Dir));

            index
                .files
                .get(&query, &hash_fn)
                .into_iter()
                .for_each(|(s, r)| push(&mut files, &s, r * 1.4, FileType::File));

            let mut tokens = tokenize_string(&query);
            tokens.dedup();

            if tokens.len() > 1 {
                for token in &tokens {
                    index
                        .dirs
                        .get(token, &hash_fn)
                        .into_iter()
                        .for_each(|(s, r)| push(&mut files, &s, r * 1.5, FileType::Dir));

                    index
                        .files
                        .get(token, &hash_fn)
                        .into_iter()
                        .for_each(|(s, r)| push(&mut files, &s, r * 1.4, FileType::File));
                }
            }

            if CONF.search_file_contents {
                let corpus_size = index.tf_idf.corpus_size();

                tokens.into_iter().for_each(|token| {
                    _tf_idf(corpus_size, index.tf_idf.clone(), &token)
                        .iter()
                        .for_each(|(r, s)| {
                            let s = index.tf_idf.get_string(s);
                            push(&mut files, &s, (*r / 17.).clamp(0., 2.), FileType::File);
                        });

                    let similar_terms = index.terms.get(&token, &hash_fn);

                    for (term, similarity) in similar_terms {
                        if term == token {
                            continue;
                        }

                        _tf_idf(corpus_size, index.tf_idf.clone(), &term)
                            .iter()
                            .for_each(|(r, s)| {
                                let s = index.tf_idf.get_string(s);
                                let r = ((*r * similarity) / 20.).clamp(0., 2.);
                                push(&mut files, &s, r, FileType::File);
                            });
                    }
                });
            }

            merge_results(
                files
                    .into_iter()
                    .filter(|(s, _)| PathBuf::from(s).exists())
                    .map(|(s, res)| {
                        self.create_result(&s, res.relevance / 2., res.kind, hash_fn(&s))
                    })
                    .collect::<Vec<SearchResult>>(),
            )
        } else {
            vec![]
        }
    }
}

fn merge_results(mut results: Vec<SearchResult>) -> Vec<SearchResult> {
    let mut relevances: HashMap<SearchResultId, Relevance> = HashMap::new();
    for result in results.iter_mut() {
        match relevances.get(&result.id) {
            Some(r) => {
                let new_r = result.relevance + r;
                relevances.insert(result.id, new_r);
            }
            None => {
                relevances.insert(result.id, result.relevance);
            }
        }
    }

    results.dedup_by(|a, b| a.id == b.id);

    for results in results.iter_mut() {
        results.relevance = *relevances.get(&results.id).unwrap_or(&0.0);
    }

    results
}

impl Files {
    fn create_result(
        &self,
        name: &String,
        relevance: f32,
        kind: FileType,
        id: u64,
    ) -> SearchResult {
        let name_cpy = name.clone();
        let render = move || {
            let name = match file_name(name_cpy.clone()) {
                Some(name) => name,
                None => name_cpy.clone(),
            };

            let icon_name = match kind {
                FileType::File => {
                    let ext = name.split('.').last().unwrap_or("");
                    find_file_icon_name(ext)
                }
                FileType::Dir => find_folder_icon_name(&name),
            };

            let icon = icon::from_gtk(icon_name);

            let mut desc = if CONF.misc.display_file_and_directory_paths {
                Some(name_cpy.clone())
            } else {
                None
            };

            if CONF.misc.run_exes_with_wine && is_windows_application(&name_cpy) {
                desc = Some("Run with wine".to_string());
            }

            standard_entry(name, icon, desc)
        };

        let name_cpy = name.clone();
        let on_select = move || {
            if CONF.misc.run_exes_with_wine && is_windows_application(&name_cpy) {
                if let Some(dir) = PathBuf::from(&name_cpy).parent() {
                    let _ = std::env::set_current_dir(dir);
                }

                let cmd = format!("wine \"{}\"", name_cpy);
                let _ = execute_detached(cmd);
            } else {
                let _ = xdg_open(&name_cpy);
            }
        };

        SearchResult {
            render: Box::new(render),
            relevance,
            id,
            on_select: Some(Box::new(on_select)),
            preview_window_data: crate::preview_window::PreviewWindowShowing::File(PathBuf::from(
                name,
            )),
        }
    }
}

fn is_windows_application(path: &String) -> bool {
    let path = PathBuf::from(path);
    let ext = path
        .extension()
        .unwrap_or_default()
        .to_str()
        .unwrap_or_default();

    ext == "exe"
}

fn find_file_icon_name(ext: &str) -> &str {
    match ext.to_lowercase().as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "svg" => "image-x-generic",
        "mp3" | "wav" | "flac" | "ogg" => "audio-x-generic",
        "mp4" | "mkv" | "avi" | "webm" => "video-x-generic",
        "pdf" => "application-pdf",
        "doc" | "docx" => "application-msword",
        "xls" | "xlsx" => "application-vnd.ms-excel",
        "ppt" | "pptx" => "application-vnd.ms-powerpoint",
        "zip" | "tar" | "gz" | "xz" | "bz2" | "7z" => "package-x-generic",
        "rs" => "text-x-rust",
        "py" => "text-x-python",
        "js" | "ts" => "text-x-javascript",
        "json" => "text-x-javascript",
        "c" => "text-x-csrc",
        "cpp" => "text-x-c++src",
        "go" => "text-x-go",
        "java" => "text-x-java",
        "hs" => "text-x-haskell",
        "sh" => "text-x-script",
        "html" | "htm" => "text-html",
        "svelte" => "text-html",
        "jsx" => "text-javascript",
        "css" => "text-css",
        "scss" => "text-css",
        "sass" => "text-css",
        "md" => "text-x-markdown",
        "exe" => "application-x-executable",
        "deb" | "rpm" => "package-x-generic",
        "tex" => "text-x-tex",
        "jar" => "application-x-java-archive",
        _ => "text-x-generic",
    }
}

fn find_folder_icon_name(name: &str) -> &str {
    match name {
        "Documents" => "folder-documents",
        "Downloads" => "folder-downloads",
        "Music" => "folder-music",
        "Pictures" => "folder-pictures",
        "Videos" => "folder-videos",
        _ => "folder",
    }
}

fn file_name(path_str: String) -> Option<String> {
    let path = std::path::Path::new(&path_str);
    let file_name = path.file_name()?.to_str()?.to_string();
    Some(file_name)
}

impl Files {
    pub fn new(rt: BoxedRuntime) -> Files {
        let index = Arc::new(tokio::sync::Mutex::new(None));

        let index_cpy = index.clone();
        rt.lock().unwrap().spawn(async move {
            let benchmark = benchmark();

            let store = index_cpy.clone();
            let mut lock = store.lock().await;
            // This lock needs to be held until we are finish with initalisation

            let db_path = PathBuf::from(&CONF.indexing.location);
            let index = FileIndex::open(&db_path, FILE_DB_READ).ok();

            if needs_reindex() {
                println!("reindexing files");
                let _ = execute_detached("glimpse-indexer".to_string());
            } else {
                *lock = index;
            }

            if let Some(benchmark) = benchmark {
                println!("Files module loaded in {:?}", benchmark.elapsed().unwrap());
            }
        });

        Files { index }
    }
}
