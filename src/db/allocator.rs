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

use std::marker::PhantomData;

use savefile_derive::Savefile;

use super::session::DBSession;

#[derive(PartialEq, PartialOrd, Savefile, Debug, Clone, Copy)]
#[repr(C)]
pub struct Address(pub usize);
#[derive(PartialEq, PartialOrd, Savefile, Debug, Clone, Copy)]
#[repr(C)]
pub struct BytesLength(pub usize);
#[derive(PartialEq, PartialOrd, Savefile, Debug, Clone, Copy)]
#[repr(C)]
pub struct ArrayLength(pub usize);

impl Address {
    pub fn offset(self, offset: BytesLength) -> Self {
        Address(self.0 + offset.0)
    }

    pub fn align_to_next(self, align: usize) -> Self {
        let remainder = self.0 % align;
        if remainder == 0 {
            self
        } else {
            Address(self.0 + align - remainder)
        }
    }
}

impl BytesLength {
    pub fn times(self, len: ArrayLength) -> Self {
        BytesLength(self.0 * len.0)
    }
}

#[derive(Savefile, Debug, Clone)]
#[repr(C)]
pub struct DBChunkDescriptor {
    pub start: Address,
    pub length: BytesLength,
    pub allocated: bool,
}

impl DBChunkDescriptor {
    fn null() -> Self {
        Self {
            start: Address(0),
            length: BytesLength(0),
            allocated: false,
        }
    }
}

#[repr(C)]
pub struct DBPointer<T> {
    pub is_null: bool,
    pub chunk: DBChunkDescriptor,
    pub length: ArrayLength,
    pub phantom: PhantomData<T>,
}

impl<T> DBPointer<T> {
    pub fn to_serializable(self) -> SerializableDBPointer<T> {
        SerializableDBPointer {
            is_null: self.is_null,
            chunk: self.chunk,
            length: self.length,
            phantom: self.phantom,
        }
    }
}

/// This is the pointer that gets written to the database file
/// but should not be used as it implements Copy which allows
/// for use after frees. The DBSession::dealloc method normally
/// takes care of this by taking ownership of the pointer but
/// if the pointer has Copy then this breaks and use after free
/// becomes possible.
#[repr(C)]
pub struct SerializableDBPointer<T> {
    pub chunk: DBChunkDescriptor,
    pub length: ArrayLength,
    pub phantom: PhantomData<T>,
    pub is_null: bool,
}

impl<T> Clone for SerializableDBPointer<T> {
    fn clone(&self) -> Self {
        Self {
            chunk: self.chunk.clone(),
            length: self.length,
            phantom: PhantomData,
            is_null: self.is_null,
        }
    }
}

impl<T> SerializableDBPointer<T> {
    pub fn to_ptr(&self) -> DBPointer<T> {
        DBPointer {
            is_null: self.is_null,
            chunk: self.chunk.clone(),
            length: self.length,
            phantom: self.phantom,
        }
    }

    pub fn null() -> SerializableDBPointer<T> {
        SerializableDBPointer {
            is_null: true,
            chunk: DBChunkDescriptor::null(),
            length: ArrayLength(0),
            phantom: PhantomData,
        }
    }
}

#[derive(Savefile, Debug)]
pub struct SaveableDBPointer {
    pub chunk: DBChunkDescriptor,
    pub length: ArrayLength,
}

impl SaveableDBPointer {
    pub fn to_ptr<T>(&self) -> DBPointer<T> {
        DBPointer {
            is_null: false,
            chunk: self.chunk.clone(),
            length: self.length,
            phantom: PhantomData,
        }
    }

    pub fn from_ptr<T>(ptr: DBPointer<T>) -> Self {
        Self {
            chunk: ptr.chunk,
            length: ptr.length,
        }
    }
}

pub trait CopyToDB {
    fn copy_to_db(&self) -> Self;
}

impl<T> CopyToDB for T
where
    T: Clone,
{
    fn copy_to_db(&self) -> Self {
        self.clone()
    }
}

impl DBSession {
    pub fn alloc<T: CopyToDB>(&mut self, value: Vec<T>) -> DBPointer<T> {
        let len = ArrayLength(value.len());

        assert!(len.0 > 0);

        let type_size = BytesLength(std::mem::size_of::<T>());
        let align = std::mem::align_of::<T>();

        let chunk = self.malloc(type_size.times(len), align);

        let mut borrowed = self.borrow_mut_raw::<T>(chunk.start, len);

        for (i, borrowed_item) in borrowed.iter_mut().enumerate() {
            **borrowed_item = value[i].copy_to_db();
        }

        drop(borrowed);

        DBPointer {
            is_null: false,
            chunk,
            length: len,
            phantom: PhantomData,
        }
    }

