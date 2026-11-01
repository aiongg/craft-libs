//! The Freeform view: cards placed by the user, stacks and snapping.

#![expect(dead_code, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::Vec2;

/// A card in the Freeform view, where the host stores it.
#[derive(Clone, Debug, PartialEq)]
pub struct FreeformCard<K> {
    /// The item or bin.
    pub key: K,
    /// Its top-left corner in the view, in points.
    pub pos: Vec2,
    /// The stack it belongs to (cards of one stack move together); the stack's first card shows
    /// the "×N" badge.
    pub stack: Option<u32>,
}

/// Draws the Freeform view: cards at their positions; a drag moves the selection and the dragged
/// card's stack together, snapping to `snap` when set; Esc cancels and puts the cards back;
/// release reports `FreeformMoved { keys, to }` once.
pub(super) fn freeform_view() {
    // filmcraft: crates/ui-egui/src/panels/project_views.rs:1014 (Freeform: cards, snap lines, stacks, Esc cancels).
    todo!()
}
