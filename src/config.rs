use std::io::{BufRead, BufReader};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub indexing: Indexing,
    pub modules: Modules,
    pub use_web_modules: bool,
    pub visual: Visual,
}

#[derive(Serialize, Deserialize)]
pub struct Modules {
    pub commands: bool,
    pub file_names: bool,
    pub file_contents: bool,
    pub pdf_contents: bool,
    pub steam_games: bool,
    pub web_modules: WebModules,
}

#[derive(Serialize, Deserialize)]
pub struct WebModules {
    pub web_search: bool,
    pub web_bookmarks: bool,
    pub web_history: bool,
    pub dictionary: bool,
}

#[derive(Serialize, Deserialize)]
pub struct Visual {
    pub show_icons: bool,
    pub icon_size: u32,
    pub result_borders: bool,
}

#[derive(Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct Indexing {
    pub location: String,
    pub size_upper_bound_GiB: f32,
}

pub fn load_config() -> Result<Config, ()> {
    if let Some(home) = home::home_dir() {
        let config_path = home.join(".config").join("prober").join("config.toml");
        if let Ok(file) = std::fs::File::open(config_path.clone()) {
            let file = BufReader::new(file);
            let file = file.lines().collect::<Result<Vec<String>, _>>().unwrap().join("\n");
            return Ok(toml::from_str(&file).unwrap());
        } else {
            let default_config = Config::default();
            let toml = toml::to_string(&default_config).unwrap();
            let config_folder = home.join(".config").join("prober");
            std::fs::create_dir_all(config_folder).unwrap();
            std::fs::write(config_path, toml).unwrap();
            return Ok(default_config);
        }
    }
    return Err(());
}

impl Default for Config {
    fn default() -> Self {
        Config {
            indexing: Indexing {
                location: String::from("/home"),
                size_upper_bound_GiB: 0.5,
            },
            modules: Modules {
                commands: true,
                file_names: true,
                file_contents: true,
                pdf_contents: true,
                steam_games: true,
                web_modules: WebModules {
                    web_search: true,
                    web_bookmarks: true,
                    web_history: true,
                    dictionary: true,
                }
            },
            use_web_modules: false,
            visual: Visual {
                show_icons: true,
                icon_size: 32,
                result_borders: true,
            },
        }
    }
}