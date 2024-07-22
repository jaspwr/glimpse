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

use super::{
    allocator::{CopyToDB, SerializableDBPointer},
    session::DBSession,
};

type ListItemPtr<T> = SerializableDBPointer<DBListNode<T>>;

#[repr(C)]
pub struct DBList<T: CopyToDB> {
    pub head: SerializableDBPointer<ListItemPtr<T>>,
}

impl<T: CopyToDB> Clone for DBList<T> {
    fn clone(&self) -> Self {
        Self {
            head: self.head.clone(),
        }
    }
}

#[repr(C)]
pub struct DBListNode<T> {
    pub next: ListItemPtr<T>,
    pub value: T,
}

impl<T: CopyToDB> CopyToDB for DBListNode<T> {
    fn copy_to_db(&self) -> Self {
        Self {
            next: self.next.clone(),
            value: self.value.copy_to_db(),
        }
    }
}

impl<T> DBList<T>
where
    T: CopyToDB,
{
    pub fn new(db: &mut DBSession) -> Self {
        let head = SerializableDBPointer::null();
        let head = db.alloc(vec![head]);
        let head = head.to_serializable();

        Self { head }
    }

    pub fn push(&mut self, db: &mut DBSession, value: T) {
        let current_head = self.fetch_current_head(db);

        let new_head = db.alloc(vec![DBListNode {
            next: current_head,
            value,
        }]);

        self.set_head(db, new_head.to_serializable());
    }

    pub fn remove(&mut self, db: &mut DBSession, cmp: impl Fn(&T, &mut DBSession) -> bool)
    where
        T: Clone,
    {
        let mut current = self.fetch_current_head(db);

        let mut prev = SerializableDBPointer::<DBListNode<T>>::null();

        while !current.is_null {
            let ptr = current.to_ptr();
            let borrowed = db.borrow_mut(&ptr);
            assert!(borrowed.len() == 1);

            let node = &borrowed[0];
            let next_ptr = node.next.clone();

            if cmp(&node.value.clone(), db) {
                if prev.is_null {
                    self.set_head(db, next_ptr);
                } else {
                    let prev_ptr = prev.to_ptr();
                    let mut prev_borrowed = db.borrow_mut(&prev_ptr);
                    assert!(prev_borrowed.len() == 1);

                    let prev_node = &mut prev_borrowed[0];
                    prev_node.next = next_ptr;
                }

                db.dealloc(ptr);
                return;
            }

            prev = current;
            current = next_ptr;
        }
    }

    fn set_head(&mut self, db: &mut DBSession, node: SerializableDBPointer<DBListNode<T>>) {
        let head_ptr = self.head.to_ptr();
        let mut head_borrowed = db.borrow_mut(&head_ptr);
        assert!(head_borrowed.len() == 1);
        *head_borrowed[0] = node;
    }

    fn fetch_current_head(&self, db: &mut DBSession) -> SerializableDBPointer<DBListNode<T>> {
        let head_ptr = self.head.to_ptr();
        let head_borrowed = db.borrow_mut(&head_ptr);
        assert!(head_borrowed.len() == 1);
        head_borrowed[0].clone()
    }

    pub fn iter<'a>(&'a self, db: &'a mut DBSession) -> DBListIter<'a, T>
    where
        T: Clone,
    {
        let current_head = self.fetch_current_head(db);

        DBListIter {
            db,
            current: current_head,
        }
    }
}

pub struct DBListIter<'a, T: Clone> {
    db: &'a mut DBSession,
    current: SerializableDBPointer<DBListNode<T>>,
}

impl<'a, T: Clone> Iterator for DBListIter<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current.is_null {
            return None;
        }

        let ptr = self.current.clone().to_ptr();
        let borrowed = self.db.borrow_mut(&ptr);
        assert!(borrowed.len() == 1);

        let node = &borrowed[0];

        self.current = node.next.clone();
        Some(node.value.clone())
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use crate::db::session::{meta_path, remove_if_exists};

    use super::*;

    #[test]
    fn linked_lists() {
        let path = PathBuf::from("linked_lists.db");

        remove_if_exists(&path);
        remove_if_exists(&meta_path(&path));

        let mut session = DBSession::open(path.clone());

        let mut list = DBList::<u32>::new(&mut session);
        list.push(&mut session, 6);
        list.push(&mut session, 5);
        list.push(&mut session, 4);
        list.push(&mut session, 3);
        list.push(&mut session, 2);
        list.push(&mut session, 1);

        drop(session);

        let mut session = DBSession::open(path.clone());

        for (i, value) in list.iter(&mut session).enumerate() {
            assert_eq!(i as u32 + 1, value);
        }

        list.remove(&mut session, |value, _| *value == 1);

        list.push(&mut session, 99);

        list.remove(&mut session, |value, _| *value == 4);
        list.remove(&mut session, |value, _| *value == 5);

        let list_vec = list.iter(&mut session).collect::<Vec<u32>>();

        assert_eq!(list_vec, vec![99, 2, 3, 6]);

        drop(session);

        fs::remove_file(path.clone()).unwrap();
        fs::remove_file(meta_path(&path)).unwrap();
    }
}
