//! Layout memory: sizes one pass measures and the next pass places by, the one discard pass a change asks for, what
//! this pass's measuring runs found, and the measurements bodies keep across passes.
//!
//! A body keeps what its last measuring run recorded, with the inputs it ran under. A later pass whose inputs are the
//! same takes those records instead of running the body's content to measure it. The real run confirms them: the
//! body's content digest (what each component in it measured) and each record its producer checks again
//! (`confirm`) must match what the real run found in the pass that measured. A mismatch drops the kept measurements
//! of every body the real run is inside and asks for the frame's discard pass, so a change shows in its frame.

use crate::runtime::egui::{pass, sizing};
use crate::runtime::frame::{self, PerViewport};
use craft_behavior::layout_memory::{SizeTween, measure_tolerance, measured_change_asks_for_discard};
use egui::{Context, Id, Vec2};
use std::any::Any;
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

/// What a measuring run found for a container (under its id) or one of its regions (under `sub(id, "header")`,
/// `"footer"`, `"end"` and `"controls"`): the room the container was offered and what the region took. Valid only in the egui pass
/// that recorded it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Measured {
    pub room: Vec2,
    pub took: Vec2,
}

impl Measured {
    /// Whether this record serves a run offered `now` along one axis, where it was offered `was` and what it measures
    /// took `took`: the rooms are equal (within `tolerance`), or the axis is not `given`, the old room did not bound
    /// what it took and it fits the room now. The one reuse rule for every record of this pass.
    pub(crate) fn serves(now: f32, was: f32, given: bool, took: f32, tolerance: f32) -> bool {
        now == was || (now - was).abs() <= tolerance || (!given && took < was - tolerance && took <= now + tolerance)
    }
}

/// A record of this pass's measures.
#[derive(Clone, Copy)]
struct Found {
    /// The cumulative pass number that recorded it.
    pass: u64,
    measured: Measured,
    /// Whether it came from a body's kept measurement rather than a run of this pass.
    kept: bool,
}

/// What a body's last measuring run found, kept for later passes.
struct Kept {
    /// The inputs the run measured under.
    key: Box<dyn Any + Send + Sync>,
    /// Every record the run left, its nested content's included; empty once a real run found the content changed.
    records: Vec<(Id, Measured)>,
    /// What the real run of the pass that measured found in the body's content.
    digest: Option<u64>,
    /// The cumulative pass that measured, and the last one that took the records.
    measured_at: u64,
    taken_at: u64,
    /// The frame the content last changed in: content that changes measures every pass.
    moved: Option<u64>,
    touched: u64,
}

/// A real run's content scope: a body's content, or content that is not part of the enclosing body's measure (a
/// collection's rows, an out-of-view row's).
struct Scope {
    body: Option<Id>,
    digest: u64,
}

#[derive(Default)]
struct Entries {
    entries: HashMap<Id, Entry>,
    measured: HashMap<Id, Found>,
    kept: HashMap<Id, Kept>,
    /// The measuring runs of bodies open now, with the records each has left so far.
    measuring: Vec<(Id, Vec<(Id, Measured)>)>,
    /// The content scopes open in the real run.
    scopes: Vec<Scope>,
    /// What a real run found for a record its producer confirms, in the pass the record was measured.
    confirmed: HashMap<Id, (u64, Vec2)>,
    ask: bool,
    asked: Option<u64>,
}

