use crate::{models::file_item::FileItem, sorters, state::FmState, utils::WidgetDataExt};
use gtk4::{
    ColumnView, ColumnViewColumn, DragIcon, DragSource, EventControllerMotion, ScrolledWindow,
    SignalListItemFactory, SingleSelection, SortListModel, gdk, gio, gio::ThemedIcon, glib,
    prelude::*,
};
use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
};

pub fn build_files_panel(
    fmstate: Rc<RefCell<FmState>>,
) -> (ScrolledWindow, gio::ListStore, ColumnView, SingleSelection) {
    let file_store = gio::ListStore::new::<FileItem>();

    // Create sorter based on settings
    let folders_first = fmstate.borrow().settings.folders_first;
    let sorter = sorters::create_name_sorter(folders_first);

    let sort_model = SortListModel::new(Some(file_store.clone()), Some(sorter.clone()));
    let selection_model = SingleSelection::new(Some(sort_model.clone()));

    let column_view = ColumnView::new(Some(selection_model.clone()));

    // Name Column
    let name_factory = create_name_column_factory(fmstate.clone());
    let name_column = ColumnViewColumn::new(Some("Name"), Some(name_factory));
    name_column.set_expand(true);
    name_column.set_sorter(Some(&sorters::create_name_sorter(folders_first)));
    column_view.append_column(&name_column);

    // Size Column
    let size_factory = create_size_column_factory();
    let size_column = ColumnViewColumn::new(Some("Size"), Some(size_factory));
    size_column.set_fixed_width(100);
    size_column.set_sorter(Some(&sorters::create_size_sorter(folders_first)));
    column_view.append_column(&size_column);

    // Modified Column
    let modified_factory = create_modified_column_factory();
    let modified_column = ColumnViewColumn::new(Some("Modified"), Some(modified_factory));
    modified_column.set_fixed_width(150);
    modified_column.set_sorter(Some(&sorters::create_date_sorter(folders_first)));
    column_view.append_column(&modified_column);

    // Type Column
    let type_factory = create_type_column_factory();
    let type_column = ColumnViewColumn::new(Some("Type"), Some(type_factory));
    type_column.set_fixed_width(120);
    type_column.set_sorter(Some(&sorters::create_type_sorter(folders_first)));
    column_view.append_column(&type_column);

    // Connect the column view sorter to the sort model
    column_view.sorter().unwrap().connect_changed(glib::clone!(
        #[weak]
        sort_model,
        move |sorter, _| {
            sort_model.set_sorter(Some(sorter));
        }
    ));

    let scroll = ScrolledWindow::builder().child(&column_view).vexpand(true).hexpand(true).build();

    (scroll, file_store, column_view, selection_model)
}

