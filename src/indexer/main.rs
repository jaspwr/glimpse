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
    file_index::{self, set_last_indexed, tokenize_string, FileIndex},
    tfidf::add_document_to_corpus,
};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.contains(&String::from("--init")) {
        if CONF.error.is_some() {
            eprintln!("Failed to initialize config");
            std::process::exit(1);
        }

        return;
    }


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

    idx.add_dir(path);

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

        idx.add_file(&entry.path());
    }
    Ok(())
}
