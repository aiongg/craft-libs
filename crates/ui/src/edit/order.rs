//! The order of inputs that start an edit while another control's typing run has the focus.
//!
//! An input that starts an edit (a press on a composite's part or a canvas overlay, a click on a
//! composite's button, a value the composite makes itself, a part's event while another part
//! owns the composite's gesture) asks the typing run to end and runs in the first pass after the
//! run's ending event has gone out, against the host's value then. A composite reports one edit
//! at a time: such an input never joins another gesture; it waits for that gesture to end.
//!
//! - A **typing run** is a gesture whose control has the keyboard focus, isn't a pointer drag,
//!   and ends when the focus leaves: a text or number field's. A gesture that
//!   [commits on interrupt](super::Gesture::commit_on_interrupt) (a pen draft) is not one.
//! - **Asking a run to end** takes the keyboard focus from it. The control ends the run by its
//!   focus rule on its next [`load`](super::Gesture::load), as for any focus loss: the event is
//!   the one a Tab away from the field would make.
//! - A gesture has **gone out** when its control reported its ending in an earlier pass, or it
//!   stopped being live: its control wasn't drawn in a whole pass, or the host ended it. Its
//!   control's draw order doesn't matter: a run drawn after the pressed part ends in the press
//!   pass, one drawn before it in the next, and the input runs in the pass after either.
//! - A typing run is **under way** until its ending has gone out. A run that ended in this pass
//!   is one to wait for: a click (press and release in one frame) moves the focus off a field
//!   drawn before the clicked control, so the run ends before the input arrives, and the input
//!   runs in the next pass all the same.
//! - Inputs never ask a drag to end: a press can't happen during one, and a screen reader's step
//!   on another part waits for the release.
//!
//! One queue per viewport holds the waiting inputs, each keyed by its holder. The holder keeps
//! the input itself (a press position, a click, a part's event) and calls [`turn`] in every pass
//! it is drawn while it has one; the queue only gives turns. The first entry runs once what it
//! waits for has gone out, one entry a pass. An entry behind one that ran also waits for the
//! gesture that input began (a press that became a drag), without asking it to end. An entry
//! whose holder skipped a pass is dropped, as is the input the holder kept.
//!
//! A holder with several inputs that run one a pass (a composite's queue of held part events)
//! [keeps](keep) its place at the head after each runs: an input that arrives meanwhile, a press
//! on one of its parts among them, waits until the holder [leaves](leave) the queue.
//!
//! Standalone controls don't wait: the [`GestureHost`](super::GestureHost) orders their events
//! (a `Begin` or `Set` while another gesture is under way ends that gesture first).

use egui::{Context, Id};

use super::{GestureId, live};

/// When an input runs, as [`Claim::turn`](crate::canvas::Claim::turn) answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Turn {
    /// Nothing to wait for: the input runs in this pass with this pass's input, and the holder
    /// keeps nothing.
    Now,
    /// The input waits; its holder keeps it and asks again in its next pass.
    Waits,
    /// The input the holder kept from an earlier pass runs now, against the host's value now.
    Runs,
}

/// One waiting input.
#[derive(Clone, Debug)]
struct Entry {
    /// Its holder.
    key: Id,
    /// The gestures it waits for.
    awaits: Vec<GestureId>,
    /// The last pass its holder asked.
    polled: u64,
    /// An input before it ran while it waited.
    behind: bool,
}

/// A viewport's waiting inputs, in their order, and the pass in which one last ran.
#[derive(Clone, Debug, Default)]
struct Queue {
    entries: Vec<Entry>,
    ran: Option<u64>,
}

const QUEUE: &str = "craft_ui::edit::order";

impl Queue {
    fn key(ctx: &Context) -> Id {
        Id::unique(QUEUE).with(ctx.viewport_id())
    }

    fn get(ctx: &Context) -> Self {
        let key = Self::key(ctx);
        ctx.data(|d| d.get_temp::<Self>(key)).unwrap_or_default()
    }

    fn put(self, ctx: &Context) {
        let key = Self::key(ctx);
        ctx.data_mut(|d| d.insert_temp(key, self));
    }
}

