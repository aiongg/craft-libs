//! The keys a dialog answers: [`resolve_keys`] over a [`KeyInput`], as a pure function.

use super::Outcome;

/// Who gets Enter.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EnterRule {
    /// The dialog answers Enter with the default button.
    #[default]
    Default,
    /// The body owns Enter (a multi-line field, a list); the dialog never answers it.
    Body,
}

/// What [`resolve_keys`] reads, taken after the body has run: a field that took Enter (a combo box
/// choosing its option) leaves none for the dialog.
#[derive(Clone, Copy, Debug)]
pub struct KeyInput {
    /// A non-repeat press of Enter is left in this pass's input.
    pub enter: bool,
    /// Esc was pressed, with no modifier.
    pub escape: bool,
    /// The dialog is modal.
    pub modal: bool,
    /// The dialog is the top modal, or (modeless) holds the keyboard focus.
    pub active: bool,
    /// The dialog has a footer.
    pub footer: bool,
    /// A popup or a craft-ui list is open: Esc and Enter belong to it.
    pub popup_open: bool,
    /// A footer button has the keyboard focus: Enter presses it.
    pub button_focused: bool,
    /// The footer has an enabled default button.
    pub default_enabled: bool,
    pub enter_rule: EnterRule,
}

/// The outcome the keys give, if any. Esc wins over Enter pressed in the same pass.
///
/// - Esc cancels a modal dialog, also while a text field is edited. In a modeless dialog it closes
///   it when the dialog has a footer and holds the focus; a tool window without one ignores it.
/// - Enter answers [`Outcome::Ok`] when the default button is enabled, no footer button has the
///   focus (egui presses that one) and the body doesn't own Enter.
/// - With a popup open both keys are the popup's.
pub fn resolve_keys(input: KeyInput) -> Option<Outcome> {
    if !input.active || input.popup_open {
        return None;
    }
    if input.escape {
        return if input.modal {
            Some(Outcome::Cancel)
        } else if input.footer {
            Some(Outcome::Close)
        } else {
            None
        };
    }
    (input.enter && !input.button_focused && input.default_enabled && input.enter_rule == EnterRule::Default).then_some(Outcome::Ok)
}
