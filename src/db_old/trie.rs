use super::{hashmap::DiskHashMap, string::DiskString, *};

impl TrieNode {
    pub fn new(db: &mut DataBase) -> Self {
        let hashmap = DiskHashMap::new(db, 10);
        let hashmap_chunk = db.malloc::<DiskHashMap<char, DbChunk<TrieNode>>>(1);
        hashmap_chunk.write(db, 0, hashmap);

        Self {
            children: hashmap_chunk,
            is_word: false,
            points_to: None,
        }
    }

    pub fn insert(
        &mut self,
        db: &mut DataBase,
        word: String,
        points_to: Option<String>,
    ) {
        println!("inserting {:?}", word);
        let mut chars = word.chars();

        if let Some(c) = chars.next() {
            println!("c: {:?}", c);
            if let Some(child) = self.children.fetch(db, 0).get(db, &c) {
                println!("A");
                let mut child_ = child.fetch(db, 0).clone();
                // This clone probably causes issues
                child_.insert(db, chars.collect(), points_to);
                child.write(db, 0, child_);
                // self.children.fetch(db, 0)?.insert(db, c, child)?;
            } else {
                println!("B");
                let mut child = TrieNode::new(db);
                child.insert(db, chars.collect(), points_to);

                let child_chunk = db.malloc::<TrieNode>(1);
                child_chunk.write(db, 0, child);

                // println!("C");
                let mut children = self.children.fetch(db, 0).clone();

                // println!("D");
                children.insert(db, c, child_chunk);

                // self.children.write(db, 0, children);
            }
        } else {
            self.is_word = true;

            self.points_to = points_to.and_then(|pt| Some(DiskString::new(db, pt)));
        }

        println!("inserted {:?}", word);
    }
}

trait AllContentsBehindRef {}

unsafe impl Zeroable for DiskTrie {}
unsafe impl Pod for DiskTrie {}

#[derive(Clone, Copy)]
pub struct DiskTrie {
    root: DbChunk<TrieNode>,
}

impl AllContentsBehindRef for DiskTrie {}

unsafe impl Zeroable for TrieNode {}
unsafe impl Pod for TrieNode {}

#[derive(Clone, Copy)]
pub struct TrieNode {
    pub children: DbChunk<DiskHashMap<char, DbChunk<TrieNode>>>,
    pub is_word: bool,
    pub points_to: Option<DiskString>,
}

impl DiskTrie {
    pub fn new(db: &mut DataBase) -> Option<Self> {
        println!("Creating new trie");
        let root = db.malloc::<TrieNode>(1);
        let node = TrieNode::new(db);
        root.write(db, 0, node);
        Some(Self { root })
    }

    pub fn insert(
        &mut self,
        db: &mut DataBase,
        word: String,
        points_to: Option<String>,
    ){
        println!("inserting {:?} -> {:?}", word, points_to);
        let mut root = self.root.fetch(db, 0).clone();
        println!("root fetched");
        root.insert(db, word, points_to);
    }

    pub fn get(&self, db: &DataBase, word: &str) -> Vec<(String, f32)> {
        let mut ret = vec![];

        let mut chars = word.chars();
        let mut current = self.root.fetch(db, 0).clone();

        let mut str_buf = String::new();

        while let Some(c) = chars.next() {
            str_buf.push(c);

            if let Some(child) = current.children.fetch(db, 0).get(db, &c) {
                current = child.fetch(db, 0).clone();
            } else {
                return ret;
            }
        }

        if current.is_word {
            let relevance = 20.0;
            append_find(current, &mut ret, db, relevance, &mut str_buf);
        } else {
            while current.children.fetch(db, 0).length.fetch(db, 0) == &1 {
                let bucket_id = current.children.fetch(db, 0).last_bucket_written_to.fetch(db, 0);
                let (c, child) = current
                    .children
                    .fetch(db, 0)
                    .buckets
                    .fetch(db, *bucket_id)
                    .iter(db)
                    .next().unwrap();

                str_buf.push(c);

                current = child.fetch(db, 0).clone();
                if current.is_word {
                    let relevance = 20.0;
                    append_find(current, &mut ret, db, relevance, &mut str_buf);
                }
            }
        }

       ret
    }
}

fn append_find(
    current: TrieNode,
    ret: &mut Vec<(String, f32)>,
    db: &DataBase,
    relevance: f32,
    str_buf: &mut String,
) {
    if let Some(points_to) = current.points_to {
        ret.push((points_to.string(db), relevance));
    } else {
        ret.push((str_buf.clone(), relevance));
    }
    *str_buf = String::new();
}
