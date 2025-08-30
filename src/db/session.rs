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

use std::{
    fs::{self, OpenOptions},
    io::{self, Seek, Write},
    path::{Path, PathBuf},
};

use memmap::{Mmap, MmapMut};
use savefile_derive::Savefile;

use super::allocator::*;

pub struct DBSession {
    pub mmap: Option<MmapMut>,
    path: PathBuf,
    pub capacity: BytesLength,
    pub meta: Meta,
}

impl DBSession {
    pub fn open(path: PathBuf) -> Self {
        assert!(if let Some(ext) = path.extension() {
            ext != "dbmeta1"
        } else {
            true
        });

        let _ = create_file_if_inexistent(&path);

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path.clone())
            .unwrap();

        let mmap = unsafe { Mmap::map(&file).unwrap() };
        let mmap = mmap.make_mut().unwrap();

        let meta_path = meta_path(&path);
        let meta = if meta_path.try_exists().unwrap() {
            Meta::load(&meta_path)
        } else {
            let new_meta = Meta::new(&meta_path);
            new_meta.save();
            new_meta
        };

        let capacity = BytesLength(mmap.len());

        Self {
            mmap: Some(mmap),
            meta,
            capacity,
            path,
        }
    }

    pub fn reset(path: PathBuf) {
        println!("resetting db at {:?}", path);
        remove_if_exists(&path);
        remove_if_exists(&meta_path(&path));
    }

    pub fn resize(&mut self, new_capacity: BytesLength) {
        self.meta.save();
        // self.mmap.as_mut().unwrap().flush().unwrap();
        self.mmap = None;

        const SECTION_SIZE: usize = 1024;
        let new_capacity = (new_capacity.0 / SECTION_SIZE + 1) * SECTION_SIZE + 2048;

        println!("resized to {} MiB", new_capacity / (1024 * 1024));

        let file = OpenOptions::new()
            .write(true)
            .open(self.path.clone())
            .unwrap();

        file.set_len(new_capacity as u64).unwrap();

        drop(file);

        *self = Self::open(self.path.clone());
    }
}

impl Drop for DBSession {
    fn drop(&mut self) {
        if let Some(mmap) = self.mmap.as_mut() {
            mmap.flush().unwrap();
        }
        self.meta.save();
    }
}

pub fn meta_path(path: &Path) -> PathBuf {
    let mut path = path.to_owned();
    path.set_extension("dbmeta1");
    path
}

fn create_file_if_inexistent(path: &Path) -> bool {
    if !path.exists() {
        let cap = 1024 * 1024 * 2; // 2 MiB

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .unwrap();

        file.set_len(cap as u64).unwrap();

        file.seek(io::SeekFrom::End(0)).unwrap();

        let zeros = vec![0; cap];
        file.write_all(&zeros).unwrap();
        return true;
    }

    false
}

#[derive(Savefile)]
pub struct Meta {
    pub path: PathBuf,
    pub max_allocated: Address,
    // pub chunk_descriptors: Vec<DBChunkDescriptor>,
    pub pointer_store: Vec<SaveableDBPointer>,
}

const META_VERSION: u32 = 0;

impl Meta {
    pub fn new(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
            // chunk_descriptors: vec![],
            max_allocated: Address(0),
            pointer_store: vec![],
        }
    }

    // pub fn chunk_at(&self, address: Address) -> Option<&DBChunkDescriptor> {
    //     self.chunk_descriptors
    //         .iter()
    //         .find(|chunk| chunk.start == address)
    // }

    pub fn load(path: &Path) -> Self {
        let mut file = fs::File::open(path).unwrap();
        savefile::load(&mut file, META_VERSION).unwrap()
    }

    pub fn save(&self) {
        let mut file = fs::File::create(self.path.clone()).unwrap();
        savefile::save(&mut file, META_VERSION, self).unwrap();
    }
}

pub fn remove_if_exists(path: &Path) {
    if path.try_exists().unwrap() {
        fs::remove_file(path).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_session() {
        let path = PathBuf::from("test_sess.db");

        remove_if_exists(&path);
        remove_if_exists(&meta_path(&path));

        let session = DBSession::open(path.clone());

        // session.resize(BytesLength(2047));

        drop(session);

        fs::remove_file(path.clone()).unwrap();
        fs::remove_file(meta_path(&path)).unwrap();
    }

    #[test]
    fn test_meta() {
        let path = PathBuf::from("test_meta.db");
        let meta_path = meta_path(&path);

        remove_if_exists(&meta_path);

        let meta = Meta::new(&meta_path);
        meta.save();

        let meta = Meta::load(&meta_path);
        // assert_eq!(meta.chunk_descriptors.len(), 0);

        // let chunk = DBChunkDescriptor {
        //     start: Address(0),
        //     length: BytesLength(1024),
        //     allocated: true,
        // };

        // meta.chunk_descriptors.push(chunk);
        meta.save();

        // let meta = Meta::load(&meta_path);

        // assert_eq!(meta.chunk_descriptors[0].length, BytesLength(1024));

        fs::remove_file(meta_path).unwrap();
    }
}
