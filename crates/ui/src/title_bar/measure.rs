//! What the last pass measured, for the passes that cannot measure it themselves: the free gap
//! between the two groups of the bar, which [`BarSlots::gap`](super::BarSlots::gap) and the modal
//! drag overlay use. The title uses the current pass's edges: both groups are laid out before it
//! is painted.

use egui::{Context, Id, Rect};

/// How much the bar's width may differ from the pass that measured the gap. A resize drag changes
/// it on every frame, and then the gap is not where it was.
const WIDTH_TOLERANCE: f32 = 0.5;

#[derive(Clone, Copy, Debug)]
struct Measured {
    pass: u64,
    width: f32,
    gap: Rect,
}

fn key(scope: Id) -> Id {
    scope.with("measure")
}

/// Keeps the gap of the bar of the window `scope` stands for, drawn at `width` on this pass.
pub(super) fn store(ctx: &Context, scope: Id, width: f32, gap: Rect) {
    let pass = ctx.cumulative_pass_nr();
    ctx.data_mut(|d| d.insert_temp(key(scope), Measured { pass, width, gap }));
}

/// The gap measured on this pass or the one before, if the bar is still `width` wide. Nothing
/// after a pass that drew no titled bar, or after the bar resized.
pub(super) fn last(ctx: &Context, scope: Id, width: f32) -> Option<Rect> {
    let measured = ctx.data(|d| d.get_temp::<Measured>(key(scope)))?;
    let fresh = ctx.cumulative_pass_nr().saturating_sub(measured.pass) <= 1;
    (fresh && (measured.width - width).abs() <= WIDTH_TOLERANCE).then_some(measured.gap)
}

/// The gap of the last pass, whatever the bar's width is now.
pub(super) fn drawn(ctx: &Context, scope: Id) -> Option<Rect> {
    let measured = ctx.data(|d| d.get_temp::<Measured>(key(scope)))?;
    (ctx.cumulative_pass_nr().saturating_sub(measured.pass) <= 1).then_some(measured.gap)
}
