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

use std::{
    fs::{self, DirEntry},
    path::PathBuf,
};

use glimpse::{
    config::CONF,
    file_index::{FileIndex, FILE_DB_READ, FILE_DB_WRITE},
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
        reindex();
    }
}

fn reindex() {
    let db_path = PathBuf::from(&CONF.indexing.location);
    FileIndex::set_last_indexed(&db_path);

    let temp_db_path = db_path.join("full_index_temp");

    fs::remove_dir_all(&temp_db_path).unwrap_or_default();

    let idx = FileIndex::open(&temp_db_path, FILE_DB_READ | FILE_DB_WRITE);

    if idx.is_err() {
        eprintln!("Failed to open index: {:?}", idx.err().unwrap());
        return;
    }

    let mut idx = idx.unwrap();

    for path in &CONF.search_paths {
        let _ = index_dir(
            path,
            &CONF.search_hidden_folders,
            &mut idx,
            &CONF.ignore_directories,
        );
    }

    idx.dirs.save_meta();
    idx.files.save_meta();
    idx.tf_idf.save_meta();
    idx.terms.save_meta();

    // close db connection
    drop(idx);

    // Copy the temp db to the main db

    FileIndex::wait_for_unlock(&db_path, 60);

    println!("Copying files...");
    FileIndex::manual_lock(&db_path).unwrap();

    let files = fs::read_dir(&temp_db_path).unwrap();
    for file in files {
        let file = file.unwrap();
        let file_name = file.file_name();
        let dest = db_path.join(file_name);

        if dest.exists() {
            fs::remove_file(&dest).unwrap();
        }

        fs::copy(file.path(), dest).unwrap();
    }

    unsafe {
        FileIndex::manual_unlock(&db_path);
    }

    fs::remove_dir_all(temp_db_path).unwrap();
}

#[inline]
fn is_hidden_file(file: &DirEntry) -> bool {
    file.file_name().to_str().unwrap().starts_with('.')
}

fn index_dir(
    path: &PathBuf,
    index_hidden: &bool,
    idx: &mut FileIndex,
    ignore_dirs: &Vec<String>,
) -> Result<(), std::io::Error> {
    if idx.exceeded_capcaity() {
        println!("Exceeded capacity");
        return Ok(());
    }

    if ignore_dirs.contains(&path.file_name().unwrap().to_str().unwrap().to_string()) {
        return Ok(());
    }

    let dir = std::fs::read_dir(path)?;

    idx.add_dir(path);

    for entry in dir {
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
    if idx.exceeded_capcaity() {
        println!("Exceeded capacity");
        return Ok(());
    }

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
