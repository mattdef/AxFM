use gtk4::{gio, glib, prelude::*, subclass::prelude::*};
use std::cell::RefCell;

mod imp {
    use super::*;

    #[derive(Debug, Default, glib::Properties)]
    #[properties(wrapper_type = super::FileItem)]
    pub struct FileItem {
        #[property(get, set)]
        path: RefCell<String>,
        #[property(get, set)]
        display_name: RefCell<String>,
        #[property(get, set)]
        size: RefCell<u64>,
        #[property(get, set)]
        modified: RefCell<i64>,
        #[property(get, set)]
        mime_type: RefCell<String>,
        #[property(get, set)]
        is_directory: RefCell<bool>,
        #[property(get, set)]
        icon: RefCell<Option<gio::Icon>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FileItem {
        const NAME: &'static str = "AxFMFileItem";
        type Type = super::FileItem;
    }

    #[glib::derived_properties]
    impl ObjectImpl for FileItem {}
}

glib::wrapper! {
    pub struct FileItem(ObjectSubclass<imp::FileItem>);
}

impl FileItem {
    pub fn new(
        path: String,
        display_name: String,
        size: u64,
        modified: i64,
        mime_type: String,
        is_directory: bool,
        icon: Option<gio::Icon>,
    ) -> Self {
        glib::Object::builder()
            .property("path", path)
            .property("display-name", display_name)
            .property("size", size)
            .property("modified", modified)
            .property("mime-type", mime_type)
            .property("is-directory", is_directory)
            .property("icon", icon)
            .build()
    }

    pub fn from_file(file: &gio::File, show_hidden: bool) -> Option<Self> {
        let info = file
            .query_info(
                "standard::*,time::*",
                gio::FileQueryInfoFlags::NONE,
                gio::Cancellable::NONE,
            )
            .ok()?;

        let display_name = info.display_name().to_string();

        // Filter hidden files
        if !show_hidden && display_name.starts_with('.') {
            return None;
        }

        let path =
            file.path().map(|p| p.display().to_string()).unwrap_or_else(|| file.uri().to_string());

        let size = info.size() as u64;

        let modified = info.modification_date_time().map(|dt| dt.to_unix()).unwrap_or(0);

        let mime_type = info
            .content_type()
            .map(|ct| gio::content_type_get_description(&ct).to_string())
            .unwrap_or_else(|| String::from("Unknown"));

        let is_directory = info.file_type() == gio::FileType::Directory;

        let icon = info.icon();

        Some(FileItem::new(path, display_name, size, modified, mime_type, is_directory, icon))
    }

    pub fn format_size(&self) -> String {
        if self.is_directory() {
            return String::from("--");
        }

        let size = self.size();
        const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
        let mut size_f = size as f64;
        let mut unit_idx = 0;

        while size_f >= 1024.0 && unit_idx < UNITS.len() - 1 {
            size_f /= 1024.0;
            unit_idx += 1;
        }

        if unit_idx == 0 {
            format!("{} {}", size, UNITS[unit_idx])
        } else {
            format!("{:.1} {}", size_f, UNITS[unit_idx])
        }
    }

    pub fn format_modified(&self) -> String {
        let timestamp = self.modified();
        if timestamp == 0 {
            return String::from("Unknown");
        }

        // For now, simple formatting - will use chrono in Phase 5
        let dt = glib::DateTime::from_unix_local(timestamp).ok();
        if let Some(dt) = dt {
            dt.format("%Y-%m-%d %H:%M")
                .unwrap_or_else(|_| glib::GString::from("Unknown"))
                .to_string()
        } else {
            String::from("Unknown")
        }
    }
}
