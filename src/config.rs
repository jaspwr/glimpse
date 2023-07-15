use std::io::{BufRead, BufReader};

use pango::glib::once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

pub static CONF: Lazy<Config> = Lazy::new(|| match load_config() {
    Ok(config) => config,
    Err(_) => {
        println!("Failed to load config. Using default.");
        Config::default()
    }
});

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub max_results: usize,
    pub indexing: Indexing,
    pub modules: Modules,
    pub use_web_modules: bool,
    pub visual: Visual,
    pub misc: Misc,
}

#[derive(Serialize, Deserialize)]
pub struct Modules {
    pub commands: bool,
    pub file_names: bool,
    pub file_contents: bool,
    pub pdf_contents: bool,
    pub steam_games: bool,
    pub web_bookmarks: bool,
    pub calculator: bool,
    pub online_modules: WebModules,
}

#[derive(Serialize, Deserialize)]
pub struct Misc {
    pub display_command_paths: bool,
    pub display_file_and_directory_paths: bool,
    pub preferred_terminal: String,
}

#[derive(Serialize, Deserialize)]
pub struct WebModules {
    pub web_search: bool,
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

pub fn load_config() -> Result<Config, Box::<dyn std::error::Error>> {
    if let Some(home) = home::home_dir() {
        let config_path = home.join(".config").join("prober").join("config.toml");
        if let Ok(file) = std::fs::File::open(config_path.clone()) {
            let file = BufReader::new(file);
            let file = file
                .lines()
                .collect::<Result<Vec<String>, _>>()
                .unwrap()
                .join("\n");
            return Ok(toml::from_str(&file)?);
        } else {
            let mut default_config = Config::default();

            let indexing_location = home
                .join(".cache")
                .join("prober");
            let indexing_location = indexing_location.to_str();
            default_config.indexing.location = match indexing_location {
                Some(location) => String::from(location),
                None => return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Error loading config.",
                )))
            };

            let toml = toml::to_string(&default_config)?;
            let config_folder = home.join(".config").join("prober");
            std::fs::create_dir_all(config_folder)?;
            std::fs::write(config_path, toml)?;
            return Ok(default_config);
        }
    }
    return Err(Box::new(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "Error loading config.",
    )));
}

impl Default for Config {
    fn default() -> Self {
        Config {
            max_results: 25,
            indexing: Indexing {
                location: String::from(""),
                size_upper_bound_GiB: 0.5,
            },
            modules: Modules {
                commands: true,
                file_names: true,
                file_contents: true,
                pdf_contents: true,
                steam_games: true,
                web_bookmarks: true,
                calculator: true,
                online_modules: WebModules {
                    web_search: true,
                    dictionary: true,
                },
            },
            use_web_modules: false,
            visual: Visual {
                show_icons: true,
                icon_size: 32,
                result_borders: true,
            },
            misc: Misc {
                display_command_paths: false,
                display_file_and_directory_paths: true,
                preferred_terminal: String::from("xterm"),
            },
        }
    }
}
