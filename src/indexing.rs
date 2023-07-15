use std::{
    collections::HashMap,
    io::{BufRead, BufReader},
    path::PathBuf,
};

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
        savefile::save_compressed(&mut file, 0, self).unwrap();
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

pub fn tokenize_file(path: &PathBuf) -> Option<Vec<String>> {
    let mut tokens = vec![];
    let file = std::fs::File::open(path).unwrap();
    let file = BufReader::new(file);

    let mut word_buf: [char; WORD_BUF_SIZE] = ['\0'; WORD_BUF_SIZE];
    let mut word_buf_index = 0;

    let mut pre_is_alpha = false;

    for line in file.lines() {
        let line = match line {
            Ok(line) => line,
            Err(_) => return None,
        };
        for c in line.chars() {
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
    }
    Some(tokens)
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
