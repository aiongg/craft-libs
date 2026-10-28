//! Which gestures' controls were drawn, per viewport: how a gesture whose control is gone is
//! found, and how it ends.

use egui::{Context, Id, ViewportId};

use super::{Ending, GestureId};

/// One active gesture: the viewport its control is drawn in, the last pass of that viewport that
/// drew it, and how it ends if it is interrupted.
#[derive(Clone, Copy, Debug)]
struct Entry {
    id: GestureId,
    viewport: ViewportId,
    pass: u64,
    ending: Ending,
}

/// The active gestures of a context. Small: one entry per gesture under way.
#[derive(Clone, Debug, Default)]
pub(super) struct Live {
    entries: Vec<Entry>,
}

/// A gesture's state as seen from the viewport running now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Judged {
    /// Its control was drawn in this pass of this viewport.
    Live,
    /// This viewport's pass ran its controls without it: the gesture is over and ends so.
    Removed(Ending),
    /// It belongs to another viewport, which judges it; `live` unless that viewport finished a
    /// pass without it.
    Elsewhere { live: bool },
}

impl Live {
    /// Records that `id`'s control was drawn in `pass` of `viewport`, and how the gesture ends if
    /// interrupted. Drops entries whose viewport has finished a pass without them and has had
    /// the pass after it to judge them.
    pub(super) fn mark(&mut self, id: GestureId, viewport: ViewportId, pass: u64, ending: Ending, pass_for: impl Fn(ViewportId) -> u64) {
        self.entries.retain(|e| e.id == id || e.pass + 1 >= pass_for(e.viewport));
        let entry = Entry { id, viewport, pass, ending };
        match self.entries.iter_mut().find(|e| e.id == id) {
            Some(e) => *e = entry,
            None => self.entries.push(entry),
        }
    }

    pub(super) fn forget(&mut self, id: GestureId) {
        self.entries.retain(|e| e.id != id);
    }

    /// Judges `id` from `current`, the viewport running now, after its controls. A gesture with no
    /// entry was never drawn, or was dropped long ago: removed, and cancelled.
    pub(super) fn judge(&self, id: GestureId, current: ViewportId, pass_for: impl Fn(ViewportId) -> u64) -> Judged {
        let Some(e) = self.entries.iter().find(|e| e.id == id) else { return Judged::Removed(Ending::Cancel) };
        let pass = pass_for(e.viewport);
        match e.viewport == current {
            true if e.pass == pass => Judged::Live,
            true => Judged::Removed(e.ending),
            // Another viewport is mid-pass (its count is this pass) or done (one past it).
            false => Judged::Elsewhere { live: e.pass + 1 >= pass },
        }
    }

    pub(super) fn ending(&self, id: GestureId) -> Option<Ending> {
        self.entries.iter().find(|e| e.id == id).map(|e| e.ending)
    }

    /// A copy: judging reads pass numbers, which egui's data lock can't be held for.
    fn get(ctx: &Context) -> Self {
        ctx.data(|d| d.get_temp::<Self>(Id::unique(LIVE_GESTURES))).unwrap_or_default()
    }

    fn put(self, ctx: &Context) {
        ctx.data_mut(|d| d.insert_temp(Id::unique(LIVE_GESTURES), self));
    }
}

const LIVE_GESTURES: &str = "craft_ui::edit::live_gestures";

/// Marks gesture `id` live in the current pass of the current viewport.
pub(super) fn mark(ctx: &Context, id: GestureId, ending: Ending) {
    let (viewport, pass) = (ctx.viewport_id(), ctx.cumulative_pass_nr());
    let mut live = Live::get(ctx);
    live.mark(id, viewport, pass, ending, |v| ctx.cumulative_pass_nr_for(v));
    live.put(ctx);
}

/// Drops `id`'s entry: it ended.
pub(super) fn forget(ctx: &Context, id: GestureId) {
    let mut live = Live::get(ctx);
    live.forget(id);
    live.put(ctx);
}

pub(super) fn judge(ctx: &Context, id: GestureId) -> Judged {
    Live::get(ctx).judge(id, ctx.viewport_id(), |v| ctx.cumulative_pass_nr_for(v))
}

pub(super) fn ending(ctx: &Context, id: GestureId) -> Option<Ending> {
    Live::get(ctx).ending(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_gesture_is_judged_against_the_passes_of_its_own_viewport() {
        let child = ViewportId::from_hash_of("child");
        let (gesture, other) = (GestureId(1), GestureId(2));
        let ending = Ending::Commit { changed: true };
        let mut live = Live::default();
        // The root viewport draws the gesture's control in its pass 3; the child is at pass 10.
        let passes = |root: u64| move |v: ViewportId| if v == ViewportId::ROOT { root } else { 10 };
        live.mark(gesture, ViewportId::ROOT, 3, ending, passes(3));

        assert_eq!(live.judge(gesture, ViewportId::ROOT, passes(3)), Judged::Live);
        // The child's passes don't count for it, whether the root is mid-pass or done with it.
        assert_eq!(live.judge(gesture, child, passes(3)), Judged::Elsewhere { live: true });
        assert_eq!(live.judge(gesture, child, passes(4)), Judged::Elsewhere { live: true });
        // The root's pass 4 ran without it.
        assert_eq!(live.judge(gesture, ViewportId::ROOT, passes(4)), Judged::Removed(ending));
        assert_eq!(live.judge(gesture, child, passes(5)), Judged::Elsewhere { live: false });

        // Many child passes keep it; the root's pass after the one that judged it drops it.
        live.mark(other, child, 11, Ending::Cancel, passes(4));
        assert_eq!(live.ending(gesture), Some(ending));
        live.mark(other, child, 12, Ending::Cancel, passes(5));
        assert_eq!(live.ending(gesture), None);
        assert_eq!(live.judge(gesture, ViewportId::ROOT, passes(5)), Judged::Removed(Ending::Cancel));
    }
}
