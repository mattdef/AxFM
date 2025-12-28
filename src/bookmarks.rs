use crate::state::FmState;
use gtk4::{
    Box as GtkBox, Button, Dialog, Label, ListBox, ListBoxRow, Orientation, ResponseType,
    ScrolledWindow, StringList, Window, gio, glib, prelude::*,
};
use std::{cell::RefCell, path::Path, rc::Rc};

#[derive(Debug, Clone)]
pub struct Bookmark {
    pub name: String,
    pub path: String,
}

impl Bookmark {
    pub fn new(name: String, path: String) -> Self {
        Self { name, path }
    }

    pub fn from_file(file: &gio::File) -> Self {
        let name = file
            .basename()
            .and_then(|n| n.to_str().map(|s| s.to_owned()))
            .unwrap_or_else(|| "Bookmark".to_string());

        let path =
            file.path().map(|p| p.display().to_string()).unwrap_or_else(|| file.uri().to_string());

        Self { name, path }
    }

    pub fn to_gio_file(&self) -> gio::File {
        if Path::new(&self.path).exists() {
            gio::File::for_path(&self.path)
        } else {
            gio::File::for_uri(&self.path)
        }
    }
}

// Manual JSON serialization (no serde dependency)
pub fn save_bookmarks(bookmarks: &[Bookmark]) -> Result<(), std::io::Error> {
    let config_dir = std::env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        format!("{}/.config", home)
    });

    let config_path = format!("{}/axfm", config_dir);
    std::fs::create_dir_all(&config_path)?;

    let file_path = format!("{}/bookmarks.json", config_path);

    // Build JSON manually
    let mut json = String::from("{\"bookmarks\":[");
    for (i, bm) in bookmarks.iter().enumerate() {
        if i > 0 {
            json.push(',');
        }
        // Escape quotes and backslashes in name and path
        let name = bm.name.replace('\\', "\\\\").replace('"', "\\\"");
        let path = bm.path.replace('\\', "\\\\").replace('"', "\\\"");
        json.push_str(&format!("{{\"name\":\"{}\",\"path\":\"{}\"}}", name, path));
    }
    json.push_str("]}");

    std::fs::write(&file_path, json)?;
    Ok(())
}

pub fn load_bookmarks() -> Vec<Bookmark> {
    match load_bookmarks_impl() {
        Ok(bookmarks) => bookmarks,
        Err(e) => {
            eprintln!("Warning: Could not load bookmarks: {}", e);
            Vec::new() // Start with empty bookmarks
        }
    }
}

fn load_bookmarks_impl() -> Result<Vec<Bookmark>, Box<dyn std::error::Error>> {
    let config_dir = std::env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        format!("{}/.config", home)
    });

    let file_path = format!("{}/axfm/bookmarks.json", config_dir);

    if !Path::new(&file_path).exists() {
        return Ok(Vec::new()); // First run, no bookmarks yet
    }

    let content = std::fs::read_to_string(&file_path)?;

    // Simple manual JSON parsing
    parse_bookmarks_json(&content)
}

fn parse_bookmarks_json(json: &str) -> Result<Vec<Bookmark>, Box<dyn std::error::Error>> {
    let mut bookmarks = Vec::new();

    // Find "bookmarks" array
    let start = json.find("[").ok_or("Invalid JSON: missing array start")?;
    let end = json.rfind("]").ok_or("Invalid JSON: missing array end")?;

    let array_content = &json[start + 1..end];

    if array_content.trim().is_empty() {
        return Ok(bookmarks);
    }

    // Split by object boundaries (simple parsing)
    let mut depth = 0;
    let mut current_obj = String::new();

    for ch in array_content.chars() {
        match ch {
            '{' => {
                depth += 1;
                current_obj.push(ch);
            }
            '}' => {
                depth -= 1;
                current_obj.push(ch);
                if depth == 0 && !current_obj.trim().is_empty() {
                    if let Ok(bookmark) = parse_bookmark_object(&current_obj) {
                        bookmarks.push(bookmark);
                    }
                    current_obj.clear();
                }
            }
            _ => {
                if depth > 0 {
                    current_obj.push(ch);
                }
            }
        }
    }

    Ok(bookmarks)
}

fn parse_bookmark_object(obj: &str) -> Result<Bookmark, Box<dyn std::error::Error>> {
    let name = extract_json_field(obj, "name")?;
    let path = extract_json_field(obj, "path")?;
    Ok(Bookmark::new(name, path))
}

