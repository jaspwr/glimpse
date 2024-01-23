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
        if self.trie.get(&mut db, word).len() == 0 {
            self.trie
                .insert(&mut db, word.as_str(), &points_to.unwrap());
        }
    }

    pub fn save_meta(&mut self) {
        // let mut db = self.db.lock().unwrap();
        // db.meta.save();
    }

    pub fn reset(path: PathBuf) {
        DBSession::reset(path);
    }
}
