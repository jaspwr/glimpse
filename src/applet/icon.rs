use prober::config::CONF;

pub fn from_file(path: &String) -> Option<gtk::Image> {
    let file = std::fs::File::open(path.clone());
    println!("path: {}", path);
    if file.is_ok() {
        let pixbuf = gtk::gdk::gdk_pixbuf::Pixbuf::from_file(path).unwrap();
        let pixbuf = pixbuf
            .scale_simple(
                CONF.visual.icon_size as i32,
                CONF.visual.icon_size as i32,
                gtk::gdk_pixbuf::InterpType::Bilinear,
            )
            .unwrap();
        return Some(gtk::Image::from_pixbuf(Some(&pixbuf)));
    }
    None
}
