use gtk::traits::{GridExt, ContainerExt, WidgetExt};

pub fn standard_entry(
    name: String,
    icon: Option<gtk::Image>,
    description: Option<String>,
) -> gtk::Box {
    let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let label = gtk::Label::with_mnemonic(&name);
    let grid = gtk::Grid::new();

    if let Some(icon) = icon {
        grid.attach(&icon, 0, 0, 1, 1);
        icon.set_margin_start(10);
        icon.set_margin_end(10);
        grid.attach_next_to(&label, Some(&icon), gtk::PositionType::Right, 1, 1);
    } else {
        grid.attach(&label, 0, 0, 1, 1);
    }

    container.add(&grid);

    if let Some(description) = description {
        let description_label = gtk::Label::with_mnemonic(&description);
        container.add(&description_label);
    }

    container
}
