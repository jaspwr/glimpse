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

use super::{
    allocator::SerializableDBPointer, hashmap::DBHashMap, list::DBList, session::DBSession,
    string::DBString,
};

#[repr(C)]
#[derive(Clone)]
pub struct DBTrie {
    root: SerializableDBPointer<DBTrieNode>,
}

#[repr(C)]
#[derive(Clone)]
pub struct DBTrieNode {
    pub points_to: DBList<DBString>,
    pub children: SerializableDBPointer<DBHashMap<char, SerializableDBPointer<DBTrieNode>>>,
}

impl DBTrie {
    pub fn new(db: &mut DBSession) -> Self {
        let root = DBTrieNode::new(db);
        let root = db.alloc(vec![root]);
        Self {
            root: root.into_serializable(),
        }
    }

    pub fn insert(&mut self, db: &mut DBSession, word: &str, points_to: &str) {
        let word = word.to_lowercase();

        // println!("inserting {:?} -> {:?}", word, points_to);
        let ptr = self.root.to_ptr();
        let borrow = db.borrow_mut(&ptr);
        assert!(borrow.len() == 1);
        let mut root = borrow[0].clone();

        root.insert(db, &word, points_to);

        assert!(self.get(db, &word).iter().any(|s| s == points_to));
    }

    pub fn get(&mut self, db: &mut DBSession, word: &str) -> Vec<String> {
        let word = word.to_lowercase();

        let mut matches = vec![];

        let ptr = self.root.to_ptr();
        let borrow = db.borrow_mut(&ptr);
        assert!(borrow.len() == 1);
        let mut current = borrow[0].clone();

        for c in word.chars() {
            if let Some(next) = current.get_child_from_char(db, c) {
                let ptr = next.to_ptr();
                let borrow = db.borrow_mut(&ptr);
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

        let points_to = current.points_to;

        push_matches(points_to, db, &mut matches);

        matches
    }

    pub fn fuzzy_get(&self, db: &mut DBSession, word: &str) -> Vec<String> {
        let mut matches = vec![];

        fuzzy_get(self.root.clone(), db, &mut matches, word);

        matches
    }
}

fn push_matches(points_to: DBList<DBString>, db: &mut DBSession, matches: &mut Vec<String>) {
    let points_to = points_to.iter(db).collect::<Vec<_>>();

    let points_to = points_to.into_iter().map(|s| s.load_string(db));

    matches.extend(points_to);
}

impl DBTrieNode {
    pub fn new(db: &mut DBSession) -> Self {
        let child_map = DBHashMap::new(db, 1);
        let child_map = db.alloc(vec![child_map]);
        let child_map = child_map.into_serializable();

        Self {
            points_to: DBList::new(db),
            children: child_map,
        }
    }

    pub fn insert(&mut self, db: &mut DBSession, word: &str, points_to: &str) {
        let mut chars = word.chars();

        if let Some(c) = chars.next() {
            let rest = chars.as_str();

            let ptr = self.children.to_ptr();
            let borrow = db.borrow_mut(&ptr);
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
                // End of string

                let str = DBString::new(db, points_to.to_string());

                let ptr = existing_node_.to_ptr();
                let borrow = db.borrow_mut(&ptr);
                assert!(borrow.len() == 1);
                borrow[0].points_to.clone().push(db, str);
            } else {
                let mut new_node = DBTrieNode::new(db);
                new_node.insert(db, rest, points_to);

                let new_node = db.alloc(vec![new_node]);
                let new_node = new_node.into_serializable();

                children.insert(db, c, new_node);
            }
        } else {
            let str = DBString::new(db, points_to.to_string());
            self.points_to.push(db, str);
        }
    }

    // Fuzzy get
    // Incorrect char correction
    //     if there are no nodes for current char but number of child nodes
    //     is low branch into all of them.
    // Finish prefixes
    //     if finished all chars but there are still child nodes, branch
    //     into all of them
    // Extra char correction
    //     if there are very few or no nodes for the current char,
    //     try the next char
    // Missing char correction
    //     if there are no nodes for the current char, branch into
    //     all of other children with the current char

    pub fn fuzzy_get(&self, db: &mut DBSession, word: &str, matches: &mut Vec<String>) {
        if matches.len() > 20 {
            return;
        }

        let mut chars = word.chars();

        let children = self.children.to_ptr();
        let children = db.borrow_mut(&children);
        assert!(children.len() == 1);
        let children = children[0].clone();

        if let Some(c) = chars.next() {
            let rest = chars.as_str();

            if let Some(node) = self.get_child_from_char(db, c) {
                fuzzy_get(node, db, matches, rest);
            } else {
                // There were no nodes for the current char.

                if children.len(db) < 4 {
                    for (_, child) in children.iter(db) {
                        fuzzy_get(child, db, matches, rest)
                    }
                }

                // Correct extra char
                if !rest.is_empty() {
                    self.fuzzy_get(db, rest, matches);
                }
            }
        } else {
            self.get_all_matches(db, matches);
        }
    }

    fn get_all_matches(&self, db: &mut DBSession, matches: &mut Vec<String>) {
        let children = self.children.to_ptr();
        let children = db.borrow_mut(&children);
        assert!(children.len() == 1);
        let children = children[0].clone();

        let ptr = self.points_to.clone();
        push_matches(ptr, db, matches);

        // if children.len(db) > 3 {
        //     return;
        // }

        let children = children.flatten(db);

        for (_, child) in children {
            if matches.len() > 20 {
                return;
            }

            let child = child.to_ptr();
            let borrow = db.borrow_mut(&child);
            assert!(borrow.len() == 1);
            let child = borrow[0].clone();
            child.get_all_matches(db, matches);
        }
    }

    pub fn get_child_from_char(
        &self,
        db: &mut DBSession,
        c: char,
    ) -> Option<SerializableDBPointer<DBTrieNode>> {
        let ptr = self.children.to_ptr();
        let borrow = db.borrow_mut(&ptr);
        assert!(borrow.len() == 1);
        let mut children = borrow[0].clone(); // Only contains a pointer so can be cloned

        children.get(db, c)
    }
}

fn fuzzy_get(
    node: SerializableDBPointer<DBTrieNode>,
    db: &mut DBSession,
    matches: &mut Vec<String>,
    rest: &str,
) {
    let ptr = node.to_ptr();
    let borrow = db.borrow_mut(&ptr);
    assert!(borrow.len() == 1);
    let node = borrow[0].clone();

    node.fuzzy_get(db, rest, matches);
}

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

        assert_eq!(trie.get(&mut session, "hello")[0], "world".to_string());
        assert_eq!(trie.get(&mut session, "help")[0], "asdhjkl".to_string());

        drop(session);

        fs::remove_file(path.clone()).unwrap();
        fs::remove_file(meta_path(&path)).unwrap();
    }
}
