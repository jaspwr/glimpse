use std::{process::{Command, Stdio}, fs};


use async_trait::async_trait;
use execute::Execute;

use crate::{result_templates::standard_entry, search::{string_search, Biases}, utils};

use super::{SearchModule, SearchResult};

pub struct Commands {
    apps: Vec<String>,
    biases: Biases
}

impl Commands {
    pub fn new() -> Commands {
        Commands {
            apps: get_list().unwrap(),
            biases: Biases::load("commands")
        }
    }

    fn create_result(&self, name: String, relevance: f32) -> SearchResult {
        let name_cpy = name.clone();
        let render = move || {
            let name = name_cpy.clone();
            let icon = find_icon(&name);
            standard_entry(name, icon, None)
        };

        let id = utils::simple_hash(&name) + 0xabcdef;

        let biases_cpy = self.biases.clone();
        let run = move || {
            let mut biases = biases_cpy.clone();
            let mut bias = 0.0;
            if let Some(b) = biases.map.get(&name) {
                bias = b.clone();
            }

            bias += 0.15;

            if bias > 0.6 {
                bias = 0.6;
            }

            biases.map.insert(name.clone(), bias);
            biases.save("commands");
            execute_detached(name.clone());
        };

        SearchResult {
            render: Box::new(render),
            relevance,
            id,
            on_select: Some(Box::new(run)),
        }
    }
}

#[async_trait]
impl SearchModule for Commands {
    async fn search(&self, query: String, max_results: u32) -> Vec<SearchResult> {
        string_search(&query, &self.apps, max_results, Some(&self.biases))
            .into_iter()
            .map(|(s, r)| self.create_result(s, r))
            .collect()
    }
}


fn execute_detached(name: String) {
    let mut command = Command::new("bash");
    command.arg("-c");
    command.arg(format!("{} & disown", name));
    let _ = command.execute();
}

fn find_icon(name: &String) -> Option<gtk::Image> {
    let mut possible_locations = vec![
        "/usr/share/pixmaps".to_string(),
        "/usr/share/icons/hicolor/32x32/apps/".to_string(),
        "/usr/share/icons/hicolor/symbolic/apps".to_string(),
        "/usr/share/icons/hicolor/scalable/apps".to_string()
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

    const POSSIBLE_EXTENSIONS: [&str; 2] = [
        ".png",
        ".svg",
    ];

    for path in possible_locations.iter() {
        let mut path = path.clone();
        path.push_str("/");
        path.push_str(name);
        for extension in POSSIBLE_EXTENSIONS.iter() {
            let mut path = path.clone();
            path.push_str(extension);

            let file = std::fs::File::open(path.clone());
            if file.is_ok() {
                let pixbuf = gtk::gdk::gdk_pixbuf::Pixbuf::from_file(path).unwrap();
                let pixbuf = pixbuf.scale_simple(32, 32, gtk::gdk_pixbuf::InterpType::Bilinear).unwrap();
                return Some(gtk::Image::from_pixbuf(Some(&pixbuf)));
            }
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