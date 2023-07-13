use std::{
    fs,
    process::{Command, Stdio},
};

use async_trait::async_trait;
use execute::Execute;
use gtk::traits::IconThemeExt;
use prober::config::CONF;

use crate::{
    biases::{Biases, BIASES},
    result_templates::standard_entry,
    search::string_search,
    utils, icon,
};

use super::{SearchModule, SearchResult};

pub struct Commands {
    apps: Vec<String>,
}

unsafe impl Send for Commands {}
unsafe impl Sync for Commands {}

fn id_hash(name: &String) -> u64 {
    utils::simple_hash(name) + 0xabcdef
}

impl Commands {
    pub fn new() -> Commands {
        Commands {
            apps: get_list().unwrap(),
        }
    }

    fn create_result(&self, name: String, relevance: f32) -> SearchResult {
        let name_cpy = name.clone();
        let render = move || {
            let name = name_cpy.clone();

            let mut icon = find_icon(&name);

            // TODO: Store pixbuf somewhere
            if icon.is_none() {
                let theme = gtk::IconTheme::default().unwrap();
                let mut icon_str = "application-x-executable";
                if is_cli_app(&name) {
                    icon_str = "utilities-terminal";
                }
                let icon_info = theme.lookup_icon(icon_str, 32, gtk::IconLookupFlags::FORCE_SIZE);
                if let Some(icon_info) = icon_info {
                    let pixbuf = gtk::gdk::gdk_pixbuf::Pixbuf::from_file_at_size(
                        icon_info.filename().unwrap().to_str().unwrap(),
                        CONF.visual.icon_size as i32,
                        CONF.visual.icon_size as i32,
                    );
                    if let Ok(pixbuf) = pixbuf {
                        icon = Some(gtk::Image::from_pixbuf(Some(&pixbuf)));
                    }
                }
            }

            if !CONF.visual.show_icons {
                icon = None;
            }

            let desc = if CONF.misc.display_command_paths {
                Some(which(&name))
            } else {
                None
            };
            standard_entry(name, icon, desc)
        };

        let id = id_hash(&name);

        let run = move || {
            if is_cli_app(&name) {
                spawn_in_terminal(&name.clone());
            } else {
                execute_detached(name.clone());
            }
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
        string_search(&query, &self.apps, max_results, Box::new(id_hash), true)
            .into_iter()
            .map(|(s, r)| self.create_result(s, r))
            .collect()
    }
}

fn which(name: &String) -> String {
    let mut command = Command::new("bash");
    command.arg("-c");
    command.arg(format!("which {}", name));

    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let output = command.execute_output().unwrap();

    let output = String::from_utf8(output.stdout).unwrap();
    let first_line = match output.lines().next() {
        Some(line) => line,
        None => "",
    };

    return first_line.to_string();
}

fn spawn_in_terminal(name: &String) {
    let mut command = Command::new("bash");
    command.arg("-c");
    command.arg(format!(
        "{} -e \"{}\" & disown",
        CONF.misc.preferred_terminal.as_str(),
        name
    ));
    let proc = command.execute();
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
        "/usr/share/icons/hicolor/scalable/apps".to_string(),
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

    const POSSIBLE_EXTENSIONS: [&str; 2] = [".png", ".svg"];

    for path in possible_locations.iter() {
        let mut path = path.clone();
        path.push_str("/");
        path.push_str(name);
        for extension in POSSIBLE_EXTENSIONS.iter() {
            let mut path = path.clone();
            path.push_str(extension);


            let i =  icon::from_file(&path);
            if i.is_some() {
                return i;
            }
        }
    }
    return None;
}

fn is_cli_app(name: &String) -> bool {
    match name.as_str() {
        "ls" => true,
        "cd" => true,
        "cat" => true,
        "rm" => true,
        "mv" => true,
        "cp" => true,
        "mkdir" => true,
        "rmdir" => true,
        "touch" => true,
        "ed" => true,
        "if" => true,
        "then" => true,
        "else" => true,
        "fi" => true,
        "for" => true,
        "do" => true,
        "done" => true,
        "while" => true,
        "until" => true,
        "case" => true,
        "esac" => true,
        "vim" => true,
        "nano" => true,
        "ghc" => true,
        "ghci" => true,
        "ghcup" => true,
        "cabal" => true,
        "rustc" => true,
        "cargo" => true,
        "clang" => true,
        "clang++" => true,
        "gcc" => true,
        "g++" => true,
        "make" => true,
        "node" => true,
        "npm" => true,
        "yarn" => true,
        "pnpm" => true,
        "npx" => true,
        "python" => true,
        "python3" => true,
        "pip" => true,
        "pip3" => true,
        "ruby" => true,
        "gem" => true,
        "java" => true,
        "javac" => true,
        "jshell" => true,
        "javadoc" => true,
        "jlink" => true,
        "jpackage" => true,
        "jdeps" => true,
        "jmod" => true,
        "jdb" => true,
        "jconsole" => true,
        "git" => true,
        "gitk" => true,
        "pacman" => true,
        "yay" => true,
        "paru" => true,
        "apt" => true,
        "apt-get" => true,
        "tar" => true,
        "unzip" => true,
        "zip" => true,
        "unrar" => true,
        "rar" => true,
        "7z" => true,
        "zstd" => true,
        "gzip" => true,
        "gunzip" => true,
        "atool" => true,
        "neofetch" => true,
        "julia" => true,
        "nvim" => true,
        "emacs" => true,
        "htop" => true,
        "top" => true,
        "btop" => true,
        "nmtui" => true,
        "nmcli" => true,
        "ip" => true,
        "ipconfig" => true,
        "ifconfig" => true,
        "gdb" => true,
        "ld" => true,
        "alias" => true,
        "kill" => true,
        "pkill" => true,
        "find" => true,
        "tree" => true,
        "sudo" => true,
        "su" => true,
        "chown" => true,
        "chmod" => true,
        "grep" => true,
        "sed" => true,
        _ => false,
    }
}

fn get_list() -> Result<Vec<String>, ()> {
    let mut command = Command::new("bash");
    command.arg("-c");
    command.arg("compgen -ac");

    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let output = command.execute_output().unwrap();

    let output = String::from_utf8(output.stdout).unwrap();
    let lines = output.lines().into_iter().map(|s| s.to_string()).collect();

    return Ok(lines);
}
