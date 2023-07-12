use std::{
    mem,
    ops::ControlFlow,
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc, Mutex,
    },
    time::{self, Duration},
};

use config::Config;
use futures::{future::abortable, stream::AbortHandle, Future};
use gdk::glib::idle_add_once;
use gdk::glib::once_cell::sync::Lazy;
use gtk::prelude::*;

mod config;
mod indexing;
mod result_templates;
mod search;
mod search_modules;
mod utils;

static CONTROL: AtomicBool = AtomicBool::new(false);

use search_modules::{SearchModule, SearchResult};

pub static CONF: Lazy<Config> = Lazy::new(|| match config::load_config() {
    Ok(config) => config,
    Err(_) => {
        println!("Failed to load config");
        Config::default()
    }
});

pub static SEARCH_MODULES: Lazy<Vec<Box<dyn SearchModule + Sync + Send>>> =
    Lazy::new(|| search_modules::load_standard_modules());

pub static FAKE_FIRST_SELECTED: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));

fn main() {
    let application = gtk::Application::builder()
        .application_id("com.jaspwr.Prober")
        .build();

    application.connect_activate(|app| {
        let width = 350;
        let height = 300;

        let window = gtk::ApplicationWindow::builder()
            .application(app)
            .title("Prober")
            .default_width(width)
            .default_height(height)
            .resizable(false)
            .decorated(false)
            // .type_hint(gdk::WindowTypeHint::PopupMenu)
            .build();

        window.set_keep_above(true);

        window.set_position(gtk::WindowPosition::Center);

        let container = gtk::Box::new(gtk::Orientation::Vertical, 0);

        let list = gtk::ListBox::new();
        list.set_selection_mode(gtk::SelectionMode::Browse);

        let search_field = gtk::Entry::new();

        container.add(&search_field);
        container.add(&list);

        let list = Arc::new(Mutex::new(SafeListBox { list }));

        let list_cpy = list.clone();

        let rt = Arc::new(Mutex::new(tokio::runtime::Runtime::new().unwrap()));

        let mut current_task_handle: Arc<Mutex<Vec<AbortHandle>>> = Arc::new(Mutex::new(vec![]));

        search_field.connect_changed(move |entry| {
            {
                let current_task_handle = current_task_handle.clone();
                let mut current_task_handle = current_task_handle.lock().unwrap();
                for handle in current_task_handle.iter() {
                    handle.abort();
                }
                (*current_task_handle).clear()
            }

            {
                let list = list_cpy.lock().unwrap();
                let children = list.list.children();
                for child in children {
                    free_entry_data(&child);

                    list.list.remove(&child);
                }
            }

            let list = list_cpy.clone();
            let rt = rt.clone();
            let query = entry.text().to_string();

            SEARCH_MODULES
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
        });

        let list_cpy = list.clone();
        search_field.connect_key_press_event(move |_, keyevent| {
            let list = list_cpy.clone();
            let key = keyevent.keyval();

            handle_search_field_keypress(key, list);

            return Inhibit(false);
        });

        search_field.connect_key_release_event(|_, keyevent| {
            let key = keyevent.keyval();
            if key == gdk::keys::constants::Control_L || key == gdk::keys::constants::Control_R {
                CONTROL.store(false, Ordering::Relaxed);
            }
            Inhibit(false)
        });

        list.lock()
            .unwrap()
            .list
            .connect_row_activated(move |list_box, _| {
                let row = list_box.selected_row().unwrap();
                perform_entry_action(row);
            });

        let search_field = Arc::new(search_field);

        let search_field_cpy = search_field.clone();

        list.lock()
            .unwrap()
            .list
            .connect_key_press_event(move |list, key_event| -> Inhibit {
                let key = key_event.keyval();
                let search_field = search_field_cpy.clone();

                handle_list_keypress(key, search_field, list);

                return Inhibit(false);
            });

        list.lock()
            .unwrap()
            .list
            .connect_key_release_event(|_, keyevent| {
                let key = keyevent.keyval();
                if key == gdk::keys::constants::Control_L || key == gdk::keys::constants::Control_R
                {
                    CONTROL.store(false, Ordering::Relaxed);
                }
                Inhibit(false)
            });

        window.set_child(Some(&container));
        window.show_all();
    });

    application.run();
}

fn perform_entry_action(row: gtk::ListBoxRow) {
    use_entry_data(
        &row,
        Box::new(|data| {
            if let Some(action) = data.action.as_ref() {
                action();
                std::process::exit(0);
            }
        }),
    );
}

fn get_entry_id(widget: &gtk::Widget) -> u64 {
    unsafe {
        if let Some(data_ptr) = widget.steal_data::<*mut ResultData>("dat") {
            let data = Box::from_raw(data_ptr);

            let id = data.id;

            let data_ptr = Box::into_raw(data);
            widget.set_data("dat", data_ptr);

            return id;
        } else {
            0
        }
    }
}

fn get_entry_relevance(widget: gtk::Widget) -> f32 {
    unsafe {
        if let Some(data_ptr) = widget.steal_data::<*mut ResultData>("dat") {
            let data = Box::from_raw(data_ptr);

            let relevance = data.relevance;

            let data_ptr = Box::into_raw(data);
            widget.set_data("dat", data_ptr);

            return relevance;
        } else {
            0.0
        }
    }
}

