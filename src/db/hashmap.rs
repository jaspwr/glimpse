use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;

use super::allocator::CopyToDB;
use super::string::DBString;
use super::{allocator::SerializableDBPointer, list::DBList, session::DBSession};

#[repr(C)]
#[derive(Clone)]
pub struct DBHashMap<K: Clone, V: Clone> {
    inner: SerializableDBPointer<__DBHashMap<K, V>>,
}

type Bucket<K, V> = DBList<KeyValuePair<K, V>>;

#[repr(C)]
struct __DBHashMap<K: Clone, V: Clone> {
    buckets: SerializableDBPointer<Bucket<K, V>>,
    buckets_count: usize,
    length: usize,
    last_bucket_written_to: usize,
}

impl<K: Clone, V: Clone> CopyToDB for __DBHashMap<K, V> {
    fn copy_to_db(&self) -> Self {
        Self {
            buckets: self.buckets.clone(),
            buckets_count: self.buckets_count,
            length: self.length,
            last_bucket_written_to: self.last_bucket_written_to,
        }
    }
}

#[derive(Clone)]
pub struct KeyValuePair<K: Clone, V: Clone> {
    pub key: K,
    pub value: V,
}

// impl<K, V> CopyToDB for KeyValuePair<K, V>
// where
//     K: CopyToDB,
//     V: CopyToDB,
// {
//     fn copy_to_db(&self) -> Self {
//         Self {
//             key: self.key.copy_to_db(),
//             value: self.value.copy_to_db(),
//         }
//     }
// }

impl<K: Clone, V: Clone> From<(K, V)> for KeyValuePair<K, V> {
    fn from((key, value): (K, V)) -> Self {
        Self { key, value }
    }
}

impl<K: Clone, V: Clone> From<KeyValuePair<K, V>> for (K, V) {
    fn from(val: KeyValuePair<K, V>) -> Self {
        (val.key, val.value)
    }
}

impl<KInDb, V> DBHashMap<KInDb, V>
where
    KInDb: Clone + EqWithDBAccess + HashWithDBAccess,
    V: Clone,
{
    pub fn new(db: &mut DBSession, buckets_count: usize) -> Self {
        let buckets = (0..buckets_count)
            .map(|_| DBList::<KeyValuePair<KInDb, V>>::new(db))
            .collect::<Vec<_>>();

        let buckets = db.alloc(buckets);
        let buckets = buckets.to_serializable();

        let map = __DBHashMap::<KInDb, V> {
            buckets,
            buckets_count,
            length: 0,
            last_bucket_written_to: 0,
        };

        let inner = db.alloc(vec![map]);
        let inner = inner.to_serializable();

        DBHashMap::<KInDb, V> { inner }
    }

    pub fn get<'a, KLookup>(&'a mut self, db: &'a mut DBSession, key: KLookup) -> Option<V>
    where
        KLookup: Hash + CompareWith<KInDb>,
    {
        let bucket = self.get_bucket(db, &key);

        // Ideally this would not have to be stored in a vector but
        // `db` needs to be borrowed again. Hopefully this gets optimised
        // out.
        let key_value_pairs = bucket.iter(db).collect::<Vec<KeyValuePair<KInDb, V>>>();

        for kvp in key_value_pairs {
            let (k, v) = kvp.into();
            if key.compare_with(&k, db) {
                return Some(v);
            }
        }

        None
    }

    pub fn insert(&mut self, db: &mut DBSession, key: KInDb, value: V) {
        let mut bucket = self.get_bucket(db, &key);

        bucket.remove(db, |kvp: &KeyValuePair<KInDb, V>, db: &mut DBSession| {
            key.eq(&kvp.key, db)
        });
        bucket.push(db, (key, value).into());
    }

    fn get_bucket<KHashable>(&mut self, db: &mut DBSession, key: &KHashable) -> Bucket<KInDb, V>
    where
        KHashable: HashWithDBAccess,
    {
        let hash = key.hash(db) as usize;

        let ptr = self.inner.to_ptr();
        let inner_ptr = ptr;
        let mut borrow = db.borrow_mut(&inner_ptr);
        assert!(borrow.len() == 1);
        let map = &mut borrow[0];

        let buckets_count = map.buckets_count;
        let bucket_index = hash % buckets_count;
        let buckets_ptr = map.buckets.clone().to_ptr();

        let borrow = db.borrow_mut(&buckets_ptr);

        assert!(borrow.len() == buckets_count);

        // This is just the list head it can be cloned as it only
        // contains a pointer.

        (*borrow[bucket_index]).clone()
    }

    pub fn len(&self, db: &mut DBSession) -> usize {
        let ptr = self.inner.clone().to_ptr();
        let inner_ptr = ptr;
        let mut borrow = db.borrow_mut(&inner_ptr);
        assert!(borrow.len() == 1);
        let map = &mut borrow[0];

        map.length
    }

    pub fn flatten(&self, db: &mut DBSession) -> Vec<(KInDb, V)> {
        let ptr = self.inner.to_ptr();
        let borrow = db.borrow_mut(&ptr);
        assert!(borrow.len() == 1);
        let map = &borrow[0];

        let bucket_count = map.buckets_count;

        let ptr = map.buckets.clone().to_ptr();
        let buckets = db.borrow_mut(&ptr);
        assert!(buckets.len() == bucket_count);

        let mut items = vec![];

        let buckets: Vec<Bucket<KInDb, V>> = (0..bucket_count)
            .map(|bucket_index| buckets[bucket_index].clone())
            .collect();

        for bucket_index in 0..bucket_count {
            let bucket = buckets[bucket_index].clone();
            let bucket_items = bucket.iter(db).collect::<Vec<_>>();
            items.extend(bucket_items.into_iter().map(|kvp| kvp.into()));
        }

        items
    }

    pub fn into_iter(&self, db: &mut DBSession) -> <Vec<(KInDb, V)> as IntoIterator>::IntoIter {
        self.flatten(db).into_iter()
    }
}

