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
