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

use fanotify::high_level::*;
use glimpse::{
    config::CONF,
    file_index::{FileIndex, FILE_DB_READ, FILE_DB_WRITE},
};
use nix::poll::{poll, PollFd, PollFlags};
use std::{os::fd::AsFd, path::PathBuf};

fn main() {
    if CONF.error.is_some() {
        eprintln!("Failed to load config");
        std::process::exit(1);
    }

    if !CONF.modules.files {
        eprintln!("Files module is disabled in config");
        std::process::exit(1);
    }

    for path in &CONF.search_paths {
        start_listener(path.clone());
    }
}

fn start_listener(path: PathBuf) {
    // TODO: Make this run in a separate thread to allow for multiple paths to be monitored.

    let ingore_dirs = &CONF.ignore_directories;
    let search_hidden = CONF.search_hidden_folders;

    let ignore_files = vec![
        "dirs",
        "files",
        "tf_idf",
        "terms",
        "dirs.dbmeta1",
        "files.dbmeta1",
        "tf_idf.dbmeta1",
        "terms.dbmeta1",
    ];

    let fd = Fanotify::new_nonblocking(FanotifyMode::CONTENT).unwrap();
    fd.add_mountpoint(FAN_CLOSE_WRITE, &path).unwrap();

    let fd_handle = fd.as_fd();
    let mut fds = [PollFd::new(&fd_handle, PollFlags::POLLIN)];
    loop {
        let poll_num = poll(&mut fds, -1).unwrap();
        if poll_num > 0 {
            for event in fd.read_event() {
                let path = PathBuf::from(&event.path);

                let segments = if path.is_dir() {
                    path.iter()
                } else {
                    path.parent().unwrap().iter()
                };

                let mut handles = true;

                if path.is_file()
                    && ignore_files.contains(&path.file_name().unwrap().to_str().unwrap())
                {
                    handles = false;
                }

                for segment in segments {
                    let segment = segment.to_str().unwrap();

                    if ingore_dirs.contains(&segment.to_string()) {
                        handles = false;
                        break;
                    }

                    if !search_hidden && segment.starts_with(".") {
                        handles = false;
                        break;
                    }
                }

                if handles {
                    let _ = handle_file(path);
                }

                fd.send_response(event.fd, FanotifyResponse::Allow);
            }
        } else {
            eprintln!("poll_num <= 0!");
            break;
        }
    }
}

fn handle_file(path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    if !path.exists() {
        return Ok(());
    }

    println!("File: {:?}", path);

    let db_path = PathBuf::from(&CONF.indexing.location);
    let mut idx = FileIndex::open(&db_path, FILE_DB_READ | FILE_DB_WRITE)?;

    if path.is_dir() {
        idx.add_dir(&path);
    } else {
        idx.add_file(&path);
    }

    Ok(())
}
