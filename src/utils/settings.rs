pub enum FileView {
    IconView,
    ListView,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortColumn {
    Name,
    Size,
    ModifiedDate,
    Type,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortOrder {
    Ascending,
    Descending,
}

pub struct FMSettings {
    pub show_hidden: bool,
    pub file_view: FileView,
    pub sort_column: SortColumn,
    pub sort_order: SortOrder,
    pub folders_first: bool,
}

impl FMSettings {
    pub fn new() -> Self {
        Self {
            show_hidden: false,
            file_view: FileView::IconView,
            sort_column: SortColumn::Name,
            sort_order: SortOrder::Ascending,
            folders_first: true,
        }
    }
}
