//! [`AltTextField`]: an object's description for people who can't see it, with Mark as
//! decorative.

use egui::{AsIdSalt, IdSalt, Label, RichText, Ui};

use super::{Checkbox, TextField};
use crate::edit::{EditEvent, EditResponse};
use crate::localized::Labels as _;
use crate::theme::{ToEgui as _, Tokens, TokensOf as _};

/// The box's height in lines (DeckCraft's `desired_rows(4)`).
const BOX_ROWS: usize = 4;

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
    labels: Option<AltTextLabels>,
}

impl<'a> AltTextField<'a> {
    pub fn new(id_salt: impl AsIdSalt, value: &'a mut AltText) -> Self {
        Self { id_salt: IdSalt::new(id_salt), value, mixed: false, enabled: true, labels: None }
    }

    /// Words other than the installed ones (WordCraft's pane has its own prompt).
    pub fn labels(mut self, labels: AltTextLabels) -> Self {
        self.labels = Some(labels);
        self
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
        let Self { id_salt, value, mixed, enabled, labels } = self;
        let labels = labels.unwrap_or_else(|| AltTextLabels::of(ui));
        let tokens = Tokens::of(ui);
        let enabled = enabled && ui.is_enabled();
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.y = ui.spacing().item_spacing.y.max(tokens.component.alt_text.row_gap);
            ui.add(
                Label::new(
                    RichText::new(labels.prompt.as_str()).font(tokens.font(craft_theme::TypeRole::Body)).color(tokens.color.text.normal.to_egui()),
                )
                .wrap(),
            );
            let placeholder = if mixed { &labels.mixed } else { &labels.alt_text };
            let mut field = TextField::new((&id_salt, "text"), labels.alt_text.as_str(), &value.text)
                .placeholder(placeholder.as_str())
                .multiline(BOX_ROWS)
                .enabled(enabled && !value.decorative);
            if !ui.layout().horizontal_justify() {
                field = field.width(tokens.component.alt_text.box_width);
            }
            let typed = field.show(ui);
            let mut event = typed.event.as_ref().map(|e| e.clone().map(|text| AltText { text, decorative: value.decorative }));
            if let Some(text) = typed.event.as_ref().and_then(EditEvent::value) {
                value.text.clone_from(text);
            }
            let mut decorative = value.decorative;
            let check = Checkbox::bool(labels.decorative.as_str(), &mut decorative).id_salt((&id_salt, "decorative")).enabled(enabled).show(ui);
            if let Some(EditEvent::Set { modifiers, .. }) = check.event {
                value.decorative = decorative;
                // The press that checks the box also ends a typing run; the run's event is the
                // one the host needs, and it carries the new flag with the text.
                let whole = AltText { text: value.text.clone(), decorative };
                event = match event {
                    Some(typed) => Some(typed.map(|_| whole)),
                    None => Some(EditEvent::set(whole, modifiers)),
                };
            }
            let mut response = typed.response;
            if check.response.changed() {
                response.mark_changed();
            }
            EditResponse::new(response, event).submitted(typed.submitted).escaped(typed.escaped)
        })
        .inner
    }
}

/// Whether Mark as decorative starts checked for objects with these flags: only when there is at
/// least one and every one is decorative.
// deckcraft: crates/ui-egui/src/dialogs/alt_text.rs:2 alt_text_starts_decorative
pub fn starts_decorative(decorative: impl IntoIterator<Item = bool>) -> bool {
    let mut any = false;
    for flag in decorative {
        if !flag {
            return false;
        }
        any = true;
    }
    any
}
