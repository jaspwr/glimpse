use std::{path::PathBuf, collections::HashMap};

use gdk::glib::{once_cell::sync::Lazy};
use savefile_derive::Savefile;

use crate::config::CONF;

pub static PATH: Lazy<PathBuf> = Lazy::new(|| {
    let path = PathBuf::from(CONF.indexing.location.clone());

    if !path.exists() {
        std::fs::create_dir_all(&path).unwrap();
    }

    path
});

#[derive(Savefile)]
pub struct Index {
    pub files: Vec<String>,
    pub dirs: Vec<String>,
    pub tf_idf: HashMap<String, Vec<(PathBuf, f32)>>,
}

impl Index {
    pub fn save(&self, name: &str) {
        let path = PATH.join(name).with_extension("bin");
        let mut file = std::fs::File::create(path).unwrap();
        savefile::save(&mut file, 0, self).unwrap();
    }

    pub fn load(name: &str) -> Option<Index> {
        match std::fs::File::open(PATH.join(name).with_extension("bin")) {
            Ok(mut file) => match savefile::load(&mut file, 0) {
                Ok(index) => Some(index),
                Err(_) => None,
            },
            Err(_) => None,
        }
    }
}
