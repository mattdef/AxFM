use crate::footer_bar;
use gtk4::{
    Box as GtkBox, Dialog, Grid, Image, Label, Orientation, ResponseType, Window, gio, prelude::*,
};

pub fn show_properties_dialog(parent_window: &Window, file_path: &str) {
    let file = gio::File::for_path(file_path);

    // Create the dialog
    let dialog = Dialog::builder()
        .title("Properties")
        .modal(true)
        .resizable(false)
        .transient_for(parent_window)
        .default_width(400)
        .default_height(450)
        .build();

    // Main container
    let content = dialog.content_area();
    let vbox = GtkBox::new(Orientation::Vertical, 12);
    vbox.set_margin_start(20);
    vbox.set_margin_end(20);
    vbox.set_margin_top(20);
    vbox.set_margin_bottom(20);

    // GIO query
    let query_attrs = "standard::icon,standard::size,standard::type,\
                       standard::content-type,standard::display-name,\
                       time::modified,access::can-read,access::can-write";

    if let Ok(info) =
        file.query_info(query_attrs, gio::FileQueryInfoFlags::NONE, None::<&gio::Cancellable>)
    {
        // Icon
        if let Some(icon) = info.icon() {
            let image = Image::from_gicon(&icon);
            image.set_pixel_size(64);
            image.set_halign(gtk4::Align::Center);
            vbox.append(&image);
        }

        // Grid for properties
        let grid = Grid::new();
        grid.set_row_spacing(8);
        grid.set_column_spacing(15);

        let mut row = 0;

        // Name
        let name = info.display_name();
        add_property_row(&grid, row, "Name:", &name.to_string());
        row += 1;

        // Type
        let file_type = footer_bar::get_file_type_description(&file);
        add_property_row(&grid, row, "Type:", &file_type);
        row += 1;

        // Location
        if let Some(parent) = file.parent() {
            if let Some(parent_path) = parent.path() {
                add_property_row(&grid, row, "Location:", &parent_path.display().to_string());
                row += 1;
            }
        }

        // Size
        let file_type_enum = info.file_type();
        if file_type_enum == gio::FileType::Regular {
            let size = info.size() as u64;
            let formatted_size = footer_bar::format_size(size);
            add_property_row(&grid, row, "Size:", &formatted_size);
        } else if file_type_enum == gio::FileType::Directory {
            add_property_row(&grid, row, "Size:", "Folder");
        }
        row += 1;

        // Modified date
        if let Some(modified) = info.modification_date_time() {
            if let Ok(formatted) = modified.format("%b %d, %Y %H:%M") {
                add_property_row(&grid, row, "Modified:", &formatted.to_string());
                row += 1;
            }
        }

        // Permissions
        // Check if file is readable/writable using has_attribute
        if info.has_attribute("access::can-read") {
            add_property_row(&grid, row, "Readable:", "Yes");
            row += 1;
        }

        if info.has_attribute("access::can-write") {
            add_property_row(&grid, row, "Writable:", "Yes");
            row += 1;
        }

        // Default app (files only)
        if file_type_enum == gio::FileType::Regular {
            if let Some(app) = footer_bar::get_default_app(&file) {
                add_property_row(&grid, row, "Opens with:", &app);
            }
        }

        vbox.append(&grid);
    } else {
        let error_label = Label::new(Some("Unable to read file information"));
        vbox.append(&error_label);
    }

    content.append(&vbox);

    // Add Close button at the bottom
    dialog.add_button("Close", ResponseType::Close);

    dialog.connect_response(|dialog, _| {
        dialog.close();
    });

    dialog.show();
}

fn add_property_row(grid: &Grid, row: i32, label_text: &str, value_text: &str) {
    let label = Label::new(Some(label_text));
    label.set_halign(gtk4::Align::End);
    label.set_valign(gtk4::Align::Start);
    label.add_css_class("dim-label");

    let value = Label::new(Some(value_text));
    value.set_halign(gtk4::Align::Start);
    value.set_valign(gtk4::Align::Start);
    value.set_selectable(true);
    value.set_wrap(true);

    grid.attach(&label, 0, row, 1, 1);
    grid.attach(&value, 1, row, 1, 1);
}
