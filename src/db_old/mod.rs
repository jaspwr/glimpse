mod hashmap;
mod linked_list;
mod string;
mod trie;

use bytemuck::{Pod, Zeroable};
use memmap::{Mmap, MmapMut};
use savefile_derive::Savefile;
use std::fs;
use std::hash::Hash;
use std::sync::Arc;
use std::sync::Mutex;
use std::{collections::hash_map::DefaultHasher, hash::Hasher};
use std::{
    collections::HashMap,
    fs::OpenOptions,
    io::{self, Seek, Write},
    marker::PhantomData,
    mem,
    path::PathBuf,
};

use hashmap::DiskHashMap;
use trie::DiskTrie;

pub struct DataBase {
    path: PathBuf,
    mmap: Option<MmapMut>,
    capacity: usize,
    meta: DbMeta,
}

struct DbMeta {
    chunks: HashMap<usize, DbChunkDescriptor>,
    max_allocated: usize,
    root_location: Option<usize>,
}

#[derive(Clone, Copy, Savefile)]
pub struct DbChunkDescriptor {
    pub start: usize,
    pub length: usize,
    pub allocated: bool,
}

unsafe impl Zeroable for DbChunkDescriptor {}
unsafe impl Pod for DbChunkDescriptor {}

unsafe impl<T: Copy + 'static> Zeroable for DbChunk<T> {}
unsafe impl<T: Copy + 'static> Pod for DbChunk<T> {}

#[derive(Clone, Copy)]
pub struct DbChunk<T> {
    pub is_null: bool,
    pub descriptor: DbChunkDescriptor,
    phantom: PhantomData<T>,
}

impl<T> DbChunk<T> {
    pub fn fetch<'a>(&'a self, db: &'a DataBase, offset: usize) -> &'a T {
        if self.is_null {
            panic!("Attempted to fetch null pointer");
        }

        let offset_bytes = offset * std::mem::size_of::<T>();

        let addr = self.descriptor.start + offset_bytes;

        println!("addr: {}", addr);

        if offset_bytes >= self.descriptor.length {
            panic!("Out of bounds read for type {}. offset: {}, size: {}", std::any::type_name::<T>(), offset_bytes, self.descriptor.length);
            // return None;
        }

        db.fetch::<T>(addr)
    }

    pub fn write(&self, db: &mut DataBase, offset: usize, data: T)
    where
        T: Pod,
    {
        let offset_bytes = offset * std::mem::size_of::<T>();

        if offset_bytes > self.descriptor.length {
            panic!("Out of bounds write for type {}. offset: {}, size: {}", std::any::type_name::<T>(), offset_bytes, self.descriptor.length);
        }

        let addr = self.descriptor.start + offset_bytes;
        db.write::<T>(addr, data);
    }
}

pub struct HashMapDb<K, V> {
    db: DataBase,
    map: DiskHashMap<K, V>,
}

impl<K, V> HashMapDb<K, V>
where
    K: Hash + PartialEq + Copy + 'static,
    V: Copy + 'static,
{
    pub fn new(path: PathBuf, bucket_count: usize) -> Self {
        let mut db = DataBase::new(path);
        db.open();

        let map = db.get_root(Box::new(move |db: &mut DataBase| {
            DiskHashMap::new(db, bucket_count)
        }));

        let map = map.fetch(&db, 0).clone();

        Self { db, map }
    }

    pub fn get(&self, key: &K) -> Option<V> {
        self.map.get(&self.db, key)
    }

    pub fn insert(&mut self, key: K, value: V) {
        self.map.insert(&mut self.db, key, value)
    }

    pub fn save_meta(&mut self) {
        self.db.save_meta();
    }
}

#[derive(Clone)]
pub struct StringSearchDb {
    db: Arc<Mutex<DataBase>>,
    trie: DiskTrie,
}

pub fn null_ptr<T>() -> DbChunk<T> {
    DbChunk {
        is_null: true,
        descriptor: DbChunkDescriptor {
            start: 0,
            length: 0,
            allocated: false,
        },
        phantom: PhantomData,
    }
}

