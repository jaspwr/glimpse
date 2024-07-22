// Glimpse - GNU/Linux Launcher and File search utility.
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
    process::{Command, Stdio},
    sync::Arc,
};

use async_trait::async_trait;
use execute::Execute;
use glimpse::config::CONF;

use crate::{
    app::BoxedRuntime,
    exec::execute_detached,
    icon,
    result_templates::standard_entry,
    search::string_search,
    utils::{benchmark, is_cli_app, simple_hash_nonce},
};

use super::{SearchModule, SearchResult};

pub struct Commands {
    apps: Arc<tokio::sync::Mutex<Option<Vec<String>>>>,
}

unsafe impl Send for Commands {}
unsafe impl Sync for Commands {}

impl Commands {
    pub fn new(rt: BoxedRuntime) -> Commands {
        let benchmark = benchmark();

        let apps_store = Arc::new(tokio::sync::Mutex::new(None));

        let apps_store_cpy = apps_store.clone();
        rt.lock().unwrap().spawn(async move {
            let mut store = apps_store_cpy.lock().await;
            // This lock needs to be held until the initialisation is done
            let app = get_list().unwrap();
            *store = Some(app);
        });

        if let Some(benchmark) = benchmark {
            println!(
                "Commands module loaded in {:?}",
                benchmark.elapsed().unwrap()
            );
        }

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

        let hash = simple_hash_nonce(&self.name());

        let id = hash(&name);

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
            preview_window_data: crate::preview_window::PreviewWindowShowing::None,
        }
    }
}

#[async_trait]
impl SearchModule for Commands {
    async fn search(&self, query: String, max_results: u32) -> Vec<SearchResult> {
        let rc = self.apps.clone();
        let lock = rc.lock().await;
        let hash_fn = simple_hash_nonce(&self.name());
        if let Some(apps) = lock.as_ref() {
            string_search(&query, apps, max_results, &hash_fn, true)
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
    let first_line = output.lines().next().unwrap_or_default();

    first_line.to_string()
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
    let lines = output.lines().map(|s| s.to_string()).collect();

    Ok(lines)
}
