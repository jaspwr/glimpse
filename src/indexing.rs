use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
};

use docx_rs::*;
use lopdf::Document;

use gdk::glib::once_cell::sync::Lazy;
use savefile_derive::Savefile;

use crate::config::CONF;

pub static PATH: Lazy<PathBuf> = Lazy::new(|| {
    let path = PathBuf::from(CONF.indexing.location.clone());

    if !path.exists() {
        std::fs::create_dir_all(&path).unwrap();
    }

    path
});

#[derive(Savefile)]
pub struct Index {
    pub files: Vec<String>,
    pub dirs: Vec<String>,
    pub tf_idf: HashMap<String, Vec<(PathBuf, f32)>>,
}

impl Index {
    pub fn save(&self, name: &str) {
        let path = PATH.join(name).with_extension("bin");
        let mut file = std::fs::File::create(path).unwrap();
        savefile::save(&mut file, 0, self).unwrap();
    }

    pub async fn load(name: &str) -> Option<Index> {
        match std::fs::File::open(PATH.join(name).with_extension("bin")) {
            Ok(mut file) => match savefile::load(&mut file, 0) {
                Ok(index) => Some(index),
                Err(_) => None,
            },
            Err(_) => None,
        }
    }
}

const WORD_BUF_SIZE: usize = 100;

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

pub fn tokenize_string(str: &String) -> Vec<String> {
    let mut tokens = vec![];

    let mut word_buf: [char; WORD_BUF_SIZE] = ['\0'; WORD_BUF_SIZE];
    let mut word_buf_index = 0;

    let mut pre_is_alpha = false;

    for c in str.chars() {
        handle_char(
            c,
            &mut pre_is_alpha,
            &mut word_buf_index,
            &mut word_buf,
            &mut tokens,
        );
    }

    if pre_is_alpha {
        append_word(&word_buf, word_buf_index, &mut tokens);
    }

    tokens
}

fn handle_char(
    c: char,
    pre_is_alpha: &mut bool,
    word_buf_index: &mut usize,
    word_buf: &mut [char; 100],
    tokens: &mut Vec<String>,
) {
    let is_alphanum = c.is_alphanumeric();

    if is_alphanum != *pre_is_alpha || *word_buf_index == WORD_BUF_SIZE {
        if *pre_is_alpha {
            append_word(&word_buf, *word_buf_index, tokens);
        }

        *word_buf_index = 0;
    }

    word_buf[*word_buf_index] = c;
    *word_buf_index += 1;

    *pre_is_alpha = is_alphanum;
}

#[inline]
fn append_word(word_buf: &[char; 100], word_buf_index: usize, tokens: &mut Vec<String>) {
    let token = word_buf
        .iter()
        .take(word_buf_index)
        .collect::<String>()
        .to_lowercase();

    tokens.push(token);
}
