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
    idx.tf_idf.save_meta();
    idx.terms.save_meta();
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

        add_document_to_corpus(idx, &entry.path());
    }
    Ok(())
}

type TokenFrequency = HashMap<String, f32>;

fn add_document_to_corpus(
    idx: &mut FileIndex,
    document: &PathBuf,
) -> Option<()> {
    let ext = document
        .extension()
        .unwrap_or_default()
        .to_ascii_lowercase();

    // TODO: Make configurable.
    if ext != "pdf" && ext != "docx" && ext != "txt" && ext != "md" && ext != "html" && ext != "htm"
    {
        return None;
    }

    let tokens = tokenize_file(document)?;

    let document_path = idx.tf_idf.alloc_string(document.to_str().unwrap().to_string());

    for (term, frequency) in term_frequency(&tokens) {
        add_term(idx.terms.clone(), &term);

        let term_allocated = idx.tf_idf.alloc_string(term.clone());

        let mut list = idx.tf_idf.get(&term).unwrap_or_else(|| {
            let list = idx.tf_idf.new_list();
            idx.tf_idf.insert(term_allocated, list.clone());
            list
        });

        idx.tf_idf.push_to_list(&mut list, (frequency, document_path.clone()));

        remove_lowest_tf_idf_for_token(20, idx.tf_idf.clone(), &term);
    }

    idx.tf_idf.increment_corpus_size();

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

fn term_frequency(tokens: &Vec<String>) -> TokenFrequency {
    let mut t = HashMap::new();

    for token in tokens {
        if !is_suitable_token(token) {
            continue;
        }

        let count = t.entry(token.clone()).or_insert(0.0);
        *count += 1.0;
    }

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

fn add_term(mut map: StringSearchDb, term: &String) {
    map.insert_if_new(&term, Some(term.clone()));
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
