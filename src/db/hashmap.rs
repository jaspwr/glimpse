use bytemuck::{Pod, Zeroable};
use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;

use super::{allocator::SerializableDBPointer, list::DBList, session::DBSession};

#[derive(Clone, Copy)]
pub struct DBHashMap<K, V>
where
    K: Copy + 'static,
    V: Copy + 'static,
{
    inner: SerializableDBPointer<__DBHashMap<K, V>>,
}

#[derive(Clone, Copy)]
struct __DBHashMap<K, V>
where
    K: Copy + 'static,
    V: Copy + 'static,
{
    buckets: SerializableDBPointer<DBList<(K, V)>>,
    buckets_count: usize,
    length: usize,
    last_bucket_written_to: usize,
}

unsafe impl<K, V> Zeroable for __DBHashMap<K, V>
where
    K: Copy + 'static,
    V: Copy + 'static,
{
}

unsafe impl<K, V> Pod for __DBHashMap<K, V>
where
    K: Copy + 'static,
    V: Copy + 'static,
{
}

unsafe impl<K, V> Zeroable for DBHashMap<K, V>
where
    K: Copy + 'static,
    V: Copy + 'static,
{
}

unsafe impl<K, V> Pod for DBHashMap<K, V>
where
    K: Copy + 'static,
    V: Copy + 'static,
{
}

impl<K, V> DBHashMap<K, V>
where
    K: Copy + 'static + Hash + PartialEq,
    V: Copy + 'static,
{
    pub fn new(db: &mut DBSession, buckets_count: usize) -> Self {
        let buckets = (0..buckets_count)
                .map(|_| DBList::<(K, V)>::new(db))
                .collect::<Vec<_>>();

        let buckets = db.alloc(
            buckets
        );
        let buckets = buckets.to_serializable();

        let map = __DBHashMap::<K, V> {
            buckets,
            buckets_count,
            length: 0,
            last_bucket_written_to: 0,
        };

        let inner = db.alloc(vec![map]);
        let inner = inner.to_serializable();

        DBHashMap::<K, V> { inner }
    }

    pub fn get<'a>(&'a mut self, db: &'a mut DBSession, key: K) -> Option<V> {
        let bucket = self.get_bucket(db, key);

        for (k, v) in bucket.iter(db) {
            if k == key {
                return Some(v);
            }
        }

        None
    }

    pub fn insert(&mut self, db: &mut DBSession, key: K, value: V) {
        let mut bucket = self.get_bucket(db, key);

        bucket.remove(db, Box::new(move |(k, _)| k == key));
        bucket.push(db, (key, value));

    }

    fn get_bucket<'a>(&'a mut self, db: &mut DBSession, key: K) -> DBList<(K, V)> where K: Copy + 'static + Hash + PartialEq {
        let ptr = self.inner.to_ptr();
        let inner_ptr = ptr;
        let mut borrow = db.borrow_mut(&inner_ptr);
        assert!(borrow.len() == 1);
        let map = &mut borrow[0];

        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let hash = hasher.finish() as usize;

        let buckets_count = map.buckets_count;
        let bucket_index = hash % buckets_count;
        let buckets_ptr = map.buckets.clone().to_ptr();

        let borrow = db.borrow_mut(&buckets_ptr);

        assert!(borrow.len() == buckets_count);

        let bucket = borrow[bucket_index].clone();
        // This is just the list head it can be cloned as it only
        // contains a pointer.

        bucket
    }
}

#[cfg(test)]

mod tests {
    use std::{fs, path::PathBuf};

    use crate::db::{session::{remove_if_exists, meta_path}, string::DBString};

    use super::*;

    #[test]
    fn hashmaps() {
        let path = PathBuf::from("hashmaps.db");

        remove_if_exists(&path);
        remove_if_exists(&meta_path(&path));

        let mut session = DBSession::open(path.clone());

        let mut map = DBHashMap::<u32, u32>::new(&mut session, 10);

        assert_eq!(map.get(&mut session, 123), None);

        map.insert(&mut session, 123, 4);
        map.insert(&mut session, 12, 5);
        map.insert(&mut session, 12, 5);
        map.insert(&mut session, 12, 5);
        map.insert(&mut session, 12, 5);
        map.insert(&mut session, 12, 5);
        map.insert(&mut session, 12, 5);

        assert_eq!(map.get(&mut session, 123), Some(4));
        assert_eq!(map.get(&mut session, 12), Some(5));
        assert_eq!(map.get(&mut session, 112323), None);
        assert_eq!(map.get(&mut session, 13), None);

        let mut map_2 = DBHashMap::<u32, DBString>::new(&mut session, 1);
        let s1 = "TESTTESTTEST".to_string();
        let s2 = "Hellllllloooooooooooooo.".to_string();
        let str_1 = DBString::new(&mut session, s1.clone());
        let str_2 = DBString::new(&mut session, s2.clone());
        map_2.insert(&mut session, 123, str_1);
        map_2.insert(&mut session, 12, str_2);

        assert_eq!(map_2.get(&mut session, 123).unwrap().load_string(&mut session), s1);
        assert_eq!(map_2.get(&mut session, 12).unwrap().load_string(&mut session), s2);

        drop(session);

        fs::remove_file(path.clone()).unwrap();
        fs::remove_file(meta_path(&path)).unwrap();
    }
}