use bytemuck::{Pod, Zeroable};
use pango::glib::ffi::G_NORMALIZE_DEFAULT_COMPOSE;

use super::{allocator::SerializableDBPointer, session::DBSession};

type ListItemPtr<T> = SerializableDBPointer<DBListNode<T>>;

#[derive(Clone, Copy)]
pub struct DBList<T>
where
    T: Copy + 'static,
{
    pub head: SerializableDBPointer<ListItemPtr<T>>,
}

unsafe impl<T> Zeroable for DBList<T> where T: Copy + 'static {}
unsafe impl<T> Pod for DBList<T> where T: Copy + 'static {}

#[derive(Clone, Copy)]
pub struct DBListNode<T>
where
    T: Copy + 'static,
{
    pub next: ListItemPtr<T>,
    pub value: T,
}

unsafe impl<T> Zeroable for DBListNode<T> where T: Copy + 'static {}
unsafe impl<T> Pod for DBListNode<T> where T: Copy + 'static {}

impl<T: Copy + 'static> DBList<T> {
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

    pub fn remove(&mut self, db: &mut DBSession, cmp: Box<dyn Fn(T) -> bool>) {
        let mut current = self.fetch_current_head(db);

        let mut prev = SerializableDBPointer::<DBListNode<T>>::null();

        while !current.is_null {
            let ptr = current.to_ptr();
            let borrowed = db.borrow_mut(&ptr);
            assert!(borrowed.len() == 1);

            let node = &borrowed[0].clone();

            if cmp(node.value) {
                if prev.is_null {
                    self.set_head(db, node.next);
                } else {
                    let prev_ptr = prev.to_ptr();
                    let mut prev_borrowed = db.borrow_mut(&prev_ptr);
                    assert!(prev_borrowed.len() == 1);

                    let prev_node = &mut prev_borrowed[0];
                    prev_node.next = node.next;
                }

                db.dealloc(ptr);
                return;
            }

            prev = current;
            current = node.next;
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
        let mut head_borrowed = db.borrow_mut(&head_ptr);
        assert!(head_borrowed.len() == 1);
        head_borrowed[0].clone()
    }

    pub fn iter<'a>(&'a self, db: &'a mut DBSession) -> DBListIter<'a, T> {
        let current_head = self.fetch_current_head(db);

        DBListIter {
            db,
            current: current_head,
        }
    }
}

pub struct DBListIter<'a, T>
where
    T: Copy + 'static,
{
    db: &'a mut DBSession,
    current: SerializableDBPointer<DBListNode<T>>,
}

impl<'a, T: Copy + 'static> Iterator for DBListIter<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current.is_null {
            return None;
        }

        let ptr = self.current.to_ptr();
        let borrowed = self.db.borrow_mut(&ptr);
        assert!(borrowed.len() == 1);

        let node = &borrowed[0];

        self.current = node.next;
        Some(node.value)
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

        list.remove(&mut session, Box::new(|value| value == 1));

        list.push(&mut session, 99);

        list.remove(&mut session, Box::new(|value| value == 4));
        list.remove(&mut session, Box::new(|value| value == 5));

        let list_vec = list.iter(&mut session).collect::<Vec<u32>>();

        assert_eq!(list_vec, vec![99, 2, 3, 6]);

        drop(session);

        fs::remove_file(path.clone()).unwrap();
        fs::remove_file(meta_path(&path)).unwrap();
    }
}
