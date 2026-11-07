//! Layout memory: sizes one pass measures and the next pass places by, and the one discard pass a change asks for.

use crate::runtime::egui::{pass, sizing};
use crate::runtime::frame::{self, PerViewport};
use craft_behavior::layout_memory::{SizeTween, measure_tolerance, measured_change_asks_for_discard};
use egui::{Context, Id, Vec2};
use std::collections::HashMap;

/// What an entry holds.
#[derive(Clone, Copy, Debug)]
enum Value {
    Size(Vec2),
    #[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "size tweens are used by the body step"))]
    Tween(SizeTween),
}

impl Value {
    fn at(self, now: f64) -> Vec2 {
        match self {
            Value::Size(size) => size,
            Value::Tween(tween) => tween.at(now),
        }
    }
}

#[derive(Clone, Debug, Default)]
struct Entry {
    value: Option<Value>,
    /// The pass that read the entry for a placement, and what it read.
    used: Option<(u64, Option<Vec2>)>,
    /// The frame the size last changed in.
    changed: Option<u64>,
    /// The last frame that read or wrote the entry.
    touched: u64,
}

#[derive(Default)]
struct Entries {
    entries: HashMap<Id, Entry>,
    ask: bool,
    asked: Option<u64>,
    /// Blocks placed without an entry so far, in every pass of the viewport.
    unknown: u64,
}

/// The pass, frame and clock of the running viewport, read before the service lock is taken.
struct Moment {
    viewport: egui::ViewportId,
    pass: u64,
    frame: u64,
    now: f64,
}

impl Moment {
    fn of(ctx: &Context) -> Moment {
        let viewport = pass::current_viewport(ctx);
        Moment { viewport, pass: pass::pass_nr(ctx, viewport), frame: pass::frame_nr(ctx, viewport), now: frame::clock(ctx) }
    }
}

fn with<R>(ctx: &Context, moment: &Moment, f: impl FnOnce(&mut Entries) -> R) -> R {
    PerViewport::<Entries>::with(ctx, moment.viewport, f)
}

/// The size entry under `id` as this pass places by it. A placement that read the entry (`None` on a block's
/// first appearance) compares it with the real size at `record`.
pub(crate) fn read(ctx: &Context, id: Id) -> Option<Vec2> {
    let moment = Moment::of(ctx);
    with(ctx, &moment, |memory| {
        let entry = memory.entries.entry(id).or_default();
        entry.touched = moment.frame;
        let value = entry.value.map(|value| value.at(moment.now));
        if entry.used.is_none_or(|(pass, _)| pass != moment.pass) {
            entry.used = Some((moment.pass, value));
        }
        value
    })
}

/// Notes that a block took its place without a size entry, which leaves the size of whatever contains it unknown.
pub(crate) fn note_unknown(ctx: &Context) {
    let moment = Moment::of(ctx);
    with(ctx, &moment, |memory| memory.unknown += 1);
}

/// How many blocks have been placed without an entry; a container that sees it grow while its content runs
/// measured that content with a placeholder.
pub(crate) fn unknown_count(ctx: &Context) -> u64 {
    let moment = Moment::of(ctx);
    with(ctx, &moment, |memory| memory.unknown)
}

/// The size entry under `id` without a placement using it.
#[cfg(feature = "testing")]
pub(crate) fn peek(ctx: &Context, id: Id) -> Option<Vec2> {
    peeking(ctx, |read| read(id))
}

/// Runs `f` with a reader of the size entries that takes no placement and fixes the pass and clock once, for a
/// caller that reads many entries.
pub(crate) fn peeking<R>(ctx: &Context, f: impl FnOnce(&dyn Fn(Id) -> Option<Vec2>) -> R) -> R {
    let moment = Moment::of(ctx);
    with(ctx, &moment, |memory| f(&|id| memory.entries.get(&id).and_then(|entry| entry.value).map(|value| value.at(moment.now))))
}

/// Records the size a block measured this pass. A change asks for a discard pass only under the tolerance
/// rule: this pass placed with the old value, and the size did not also change in the previous frame.
pub(crate) fn record(ctx: &Context, id: Id, real: Vec2) {
    let moment = Moment::of(ctx);
    let tolerance = measure_tolerance(crate::runtime::egui::scale::pixels_per_point(ctx));
    with(ctx, &moment, |memory| {
        let entry = memory.entries.entry(id).or_default();
        entry.touched = moment.frame;
        let placed = entry.used.take().filter(|(pass, _)| *pass == moment.pass).map(|(_, value)| value);
        let changed_last_frame = entry.changed.is_some_and(|frame| frame + 1 == moment.frame);
        let previous = entry.value.map(|value| value.at(moment.now));
        if let Some(recorded) = placed
            && measured_change_asks_for_discard(recorded, real, changed_last_frame, tolerance)
        {
            memory.ask = true;
        }
        let entry = memory.entries.entry(id).or_default();
        let differs = previous.is_none_or(|old| (old - real).abs().max_elem() > tolerance);
        if differs {
            entry.changed = Some(moment.frame);
        }
        entry.value = Some(Value::Size(if differs { real } else { previous.unwrap_or(real) }));
    });
}

/// Starts a size tween under `id` that readers evaluate at their clock. It asks for no discard pass: the
/// size changes by design while it runs.
#[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "size tweens are used by the body step"))]
pub(crate) fn tween(ctx: &Context, id: Id, from: Vec2, to: Vec2, duration: f64) {
    let moment = Moment::of(ctx);
    with(ctx, &moment, |memory| {
        let entry = memory.entries.entry(id).or_default();
        entry.touched = moment.frame;
        entry.value = Some(Value::Tween(SizeTween::new(from, to, moment.now, duration)));
    });
}

/// Whether a tween under `id` has not reached its end.
#[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "size tweens are used by the body step"))]
pub(crate) fn is_animating(ctx: &Context, id: Id) -> bool {
    let moment = Moment::of(ctx);
    with(ctx, &moment, |memory| {
        memory.entries.get(&id).is_some_and(|entry| matches!(entry.value, Some(Value::Tween(tween)) if !tween.is_done(moment.now)))
    })
}

/// Asks the end hook for the frame's discard pass, as a change of measured size does.
pub(crate) fn ask_discard(ctx: &Context) {
    let moment = Moment::of(ctx);
    with(ctx, &moment, |memory| memory.ask = true);
}

/// The end hook: asks egui for at most one discard pass per frame, then drops the entries no pass touched
/// in the last frame.
pub(crate) fn end_hook(ctx: &Context) {
    let moment = Moment::of(ctx);
    let ask = with(ctx, &moment, |memory| {
        let ask = std::mem::take(&mut memory.ask) && memory.asked != Some(moment.frame);
        if ask {
            memory.asked = Some(moment.frame);
        }
        memory.entries.retain(|_, entry| entry.touched + 1 >= moment.frame);
        ask
    });
    if ask {
        sizing::request_discard(ctx, "craft_ui: layout memory changed");
    }
}
