use crate::{files_panel, state::FmState};
use gtk4::{
    Application, ApplicationWindow, Box as GtkBox, MenuButton,
    gio::{Menu, SimpleAction},
    glib,
    prelude::*,
};
use std::{cell::RefCell, rc::Rc};

pub fn build_headerbar() -> GtkBox {
    let headerbar = GtkBox::new(gtk4::Orientation::Horizontal, 6);

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
    menu.append_submenu(Some("View"), &view_submenu);

    // Menu button
    let menu_button = MenuButton::new();
    menu_button.set_menu_model(Some(&menu));

    headerbar.append(&menu_button);
    headerbar
}

pub fn implement_actions(
    window: &ApplicationWindow,
    app: &Application,
    fmstate: Rc<RefCell<FmState>>,
    files_list: &gtk4::StringList,
    sidebar_selection: &gtk4::SingleSelection,
) {
    // Show Hidden Files action
    let show_hidden_initial = fmstate.borrow().settings.show_hidden;
    let show_hidden_action =
        SimpleAction::new_stateful("show_hidden", None, &show_hidden_initial.into());

    show_hidden_action.connect_activate(glib::clone!(
        #[strong]
        fmstate,
        #[weak]
        files_list,
        move |action, _| {
            let current: bool = action.state().unwrap().get().unwrap();
            action.set_state(&(!current).into());

            let mut fmstate_mut = fmstate.borrow_mut();
            fmstate_mut.settings.show_hidden = !current;

            files_panel::populate_files_list(
                &files_list,
                &fmstate_mut.current_path,
                &fmstate_mut.settings.show_hidden,
            );
        }
    ));

    window.add_action(&show_hidden_action);

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
        files_list,
        #[weak]
        sidebar_selection,
        move |_, _| {
            let mut fmstate_mut = fmstate.borrow_mut();
            if let Some(file) = fmstate_mut.go_back_in_history() {
                files_panel::populate_files_list(
                    &files_list,
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
        files_list,
        #[weak]
        sidebar_selection,
        move |_, _| {
            let mut fmstate_mut = fmstate.borrow_mut();
            if let Some(file) = fmstate_mut.go_forward_in_history() {
                files_panel::populate_files_list(
                    &files_list,
                    &file,
                    &fmstate_mut.settings.show_hidden,
                );
                sidebar_selection.unselect_all();
            }
        }
    ));
    window.add_action(&redo_action);
}
