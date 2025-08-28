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

use std::{collections::HashMap, path::PathBuf};

use docx_rs::*;
use lopdf::Document;

use crate::{
    db::{hashmap_db::HashMapDB, list::DBList, string::DBString, string_search_db::StringSearchDb},
    file_index::{tokenize_string, FileIndex},
    prelude::Relevance,
};

pub type TfIdfMap = HashMapDB<DBString, DBList<(Relevance, DBString)>>;
type TokenFrequency = HashMap<String, f32>;

pub fn _tf_idf(corpus_size: usize, mut map: TfIdfMap, token: &String) -> Vec<(f32, DBString)> {
    let appearances = match map.get(token) {
        Some(list) => map.get_list(&list),
        None => return vec![],
    };

    let appears_in = appearances.len() as f32;

    if appears_in == 0.0 {
        return vec![];
    }

    let idf = f32::ln(corpus_size as f32 / appears_in);

    appearances
        .into_iter()
        .map(|(tf, doc)| (tf * idf, doc))
        .collect()
}

pub fn add_document_to_corpus(idx: &mut FileIndex, document: &PathBuf) -> Option<()> {
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

    let document_path = idx
        .tf_idf
        .alloc_string(document.to_str().unwrap().to_string());

    for (term, frequency) in term_frequency(&tokens) {
        let term_allocated = idx.tf_idf.alloc_string(term.clone());

        let mut list = idx.tf_idf.get(&term).unwrap_or_else(|| {
            let list = idx.tf_idf.new_list();
            idx.tf_idf.insert(term_allocated, list.clone());
            list
        });

        idx.tf_idf
            .push_to_list(&mut list, (frequency, document_path.clone()));

        remove_lowest_tf_idf_for_token(20, idx.tf_idf.clone(), &term);

        if frequency > 3. {
            add_term(idx.terms.clone(), &term);
        }
    }

    idx.tf_idf.increment_corpus_size();

    Some(())
}

fn remove_lowest_tf_idf_for_token(corpus_size: usize, mut map: TfIdfMap, token: &String) {
    let mut tf_idf = _tf_idf(corpus_size, map.clone(), token);

    if tf_idf.len() < 15 {
        return;
    }

    if tf_idf.is_empty() {
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

fn is_suitable_token(token: &str) -> bool {
    token.len() > 3 && token.len() < 32 && !token.chars().all(|c| c.is_numeric())
}

fn add_term(mut map: StringSearchDb, term: &str) {
    map.insert_if_new(term, Some(term.to_owned()));
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
        if let DocumentChild::Paragraph(paragraph) = child {
            for child in paragraph.children {
                handle_paragraph_child(child, &mut ret);
            }
        }
    }
    ret
}

#[inline]
fn handle_paragraph_child(child: ParagraphChild, ret: &mut String) {
    if let ParagraphChild::Run(run) = child {
        handle_run(run, ret);
    }
}

#[inline]
fn handle_run(run: Box<Run>, ret: &mut String) {
    for child in run.children {
        if let RunChild::Text(text) = child {
            *ret += text.text.to_string().as_str();
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
        Some(ty) => match ty.mime_type() {
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
