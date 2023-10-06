use std::{collections::HashMap, fs, sync::Arc};

use async_trait::async_trait;

use crate::{
    exec::execute_detached, icon, result_templates::standard_entry, search::string_search,
    utils::{simple_hash, simple_hash_nonce, HashFn}, BoxedRuntime,
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

pub fn steam_folder_exists() -> bool {
    let home = home::home_dir().unwrap();
    let steamapps = home.join(".steam");
    steamapps.exists()
}

#[async_trait]
impl SearchModule for SteamGames {
    async fn search(&self, query: String, max_results: u32) -> Vec<SearchResult> {
        let query = query.to_lowercase();
        let data = self.data.lock().await;

        let hash_fn = simple_hash_nonce(std::any::type_name::<Self>());

        if data.is_none() {
            return vec![];
        }

        string_search(
            &query,
            &data.as_ref().unwrap().game_names,
            max_results,
            &hash_fn,
            false,
        )
        .into_iter()
        .map(|(n, r)| create_result(data.as_ref().unwrap(), &n, r, hash_fn(&*n)))
        .collect::<Vec<SearchResult>>()
    }
}


fn create_result(data: &GamesData, name: &String, relevance: f32, id: u64) -> SearchResult {
    let steam_id = *data.game_ids.get(name).unwrap();
    let cased_name = data.cased_game_names.get(name).unwrap().clone();
    let render = move || {
        let icon = find_icon(steam_id);
        standard_entry(cased_name.clone(), icon, None)
    };

    let on_select = move || {
        let cmd = format!("steam steam://rungameid/{} & disown", id.clone());
        let _ = execute_detached(cmd);
    };

    SearchResult {
        render: Box::new(render),
        relevance,
        id,
        on_select: Some(Box::new(on_select)),
        preview_window_data: crate::preview_window::PreviewWindowShowing::None,
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
            dir.flatten().for_each(|entry| {
                handle_dir_entry(entry, &mut games);
            });
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
            .zip(cased_game_names)
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

fn handle_dir_entry(entry: fs::DirEntry, games: &mut Vec<Game>) {
    if entry.file_type().unwrap().is_dir() {
        return;
    }
    let name = entry.file_name().into_string().unwrap();
    if name.len() < 4 || &name[name.len() - 4..] != ".acf" {
        return;
    }
    let mut appid = 0;
    let mut name = String::from("");
    let file = fs::read_to_string(entry.path()).unwrap();
    file.lines().into_iter().for_each(|l| {
        if l.starts_with("\t\"appid\"") {
            appid = l.split('\"').nth(3).unwrap().parse::<u32>().unwrap();
        } else if l.starts_with("\t\"name\"") {
            name = l.split('\"').nth(3).unwrap().to_string();
        }
    });
    games.push(Game { name, appid });
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
