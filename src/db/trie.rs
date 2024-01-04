use bytemuck::{Pod, Zeroable};

use crate::prelude::Relevance;

use super::{
    allocator::{DBPointer, SerializableDBPointer},
    hashmap::DBHashMap,
    session::DBSession,
    string::DBString,
};

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

        assert!(self.get(db, word).iter().any(|(s, _)| s == points_to));
    }

    pub fn get(&mut self, db: &mut DBSession, word: &str) -> Vec<(String, Relevance)> {
        let mut matches = vec![];

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
                return matches;
            }
        }

        // while current.points_to.is_null {
        //     let children = current.children.to_ptr();
        //     let mut borrow = db.borrow_mut(&children);
        //     assert!(borrow.len() == 1);
        //     let children = borrow[0].clone();

        //     if children.len() != 1 {
        //         break;
        //     }


        // }

        let mut ptr = current.points_to.to_ptr();

        push_string(ptr, 20.0, db, &mut matches);

        return matches;
    }
}

fn push_string(
    ptr: DBPointer<DBString>,
    relevance: Relevance,
    db: &mut DBSession,
    matches: &mut Vec<(String, f32)>,
) {
    if ptr.is_null {
        return;
    }

    let mut borrow = db.borrow_mut(&ptr);
    assert!(borrow.len() == 1);
    let points_to = borrow[0].clone();

    matches.push((points_to.load_string(db), relevance));
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

            if let Some(existing_node_) = children.get(db, c) {
                if !chars.as_str().is_empty() {
                    let ptr = existing_node_.to_ptr();
                    let borrow = db.borrow_mut(&ptr);
                    assert!(borrow.len() == 1);
                    borrow[0].clone().insert(db, rest, points_to);
                    return;
                }

                let points_to = allocate_string(db, points_to);

                let ptr = existing_node_.to_ptr();
                let mut borrow = db.borrow_mut(&ptr);
                assert!(borrow.len() == 1);
                borrow[0].points_to = points_to;

            } else {
                let mut new_node = DBTrieNode::new(db);
                new_node.insert(db, rest, points_to);

                let new_node = db.alloc(vec![new_node]);
                let new_node = new_node.to_serializable();

                children.insert(db, c, new_node);
            }
        } else {
            let points_to = allocate_string(db, points_to);
            self.points_to = points_to;
        }
    }

    pub fn get(
        &mut self,
        db: &mut DBSession,
        c: char,
    ) -> Option<SerializableDBPointer<DBTrieNode>> {
        let ptr = self.children.to_ptr();
        let mut borrow = db.borrow_mut(&ptr);
        assert!(borrow.len() == 1);
        let mut children = borrow[0].clone(); // Only contains a pointer so can be cloned

        children.get(db, c)
    }
}

fn allocate_string(db: &mut DBSession, points_to: &str) -> SerializableDBPointer<DBString> {
    let points_to = DBString::new(db, points_to.to_string());
    let points_to = db.alloc(vec![points_to]);
    let points_to = points_to.to_serializable();
    points_to
}

unsafe impl Zeroable for DBTrieNode {}
unsafe impl Pod for DBTrieNode {}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use crate::db::session::{meta_path, remove_if_exists, DBSession};

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

        assert_eq!(trie.get(&mut session, "hello")[0].0, "world".to_string());
        assert_eq!(trie.get(&mut session, "help")[0].0, "asdhjkl".to_string());

        drop(session);

        fs::remove_file(path.clone()).unwrap();
        fs::remove_file(meta_path(&path)).unwrap();
    }
}
