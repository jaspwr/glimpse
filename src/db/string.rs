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

use std::{collections::hash_map::DefaultHasher, hash::Hasher};

use super::{
    allocator::SerializableDBPointer,
    hashmap::{EqWithDBAccess, HashWithDBAccess},
    session::DBSession,
};

#[repr(C)]
#[derive(Clone)]
pub struct DBString(SerializableDBPointer<u8>);

impl DBString {
    pub fn new(db: &mut DBSession, str: String) -> Self {
        let chars = str.bytes().collect::<Vec<u8>>();
        let chunk = db.alloc(chars);
        Self(chunk.to_serializable())
    }

    pub fn load_string(&self, db: &mut DBSession) -> String {
        let ptr = self.0.to_ptr();
        let bytes = db
            .borrow_mut(&ptr)
            .into_iter()
            .map(|b| *b)
            .collect::<Vec<u8>>();
        String::from_utf8(bytes).unwrap()
    }

    pub fn dealloc(&self, db: &mut DBSession) {
        db.dealloc(self.0.to_ptr());
    }
}

impl HashWithDBAccess for DBString {
    fn hash(&self, db: &mut DBSession) -> u64 {
        let str = self.load_string(db);
        let mut hasher = DefaultHasher::new();
        std::hash::Hash::hash(&str, &mut hasher);
        hasher.finish()
    }
}

impl EqWithDBAccess for DBString {
    fn eq(&self, other: &Self, db: &mut DBSession) -> bool {
        let lhs = self.load_string(db);
        let rhs = other.load_string(db);
        lhs == rhs
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use crate::db::session::{meta_path, remove_if_exists};

    use super::*;

    #[test]
    fn test_db_string() {
        let path = PathBuf::from("test_string.db");

        remove_if_exists(&path);
        remove_if_exists(&meta_path(&path));

        let mut session = DBSession::open(path.clone());

        let string = String::from("Hello! Γειά σου! नमस्ते! 你好！");
        let db_string = DBString::new(&mut session, string.clone());

        drop(session);

        let mut session = DBSession::open(path.clone());

        let loaded_string = db_string.load_string(&mut session);

        db_string.dealloc(&mut session);

        assert_eq!(string, loaded_string);

        drop(session);

        fs::remove_file(path.clone()).unwrap();
        fs::remove_file(meta_path(&path)).unwrap();
    }
}
