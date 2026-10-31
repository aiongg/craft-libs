//! The Language menu: the interface languages as radio rows, with Automatic (the system's
//! language) first.
//!
//! The Mac layout puts the submenu in the app menu after Settings ([`crate::ItemRole::Language`]);
//! Windows and Linux keep it where the app puts it (Edit › Preferences or View). Choosing a row
//! reports [`LANGUAGE_COMMAND`] with the language code; the app stores it in its preferences and
//! installs the catalog. No restart: craft-i18n switches at once.

#![expect(unused_variables, unused_mut, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_i18n::{I18n, Lang};
use craft_keymap::CommandRef;

use crate::model::Node;

/// The command every Language row runs, with `{"lang": "<code>"}` (`"auto"` for Automatic).
// vectorcraft: crates/ui-egui/src/native_menu.rs:366 (`app.language` with `{"lang": …}`).
pub const LANGUAGE_COMMAND: &str = "app.language";

/// What a Language row chose.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LanguageChoice {
    /// Follow the system's language ([`I18n::system_lang`]).
    Automatic,
    /// A language by its code (`fr`, `zh-Hant`).
    Code(String),
}

/// One language row: its code and its name in itself ("Français").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LanguageRow {
    pub code: String,
    pub name: String,
}

/// The Language submenu.
///
/// ```no_run
/// use craft_menu::{LanguageChoice, LanguageLabels, LanguageMenu};
/// # fn demo(i18n: &craft_i18n::I18n, command: &craft_menu::CommandRef) {
/// let menu = LanguageMenu::from_i18n(i18n, LanguageChoice::Automatic);
/// let node = menu.node(&LanguageLabels::default()); // into the app's View menu, or anywhere on a Mac
/// if let Some(choice) = LanguageMenu::chosen(command) { /* store it, install the language */ }
/// # }
/// ```
///
/// Rows: Automatic, a separator, then English and every offered language by code, each in its
/// own name ([`I18n::languages`], [`I18n::name`]). The current choice is the selected radio row
/// (a check mark on macOS). Names are never translated: a user who can't read the current
/// language still finds their own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LanguageMenu {
    rows: Vec<LanguageRow>,
    current: LanguageChoice,
    automatic: bool,
}

impl LanguageMenu {
    /// The menu of `rows` (English first), with `current` selected and an Automatic row.
    pub fn new(rows: Vec<LanguageRow>, current: LanguageChoice) -> Self {
        todo!()
    }

    /// The menu of the languages `i18n` offers.
    // vectorcraft: crates/ui-egui/src/native_menu.rs:364 `language_node` (`Lang::for_picker`).
    pub fn from_i18n(i18n: &I18n, current: LanguageChoice) -> Self {
        todo!()
    }

    /// Whether the Automatic row and its separator show (on by default).
    pub fn automatic(mut self, on: bool) -> Self {
        todo!()
    }

    /// The rows as radio items running [`LANGUAGE_COMMAND`].
    pub fn items(&self, labels: &LanguageLabels) -> Vec<Node> {
        todo!()
    }

    /// The "Language" submenu, tagged [`crate::ItemRole::Language`].
    pub fn node(&self, labels: &LanguageLabels) -> Node {
        todo!()
    }

    /// The choice a command made, `None` for any other command or an unknown parameter.
    pub fn chosen(command: &CommandRef) -> Option<LanguageChoice> {
        todo!()
    }

    /// The language a choice means: the system's for Automatic, English for a code `i18n`
    /// doesn't have.
    pub fn resolve(choice: &LanguageChoice, i18n: &I18n) -> Lang {
        todo!()
    }
}

craft_ui::words! {
    /// The Language menu's labels, English by default.
    #[non_exhaustive]
    pub struct LanguageLabels {
        language: "language.menu" = "Language";
        automatic: "language.automatic" = "Automatic";
    }
}
