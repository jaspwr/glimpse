use std::{
    collections::HashMap,
    fs::DirEntry,
    path::{Path, PathBuf},
};

use docx_rs::*;
use lopdf::Document;

use glimpse::{
    config::CONF,
    db::{string::DBString, string_search_db::StringSearchDb},
    file_index::{self, _tf_idf, set_last_indexed, tokenize_string, FileIndex, TfIdfMap},
};

fn main() {
    if CONF.modules.files {
        if file_index::is_locked() {
            println!("Lock file exists, skipping indexing.");
            return;
        }

        file_index::lock().expect("Failed to lock file index.");
        reindex();
        file_index::unlock();
    }
}

fn reindex() {
    set_last_indexed();

    FileIndex::reset_all();
    let mut idx = FileIndex::open().unwrap();

    for path in &CONF.search_paths {
        let _ = index_dir(
            &path,
            &CONF.search_hidden_folders,
            &mut idx,
            &CONF.ignore_directories,
        );
    }

    idx.dirs.save_meta();
    idx.files.save_meta();

    // create_token_to_document_map(idx.tf_idf.clone(), &files_list);

    idx.tf_idf.save_meta();

    // let files = files
    //     .into_iter()
    //     .map(|path| path.to_str().unwrap().to_string())
    //     .collect();

    // let dirs = dirs
    //     .into_iter()
    //     .map(|path| path.to_str().unwrap().to_string())
    //     .collect();

    // FileIndex {
    //     files,
    //     dirs,
    //     tf_idf,
    // }
    // .save("index");
}

#[inline]
fn is_hidden_file(file: &DirEntry) -> bool {
    file.file_name().to_str().unwrap().starts_with(".")
}

fn index_dir(
    path: &PathBuf,
    index_hidden: &bool,
    idx: &mut FileIndex,
    ignore_dirs: &Vec<String>,
) -> Result<(), std::io::Error> {
    if ignore_dirs.contains(&path.file_name().unwrap().to_str().unwrap().to_string()) {
        return Ok(());
    }

    let mut dir = std::fs::read_dir(&path)?;

    let folder_name = path.file_name().unwrap().to_str().unwrap().to_string();
    let dir_name = path.to_str().unwrap().to_string();

    idx.dirs.insert(folder_name, Some(dir_name));

    while let Some(entry) = dir.next() {
        let _ = handle_dir_entry(entry, index_hidden, idx, ignore_dirs);
    }

    Ok(())
}

fn handle_dir_entry(
    entry: Result<DirEntry, std::io::Error>,
    index_hidden: &bool,
    idx: &mut FileIndex,
    ignore_dirs: &Vec<String>,
) -> Result<(), std::io::Error> {
    let entry = entry?;

    if entry.file_type()?.is_dir() {
        if !index_hidden && is_hidden_file(&entry) {
            return Ok(());
        }
        let _ = index_dir(&entry.path(), index_hidden, idx, ignore_dirs);
    } else {
        if !index_hidden && is_hidden_file(&entry) {
            return Ok(());
        }

        let file_name = entry.file_name().to_str().unwrap().to_string();
        let file_path = entry.path().to_str().unwrap().to_string();

        idx.files.insert(file_name, Some(file_path));

        add_document_to_corpus(idx.tf_idf.clone(), &entry.path());
        // TODO
    }
    Ok(())
}

type TfIdf = HashMap<String, f32>;

type TokenFrequency = HashMap<String, f32>;

type InverseDocumentFrequency = HashMap<String, f32>;

fn create_token_to_document_map(
    mut map: TfIdfMap,
    documents: &Vec<PathBuf>,
) -> HashMap<String, Vec<(PathBuf, f32)>> {
    let mut token_to_document = HashMap::new();

    let doc_to_tf_idf = documents_to_tf_idf(documents);

    for (path, tf_idf) in doc_to_tf_idf {
        for (token, tf_idf) in tf_idf {
            if token.len() < 2 {
                continue;
            }

            let allocated_token = map.alloc_string(token.clone());
            let mut doc_list = if let Some(list) = map.get(token.clone()) {
                list
            } else {
                let list = map.new_list();
                map.insert(allocated_token, list.clone());
                list
            };

            if tf_idf > 0. {
                let allocated_path = map.alloc_string(path.to_str().unwrap().to_string());

                map.push_to_list(&mut doc_list, (tf_idf, allocated_path));
            }

            // doc_vec.sort_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap());

            // if doc_vec.len() > 5 {
            //     doc_vec.pop();
            // }
        }
    }

    token_to_document
}

fn documents_to_tf_idf(documents: &Vec<PathBuf>) -> HashMap<PathBuf, TfIdf> {
    let mut tf_idf = HashMap::new();

    let mut documents = documents
        .into_iter()
        .filter_map(|path| match tokenize_file(&path) {
            Some(tokens) => Some((path.clone(), tokens)),
            None => return None,
        })
        .collect::<Vec<(PathBuf, Vec<String>)>>();

    let tf = documents
        .iter()
        .map(|(_, tokens)| term_frequency(&tokens))
        .collect::<Vec<TokenFrequency>>();

    let idf = inverse_document_frequency(&tf);

    for i in 0..documents.len() {
        let (path, tokens) = documents.remove(0);
        let mut tf_idf_doc = HashMap::new();

        for token in tokens {
            let tf = tf[i].get(&token).unwrap_or(&0.0);
            let idf = idf.get(&token).unwrap_or(&0.0);

            tf_idf_doc.insert(token, tf * idf);
        }

        tf_idf.insert(path, tf_idf_doc);
    }

    tf_idf
}

