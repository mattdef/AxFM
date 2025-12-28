use gtk4::{Box as GtkBox, Label, Orientation, gio, prelude::*};
use std::path::Path;
use sysinfo::Disks;

pub struct FooterBarComponents {
    pub left_label: Label,
    pub center_label: Label,
    pub right_label: Label,
}

pub fn build_footer_bar() -> (GtkBox, FooterBarComponents) {
    let footer_bar = GtkBox::new(Orientation::Horizontal, 0);
    footer_bar.set_height_request(30);
    footer_bar.add_css_class("footer-bar");

    // Left label: Disk space
    let left_label = Label::new(Some(""));
    left_label.set_halign(gtk4::Align::Start);
    left_label.add_css_class("footer-label");

    // Center label: Item count or selection info
    let center_label = Label::new(Some(""));
    center_label.set_halign(gtk4::Align::Center);
    center_label.set_hexpand(true);
    center_label.add_css_class("footer-label");

    // Right label: Default application
    let right_label = Label::new(Some(""));
    right_label.set_halign(gtk4::Align::End);
    right_label.add_css_class("footer-label");

    footer_bar.append(&left_label);
    footer_bar.append(&center_label);
    footer_bar.append(&right_label);

    let components = FooterBarComponents { left_label, center_label, right_label };

    (footer_bar, components)
}

pub fn update_disk_space(label: &Label, path: &gio::File) {
    if let Some(path_buf) = path.path() {
        if let Some((available, total)) = get_disk_space(&path_buf) {
            let available_gb = available as f64 / 1_073_741_824.0; // Convert to GB
            let total_gb = total as f64 / 1_073_741_824.0;
            let used_percent = ((total - available) as f64 / total as f64 * 100.0) as u32;

            label.set_text(&format!(
                "{:.2} GB free of {:.2} GB ({}% used)",
                available_gb, total_gb, used_percent
            ));
        } else {
            label.set_text("N/A");
        }
    } else {
        // Remote URIs (trash:///, network paths, etc.)
        label.set_text("N/A");
    }
}

pub fn update_item_count(label: &Label, count: usize) {
    let text = if count == 1 { "1 item".to_string() } else { format!("{} items", count) };
    label.set_text(&text);
}

pub fn update_selection_info(label: &Label, file: &gio::File) {
    if let Ok(info) = file.query_info(
        "standard::size,standard::type,standard::content-type",
        gio::FileQueryInfoFlags::NONE,
        None::<&gio::Cancellable>,
    ) {
        let file_type = info.file_type();

        if file_type == gio::FileType::Directory {
            label.set_text("Directory");
        } else {
            let size = info.size();
            let size_str = format_size(size as u64);

            // Get file type description
            let type_desc = get_file_type_description(file);

            label.set_text(&format!("{} - {}", type_desc, size_str));
        }
    } else {
        label.set_text("");
    }
}

pub fn update_default_app(label: &Label, file: &gio::File) {
    // Only show default app for regular files, not directories
    if let Ok(info) = file.query_info(
        "standard::type,standard::content-type",
        gio::FileQueryInfoFlags::NONE,
        None::<&gio::Cancellable>,
    ) {
        let file_type = info.file_type();

        if file_type == gio::FileType::Regular {
            if let Some(app_name) = get_default_app(file) {
                label.set_text(&format!("Opens with: {}", app_name));
            } else {
                label.set_text("");
            }
        } else {
            label.set_text("");
        }
    } else {
        label.set_text("");
    }
}

pub fn count_items(dir: &gio::File, show_hidden: bool) -> usize {
    let mut count = 0;

    if let Ok(enumerator) = dir.enumerate_children(
        "standard::name",
        gio::FileQueryInfoFlags::NONE,
        None::<&gio::Cancellable>,
    ) {
        while let Some(info) = enumerator.next_file(None::<&gio::Cancellable>).unwrap_or(None) {
            let name = info.name();
            let name_str = name.to_string_lossy();

            if !show_hidden && name_str.starts_with('.') {
                continue;
            }

            count += 1;
        }
    }

    count
}

fn get_disk_space(path: &Path) -> Option<(u64, u64)> {
    let disks = Disks::new_with_refreshed_list();

    for disk in disks.list() {
        if path.starts_with(disk.mount_point()) {
            return Some((disk.available_space(), disk.total_space()));
        }
    }

    None
}

pub fn get_file_type_description(file: &gio::File) -> String {
    // Try to get MIME type from gio first
    if let Ok(info) = file.query_info(
        "standard::content-type",
        gio::FileQueryInfoFlags::NONE,
        None::<&gio::Cancellable>,
    ) {
        if let Some(content_type) = info.content_type() {
            let content_str = content_type.to_string();

            // Convert MIME type to friendly name
            return match content_str.as_str() {
                "application/pdf" => "PDF Document".to_string(),
                "text/plain" => "Text File".to_string(),
                "text/html" => "HTML Document".to_string(),
                "image/jpeg" | "image/jpg" => "JPEG Image".to_string(),
                "image/png" => "PNG Image".to_string(),
                "image/gif" => "GIF Image".to_string(),
                "image/svg+xml" => "SVG Image".to_string(),
                "video/mp4" => "MP4 Video".to_string(),
                "video/x-matroska" => "MKV Video".to_string(),
                "audio/mpeg" => "MP3 Audio".to_string(),
                "audio/ogg" => "OGG Audio".to_string(),
                "application/zip" => "ZIP Archive".to_string(),
                "application/x-tar" => "TAR Archive".to_string(),
                "application/gzip" => "GZIP Archive".to_string(),
                _ => {
                    // Extract main type (e.g., "image" from "image/png")
                    if let Some(main_type) = content_str.split('/').next() {
                        format!("{} file", main_type.to_uppercase())
                    } else {
                        content_str
                    }
                }
            };
        }
    }

    // Fallback to mime_guess if gio doesn't work
    if let Some(path) = file.path() {
        if let Some(mime) = mime_guess::from_path(&path).first() {
            let mime_str = mime.essence_str();
            return match mime_str {
                "application/pdf" => "PDF Document".to_string(),
                "text/plain" => "Text File".to_string(),
                _ => mime_str.to_string(),
            };
        }
    }

    "Unknown type".to_string()
}

pub fn get_default_app(file: &gio::File) -> Option<String> {
    if let Ok(info) = file.query_info(
        "standard::content-type",
        gio::FileQueryInfoFlags::NONE,
        None::<&gio::Cancellable>,
    ) {
        if let Some(content_type) = info.content_type() {
            if let Some(app) = gio::AppInfo::default_for_type(&content_type, false) {
                return Some(app.name().to_string());
            }
        }
    }

    None
}

pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else if bytes == 1 {
        "1 byte".to_string()
    } else {
        format!("{} bytes", bytes)
    }
}
