use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use biases::increment_bias;
use futures::{future::abortable, stream::AbortHandle};
use gdk::glib::once_cell::sync::Lazy;
use gdk::{glib::idle_add_once, SeatCapabilities};
use gtk::prelude::*;

mod biases;
mod exec;
mod icon;
mod prelude;
mod result_templates;
mod search;
mod search_modules;
mod utils;

static CONTROL: AtomicBool = AtomicBool::new(false);

use glimpse::config::{CONF, CONF_FILE_PATH};
use search_modules::{SearchModule, SearchResult};

pub static RUNTIME: Lazy<BoxedRuntime> = Lazy::new(|| {
    let rt = tokio::runtime::Runtime::new().unwrap();
    Arc::new(Mutex::new(rt))
});

pub static SEARCH_MODULES: Lazy<Vec<Box<dyn SearchModule + Sync + Send>>> =
    Lazy::new(|| search_modules::load_standard_modules(RUNTIME.clone()));

pub static FAKE_FIRST_SELECTED: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));

pub type BoxedRuntime = Arc<Mutex<tokio::runtime::Runtime>>;

fn main() {
    let application = gtk::Application::builder()
        .application_id("com.jaspwr.glimpse")
        .build();

    application.connect_activate(|app| {
        let width = CONF.window.width as i32;
        let height = CONF.window.height as i32;

        let window = gtk::ApplicationWindow::builder()
            .application(app)
            .title("glimpse")
            .default_width(width)
            .default_height(1)
            .resizable(false)
            .decorated(false)
            .type_hint(gdk::WindowTypeHint::PopupMenu)
            .build();

        window.set_keep_above(true);

        let (window_x, window_y) = window.position();

        let display = window.display();
        let monitor = display.monitor_at_point(window_x, window_y).unwrap();
        let monitor = monitor.geometry();

        let win_x = (monitor.width() - width) / 2;
        let win_y = (monitor.height() - height) / 2;

        window.move_(win_x, win_y);

        let container = gtk::Box::new(gtk::Orientation::Vertical, 0);

        let list = gtk::ListBox::new();
        list.set_selection_mode(gtk::SelectionMode::Browse);

        let search_field = gtk::Entry::new();

        let search_field_height = 30;

        container.add(&search_field);

        let scrolled_window = gtk::ScrolledWindow::new(
            Option::<&gtk::Adjustment>::None,
            Option::<&gtk::Adjustment>::None,
        );

        scrolled_window.set_policy(gtk::PolicyType::External, gtk::PolicyType::Automatic);

        scrolled_window.set_size_request(width, height - search_field_height);

        scrolled_window.add(&list);

        container.add(&scrolled_window);

        let boxed_scrolled_window = Arc::new(scrolled_window);

        let style_provider = gtk::CssProvider::new();

        if CONF.visual.result_borders {
            if CONF.visual.dark_result_borders {
                #[rustfmt::skip]
                style_provider.load_from_data(
                    ".outlined-container {
                        border-bottom: 1px solid rgba(0,0,0,.1);
                    }".as_bytes(),).unwrap();
            } else {
                #[rustfmt::skip]
                style_provider.load_from_data(
                    ".outlined-container {
                        border-bottom: 1px solid rgba(255,255,255,.1);
                    }".as_bytes(),).unwrap();
            }
        }

        let list = Arc::new(Mutex::new(SafeListBox { list }));

        let list_cpy = list.clone();

        let rt = Arc::new(Mutex::new(tokio::runtime::Runtime::new().unwrap()));

        let current_task_handle: Arc<Mutex<Vec<AbortHandle>>> = Arc::new(Mutex::new(vec![]));

        let current_task_handle_cpy = current_task_handle.clone();
        let rt_cpy = rt.clone();
        search_field.connect_changed(move |entry| {
            if entry.text().is_empty() {
                boxed_scrolled_window.clone().hide();
            } else {
                boxed_scrolled_window.clone().show_all();
            }

            let current_task_handle = current_task_handle_cpy.clone();
            {
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
            let rt = rt_cpy.clone();
            let query = entry.text().to_string();

            perform_search(query, list, current_task_handle, rt);
        });

        let list_cpy = list.clone();
        search_field.connect_key_press_event(move |_, keyevent| {
            let list = list_cpy.clone();
            let key = keyevent.keyval();

            handle_search_field_keypress(key, list);

            Inhibit(false)
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

                Inhibit(false)
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

        if let Some(err) = CONF.error.as_ref() {
            #[rustfmt::skip]
            style_provider.load_from_data(".error-title {
                color: red;
                font-weight: bold;
            }

            .error-details {
                color: red;
                font-family: monospace, monospace;
            }".as_bytes()).unwrap();

            let error_title = format!("Error loading config; using default config. Either correct the errors or delete the config file to have a new one generated. Your config file is located at: \"{}\"",
                CONF_FILE_PATH.to_str().unwrap());

            let error_title = gtk::Label::new(Some(&error_title.as_str()));

            error_title.set_halign(gtk::Align::Start);
            error_title.set_line_wrap(true);
            error_title.set_line_wrap_mode(pango::WrapMode::WordChar);
            error_title.set_max_width_chars(40);

            error_title.style_context().add_class("error-title");
            container.add(&error_title);
            error_title.show();


            let error_details = gtk::Label::new(Some(err));
            error_details.style_context().add_class("error-details");
            container.add(&error_details);
            error_details.show();
        }

        window.set_child(Some(&container));

        let screen = gdk::Screen::default().unwrap();
        gtk::StyleContext::add_provider_for_screen(
            &screen,
            &style_provider,
            gtk::STYLE_PROVIDER_PRIORITY_USER,
        );

        container.show();
        window.show();
        search_field.show();

        search_field.grab_focus();

        window.connect_focus_in_event(|window, _| {
            grab_seat(&window.window().unwrap());
            gtk::Inhibit(false)
        });

        window.activate();

        window.set_size_request(width, 1);

        perform_search("".to_string(), list, current_task_handle, rt);
    });

    application.run();
}

fn perform_search(
    query: String,
    list: Arc<Mutex<SafeListBox>>,
    current_task_handle: Arc<Mutex<Vec<AbortHandle>>>,
    rt: BoxedRuntime,
) {
    SEARCH_MODULES
        .iter()
        .filter(|module| module.is_ready())
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
}

fn perform_entry_action(row: gtk::ListBoxRow) {
    use_entry_data(
        &row,
        Box::new(|data| {
            if let Some(action) = data.action.as_ref() {
                action();
                increment_bias(data.id, 0.5);
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

            id
        } else {
            0
        }
    }
}

#[inline]
fn get_entry_relevance(widget: gtk::Widget) -> f32 {
    unsafe {
        if let Some(data_ptr) = widget.steal_data::<*mut ResultData>("dat") {
            let data = Box::from_raw(data_ptr);

            let relevance = data.relevance;

            let data_ptr = Box::into_raw(data);
            widget.set_data("dat", data_ptr);

            relevance
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
    if *FAKE_FIRST_SELECTED.lock().unwrap()
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

        let pos = step_back_word(text, control);

        search_field.set_position(pos);
        return;
    }

    let backspace = key == gdk::keys::constants::BackSpace;

    if let Some(key) = key.to_unicode() {
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
            if !text.is_empty() {
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
}

#[inline]
fn step_back_word(text: String, control: bool) -> i32 {
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
    pos
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

            if CONF.visual.result_borders {
                row.style_context().add_class("outlined-container");
            }

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

        remove_excess_entries(&list);

        if list.list.selected_row().is_none() || *FAKE_FIRST_SELECTED.lock().unwrap() {
            if let Some(first_row) = list.list.row_at_index(0) {
                list.list.select_row(Some(&first_row));
            }
            (*FAKE_FIRST_SELECTED.lock().unwrap()) = true;
        }
    });
}

fn remove_excess_entries(list: &std::sync::MutexGuard<'_, SafeListBox>) {
    while list.list.children().len() > CONF.max_results {
        let children = list.list.children();
        let last_child = children.last().unwrap();
        free_entry_data(last_child);
        list.list.remove(last_child);
    }
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
    #[allow(clippy::explicit_counter_loop)]
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

#[inline]
fn grab_seat(window: &gtk::gdk::Window) {
    let display = window.display();
    let seat = display.default_seat().unwrap();

    let capabilities = gdk_sys::GDK_SEAT_CAPABILITY_POINTER | gdk_sys::GDK_SEAT_CAPABILITY_KEYBOARD;

    let status = seat.grab(
        window,
        unsafe { SeatCapabilities::from_bits_unchecked(capabilities) },
        true,
        None,
        None,
        None,
    );

    if status != gtk::gdk::GrabStatus::Success {
        println!("Grab failed: {:?}", status);
    }
}
