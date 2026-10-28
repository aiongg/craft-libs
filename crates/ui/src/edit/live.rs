//! Which gestures' controls were drawn, per viewport: how a gesture whose control is gone is
//! found, and how it ends.

use std::any::Any;
use std::sync::Arc;

use egui::{Context, Id, ViewportId};

use super::{Ending, GestureId};

/// A gesture's value, whatever the control's value type.
pub(super) type Value = Arc<dyn Any + Send + Sync>;

/// One active gesture: the viewport its control is drawn in, the last pass of that viewport that
/// drew it, how it ends if it is interrupted, the value it ends with when the host hasn't seen
/// it, whether its control had the keyboard focus in that pass, and whether the host has ended
/// it before its control did.
#[derive(Clone)]
struct Entry {
    id: GestureId,
    viewport: ViewportId,
    pass: u64,
    ending: Ending,
    unreported: Option<Value>,
    focused: bool,
    ended: bool,
}

impl std::fmt::Debug for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Entry")
            .field("id", &self.id)
            .field("viewport", &self.viewport)
            .field("pass", &self.pass)
            .field("ending", &self.ending)
            .field("unreported", &self.unreported.is_some())
            .field("focused", &self.focused)
            .field("ended", &self.ended)
            .finish()
    }
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
    /// Records `entry`: its gesture's control was drawn in the entry's pass. Drops entries whose
    /// viewport has finished a pass without them and has had the pass after it to judge them. A
    /// gesture the host has ended stays ended until its control learns of it.
    fn mark(&mut self, entry: Entry, pass_for: impl Fn(ViewportId) -> u64) {
        self.entries.retain(|e| e.id == entry.id || e.pass + 1 >= pass_for(e.viewport));
        match self.entries.iter_mut().find(|e| e.id == entry.id) {
            Some(e) if e.ended => *e = Entry { focused: false, ended: true, ..entry },
            Some(e) => *e = entry,
            None => self.entries.push(entry),
        }
    }

    pub(super) fn forget(&mut self, id: GestureId) {
        self.entries.retain(|e| e.id != id);
    }

    /// Marks `id` ended by the host: no longer live or focused. The entry stays until its control
    /// has loaded once more, which learns of it with [`ended`](Live::ended).
    fn end(&mut self, id: GestureId) {
        if let Some(e) = self.entries.iter_mut().find(|e| e.id == id) {
            e.ended = true;
            e.focused = false;
        }
    }

    fn ended(&self, id: GestureId) -> bool {
        self.entries.iter().any(|e| e.id == id && e.ended)
    }

    /// Judges `id` from `current`, the viewport running now, after its controls. A gesture with no
    /// entry was never drawn, or was dropped long ago: removed, and cancelled.
    pub(super) fn judge(&self, id: GestureId, current: ViewportId, pass_for: impl Fn(ViewportId) -> u64) -> Judged {
        let Some(e) = self.entries.iter().find(|e| e.id == id) else { return Judged::Removed(Ending::Cancel) };
        let pass = pass_for(e.viewport);
        if e.ended {
            return match e.viewport == current {
                true => Judged::Removed(e.ending),
                false => Judged::Elsewhere { live: false },
            };
        }
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

    pub(super) fn unreported(&self, id: GestureId) -> Option<Value> {
        self.entries.iter().find(|e| e.id == id).and_then(|e| e.unreported.clone())
    }

    /// The newest gesture of `current` whose control had the focus in this pass or the last.
    pub(super) fn focused(&self, current: ViewportId, pass: u64) -> Option<GestureId> {
        self.entries.iter().filter(|e| e.focused && e.viewport == current && e.pass + 1 >= pass).map(|e| e.id).max()
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

/// Marks gesture `id` live in the current pass of the current viewport, ending as `ending` with
/// `unreported` if interrupted; `focused` when its control has the keyboard focus.
pub(super) fn mark(ctx: &Context, id: GestureId, ending: Ending, unreported: Option<Value>, focused: bool) {
    let (viewport, pass) = (ctx.viewport_id(), ctx.cumulative_pass_nr());
    let mut live = Live::get(ctx);
    live.mark(Entry { id, viewport, pass, ending, unreported, focused, ended: false }, |v| ctx.cumulative_pass_nr_for(v));
    live.put(ctx);
}

/// Drops `id`'s entry: it ended.
pub(super) fn forget(ctx: &Context, id: GestureId) {
    let mut live = Live::get(ctx);
    live.forget(id);
    live.put(ctx);
}

/// Marks `id` ended by the host before its control ended it.
pub(super) fn end(ctx: &Context, id: GestureId) {
    let mut live = Live::get(ctx);
    live.end(id);
    live.put(ctx);
}

/// Whether the host ended `id` before its control did.
pub(super) fn ended(ctx: &Context, id: GestureId) -> bool {
    Live::get(ctx).ended(id)
}

pub(super) fn judge(ctx: &Context, id: GestureId) -> Judged {
    Live::get(ctx).judge(id, ctx.viewport_id(), |v| ctx.cumulative_pass_nr_for(v))
}

pub(super) fn ending(ctx: &Context, id: GestureId) -> Option<Ending> {
    Live::get(ctx).ending(id)
}

pub(super) fn unreported(ctx: &Context, id: GestureId) -> Option<Value> {
    Live::get(ctx).unreported(id)
}

pub(super) fn focused(ctx: &Context) -> Option<GestureId> {
    Live::get(ctx).focused(ctx.viewport_id(), ctx.cumulative_pass_nr())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: GestureId, viewport: ViewportId, pass: u64, ending: Ending) -> Entry {
        Entry { id, viewport, pass, ending, unreported: None, focused: false, ended: false }
    }

    #[test]
    fn a_gesture_is_judged_against_the_passes_of_its_own_viewport() {
        let child = ViewportId::from_hash_of("child");
        let (gesture, other) = (GestureId(1), GestureId(2));
        let ending = Ending::Commit { changed: true };
        let mut live = Live::default();
        // The root viewport draws the gesture's control in its pass 3; the child is at pass 10.
        let passes = |root: u64| move |v: ViewportId| if v == ViewportId::ROOT { root } else { 10 };
        live.mark(entry(gesture, ViewportId::ROOT, 3, ending), passes(3));

        assert_eq!(live.judge(gesture, ViewportId::ROOT, passes(3)), Judged::Live);
        // The child's passes don't count for it, whether the root is mid-pass or done with it.
        assert_eq!(live.judge(gesture, child, passes(3)), Judged::Elsewhere { live: true });
        assert_eq!(live.judge(gesture, child, passes(4)), Judged::Elsewhere { live: true });
        // The root's pass 4 ran without it.
        assert_eq!(live.judge(gesture, ViewportId::ROOT, passes(4)), Judged::Removed(ending));
        assert_eq!(live.judge(gesture, child, passes(5)), Judged::Elsewhere { live: false });

        // Many child passes keep it; the root's pass after the one that judged it drops it.
        live.mark(entry(other, child, 11, Ending::Cancel), passes(4));
        assert_eq!(live.ending(gesture), Some(ending));
        live.mark(entry(other, child, 12, Ending::Cancel), passes(5));
        assert_eq!(live.ending(gesture), None);
        assert_eq!(live.judge(gesture, ViewportId::ROOT, passes(5)), Judged::Removed(Ending::Cancel));
    }
}
