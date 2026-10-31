//! Cursor chat: your bubble at the pointer, and everyone's bubbles (N5.09).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{Color32, Painter, Pos2, Ui};

use crate::collab::CollabTransport;

/// A chat bubble with its corner at `tip` (beside the pointer): the name over the text when
/// given, [`CHAT_WIDTH`](crate::collab::CHAT_WIDTH) wide, in `fill` at `alpha`; rounded 14
/// points except the corner at the pointer (2).
// figcraft: crates/ui-egui/src/live.rs:232 `chat_bubble`.
pub fn paint_chat_bubble(painter: &Painter, tip: Pos2, name: Option<&str>, text: &str, fill: Color32, alpha: f32) {
    todo!()
}

/// Your cursor chat: `/` over the canvas opens a bubble at the pointer with a text field (the
/// focus moves into it); each change is sent through the transport; Enter sends and keeps the
/// bubble, Esc or an empty Enter closes it. The bubble follows the pointer.
pub struct CursorChatBox<'a> {
    transport: &'a mut dyn CollabTransport,
    text: &'a mut String,
    open: &'a mut bool,
}

impl<'a> CursorChatBox<'a> {
    pub fn new(transport: &'a mut dyn CollabTransport, text: &'a mut String, open: &'a mut bool) -> Self {
        Self { transport, text, open }
    }

    // figcraft: crates/ui-egui/src/live.rs:353 `open_chat`, `:362` `chat_box`.
    pub fn show(self, ui: &mut Ui, pointer: Option<Pos2>) {
        todo!()
    }
}
