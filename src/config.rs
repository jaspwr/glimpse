use std::{
    error::Error,
    fs,
    io::{BufRead, BufReader},
    path::PathBuf,
};

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

pub static CONF: Lazy<Config> = Lazy::new(|| match load_config() {
    Ok(mut config) => {
        config.error = None;
        config
    }
    Err(err) => {
        let mut ret = Config::default();
        ret.error = Some(err.to_string());
        ret
    }
});

pub static CONF_FILE_PATH: Lazy<PathBuf> = Lazy::new(|| {
    let path = home::home_dir()
        .unwrap()
        .join(".config")
        .join("glimpse")
        .join("config.toml");

    if !path.exists() {
        std::fs::create_dir_all(&path).unwrap();
    }

    path
});

#[rustfmt::skip]
static DEFAULT_CSS: &str = ".search-field {}

.outer-container {}

.results-list {}

.result-box {}

.result-title {}

.result-subtext {}

.result-icon {}

.odd-row {}

.even-row {}

scrollbar {}

scrollbar slider {}

.preview-window {}

.preview-text {}
";

pub static CSS: Lazy<String> = Lazy::new(|| {
    let css_path = CONF_FILE_PATH.parent().unwrap().join("style.css");

    if !css_path.exists() {
        let _ = fs::write(&css_path, DEFAULT_CSS);
        DEFAULT_CSS.to_string()
    } else {
        fs::read_to_string(&css_path).unwrap()
    }
});

#[derive(Serialize, Deserialize)]
pub struct Config {
    #[serde(skip)]
    pub error: Option<String>,

    pub max_results: usize,
    pub indexing: Indexing,
    pub modules: Modules,
    pub search_paths: Vec<PathBuf>,
    pub search_hidden_folders: bool,
    pub ignore_directories: Vec<String>,
    pub search_file_contents: bool,
    pub visual: Visual,
    pub window: Window,
    pub preview_window: PreviewWindow,
    pub misc: Misc,
}

#[derive(Serialize, Deserialize)]
pub struct Modules {
    pub commands: bool,
    pub files: bool,
    pub steam_games: bool,
    pub web_bookmarks: bool,
    pub calculator: bool,
    pub dictionary: bool,
}

#[derive(Serialize, Deserialize)]
pub struct Misc {
    pub display_command_paths: bool,
    pub display_file_and_directory_paths: bool,
    pub preferred_terminal: String,
    pub run_exes_with_wine: bool,
}

#[derive(Serialize, Deserialize)]
pub struct Window {
    pub width: u32,
    pub height: u32,
}

#[derive(Serialize, Deserialize)]
pub struct Visual {
    pub show_icons: bool,
    pub icon_size: u32,
    pub result_borders: bool,
    pub dark_result_borders: bool,
}

#[derive(Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct Indexing {
    pub location: String,
    pub size_upper_bound_GiB: f32,
}

#[derive(Serialize, Deserialize)]
pub struct PreviewWindow {
    pub enabled: bool,
    pub show_automatically: bool,
    pub width: u32,
    pub image_size: u32,
}

pub fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    if let Some(home) = home::home_dir() {
        let mut config_path = home.join(".config").join("glimpse").join("config.toml");

        if home.file_name().is_none() || home.file_name().unwrap() == "root" {
            config_path = find_user_config()?;
        }

        println!("Config path: {:?}", config_path);

        if let Ok(file) = std::fs::File::open(config_path.clone()) {
            let file = BufReader::new(file);
            let file = file
                .lines()
                .collect::<Result<Vec<String>, _>>()
                .unwrap()
                .join("\n");

            let conf: Config = toml::from_str(&file)?;

            if conf.indexing.size_upper_bound_GiB < 0.0 {
                return Err("Can't have negative size for upper bound of indexing."
                    .to_string()
                    .into());
            }

            for path in &conf.search_paths {
                if !PathBuf::from(path.clone()).exists() {
                    return Err("Indexing location does not exist.".to_string().into());
                }
            }

            Ok(conf)
        } else {
            create_new_config_file(home, config_path)
        }
    } else {
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Error loading config.",
        )))
    }
}

fn find_user_config() -> Result<PathBuf, Box<dyn Error>> {
    let mut config_path = PathBuf::from("/home");

    for entry in fs::read_dir("/home")? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let path = path.file_name().unwrap().to_str().unwrap();

            if path == "root" {
                continue;
            }

            config_path = PathBuf::from("/home")
                .join(path)
                .join(".config")
                .join("glimpse")
                .join("config.toml");

            if config_path.exists() {
                return Ok(config_path);
            }
        }
    }

    Err(Box::new(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "Error loading config.",
    )))
}

fn create_new_config_file(home: PathBuf, config_path: PathBuf) -> Result<Config, Box<dyn Error>> {
    let mut default_config = Config::default();
    let indexing_location = home.join(".cache").join("glimpse");
    let indexing_location = indexing_location.to_str();
    default_config.indexing.location = match indexing_location {
        Some(location) => String::from(location),
        None => {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Error loading config.",
            )))
        }
    };
    let toml = toml::to_string(&default_config)?;
    let config_folder = home.join(".config").join("glimpse");
    std::fs::create_dir_all(config_folder)?;
    std::fs::write(config_path, toml)?;
    Ok(default_config)
}

impl Default for Config {
    fn default() -> Self {
        let ignore_dirs = vec![
            "node_modules".to_string(),
            "zig-cache".to_string(),
            "zig-out".to_string(),
            "target".to_string(),
            "_prefix32_wine".to_string(),
            "texmf".to_string(),
            "VirtualBox VMs".to_string(),
            "x86_64-pc-linux-gnu-library".to_string(),
            "x86_64-unknown-linux-gnu-library".to_string(),
        ];

        Config {
            error: None,
            max_results: 25,
            indexing: Indexing {
                location: String::from(""),
                size_upper_bound_GiB: 0.5,
            },
            modules: Modules {
                commands: true,
                files: true,
                steam_games: true,
                web_bookmarks: true,
                calculator: true,
                dictionary: false,
            },
            search_paths: vec![home::home_dir().unwrap_or(PathBuf::from("/home"))],
            search_hidden_folders: false,
            search_file_contents: true,
            ignore_directories: ignore_dirs,
            visual: Visual {
                show_icons: true,
                icon_size: 32,
                result_borders: false,
                dark_result_borders: false,
            },
            window: Window {
                width: 540,
                height: 410,
            },
            preview_window: PreviewWindow {
                enabled: true,
                show_automatically: true,
                width: 420,
                image_size: 350,
            },
            misc: Misc {
                display_command_paths: false,
                display_file_and_directory_paths: true,
                preferred_terminal: String::from("xterm"),
                run_exes_with_wine: true,
            },
        }
    }
}
