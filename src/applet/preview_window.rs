use std::{path::PathBuf, sync::{Mutex, Arc}};

use gdk::gdk_pixbuf;
use glimpse::config::CONF;
use gtk::traits::{ContainerExt, WidgetExt, ScrolledWindowExt, LabelExt, GridExt, StyleContextExt};
use pango::{WrapMode, glib::idle_add_once};
// use poppler::PopplerDocument;

use glimpse::prelude::*;

pub struct PreviewWindow {
    pub container: Arc<Mutex<SafeBox>>,
    pub showing: PreviewWindowShowing,
}

unsafe impl Sync for PreviewWindow {}
unsafe impl Send for PreviewWindow {}

#[derive(PartialEq, Clone)]
pub enum PreviewWindowShowing {
    None,
    File(PathBuf),
    Text(String),
    Error(String),
}

#[derive(Debug, Clone)]
pub enum PreviewWindowContents {
    None,
    Image(PathBuf),
    TextFile(String, PathBuf),
    Directory(PathBuf),
    Error(String)
}

impl PreviewWindow {
    pub async fn update(&mut self) {
        if self.showing == PreviewWindowShowing::None {
            self.hide();
            return;
        }

        match self.showing.clone() {
            PreviewWindowShowing::None => unreachable!(),
            PreviewWindowShowing::File(path) => {
                if let Some(prev) = create_file_preview(path.clone()).await {
                    let container_cpy = self.container.clone();
                    idle_add_once(move || {
                        create_file_preview_widget(container_cpy, prev, path);
                    });
                } else {
                    self.hide();
                }
            }
            _ => {
                todo!()
            },
        }
    }

    pub fn hide(&mut self) {
        let container = self.container.clone();
        idle_add_once(move || {
            container.lock().unwrap().container.hide();
        });
    }

    pub async fn set(&mut self, showing: PreviewWindowShowing) {
        self.showing = showing;
        self.update().await;
    }
}

fn create_file_preview_widget(container_cpy: Arc<Mutex<SafeBox>>, prev: PreviewWindowContents, path: PathBuf) -> Option<()> {
    let container = container_cpy.clone();
    let container = container.lock().unwrap();

    container.container.foreach(|child| {
        container.container.remove(child);
    });

    let prev = prev.clone();

    container.container.add(&match prev {
        PreviewWindowContents::None => unreachable!(),
        PreviewWindowContents::Image(path) => load_image(&path)?,
        PreviewWindowContents::TextFile(text, _) => plain_text_preview(text),
        PreviewWindowContents::Directory(path) => dir_listing(&path),
        PreviewWindowContents::Error(text) => plain_text_preview(text),
    });

    let label = gtk::Label::new(Some(&format!("{}", path.to_str().unwrap())));
    label.set_line_wrap(true);
    label.set_wrap_mode(WrapMode::Char);
    label.set_max_width_chars(20);
    container.container.add(&label);

    container.container.show_all();
    Some(())
}

async fn trunc_long_lines(text: String) -> String {
    const SPLIT_LEN: usize = 600;

    let mut new_text = String::new();
    for line in text.lines() {
        if line.len() > SPLIT_LEN {
            new_text.push_str(&line[..SPLIT_LEN]);
            new_text.push_str("…");
        } else {
            new_text.push_str(line);
        }
        new_text.push('\n');
    }
    new_text
}

async fn create_file_preview(path: PathBuf) -> Option<PreviewWindowContents> {
    if path.is_dir() {
        return Some(PreviewWindowContents::Directory(path.clone()));
    }

    if let Some(widget) = try_from_infer(&path).await {
        return Some(widget);
    }

    if let Some(widget) = create_plain_text_file_preview(&path).await {
        return Some(widget);
    }

    None
}

