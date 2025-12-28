use crate::{glib::UserDirectory, state::FmState};
use gtk4::{
    Box as GtkBox, ListView, Orientation, ScrolledWindow, SignalListItemFactory, SingleSelection,
    StringList, gdk, gio, glib, prelude::*,
};
use std::{cell::RefCell, rc::Rc};

pub fn build_sidebar(
    fmstate: Rc<RefCell<FmState>>,
    file_store: &gtk4::gio::ListStore,
) -> (GtkBox, SingleSelection, StringList) {
    let sidebar_list = StringList::new(&[]);
    let sidebar_selection = SingleSelection::new(Some(sidebar_list.clone()));
    sidebar_selection.set_can_unselect(true);
    sidebar_selection.set_autoselect(false);

    // Initial population
    refresh_sidebar(&sidebar_list, &fmstate);

    let factory = SignalListItemFactory::new();
    factory.connect_setup(glib::clone!(
        #[weak]
        file_store,
        #[strong]
        fmstate,
        move |_, item| {
            let hbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);

            let icon = gtk4::Image::new();
            icon.set_pixel_size(24);

            let label = gtk4::Label::new(None);
            label.set_xalign(0.0);

            hbox.append(&icon);
            hbox.append(&label);

            hbox.set_margin_start(6);
            hbox.set_margin_end(6);
            hbox.set_margin_top(4);
            hbox.set_margin_bottom(4);

            // add drop target
            let drop_target = gtk4::DropTarget::new(String::static_type(), gdk::DragAction::COPY);
            drop_target.connect_drop(glib::clone!(
                #[weak_allow_none]
                label,
                #[weak_allow_none]
                file_store,
                #[strong]
                fmstate,
                move |_drop_target, value, _, _| {
                    if let Some(label) = label.as_ref() {
                        let label_text = label.text();

                        let sidebar_items = get_sidebar_items();
                        if let Some((_, target_path)) =
                            sidebar_items.iter().find(|(n, _)| **n == label_text)
                        {
                            if let Ok(uri) = value.get::<glib::GString>() {
                                let src_file = gio::File::for_uri(&uri);
                                let src_filename =
                                    src_file.basename().unwrap_or_else(|| "unknown".into());
                                let dest_file =
                                    target_path.child(src_filename.to_str().unwrap_or("unknown"));

                                match src_file.move_(
                                    &dest_file,
                                    gio::FileCopyFlags::OVERWRITE,
                                    None::<&gio::Cancellable>,
                                    None::<&mut dyn FnMut(i64, i64)>,
                                ) {
                                    Ok(_) => {
                                        if let Some(file_store) = &file_store {
                                            let fmstate_ref = fmstate.borrow();
                                            crate::files_panel::populate_files_list(
                                                file_store,
                                                &fmstate_ref.current_path,
                                                &fmstate_ref.settings.show_hidden,
                                            );
                                        }
                                    }
                                    Err(e) => eprintln!("Error while moving file: {}", e),
                                }
                            }
                        }
                    }
                    true
                }
            ));
            hbox.add_controller(drop_target);

            item.set_child(Some(&hbox));
        }
    ));

    factory.connect_bind(glib::clone!(
        #[strong]
        fmstate,
        move |_, item| {
            let hbox = item.child().and_downcast::<gtk4::Box>().unwrap();
            let icon = hbox.first_child().and_downcast::<gtk4::Image>().unwrap();
            let label = hbox.last_child().and_downcast::<gtk4::Label>().unwrap();

            let obj = item.item().unwrap().downcast::<gtk4::StringObject>().unwrap();
            let label_text = obj.string();
            label.set_text(&label_text);

            // Check if it's a heading
            if label_text == "Places" || label_text == "Bookmarks" {
                label.remove_css_class("sidebar-item");
                label.add_css_class("sidebar-heading");
                icon.set_visible(false);
                label.set_tooltip_text(None);
                return;
            }

            // Regular item
            label.remove_css_class("sidebar-heading");
            label.add_css_class("sidebar-item");
            icon.set_visible(true);

            // Get sidebar items
            let sidebar_items = get_sidebar_items();

            if let Some((name, file)) = sidebar_items.iter().find(|(n, _)| *n == label_text) {
                let tooltip = file
                    .path()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| file.uri().to_string());
                label.set_tooltip_text(Some(&tooltip));

                let icon_name = match *name {
                    "Home" => "user-home",
                    "Documents" => "folder-documents",
                    "Downloads" => "folder-download",
                    "Music" => "folder-music",
                    "Pictures" => "folder-pictures",
                    "Videos" => "folder-videos",
                    "Trash" => "user-trash",
                    _ => "folder",
                };
                icon.set_icon_name(Some(icon_name));
            } else {
                // It's a bookmark
                icon.set_icon_name(Some("starred"));

                // Find the bookmark to get its path for tooltip
                let bookmarks = fmstate.borrow().bookmarks.clone();
                if let Some(bookmark) = bookmarks.iter().find(|b| b.name == label_text) {
                    label.set_tooltip_text(Some(&bookmark.path));
                } else {
                    label.set_tooltip_text(None);
                }
            }
        }
    ));

    let list_view = ListView::new(Some(sidebar_selection.clone()), Some(factory));
    let scroll =
        ScrolledWindow::builder().child(&list_view).min_content_width(180).vexpand(true).build();

    // Building Sidebar
    let sidebar_box = GtkBox::new(Orientation::Vertical, 0);
    sidebar_box.set_hexpand(false);
    sidebar_box.set_width_request(180);

    let heading_box = GtkBox::new(Orientation::Horizontal, 0);
    let heading = gtk4::Label::new(Some("Places"));
    heading.add_css_class("sidebar-heading");
    heading.set_margin_top(6);
    heading.set_margin_bottom(6);
    heading.set_margin_start(12);
    heading.set_margin_end(12);
    heading.set_xalign(0.0);

    heading_box.append(&heading);

    sidebar_box.append(&scroll);

    (sidebar_box, sidebar_selection, sidebar_list)
}

pub fn get_sidebar_items() -> Vec<(&'static str, gio::File)> {
    let home = gio::File::for_path(glib::home_dir());
    let dirs = |d: UserDirectory| {
        let path = glib::user_special_dir(d).unwrap();
        gio::File::for_path(path)
    };

    vec![
        ("Home", home.clone()),
        ("Documents", dirs(UserDirectory::Documents)),
        ("Downloads", dirs(UserDirectory::Downloads)),
        ("Music", dirs(UserDirectory::Music)),
        ("Pictures", dirs(UserDirectory::Pictures)),
        ("Videos", dirs(UserDirectory::Videos)),
        ("Trash", gio::File::for_uri("trash:///")),
    ]
}

pub fn refresh_sidebar(sidebar_list: &StringList, fmstate: &Rc<RefCell<FmState>>) {
    // Clear existing items
    sidebar_list.splice(0, sidebar_list.n_items(), &[]);

    // Add Places section
    sidebar_list.append("Places");
    let places = get_sidebar_items();
    for (name, _) in places.iter() {
        sidebar_list.append(name);
    }

    // Add Bookmarks section
    sidebar_list.append("Bookmarks");
    let bookmarks = fmstate.borrow().bookmarks.clone();
    for bookmark in bookmarks.iter() {
        sidebar_list.append(&bookmark.name);
    }
}
