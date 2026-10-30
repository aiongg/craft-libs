//! [`PresetLabels`]: every string a preset browser shows or announces, for the host to translate.

use crate::list::TreeText;
use crate::localized::{Labels, Word, Words};

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
    /// The name of the search field's clear button: "Clear".
    pub clear: String,
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
        <Self as Labels>::english()
    }
}

/// One word per text field, in the order of [`PresetLabels::texts_mut`].
const WORDS: [Word; 20] = [
    Word::keyed("preset.gridView", "Grid View"),
    Word::keyed("preset.listView", "List View"),
    Word::keyed("preset.thumbSize", "Thumbnail size"),
    Word::keyed("preset.footer", "Preset actions"),
    Word::keyed("preset.search", "Search"),
    Word::keyed("preset.clear", "Clear"),
    Word::keyed("preset.name", "Name"),
    Word::keyed("preset.rename", "Rename…"),
    Word::keyed("preset.renameGroup", "Rename Group…"),
    Word::keyed("preset.delete", "Delete"),
    Word::keyed("preset.deleteGroup", "Delete Group"),
    Word::keyed("preset.moveTo", "Move to"),
    Word::keyed("preset.pathSeparator", " / "),
    Word::keyed("preset.new", "New"),
    Word::keyed("preset.newGroup", "New Group"),
    Word::keyed("preset.deleteButton", "Delete"),
    Word::keyed("preset.count", "{n} presets"),
    Word::keyed("preset.empty", "No presets"),
    Word::keyed("preset.noMatch", "No presets match"),
    Word::keyed("preset.groupCount", "{n} presets"),
];

impl Labels for PresetLabels {
    const WORDS: &'static [Word] = &WORDS;

    fn english() -> Self {
        Self::localized_in(Self::blank(), &Words::english())
    }

    fn localized_in(mut self, words: &Words) -> Self {
        for (word, text) in WORDS.iter().zip(self.texts_mut()) {
            *text = words.get(*word).into_owned();
        }
        self.tree = TreeText::english().localized_in(words);
        self
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
    /// | `preset.clear` | `clear` |
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
        for (word, text) in WORDS.iter().zip(self.texts_mut()) {
            if let Some(translated) = translate(word.key()) {
                *text = translated;
            }
        }
        self.tree = self.tree.localized(translate);
        self
    }

    fn blank() -> Self {
        Self {
            grid_view: String::new(),
            list_view: String::new(),
            thumb_size: String::new(),
            footer: String::new(),
            search: String::new(),
            clear: String::new(),
            name: String::new(),
            rename: String::new(),
            rename_group: String::new(),
            delete: String::new(),
            delete_group: String::new(),
            move_to: String::new(),
            path_separator: String::new(),
            new: String::new(),
            new_group: String::new(),
            delete_button: String::new(),
            count: String::new(),
            empty: String::new(),
            no_match: String::new(),
            group_count: String::new(),
            tree: TreeText::english(),
        }
    }

    fn texts_mut(&mut self) -> [&mut String; 20] {
        [
            &mut self.grid_view,
            &mut self.list_view,
            &mut self.thumb_size,
            &mut self.footer,
            &mut self.search,
            &mut self.clear,
            &mut self.name,
            &mut self.rename,
            &mut self.rename_group,
            &mut self.delete,
            &mut self.delete_group,
            &mut self.move_to,
            &mut self.path_separator,
            &mut self.new,
            &mut self.new_group,
            &mut self.delete_button,
            &mut self.count,
            &mut self.empty,
            &mut self.no_match,
            &mut self.group_count,
        ]
    }
}
