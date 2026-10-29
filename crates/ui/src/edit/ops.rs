//! What the user did to a part's value, as operations: a composite queues them while the part's
//! event waits for its turn, and applies them to its value of that turn
//! ([`Edits::push`](super::parts::Edits::push)).
//!
//! A control whose edit response is public ([`Slider`](crate::controls::Slider),
//! [`NumberField`](crate::controls::NumberField)) reports its operations through the context
//! instead ([`report`], [`take`]).

use egui::{Context, Id, Modifiers, ViewportId};

/// An operation on a one-value control's value: a slider's, a number field's, a dial's angle.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ScalarOp {
    /// A change by this much, in value units: arrow and Page keys, the wheel, a screen reader's
    /// Increment and Decrement.
    By(f64),
    /// A number field's steps, each with the modifiers held for it and its signed count of steps
    /// (a key press is one, a wheel gesture its notches): Shift and Cmd keep the field's rounding.
    Presses(Vec<(Modifiers, f64)>),
    /// An absolute value: a screen reader's SetValue, a typed value, a reset to the default.
    Set(f64),
    /// Home or End: the bound of the value the operation applies to, which for a thumb with an
    /// order is its neighbour plus the gap.
    ToBound(Bound),
}

/// Which bound [`ScalarOp::ToBound`] goes to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Bound {
    Low,
    High,
}

/// Operations a control reported in a pass of a viewport.
#[derive(Clone)]
struct Reported<O> {
    viewport: ViewportId,
    pass: u64,
    ops: Vec<O>,
}

fn key(id: Id) -> Id {
    id.with("craft_ui::edit::ops")
}

/// Reports control `id`'s operations of this pass, for the composite that draws it as a part to
/// [`take`]. A second report in the same pass adds to the first.
pub(crate) fn report<O: Clone + Send + Sync + 'static>(ctx: &Context, id: Id, mut ops: Vec<O>) {
    let (viewport, pass) = (ctx.viewport_id(), ctx.cumulative_pass_nr());
    ctx.data_mut(|d| {
        let reported = d.get_temp_mut_or_insert_with(key(id), || Reported::<O> { viewport, pass, ops: Vec::new() });
        if (reported.viewport, reported.pass) != (viewport, pass) {
            *reported = Reported { viewport, pass, ops: Vec::new() };
        }
        reported.ops.append(&mut ops);
    });
}

/// The operations control `id` [reported](report) in this pass, taken: a second call returns
/// none.
pub(crate) fn take<O: Clone + Send + Sync + 'static>(ctx: &Context, id: Id) -> Vec<O> {
    let (viewport, pass) = (ctx.viewport_id(), ctx.cumulative_pass_nr());
    let reported = ctx.data_mut(|d| {
        let reported = d.get_temp::<Reported<O>>(key(id));
        d.remove::<Reported<O>>(key(id));
        reported
    });
    reported.filter(|r| (r.viewport, r.pass) == (viewport, pass)).map_or_else(Vec::new, |r| r.ops)
}
