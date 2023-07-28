use std::{collections::HashMap, fs, io::Write, path::PathBuf};

use chrono;
use gdk::glib::once_cell::sync::Lazy;
use savefile_derive::Savefile;

use crate::config::CONF;
use crate::prelude::*;

pub static PATH: Lazy<PathBuf> = Lazy::new(|| {
    let path = PathBuf::from(CONF.indexing.location.clone());

    if !path.exists() {
        std::fs::create_dir_all(&path).unwrap();
    }

    path
});

pub static LOCK_PATH: Lazy<PathBuf> = Lazy::new(|| PATH.join("lock"));

pub static LAST_INDEXED_PATH: Lazy<PathBuf> = Lazy::new(|| PATH.join("last_indexed"));

#[derive(Savefile)]
pub struct Index {
    pub files: Vec<String>,
    pub dirs: Vec<String>,
    pub tf_idf: HashMap<String, Vec<(PathBuf, f32)>>,
}

pub fn lock() -> Result<(), Box<dyn std::error::Error>> {
    let mut lock_file = fs::File::create(LOCK_PATH.clone())?;
    let time = format!("{}", chrono::Utc::now().timestamp());
    lock_file.write_all(time.as_bytes())?;
    Ok(())
}

pub fn unlock() {
    let _ = fs::remove_file(LOCK_PATH.clone());
}

pub fn is_locked() -> bool {
    if LOCK_PATH.exists() {
        if let Some(value) = unlock_if_old() {
            return value;
        }
        return true;
    }

    return false;
}

pub fn last_indexed() -> Option<i64> {
    fs::read_to_string(LAST_INDEXED_PATH.clone())
        .ok()
        .bind(|s| s.parse::<i64>().ok())
}

pub fn set_last_indexed() {
    let time = format!("{}", chrono::Utc::now().timestamp());
    let _ = fs::write(LAST_INDEXED_PATH.clone(), time);
}

fn unlock_if_old() -> Option<bool> {
    if let Ok(timestamp) = fs::read_to_string(LOCK_PATH.clone()) {
        if let Ok(timestamp) = timestamp.parse::<i64>() {
            const HOUR: i64 = 60 * 60;

            let now = chrono::Utc::now().timestamp();
            if now - timestamp > HOUR * 3 {
                unlock();
                return Some(false);
            }
        }
    }
    None
}

impl Index {
    pub fn save(&self, name: &str) {
        let path = PATH.join(name).with_extension("bin");
        let mut file = fs::File::create(path).unwrap();
        savefile::save(&mut file, 0, self).unwrap();
        set_last_indexed();
    }

    pub async fn load(name: &str) -> Option<Index> {
        match fs::File::open(PATH.join(name).with_extension("bin")) {
            Ok(mut file) => match savefile::load(&mut file, 0) {
                Ok(index) => Some(index),
                Err(_) => None,
            },
            Err(_) => None,
        }
    }
}

const WORD_BUF_SIZE: usize = 100;

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
