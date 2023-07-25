use std::fs;

use gdk::gdk_pixbuf;
use gtk::traits::IconThemeExt;
use prober::config::CONF;

use crate::utils::is_cli_app;

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

pub fn find_application_icon(name: &String) -> Option<gtk::Image> {
    let mut icon = __find_application_icon(name);

    if icon.is_none() {
        let icon_str = if is_cli_app(name) {
            "utilities-terminal"
        } else {
            "application-x-executable"
        };
        icon = from_gtk(icon_str);
    }
    icon
}

fn __find_application_icon(name: &String) -> Option<gtk::Image> {
    let mut possible_locations = vec![
        "/usr/share/pixmaps".to_string(),
        "/usr/share/icons/hicolor/32x32/apps/".to_string(),
        "/usr/share/icons/hicolor/symbolic/apps".to_string(),
        "/usr/share/icons/hicolor/scalable/apps".to_string(),
    ];

    let home_dir = std::env::var("HOME").unwrap().to_string();

    if let Ok(paths) = fs::read_dir(home_dir + "/.icons") {
        for path in paths {
            let path = path.unwrap().path();
            if path.is_dir() {
                let path = path.to_str().unwrap().to_string();
                possible_locations.push(path + "/32x32/apps");
            }
        }
    }

    const POSSIBLE_EXTENSIONS: [&str; 2] = [".png", ".svg"];

    for path in possible_locations.iter() {
        let mut path = path.clone();
        path.push_str("/");
        path.push_str(name);
        for extension in POSSIBLE_EXTENSIONS.iter() {
            let mut path = path.clone();
            path.push_str(extension);

            let i = from_file(&path);
            if i.is_some() {
                return i;
            }
        }
    }
    return None;
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