impl Entries {
    /// Drops the kept measurement of every body the real run is inside, and asks for the discard pass.
    fn changed(&mut self, frame: u64) {
        for scope in &self.scopes {
            if let Some(kept) = scope.body.and_then(|body| self.kept.get_mut(&body)) {
                kept.records.clear();
                kept.moved = Some(frame);
            }
        }
        self.ask = true;
    }
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
    if crate::runtime::measure::active(ctx) {
        return peeking(ctx, |read| read(id));
    }
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

/// Records what a measuring run found under `id`, for the rest of this pass and in the kept measurement of each body
/// whose measuring run is open.
pub(crate) fn record_measured(ctx: &Context, id: Id, measured: Measured) {
    let moment = Moment::of(ctx);
    let pass = ctx.cumulative_pass_nr();
    with(ctx, &moment, |memory| {
        memory.measured.insert(id, Found { pass, measured, kept: false });
        if let Some((_, records)) = memory.measuring.last_mut() {
            records.push((id, measured));
        }
    });
}

/// Records what a measuring run found under `id` for the rest of this pass only: a record its producer cannot confirm
/// in a real run, so a later pass measures it again.
pub(crate) fn record_measured_unkept(ctx: &Context, id: Id, measured: Measured) {
    let moment = Moment::of(ctx);
    let pass = ctx.cumulative_pass_nr();
    with(ctx, &moment, |memory| memory.measured.insert(id, Found { pass, measured, kept: false }));
}

/// What a measuring run of this pass found under `id`, or a kept measurement this pass took; `None` when neither.
pub(crate) fn measured(ctx: &Context, id: Id) -> Option<Measured> {
    let moment = Moment::of(ctx);
    let pass = ctx.cumulative_pass_nr();
    with(ctx, &moment, |memory| memory.measured.get(&id).filter(|found| found.pass == pass).map(|found| found.measured))
}

/// Whether a run of this pass, not a kept measurement, recorded what serves under `id`.
pub(crate) fn run_found(ctx: &Context, id: Id) -> bool {
    let moment = Moment::of(ctx);
    let pass = ctx.cumulative_pass_nr();
    with(ctx, &moment, |memory| memory.measured.get(&id).is_some_and(|found| found.pass == pass && !found.kept))
}

/// Opens the measuring run of `body`: what it records until `end_measure` becomes its kept measurement.
pub(crate) fn begin_measure(ctx: &Context, body: Id) {
    let moment = Moment::of(ctx);
    with(ctx, &moment, |memory| memory.measuring.push((body, Vec::new())));
}

/// The bodies whose measuring runs are open, innermost last.
pub(crate) fn measuring_bodies(ctx: &Context) -> Vec<Id> {
    let moment = Moment::of(ctx);
    with(ctx, &moment, |memory| memory.measuring.iter().map(|(body, _)| *body).collect())
}

/// Closes the measuring run of `body` and keeps what it recorded under the inputs `key` it ran under.
pub(crate) fn end_measure<K: PartialEq + Send + Sync + 'static>(ctx: &Context, body: Id, key: K) {
    let moment = Moment::of(ctx);
    let pass = ctx.cumulative_pass_nr();
    with(ctx, &moment, |memory| {
        let Some(at) = memory.measuring.iter().rposition(|(open, _)| *open == body) else { return };
        let (_, records) = memory.measuring.remove(at);
        if let Some((_, outer)) = memory.measuring.last_mut() {
            outer.extend_from_slice(&records);
        }
        let last = memory.kept.remove(&body);
        let (digest, moved) = last.map_or((None, None), |last| (last.digest, last.moved));
        memory.kept.insert(body, Kept { key: Box::new(key), records, digest, measured_at: pass, taken_at: 0, moved, touched: moment.frame });
    });
}

/// Whether `body`, offered the inputs `key`, takes its kept measurement in this pass instead of measuring: the inputs
/// are the ones it measured under, no real run since found its content changed, its content did not change in this
/// frame or the last, and this pass is not a rerun that shows something the measuring pass did not see. When it does,
/// the records serve this pass as if its measuring run had left them.
pub(crate) fn kept_serves<K: PartialEq + Send + Sync + 'static>(ctx: &Context, body: Id, key: &K) -> bool {
    // A rerun for an edit or a held update may show values the content read before them: every body measures again.
    if sizing::rerun_beyond_layout(ctx) {
        return false;
    }
    let moment = Moment::of(ctx);
    let pass = ctx.cumulative_pass_nr();
    with(ctx, &moment, |memory| {
        let Some(kept) = memory.kept.get_mut(&body) else { return false };
        let settled = kept.moved.is_none_or(|frame| frame + 1 < moment.frame);
        if !settled || kept.records.is_empty() || kept.key.downcast_ref::<K>() != Some(key) {
            return false;
        }
        kept.taken_at = pass;
        kept.touched = moment.frame;
        for (id, measured) in &kept.records {
            memory.measured.insert(*id, Found { pass, measured: *measured, kept: true });
        }
        if let Some((_, outer)) = memory.measuring.last_mut() {
            outer.extend_from_slice(&kept.records);
        }
        true
    })
}

