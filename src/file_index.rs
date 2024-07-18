use std::sync::Mutex;
use std::{collections::HashMap, fs, io::Write, path::PathBuf};

use chrono;

use once_cell::sync::Lazy;

use savefile_derive::Savefile;

use crate::config::CONF;
use crate::db::hashmap_db::HashMapDB;
use crate::db::list::DBList;
use crate::db::string::DBString;
use crate::db::string_search_db::StringSearchDb;
use crate::prelude::*;
use crate::tfidf::{add_document_to_corpus, TfIdfMap};

pub const FILE_DB_READ: i32 = 0b1;
pub const FILE_DB_WRITE: i32 = 0b10;

static HELD_LOCKS: Lazy<Mutex<Vec<PathBuf>>> = Lazy::new(|| Mutex::new(vec![]));

pub struct FileIndex {
    path: PathBuf,
    pub files: StringSearchDb,
    pub dirs: StringSearchDb,
    // pub tf_idf: HashMap<String, Vec<(PathBuf, f32)>>,
    pub tf_idf: TfIdfMap,
    pub terms: StringSearchDb,
}

// pub fn lock() -> Result<(), Box<dyn std::error::Error>> {
//     let mut lock_file = fs::File::create(LOCK_PATH.clone())?;
//     let time = format!("{}", chrono::Utc::now().timestamp());
//     lock_file.write_all(time.as_bytes())?;
//     Ok(())
// }
//
// pub fn unlock() {
//     let _ = fs::remove_file(LOCK_PATH.clone());
// }
//
// pub fn is_locked() -> bool {
//     if LOCK_PATH.exists() {
//         if let Some(value) = unlock_if_old() {
//             return value;
//         }
//         return true;
//     }
//
//     return false;
// }

#[derive(Debug)]
pub struct IsLocked;

impl std::fmt::Display for IsLocked {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "File index is locked.")
    }
}

impl std::error::Error for IsLocked {}

impl FileIndex {
    fn files_path(path: &PathBuf) -> PathBuf {
        path.join("files")
    }

    fn dirs_path(path: &PathBuf) -> PathBuf {
        path.join("dirs")
    }

    fn tf_idf_path(path: &PathBuf) -> PathBuf {
        path.join("tf_idf")
    }

    fn terms_path(path: &PathBuf) -> PathBuf {
        path.join("terms")
    }

    fn lock_path(path: &PathBuf) -> PathBuf {
        path.join("lock")
    }

    fn last_indexed_path(path: &PathBuf) -> PathBuf {
        path.join("last_indexed")
    }

    pub fn open(
        path: &PathBuf,
        access_flags: i32,
    ) -> Result<FileIndex, Box<dyn std::error::Error>> {
        if !path.exists() {
            std::fs::create_dir_all(&path).unwrap();
        }

        if Self::is_locked(path) {
            return Err(Box::new(IsLocked));
        }

        Self::lock(path)?;

        let files = StringSearchDb::open(Self::files_path(path));
        let dirs = StringSearchDb::open(Self::dirs_path(path));
        let tf_idf = HashMapDB::open(Self::tf_idf_path(path), 5000);
        let terms = StringSearchDb::open(Self::terms_path(path));

        Ok(FileIndex {
            path: path.clone(),
            files,
            dirs,
            tf_idf,
            terms,
        })
    }

    pub fn reset_all(path: &PathBuf) {
        Self::lock(path).unwrap();

        if !path.exists() {
            std::fs::create_dir_all(&path).unwrap();
        }

        StringSearchDb::reset(Self::files_path(path));
        StringSearchDb::reset(Self::dirs_path(path));
        StringSearchDb::reset(Self::tf_idf_path(path));
        StringSearchDb::reset(Self::terms_path(path));

        Self::unlock(path);
    }

    /// Full size of all databases in bytes
    fn db_size(&self) -> usize {
        self.files.size() + self.dirs.size() + self.tf_idf.size() + self.terms.size()
    }

    pub fn exceeded_capcaity(&self) -> bool {
        #[allow(non_upper_case_globals)]
        const GiB: usize = 1024 * 1024 * 1024;
        self.db_size() > (CONF.indexing.size_upper_bound_GiB * GiB as f32) as usize
    }

    pub fn add_file(&mut self, path: &PathBuf) {
        if self.exceeded_capcaity() {
            return;
        }

        let file_name = path.file_name().unwrap().to_str().unwrap().to_string();
        let file_path = path.to_str().unwrap().to_string();

        self.files
            .insert(file_name.clone(), Some(file_path.clone()));

        let keywords = file_name_inner_keywords(&file_name);
        for keyword in keywords {
            self.files.insert(keyword.clone(), Some(file_path.clone()));
        }

        if CONF.search_file_contents {
            add_document_to_corpus(self, path);
        }
    }

