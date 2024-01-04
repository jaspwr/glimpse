use bytemuck::{Pod, Zeroable};

use super::*;
unsafe impl<T> Zeroable for DiskLinkedList<T> {}
unsafe impl<T: Copy + 'static> Pod for DiskLinkedList<T> {}

#[derive(Clone, Copy)]
pub struct DiskLinkedList<T> {
    head: DbChunk<DbChunk<LinkedListNode<T>>>,
}

unsafe impl<T> Zeroable for LinkedListNode<T> {}
unsafe impl<T: Copy + 'static> Pod for LinkedListNode<T> {}

#[derive(Clone, Copy)]
pub struct LinkedListNode<T> {
    pub next: DbChunk<LinkedListNode<T>>,
    pub data: T,
}

impl<T: Copy + 'static> DiskLinkedList<T> {
    pub fn new(db: &mut DataBase) -> Self {
        let head = db.malloc::<DbChunk<LinkedListNode<T>>>(1);

        head.write(db, 0, null_ptr());
        println!("init linked list");
        Self { head }
    }

    pub fn push(&self, db: &mut DataBase, data: T) {
        let new_head = db.malloc::<LinkedListNode<T>>(1);

        if self.head.is_null {
            new_head.write(db, 0, LinkedListNode { next: null_ptr(), data });
            self.head.write(db, 0, new_head);
            return;
        }

        let head = *self.head.fetch(db, 0);

        println!("E");

        new_head.write(db, 0, LinkedListNode { next: head, data });

        println!("F");
        self.head.write(db, 0, new_head);
        println!("G");
    }

    pub fn iter<'a>(&'a self, db: &'a DataBase) -> DiskLinkedListIter<'a, T> {
        DiskLinkedListIter {
            db,
            current: *self.head.fetch(db, 0),
        }
    }

    pub fn remove(&self, db: &mut DataBase, cmp: Box<dyn Fn(T) -> bool>) {
        if self.head.is_null {
            return;
        }

        let mut current = *self.head.fetch(db, 0);
        let mut prev: DbChunk<LinkedListNode<T>> = null_ptr();

        while !current.is_null {
            let node = current.fetch(db, 0).clone();
            if cmp(node.data) {
                if prev.is_null {
                    self.head.write(db, 0, node.next);
                } else {
                    let mut prev_node = prev.fetch(db, 0).clone();
                    prev_node.next = node.next;
                    prev.write(db, 0, prev_node);
                }
                db.free(current);
            }
            prev = current;
            current = node.next;
        }
    }
}

pub struct DiskLinkedListIter<'a, T> {
    db: &'a DataBase,
    current: DbChunk<LinkedListNode<T>>,
}

impl<'a, T: Copy + 'static> Iterator for DiskLinkedListIter<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        println!("next");

        let current = self.current;
        if current.is_null {
            return None;
        }

        let node = current.fetch(self.db, 0);
        self.current = node.next;
        Some(node.data)
    }
}
