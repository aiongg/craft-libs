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
    let mut out = String::new();
    let mut depth = 0usize;
    for c in text.chars() {
        match c {
            '⟦' => depth += 1,
            '⟧' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out
}

/// Whether `text` has a word outside the marks: two ASCII letters in a row in [`unmarked`].
pub fn english_words(text: &str) -> bool {
    unmarked(text).as_bytes().windows(2).any(|w| w[0].is_ascii_alphabetic() && w[1].is_ascii_alphabetic())
}

/// Every text a frame painted, in paint order: each `Shape::Text`'s galley text, with
/// `Shape::Vec` flattened. Pass `Harness::output()` or a `Context::run_ui` result.
pub fn drawn_texts(output: &egui::FullOutput) -> Vec<String> {
    fn collect(shape: &egui::Shape, out: &mut Vec<String>) {
        match shape {
            egui::Shape::Text(t) => out.push(t.galley.text().to_string()),
            egui::Shape::Vec(v) => v.iter().for_each(|s| collect(s, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    for clipped in &output.shapes {
        collect(&clipped.shape, &mut out);
    }
    out
}

/// Text that is right to show untranslated.
///
/// Always allowed, on the unmarked text trimmed (and a trailing ` —` dropped):
/// - a shortcut: text with a `+` whose every part is a key name (`Ctrl`, `Shift`, `Alt`, `Cmd`,
///   `Esc`, `Tab`, `Enter`, or what `egui::Key::from_name` knows: `S`, `F5`, `Plus`), or a single
///   key cap on its own: one of those modifier names, an F-key, or one character. A word of three
///   letters such as "Add" or "All" is text and must be translated;
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
    pub fn names<S: Into<String>>(mut self, names: impl IntoIterator<Item = S>) -> Self {
        self.names.extend(names.into_iter().map(Into::into));
        self
    }

    /// The document's text: any drawn text it contains is allowed (headings in a navigation
    /// pane, the title in the title bar). Appended to earlier calls with a newline.
    pub fn body(mut self, text: &str) -> Self {
        if !self.body.is_empty() {
            self.body.push('\n');
        }
        self.body.push_str(text);
        self
    }

    /// Starts of text the user or the machine supplied (`@` for contributor handles, `Style` for a
    /// new style's default name).
    pub fn prefixes<S: Into<String>>(mut self, prefixes: impl IntoIterator<Item = S>) -> Self {
        self.prefixes.extend(prefixes.into_iter().map(Into::into));
        self
    }

    /// Whether `text` may be drawn as is.
    pub fn allows(&self, text: &str) -> bool {
        let plain = unmarked(text);
        let plain = plain.trim().trim_end_matches(" —").trim();
        let is_shortcut = if plain.contains('+') && !plain.starts_with('+') && !plain.ends_with('+') {
            plain.split('+').all(|k| is_modifier_or_key(k) || egui::Key::from_name(k).is_some())
        } else {
            is_modifier_or_key(plain) || plain.chars().count() == 1 || is_function_key(plain)
        };
        let initials = plain.chars().count() == 2 && plain.chars().all(|c| c.is_ascii_uppercase());
        is_shortcut
            || initials
            || self.body.contains(plain)
            || self.names.contains(plain)
            || self.prefixes.iter().any(|p| plain.starts_with(p.as_str()))
    }
}

fn is_modifier_or_key(k: &str) -> bool {
    matches!(k, "Ctrl" | "Shift" | "Alt" | "Cmd" | "Esc" | "Tab" | "Enter")
}

/// `F1` to `F35`.
fn is_function_key(k: &str) -> bool {
    k.strip_prefix('F').is_some_and(|n| !n.is_empty() && n.len() <= 2 && n.bytes().all(|b| b.is_ascii_digit()) && n != "0")
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
        for text in drawn {
            if english_words(&text) && !exempt.allows(&text) {
                self.found.entry(text).or_default().insert(state.to_string());
            }
        }
    }

    /// Whether nothing was found.
    pub fn is_empty(&self) -> bool {
        self.found.is_empty()
    }

    /// The findings, sorted by text: the drawn text and its states, sorted.
    pub fn findings(&self) -> Vec<(&str, Vec<&str>)> {
        self.found.iter().map(|(text, states)| (text.as_str(), states.iter().map(String::as_str).collect())).collect()
    }

    /// One line per finding: `{unmarked text:?}  [{state}, {state}]\n`.
    pub fn report(&self) -> String {
        let mut report = String::new();
        for (text, states) in &self.found {
            let states: Vec<&str> = states.iter().map(String::as_str).collect();
            report.push_str(&format!("{:?}  [{}]\n", unmarked(text), states.join(", ")));
        }
        report
    }

    /// Fail the test when anything was found, with
    /// `{n} interface texts skip translation (look them up in the catalog):\n{report}`.
    #[track_caller]
    pub fn assert_clean(&self) {
        assert!(self.is_empty(), "{} interface texts skip translation (look them up in the catalog):\n{}", self.found.len(), self.report());
    }
}

#[cfg(test)]
#[path = "i18n_audit_tests.rs"]
mod i18n_audit_tests;
