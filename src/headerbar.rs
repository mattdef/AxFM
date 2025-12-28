use crate::{files_panel, state::FmState};
use gtk4::{
    Application, ApplicationWindow, Button, HeaderBar, MenuButton,
    gio::{Menu, SimpleAction},
    glib,
    prelude::*,
};
use std::{cell::RefCell, rc::Rc};

pub fn build_headerbar(fmstate: Rc<RefCell<FmState>>) -> HeaderBar {
    let headerbar = HeaderBar::new();

    // Back button
    let back_button =
        Button::builder().icon_name("go-previous-symbolic").tooltip_text("Back").build();

    // Forward button
    let forward_button =
        Button::builder().icon_name("go-next-symbolic").tooltip_text("Forward").build();

    // Main menu model
    let menu = Menu::new();

    // "File" submenu
    let file_submenu = Menu::new();
    file_submenu.append(Some("New Window"), Some("win.open_new_window"));
    file_submenu.append(Some("Close Window"), Some("win.close_window"));
    menu.append_submenu(Some("File"), &file_submenu);

    // "Edit" submenu
    let edit_submenu = Menu::new();
    edit_submenu.append(Some("Undo"), Some("win.undo_history"));
    edit_submenu.append(Some("Redo"), Some("win.redo_history"));
    menu.append_submenu(Some("Edit"), &edit_submenu);

    // "View" submenu
    let view_submenu = Menu::new();
    view_submenu.append(Some("Show Hidden Files"), Some("win.show_hidden"));
    view_submenu.append(Some("Folders First"), Some("win.folders_first"));
    menu.append_submenu(Some("View"), &view_submenu);

    // Menu button
    let menu_button = MenuButton::new();
    menu_button.set_icon_name("open-menu-symbolic");
    menu_button.set_menu_model(Some(&menu));

    // Add widgets to headerbar
    headerbar.pack_start(&back_button);
    headerbar.pack_start(&forward_button);
    headerbar.pack_end(&menu_button);

    headerbar
}

pub fn implement_actions(
    window: &ApplicationWindow,
    app: &Application,
    fmstate: Rc<RefCell<FmState>>,
    file_store: &gtk4::gio::ListStore,
    sidebar_selection: &gtk4::SingleSelection,
    headerbar: &HeaderBar,
) {
    // Set initial window title
    let current_dir = fmstate.borrow().current_path.basename();
    if let Some(dir_name) = current_dir {
        window.set_title(Some(&dir_name.to_string_lossy()));
    } else {
        window.set_title(Some("Ax File Manager"));
    }
    // Show Hidden Files action
    let show_hidden_initial = fmstate.borrow().settings.show_hidden;
    let show_hidden_action =
        SimpleAction::new_stateful("show_hidden", None, &show_hidden_initial.into());

    show_hidden_action.connect_activate(glib::clone!(
        #[strong]
        fmstate,
        #[weak]
        file_store,
        move |action, _| {
            let current: bool = action.state().unwrap().get().unwrap();
            action.set_state(&(!current).into());

            let mut fmstate_mut = fmstate.borrow_mut();
            fmstate_mut.settings.show_hidden = !current;

            files_panel::populate_files_list(
                &file_store,
                &fmstate_mut.current_path,
                &fmstate_mut.settings.show_hidden,
            );
        }
    ));

    window.add_action(&show_hidden_action);

    // Folders First action
    let folders_first_initial = fmstate.borrow().settings.folders_first;
    let folders_first_action =
        SimpleAction::new_stateful("folders_first", None, &folders_first_initial.into());

    folders_first_action.connect_activate(glib::clone!(
        #[strong]
        fmstate,
        move |action, _| {
            let current: bool = action.state().unwrap().get().unwrap();
            action.set_state(&(!current).into());

            let mut fmstate_mut = fmstate.borrow_mut();
            fmstate_mut.settings.folders_first = !current;
            // Sorting will be implemented in Phase 3
        }
    ));

    window.add_action(&folders_first_action);

    let new_window_action = SimpleAction::new("open_new_window", None);
    new_window_action.connect_activate(glib::clone!(
        #[weak]
        app,
        move |_, _| {
            crate::build_fm(&app);
        }
    ));
    window.add_action(&new_window_action);

    let close_window_action = SimpleAction::new("close_window", None);
    close_window_action.connect_activate(glib::clone!(
        #[weak]
        window,
        move |_, _| {
            window.close();
        }
    ));
    window.add_action(&close_window_action);

    let undo_action = SimpleAction::new("undo_history", None);
    undo_action.connect_activate(glib::clone!(
        #[strong]
        fmstate,
        #[weak]
        file_store,
        #[weak]
        sidebar_selection,
        move |_, _| {
            let mut fmstate_mut = fmstate.borrow_mut();
            if let Some(file) = fmstate_mut.go_back_in_history() {
                files_panel::populate_files_list(
                    &file_store,
                    &file,
                    &fmstate_mut.settings.show_hidden,
                );
                sidebar_selection.unselect_all();
            }
        }
    ));
    window.add_action(&undo_action);

    let redo_action = SimpleAction::new("redo_history", None);
    redo_action.connect_activate(glib::clone!(
        #[strong]
        fmstate,
        #[weak]
        file_store,
        #[weak]
        sidebar_selection,
        move |_, _| {
            let mut fmstate_mut = fmstate.borrow_mut();
            if let Some(file) = fmstate_mut.go_forward_in_history() {
                files_panel::populate_files_list(
                    &file_store,
                    &file,
                    &fmstate_mut.settings.show_hidden,
                );
                sidebar_selection.unselect_all();
            }
        }
    ));
    window.add_action(&redo_action);

    // Update window title when path changes
    fmstate.borrow_mut().on_path_changed.push(Box::new(glib::clone!(
        #[weak]
        window,
        move |file| {
            if let Some(dir_name) = file.basename() {
                window.set_title(Some(&dir_name.to_string_lossy()));
            } else {
                window.set_title(Some("Ax File Manager"));
            }
        }
    )));

    // Connect back button from headerbar
    if let Some(back_button) = headerbar.first_child().and_downcast::<Button>() {
        back_button.connect_clicked(glib::clone!(
            #[strong]
            fmstate,
            #[weak]
            file_store,
            #[weak]
            sidebar_selection,
            move |_| {
                let mut fmstate_mut = fmstate.borrow_mut();
                if let Some(file) = fmstate_mut.go_back_in_history() {
                    files_panel::populate_files_list(
                        &file_store,
                        &file,
                        &fmstate_mut.settings.show_hidden,
                    );
                    sidebar_selection.unselect_all();
                }
            }
        ));
    }

    // Connect forward button from headerbar (second child)
    if let Some(back_button) = headerbar.first_child() {
        if let Some(forward_button) = back_button.next_sibling().and_downcast::<Button>() {
            forward_button.connect_clicked(glib::clone!(
                #[strong]
                fmstate,
                #[weak]
                file_store,
                #[weak]
                sidebar_selection,
                move |_| {
                    let mut fmstate_mut = fmstate.borrow_mut();
                    if let Some(file) = fmstate_mut.go_forward_in_history() {
                        files_panel::populate_files_list(
                            &file_store,
                            &file,
                            &fmstate_mut.settings.show_hidden,
                        );
                        sidebar_selection.unselect_all();
                    }
                }
            ));
        }
    }
}
