use std::{collections::HashMap, path::PathBuf, sync::Arc};

use async_trait::async_trait;
use sqlite::State;

use crate::{
    exec::xdg_open, icon, result_templates::standard_entry, search::string_search,
    utils::simple_hash, BoxedRuntime,
};

use super::{SearchModule, SearchResult};

pub struct WebBookmarks {
    data: Arc<tokio::sync::Mutex<Option<BookMarksData>>>,
}

struct BookMarksData {
    titles: Vec<String>,
    url_map: HashMap<String, String>,
}

#[async_trait]
impl SearchModule for WebBookmarks {
    fn is_ready(&self) -> bool {
        true
    }

    async fn search(&self, query: String, max_results: u32) -> Vec<SearchResult> {
        let query = query.to_lowercase();
        let lock = self.data.lock().await;
        let list = lock.as_ref();
        if let Some(list) = list {
            string_search(&query, &list.titles, max_results, Box::new(id_hash), false)
                .into_iter()
                .map(|(name, rel)| Self::create_result(&name, rel, &list.url_map))
                .collect()
        } else {
            vec![]
        }
    }
}

fn id_hash(name: &String) -> u64 {
    (simple_hash(name) >> 3) + 0x123809abedf
}

impl WebBookmarks {
    pub fn new(rt: BoxedRuntime) -> WebBookmarks {
        let data = Arc::new(tokio::sync::Mutex::new(None));

        let data_cpy = data.clone();
        rt.lock().unwrap().spawn(async move {
            let data = data_cpy;
            let mut list = data.lock().await;
            let mut bookmarks = vec![];

            bookmarks.append(&mut get_chromium_bookmarks());
            bookmarks.append(&mut get_firefox_bookmarks());

            let mut titles = vec![];
            let mut url_map = HashMap::new();

            for bookmark in bookmarks {
                titles.push(bookmark.title.clone());
                url_map.insert(bookmark.title, bookmark.url);
            }

            *list = Some(BookMarksData { titles, url_map });
        });

        WebBookmarks { data }
    }

    fn create_result(
        name: &String,
        relevance: f32,
        url_map: &HashMap<String, String>,
    ) -> SearchResult {
        // let icon = fetch_favicon(&list.url_map.get(&name).unwrap()).await;

        let name_cpy = name.clone();
        let render = move || {
            let icon = icon::from_gtk("emblem-web");
            standard_entry(name_cpy.clone(), icon, None)
        };

        let url = url_map.get(name).unwrap().clone();

        let on_select = move || {
            let _ = xdg_open(&url);
        };

        SearchResult {
            render: Box::new(render),
            relevance,
            id: id_hash(name),
            on_select: Some(Box::new(on_select)),
        }
    }
}

fn get_chromium_bookmarks() -> Vec<BookmarkEntry> {
    if let Some(bookmarks) = __get_chromium_bookmarks() {
        bookmarks
    } else {
        vec![]
    }
}

fn __get_chromium_bookmarks() -> Option<Vec<BookmarkEntry>> {
    let home = home::home_dir()?;

    let search_locations = vec![
        format!("{}/.config/chromium/Default/Bookmarks", home.to_str()?),
        format!("{}/.config/google-chrome/Default/Bookmarks", home.to_str()?),
    ];

    let bookmarks = search_locations
        .into_iter()
        .filter_map(|path| std::fs::read_to_string(path).ok())
        .filter_map(handle_bookmarks_file)
        .flatten()
        .collect::<Vec<BookmarkEntry>>();

    if bookmarks.is_empty() {
        return None;
    }

    Some(bookmarks)
}

struct BookmarkEntry {
    title: String,
    url: String,
}

fn handle_bookmarks_file(unparsed: String) -> Option<Vec<BookmarkEntry>> {
    let json: serde_json::Value = serde_json::from_str(&unparsed).ok()?;

    let search_locations = vec!["bookmark_bar", "other", "synced"];

    let list = search_locations
        .into_iter()
        .filter_map(|name| try_add_list(&json, name))
        .flatten()
        .collect();

    Some(list)
}

fn try_add_list(json: &serde_json::Value, name: &str) -> Option<Vec<BookmarkEntry>> {
    let children = json["roots"][name]["children"].as_array()?;

    let list = handle_children_list(children);

    Some(list)
}

fn handle_children_list(children: &Vec<serde_json::Value>) -> Vec<BookmarkEntry> {
    let mut list = vec![];

    for child in children {
        if let Some(entry) = try_create_child(child) {
            list.push(entry);
        } else {
            if let Some(children) = child["children"].as_array() {
                list.append(&mut handle_children_list(children));
            }
        }
    }

    list
}

fn try_create_child(child: &serde_json::Value) -> Option<BookmarkEntry> {
    let title = child["name"].as_str()?.to_string();
    let url = child["url"].as_str()?.to_string();

    Some(BookmarkEntry { title, url })
}

// struct SafeImage {
//     image: Option<gtk::Image>,
// }

// unsafe impl Send for SafeImage {}
// unsafe impl Sync for SafeImage {}

// async fn fetch_favicon(url: &String) -> SafeImage {
//     if !CONF.use_web_modules {
//         return SafeImage {
//             image: icon::from_gtk("emblem-web"),
//         };
//     }

//     if let Some(data) = request_favicon(url).await {
//          SafeImage {
//             image: icon::from_bytes(&data.into_iter().collect::<Vec<u8>>()),
//         }
//     } else {
//         SafeImage {
//             image: icon::from_gtk("emblem-web"),
//         }
//     }
// }

// async fn request_favicon(url: &String) -> Option<Vec<u8>> {
//     let domain = url::Url::parse(url).ok()?.domain()?.to_string();
//     let url = format!("https://www.google.com/s2/favicons?domain={}", domain);

//     reqwest::get(url).await.ok()?.bytes().await.ok().into_iter().collect::Vec<u8>>()
// }

fn get_firefox_bookmarks() -> Vec<BookmarkEntry> {
    let home = home::home_dir().unwrap();
    let firefox_path = home.join(".mozilla").join("firefox");
    let dir = match std::fs::read_dir(firefox_path) {
        Ok(dir) => dir,
        Err(_) => return vec![],
    };

    dir.into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().unwrap().is_dir())
        .map(|entry| entry.path().join("places.sqlite"))
        .filter(|path| path.exists())
        .filter_map(|path| query_firefox_db(path).ok())
        .flatten()
        .collect()
}

fn query_firefox_db(db_path: PathBuf) -> Result<Vec<BookmarkEntry>, Box<dyn std::error::Error>> {
    let mut ret = vec![];

    let connection = sqlite::open(db_path)?;

    let query = "SELECT title, fk FROM moz_bookmarks;";

    let mut statement = connection.prepare(query)?;

    while let Ok(State::Row) = statement.next() {
        let title = statement.read::<String, _>("title")?;
        let fk = statement.read::<i64, _>("fk")?;
        let mut url = String::new();

        if fk == 0 {
            continue;
        }

        let query = "SELECT url FROM moz_places WHERE id = ?";
        let mut statement = connection.prepare(query)?;
        statement.bind((1, fk))?;

        while let Ok(State::Row) = statement.next() {
            url = statement.read::<String, _>("url")?;
        }

        ret.push(BookmarkEntry { title, url });
    }
    Ok(ret)
}
