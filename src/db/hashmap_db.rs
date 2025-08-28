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

use std::hash::Hash;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::db::allocator::SaveableDBPointer;

use super::allocator::SerializableDBPointer;
use super::hashmap::{CompareWith, EqWithDBAccess, HashWithDBAccess};
use super::list::DBList;
use super::string::DBString;
use super::{hashmap::DBHashMap, session::DBSession};

#[derive(Clone)]
pub struct HashMapDB<K, V>
where
    K: Clone + HashWithDBAccess + EqWithDBAccess,
    V: Clone,
{
    db: Arc<Mutex<DBSession>>,
    map: DBHashMap<K, V>,
    corpus_size: SerializableDBPointer<usize>,
}

impl<KInDb, V> HashMapDB<KInDb, V>
where
    KInDb: Clone + HashWithDBAccess + EqWithDBAccess,
    V: Clone,
{
    pub fn open(path: PathBuf, buckets_count: usize) -> Self {
        let mut db = DBSession::open(path);

        let (map, corpus_size) = if db.meta.pointer_store.len() == 2 {
            let map_ptr = db.meta.pointer_store[0].to_ptr::<DBHashMap<KInDb, V>>();
            let map_borrowed = db.borrow_mut(&map_ptr);
            assert!(map_borrowed.len() == 1);
            let map = (*map_borrowed[0]).clone();

            let corpus_size = db.meta.pointer_store[1].to_ptr::<usize>().into_serializable();

            (map, corpus_size)
        } else {
            assert!(db.meta.pointer_store.is_empty());
            let map = DBHashMap::<KInDb, V>::new(&mut db, buckets_count);
            let map_alloc = db.alloc(vec![map.clone()]);
            db.meta
                .pointer_store
                .push(SaveableDBPointer::from_ptr(map_alloc));

            let corpus_size = db.alloc(vec![0]).into_serializable();

            db.meta
                .pointer_store
                .push(SaveableDBPointer::from_ptr(corpus_size.clone().to_ptr()));

            db.meta.save();
            (map, corpus_size)
        };

        Self {
            db: Arc::new(Mutex::new(db)),
            map,
            corpus_size,
        }
    }

    pub fn insert(&mut self, key: KInDb, value: V) {
        let mut db = self.db.lock().unwrap();
        self.map.insert(&mut db, key, value);
    }

    pub fn get<KLookup>(&mut self, key: KLookup) -> Option<V>
    where
        KLookup: CompareWith<KInDb> + Hash,
    {
        let mut db = self.db.lock().unwrap();
        self.map.get(&mut db, key)
    }

    pub fn save_meta(&mut self) {
        let db = self.db.lock().unwrap();
        db.meta.save();
    }

    pub fn reset(path: PathBuf) {
        DBSession::reset(path);
    }

    pub fn corpus_size(&mut self) -> usize {
        let mut db = self.db.lock().unwrap();
        *(*db).borrow_mut(&self.corpus_size.to_ptr())[0]
    }

    pub fn increment_corpus_size(&mut self) {
        let mut db = self.db.lock().unwrap();
        *(*db).borrow_mut(&self.corpus_size.to_ptr())[0] += 1;
    }

    pub fn alloc_string(&mut self, string: String) -> DBString {
        let mut db = self.db.lock().unwrap();

        DBString::new(&mut db, string)
    }

    pub fn new_list<T: Clone>(&mut self) -> DBList<T> {
        let mut db = self.db.lock().unwrap();

        DBList::new(&mut db)
    }

    pub fn push_to_list<T: Clone>(&mut self, list: &mut DBList<T>, value: T) {
        let mut db = self.db.lock().unwrap();
        list.push(&mut db, value);
    }

    pub fn remove_from_list<T: Clone + CompareWith<T>, U: Clone>(
        &mut self,
        list: &mut DBList<(U, T)>,
        value: &T,
    ) {
        let mut db = self.db.lock().unwrap();
        list.remove(&mut db, |v, db| v.1.compare_with(value, db));
    }

    pub fn get_string(&mut self, string: &DBString) -> String {
        let mut db = self.db.lock().unwrap();
        string.load_string(&mut db)
    }

    pub fn get_list<T: Clone>(&mut self, list: &DBList<T>) -> Vec<T> {
        let mut db = self.db.lock().unwrap();
        list.iter(&mut db).collect()
    }

    /// Database size in bytes
    pub fn size(&self) -> usize {
        let db = self.db.lock().unwrap();
        db.capacity.0
    }
}
