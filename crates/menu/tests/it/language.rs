//! The Language menu.

use serde_json::json;

use craft_menu::{CommandRef, ItemKind, ItemRole, LANGUAGE_COMMAND, LanguageChoice, LanguageLabels, LanguageMenu, LanguageRow, Node};
use craft_ui::testing::trial;

fn row(code: &str, name: &str) -> LanguageRow {
    LanguageRow::new(code, name)
}

// Automatic, a separator, then English and the offered languages in their own names; the current one selected.

// app.language {"lang":"fr"} reads as Code("fr"), {"lang":"auto"} as Automatic, other commands as None.
