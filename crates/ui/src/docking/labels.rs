//! One keyed hook for every label in [`DockConfig`]. The dock's menu rows are not here: they are
//! [`super::panel_menu::PanelMenuLabels`], which an app overrides with [`DockConfig::panel_menu`].

use super::{DockConfig, DockStyle};
use crate::localized::{Labels, Word, Words};

impl DockConfig {
    /// Replace every `*_label` field with `translate(key)`, keeping the English default where it
    /// returns `None`. The defaults of menu items are in title case; those of controls, which
    /// only screen readers announce, in sentence case. An application translates the docking
    /// chrome in one place:
    ///
    /// ```
    /// fn dock_config() -> craft_ui::docking::DockConfig {
    ///     craft_ui::docking::DockConfig::default().localized(|key| match key {
    ///         "dock.panels" => Some("Panneaux".to_owned()),
    ///         _ => None,
    ///     })
    /// }
    /// ```
    ///
    /// | Key | Field | English default |
    /// |---|---|---|
    /// | `dock.panels` | `panels_label` | Panels |
    /// | `dock.resize` | `resize_label` | Resize panels |
    /// | `dock.resizeWindow` | `resize_window_label` | Resize panel window |
    /// | `dock.moveGroup` | `move_group_label` | Move tab group |
    /// | `dock.moveColumn` | `move_column_label` | Move panel column |
    /// | `dock.collapseColumn` | `collapse_column_label` | Collapse column to icons |
    /// | `dock.expandColumn` | `expand_column_label` | Expand column |
    /// | `dock.moveWindow` | `move_window_label` | Move panel window |
    /// | `dock.closeWindow` | `close_window_label` | Close panel window |
    /// | `dock.floatHint` | `float_hint_label` | Hold {key} to undock |
    /// | `dock.panelMenu` | `panel_menu_label` | Panel Menu |
    ///
    /// Every group's drag bar and icon grip, and every column bar, has the same default name.
    /// Opt in to naming them, in the English label and each translation: a `{name}` in
    /// `move_group_label` stands for the logical label of the panel the group shows, and in
    /// `move_column_label` for those of the panels the column's groups show, joined by ", ", so a
    /// screen reader tells the handles apart. The ", " is fixed. Untitled panels are left out;
    /// with no title at all the placeholder goes with the brackets or quotes around it, a
    /// separator right after it and one of the spaces beside it ("Move the tab group"). Groups
    /// showing panels of the same title still share a name:
    ///
    /// ```
    /// let config = craft_ui::docking::DockConfig::default().localized(|key| match key {
    ///     "dock.moveGroup" => Some("Move the {name} tab group".to_owned()),
    ///     "dock.moveColumn" => Some("Move the {name} panel column".to_owned()),
    ///     _ => None,
    /// });
    /// # assert_eq!(config.move_group_label, "Move the {name} tab group");
    /// ```
    #[must_use]
    pub fn localized(mut self, translate: impl Fn(&str) -> Option<String>) -> Self {
        for (word, label) in WORDS.iter().zip(self.labels_mut()) {
            if let Some(text) = translate(word.key()) {
                *label = text;
            }
        }
        self
    }

    fn labels_mut(&mut self) -> [&mut String; 11] {
        [
            &mut self.panels_label,
            &mut self.resize_label,
            &mut self.resize_window_label,
            &mut self.move_group_label,
            &mut self.move_column_label,
            &mut self.collapse_column_label,
            &mut self.expand_column_label,
            &mut self.move_window_label,
            &mut self.close_window_label,
            &mut self.float_hint_label,
            &mut self.panel_menu_label,
        ]
    }
}

/// One word per label field, in the order of [`DockConfig::labels_mut`].
const WORDS: [Word; 11] = [
    Word::keyed("dock.panels", "Panels"),
    Word::keyed("dock.resize", "Resize panels"),
    Word::keyed("dock.resizeWindow", "Resize panel window"),
    Word::keyed("dock.moveGroup", "Move tab group"),
    Word::keyed("dock.moveColumn", "Move panel column"),
    Word::keyed("dock.collapseColumn", "Collapse column to icons"),
    Word::keyed("dock.expandColumn", "Expand column"),
    Word::keyed("dock.moveWindow", "Move panel window"),
    Word::keyed("dock.closeWindow", "Close panel window"),
    Word::keyed("dock.floatHint", "Hold {key} to undock"),
    Word::keyed("dock.panelMenu", "Panel Menu"),
];