    pub fn borrow_mut<'a, T>(&'a mut self, ptr: &DBPointer<T>) -> Vec<&'a mut T> {
        assert!(!ptr.is_null);

        let type_size = std::mem::size_of::<T>();
        let buffer_size = ptr.length.0 * type_size;
        assert!(buffer_size <= ptr.chunk.length.0);

        self.borrow_mut_raw(ptr.chunk.start, ptr.length)
    }

    pub fn dealloc<T>(&mut self, ptr: DBPointer<T>) {
        assert!(!ptr.is_null);

        self.free(ptr.chunk);
    }

    fn malloc(&mut self, length: BytesLength, align: usize) -> DBChunkDescriptor {
        let start = self.meta.max_allocated;
        let start = start.align_to_next(align);

        self.meta.max_allocated = Address(start.offset(length).0 + 1);

        let end = start.offset(length);
        let needed_length = BytesLength(end.0);

        if self.capacity <= needed_length {
            self.resize(BytesLength(end.0 + 1024 * 1024));
        }

        assert!(BytesLength(end.0) < self.capacity);

        // self.meta.chunk_descriptors.push(chunk_desc);

        DBChunkDescriptor {
            start,
            length,
            allocated: true,
        }
    }

    fn free(&mut self, _chunk: DBChunkDescriptor) {
        // self.meta
        //     .chunk_descriptors
        //     .iter_mut()
        //     .find(|c| c.start == chunk.start)
        //     .unwrap()
        //     .allocated = false;
    }

    // fn write<T>(&mut self, position: Address, value: T)
    // where
    //     T: Pod,
    // {
    //     let length = std::mem::size_of::<T>();

    //     assert!(BytesLength(position.offset(BytesLength(length)).0) < self.capacity);

    //     let mmap = self.mmap.as_mut().unwrap();

    //     for (i, byte) in bytemuck::bytes_of(&value).iter().enumerate() {
    //         assert!(i < length);
    //         mmap[position.0 + i] = *byte;
    //     }
    // }

    fn borrow_mut_raw<T>(&mut self, position: Address, amount: ArrayLength) -> Vec<&mut T> {
        let item_length = std::mem::size_of::<T>();

        // FIXME: This sucks... Make some helper methods for converting these tuple
        //        structs to each other or something.
        assert!(
            BytesLength(position.offset(BytesLength(item_length * amount.0)).0) < self.capacity,
            "position: {:?}, amount: {:?}, item_length: {:?}, capacity: {:?}",
            position,
            amount,
            item_length,
            self.capacity
        );

        assert!(position.0 % std::mem::align_of::<T>() == 0);

        (0..amount.0)
            .map(|i| {
                let mmap = self.mmap.as_mut().unwrap();
                let ptr = &mmap[position.0 + i * item_length] as *const u8;
                #[allow(invalid_reference_casting)]
                unsafe {
                    &mut *(ptr as *mut T)
                }
            })
            .collect()
    }
}

// fn overlapping_chunks(chunks: &Vec<DBChunkDescriptor>) -> bool {
//     for (i, chunk_i) in chunks.iter().enumerate() {
//         for (j, chunk_j) in chunks.iter().enumerate() {
//             if i == j {
//                 continue;
//             }
//
//             if chunk_j.start >= chunk_i.start
//                 && chunk_j.start < chunk_i.start.offset(chunk_i.length)
//             {
//                 return true;
//             }
//         }
//     }
//     false
// }

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use crate::db::{
        allocator::{Address, ArrayLength, BytesLength},
        session::{meta_path, DBSession},
    };

    fn remove_if_exists(path: &PathBuf) {
        if path.try_exists().unwrap() {
            fs::remove_file(path.clone()).unwrap();
        }
    }

    #[derive(Clone, Copy, Debug)]
    #[allow(unused)]
    struct TestStruct {
        a: i32,
        b: f64,
        c: usize,
    }

    #[test]
    fn basic_allocation() {
        let path = PathBuf::from("test_allocs.db");

        remove_if_exists(&path);
        remove_if_exists(&meta_path(&path));

        let mut session = DBSession::open(path.clone());

        let i32_align = std::mem::align_of::<i32>();

        let buf_1 = session.malloc(BytesLength(10), i32_align);

        assert!(buf_1.start == Address(0));
        assert!(buf_1.length == BytesLength(10));

        let buf_2 = session.malloc(BytesLength(2048), i32_align);

        // assert!(!overlapping_chunks(&session.meta.chunk_descriptors));

        let mut borrowed_ints = session.borrow_mut_raw::<i32>(buf_2.start, ArrayLength(512));

        for (i, borrowed_int) in borrowed_ints.iter_mut().enumerate() {
            **borrowed_int = i as i32;
        }

        drop(session);

        let mut session = DBSession::open(path.clone());

        let borrowed_ints = session.borrow_mut_raw::<i32>(buf_2.start, ArrayLength(512));
        assert!(borrowed_ints[0] == &0);
        assert!(borrowed_ints[123] == &123);
        assert!(borrowed_ints[511] == &511);

        // assert!(session.meta.chunk_descriptors.len() == 2);
        // assert!(session.meta.chunk_descriptors[0].allocated == true);
        // assert!(session.meta.chunk_descriptors[1].allocated == true);
        session.free(buf_1);
        session.free(buf_2);
        // assert!(session.meta.chunk_descriptors[0].allocated == false);
        // assert!(session.meta.chunk_descriptors[1].allocated == false);

        let _ = session.alloc(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        let ptr = session.alloc(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);

        session.dealloc(ptr);

        let _ = session.alloc(vec![TestStruct { a: 1, b: 2.0, c: 3 }]);

        let ptr = session.alloc(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);

        // assert!(!overlapping_chunks(&session.meta.chunk_descriptors));

        drop(session);

        let mut session = DBSession::open(path.clone());

        let mut borrowed = session.borrow_mut(&ptr);

        assert_eq!(borrowed.len(), 10);
        assert_eq!(*borrowed[0], 1);
        assert_eq!(*borrowed[4], 5);
        assert_eq!(*borrowed[9], 10);

        *borrowed[0] = 123;

        drop(session);

        let mut session = DBSession::open(path.clone());

        let borrowed = session.borrow_mut(&ptr);

        assert_eq!(*borrowed[0], 123);

        drop(session);

        fs::remove_file(path.clone()).unwrap();
        fs::remove_file(meta_path(&path)).unwrap();
    }
}
