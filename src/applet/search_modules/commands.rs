use std::{
    fs,
    process::{Command, Stdio},
    sync::Arc,
};

use async_trait::async_trait;
use execute::Execute;
use prober::config::CONF;

use crate::{icon, result_templates::standard_entry, search::string_search, utils, BoxedRuntime, exec::execute_detached};

use super::{SearchModule, SearchResult};

pub struct Commands {
    apps: Arc<tokio::sync::Mutex<Option<Vec<String>>>>,
}

unsafe impl Send for Commands {}
unsafe impl Sync for Commands {}

fn id_hash(name: &String) -> u64 {
    utils::simple_hash(name) + 0xabcdef
}

impl Commands {
    pub fn new(rt: BoxedRuntime) -> Commands {
        let apps_store = Arc::new(tokio::sync::Mutex::new(None));

        let apps_store_cpy = apps_store.clone();
        rt.lock().unwrap().spawn(async move {
            let mut store = apps_store_cpy.lock().await;
            // This lock needs to be held until the initialisation is done
            let app = get_list().unwrap();
            *store = Some(app);
        });

        Commands { apps: apps_store }
    }

    fn create_result(&self, name: String, relevance: f32) -> SearchResult {
        let name_cpy = name.clone();
        let render = move || {
            let name = name_cpy.clone();

            let mut icon = find_icon(&name);

            // TODO: Store pixbuf somewhere
            if icon.is_none() {
                let icon_str = if is_cli_app(&name) {
                    "utilities-terminal"
                } else {
                    "application-x-executable"
                };
                icon = icon::from_gtk(icon_str);
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
                let _ = execute_detached(name.clone());
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
    fn is_ready(&self) -> bool {
        true
        // self.apps.lock().await.is_some()
    }

    async fn search(&self, query: String, max_results: u32) -> Vec<SearchResult> {
        let rc = self.apps.clone();
        let lock = rc.lock().await;
        if let Some(apps) = lock.as_ref() {
            string_search(&query, apps, max_results, Box::new(id_hash), true)
                .into_iter()
                .map(|(s, r)| self.create_result(s, r + 0.3))
                .collect()
        } else {
            vec![]
        }
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

            let i = icon::from_file(&path);
            if i.is_some() {
                return i;
            }
        }
    }
    return None;
}

fn is_cli_app(name: &String) -> bool {
    matches!(
        name.as_str(),
        "ls" | "cd"
            | "cat"
            | "rm"
            | "mv"
            | "cp"
            | "mkdir"
            | "rmdir"
            | "touch"
            | "ed"
            | "if"
            | "then"
            | "else"
            | "fi"
            | "for"
            | "do"
            | "done"
            | "while"
            | "until"
            | "case"
            | "esac"
            | "vim"
            | "nano"
            | "ghc"
            | "ghci"
            | "ghcup"
            | "cabal"
            | "rustc"
            | "cargo"
            | "clang"
            | "clang++"
            | "gcc"
            | "g++"
            | "make"
            | "node"
            | "npm"
            | "yarn"
            | "pnpm"
            | "npx"
            | "python"
            | "python3"
            | "pip"
            | "pip3"
            | "ruby"
            | "gem"
            | "java"
            | "javac"
            | "jshell"
            | "javadoc"
            | "jlink"
            | "jpackage"
            | "jdeps"
            | "jmod"
            | "jdb"
            | "jconsole"
            | "git"
            | "gitk"
            | "pacman"
            | "yay"
            | "paru"
            | "apt"
            | "apt-get"
            | "tar"
            | "unzip"
            | "zip"
            | "unrar"
            | "rar"
            | "7z"
            | "zstd"
            | "gzip"
            | "gunzip"
            | "atool"
            | "neofetch"
            | "julia"
            | "nvim"
            | "emacs"
            | "htop"
            | "top"
            | "btop"
            | "nmtui"
            | "nmcli"
            | "ip"
            | "ipconfig"
            | "ifconfig"
            | "gdb"
            | "ld"
            | "alias"
            | "kill"
            | "pkill"
            | "find"
            | "tree"
            | "sudo"
            | "su"
            | "chown"
            | "chmod"
            | "grep"
            | "sed"
    )
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