impl Labels for DockConfig {
    const WORDS: &'static [Word] = &WORDS;

    fn english() -> Self {
        Self::default()
    }

    fn localized_in(mut self, words: &Words) -> Self {
        for (word, label) in WORDS.iter().zip(self.labels_mut()) {
            *label = words.get(*word).into_owned();
        }
        self
    }
}

impl DockStyle {
    /// [`DockConfig::localized`] for the flat style:
    ///
    /// ```
    /// fn dock_style(ui: &egui::Ui) -> craft_ui::docking::DockStyle {
    ///     craft_ui::docking::DockStyle::from_ui(ui).localized(|key| match key {
    ///         "dock.panels" => Some("Panneaux".to_owned()),
    ///         _ => None,
    ///     })
    /// }
    /// ```
    #[must_use]
    pub fn localized(self, translate: impl Fn(&str) -> Option<String>) -> Self {
        let (config, look) = self.split();
        Self::join(&config.localized(translate), &look)
    }
}

/// `label` with every `{name}` replaced by `name()`, which only runs when there is one. A blank
/// name leaves each placeholder out with what only made sense around it (see [`unnamed`]); a
/// label with no letter or digit left gives `fallback`, so no handle is announced with a hole in
/// its name or without one.
pub(super) fn named(label: &str, fallback: &str, name: impl FnOnce() -> String) -> String {
    if !label.contains(NAME) {
        return label.to_owned();
    }
    let name = name();
    if !name.trim().is_empty() {
        return label.replace(NAME, name.trim());
    }
    let bare = unnamed(label);
    if bare.chars().any(char::is_alphanumeric) { bare } else { fallback.to_owned() }
}

/// `label` without its placeholders. Each goes with the brackets or quotes around it and the
/// spaces inside them, and with a separator right after it. Where both sides were spaces, one
/// stays; where neither was, none is added. Separators and spaces at the ends go too.
fn unnamed(label: &str) -> String {
    let mut text = label.to_owned();
    // Every pass removes a placeholder, so the text gets shorter each time.
    while let Some((before, after)) = text.split_once(NAME) {
        let (mut head, mut tail) = (before, after);
        let (open, close) = (head.trim_end(), tail.trim_start());
        if let (Some(left), Some(right)) = (open.chars().next_back(), close.chars().next())
            && PAIRS.contains(&(left, right))
        {
            head = open.strip_suffix(left).unwrap_or(open);
            tail = close.strip_prefix(right).unwrap_or(close);
        }
        let next = tail.trim_start();
        if let Some(separator) = next.chars().next().filter(|c| SEPARATORS.contains(c))
            && !head.trim().is_empty()
        {
            tail = next.strip_prefix(separator).unwrap_or(next);
        }
        text = if head.ends_with(char::is_whitespace) && tail.starts_with(char::is_whitespace) {
            format!("{} {}", head.trim_end(), tail.trim_start())
        } else {
            format!("{head}{tail}")
        };
    }
    text.trim_matches(|c: char| c.is_whitespace() || SEPARATORS.contains(&c) || DASHES.contains(&c)).to_owned()
}

/// Brackets and quotes, opening and closing, that can wrap the placeholder.
const PAIRS: &[(char, char)] = &[
    ('(', ')'),
    ('[', ']'),
    ('<', '>'),
    ('"', '"'),
    ('\'', '\''),
    ('«', '»'),
    ('»', '«'),
    ('‹', '›'),
    ('›', '‹'),
    ('“', '”'),
    ('„', '“'),
    ('”', '”'),
    ('‘', '’'),
    ('‚', '‘'),
    ('’', '’'),
    ('「', '」'),
    ('『', '』'),
    ('（', '）'),
    ('［', '］'),
    ('【', '】'),
    ('〈', '〉'),
    ('《', '》'),
];

/// Separators that go with a placeholder they follow, or that are left at an end.
const SEPARATORS: &[char] = &[',', ';', ':', '、', '，', '；', '：'];

/// Dashes left at an end.
const DASHES: &[char] = &['-', '–', '—'];

/// The default [`DockStyle::move_group_label`].
pub(super) const MOVE_GROUP: &str = "Move tab group";
/// The default [`DockStyle::move_column_label`].
pub(super) const MOVE_COLUMN: &str = "Move panel column";

/// What [`DockStyle::move_group_label`] and [`DockStyle::move_column_label`] put the moved
/// group's or column's name in place of.
const NAME: &str = "{name}";
