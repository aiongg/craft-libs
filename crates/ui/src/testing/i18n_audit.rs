#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! Interface text drawn without going through translation.
//!
//! The app runs in craft-i18n's pseudo locale: its Host installed with `.lang(Lang::PSEUDO)`, so
//! the app's lookups and craft-ui's own `Words` come back as `⟦text⟧`. A test draws the app in
//! many states (each tab, page, dialog, menu, tooltip) and hands each frame's text to an
//! [`Audit`]; any word left outside `⟦…⟧` was drawn as a literal. Text the author or the machine
//! supplied (document text, font names, file names, shortcuts) is [`Exempt`].
//!
//! The audit reads what was painted (`Shape::Text`), not accessibility labels, so a tooltip or a
//! value drawn by hand is caught too. Nothing here depends on catalogs, so a missing translation
//! never fails it: the pseudo locale marks every lookup, translated or not.
//!
//! ```ignore
//! use craft_ui::testing::i18n_audit::{Audit, Exempt, drawn_texts};
//!
//! let mut audit = Audit::new();
//! for tab in TABS {
//!     app.run("ui.tab", json!({"tab": tab}));
//!     harness.run();
//!     let exempt = Exempt::new().names(font_families()).body(app.document_text());
//!     audit.record(&format!("tab {tab}"), drawn_texts(harness.output()), &exempt);
//! }
//! audit.assert_clean();
//! ```

use std::collections::{BTreeMap, BTreeSet, HashSet};

/// `text` without the parts inside `⟦…⟧` marks (marks may nest; an unmatched `⟧` is ignored).
pub fn unmarked(text: &str) -> String {
    // wordcraft: crates/ui-egui/src/i18n/pseudo.rs:28
    todo!()
}

/// Whether `text` has a word outside the marks: two ASCII letters in a row in [`unmarked`].
pub fn english_words(text: &str) -> bool {
    // wordcraft: crates/ui-egui/src/i18n/audit.rs:111
    todo!()
}

/// Every text a frame painted, in paint order: each `Shape::Text`'s galley text, with
/// `Shape::Vec` flattened. Pass `Harness::output()` or a `Context::run_ui` result.
pub fn drawn_texts(output: &egui::FullOutput) -> Vec<String> {
    // wordcraft: crates/ui-egui/src/i18n/audit.rs:29
    todo!()
}

/// Text that is right to show untranslated.
///
/// Always allowed, on the unmarked text trimmed (and a trailing ` —` dropped):
/// - a shortcut: every `+`-separated part is `Ctrl`, `Shift`, `Alt`, `Cmd`, `Esc`, `Tab` or
///   `Enter`, or at most 3 characters;
/// - account initials: exactly 2 ASCII capitals.
///
/// The app adds its own: exact [`Self::names`], document [`Self::body`] text and [`Self::prefixes`].
#[derive(Clone, Debug, Default)]
pub struct Exempt {
    names: HashSet<String>,
    body: String,
    prefixes: Vec<String>,
}

impl Exempt {
    /// Only the built-in rules.
    pub fn new() -> Self {
        Self::default()
    }

    /// Texts allowed exactly: font families, style names, product names, the user's dictionary.
    pub fn names<S: Into<String>>(self, names: impl IntoIterator<Item = S>) -> Self {
        // wordcraft: crates/ui-egui/src/i18n/audit.rs:150
        todo!()
    }

    /// The document's text: any drawn text it contains is allowed (headings in a navigation
    /// pane, the title in the title bar). Appended to earlier calls with a newline.
    pub fn body(self, text: &str) -> Self {
        // wordcraft: crates/ui-egui/src/i18n/audit.rs:175
        todo!()
    }

    /// Starts of text the user or the machine supplied (`@` for contributor handles, `Style` for a
    /// new style's default name).
    pub fn prefixes<S: Into<String>>(self, prefixes: impl IntoIterator<Item = S>) -> Self {
        // wordcraft: crates/ui-egui/src/i18n/audit.rs:206
        todo!()
    }

    /// Whether `text` may be drawn as is.
    pub fn allows(&self, text: &str) -> bool {
        // wordcraft: crates/ui-egui/src/i18n/audit.rs:182
        todo!()
    }
}

/// The untranslated texts found, each with the UI states it showed in.
#[derive(Clone, Debug, Default)]
pub struct Audit {
    found: BTreeMap<String, BTreeSet<String>>,
}

impl Audit {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record the texts of one UI state (`"tab Home"`, `"dialog font"`): those with
    /// [`english_words`] that `exempt` doesn't allow.
    pub fn record(&mut self, state: &str, drawn: impl IntoIterator<Item = String>, exempt: &Exempt) {
        // wordcraft: crates/ui-egui/src/i18n/audit.rs:239
        todo!()
    }

    /// Whether nothing was found.
    pub fn is_empty(&self) -> bool {
        // new: the check behind assert_clean
        todo!()
    }

    /// The findings, sorted by text: the drawn text and its states, sorted.
    pub fn findings(&self) -> Vec<(&str, Vec<&str>)> {
        // new: for an app that reports instead of failing
        todo!()
    }

    /// One line per finding: `{unmarked text:?}  [{state}, {state}]\n`.
    pub fn report(&self) -> String {
        // wordcraft: crates/ui-egui/src/i18n/audit.rs:382
        todo!()
    }

    /// Fail the test when anything was found, with
    /// `{n} interface texts skip translation (look them up in the catalog):\n{report}`.
    #[track_caller]
    pub fn assert_clean(&self) {
        // wordcraft: crates/ui-egui/src/i18n/audit.rs:385
        todo!()
    }
}

#[cfg(test)]
#[path = "i18n_audit_tests.rs"]
mod i18n_audit_tests;
