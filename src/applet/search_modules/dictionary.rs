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

use async_trait::async_trait;
use gtk::traits::{ContainerExt, GridExt, LabelExt, WidgetExt};

use crate::utils;

use super::{SearchModule, SearchResult};

pub struct Dictionary {}

impl Dictionary {
    pub fn new() -> Dictionary {
        Dictionary {}
    }
}

#[async_trait]
impl SearchModule for Dictionary {
    async fn search(&self, query: String, _: u32) -> Vec<SearchResult> {
        if query.is_empty() {
            return vec![];
        }

        // wait 0.5 seconds to allow the user to type more
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        match try_fetch(query).await {
            Some(result) => vec![result],
            None => vec![],
        }
    }
}

async fn try_fetch(query: String) -> Option<SearchResult> {
    let mut query = query.to_lowercase();
    let mut relevance: f32 = 1.5;

    if query.is_empty() {
        return None;
    }

    if query.starts_with("define ") {
        query = query.replace("define ", "");
        relevance += 2.0;
    } else if query.ends_with(" meaning") {
        query = query.replace(" meaning", "");
        relevance += 2.0;
    } else if query.ends_with(" definition") {
        query = query.replace(" definition", "");
        relevance += 2.0;
    }

    for c in query.chars() {
        if !c.is_alphabetic() {
            return None;
        }
    }

    #[rustfmt::skip]
    let body = reqwest::get(format!("https://api.dictionaryapi.dev/api/v2/entries/en/{}", query))
        .await.ok()?.text().await.ok()?;

    create_result(body, relevance)
}

fn create_result(response: String, relevance: f32) -> Option<SearchResult> {
    let response: serde_json::Value = match serde_json::from_str(&response) {
        Ok(response) => response,
        Err(_) => return None,
    };

    let word = response[0]["word"].as_str()?.to_string();

    let phonetics = response[0]["phonetics"][1]["text"]
        .as_str()
        .map(|ph| ph.to_string());

    let part_of_speach = match response[0]["meanings"][0]["partOfSpeech"].as_str() {
        Some(particle_of_speach) => particle_of_speach.to_string(),
        None => "".to_string(),
    };

    let definition = response[0]["meanings"][0]["definitions"][0]["definition"]
        .as_str()?
        .to_string();

    let id = utils::simple_hash(&word) + 0xa0a0a0a0;

    let render = move || {
        let word = gtk::Label::new(Some(&word));
        let particle_of_speach = gtk::Label::new(Some(&part_of_speach));
        let definition = gtk::Label::new(Some(&definition));
        let def_container = gtk::Box::new(gtk::Orientation::Vertical, 0);

        def_container.add(&word);

        if let Some(phonetics_text) = phonetics.clone() {
            let phonetics_element = gtk::Label::new(Some(&phonetics_text));
            phonetics_element.set_halign(gtk::Align::Start);
            def_container.add(&phonetics_element);
        }

        def_container.add(&particle_of_speach);
        def_container.add(&definition);

        let word_attributes = pango::AttrList::new();
        let mut word_desc = pango::FontDescription::from_string("24");
        word_desc.set_family("Times New Roman");
        let word_size_attrib = pango::AttrFontDesc::new(&word_desc);
        word_attributes.insert(word_size_attrib);

        def_container.set_margin(10);
        def_container.set_margin_top(4);

        word.set_halign(gtk::Align::Start);
        word.set_attributes(Some(&word_attributes));

        let particle_of_speach_attributes = pango::AttrList::new();
        let mut particle_of_speach_desc = pango::FontDescription::new();
        particle_of_speach_desc.set_weight(pango::Weight::Bold);
        let particle_of_speach_size_attrib = pango::AttrFontDesc::new(&particle_of_speach_desc);
        particle_of_speach_attributes.insert(particle_of_speach_size_attrib);

        particle_of_speach.set_halign(gtk::Align::Start);
        particle_of_speach.set_attributes(Some(&particle_of_speach_attributes));

        definition.set_halign(gtk::Align::Start);
        definition.set_line_wrap(true);
        definition.set_line_wrap_mode(pango::WrapMode::WordChar);
        definition.set_max_width_chars(40);

        let dict_icon = gtk::Image::from_icon_name(
            Some("accessories-dictionary-symbolic"),
            gtk::IconSize::LargeToolbar,
        );
        dict_icon.set_halign(gtk::Align::Start);
        dict_icon.set_valign(gtk::Align::Start);
        dict_icon.set_margin(10);
        dict_icon.set_margin_top(15);

        let grid = gtk::Grid::new();
        grid.attach(&dict_icon, 0, 0, 1, 1);
        grid.attach_next_to(
            &def_container,
            Some(&dict_icon),
            gtk::PositionType::Right,
            1,
            1,
        );

        let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
        container.add(&grid);
        container
    };

    Some(SearchResult {
        render: Box::new(render),
        relevance,
        id,
        on_select: None,
        preview_window_data: crate::preview_window::PreviewWindowShowing::None,
    })
}
