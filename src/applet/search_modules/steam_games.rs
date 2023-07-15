use std::{collections::HashMap, fs, ops::ControlFlow, sync::Arc};

use async_trait::async_trait;
use execute::Execute;

use crate::{
    icon, result_templates::standard_entry, search::string_search, utils::simple_hash, BoxedRuntime, exec::execute_detached,
};

use super::{SearchModule, SearchResult};

pub struct SteamGames {
    data: Arc<tokio::sync::Mutex<Option<GamesData>>>,
}

struct Game {
    name: String,
    appid: u32,
}

struct GamesData {
    game_names: Vec<String>,
    game_ids: HashMap<String, u32>,
    cased_game_names: HashMap<String, String>,
}

#[async_trait]
impl SearchModule for SteamGames {
    fn is_ready(&self) -> bool {
        // let lock = self.data.lock().unwrap();
        // lock.is_some()
        true
    }

    async fn search(&self, query: String, max_results: u32) -> Vec<SearchResult> {
        let query = query.to_lowercase();
        let data = self.data.lock().await;

        if data.is_none() {
            return vec![];
        }

        string_search(
            &query,
            &data.as_ref().unwrap().game_names,
            max_results,
            Box::new(id_hash),
            false,
        )
        .into_iter()
        .map(|(s, r)| create_result(data.as_ref().unwrap(), s, r))
        .collect::<Vec<SearchResult>>()
    }
}

fn id_hash(name: &String) -> u64 {
    simple_hash(name) + 0x0e0e00e0e00e
}

fn create_result(data: &GamesData, name: String, relevance: f32) -> SearchResult {
    let id = *data.game_ids.get(&name).unwrap();
    let cased_name = data.cased_game_names.get(&name).unwrap().clone();
    let render = move || {
        // TODO: icon for not found.
        let icon = find_icon(id);
        standard_entry(cased_name.clone(), icon, None)
    };

    let on_select = move || {
        let cmd = format!("steam steam://rungameid/{} & disown", id.clone());
        let _ = execute_detached(cmd);
    };

    SearchResult {
        render: Box::new(render),
        relevance,
        id: id_hash(&name),
        on_select: Some(Box::new(on_select)),
    }
}

impl SteamGames {
    pub fn new(rt: BoxedRuntime) -> SteamGames {
        let data = Arc::new(tokio::sync::Mutex::new(None));

        let data_cpy = data.clone();
        rt.lock().unwrap().spawn(async move {
            let store = data_cpy.clone();
            let mut lock = store.lock().await;
            // This lock needs to be held until the initialisation is done

            let data = Some(GamesData::new());
            *lock = data;
        });

        SteamGames { data }
    }
}

impl GamesData {
    fn new() -> Self {
        let home = home::home_dir().unwrap();
        let steamapps = home.join(".steam").join("steam").join("steamapps");

        let mut games = vec![];
        if let Ok(dir) = fs::read_dir(steamapps) {
            for entry in dir {
                if let Ok(entry) = entry {
                    if let ControlFlow::Break(_) = handle_dir_entry(entry, &mut games) {
                        continue;
                    }
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

        let cased_game_names = game_names
            .clone()
            .into_iter()
            .zip(cased_game_names.clone())
            .collect::<HashMap<String, String>>();

        let game_ids = games
            .into_iter()
            .map(|g| (g.name.to_lowercase(), g.appid))
            .collect::<HashMap<String, u32>>();

        GamesData {
            game_names,
            game_ids,
            cased_game_names,
        }
    }
}

fn handle_dir_entry(entry: fs::DirEntry, games: &mut Vec<Game>) -> ControlFlow<()> {
    if entry.file_type().unwrap().is_dir() {
        return ControlFlow::Break(());
    }
    let name = entry.file_name().into_string().unwrap();
    if name.len() < 4 || &name[name.len() - 4..] != ".acf" {
        return ControlFlow::Break(());
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

    ControlFlow::Continue(())
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

    let path = &path.to_str().unwrap().to_string();
    match icon::from_file(path) {
        Some(icon) => Some(icon),
        None => icon::from_gtk("application-x-executable"),
    }
}