fn create_name_column_factory(fmstate: Rc<RefCell<FmState>>) -> SignalListItemFactory {
    let factory = SignalListItemFactory::new();

    factory.connect_setup(glib::clone!(
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
            item.set_child(Some(&hbox));

            // Setup hover detection
            let motion = EventControllerMotion::new();

            motion.connect_enter(glib::clone!(
                #[strong]
                fmstate,
                #[weak]
                item,
                move |_, _, _| {
                    if let Some(obj) = item.item() {
                        if let Some(file_item) = obj.downcast_ref::<FileItem>() {
                            if let Ok(mut fmstate_mut) = fmstate.try_borrow_mut() {
                                fmstate_mut.hovered_file = Some(file_item.path().into());
                            }
                        }
                    }
                }
            ));

            motion.connect_leave(glib::clone!(
                #[strong]
                fmstate,
                move |_| {
                    if let Ok(mut fmstate_mut) = fmstate.try_borrow_mut() {
                        fmstate_mut.hovered_file = None;
                    }
                }
            ));

            hbox.add_controller(motion);

            // Setup drag
            let drag_source = DragSource::new();
            drag_source.set_actions(gdk::DragAction::COPY);

            drag_source.connect_prepare(glib::clone!(
                #[strong]
                fmstate,
                move |_, _, _| {
                    if let Some(file) = &fmstate.borrow().hovered_file {
                        let uri = gtk4::gio::File::for_path(file).uri();
                        Some(gdk::ContentProvider::for_value(&uri.to_value()))
                    } else {
                        None
                    }
                }
            ));

            drag_source.connect_drag_begin(glib::clone!(
                #[weak]
                icon,
                move |_, drag| {
                    if let Some(gicon) = icon.gicon() {
                        let paintable = gtk4::IconTheme::default().lookup_by_gicon(
                            &gicon,
                            24,
                            1,
                            gtk4::TextDirection::None,
                            gtk4::IconLookupFlags::empty(),
                        );
                        DragIcon::set_from_paintable(drag, &paintable, 0, 0);
                    } else {
                        let icon_theme = gtk4::IconTheme::default();
                        let icon = icon_theme.lookup_by_gicon(
                            &ThemedIcon::new("text-x-generic"),
                            24,
                            1,
                            gtk4::TextDirection::None,
                            gtk4::IconLookupFlags::empty(),
                        );
                        DragIcon::set_from_paintable(drag, &icon, 0, 0);
                    }
                }
            ));

            hbox.add_controller(drag_source);
        }
    ));

    factory.connect_bind(glib::clone!(
        #[strong]
        fmstate,
        move |_, item| {
            let hbox = item.child().and_downcast::<gtk4::Box>().unwrap();
            let icon = hbox.first_child().and_downcast::<gtk4::Image>().unwrap();
            let label = hbox.last_child().and_downcast::<gtk4::Label>().unwrap();

            if let Some(obj) = item.item() {
                if let Some(file_item) = obj.downcast_ref::<FileItem>() {
                    label.set_text(&file_item.display_name());

                    if let Some(icon_gio) = file_item.icon() {
                        icon.set_from_gicon(&icon_gio);
                    } else {
                        icon.set_icon_name(Some("gtk-missing-image"));
                    }

                    // Add drop target for directories
                    let is_dir = file_item.is_directory();
                    let file_path = file_item.path();

                    hbox.set_typed_data("file-path", glib::GString::from(file_path.clone()));
                    hbox.set_flag("is-dir", is_dir);
                    hbox.track_widget_cleanup();

                    if is_dir {
                        let drop_target =
                            gtk4::DropTarget::new(String::static_type(), gdk::DragAction::COPY);

                        drop_target.connect_drop(glib::clone!(
                            #[strong]
                            fmstate,
                            move |drop_target, value, _, _| {
                                if let Some(target_widget) = drop_target.widget() {
                                    if let Some(target_path) =
                                        target_widget.get_typed_data::<glib::GString>("file-path")
                                    {
                                        if let Ok(uri) = value.get::<glib::GString>() {
                                            let src_file = gio::File::for_uri(&uri);
                                            let mut dest_path = PathBuf::from(&target_path);

                                            let src_filename = src_file
                                                .basename()
                                                .unwrap_or_else(|| "unknown".into());

                                            dest_path.push(src_filename);
                                            let dest_file = gtk4::gio::File::for_path(dest_path);

                                            match src_file.move_(
                                                &dest_file,
                                                gio::FileCopyFlags::OVERWRITE,
                                                None::<&gio::Cancellable>,
                                                None::<&mut dyn FnMut(i64, i64)>,
                                            ) {
                                                Ok(_) => {
                                                    // File moved successfully
                                                    // Repopulation will be handled by caller
                                                }
                                                Err(e) => {
                                                    eprintln!("Error while moving file: {}", e)
                                                }
                                            }
                                        }
                                    }
                                }

                                true
                            }
                        ));

                        hbox.add_controller(drop_target);
                    }
                }
            }
        }
    ));

    factory
}

fn create_size_column_factory() -> SignalListItemFactory {
    let factory = SignalListItemFactory::new();

    factory.connect_setup(|_, item| {
        let label = gtk4::Label::new(None);
        label.set_xalign(1.0); // Right align
        item.set_child(Some(&label));
    });

    factory.connect_bind(|_, item| {
        let label = item.child().and_downcast::<gtk4::Label>().unwrap();

        if let Some(obj) = item.item() {
            if let Some(file_item) = obj.downcast_ref::<FileItem>() {
                label.set_text(&file_item.format_size());
            }
        }
    });

    factory
}

fn create_modified_column_factory() -> SignalListItemFactory {
    let factory = SignalListItemFactory::new();

    factory.connect_setup(|_, item| {
        let label = gtk4::Label::new(None);
        label.set_xalign(0.0);
        item.set_child(Some(&label));
    });

    factory.connect_bind(|_, item| {
        let label = item.child().and_downcast::<gtk4::Label>().unwrap();

        if let Some(obj) = item.item() {
            if let Some(file_item) = obj.downcast_ref::<FileItem>() {
                label.set_text(&file_item.format_modified());
            }
        }
    });

    factory
}

fn create_type_column_factory() -> SignalListItemFactory {
    let factory = SignalListItemFactory::new();

    factory.connect_setup(|_, item| {
        let label = gtk4::Label::new(None);
        label.set_xalign(0.0);
        item.set_child(Some(&label));
    });

    factory.connect_bind(|_, item| {
        let label = item.child().and_downcast::<gtk4::Label>().unwrap();

        if let Some(obj) = item.item() {
            if let Some(file_item) = obj.downcast_ref::<FileItem>() {
                label.set_text(&file_item.mime_type());
            }
        }
    });

    factory
}

pub fn populate_files_list(file_store: &gio::ListStore, dir: &gio::File, show_hidden: &bool) {
    file_store.remove_all();

    if let Ok(enumerator) =
        dir.enumerate_children("*", gio::FileQueryInfoFlags::NONE, None::<&gio::Cancellable>)
    {
        while let Some(_info) = enumerator.next_file(None::<&gio::Cancellable>).unwrap_or(None) {
            let name = _info.display_name();

            if !show_hidden && name.starts_with('.') {
                continue;
            }

            let child_file = dir.child(&name);

            if let Some(file_item) = FileItem::from_file(&child_file, *show_hidden) {
                file_store.append(&file_item);
            }
        }
    }
}
