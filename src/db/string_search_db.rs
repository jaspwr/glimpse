// Glimpse - GNU/Linux Launcher and File search utility.
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
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::{db::allocator::SaveableDBPointer, string_similarity::word_similarity};

use super::{session::DBSession, trie::DBTrie};

#[derive(Clone)]
pub struct StringSearchDb {
    db: Arc<Mutex<DBSession>>,
    trie: DBTrie,
}

impl StringSearchDb {
    pub fn open(path: PathBuf) -> Self {
        let mut db = DBSession::open(path);

        let trie = if db.meta.pointer_store.len() == 1 {
            let ptr = db.meta.pointer_store[0].to_ptr::<DBTrie>();
            let borrowed = db.borrow_mut(&ptr);
            assert!(borrowed.len() == 1);
            borrowed[0].clone()
        } else {
            // assert!(db.meta.chunk_descriptors.len() == 0);
            let trie = DBTrie::new(&mut db);
            let trie_alloc = db.alloc(vec![trie.clone()]);
            db.meta
                .pointer_store
                .push(SaveableDBPointer::from_ptr(trie_alloc));
            db.meta.save();
            trie
        };

        Self {
            db: Arc::new(Mutex::new(db)),
            trie,
        }
    }

    pub fn insert(&mut self, word: String, points_to: Option<String>) {
        let mut db = self.db.lock().unwrap();
        self.trie
            .insert(&mut db, word.as_str(), &points_to.unwrap());
    }

    pub fn get(&mut self, word: &str, id_hash: &Box<dyn Fn(&str) -> u64>) -> Vec<(String, f32)> {
        if word.len() < 3 {
            return vec![];
        }

        let mut db = self.db.lock().unwrap();

        let mut results = vec![];

        results.extend(
            self.trie
                .fuzzy_get(&mut db, word)
                .into_iter()
                .map(|s| (s.clone(), word_similarity(&word.to_string(), s, id_hash))),
        );

        results
    }

    pub fn insert_if_new(&mut self, word: &String, points_to: Option<String>) {
        let mut db = self.db.lock().unwrap();
        if self.trie.get(&mut db, word).is_empty() {
            self.trie
                .insert(&mut db, word.as_str(), &points_to.unwrap());
        }
    }

    pub fn save_meta(&mut self) {
        // let mut db = self.db.lock().unwrap();
        // db.meta.save();
    }

    /// Database size in bytes
    pub fn size(&self) -> usize {
        let db = self.db.lock().unwrap();
        db.capacity.0
    }

    pub fn reset(path: PathBuf) {
        DBSession::reset(path);
    }
}
