use std::sync::{Arc, Mutex};

use config::Config;
use futures::{future::abortable, stream::AbortHandle, Future};
use gdk::glib::once_cell::sync::Lazy;
use gtk::prelude::*;

mod config;
mod result_templates;
mod search;
mod search_modules;

use search_modules::{search, SearchModule, append_results};

pub static CONF: Lazy<Config> = Lazy::new(|| match config::load_config() {
    Ok(config) => config,
    Err(_) => {
        println!("Failed to load config");
        Config::default()
    }
});

pub static SEARCH_MODULES: Lazy<Vec<Box<dyn SearchModule + Sync + Send>>> =
    Lazy::new(|| search_modules::load_standard_modules());

fn main() {
    let application = gtk::Application::builder()
        .application_id("com.jaspwr.Prober")
        .build();

    application.connect_activate(|app| {
        let window = gtk::ApplicationWindow::builder()
            .application(app)
            .title("Prober")
            .default_width(350)
            .default_height(70)
            .resizable(false)
            .decorated(false)
            // .type_hint(gdk::WindowTypeHint::PopupMenu)
            .build();

        let container = gtk::Box::new(gtk::Orientation::Vertical, 0);

        let list = gtk::ListBox::new();
        list.set_selection_mode(gtk::SelectionMode::Browse);

        let search_field = gtk::Entry::new();

        container.add(&search_field);
        container.add(&list);

        let list = Arc::new(Mutex::new(SafeListBox { list }));

        let list_cpy = list.clone();

        let actions: Arc<Mutex<Vec<Option<Box<dyn Fn() -> ()>>>>> = Arc::new(Mutex::new(vec![]));

        let actions_cpy = actions.clone();

        let fake_first_selected = Arc::new(Mutex::new(false));

        let rt = Arc::new(Mutex::new(tokio::runtime::Runtime::new().unwrap()));

        let mut current_task_handle: Arc<Mutex<Vec<AbortHandle>>> = Arc::new(Mutex::new(vec![]));

        let fake_first_selected_cpy = fake_first_selected.clone();
        search_field.connect_changed(move |entry| {
            // TODO: Loading ect.
            {
                let list = list_cpy.lock().unwrap();
                let children = list.list.children();
                for child in children {
                    list.list.remove(&child);
                }

                let current_task_handle = current_task_handle.clone();
                let mut current_task_handle = current_task_handle.lock().unwrap();
                for handle in current_task_handle.iter() {
                    handle.abort();
                }
                (*current_task_handle).clear()
                // abort here
            }

            //let current_task = current_task.clone();

            let fake_first_selected = fake_first_selected_cpy.clone();
            let actions = actions_cpy.clone();
            let list = list_cpy.clone();
            let rt = rt.clone();
            let query = entry.text().to_string();


            let module_search_futures = SEARCH_MODULES
                .iter()
                .map(|module| module.search(query.clone(), 10))
                .for_each(|f| {
                    let list = list.clone();
                    let (task, handle) = abortable(async move {
                            let results = f.await;
                            append_results(results, list.clone()).await;
                        });
                    current_task_handle.lock().unwrap().push(handle);
                    rt.lock().unwrap().spawn(task);
                });

            //. let (task, handle) = abortable(search(&loaded_modules, query.to_string(), list));

            // *current_task_handle.lock().unwrap() = Some(handle);
            // rt.lock().unwrap().block_on(task);


                // .collect::<Vec<_>>();
            // {
            //     let mut current_task = current_task.lock().unwrap();
            //     if let Some(task) = current_task.as_mut() {
            //         // task.abort();
            //     }
            //     current_task = Some(task);
            // }

        });

        let actions_cpy = actions.clone();
        let fake_first_selected_cpy = fake_first_selected.clone();
        let list_cpy = list.clone();
        search_field.connect_key_press_event(move |_, keyevent| {
            let fake_first_selected = fake_first_selected_cpy.clone();
            let actions = actions_cpy.clone();
            let list = list_cpy.clone();
            let key = keyevent.keyval();

            if fake_first_selected.lock().unwrap().clone() && key == gdk::keys::constants::Down {
                *fake_first_selected.lock().unwrap() = false;
                {
                    let list = list.lock().unwrap();
                    list.list.select_row(Some(&list.list.row_at_index(1).unwrap()));
                }
                return Inhibit(false);
            }

            match key.to_unicode() {
                Some(key) => {
                    if key == 0x1B as char {
                        std::process::exit(0);
                    }
                    if key == 0x0D as char {
                        // This is broken
                        if let Some(action) = actions.lock().unwrap()[0].as_ref() {
                            action();
                            std::process::exit(0);
                        }
                    }
                }
                None => {}
            };
            return Inhibit(false);
        });

        let search_field = Arc::new(search_field);

        list.lock()
            .unwrap().list
            .connect_row_activated(move |list_box, _| {
                let row_id: usize = list_box.selected_row().unwrap().index() as usize;
                let actions = actions.clone();
                {
                    let actions = actions.lock().unwrap();
                    if let Some(action) = actions[row_id].as_ref() {
                        action();
                        std::process::exit(0);
                    }
                }
            });

        let search_field_cpy = search_field.clone();

        list.lock()
            .unwrap().list
            .connect_key_press_event(move |list, key_event| -> Inhibit {
                let key = key_event.keyval();
                let search_field = search_field_cpy.clone();

                let ret = Inhibit(false);

                match key.to_unicode() {
                    Some(key) => {
                        if key == 0x0D as char {
                            return ret;
                        }

                        if key == 0x1B as char {
                            std::process::exit(0);
                            return ret;
                        }

                        let backspace = key == 0x08 as char;

                        let text = search_field.text().to_string();
                        if !backspace {
                            search_field.set_text((text + &key.to_string()).as_str());
                        } else {
                            search_field.set_text(&text[0..text.len()]);
                        }
                        list.unselect_all();
                        search_field.grab_focus();
                        search_field.set_position(-1);
                    }
                    None => {}
                }

                ret
            });

        window.set_child(Some(&container));
        window.show_all();
    });

    application.run();
}

pub struct SafeListBox {
    list: gtk::ListBox,
}

unsafe impl Send for SafeListBox {}
unsafe impl Sync for SafeListBox {}
