//! [`PresetLabels`]: every string a preset browser shows or announces, for the host to translate.

use crate::list::TreeText;

/// The words of a `PresetBrowser`, its toolbar, rename bar, context menu
/// and `PresetFooter`. Defaults are English; menu entries are in title
/// case, text only a screen reader reads in sentence case. Panels name their own things ("Rename
/// Brush…", "New Tool Preset") by overriding the fields. `{n}` stands for a number and `{name}` for
/// a preset's or group's name.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct PresetLabels {
    /// Grid view's entry in the panel menu and name of the toolbar's grid button: "Grid View".
    pub grid_view: String,
    /// List view's entry and button: "List View".
    pub list_view: String,
    /// The name of the thumbnail size slider.
    pub thumb_size: String,
    /// The name of the footer toolbar.
    pub footer: String,
    /// The name of the search field and its placeholder.
    pub search: String,
    /// The rename bar's label: "Name".
    pub name: String,
    /// Context menu, item: "Rename…".
    pub rename: String,
    /// Context menu, group: "Rename Group…".
    pub rename_group: String,
    /// Context menu, item: "Delete".
    pub delete: String,
    /// Context menu, group: "Delete Group".
    pub delete_group: String,
    /// Context menu submenu of the other groups: "Move to".
    pub move_to: String,
    /// Between a folder's ancestors in the Move to entries: " / ".
    pub path_separator: String,
    /// Footer, tooltip of the new button: "New".
    pub new: String,
    /// Footer, tooltip of the new group button: "New Group".
    pub new_group: String,
    /// Footer, tooltip of the delete button: "Delete".
    pub delete_button: String,
    /// Footer, count text: "{n} presets".
    pub count: String,
    /// Shown when the library has no presets.
    pub empty: String,
    /// Shown when the search matches no preset.
    pub no_match: String,
    /// A header's screen reader value: "{n} presets".
    pub group_count: String,
    /// The disclosure triangles' words.
    pub tree: TreeText,
}

impl Default for PresetLabels {
    fn default() -> Self {
        Self {
            grid_view: "Grid View".to_owned(),
            list_view: "List View".to_owned(),
            thumb_size: "Thumbnail size".to_owned(),
            footer: "Preset actions".to_owned(),
            search: "Search".to_owned(),
            name: "Name".to_owned(),
            rename: "Rename…".to_owned(),
            rename_group: "Rename Group…".to_owned(),
            delete: "Delete".to_owned(),
            delete_group: "Delete Group".to_owned(),
            move_to: "Move to".to_owned(),
            path_separator: " / ".to_owned(),
            new: "New".to_owned(),
            new_group: "New Group".to_owned(),
            delete_button: "Delete".to_owned(),
            count: "{n} presets".to_owned(),
            empty: "No presets".to_owned(),
            no_match: "No presets match".to_owned(),
            group_count: "{n} presets".to_owned(),
            tree: TreeText::default(),
        }
    }
}

impl PresetLabels {
    /// Replace every field with `translate(key)`, keeping the English default where it returns
    /// `None`, as [`DockConfig::localized`](crate::docking::DockConfig::localized) does:
    ///
    /// ```
    /// let labels = craft_ui::presets::PresetLabels::default().localized(|key| match key {
    ///     "preset.gridView" => Some("Vue en grille".to_owned()),
    ///     _ => None,
    /// });
    /// assert_eq!(labels.grid_view, "Vue en grille");
    /// ```
    ///
    /// | Key | Field |
    /// |---|---|
    /// | `preset.gridView` | `grid_view` |
    /// | `preset.listView` | `list_view` |
    /// | `preset.thumbSize` | `thumb_size` |
    /// | `preset.footer` | `footer` |
    /// | `preset.search` | `search` |
    /// | `preset.name` | `name` |
    /// | `preset.rename` | `rename` |
    /// | `preset.renameGroup` | `rename_group` |
    /// | `preset.delete` | `delete` |
    /// | `preset.deleteGroup` | `delete_group` |
    /// | `preset.moveTo` | `move_to` |
    /// | `preset.pathSeparator` | `path_separator` |
    /// | `preset.new` | `new` |
    /// | `preset.newGroup` | `new_group` |
    /// | `preset.deleteButton` | `delete_button` |
    /// | `preset.count` | `count` |
    /// | `preset.empty` | `empty` |
    /// | `preset.noMatch` | `no_match` |
    /// | `preset.groupCount` | `group_count` |
    /// | `preset.expand`, `preset.collapse`, `preset.expandTip`, `preset.collapseTip` | `tree` |
    #[must_use]
    pub fn localized(mut self, translate: impl Fn(&str) -> Option<String>) -> Self {
        for (key, label) in self.labels_mut() {
            if let Some(text) = translate(key) {
                *label = text;
            }
        }
        self
    }

    fn labels_mut(&mut self) -> [(&'static str, &mut String); 23] {
        [
            ("preset.gridView", &mut self.grid_view),
            ("preset.listView", &mut self.list_view),
            ("preset.thumbSize", &mut self.thumb_size),
            ("preset.footer", &mut self.footer),
            ("preset.search", &mut self.search),
            ("preset.name", &mut self.name),
            ("preset.rename", &mut self.rename),
            ("preset.renameGroup", &mut self.rename_group),
            ("preset.delete", &mut self.delete),
            ("preset.deleteGroup", &mut self.delete_group),
            ("preset.moveTo", &mut self.move_to),
            ("preset.pathSeparator", &mut self.path_separator),
            ("preset.new", &mut self.new),
            ("preset.newGroup", &mut self.new_group),
            ("preset.deleteButton", &mut self.delete_button),
            ("preset.count", &mut self.count),
            ("preset.empty", &mut self.empty),
            ("preset.noMatch", &mut self.no_match),
            ("preset.groupCount", &mut self.group_count),
            ("preset.expand", &mut self.tree.expand),
            ("preset.collapse", &mut self.tree.collapse),
            ("preset.expandTip", &mut self.tree.expand_tip),
            ("preset.collapseTip", &mut self.tree.collapse_tip),
        ]
    }
}
