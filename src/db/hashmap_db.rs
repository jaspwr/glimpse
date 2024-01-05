use std::hash::Hash;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::db::allocator::SaveableDBPointer;

use super::allocator::CopyToDB;
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
}

impl<K_in_db, V> HashMapDB<K_in_db, V>
where
    K_in_db: Clone + HashWithDBAccess + EqWithDBAccess,
    V: Clone,
{
    pub fn open(path: PathBuf, buckets_count: usize) -> Self {
        let mut db = DBSession::open(path);

        let map = if db.meta.pointer_store.len() == 1 {
            let ptr = db.meta.pointer_store[0].to_ptr::<DBHashMap<K_in_db, V>>();
            let borrowed = db.borrow_mut(&ptr);
            assert!(borrowed.len() == 1);
            (*borrowed[0]).clone()
        } else {
            assert!(db.meta.pointer_store.len() == 0);
            let map = DBHashMap::<K_in_db, V>::new(&mut db, buckets_count);
            let map_alloc = db.alloc(vec![map.clone()]);
            db.meta
                .pointer_store
                .push(SaveableDBPointer::from_ptr(map_alloc));
            db.meta.save();
            map
        };

        Self {
            db: Arc::new(Mutex::new(db)),
            map,
        }
    }

    pub fn insert(&mut self, key: K_in_db, value: V) {
        let mut db = self.db.lock().unwrap();
        self.map.insert(&mut db, key, value);
    }

    pub fn get<K_lookup>(&mut self, key: K_lookup) -> Option<V>
    where
        K_lookup: CompareWith<K_in_db> + Hash,
    {
        let mut db = self.db.lock().unwrap();
        self.map.get(&mut db, key)
    }

    pub fn save_meta(&mut self) {
        // let mut db = self.db.lock().unwrap();
        // db.meta.save();
    }

    pub fn reset(path: PathBuf) {
        DBSession::reset(path);
    }

    pub fn alloc_string(&mut self, string: String) -> DBString {
        let mut db = self.db.lock().unwrap();
        let str = DBString::new(&mut db, string);
        str
    }

    pub fn new_list<T: Clone>(&mut self) -> DBList<T> {
        let mut db = self.db.lock().unwrap();
        let list = DBList::new(&mut db);
        list
    }

    pub fn push_to_list<T: Clone>(&mut self, list: &mut DBList<T>, value: T) {
        let mut db = self.db.lock().unwrap();
        list.push(&mut db, value);
    }

    pub fn get_string(&mut self, string: &DBString) -> String {
        let mut db = self.db.lock().unwrap();
        string.load_string(&mut db)
    }

    pub fn get_list<T: Clone>(&mut self, list: &DBList<T>) -> Vec<T> {
        let mut db = self.db.lock().unwrap();
        list.iter(&mut db).collect()
    }
}
