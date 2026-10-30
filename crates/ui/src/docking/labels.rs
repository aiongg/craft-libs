//! One keyed hook for every label in [`DockConfig`].

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
    ///         "dock.close" => Some("Fermer le panneau".to_owned()),
    ///         _ => None,
    ///     })
    /// }
    /// ```
    ///
    /// | Key | Field | English default |
    /// |---|---|---|
    /// | `dock.float` | `float_label` | Float Panel |
    /// | `dock.close` | `close_label` | Close Panel |
    /// | `dock.movePanelTo` | `move_panel_to_label` | Move Panel to Group |
    /// | `dock.panels` | `panels_label` | Panels |
    /// | `dock.resize` | `resize_label` | Resize panels |
    /// | `dock.resizeWindow` | `resize_window_label` | Resize panel window |
    /// | `dock.moveGroup` | `move_group_label` | Move tab group |
    /// | `dock.closeGroup` | `close_group_label` | Close Tab Group |
    /// | `dock.minimize` | `minimize_label` | Minimize |
    /// | `dock.expand` | `expand_label` | Expand |
    /// | `dock.floatGroup` | `float_group_label` | Float Tab Group |
    /// | `dock.moveGroupUp` | `move_group_up_label` | Move Group Up |
    /// | `dock.moveGroupDown` | `move_group_down_label` | Move Group Down |
    /// | `dock.moveGroupTo` | `move_group_to_label` | Move Group To |
    /// | `dock.moveColumn` | `move_column_label` | Move panel column |
    /// | `dock.collapseToIcons` | `collapse_to_icons_label` | Collapse to Icons |
    /// | `dock.expandPanels` | `expand_panels_label` | Expand Panels |
    /// | `dock.collapseColumn` | `collapse_column_label` | Collapse column to icons |
    /// | `dock.expandColumn` | `expand_column_label` | Expand column |
    /// | `dock.moveWindow` | `move_window_label` | Move panel window |
    /// | `dock.closeWindow` | `close_window_label` | Close panel window |
    /// | `dock.closeOthers` | `close_others_label` | Close Other Panels in Group |
    /// | `dock.maximize` | `maximize_label` | Maximize Frame |
    /// | `dock.restoreFrame` | `restore_frame_label` | Restore Frame Size |
    /// | `dock.autoCollapse` | `auto_collapse_label` | Auto-Collapse Iconic Panels |
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

    fn labels_mut(&mut self) -> [&mut String; 27] {
        [
            &mut self.float_label,
            &mut self.close_label,
            &mut self.move_panel_to_label,
            &mut self.panels_label,
            &mut self.resize_label,
            &mut self.resize_window_label,
            &mut self.move_group_label,
            &mut self.close_group_label,
            &mut self.minimize_label,
            &mut self.expand_label,
            &mut self.float_group_label,
            &mut self.move_group_up_label,
            &mut self.move_group_down_label,
            &mut self.move_group_to_label,
            &mut self.move_column_label,
            &mut self.collapse_to_icons_label,
            &mut self.expand_panels_label,
            &mut self.collapse_column_label,
            &mut self.expand_column_label,
            &mut self.move_window_label,
            &mut self.close_window_label,
            &mut self.close_others_label,
            &mut self.maximize_label,
            &mut self.restore_frame_label,
            &mut self.auto_collapse_label,
            &mut self.float_hint_label,
            &mut self.panel_menu_label,
        ]
    }
}

/// One word per label field, in the order of [`DockConfig::labels_mut`].
const WORDS: [Word; 27] = [
    Word::keyed("dock.float", "Float Panel"),
    Word::keyed("dock.close", "Close Panel"),
    Word::keyed("dock.movePanelTo", "Move Panel to Group"),
    Word::keyed("dock.panels", "Panels"),
    Word::keyed("dock.resize", "Resize panels"),
    Word::keyed("dock.resizeWindow", "Resize panel window"),
    Word::keyed("dock.moveGroup", "Move tab group"),
    Word::keyed("dock.closeGroup", "Close Tab Group"),
    Word::keyed("dock.minimize", "Minimize"),
    Word::keyed("dock.expand", "Expand"),
    Word::keyed("dock.floatGroup", "Float Tab Group"),
    Word::keyed("dock.moveGroupUp", "Move Group Up"),
    Word::keyed("dock.moveGroupDown", "Move Group Down"),
    Word::keyed("dock.moveGroupTo", "Move Group To"),
    Word::keyed("dock.moveColumn", "Move panel column"),
    Word::keyed("dock.collapseToIcons", "Collapse to Icons"),
    Word::keyed("dock.expandPanels", "Expand Panels"),
    Word::keyed("dock.collapseColumn", "Collapse column to icons"),
    Word::keyed("dock.expandColumn", "Expand column"),
    Word::keyed("dock.moveWindow", "Move panel window"),
    Word::keyed("dock.closeWindow", "Close panel window"),
    Word::keyed("dock.closeOthers", "Close Other Panels in Group"),
    Word::keyed("dock.maximize", "Maximize Frame"),
    Word::keyed("dock.restoreFrame", "Restore Frame Size"),
    Word::keyed("dock.autoCollapse", "Auto-Collapse Iconic Panels"),
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
    ///         "dock.close" => Some("Fermer le panneau".to_owned()),
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
