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
    pub full_reindex_after_days: f32,
}

#[derive(Serialize, Deserialize)]
pub struct PreviewWindow {
    pub enabled: bool,
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
                if !path.clone().exists() {
                    return Err("Indexing location does not exist.".to_string().into());
                }
            }

            if !PathBuf::from(&conf.indexing.location).exists() {
                std::fs::create_dir_all(&conf.indexing.location).unwrap();
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
    for entry in fs::read_dir("/home")? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let path = path.file_name().unwrap().to_str().unwrap();

            if path == "root" {
                continue;
            }

            let config_path = PathBuf::from("/home")
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
    let toml = add_comment_to("search_paths", "Directory to search for files from", toml);
    let toml = add_comment_to("location", "Where to store the file database.", toml);
    let toml = add_comment_to("size_upper_bound_GiB", "The maximum size of the file index database in GiB. If this is exceeded, new data will not be added until there has been a full reindex.", toml);
    let toml = add_comment_to("search_file_contents", "Index and search files by keywords they contain. Works for pdf, docx, txt and a few other plaintext filetypes. Will take considerably longer to index. It is recommended that full reindexes are done infrequently with this option.", toml);
    let toml = add_comment_to(
        "run_exes_with_wine",
        "Open files with a .exe extension with wine.",
        toml,
    );
    let toml = add_comment_to(
        "ignore_directories",
        "Directories to ignore when indexing and searching files.",
        toml,
    );
    let toml = add_comment_to(
        "full_reindex_after_days",
        "Recrawl and reindex the file system after this many days",
        toml,
    );

    let config_folder = home.join(".config").join("glimpse");
    std::fs::create_dir_all(config_folder)?;
    std::fs::write(config_path, toml)?;
    Ok(default_config)
}

#[inline]
fn add_comment_to(item: &str, comment: &str, toml: String) -> String {
    let mut output = String::new();

    for line in toml.lines() {
        if line.starts_with(item) {
            let comment = split_comment_over_lines(comment)
                .into_iter()
                .map(|line| format!("# {}\n", line))
                .collect::<String>();

            output.push_str(&format!("\n{}", &comment));
        }

        output.push_str(&format!("{}\n", line));
    }

    output
}

fn split_comment_over_lines(comment: &str) -> Vec<String> {
    let mut lines = vec![vec![]];

    let mut line_length = 0;

    const ROW_SIZE: usize = 60;

    for word in comment.split_whitespace() {
        if line_length + word.len() > ROW_SIZE {
            lines.push(vec![]);
            line_length = 0;
        }

        lines.last_mut().unwrap().push(word.to_string());
        line_length += word.len();
    }

    lines.into_iter().map(|line| line.join(" ")).collect()
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
                size_upper_bound_GiB: 5.0,
                full_reindex_after_days: 0.6,
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
            search_file_contents: false,
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