fn add_document_to_corpus(mut map: TfIdfMap, document: &PathBuf) -> Option<()> {
    let ext = document
        .extension()
        .unwrap_or_default()
        .to_ascii_lowercase();

    if ext != "pdf" && ext != "docx" && ext != "txt" && ext != "md" && ext != "html" && ext != "htm"
    {
        return None;
    }

    let tokens = tokenize_file(document)?;

    let document_path = map.alloc_string(document.to_str().unwrap().to_string());

    for (term, frequency) in term_frequency(&tokens) {
        let term_allocated = map.alloc_string(term.clone());

        let mut list = map.get(&term).unwrap_or_else(|| {
            let list = map.new_list();
            map.insert(term_allocated, list.clone());
            list
        });

        map.push_to_list(&mut list, (frequency, document_path.clone()));

        remove_lowest_tf_idf_for_token(20, map.clone(), &term);
    }

    Some(())
}

fn remove_lowest_tf_idf_for_token(corpus_size: usize, mut map: TfIdfMap, token: &String) {
    let mut tf_idf = _tf_idf(corpus_size, map.clone(), token);

    if tf_idf.len() < 15 {
        return;
    }

    if tf_idf.len() == 0 {
        return;
    }

    tf_idf.sort_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap());

    let mut list = map.get(token).unwrap();

    let lowest = &tf_idf[0];

    map.remove_from_list(&mut list, &lowest.1);
}

fn inverse_document_frequency(documents: &Vec<TokenFrequency>) -> InverseDocumentFrequency {
    let mut d = HashMap::new();

    for doc in documents {
        for (token, _) in doc.iter() {
            let count = d.entry(token).or_insert(0.0);
            *count += 1.0;
        }
    }

    #[allow(non_snake_case)]
    let N = documents.len() as f32;

    d.into_iter()
        .map(|(token, f)| (token.clone(), f32::ln(N / f)))
        .collect()
}

fn term_frequency(tokens: &Vec<String>) -> TokenFrequency {
    let mut t = HashMap::new();

    for token in tokens {
        if !is_suitable_token(token) {
            continue;
        }

        let count = t.entry(token.clone()).or_insert(0.0);
        *count += 1.0;
    }

    // let t_prime = t
    //     .iter()
    //     .map(|(_, f)| f.clone())
    //     .reduce(|mut max: f32, f| {
    //         if f > max {
    //             max = f;
    //         }
    //         max
    //     })
    //     .unwrap_or(0.0);
    //
    // for (_, count) in t.iter_mut() {
    //     *count /= t_prime;
    // }

    if t.len() > 15 {
        let mut t = t.into_iter().collect::<Vec<(String, f32)>>();
        t.sort_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap());

        t.remove(t.len() - 1);
        t.remove(t.len() - 1);
        t.remove(t.len() - 1);
        t.remove(t.len() - 1);
        t.remove(t.len() - 1);

        t.into_iter().collect()
    } else {
        t
    }
}

fn is_suitable_token(token: &String) -> bool {
    token.len() > 3 && token.len() < 32 && !token.chars().all(|c| c.is_numeric())
}

fn load_as_pdf(path: &PathBuf) -> Option<String> {
    let doc = Document::load(path).ok()?;
    let pages = doc.get_pages();

    let mut pages = pages.len() as u32;

    if pages == 0 {
        return None;
    }

    const MAX_PAGES: u32 = 70;
    if pages > MAX_PAGES {
        pages = MAX_PAGES;
    }

    let range = (1..=pages).collect::<Vec<u32>>();
    let text = doc.extract_text(&range).ok()?;

    Some(text)
}

fn load_docx(path: &PathBuf) -> Option<String> {
    let bytes = std::fs::read(path).unwrap();

    let doc = docx_rs::read_docx(&bytes).ok()?.document;
    let contents = get_doc_text(doc.children);

    Some(contents)
}

fn get_doc_text(doc: Vec<DocumentChild>) -> String {
    // This just has to be like this...
    // This crate isn't really meant to be used like this I think.
    let mut ret = String::new();
    for child in doc {
        match child {
            DocumentChild::Paragraph(paragraph) => {
                for child in paragraph.children {
                    handle_paragraph_child(child, &mut ret);
                }
            }
            _ => {}
        }
    }
    ret
}

#[inline]
fn handle_paragraph_child(child: ParagraphChild, ret: &mut String) {
    match child {
        ParagraphChild::Run(run) => {
            handle_run(run, ret);
        }
        _ => {}
    }
}

#[inline]
fn handle_run(run: Box<Run>, ret: &mut String) {
    for child in run.children {
        match child {
            RunChild::Text(text) => {
                *ret += format!("{}", text.text).as_str();
            }
            _ => {}
        }
    }
}

enum FileType {
    Unknown,
    Pdf,
    Docx,
}

pub fn tokenize_file(path: &PathBuf) -> Option<Vec<String>> {
    let mut file_type = match infer::get_from_path(path).ok()? {
        Some(type_) => match type_.mime_type() {
            "application/pdf" => FileType::Pdf,
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
            | "application/word" => FileType::Docx,

            _ => FileType::Unknown,
        },
        None => FileType::Unknown,
    };

    if let Some(ext) = path.extension() {
        if ext == "docx" {
            file_type = FileType::Docx;
        }
    }

    let file = match file_type {
        FileType::Pdf => load_as_pdf(path)?,
        FileType::Docx => load_docx(path)?,
        _ => std::fs::read_to_string(path).ok()?,
    };

    Some(tokenize_string(&file))
}
