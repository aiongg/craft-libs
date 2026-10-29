//! What the bar measured, for the code that runs after it or on the next pass: the free gap
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
    /// The bar is the window's title bar: it has a drag area (not under `Frame::Native`, in full
    /// screen or as a preview).
    titled: bool,
}

fn key(scope: Id) -> Id {
    scope.with("measure")
}

/// Keeps the gap of the bar of the window `scope` stands for, drawn at `width` on this pass.
pub(super) fn store(ctx: &Context, scope: Id, width: f32, gap: Rect, titled: bool) {
    let pass = ctx.cumulative_pass_nr();
    ctx.data_mut(|d| d.insert_temp(key(scope), Measured { pass, width, gap, titled }));
}

/// The gap measured on the pass before this one or on this one, if the bar is still `width` wide.
/// Nothing after a pass that drew no bar, or after the bar resized.
pub(super) fn last(ctx: &Context, scope: Id, width: f32) -> Option<Rect> {
    let measured = ctx.data(|d| d.get_temp::<Measured>(key(scope)))?;
    let fresh = ctx.cumulative_pass_nr().saturating_sub(measured.pass) <= 1;
    (fresh && (measured.width - width).abs() <= WIDTH_TOLERANCE).then_some(measured.gap)
}

/// The gap of a titled bar drawn on this very pass, whatever the bar's width is: the drag overlay
/// runs after the windows are drawn, and an earlier pass's gap may no longer be where the bar is.
pub(super) fn titled_gap(ctx: &Context, scope: Id) -> Option<Rect> {
    let measured = ctx.data(|d| d.get_temp::<Measured>(key(scope)))?;
    (measured.titled && measured.pass == ctx.cumulative_pass_nr()).then_some(measured.gap)
}
