#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! [`AltTextField`]: an object's description for people who can't see it, with Mark as
//! decorative.

use egui::{AsIdSalt, IdSalt, Ui};

use crate::edit::EditResponse;

crate::words! {
    /// The field's words, English by default (PowerPoint's and DeckCraft's).
    #[non_exhaustive]
    pub struct AltTextLabels {
        /// The prompt over the box.
        prompt: "altText.prompt" = "How would you describe this object and its context to someone who is blind or has low vision?";
        /// The box's placeholder and accessible name.
        alt_text: "altText.altText" = "Alt text";
        decorative: "altText.decorative" = "Mark as decorative";
        /// The box's placeholder while the description is mixed across the selection.
        mixed: "altText.mixed" = "Several descriptions";
    }
}

/// An object's alt text: its description and whether it is decorative (read by no one).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AltText {
    pub text: String,
    pub decorative: bool,
}

/// Alt text's editor, after DeckCraft's Alt Text dialog and WordCraft's Alt Text pane.
///
/// ```ignore
/// let start = AltText { text: first.descr.clone(), decorative: starts_decorative(shapes.iter().map(|s| s.decorative)) };
/// let out = AltTextField::new("alt", &mut dialog.alt).show(ui);
/// // On OK: app.run("shape.altText", dialog.alt)
/// ```
///
/// - **Look.** [`AltTextLabels::prompt`] in the `Body` role, wrapping, over a multi-line text box
///   four lines high and 340 points wide (DeckCraft's `desired_rows(4)`, `desired_width(340)`;
///   it stretches in a justified layout), then a [`Checkbox`](crate::controls::Checkbox) Mark
///   as decorative.
/// - **Decorative.** While checked, the box is disabled and keeps its text (PowerPoint's): the
///   app stores the text and the flag; unchecking brings the text back.
/// - **Mixed** ([`mixed`](Self::mixed)): the selection's descriptions differ; the box is empty
///   with [`AltTextLabels::mixed`] as its placeholder until the user types.
/// - **Report.** The box is one typing run per gesture, as a
///   [`TextField`](crate::controls::TextField); the check is a `Set`. Both report the whole
///   [`AltText`] in one [`EditResponse`].
/// - **Keys.** The box takes Tab as focus movement, not a tab character; Enter is a new line,
///   Cmd/Ctrl+Enter submits ([`EditResponse::submitted`], the dialog's OK).
pub struct AltTextField<'a> {
    id_salt: IdSalt,
    value: &'a mut AltText,
    mixed: bool,
    enabled: bool,
}

impl<'a> AltTextField<'a> {
    pub fn new(id_salt: impl AsIdSalt, value: &'a mut AltText) -> Self {
        Self { id_salt: IdSalt::new(id_salt), value, mixed: false, enabled: true }
    }

    /// The selected objects' descriptions differ.
    pub fn mixed(mut self, mixed: bool) -> Self {
        self.mixed = mixed;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    // deckcraft: crates/ui-egui/src/dialogs/body.rs:596 dlg_alt_text
    pub fn show(self, ui: &mut Ui) -> EditResponse<AltText> {
        todo!()
    }
}

/// Whether Mark as decorative starts checked for objects with these flags: only when there is at
/// least one and every one is decorative.
// deckcraft: crates/ui-egui/src/dialogs/alt_text.rs:2 alt_text_starts_decorative
pub fn starts_decorative(decorative: impl IntoIterator<Item = bool>) -> bool {
    todo!()
}
