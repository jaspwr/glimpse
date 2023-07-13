use async_trait::async_trait;
use execute::Execute;
use prober::indexing::Index;

use crate::{result_templates::standard_entry, search::string_search, utils::simple_hash, icon};

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
                .map(|(s, relevance)| {
                    let s = s.to_str().unwrap().to_string();

                    let mut relevance = *relevance;
                    if relevance > 3.0 {
                        relevance = 3.0;
                    }

                    self.create_result(s, relevance, FileType::File)
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

            let icon_name = match kind {
                FileType::File => {
                    let ext = name.split('.').last().unwrap_or("");
                    find_file_icon_name(ext)
                }
                FileType::Dir => {
                    find_folder_icon_name(&name)
                }
            };

            let icon = icon::from_gtk(icon_name);

            let desc = Some(name_cpy.clone());
            standard_entry(name, icon, desc)
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
        "js" => "text-x-javascript",
        "json" => "text-x-javascript",
        "c" => "text-x-csrc",
        "cpp" => "text-x-c++src",
        "go" => "text-x-go",
        "java" => "text-x-java",
        "hs" => "text-x-haskell",
        "sh" => "text-x-script",
        "html" | "htm" => "text-html",
        "css" => "text-css",
        "md" => "text-x-markdown",
        "exe" => "application-x-executable",
        "deb" | "rpm" => "package-x-generic",
        _  => "text-x-generic",
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
    pub fn new() -> Files {
        let index = Index::load("index");

        Files {
            index,
        }
    }
}
