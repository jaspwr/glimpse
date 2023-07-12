use std::{collections::HashMap, fs};

use async_trait::async_trait;
use execute::Execute;
use gtk::traits::ContainerExt;

use crate::{search::string_search, CONF, result_templates::standard_entry};

use super::{SearchModule, SearchResult};

#[derive(Debug)]
pub struct SteamGames {
    game_names: Vec<String>,
    game_ids: HashMap<String, u32>,
    cased_game_names: HashMap<String, String>,
}

struct Game {
    name: String,
    appid: u32,
}

#[async_trait]
impl SearchModule for SteamGames {
    async fn search(&self, query: String, max_results: u32) -> Vec<SearchResult> {
        let query = query.to_lowercase();
        string_search(&query, &self.game_names, max_results, None)
            .into_iter()
            .map(|(s, r)| self.create_result(s, r))
            .collect::<Vec<SearchResult>>()
    }
}

impl SteamGames {
    fn create_result(&self, name: String, relevance: f32) -> SearchResult {
        let id = *self.game_ids.get(&name).unwrap();
        let name = self.cased_game_names.get(&name).unwrap().clone();
        let render = move || {
            // TODO: icon for not found.
            let icon = find_icon(id);
            standard_entry(name.clone(), icon, None)
        };

        let on_select = move || {
            let mut command = std::process::Command::new("bash");
            command.arg("-c");
            command.arg(format!("steam steam://rungameid/{} & disown", id.clone()));
            let _ = command.execute();
        };

        SearchResult {
            render: Box::new(render),
            relevance,
            id: id as u64,
            on_select: Some(Box::new(on_select)),
        }
    }
}

impl SteamGames {
    pub fn new() -> SteamGames {
        let home = home::home_dir().unwrap();
        let steamapps = home.join(".steam").join("steam").join("steamapps");

        let mut games = vec![];
        if let Ok(dir) = fs::read_dir(steamapps) {
            for entry in dir {
                if let Ok(entry) = entry {
                    if entry.file_type().unwrap().is_dir() {
                        continue;
                    }
                    let name = entry.file_name().into_string().unwrap();

                    if name.len() < 4 || &name[name.len() - 4..] != ".acf" {
                        continue;
                    }

                    let mut appid = 0;
                    let mut name = String::from("");
                    let file = fs::read_to_string(entry.path()).unwrap();
                    file.lines().into_iter().for_each(|l| {
                        if l.starts_with("\t\"appid\"") {
                            appid = l.split("\"").nth(3).unwrap().parse::<u32>().unwrap();
                        } else if l.starts_with("\t\"name\"") {
                            name = l.split("\"").nth(3).unwrap().to_string();
                        }
                    });

                    games.push(Game { name, appid });
                }
            }
        }

        let cased_game_names = games
            .iter()
            .map(|g| g.name.clone())
            .collect::<Vec<String>>();

        let game_names = games
            .iter()
            .map(|g| g.name.to_lowercase())
            .collect::<Vec<String>>();

        let cased_game_names = game_names.clone()
            .into_iter()
            .zip(cased_game_names.clone())
            .collect::<HashMap<String, String>>();

        let game_ids = games
            .into_iter()
            .map(|g| (g.name.to_lowercase(), g.appid))
            .collect::<HashMap<String, u32>>();

        SteamGames {
            game_names,
            game_ids,
            cased_game_names,
        }
    }
}

fn find_icon(appid: u32) -> Option<gtk::Image> {
    let home = home::home_dir().unwrap();
    let path = home
        .join(".local")
        .join("share")
        .join("icons")
        .join("hicolor")
        .join("32x32")
        .join("apps")
        .join(format!("steam_icon_{}.png", appid));

    let file = std::fs::File::open(&path);
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
