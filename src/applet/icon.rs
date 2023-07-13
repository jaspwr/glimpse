use gtk::traits::IconThemeExt;
use prober::config::CONF;

pub fn from_file(path: &String) -> Option<gtk::Image> {
    let file = std::fs::File::open(path.clone());
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

pub fn from_gtk(path: &str) -> Option<gtk::Image> {
    let theme = gtk::IconTheme::default().unwrap();
    let icon_info = theme.lookup_icon(path, 32, gtk::IconLookupFlags::FORCE_SIZE);
    if let Some(icon_info) = icon_info {
        let pixbuf = gtk::gdk::gdk_pixbuf::Pixbuf::from_file_at_size(
            icon_info.filename().unwrap().to_str().unwrap(),
            CONF.visual.icon_size as i32,
            CONF.visual.icon_size as i32,
        );
        if let Ok(pixbuf) = pixbuf {
            return Some(gtk::Image::from_pixbuf(Some(&pixbuf)));
        }
    }
    None
}