    pub fn add_dir(&mut self, path: &PathBuf) {
        if self.exceeded_capcaity() {
            return;
        }

        let dir_name = path.file_name().unwrap().to_str().unwrap().to_string();
        let dir_path = path.to_str().unwrap().to_string();

        self.dirs.insert(dir_name.clone(), Some(dir_path.clone()));

        let keywords = file_name_inner_keywords(&dir_name.clone());
        for keyword in keywords {
            self.dirs.insert(keyword.clone(), Some(dir_path.clone()));
        }
    }

    fn lock(path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let mut lock_file = fs::File::create(Self::lock_path(path))?;
        let time = format!("{}", chrono::Utc::now().timestamp());
        lock_file.write_all(time.as_bytes())?;

        HELD_LOCKS.lock().unwrap().push(path.clone());

        Ok(())
    }

    fn unlock(path: &PathBuf) {
        let _ = fs::remove_file(Self::lock_path(path));

        HELD_LOCKS.lock().unwrap().retain(|p| p != path);
    }

    fn is_locked(path: &PathBuf) -> bool {
        if Self::lock_path(path).exists() {
            if let Some(value) = Self::unlock_if_old(path) {
                return value;
            }
            return true;
        }

        return false;
    }

    pub fn remove_all_locks() {
        let locks = HELD_LOCKS.lock().unwrap();
        let locks_ = locks.clone();
        drop(locks);
        for lock in locks_.iter() {
            Self::unlock(lock);
        }
    }

    pub fn manual_lock(path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        Self::lock(path)
    }

    pub unsafe fn manual_unlock(path: &PathBuf) {
        Self::unlock(path);
    }

    /// * `timeout` - in seconds
    pub fn wait_for_unlock(path: &PathBuf, timeout: u64) {
        let mut time = 0;
        while Self::is_locked(path) {
            std::thread::sleep(std::time::Duration::from_secs(1));
            time += 1;
            println!("Waiting for database lock for {} seconds...", time);

            if time > timeout {
                eprintln!("Timeout waiting for database lock.");
                std::process::exit(1);
            }
        }
    }

    pub fn last_indexed(path: &PathBuf) -> Option<i64> {
        fs::read_to_string(Self::last_indexed_path(path))
            .ok()
            .and_then(|s| s.parse::<i64>().ok())
    }

    pub fn set_last_indexed(path: &PathBuf) {
        let time = format!("{}", chrono::Utc::now().timestamp());
        let _ = fs::write(Self::last_indexed_path(path), time);
    }

    fn unlock_if_old(path: &PathBuf) -> Option<bool> {
        if let Ok(timestamp) = fs::read_to_string(Self::lock_path(path)) {
            if let Ok(timestamp) = timestamp.parse::<i64>() {
                const HOUR: i64 = 60 * 60;

                let now = chrono::Utc::now().timestamp();
                if now - timestamp > HOUR * 3 {
                    Self::unlock(path);
                    return Some(false);
                }
            }
        }
        None
    }
}

impl Drop for FileIndex {
    fn drop(&mut self) {
        Self::unlock(&self.path);
        println!("Unlocked.");
    }
}

// impl Index {
//     pub fn save(&self, name: &str) {
//         let path = PATH.join(name).with_extension("bin");
//         let mut file = fs::File::create(path).unwrap();
//         savefile::save(&mut file, 0, self).unwrap();
//         set_last_indexed();
//     }

//     pub async fn load(name: &str) -> Option<Index> {
//         match fs::File::open(PATH.join(name).with_extension("bin")) {
//             Ok(mut file) => match savefile::load(&mut file, 0) {
//                 Ok(index) => Some(index),
//                 Err(_) => None,
//             },
//             Err(_) => None,
//         }
//     }
// }

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

fn file_name_inner_keywords(file_name: &str) -> Vec<String> {
    let mut tokens = vec![];

    let mut file_name = file_name.to_string();
    // Remove file extension
    if let Some(index) = file_name.chars().rev().position(|c| c == '.') {
        file_name = file_name
            .chars()
            .take(file_name.len() - index - 1)
            .collect::<String>();
    }

    let separator = if file_name.contains('_') {
        Some('_')
    } else if file_name.contains(' ') {
        Some(' ')
    } else if file_name.contains('-') {
        Some('-')
    } else if file_name.contains('.') {
        Some('.')
    } else {
        None
    };

    if let Some(separator) = separator {
        for token in file_name.split(separator).skip(1) {
            tokens.push(token.to_string());
        }
    }

    tokens
        .into_iter()
        .filter(|s| s.len() > 1 && s.chars().all(|c| c.is_alphanumeric()))
        .map(|s| s.to_lowercase())
        .collect()
}
