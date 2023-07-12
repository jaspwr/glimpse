use std::process::{Command, Stdio};


use async_trait::async_trait;
use execute::Execute;
use gtk::traits::ContainerExt;

use crate::{result_templates::standard_entry, search::string_search};

use super::{SearchModule, SearchResult};

pub struct Commands {
    apps: Vec<String>
}

impl Commands {
    pub fn new() -> Commands {
        Commands {
            apps: get_list().unwrap()
        }
    }
}

#[async_trait]
impl SearchModule for Commands {
    async fn search(&self, query: String, max_results: u32) -> Vec<SearchResult> {
        string_search(&query, &self.apps, max_results)
            .into_iter()
            .map(|s| create_result(s))
            .collect()
    }
}

fn create_result(name: String) -> SearchResult {
    let name_cpy = name.clone();
    let render = move || {
        let name = name_cpy.clone();
        let icon = find_icon(&name);
        standard_entry(name, icon, None)
    };
    let run = move || {
        execute_detached(name.clone());
    };
    SearchResult {
        render: Box::new(render),
        relevance: 1.0,
        on_select: Some(Box::new(run)),
    }
}

fn execute_detached(name: String) {
    let mut command = Command::new("bash");
    command.arg("-c");
    command.arg(format!("{} & disown", name));
    command.execute();
}

fn find_icon(name: &String) -> Option<gtk::Image> {
    const possible_locations: [&str; 2] = [
        "/usr/share/pixmaps",
        "/usr/share/icons/hicolor/32x32/apps/",
    ];

    for location in possible_locations.iter() {
        let mut path = String::from(*location);
        path.push_str("/");
        path.push_str(name);
        path.push_str(".png");

        let file = std::fs::File::open(path.clone());
        if file.is_ok() {
            let pixbuf = gtk::gdk::gdk_pixbuf::Pixbuf::from_file(path).unwrap();
            let pixbuf = pixbuf.scale_simple(32, 32, gtk::gdk_pixbuf::InterpType::Bilinear).unwrap();
            return Some(gtk::Image::from_pixbuf(Some(&pixbuf)));
        }
    }
    return None;
}

fn get_list() -> Result<Vec<String>, ()> {
    let mut command = Command::new("bash");
    command.arg("-c");
    command.arg("compgen -ac");

    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let output = command.execute_output().unwrap();

    let output = String::from_utf8(output.stdout).unwrap();
    let lines = output.lines()
        .into_iter()
        .map(|s| s.to_string())
        .collect();

    return Ok(lines);
}