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

use std::path::PathBuf;

use glimpse::{config::CONF, file_index::FileIndex};

pub fn simple_hash(s: &str) -> u64 {
    let mut ret: u64 = 0;
    for c in s.chars() {
        ret += c as u64;
        ret <<= 4;
        ret |= c as u64;
    }
    ret
}

pub type HashFn = Box<dyn Fn(&str) -> u64>;

pub fn simple_hash_nonce(n: &str) -> HashFn {
    let nonce = simple_hash(n);
    Box::new(move |s| simple_hash(s) ^ nonce)
}

pub struct BenchmarkTimer {
    time: std::time::SystemTime,
}

impl BenchmarkTimer {
    fn new() -> BenchmarkTimer {
        BenchmarkTimer {
            time: std::time::SystemTime::now(),
        }
    }

    pub fn elapsed(&self) -> Result<String, std::time::SystemTimeError> {
        let t = self.time.elapsed()?;
        Ok(format!("{:?}", t))
    }
}

pub fn benchmark() -> Option<BenchmarkTimer> {
    if cfg!(debug_assertions) {
        Some(BenchmarkTimer::new())
    } else {
        None
    }
}

pub fn needs_reindex() -> bool {
    let days = CONF.indexing.full_reindex_after_days;
    let now = chrono::Utc::now().timestamp();
    const HOUR: f32 = 60. * 60.;
    const DAY: f32 = HOUR * 24.;

    let db_path = PathBuf::from(&CONF.indexing.location);
    now - FileIndex::last_indexed(&db_path).unwrap_or(0) > (DAY * days) as i64
}

pub fn is_cli_app(name: &str) -> bool {
    matches!(
        name,
        "ls" | "cd"
            | "cat"
            | "rm"
            | "mv"
            | "cp"
            | "mkdir"
            | "rmdir"
            | "touch"
            | "ed"
            | "if"
            | "then"
            | "else"
            | "fi"
            | "for"
            | "do"
            | "done"
            | "while"
            | "until"
            | "case"
            | "esac"
            | "vim"
            | "nano"
            | "ghc"
            | "ghci"
            | "ghcup"
            | "cabal"
            | "rustc"
            | "cargo"
            | "clang"
            | "clang++"
            | "gcc"
            | "g++"
            | "make"
            | "node"
            | "npm"
            | "yarn"
            | "pnpm"
            | "npx"
            | "python"
            | "python3"
            | "pip"
            | "pip3"
            | "ruby"
            | "gem"
            | "java"
            | "javac"
            | "jshell"
            | "javadoc"
            | "jlink"
            | "jpackage"
            | "jdeps"
            | "jmod"
            | "jdb"
            | "jconsole"
            | "git"
            | "gitk"
            | "pacman"
            | "yay"
            | "paru"
            | "apt"
            | "apt-get"
            | "tar"
            | "unzip"
            | "zip"
            | "unrar"
            | "rar"
            | "7z"
            | "zstd"
            | "gzip"
            | "gunzip"
            | "atool"
            | "neofetch"
            | "julia"
            | "nvim"
            | "emacs"
            | "htop"
            | "top"
            | "btop"
            | "nmtui"
            | "nmcli"
            | "ip"
            | "ipconfig"
            | "ifconfig"
            | "gdb"
            | "ld"
            | "alias"
            | "kill"
            | "pkill"
            | "find"
            | "tree"
            | "sudo"
            | "su"
            | "chown"
            | "chmod"
            | "grep"
            | "sed"
    )
}
