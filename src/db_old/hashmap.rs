use bytemuck::{Pod, Zeroable};

use super::{*, linked_list::DiskLinkedList};
type Bucket<K, V> = DiskLinkedList<(K, V)>;

unsafe impl<K: Copy + 'static, V: Copy + 'static> Pod for DiskHashMap<K, V> {}
unsafe impl<K: Copy + 'static, V: Copy + 'static> Zeroable for DiskHashMap<K, V> {}

#[derive(Clone, Copy)]
pub struct DiskHashMap<K, V> {
    pub buckets: DbChunk<Bucket<K, V>>,
    pub buckets_count: DbChunk<usize>,
    pub length: DbChunk<usize>,
    pub last_bucket_written_to: DbChunk<usize>,
}

impl<K: Hash + Copy + PartialEq + 'static, V: Copy + 'static> DiskHashMap<K, V> {
    pub fn new(db: &mut DataBase, buckets_count: usize) -> Self {
        let buckets = db.malloc::<Bucket<K, V>>(buckets_count);

        for i in 0..buckets_count {
            let new_bucket = DiskLinkedList::new(db);
            buckets.write(db, i, new_bucket);
        }

        let buckets_count_chunk = db.malloc(1);
        buckets_count_chunk.write(db, 0, buckets_count);
        let length = db.malloc(1);
        length.write(db, 0, 0);

        let last_bucket_written_to = db.malloc(1);
        last_bucket_written_to.write(db, 0, 0);

        Self {
            buckets,
            buckets_count: buckets_count_chunk,
            length,
            last_bucket_written_to,
        }
    }

    pub fn get(&self, db: &DataBase, key: &K) -> Option<V> {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let hash = hasher.finish() as usize;

        let buckets_count = self.buckets_count.fetch(db, 0);
        println!("bucket count: {}", buckets_count);
        let bucket_index = hash % buckets_count;
        let bucket = self.buckets.fetch(db, bucket_index);

        for (k, v) in bucket.iter(db) {
            if k == *key {
                return Some(v);
            }
        }
        None
    }

    pub fn insert(&mut self, db: &mut DataBase, key: K, value: V) {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let hash = hasher.finish() as usize;

        assert!(!self.buckets_count.is_null);
        let buckets_count = self.buckets_count.fetch(db, 0);
        println!("INSERT bucket count: {}", buckets_count);


        let bucket_index = hash % buckets_count;
        println!("bucket idx {}", bucket_index);
        let bucket = self.buckets.fetch(db, bucket_index).clone();

        println!("HEEERE 1");
        bucket.remove(db, Box::new(move |(k, _)| {
            k == key
        }));
        println!("HEEERE 1");
        bucket.push(db, (key, value));

        println!("HEEERE 2");

        let mut length = self.length.fetch(db, 0).clone();
        length += 1;
        self.length.write(db, 0, length);

        self.last_bucket_written_to.write(db, 0, bucket_index);
    }
}
