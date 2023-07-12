use std::path::PathBuf;

use crate::CONF;

use gdk::glib::once_cell::sync::Lazy;

pub static PATH: Lazy<PathBuf> = Lazy::new(|| {
    let path = PathBuf::from(CONF.indexing.location.clone());

    if !path.exists() {
        std::fs::create_dir_all(&path).unwrap();
    }

    path
});