/// Opens a content scope of the real run: `body`'s content, or (`None`) content the enclosing body's measure does not
/// count component by component.
pub(crate) fn open_content(ctx: &Context, body: Option<Id>) {
    let moment = Moment::of(ctx);
    with(ctx, &moment, |memory| memory.scopes.push(Scope { body, digest: 0xcbf2_9ce4_8422_2325 }));
}

/// Counts what component `id` measured in the real run into the innermost content scope.
pub(crate) fn fold(ctx: &Context, id: Id, size: Vec2) {
    let moment = Moment::of(ctx);
    with(ctx, &moment, |memory| {
        if let Some(scope) = memory.scopes.last_mut() {
            // Steps of 1/64 point: the same content measures the same in every pass.
            let step = |value: f32| if value.is_finite() { (value * 64.0).round() as i64 as u64 } else { u64::MAX };
            for part in [id.value(), step(size.x), step(size.y)] {
                scope.digest = (scope.digest ^ part).wrapping_mul(0x0100_0000_01b3);
            }
        }
    });
}

/// Closes the innermost content scope. For a body, its digest confirms the kept measurement the pass took, or becomes
/// the one later passes confirm against when the pass measured.
pub(crate) fn close_content(ctx: &Context) {
    let moment = Moment::of(ctx);
    let pass = ctx.cumulative_pass_nr();
    with(ctx, &moment, |memory| {
        let Some(scope) = memory.scopes.pop() else { return };
        let Some(kept) = scope.body.and_then(|body| memory.kept.get_mut(&body)) else { return };
        let same = kept.digest == Some(scope.digest);
        kept.digest = Some(scope.digest);
        if kept.measured_at == pass {
            if !same {
                kept.moved = Some(moment.frame);
            }
        } else if kept.taken_at == pass && !same {
            kept.records.clear();
            kept.moved = Some(moment.frame);
            memory.changed(moment.frame);
        }
    });
}

/// A real run's check of the record under `id`: `found` is what the producer found in this run, from the content
/// itself and not from the record. In the pass that measured the record it becomes what later passes compare with; in
/// a pass that took the record from a kept measurement a difference drops the kept measurements the real run is in.
pub(crate) fn confirm(ctx: &Context, id: Id, found: Vec2) {
    let moment = Moment::of(ctx);
    let pass = ctx.cumulative_pass_nr();
    let tolerance = measure_tolerance(crate::runtime::egui::scale::pixels_per_point(ctx));
    with(ctx, &moment, |memory| {
        let Some(record) = memory.measured.get(&id).filter(|record| record.pass == pass).copied() else { return };
        let last = memory.confirmed.insert(id, (moment.frame, found));
        if record.kept && last.is_none_or(|(_, last)| (last - found).abs().max_elem() > tolerance) {
            memory.changed(moment.frame);
        }
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

/// Drops every kept measurement, so the next pass measures each body as a first pass would.
#[cfg(feature = "testing")]
pub fn forget_kept(ctx: &Context) {
    let moment = Moment::of(ctx);
    with(ctx, &moment, |memory| memory.kept.clear());
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
        memory.kept.retain(|_, kept| kept.touched + 1 >= moment.frame);
        memory.confirmed.retain(|_, (frame, _)| *frame + 1 >= moment.frame);
        memory.measured.clear();
        memory.measuring.clear();
        memory.scopes.clear();
        ask
    });
    if ask {
        sizing::request_discard(ctx, sizing::LAYOUT_RERUN);
    }
}
