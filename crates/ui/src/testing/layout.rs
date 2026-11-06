//! The layout services the trials check: bodies, measured blocks, the label column, `measure_slot` and
//! `arrange`, with placeholder children that take a given size.

use crate::layout::{BodyLayout, Show, run_body};
use crate::runtime::egui::fixture;
use crate::runtime::layout_memory;
use crate::runtime::memory::sub;
use craft_behavior::Proposal;
use egui::{Id, Rect, Response, Ui, Vec2};

/// Runs a body laid out by `layout` under `key` and places one child per entry of `sizes` in flow, as step 6
/// does. Returns each child's rect.
pub fn body(ui: &mut Ui, key: &'static str, layout: &BodyLayout, sizes: &[Vec2]) -> Vec<Rect> {
    let id = sub(ui.scope_id(), key);
    run_body(ui, id, layout, Vec2::ZERO, |ui, cells| sizes.iter().map(|size| cells.place(ui, *size)).collect())
}

/// A block that placed itself from `key`'s entry of last pass and measured `real` this pass. Returns the size
/// it was placed with, `None` on its first appearance.
pub fn measured_block(ui: &Ui, key: &'static str, real: Vec2) -> Option<Vec2> {
    let id = sub(ui.scope_id(), key);
    let placed = layout_memory::read(ui.ctx(), id);
    layout_memory::record(ui.ctx(), id, real);
    placed
}

/// Starts a size tween under `key`.
pub fn start_tween(ui: &Ui, key: &'static str, from: Vec2, to: Vec2, duration: f64) {
    layout_memory::tween(ui.ctx(), sub(ui.scope_id(), key), from, to, duration);
}

/// The size under `key` as a placement reads it.
pub fn read_size(ui: &Ui, key: &'static str) -> Option<Vec2> {
    layout_memory::read(ui.ctx(), sub(ui.scope_id(), key))
}

/// The size under `key` without a placement using it.
pub fn peek_size(ui: &Ui, key: &'static str) -> Option<Vec2> {
    layout_memory::peek(ui.ctx(), sub(ui.scope_id(), key))
}

/// Whether the tween under `key` is still running.
pub fn is_animating(ui: &Ui, key: &'static str) -> bool {
    layout_memory::is_animating(ui.ctx(), sub(ui.scope_id(), key))
}

/// The id a keyed entry of this `Ui` lives under.
pub fn entry_id(ui: &Ui, key: &'static str) -> Id {
    sub(ui.scope_id(), key)
}

/// Shows `text` wrapped to the available width of a sizing slot, for `measure_slot` trials.
pub fn wrapped_text(ui: &mut Ui, text: &str) {
    fixture::wrapped_text(ui, text);
}

/// The default font definitions in another order.
pub fn reordered_fonts() -> egui::FontDefinitions {
    fixture::reordered_fonts()
}

/// A placeholder child of a fixed size.
pub struct Block(Vec2);

impl Block {
    /// A child that takes `size`.
    pub const fn new(size: Vec2) -> Self {
        Block(size)
    }
}

impl Show for Block {
    fn measure_in(&mut self, _: &mut Ui, _: Id, _: Proposal) -> Vec2 {
        self.0
    }

    fn show_at(&mut self, ui: &mut Ui, id: Id, rect: Rect) -> Response {
        fixture::placeholder(ui, id, rect)
    }
}