fn extract_json_field(obj: &str, field: &str) -> Result<String, Box<dyn std::error::Error>> {
    let pattern = format!("\"{}\":", field);
    let start = obj.find(&pattern).ok_or(format!("Field '{}' not found", field))? + pattern.len();

    let after_colon = &obj[start..].trim_start();
    if !after_colon.starts_with('"') {
        return Err("Expected quoted string".into());
    }

    let mut value = String::new();
    let mut escaped = false;
    let mut chars = after_colon.chars().skip(1);

    while let Some(ch) = chars.next() {
        if escaped {
            match ch {
                'n' => value.push('\n'),
                't' => value.push('\t'),
                'r' => value.push('\r'),
                '\\' => value.push('\\'),
                '"' => value.push('"'),
                _ => {
                    value.push('\\');
                    value.push(ch);
                }
            }
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            break;
        } else {
            value.push(ch);
        }
    }

    Ok(value)
}

pub fn show_manage_bookmarks_dialog(
    parent: &Window,
    fmstate: Rc<RefCell<FmState>>,
    sidebar_list: &StringList,
) {
    let dialog = Dialog::builder()
        .title("Manage Bookmarks")
        .transient_for(parent)
        .modal(true)
        .default_width(400)
        .default_height(300)
        .build();

    let content = dialog.content_area();

    let scrolled = ScrolledWindow::builder().vexpand(true).hexpand(true).build();

    let list_box = ListBox::new();
    list_box.set_selection_mode(gtk4::SelectionMode::None);

    // Populate with current bookmarks
    let bookmarks = fmstate.borrow().bookmarks.clone();
    for (index, bookmark) in bookmarks.iter().enumerate() {
        let row = create_bookmark_row(
            bookmark,
            index,
            fmstate.clone(),
            sidebar_list.clone(),
            dialog.clone(),
        );
        list_box.append(&row);
    }

    if bookmarks.is_empty() {
        let empty_label = Label::new(Some("No bookmarks yet"));
        empty_label.set_margin_top(20);
        empty_label.set_margin_bottom(20);
        empty_label.add_css_class("dim-label");
        list_box.append(&empty_label);
    }

    scrolled.set_child(Some(&list_box));
    content.append(&scrolled);

    dialog.add_button("Close", ResponseType::Close);

    dialog.connect_response(move |dialog, response| {
        if response == ResponseType::Close {
            dialog.close();
        }
    });

    dialog.present();
}

fn create_bookmark_row(
    bookmark: &Bookmark,
    index: usize,
    fmstate: Rc<RefCell<FmState>>,
    sidebar_list: StringList,
    dialog: Dialog,
) -> ListBoxRow {
    let row = ListBoxRow::new();

    let hbox = GtkBox::new(Orientation::Horizontal, 12);
    hbox.set_margin_start(12);
    hbox.set_margin_end(12);
    hbox.set_margin_top(6);
    hbox.set_margin_bottom(6);

    // Icon
    let icon = gtk4::Image::from_icon_name("starred");
    icon.set_pixel_size(24);

    // Labels box
    let vbox = GtkBox::new(Orientation::Vertical, 2);
    vbox.set_hexpand(true);

    let name_label = Label::new(Some(&bookmark.name));
    name_label.set_xalign(0.0);
    name_label.add_css_class("heading");

    let path_label = Label::new(Some(&bookmark.path));
    path_label.set_xalign(0.0);
    path_label.add_css_class("dim-label");
    path_label.set_ellipsize(gtk4::pango::EllipsizeMode::Middle);

    vbox.append(&name_label);
    vbox.append(&path_label);

    // Delete button
    let delete_button = Button::builder()
        .icon_name("user-trash-symbolic")
        .tooltip_text("Remove bookmark")
        .valign(gtk4::Align::Center)
        .build();

    delete_button.connect_clicked(glib::clone!(
        #[strong]
        fmstate,
        #[strong]
        sidebar_list,
        #[weak]
        dialog,
        move |_| {
            if let Err(e) = fmstate.borrow_mut().remove_bookmark(index) {
                eprintln!("Failed to remove bookmark: {}", e);
            } else {
                // Refresh sidebar
                crate::sidebar::refresh_sidebar(&sidebar_list, &fmstate);
                // Close and reopen dialog to refresh list
                dialog.close();
                dialog.destroy();
            }
        }
    ));

    hbox.append(&icon);
    hbox.append(&vbox);
    hbox.append(&delete_button);

    row.set_child(Some(&hbox));
    row
}