fn dir_listing(path: &PathBuf) -> gtk::Box {
    let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let dir = path.read_dir();
    for entry in dir.unwrap() {
        if let Ok(entry) = entry {
            let __file_name = entry.file_name();
            let file_name = __file_name.to_str().unwrap();
            let (name, icon_name) = if entry.path().is_dir() {
                (format!("/{}", file_name), "folder")
            } else {
                (format!("{}", file_name), "folder-documents-symbolic")
            };

            let grid = gtk::Grid::new();
            let icon = gtk::Image::from_icon_name(Some(icon_name), gtk::IconSize::Button);
            let label = gtk::Label::new(Some(&name));
            grid.attach(&icon, 0, 0, 1, 1);
            grid.attach(&label, 1, 0, 1, 1);
            container.add(&grid);
        }
    }
    let scrolled_window = create_scrolled_window();
    scrolled_window.add(&container);
    let outer_container = gtk::Box::new(gtk::Orientation::Vertical, 0);
    outer_container.add(&scrolled_window);
    outer_container
}

async fn try_from_infer(path: &PathBuf) -> Option<PreviewWindowContents> {
    let kind = infer::get_from_path(&path).ok()??;

    if kind.mime_type().starts_with("text") {
        create_plain_text_file_preview(&path).await?;
    }

    if kind.mime_type().starts_with("image") {
        return Some(PreviewWindowContents::Image(path.clone()));
    }

    None
}

fn load_image(path: &PathBuf) -> Option<gtk::Box> {
    let pixbuf = gdk_pixbuf::Pixbuf::from_file_at_scale(
        path.to_str()?,
        CONF.preview_window.image_size as i32,
        CONF.preview_window.image_size as i32,
        true,
    )
    .ok()?;
    let image = gtk::Image::from_pixbuf(Some(&pixbuf));
    let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
    container.add(&image);
    Some(container)
}

async fn create_plain_text_file_preview(path: &PathBuf) -> Option<PreviewWindowContents> {
    let text = tokio::fs::read_to_string(path.to_str()?).await.ok()?;

    let text = trunc_long_lines(text).await;

    let text = text.trunc(7000);

    Some(PreviewWindowContents::TextFile(text, path.clone()))
}

fn plain_text_preview(text: String) -> gtk::Box {
    let label = gtk::Label::new(Some(&text));
    label.set_valign(gtk::Align::Start);
    label.set_halign(gtk::Align::Start);

    let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let scrolled_window = create_scrolled_window();
    scrolled_window.add(&label);
    container.add(&scrolled_window);
    container
}

fn create_scrolled_window() -> gtk::ScrolledWindow {
    let scrolled_window = gtk::ScrolledWindow::new(
        Option::<&gtk::Adjustment>::None,
        Option::<&gtk::Adjustment>::None,
    );
    scrolled_window.style_context().add_class("preview-text");
    scrolled_window.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);
    scrolled_window.set_size_request(
        CONF.preview_window.width as i32 - 50,
        CONF.window.height as i32 - 50,
    );
    scrolled_window
}

#[derive(Debug, Clone)]
pub struct SafeBox {
    pub container: gtk::Box,
}

unsafe impl Sync for SafeBox {}
unsafe impl Send for SafeBox {}

// fn render_first_page(pdf_path: PathBuf) -> Option<DrawingArea> {
//     let drawing_area = DrawingArea::new();

//     if let Ok(doc) = PopplerDocument::new_from_file(&pdf_path, "") {
//         if let Some(page) = doc.get_page(0) {
//             let (pdf_width, pdf_height) = page.get_size();
//             let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, pdf_width as i32, pdf_height as i32).ok()?;
//             let cr = Context::new(&surface).ok()?;

//             cr.set_source_rgb(1.0, 1.0, 1.0);
//             cr.paint();



//             drawing_area.connect_draw(move |_, context| {
//                 context.set_source_surface(&surface, 0.0, 0.0);
//                 context.paint();
//                 Inhibit(false)
//             });
//         }
//     }

//     Some(drawing_area)
// }

