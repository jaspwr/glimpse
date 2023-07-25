use std::{
    process::{Command, Stdio},
    sync::Arc,
};

use async_trait::async_trait;
use execute::Execute;
use prober::config::CONF;

use crate::{
    exec::execute_detached,
    icon,
    result_templates::standard_entry,
    search::string_search,
    utils::{self, is_cli_app},
    BoxedRuntime,
};

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

            let icon = icon::find_application_icon(&name);

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