fn free_entry_data(widget: &gtk::Widget) {
    unsafe {
        if let Some(data_ptr) = widget.steal_data::<*mut ResultData>("dat") {
            let data = Box::from_raw(data_ptr);
            drop(data);
        }
    }
}

fn use_entry_data(widget: &gtk::ListBoxRow, action: Box<dyn Fn(&ResultData) -> ()>) {
    unsafe {
        if let Some(data_ptr) = widget.steal_data::<*mut ResultData>("dat") {
            let data = Box::from_raw(data_ptr);
            action(&data);
            let data_ptr = Box::into_raw(data);
            widget.set_data("dat", data_ptr);
        }
    }
}

fn handle_search_field_keypress(key: gdk::keys::Key, list: Arc<Mutex<SafeListBox>>) {
    if FAKE_FIRST_SELECTED.lock().unwrap().clone()
        && (key == gdk::keys::constants::Down || key == gdk::keys::constants::Up)
    {
        (*FAKE_FIRST_SELECTED.lock().unwrap()) = false;
        let moves = if key == gdk::keys::constants::Down {
            1
        } else {
            -1
        };
        {
            let list = list.lock().unwrap();
            if let Some(current_row) = list.list.selected_row() {
                let selected_index = current_row.index();
                if let Some(next_row) = list.list.row_at_index(selected_index + moves) {
                    list.list.select_row(Some(&next_row));
                }
            }
        }
        return;
    }

    if key == gdk::keys::constants::Escape {
        std::process::exit(0);
    }

    if key == gdk::keys::constants::Return {
        let first_entry = {
            let list = list.lock().unwrap();
            list.list.row_at_index(0)
        };

        if let Some(first_entry) = first_entry {
            perform_entry_action(first_entry);
        }

        return;
    }
}

fn handle_list_keypress(key: gdk::keys::Key, search_field: Arc<gtk::Entry>, list: &gtk::ListBox) {
    if key == gdk::keys::constants::Control_L || key == gdk::keys::constants::Control_R {
        CONTROL.store(true, Ordering::Relaxed);
    }

    if key == gdk::keys::constants::Escape {
        std::process::exit(0);
    }

    if key == gdk::keys::constants::Return {
        return;
    }

    if key == gdk::keys::constants::Left {
        search_field.grab_focus();
        let text = search_field.text().to_string();
        let control = CONTROL.load(Ordering::Relaxed);

        let mut pos = text.len() as i32 - 1;
        if control {
            while pos > 0 {
                if text.chars().nth(pos as usize).unwrap() == ' ' {
                    break;
                }
                pos -= 1;
            }
            if pos > 0 {
                pos += 1;
            }
        }

        search_field.set_position(pos);
        return;
    }

    let backspace = key == gdk::keys::constants::BackSpace;

    match key.to_unicode() {
        Some(key) => {
            let control = CONTROL.load(Ordering::Relaxed);
            let text = search_field.text().to_string();
            if !backspace {
                if (key == 'a' || key == 'A') && control {
                    // HACK: The text is generally just selected
                    //       by default. Returning means we don't
                    //       call unselect all.
                    search_field.grab_focus();
                    return;
                }
                search_field.set_text((text + &key.to_string()).as_str());
            } else {
                if text.len() > 0 {
                    if control {
                        search_field.set_text("");
                    } else {
                        search_field.set_text(&text[0..text.len() - 1]);
                    }
                }
            }
            list.unselect_all();
            search_field.grab_focus();
            search_field.set_position(-1);
        }
        None => {}
    }
}

pub struct SafeListBox {
    list: gtk::ListBox,
}

unsafe impl Send for SafeListBox {}
unsafe impl Sync for SafeListBox {}

pub async fn append_results(results: Vec<SearchResult>, list: Arc<std::sync::Mutex<SafeListBox>>) {
    let mut results = results;
    results.sort_by(|a, b| a.relevance.partial_cmp(&b.relevance).unwrap());

    idle_add_once(move || {
        let list = list.lock().unwrap();
        for result in results {
            let index = match find_slot(result.relevance, result.id, &list) {
                Some(index) => index,
                None => continue,
            };

            let entry = (result.render)();

            list.list.insert(&entry, index);
            let row = list.list.row_at_index(index).unwrap();

            let data = ResultData {
                relevance: result.relevance,
                id: result.id,
                action: result.on_select,
            };

            unsafe {
                let data = Box::new(data);
                let data_ptr = Box::into_raw(data);
                row.set_data("dat", data_ptr);
            }

            entry.show_all();
        }

        if list.list.selected_row().is_none()
            || FAKE_FIRST_SELECTED.lock().unwrap().clone() {
            if let Some(first_row) = list.list.row_at_index(0) {
                list.list.select_row(Some(&first_row));
            }
            (*FAKE_FIRST_SELECTED.lock().unwrap()) = true;
        }
    });
}

fn find_slot(relevance: f32, id: u64, list: &SafeListBox) -> Option<i32> {
    let children = list.list.children();

    for child in &children {
        let entry_id = get_entry_id(child);
        if entry_id == id {
            return None;
        }
    }

    let len = children.len();

    let mut index = 0;
    for child in children {
        let child_relevance = get_entry_relevance(child);
        if child_relevance < relevance {
            return Some(index);
        }
        index += 1;
    }
    Some(len as i32)
}

struct ResultData {
    relevance: f32,
    id: u64,
    action: Option<Box<dyn Fn() -> () + Sync + Send>>,
}