/// When the input of holder `key` runs. `own` is the holder's own gesture: its typing run is no
/// reason to wait (a number field's scrub ends its own run). Call it in every pass the holder is
/// drawn while it has an input, the pass the input arrives in first. On a [`Waits`](Turn::Waits)
/// the holder keeps the input; a repaint is requested.
pub(crate) fn turn(ctx: &Context, key: Id, own: Option<GestureId>) -> Turn {
    turn_after(ctx, key, own, None)
}

/// [`turn`] for an input that also waits for `gesture` while it is under way: a composite's
/// gesture that another part owns.
pub(crate) fn turn_after(ctx: &Context, key: Id, own: Option<GestureId>, gesture: Option<GestureId>) -> Turn {
    let pass = ctx.cumulative_pass_nr();
    let mut queue = Queue::get(ctx);
    queue.entries.retain(|e| e.polled + 1 >= pass);
    let turn = match queue.entries.iter().position(|e| e.key == key) {
        Some(i) => poll(ctx, &mut queue, i, own, pass),
        None => arrive(ctx, &mut queue, key, own, gesture, pass),
    };
    queue.put(ctx);
    if turn == Turn::Waits {
        ctx.request_repaint();
    }
    turn
}

/// A new input: it runs now when nothing waits and no other control's typing run is under way
/// (it has the focus, or its ending went out in this pass); else it queues, and a run that has
/// the focus is asked to end.
fn arrive(ctx: &Context, queue: &mut Queue, key: Id, own: Option<GestureId>, gesture: Option<GestureId>, pass: u64) -> Turn {
    let mut awaits: Vec<GestureId> = live::typing_ended(ctx).into_iter().filter(|&run| Some(run) != own).collect();
    if let Some((run, widget)) = live::typing(ctx).filter(|&(run, _)| Some(run) != own) {
        ctx.memory_mut(|m| m.surrender_focus(widget));
        awaits.push(run);
    }
    if let Some(gesture) = gesture.filter(|&g| !live::gone(ctx, g) && !awaits.contains(&g)) {
        awaits.push(gesture);
    }
    if awaits.is_empty() && queue.entries.is_empty() {
        return Turn::Now;
    }
    queue.entries.push(Entry { key, awaits, polled: pass, behind: false });
    Turn::Waits
}

/// A queued input: it runs when it is first, what it waits for has gone out, and no other input
/// ran in this pass.
fn poll(ctx: &Context, queue: &mut Queue, i: usize, own: Option<GestureId>, pass: u64) -> Turn {
    let entry = &mut queue.entries[i];
    entry.polled = pass;
    if i > 0 || queue.ran == Some(pass) {
        return Turn::Waits;
    }
    entry.awaits.retain(|&g| !live::gone(ctx, g));
    if entry.behind
        && let Some(gesture) = super::focused_gesture(ctx).filter(|&g| Some(g) != own && !entry.awaits.contains(&g))
    {
        entry.awaits.push(gesture);
    }
    if !entry.awaits.is_empty() {
        return Turn::Waits;
    }
    queue.entries.remove(i);
    queue.ran = Some(pass);
    for e in &mut queue.entries {
        e.behind = true;
    }
    Turn::Runs
}

/// Keeps holder `key`'s place at the head of the queue, for the next of the inputs it runs one a
/// pass, also when its last ran in this pass: no input that arrives meanwhile runs before it. It
/// also waits for `gesture` (one its input began). The holder asks for its turn with
/// [`turn_after`] in its next pass, and gives the place up with [`leave`] once it has no input.
pub(crate) fn keep(ctx: &Context, key: Id, gesture: Option<GestureId>) {
    let pass = ctx.cumulative_pass_nr();
    let mut queue = Queue::get(ctx);
    queue.entries.retain(|e| e.polled + 1 >= pass);
    let i = queue.entries.iter().position(|e| e.key == key).unwrap_or_else(|| {
        queue.entries.insert(0, Entry { key, awaits: Vec::new(), polled: pass, behind: false });
        0
    });
    let entry = &mut queue.entries[i];
    entry.polled = pass;
    if let Some(gesture) = gesture.filter(|g| !entry.awaits.contains(g)) {
        entry.awaits.push(gesture);
    }
    queue.put(ctx);
}

/// Removes holder `key`'s place from the queue ([`keep`]).
pub(crate) fn leave(ctx: &Context, key: Id) {
    let mut queue = Queue::get(ctx);
    queue.entries.retain(|e| e.key != key);
    queue.put(ctx);
}
