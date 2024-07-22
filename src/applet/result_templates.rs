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

use gtk::traits::{ContainerExt, GridExt, StyleContextExt, WidgetExt};

use glimpse::prelude::*;

pub fn standard_entry(
    name: String,
    icon: Option<gtk::Image>,
    description: Option<String>,
) -> gtk::Box {
    let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let name = name.trunc(40);
    let label = gtk::Label::with_mnemonic(&name);
    label.style_context().add_class("result-title");
    label.set_halign(gtk::Align::Start);

    let grid = gtk::Grid::new();

    let text_container = gtk::Box::new(gtk::Orientation::Vertical, 0);
    text_container.add(&label);

    if let Some(description) = description {
        let description = description.trunc(40);
        let description_label = gtk::Label::with_mnemonic(&description);
        description_label.set_halign(gtk::Align::Start);
        description_label.set_opacity(0.6);
        description_label
            .style_context()
            .add_class("result-subtext");
        text_container.add(&description_label);
    }

    if let Some(icon) = icon {
        grid.attach(&icon, 0, 0, 1, 1);
        icon.set_margin_start(10);
        icon.set_margin_end(10);
        icon.style_context().add_class("result-icon");
        grid.attach_next_to(&text_container, Some(&icon), gtk::PositionType::Right, 1, 1);
    } else {
        grid.attach(&text_container, 0, 0, 1, 1);
    }

    container.add(&grid);

    container
}