pub trait CompareWith<K> {
    fn compare_with(&self, other: &K, db: &mut DBSession) -> bool;
}

impl<T> CompareWith<T> for T
where
    T: PartialEq,
{
    fn compare_with(&self, other: &T, _: &mut DBSession) -> bool {
        self == other
    }
}

// impl<A, B> CompareWith<A> for &B
// where
//     A: CompareWith<B>,
// {
//     fn compare_with(&self, other: A, _: &mut DBSession) -> bool {
//         self == &&other
//     }
// }

impl CompareWith<DBString> for String {
    fn compare_with(&self, other: &DBString, db: &mut DBSession) -> bool {
        self == &other.load_string(db)
    }
}

impl CompareWith<DBString> for &String {
    fn compare_with(&self, other: &DBString, db: &mut DBSession) -> bool {
        *self == &other.load_string(db)
    }
}

impl CompareWith<DBString> for DBString {
    fn compare_with(&self, other: &DBString, db: &mut DBSession) -> bool {
        self.load_string(db) == other.load_string(db)
    }
}

pub trait HashWithDBAccess {
    fn hash(&self, db: &mut DBSession) -> u64;
}

impl<T> HashWithDBAccess for T
where
    T: Hash,
{
    fn hash(&self, _: &mut DBSession) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

pub trait EqWithDBAccess {
    fn eq(&self, other: &Self, db: &mut DBSession) -> bool;
}

impl<T> EqWithDBAccess for T
where
    T: PartialEq,
{
    fn eq(&self, other: &Self, _: &mut DBSession) -> bool {
        self == other
    }
}

#[cfg(test)]

mod tests {
    use std::{fs, path::PathBuf};

    use crate::db::{
        session::{meta_path, remove_if_exists},
        string::DBString,
    };

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

        let flattened = map.flatten(&mut session);
        // NOTE: Order is deterministic but not somewhat random.
        assert_eq!(flattened, vec![(123, 4), (12, 5),]);

        let mut map_2 = DBHashMap::<u32, DBString>::new(&mut session, 1);
        let s1 = "TESTTESTTEST".to_string();
        let s2 = "Hellllllloooooooooooooo.".to_string();
        let str_1 = DBString::new(&mut session, s1.clone());
        let str_2 = DBString::new(&mut session, s2.clone());
        map_2.insert(&mut session, 123, str_1);
        map_2.insert(&mut session, 12, str_2);

        assert_eq!(
            map_2
                .get(&mut session, 123)
                .unwrap()
                .load_string(&mut session),
            s1
        );
        assert_eq!(
            map_2
                .get(&mut session, 12)
                .unwrap()
                .load_string(&mut session),
            s2
        );

        // let map_3 = DBHashMap::<DBString, u32>::new(&mut session, 123);

        drop(session);

        fs::remove_file(path.clone()).unwrap();
        fs::remove_file(meta_path(&path)).unwrap();
    }
}