impl StringSearchDb {
    pub fn open(path: PathBuf) -> Self {
        let mut db = DataBase::new(path);
        db.open();

        let trie = db.get_root(Box::new(move |db: &mut DataBase| {
            DiskTrie::new(db).unwrap()
        }));

        let trie = trie.fetch(&db, 0).clone();

        Self {
            db: Arc::new(Mutex::new(db)),
            trie,
        }
    }

    pub fn insert(&mut self, word: String, points_to: Option<String>) {
        let mut db = self.db.lock().unwrap();
        self.trie.insert(&mut db, word, points_to)
    }

    pub fn get(&self, word: &str) -> Vec<(String, f32)> {
        let mut db = self.db.lock().unwrap();
        self.trie.get(&mut db, word)
    }

    pub fn save_meta(&mut self) {
        let db = self.db.lock().unwrap();
        db.save_meta();
    }
}

impl DataBase {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            mmap: None,
            capacity: 0,
            meta: DbMeta {
                chunks: HashMap::new(),
                max_allocated: 0,
                root_location: None,
            },
        }
    }

    #[inline]
    pub fn meta_path(&self) -> PathBuf {
        let mut path = self.path.clone();
        path.set_extension("dbmeta");
        path
    }

    pub fn save_meta(&self) {
        let path = self.meta_path();
        let mut file = fs::File::create(path).unwrap();
        savefile::save(&mut file, 0, &self.meta).unwrap();
    }

    pub fn open(&mut self) {
        if !self.path.exists() {
            let cap = 1024;

            let mut file = OpenOptions::new()
                .write(true)
                .create(true)
                .open(self.path.clone())
                .unwrap();

            file.set_len(cap as u64).unwrap();

            file.seek(io::SeekFrom::End(0)).unwrap();

            let zeros = vec![0; cap];
            file.write_all(&zeros).unwrap();
        }

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.path)
            .unwrap();

        let mmap = unsafe { Mmap::map(&file).unwrap() };
        self.capacity = mmap.len();

        self.mmap = Some(mmap.make_mut().unwrap());

        if self.meta_path().exists() {
            let mut file = fs::File::open(self.meta_path()).unwrap();
            self.meta = savefile::load(&mut file, 0).unwrap();
        }
    }

    pub fn close(&mut self) {
        self.mmap.as_mut().unwrap().flush().unwrap();
        self.mmap = None;
    }

    pub fn set_root<_T>(&mut self, root: &DbChunk<_T>) {
        self.meta.root_location = Some(root.descriptor.start);
    }

    pub fn get_root<'a, T>(
        &'a mut self,
        create_new_root: Box<dyn FnOnce(&mut DataBase) -> T>,
    ) -> DbChunk<T>
    where
        T: Clone + Pod + Copy,
    {
        let root = if let Some(root) = self.try_get_root::<T>() {
            root
        } else {
            let new_root = create_new_root(self);

            let chunk = self.malloc::<T>(1);
            chunk.write(self, 0, new_root);

            self.set_root(&chunk);

            self.save_meta();
            chunk
        };

        root
    }

    fn try_get_root<'a, T>(&'a self) -> Option<DbChunk<T>> {
        if let Some(root_location) = self.meta.root_location {
            let root = DbChunk {
                is_null: false,
                descriptor: self.meta.chunks[&root_location],
                phantom: PhantomData,
            };

            Some(root)
        } else {
            None
        }
    }

    pub fn fetch<T>(&self, offset: usize) -> &T {
        if offset > self.capacity {
            panic!("Out of bounds buffer read. offset: {}, capacity: {}", offset, self.capacity);
        }

        if let Some(mmap) = &self.mmap {
            let slice = &mmap[offset..offset + std::mem::size_of::<T>()];

            let loaded_value = unsafe { transmute::<T>(slice) };

            loaded_value.unwrap()
        } else {
            panic!("Mmap not initialized");
        }
    }

    pub fn malloc<T>(&mut self, size: usize) -> DbChunk<T>
    where
        T: Clone,
    {
        assert!(size > 0);

        let mut chunk: DbChunk<T> = DbChunk {
            is_null: false,
            descriptor: DbChunkDescriptor {
                start: 0,
                length: size * std::mem::size_of::<T>(),
                allocated: true,
            },
            phantom: PhantomData,
        };

        // let (created_chunks, return_chunk) = self.try_alloc_in_free_chunks(size, &mut chunk);

        // if let Some(return_chunk) = return_chunk {
        //     for (start, chunk_desc) in created_chunks {
        //         self.meta.chunks.insert(start, chunk_desc);
        //     }

        //     return Some(return_chunk);
        // }

        // There was no free chunks available so allocate at end.
        chunk.descriptor.start = self.meta.max_allocated;
        self.meta.max_allocated += chunk.descriptor.length + 512;

        if self.meta.max_allocated + 2048 >= self.capacity {
            println!("Resizing");
            self.resize(self.meta.max_allocated+ 2048);
        }

        println!("Allocating at {} -> {}", chunk.descriptor.start, chunk.descriptor.start + chunk.descriptor.length - 1);

        self.meta
            .chunks
            .insert(chunk.descriptor.start, chunk.descriptor);

        self.save_meta();

        chunk
    }

    fn try_alloc_in_free_chunks<T>(
        &mut self,
        size: usize,
        chunk: &mut DbChunk<T>,
    ) -> (Vec<(usize, DbChunkDescriptor)>, Option<DbChunk<T>>)
    where
        T: Clone,
    {
        let mut created_chunks = vec![];
        let mut return_chunk = None;

        for (_, chunk_desc) in &mut self.meta.chunks {
            if chunk_desc.allocated {
                continue;
            }

            if chunk_desc.length >= size * std::mem::size_of::<T>() {
                allocate_to_free_chunk(
                    chunk_desc,
                    size,
                    &mut created_chunks,
                    chunk,
                    &mut return_chunk,
                );
            }
        }
        (created_chunks, return_chunk)
    }

    pub fn free<T>(&mut self, chunk: DbChunk<T>) {
        self.meta.chunks.get_mut(&chunk.descriptor.start).unwrap().allocated = false;
        // TODO: Merge free chunks
        self.save_meta();
    }

    pub fn resize(&mut self, new_capacity: usize) {
        self.close();

        const SECTION_SIZE: usize = 1024;
        let new_capacity = (new_capacity / SECTION_SIZE + 1) * SECTION_SIZE;

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(self.path.clone())
            .unwrap();

        file.set_len(new_capacity as u64).unwrap();

        // file.seek(io::SeekFrom::End(0)).unwrap();

        // let zeros = vec![0; new_capacity];
        // file.write_all(&zeros).unwrap();

        self.open();
    }

    pub fn write<T>(&mut self, offset: usize, data: T)
    where
        T: Pod,
    {
        if offset >= self.capacity {
            panic!("Out of bounds write. offset: {}, capacity: {}", offset, self.capacity);
        }

        if let Some(mmap) = &mut self.mmap {
            for (i, byte) in bytemuck::bytes_of(&data).iter().enumerate() {
                assert!(i < std::mem::size_of::<T>());
                mmap[offset + i] = *byte;
            }
        }
    }
}

fn allocate_to_free_chunk<T>(
    chunk_desc: &mut DbChunkDescriptor,
    size: usize,
    created_chunks: &mut Vec<(usize, DbChunkDescriptor)>,
    chunk: &mut DbChunk<T>,
    return_chunk: &mut Option<DbChunk<T>>,
) where
    T: Clone,
{
    chunk_desc.allocated = true;

    let new_chunk_size = size * std::mem::size_of::<T>();

    if chunk_desc.length > new_chunk_size {
        chunk_desc.length = new_chunk_size;
        created_chunks.push((
            chunk_desc.start,
            DbChunkDescriptor {
                start: chunk_desc.start + new_chunk_size,
                length: chunk_desc.length - new_chunk_size,
                allocated: false,
            },
        ));
    }

    chunk.descriptor = chunk_desc.clone();

    *return_chunk = Some(chunk.clone());
}

impl std::ops::Drop for DataBase {
    fn drop(&mut self) {
        self.close();
    }
}

unsafe fn transmute<T>(data: &[u8]) -> Option<&T> {
    if data.len() != mem::size_of::<T>() {
        return None;
    }

    let result: &T = mem::transmute(data.as_ptr());

    Some(result)
}
