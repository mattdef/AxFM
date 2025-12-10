use crate::utils::FMSettings;
use gtk4::{gio, glib::GString};
use std::path::PathBuf;

pub struct FmState {
    pub current_path: gio::File,
    pub on_path_changed: Vec<Box<dyn Fn(&gio::File)>>,
    pub settings: FMSettings,
    pub hovered_file: Option<GString>,
    pub popup_focused_file: Option<GString>,
    pub clipboard: Vec<PathBuf>,
    pub clipboard_is_cut: bool,
    pub history: Vec<gio::File>,
    pub history_index: usize,
}

impl FmState {
    pub fn new(current_path: gio::File) -> Self {
        let mut history = Vec::new();
        history.push(current_path.clone());

        Self {
            current_path,
            on_path_changed: Vec::new(),
            settings: FMSettings::new(),
            hovered_file: None,
            popup_focused_file: None,
            clipboard: Vec::new(),
            clipboard_is_cut: false,
            history,
            history_index: 0,
        }
    }

    pub fn set_path(&mut self, new_path: gio::File) {
        self.current_path = new_path.clone();
        for cb in self.on_path_changed.iter() {
            cb(&new_path);
        }
    }

    pub fn connect_path_changed<F: Fn(&gio::File) + 'static>(&mut self, f: F) {
        self.on_path_changed.push(Box::new(f));
    }

    pub fn update_history(&mut self, file: gio::File) {
        if self.history_index + 1 < self.history.len() {
            self.history.truncate(self.history_index + 1);
        }

        self.history.push(file);
        self.history_index = self.history.len() - 1;
    }

    pub fn go_back_in_history(&mut self) -> Option<gio::File> {
        if self.history_index == 0 {
            return None;
        }

        self.history_index -= 1;
        let file = self.history[self.history_index].clone();
        self.set_path(file.clone());
        Some(file)
    }

    pub fn go_forward_in_history(&mut self) -> Option<gio::File> {
        if self.history_index + 1 >= self.history.len() {
            return None;
        }

        self.history_index += 1;
        let file = self.history[self.history_index].clone();
        self.set_path(file.clone());
        Some(file)
    }
}
