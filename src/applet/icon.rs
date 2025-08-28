// Glimpse - GNU/Linux launcher and file search utility.
// Copyright (C) 2024 https://github.com/jaspwr

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::{borrow::Cow, fs};

use gdk::gdk_pixbuf;
use glimpse::config::CONF;
use gtk::traits::IconThemeExt;

use crate::utils::is_cli_app;

pub fn from_file(path: &String) -> Option<gtk::Image> {
    if !CONF.visual.show_icons {
        return None;
    }

    let pixbuf = gdk_pixbuf::Pixbuf::from_file(path).ok()?;
    let pixbuf = pixbuf
        .scale_simple(
            CONF.visual.icon_size as i32,
            CONF.visual.icon_size as i32,
            gdk_pixbuf::InterpType::Bilinear,
        )
        .unwrap();
    Some(gtk::Image::from_pixbuf(Some(&pixbuf)))
}

pub fn find_application_icon(name: &str) -> Option<gtk::Image> {
    let mut icon = find_application_icon_inner(name);

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

fn find_application_icon_inner(name: &str) -> Option<gtk::Image> {
    if !CONF.visual.show_icons {
        return None;
    }

    let home_dir = std::env::var("HOME").unwrap().to_string();

    let possible_locations = [
        Cow::Borrowed("/usr/share/pixmaps"),
        Cow::Borrowed("/usr/share/icons/hicolor/32x32/apps/"),
        Cow::Borrowed("/usr/share/icons/hicolor/symbolic/apps"),
        Cow::Borrowed("/usr/share/icons/hicolor/scalable/apps"),
    ]
        .into_iter()
        .chain(fs::read_dir(home_dir + "/.icons")
            .into_iter()
            .flat_map(|paths| paths
                .into_iter()
                .flat_map(|path| {
                    let path = path.unwrap().path();
                    path.is_dir()
                        .then(|| {
                            let path = path.to_str().unwrap().to_string();
                            Cow::Owned(path + "/32x32/apps")
                        })
                })
            )
        );

    const POSSIBLE_EXTENSIONS: [&str; 2] = [".png", ".svg"];

    for path in possible_locations {
        let mut path = path.into_owned();
        path.push('/');
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
    None
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
