//! Cursor chat: your bubble at the pointer, and everyone's bubbles (N5.09).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{AsIdSalt, Color32, IdSalt, Painter, Pos2, Ui};

use crate::collab::CollabRequest;

/// A chat bubble with its corner at `tip` (beside the pointer): the name over the text when
/// given, `style.chat_width` wide, in `fill` at `alpha`; rounded `style.chat_radius` except the
/// corner at the pointer, which is square (FigCraft's bubble).
// figcraft: crates/ui-egui/src/live.rs:232 `chat_bubble`.
pub fn paint_chat_bubble(painter: &Painter, tip: Pos2, name: Option<&str>, text: &str, fill: Color32, alpha: f32, style: &super::ReviewStyle) {
    todo!()
}

/// Your cursor chat: `/` over the canvas opens a bubble at the pointer with a text field (the
/// focus moves into it); each change comes back as a [`CollabRequest::Chat`] for the app to
/// send; Enter sends and keeps the bubble, Esc or an empty Enter closes it (an empty `Chat`).
/// The bubble follows the pointer. The text field is the kernel's
/// [`TextField`](craft_ui::controls::TextField); `/` is taken only while the canvas holds the
/// keys ([`focus::key_owner`](craft_ui::focus::key_owner)).
pub struct CursorChatBox<'a> {
    id_salt: IdSalt,
    text: &'a mut String,
    open: &'a mut bool,
}

impl<'a> CursorChatBox<'a> {
    pub fn new(id_salt: impl AsIdSalt, text: &'a mut String, open: &'a mut bool) -> Self {
        Self { id_salt: IdSalt::new(id_salt), text, open }
    }

    // figcraft: crates/ui-egui/src/live.rs:353 `open_chat`, `:362` `chat_box`.
    pub fn show(self, ui: &mut Ui, pointer: Option<Pos2>) -> CursorChatBoxOutput {
        todo!()
    }
}

/// What the chat box did this pass.
#[must_use]
pub struct CursorChatBoxOutput {
    pub requests: Vec<CollabRequest>,
}
