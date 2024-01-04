use bytemuck::{Pod, Zeroable};

use super::{allocator::SerializableDBPointer, hashmap::DBHashMap, string::DBString, session::DBSession};

#[derive(Clone, Copy)]
pub struct DBTrie {
    root: SerializableDBPointer<DBTrieNode>,
}

unsafe impl Zeroable for DBTrie {}
unsafe impl Pod for DBTrie {}

#[derive(Clone, Copy)]
pub struct DBTrieNode {
    pub points_to: SerializableDBPointer<DBString>,
    pub children: SerializableDBPointer<DBHashMap<char, SerializableDBPointer<DBTrieNode>>>,
}

impl DBTrie {
    pub fn new(db: &mut DBSession) -> Self {
        let root = DBTrieNode::new(db);
        let root = db.alloc(vec![root]);
        Self {
            root: root.to_serializable(),
        }
    }

    pub fn insert(&mut self, db: &mut DBSession, word: &str, points_to: &str) {
        println!("inserting {:?} -> {:?}", word, points_to);
        let ptr = self.root.to_ptr();
        let mut borrow = db.borrow_mut(&ptr);
        assert!(borrow.len() == 1);
        let mut root = borrow[0].clone();

        root.insert(db, word, points_to);
    }

    pub fn get(&mut self, db: &mut DBSession, word: &str) -> Vec<String> {
        let ptr = self.root.to_ptr();
        let mut borrow = db.borrow_mut(&ptr);
        assert!(borrow.len() == 1);
        let mut current = borrow[0].clone();

        for c in word.chars() {
            if let Some(next) = current.get(db, c) {
                let ptr = next.to_ptr();
                let mut borrow = db.borrow_mut(&ptr);
                assert!(borrow.len() == 1);
                current = borrow[0].clone();


            } else {
                return vec![];
            }
        }

        let ptr = current.points_to.to_ptr();

        if ptr.is_null {
            return vec![];
        }
        let mut borrow = db.borrow_mut(&ptr);
        assert!(borrow.len() == 1);
        let points_to = borrow[0].clone();


        vec![points_to.load_string(db)]
    }
}

impl DBTrieNode {
    pub fn new(db: &mut DBSession) -> Self {
        let child_map = DBHashMap::new(db, 1);
        let child_map = db.alloc(vec![child_map]);
        let child_map = child_map.to_serializable();

        Self {
            points_to: SerializableDBPointer::null(),
            children: child_map,
        }
    }

    pub fn insert(&mut self, db: &mut DBSession, word: &str, points_to: &str) {
        let mut chars = word.chars();

        if let Some(c) = chars.next() {
            let rest = chars.as_str();

            let ptr = self.children.to_ptr();
            let mut borrow = db.borrow_mut(&ptr);
            assert!(borrow.len() == 1);
            let mut children = borrow[0].clone(); // Only contains a pointer so can be cloned

            if let Some(existing_node) = children.get(db, c) {
                let ptr = existing_node.to_ptr();
                let mut borrow = db.borrow_mut(&ptr);
                assert!(borrow.len() == 1);
                let mut existing_node = borrow[0].clone();

                existing_node.insert(db, rest, points_to);
            } else {
                let mut new_node = DBTrieNode::new(db);
                new_node.insert(db, rest, points_to);

                let new_node = db.alloc(vec![new_node]);
                let new_node = new_node.to_serializable();

                children.insert(db, c, new_node);
            }
        } else {
            let points_to = DBString::new(db, points_to.to_string());
            let points_to = db.alloc(vec![points_to]);
            let points_to = points_to.to_serializable();

            self.points_to = points_to;
        }
    }

    pub fn get(&mut self, db: &mut DBSession, c: char) -> Option<SerializableDBPointer<DBTrieNode>> {
        let ptr = self.children.to_ptr();
        let mut borrow = db.borrow_mut(&ptr);
        assert!(borrow.len() == 1);
        let mut children = borrow[0].clone(); // Only contains a pointer so can be cloned

        children.get(db, c)
    }
}

unsafe impl Zeroable for DBTrieNode {}
unsafe impl Pod for DBTrieNode {}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, fs};

    use crate::db::session::{remove_if_exists, meta_path, DBSession};

    use super::*;

    #[test]
    fn tries() {
        let path = PathBuf::from("tries.db");

        remove_if_exists(&path);
        remove_if_exists(&meta_path(&path));

        let mut session = DBSession::open(path.clone());

        let mut trie = DBTrie::new(&mut session);

        trie.insert(&mut session, "hello", "world");
        trie.insert(&mut session, "help", "asdhjkl");

        assert_eq!(trie.get(&mut session, "hello"), vec!["world".to_string()]);
        assert_eq!(trie.get(&mut session, "help"), vec!["asdhjkl".to_string()]);

        drop(session);

        fs::remove_file(path.clone()).unwrap();
        fs::remove_file(meta_path(&path)).unwrap();
    }
}