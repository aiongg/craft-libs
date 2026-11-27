//! `LabelColumn` rows: the label aligned to the control's first baseline from its last run.

use crate::contract::{Context, Id, Ui};
use crate::runtime::memory::{sub, with_session};
use craft_behavior::{Flow, LabelColumn, LabelPlacement};
use craft_ui_rt::Vec2;

/// A distance from a control's top its last run recorded; `None` before the first.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Line(Option<f32>);

/// Places a label of `label` size with baseline `label_baseline` and the control of the row `id` in `flow`'s
/// rect. The label's baseline follows the control baseline `record_control_baseline` stored at the control's last
/// run; on the first appearance the tops align, and the control's record builds the row again in this frame.
pub fn place_label(ui: &Ui, id: Id, column: &LabelColumn, flow: Flow, label: Vec2, label_baseline: f32) -> LabelPlacement {
    let control_baseline = control_baseline(ui.ctx(), id, label, label_baseline);
    column.place(flow, label, label_baseline, control_baseline)
}

/// Stores the distance from the top of the control of the row `id` to its first baseline, for the label step's
/// `place_label`. A baseline other than the one stored builds the calling scope again in this frame, so the label
/// placed from the old one moves.
pub fn record_control_baseline(ctx: &Context, id: Id, baseline: f32) {
    record(ctx, sub(id, "baseline"), baseline);
}

/// Stores the centre line, from its top, of the control of the row `id` that paints no text, in place of a baseline
/// (`control_baseline`). Builds again as `record_control_baseline` does.
pub fn record_control_centre(ctx: &Context, id: Id, centre: f32) {
    record(ctx, sub(id, "centre"), centre);
}

fn record(ctx: &Context, key: Id, value: f32) {
    let moved = with_session::<Line, _>(ctx, key, |line| std::mem::replace(line, Line(Some(value))) != Line(Some(value)));
    if moved {
        ctx.request_repaint();
    }
}

/// The control's first baseline from its top, as `record_control_baseline` stored it. A control that paints no text
/// stored its centre line instead; the label's baseline then sits where its text is centred on that line.
pub fn control_baseline(ctx: &Context, id: Id, label: Vec2, label_baseline: f32) -> Option<f32> {
    let line = |tag: &'static str| with_session::<Line, _>(ctx, sub(id, tag), |line| line.0);
    line("baseline").or_else(|| line("centre").map(|centre| centre + label_baseline - label.y / 2.0))
}
