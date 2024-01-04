use super::*;

#[derive(Clone, Copy)]
pub struct DiskString(DbChunk<u8>);

impl DiskString {
    pub fn new(db: &mut DataBase, str: String) -> Self {
        let bytes = str.as_bytes();
        let len = bytes.len();
        let chunk = db.malloc::<u8>(len);
        for (i, byte) in bytes.iter().enumerate() {
            chunk.write(db, i, *byte);
        }
        Self(chunk)
    }

    pub fn string(&self, db: &DataBase) -> String {
        let mut bytes = vec![];
        for i in 0..self.0.descriptor.length {
            bytes.push(*self.0.fetch(db, i));
        }
        String::from_utf8(bytes).unwrap()
    }
}

