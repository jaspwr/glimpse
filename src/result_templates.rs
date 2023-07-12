use gtk::traits::{GridExt, ContainerExt, WidgetExt};

pub fn standard_entry(
    name: String,
    icon: Option<gtk::Image>,
    description: Option<String>,
) -> gtk::Box {
    let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let label = gtk::Label::with_mnemonic(&name);
    label.set_halign(gtk::Align::Start);

    let grid = gtk::Grid::new();

    let text_container = gtk::Box::new(gtk::Orientation::Vertical, 0);
    text_container.add(&label);

    if let Some(description) = description {
        let mut description = description;
        if description.len() > 40 {
            description.truncate(40);
            description.push_str("…");
        }
        let description_label = gtk::Label::with_mnemonic(&description);
        description_label.set_halign(gtk::Align::Start);
        description_label.set_opacity(0.6);
        text_container.add(&description_label);
    }

    if let Some(icon) = icon {
        grid.attach(&icon, 0, 0, 1, 1);
        icon.set_margin_start(10);
        icon.set_margin_end(10);
        grid.attach_next_to(&text_container, Some(&icon), gtk::PositionType::Right, 1, 1);
    } else {
        grid.attach(&text_container, 0, 0, 1, 1);
    }

    container.add(&grid);

    container
}
