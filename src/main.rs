mod files_panel;
mod footer_bar;
mod headerbar;
mod models;
mod pathbar;
mod popup_menu;
mod properties_dialog;
mod sidebar;
mod sorters;
mod state;
mod style;
mod utils;

use crate::models::file_item::FileItem;
use gtk4::{
    Application, ApplicationWindow, Box as GtkBox, GestureClick, Orientation, Paned, gio, glib,
    prelude::*,
};
use std::{cell::RefCell, rc::Rc};

const APP_ID: &str = "org.filemanager.axfm";

fn main() {
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_fm);
    app.run();
}

fn build_fm(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Ax File Manager")
        .default_width(800)
        .default_height(500)
        .build();

    style::load_css();

    // where files will be shown
    let content_area = GtkBox::new(Orientation::Vertical, 0);

    let home_path = gio::File::for_path(glib::home_dir());
    let fmstate = Rc::new(RefCell::new(state::FmState::new(home_path.clone())));

    let (files_scroll, file_store, column_view, files_selection) =
        files_panel::build_files_panel(fmstate.clone());
    let (sidebar_box, sidebar_selection) = sidebar::build_sidebar(fmstate.clone(), &file_store);
    let path_bar = pathbar::build_pathbar(&mut fmstate.borrow_mut());

    // Build and set headerbar at window level
    let headerbar = headerbar::build_headerbar(fmstate.clone());
    window.set_titlebar(Some(&headerbar));

    // right click menus
    let empty_area_menu =
        popup_menu::get_empty_right_click(&content_area, fmstate.clone(), &file_store);
    let file_area_menu =
        popup_menu::get_file_right_click(&content_area, fmstate.clone(), &file_store, &column_view);

    // implement all actions for the headerbar
    headerbar::implement_actions(
        &window,
        &app,
        fmstate.clone(),
        &file_store,
        &sidebar_selection,
        &headerbar,
    );

    files_panel::populate_files_list(
        &file_store,
        &home_path,
        &fmstate.borrow().settings.show_hidden,
    );

    sidebar_selection.connect_selected_notify(glib::clone!(
        #[weak]
        file_store,
        #[strong]
        fmstate,
        move |sel| {
            let idx = sel.selected();
            if idx == gtk4::INVALID_LIST_POSITION {
                return;
            }

            let sidebar_items = sidebar::get_sidebar_items();

            if let Some((_, file)) = sidebar_items.get(idx as usize) {
                let mut fmstate_mut = fmstate.borrow_mut();

                files_panel::populate_files_list(
                    &file_store,
                    &file,
                    &fmstate_mut.settings.show_hidden,
                );
                fmstate_mut.set_path(file.clone());
                fmstate_mut.update_history(file.clone());
            }
        }
    ));

    path_bar.connect_activate(glib::clone!(
        #[weak]
        file_store,
        #[weak]
        sidebar_selection,
        #[strong]
        fmstate,
        move |widget| {
            let text = widget.text();

            let file = if std::path::Path::new(&text).exists() {
                gio::File::for_path(&text)
            } else {
                gio::File::for_uri(&text)
            };

            files_panel::populate_files_list(
                &file_store,
                &file,
                &fmstate.borrow().settings.show_hidden,
            );

            let mut fmstate_mut = fmstate.borrow_mut();

            fmstate_mut.set_path(file.clone());
            fmstate_mut.update_history(file);
            sidebar_selection.unselect_all();
        }
    ));

    column_view.connect_activate(glib::clone!(
        #[strong]
        fmstate,
        #[weak]
        file_store,
        #[weak]
        sidebar_selection,
        move |cv, position| {
            if let Some(obj) = cv.model().and_then(|m| m.item(position)) {
                if let Some(file_item) = obj.downcast_ref::<FileItem>() {
                    let file_path = file_item.path();

                    // Try local path first, otherwise fallback to URI
                    let file = if std::path::Path::new(&file_path).exists() {
                        gio::File::for_path(&file_path)
                    } else {
                        gio::File::for_uri(&file_path)
                    };

                    if file_item.is_directory() {
                        files_panel::populate_files_list(
                            &file_store,
                            &file,
                            &fmstate.borrow().settings.show_hidden,
                        );

                        let mut fmstate_mut = fmstate.borrow_mut();

                        fmstate_mut.set_path(file.clone());
                        fmstate_mut.update_history(file.clone());
                        sidebar_selection.unselect_all();
                    }
                }
            }
        }
    ));

    // controllers
    let right_click = GestureClick::new();
    right_click.set_button(3);

    right_click.connect_released(glib::clone!(
        #[strong]
        fmstate,
        #[weak]
        empty_area_menu,
        #[weak]
        file_area_menu,
        move |_, _, x, y| {
            let click_rect = gtk4::gdk::Rectangle::new(x as i32, y as i32, 1, 1);

            let mut fmstate_mut = fmstate.borrow_mut();
            let hovered_file_opt = fmstate_mut.hovered_file.clone();

            if let Some(file_name) = hovered_file_opt {
                fmstate_mut.popup_focused_file = Some(file_name);

                // immedeatly drop to prevent issues
                // trust me, it will panic without this.
                drop(fmstate_mut);

                file_area_menu.set_pointing_to(Some(&click_rect));
                file_area_menu.popup();

                file_area_menu.connect_closed(glib::clone!(
                    #[strong]
                    fmstate,
                    move |_| {
                        fmstate.borrow_mut().popup_focused_file = None;
                    }
                ));
            } else {
                empty_area_menu.set_pointing_to(Some(&click_rect));
                empty_area_menu.popup();
            }
        }
    ));

    // content area
    content_area.append(&path_bar);
    content_area.append(&files_scroll);

    // setup controllers
    content_area.add_controller(right_click);

    let paned = Paned::new(Orientation::Horizontal);
    paned.set_start_child(Some(&sidebar_box));
    paned.set_end_child(Some(&content_area));
    paned.set_position(200);
    paned.set_wide_handle(true);
    paned.set_resize_start_child(false);
    paned.set_shrink_start_child(false);

    // Build footer bar
    let (footer_bar, footer_components) = footer_bar::build_footer_bar();

    // Create main vertical box to hold paned and footer
    let main_vbox = GtkBox::new(Orientation::Vertical, 0);
    main_vbox.append(&paned);
    main_vbox.append(&footer_bar);

    // Store labels in local variables for cloning
    let left_label = footer_components.left_label.clone();
    let center_label = footer_components.center_label.clone();
    let right_label = footer_components.right_label.clone();
    let file_store_path = file_store.clone();

    // Connect footer updates for path changes
    fmstate.borrow_mut().connect_path_changed(glib::clone!(
        #[weak]
        left_label,
        #[weak]
        center_label,
        #[weak]
        right_label,
        #[weak]
        file_store_path,
        move |new_path| {
            // Update disk space
            footer_bar::update_disk_space(&left_label, new_path);

            // Update item count based on what's actually displayed in the list
            let count = file_store_path.n_items() as usize;
            footer_bar::update_item_count(&center_label, count);

            // Clear selection info and default app
            right_label.set_text("");
        }
    ));

    // Clone labels again for selection callback
    let center_label_sel = footer_components.center_label.clone();
    let right_label_sel = footer_components.right_label.clone();
    let file_store_sel = file_store.clone();

    // Connect footer updates for selection changes
    files_selection.connect_selected_notify(glib::clone!(
        #[weak]
        center_label_sel,
        #[weak]
        right_label_sel,
        #[weak]
        file_store_sel,
        move |sel| {
            let idx = sel.selected();

            if idx == gtk4::INVALID_LIST_POSITION {
                // No selection - show item count based on displayed items
                let count = file_store_sel.n_items() as usize;
                footer_bar::update_item_count(&center_label_sel, count);
                right_label_sel.set_text("");
            } else {
                // Selection - show file info
                if let Some(obj) = file_store_sel.item(idx) {
                    if let Some(file_item) = obj.downcast_ref::<FileItem>() {
                        let file_path = file_item.path();

                        let file = if std::path::Path::new(&file_path).exists() {
                            gio::File::for_path(&file_path)
                        } else {
                            gio::File::for_uri(&file_path)
                        };

                        footer_bar::update_selection_info(&center_label_sel, &file);
                        footer_bar::update_default_app(&right_label_sel, &file);
                    }
                }
            }
        }
    ));

    // Initialize footer with current state
    let current_path = fmstate.borrow().current_path.clone();
    footer_bar::update_disk_space(&footer_components.left_label, &current_path);
    let count = footer_bar::count_items(&current_path, fmstate.borrow().settings.show_hidden);
    footer_bar::update_item_count(&footer_components.center_label, count);

    window.set_child(Some(&main_vbox));
    window.present();
}
