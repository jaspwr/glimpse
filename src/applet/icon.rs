use gdk::gdk_pixbuf;
use gtk::traits::IconThemeExt;
use pango::glib;
use prober::config::CONF;

pub fn from_file(path: &String) -> Option<gtk::Image> {
    if !CONF.visual.show_icons {
        return None;
    }

    let file = std::fs::File::open(path.clone());
    if file.is_ok() {
        let pixbuf = gdk_pixbuf::Pixbuf::from_file(path).unwrap();
        let pixbuf = pixbuf
            .scale_simple(
                CONF.visual.icon_size as i32,
                CONF.visual.icon_size as i32,
                gdk_pixbuf::InterpType::Bilinear,
            )
            .unwrap();
        return Some(gtk::Image::from_pixbuf(Some(&pixbuf)));
    }
    None
}

pub fn from_bytes(data: &[u8]) -> Option<gtk::Image> {
    if !CONF.visual.show_icons {
        return None;
    }

    let data = glib::Bytes::from(data);

    let height = 16;
    let width = 16;

    let bits_per_sample = 8;
    let row_stride = bits_per_sample / 8 * width;
    let pixbuf = gdk_pixbuf::Pixbuf::from_bytes(
        &data,
        gdk_pixbuf::Colorspace::Rgb,
        true,
        bits_per_sample as i32,
        width,
        height,
        row_stride as i32,
    );
    let pixbuf = pixbuf
        .scale_simple(
            CONF.visual.icon_size as i32,
            CONF.visual.icon_size as i32,
            gdk_pixbuf::InterpType::Bilinear,
        )?;
    Some(gtk::Image::from_pixbuf(Some(&pixbuf)))
}

pub fn from_gtk(path: &str) -> Option<gtk::Image> {
    if !CONF.visual.show_icons {
        return None;
    }

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
