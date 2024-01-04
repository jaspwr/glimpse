use bytemuck::{Zeroable, Pod};

use super::{
    allocator::{DBPointer, SerializableDBPointer},
    session::DBSession,
};

#[derive(Clone, Copy)]
pub struct DBString(SerializableDBPointer<u8>);

unsafe impl Zeroable for DBString {}
unsafe impl Pod for DBString {}

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
            .iter()
            .map(|b| **b.clone())
            .collect::<Vec<u8>>();
        String::from_utf8(bytes).unwrap()
    }

    pub fn dealloc(&self, db: &mut DBSession) {
        db.dealloc(self.0.to_ptr());
    }
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, fs};

    use crate::db::session::{remove_if_exists, meta_path};

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