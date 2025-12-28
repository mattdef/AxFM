use crate::models::file_item::FileItem;
use gtk4::{CustomSorter, Ordering, glib, prelude::*};
use std::cmp::Ordering as StdOrdering;

fn compare_with_folders_first(
    item1: &FileItem,
    item2: &FileItem,
    folders_first: bool,
    compare_fn: impl FnOnce(&FileItem, &FileItem) -> StdOrdering,
) -> StdOrdering {
    if folders_first {
        let is_dir1 = item1.is_directory();
        let is_dir2 = item2.is_directory();

        if is_dir1 && !is_dir2 {
            return StdOrdering::Less;
        } else if !is_dir1 && is_dir2 {
            return StdOrdering::Greater;
        }
    }

    compare_fn(item1, item2)
}

pub fn create_name_sorter(folders_first: bool) -> CustomSorter {
    CustomSorter::new(move |obj1, obj2| {
        let item1 = obj1.downcast_ref::<FileItem>().unwrap();
        let item2 = obj2.downcast_ref::<FileItem>().unwrap();

        let ordering = compare_with_folders_first(item1, item2, folders_first, |i1, i2| {
            i1.display_name().to_lowercase().cmp(&i2.display_name().to_lowercase())
        });

        match ordering {
            StdOrdering::Less => Ordering::Smaller,
            StdOrdering::Equal => Ordering::Equal,
            StdOrdering::Greater => Ordering::Larger,
        }
    })
}

pub fn create_size_sorter(folders_first: bool) -> CustomSorter {
    CustomSorter::new(move |obj1, obj2| {
        let item1 = obj1.downcast_ref::<FileItem>().unwrap();
        let item2 = obj2.downcast_ref::<FileItem>().unwrap();

        let ordering = compare_with_folders_first(item1, item2, folders_first, |i1, i2| {
            i1.size().cmp(&i2.size())
        });

        match ordering {
            StdOrdering::Less => Ordering::Smaller,
            StdOrdering::Equal => Ordering::Equal,
            StdOrdering::Greater => Ordering::Larger,
        }
    })
}

pub fn create_date_sorter(folders_first: bool) -> CustomSorter {
    CustomSorter::new(move |obj1, obj2| {
        let item1 = obj1.downcast_ref::<FileItem>().unwrap();
        let item2 = obj2.downcast_ref::<FileItem>().unwrap();

        let ordering = compare_with_folders_first(item1, item2, folders_first, |i1, i2| {
            i1.modified().cmp(&i2.modified())
        });

        match ordering {
            StdOrdering::Less => Ordering::Smaller,
            StdOrdering::Equal => Ordering::Equal,
            StdOrdering::Greater => Ordering::Larger,
        }
    })
}

pub fn create_type_sorter(folders_first: bool) -> CustomSorter {
    CustomSorter::new(move |obj1, obj2| {
        let item1 = obj1.downcast_ref::<FileItem>().unwrap();
        let item2 = obj2.downcast_ref::<FileItem>().unwrap();

        let ordering = compare_with_folders_first(item1, item2, folders_first, |i1, i2| {
            i1.mime_type().cmp(&i2.mime_type())
        });

        match ordering {
            StdOrdering::Less => Ordering::Smaller,
            StdOrdering::Equal => Ordering::Equal,
            StdOrdering::Greater => Ordering::Larger,
        }
    })
}
