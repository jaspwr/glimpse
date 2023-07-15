use std::{sync::Arc, path::PathBuf};

use async_trait::async_trait;
use execute::Execute;
use prober::{indexing::{tokenize_string, Index}, config::CONF};

use crate::{
    icon, result_templates::standard_entry, search::string_search, utils::simple_hash, BoxedRuntime, exec::{xdg_open, execute_detached},
};

use super::{SearchModule, SearchResult};

pub struct Files {
    index: Arc<tokio::sync::Mutex<Option<Index>>>,
}

enum FileType {
    File,
    Dir,
}

#[async_trait]
impl SearchModule for Files {
    fn is_ready(&self) -> bool {
        // let index = self.index.lock().unwrap();
        // index.is_some()
        true
    }

    async fn search(&self, query: String, max_results: u32) -> Vec<SearchResult> {
        let index = self.index.lock().await;

        if let Some(index) = index.as_ref() {
            let query = query.to_lowercase();

            let mut files =
                string_search(&query, &index.files, max_results, Box::new(id_hash), false)
                    .into_iter()
                    .map(|(s, r)| self.create_result(s, r, FileType::File))
                    .collect::<Vec<SearchResult>>();

            let mut dirs =
                string_search(&query, &index.dirs, max_results, Box::new(id_hash), false)
                    .into_iter()
                    .map(|(s, r)| self.create_result(s, r, FileType::Dir))
                    .collect::<Vec<SearchResult>>();

            let tokens = tokenize_string(&query);

            let mut file_contents_matches = tokens
                .into_iter()
                .map(|token| {
                    index
                        .tf_idf
                        .get(&token)
                        .unwrap_or(&vec![])
                        .into_iter()
                        .map(|(s, r)| self.handle_tf_idf_result(s, r))
                        .collect::<Vec<SearchResult>>()
                })
                .flatten()
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
    fn handle_tf_idf_result(&self, s: &PathBuf, relevance: &f32) -> SearchResult {
        let s = s.to_str().unwrap().to_string();

        let relevance = clamp_relevance(relevance);

        self.create_result(s, relevance, FileType::File)
    }

    fn create_result(&self, name: String, relevance: f32, kind: FileType) -> SearchResult {
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
                let cmd = format!("wine \"{}\"", name_cpy);
                let _ = execute_detached(cmd);
            } else {
                let _ = xdg_open(&name_cpy);
            }
        };

        SearchResult {
            render: Box::new(render),
            relevance,
            id: id_hash(&name),
            on_select: Some(Box::new(on_select)),
        }
    }
}

#[inline]
fn clamp_relevance(relevance: &f32) -> f32 {
    let mut relevance = *relevance;
    if relevance > 3.0 {
        relevance = 3.0;
    }
    relevance
}

fn is_windows_application(path: &String) -> bool {
    let path = PathBuf::from(path);
    let ext = path.extension().unwrap_or_default().to_str().unwrap_or_default();

    ext == "exe"
}

fn find_file_icon_name(ext: &str) -> &str {
    match ext {
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
        "toml" => "text-x-toml",
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

fn id_hash(name: &String) -> u64 {
    simple_hash(name) + 0x12389
}

impl Files {
    pub fn new(rt: BoxedRuntime) -> Files {
        let index = Arc::new(tokio::sync::Mutex::new(None));

        let index_cpy = index.clone();
        rt.lock().unwrap().spawn(async move {
            let store = index_cpy.clone();
            let mut lock = store.lock().await;
            // This lock needs to be held until we are finish with initalisation

            let index = Index::load("index").await;

            *lock = index;
        });

        Files { index }
    }
}
