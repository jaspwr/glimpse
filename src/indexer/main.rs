use std::{collections::HashMap, fs::DirEntry, path::PathBuf};

use docx_rs::*;
use lopdf::Document;

use glimpse::{
    config::CONF,
    indexing::{tokenize_string, Index, self},
};

fn main() {
    if CONF.modules.files {
        if indexing::is_locked() {
            println!("Lock file exists, skipping indexing.");
            return;
        }

        indexing::lock().expect("Failed to lock index.");
        reindex();
        indexing::unlock();
    }
}

fn reindex() {
    let mut files: Vec<PathBuf> = vec![];
    let mut dirs: Vec<PathBuf> = vec![];

    for path in &CONF.search_paths {
        let _ = index_dir(
            &path,
            &CONF.search_hidden_folders,
            &mut files,
            &mut dirs,
            &CONF.ignore_directories,
        );
    }

    let tf_idf = create_token_to_document_map(&files);

    let files = files
        .into_iter()
        .map(|path| path.to_str().unwrap().to_string())
        .collect();

    let dirs = dirs
        .into_iter()
        .map(|path| path.to_str().unwrap().to_string())
        .collect();

    Index {
        files,
        dirs,
        tf_idf,
    }
    .save("index");
}

#[inline]
fn is_hidden_file(file: &DirEntry) -> bool {
    file.file_name().to_str().unwrap().starts_with(".")
}

fn index_dir(
    path: &PathBuf,
    index_hidden: &bool,
    files: &mut Vec<PathBuf>,
    dirs: &mut Vec<PathBuf>,
    ignore_dirs: &Vec<String>,
) -> Result<(), std::io::Error> {
    if ignore_dirs.contains(&path.file_name().unwrap().to_str().unwrap().to_string()) {
        return Ok(());
    }

    let mut dir = std::fs::read_dir(&path)?;

    dirs.push(path.clone());

    while let Some(entry) = dir.next() {
        let _ = handle_dir_entry(entry, index_hidden, files, dirs, ignore_dirs);
    }

    Ok(())
}

fn handle_dir_entry(
    entry: Result<DirEntry, std::io::Error>,
    index_hidden: &bool,
    files: &mut Vec<PathBuf>,
    dirs: &mut Vec<PathBuf>,
    ignore_dirs: &Vec<String>,
) -> Result<(), std::io::Error> {
    let entry = entry?;

    if entry.file_type()?.is_dir() {
        if !index_hidden && is_hidden_file(&entry) {
            return Ok(());
        }
        let _ = index_dir(&entry.path(), index_hidden, files, dirs, ignore_dirs);
    } else {
        if !index_hidden && is_hidden_file(&entry) {
            return Ok(());
        }
        files.push(entry.path());
        // TODO
    }
    Ok(())
}

type TfIdf = HashMap<String, f32>;

type TokenFrequency = HashMap<String, f32>;

type InverseDocumentFrequency = HashMap<String, f32>;

fn create_token_to_document_map(documents: &Vec<PathBuf>) -> HashMap<String, Vec<(PathBuf, f32)>> {
    let mut token_to_document = HashMap::new();

    let doc_to_tf_idf = documents_to_tf_idf(documents);

    for (path, tf_idf) in doc_to_tf_idf {
        for (token, tf_idf) in tf_idf {
            let doc_vec = token_to_document.entry(token).or_insert(vec![]);
            if tf_idf > 0.0 {
                doc_vec.push((path.clone(), tf_idf));
            }
            doc_vec.sort_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap());
            if doc_vec.len() > 5 {
                doc_vec.pop();
            }
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
        .map(|(_, tokens)| term_frequency(tokens.clone()))
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

fn term_frequency(tokens: Vec<String>) -> TokenFrequency {
    let mut t = HashMap::new();

    for token in tokens {
        let count = t.entry(token).or_insert(0.0);
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

    t
}

fn load_as_pdf(path: &PathBuf) -> Option<String> {
    let doc = Document::load(path).ok()?;
    let pages = doc.get_pages();

    let mut pages = pages.len() as u32;

    if pages == 0 {
        return None;
    }

    const MAX_PAGES: u32 = 50;
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
