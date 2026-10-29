//! A composite's parts as one edit: the plumbing that turns the gestures of a composite's inner
//! controls (a colour picker's area, sliders and fields; a gradient editor's bar and fields) into
//! one [`Gesture`] of the composite's value.
//!
//! Each pass the composite draws its parts from its gesture's [`shown`](Gesture::shown) value and
//! collects their events in drawing order, as its own value ([`Edits::push`], [`Edits::typing`]),
//! with its own inputs (a click on a button part, a sample: [`Edits::input`], [`Edits::inputs`])
//! and the values it makes from them ([`Edits::set`]). [`Edits::feed`] then gives them to its
//! gesture:
//!
//! - The part whose Begin (or first Update) starts the gesture owns it; its Commit commits and
//!   its Cancel cancels. The gesture holds only its owner's values: another part's event waits
//!   for the gesture to end (below). Sets are [`set`](Gesture::set).
//! - The owner's Commit ends the gesture with the owner's value as the composite's, which changed
//!   the gesture when it differs from the composite's start. While the gesture runs from the
//!   composite's start (it isn't carried on from the host's value after the host ended it), an
//!   owner's value that is the owner's own start is the composite's start exactly: a value that
//!   doesn't map back to the same composite value (float units, colour spaces) doesn't make a
//!   change of nothing.
//! - Inside [`scope`](super::scope) every value is a `set`, so only Updates go out.
//! - The typing fields are the gesture's [`focus_owners`](Gesture::focus_owners). While the
//!   owner's typing run is under way, its ending event ends the composite's gesture, also when the
//!   focus has left the field ([`Gesture::ended_by_part`]).
//! - An interrupted composite gesture ends as its owner's gesture would
//!   ([`Gesture::interrupted_as`]): when the composite stops being drawn, or the host ends its
//!   gesture for another one ([`GestureHost`](super::GestureHost)), the host gets the owner's
//!   ending, a commit with the owner's last value (also one no update carried, as the
//!   composite's value) or a cancel, whatever the composite's own focus rule would say. A
//!   popover's picker in the middle of a hex run commits the typed colour although its field is
//!   not one of the composite's focus owners.
//! - The composite pushes every part it draws in every pass, with or without an event; a part
//!   it didn't push wasn't drawn. When the owner wasn't drawn its gesture is over (as for any
//!   control, [`is_live`](super::is_live)), and the composite's gesture ends as the owner's would
//!   have, as the owner's last pass recorded it ([`ending`](super::ending)): a commit with the
//!   owner's last value, also one no update carried ([`unreported`](super::unreported)), or a
//!   cancel ([`CancelReason::Removed`]). So a part that goes mid-gesture (a popover's picker that
//!   closes, a field of a selection that changed) never leaves the composite's gesture open.
//! - A part's event that would begin, change or set the composite's value while another part
//!   owns its gesture, or in the pass where the gesture ends (the owner's Commit or Cancel, the
//!   owner not drawn, or a cancel as the gesture loaded), is held as the part's own value and
//!   takes its turn from the edit order (`edit::order`): it waits for the gesture's ending event
//!   to go out, and asks a typing run that has the focus to end. The part feeds it when it is
//!   drawn in the pass its turn comes, mapped onto the composite's value then, so the host gets
//!   the ending, then the next edit (a Set, or a gesture that part owns), and no part's value is
//!   made from the value before the ending: a screen reader's step on one part during another
//!   part's drag is its own Set after the release. A part whose value holds several of the
//!   composite's values ([`Edits::push_from`]: a row's thumbs, a graph's points) applies only
//!   what its event changed from the value it was drawn from, onto its value of that pass
//!   ([`Rebase`]): a step on the gamma thumb keeps the black point a field scrub committed. An
//!   event whose change can't be told on that value is dropped rather than replayed whole,
//!   which would undo what went out meanwhile. While it waits, the part's newer events replace it. A part that isn't drawn in a pass drops
//!   its held event, and so does every part of a composite that isn't (a popover's picker that
//!   closes with the commit).
//! - A press on a part takes its turn from the edit order too (`controls::slider::press_turn`),
//!   and so do the composite's own inputs ([`Edits::inputs`]): they wait for another control's
//!   typing run and for the composite's gesture to end, then the composite makes their values
//!   ([`Edits::set`]) from the host's value. Inputs that run in one pass make one Set with the
//!   last value.
//! - Every event a pass applies (not a mere Begin) asks for a repaint, so a part drawn
//!   before the change shows it in the next frame whatever the host's update policy.

use std::any::Any;
use std::fmt;
use std::sync::Arc;

use egui::{Context, Id, Modifiers};

use super::gesture::Interrupted;
use super::order::{self, Turn};
use super::{CancelReason, EditEvent, EditResponse, Ending, Gesture, GestureId};

/// How a part's gesture under way would end if the part stopped being drawn now, as the
/// composite's value.
#[derive(Clone)]
enum End<V> {
    /// A commit with the part's last value: `value` when no update carried it, else the value
    /// the composite's gesture has. `changed` is the part's own: whether that value differs from
    /// the part's start.
    Commit {
        value: Option<V>,
        changed: bool,
    },
    Cancel,
}

impl<V: Clone + PartialEq + Send + Sync + 'static> End<V> {
    /// The value a commit ends the composite's `outer` gesture with, when not its current one.
    /// `pure`: the gesture holds only its owner's values from the composite's start
    /// ([`State::pure`]), so an unchanged owner leaves the composite at its start.
    fn value(&self, outer: &Gesture<V>, pure: bool) -> Option<V> {
        match self {
            End::Commit { changed: false, .. } if pure => outer.start().cloned(),
            End::Commit { value, .. } => value.clone(),
            End::Cancel => None,
        }
    }
}

/// A part's event, as its own value, kept between passes whatever its type: a [`PartEvent`].
type Raw = Arc<dyn Any + Send + Sync>;

/// A part's value that holds several of the composite's values (a graph's points), which a held
/// event changes only some of ([`Edits::push_from`]).
pub(crate) trait Rebase: Sized {
    /// `self`, an event's value made from `base`, with what it changed from `base` applied onto
    /// `now` instead, keeping the rules the part keeps on its own edits. `None` when that change
    /// can't be told on `now`, or doesn't fit it: the held event is dropped, as no guess at it
    /// may undo what went out after `base`.
    fn rebase(self, base: &Self, now: &Self) -> Option<Self>;
}

/// How a part's value made from one value applies onto another ([`Rebase::rebase`]).
type Rebaser<T> = Arc<dyn Fn(T, &T, &T) -> Option<T> + Send + Sync>;

/// A part's event, with the value the part was drawn from when the event rebases
/// ([`Edits::push_from`]).
struct PartEvent<T> {
    event: EditEvent<T>,
    drawn: Option<Drawn<T>>,
}

/// The value a part was drawn from, and how an event's value made from it rebases.
#[derive(Clone)]
struct Drawn<T> {
    value: T,
    rebase: Rebaser<T>,
}

/// The owner's [`End`] kept between passes, whatever the composite's value type.
#[derive(Clone)]
struct KeptEnd(Arc<dyn Any + Send + Sync>);

impl KeptEnd {
    fn get<V: Clone + 'static>(&self) -> Option<End<V>> {
        self.0.downcast_ref::<End<V>>().cloned()
    }
}

impl fmt::Debug for KeptEnd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("KeptEnd")
    }
}

/// What a composite keeps between passes for its parts' edits, with its own inputs of type `C`.
/// The composite stores it with its own state.
#[derive(Clone, Debug)]
pub(crate) struct State<C> {
    /// The part whose gesture the composite's gesture is.
    owner: Option<Id>,
    /// The owner's own gesture, as its events named it.
    run: Option<(Id, GestureId)>,
    /// How the owner's gesture ends if the owner isn't drawn, as its last pass recorded it.
    end: Option<KeptEnd>,
    /// Whether the composite's gesture went on after the last pass.
    active: bool,
    /// The composite's gesture as the last pass left it.
    gesture: Option<GestureId>,
    /// The composite's gesture while it runs from the composite's start: it isn't a gesture
    /// carried on from the host's value (which has another id). The owner's start is then the
    /// composite's start.
    pure: Option<GestureId>,
    /// The parts' events that wait for their turn, and the pass that held them.
    held: Vec<(Id, Held)>,
    held_pass: u64,
    /// The composite's inputs that wait for their turn ([`Edits::inputs`]), and the pass that
    /// held them.
    inputs: Option<(u64, Vec<C>)>,
}

impl<C> Default for State<C> {
    fn default() -> Self {
        Self { owner: None, run: None, end: None, active: false, gesture: None, pure: None, held: Vec::new(), held_pass: 0, inputs: None }
    }
}

/// A part's held event ([`State::held`]).
#[derive(Clone)]
struct Held(Raw);

impl fmt::Debug for Held {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Held")
    }
}

/// The parts' edits of a pass, in drawing order, as the composite's values `V`, and the
/// composite's inputs of type `C` (the composite turns them into values once their turn comes).
pub(crate) struct Edits<V, C = ()> {
    /// The composite.
    id: Id,
    /// Each with the part's own event, held if it isn't the gesture's to take.
    events: Vec<(Id, EditEvent<V>, Held)>,
    /// The events held from the last pass, fed when their parts are pushed and their turn has
    /// come, and the pass that held them: they are dropped in any pass but the next.
    held: Vec<(Id, Held)>,
    held_pass: u64,
    /// The held events that wait on.
    waiting: Vec<(Id, Held)>,
    /// The composite's gesture, and whether it went on, as the last pass left them.
    gesture: Option<GestureId>,
    active: bool,
    /// The context, once a part is pushed.
    ctx: Option<Context>,
    /// The fields whose focus is a typing run's: the composite's gesture commits when it leaves
    /// them.
    typing: Vec<Id>,
    /// The owner's gesture as the last pass left it ([`State::run`]).
    run: Option<(Id, GestureId)>,
    /// The parts drawn in this pass, each with how its gesture under way would end if it stopped
    /// being drawn.
    drawn: Vec<(Id, Option<End<V>>)>,
    /// Values to set, after the parts' events.
    sets: Vec<V>,
    inputs: Vec<C>,
}

impl<V: Clone + PartialEq + Send + Sync + 'static, C> Edits<V, C> {
    /// The edits of composite `id` (its widget id) in this pass.
    pub fn new(id: Id, state: &State<C>) -> Self {
        Self {
            id,
            events: Vec::new(),
            held: state.held.clone(),
            held_pass: state.held_pass,
            waiting: Vec::new(),
            gesture: state.gesture,
            active: state.active,
            ctx: None,
            typing: Vec::new(),
            run: state.run,
            drawn: Vec::new(),
            sets: Vec::new(),
            inputs: Vec::new(),
        }
    }

    /// The composite's gesture while another part owns it: what a part's held event waits for.
    fn owned(&self) -> Option<GestureId> {
        self.gesture.filter(|_| self.active)
    }

    /// A part's edit, its value mapped to the composite's. Called for every part drawn, also
    /// without an event. For a part with one of the composite's values (a slider, a dial): a
    /// held event's value is mapped onto the composite's value of the pass its turn comes.
    pub fn push<T: Clone + Send + Sync + 'static>(&mut self, edit: &EditResponse<T>, map: impl Fn(T) -> V) {
        self.push_as(&edit.response.ctx, edit.response.id, edit.event.as_ref(), map);
    }

    /// [`push`](Edits::push) for a part whose value holds several of the composite's values (a
    /// graph's points), drawn from `drawn` in this pass. A held event applies only what it
    /// changed from the value its part was drawn from, onto `drawn` of the pass its turn comes
    /// ([`Rebase`]), or is dropped.
    pub fn push_from<T: Rebase + Clone + Send + Sync + 'static>(&mut self, edit: &EditResponse<T>, drawn: &T, map: impl Fn(T) -> V) {
        self.push_rebased(edit, drawn, T::rebase, map);
    }

    /// [`push_from`](Edits::push_from) for a part whose rules depend on how it is set up (a
    /// row's thumb order): `rebase` is the part's own, as [`Rebase::rebase`].
    pub fn push_rebased<T: Clone + Send + Sync + 'static>(
        &mut self,
        edit: &EditResponse<T>,
        drawn: &T,
        rebase: impl Fn(T, &T, &T) -> Option<T> + Send + Sync + 'static,
        map: impl Fn(T) -> V,
    ) {
        let drawn = Drawn { value: drawn.clone(), rebase: Arc::new(rebase) };
        self.add(&edit.response.ctx, edit.response.id, edit.event.as_ref(), Some(drawn), map);
    }

    /// [`push`](Edits::push) for a part whose edit carries another widget's response (a
    /// popover's picker, reported on its swatch): `id` is the part's own.
    pub fn push_as<T: Clone + Send + Sync + 'static>(&mut self, ctx: &Context, id: Id, event: Option<&EditEvent<T>>, map: impl Fn(T) -> V) {
        self.add(ctx, id, event, None, map);
    }

    fn add<T: Clone + Send + Sync + 'static>(
        &mut self,
        ctx: &Context,
        id: Id,
        event: Option<&EditEvent<T>>,
        drawn: Option<Drawn<T>>,
        map: impl Fn(T) -> V,
    ) {
        let run = event.and_then(EditEvent::id).or_else(|| self.run.filter(|(owner, _)| *owner == id).map(|(_, run)| run));
        let end = run.filter(|&run| super::is_live(ctx, run)).map(|run| match super::ending(ctx, run) {
            Ending::Commit { changed } => End::Commit { value: super::unreported::<T>(ctx, run).map(&map), changed },
            Ending::Cancel => End::Cancel,
        });
        self.drawn.push((id, end));
        self.ctx.get_or_insert_with(|| ctx.clone());
        if self.held_pass + 1 != ctx.cumulative_pass_nr() {
            self.held.clear();
        }
        let new = event.map(|event| Held(Arc::new(PartEvent { event: event.clone(), drawn: drawn.clone() })));
        if let Some(i) = self.held.iter().position(|(part, _)| *part == id) {
            let (_, held) = self.held.remove(i);
            if order::turn_after(ctx, held_key(self.id, id), None, self.owned()) == Turn::Waits {
                self.waiting.push((id, new.unwrap_or(held)));
                return;
            }
            if let Some(part) = held.0.downcast_ref::<PartEvent<T>>() {
                let event = match (&part.drawn, &drawn, part.event.value()) {
                    (Some(base), Some(now), Some(value)) => {
                        (now.rebase)(value.clone(), &base.value, &now.value).map(|value| part.event.clone().map(|_| value))
                    }
                    _ => Some(part.event.clone()),
                };
                if let Some(event) = event {
                    self.events.push((id, event.map(&map), held.clone()));
                }
            }
        }
        if let (Some(event), Some(new)) = (event, new) {
            self.events.push((id, event.clone().map(&map), new));
        }
    }

    /// [`push`](Edits::push) for a field whose focus is a typing run's.
    pub fn typing<T: Clone + Send + Sync + 'static>(&mut self, ctx: &Context, field: &EditResponse<T>, map: impl Fn(T) -> V) {
        self.typing.push(field.response.id);
        self.push_as(ctx, field.response.id, field.event.as_ref(), map);
    }

    /// A value made from an input that [`inputs`](Edits::inputs) returned, set after the parts'
    /// events in the order given.
    pub fn set(&mut self, value: V) {
        self.sets.push(value);
    }

    /// One of the composite's own inputs: a click on a button part (a compare swatch's half, a
    /// delete button) or a value it makes itself (a sample). [`inputs`](Edits::inputs) says when
    /// it runs.
    pub fn input(&mut self, input: C) {
        self.inputs.push(input);
    }

    /// The composite's inputs that run in this pass: those held from earlier passes, then this
    /// pass's. They take their turn from the edit order (`edit::order`): they wait while another
    /// control's typing run has the focus (asking it to end) and while the composite's gesture
    /// is under way, and run in the first pass after its ending event has gone out. The
    /// composite makes their values from the host's value then ([`set`](Edits::set)).
    pub fn inputs(&mut self, ctx: &Context, state: &mut State<C>) -> Vec<C> {
        let pass = ctx.cumulative_pass_nr();
        let mut inputs = state.inputs.take().filter(|(last, _)| last + 1 == pass).map_or_else(Vec::new, |(_, inputs)| inputs);
        inputs.append(&mut self.inputs);
        if inputs.is_empty() {
            return inputs;
        }
        match order::turn_after(ctx, self.id.with("craft_ui::edit::parts::inputs"), None, self.owned()) {
            Turn::Waits => {
                state.inputs = Some((pass, inputs));
                Vec::new()
            }
            Turn::Now | Turn::Runs => inputs,
        }
    }

    fn is_drawn(&self, part: Id) -> bool {
        self.drawn.iter().any(|(id, _)| *id == part)
    }

    /// Whether `owner`'s event of this pass ends its gesture.
    fn owner_ends(&self, owner: Id) -> bool {
        self.events.iter().any(|(id, event, _)| *id == owner && matches!(event, EditEvent::Commit { .. } | EditEvent::Cancel { .. }))
    }

    /// Turns the parts' gestures into the composite's `outer` gesture (see the module docs) and
    /// returns it; `scoped` when the composite is inside [`scope`](super::scope). Inputs not
    /// taken with [`inputs`](Edits::inputs) are dropped.
    pub fn feed(self, outer: Gesture<V>, state: &mut State<C>, scoped: bool) -> Gesture<V> {
        let mut outer = outer.focus_owners(&self.typing);
        let ended = state.end.take();
        // The owner wasn't drawn: the composite's gesture ends in this pass.
        let gone = outer.is_active() && state.owner.is_some_and(|owner| !self.is_drawn(owner));
        // The owner whose event ends the composite's gesture in this pass: the other parts'
        // events wait for the next.
        let ender = state.owner.filter(|&owner| !scoped && outer.is_active() && !gone && self.owner_ends(owner));
        let ends = !scoped && (gone || outer.is_ending() || ender.is_some());
        // What a held event waits for: the gesture under way, or the one cancelled as it loaded.
        let gesture = outer.id().or(state.gesture);
        let mut held = self.waiting;
        // Decided from the events, not by comparing values: a value type holding a NaN is never
        // equal to itself.
        let mut moved = gone || !self.sets.is_empty();
        if gone {
            // A gesture no control recorded is cancelled (`edit::ending`).
            match ended.and_then(|end| end.get::<V>()).unwrap_or(End::Cancel) {
                End::Cancel => outer.cancel(CancelReason::Removed),
                end => {
                    if let Some(value) = end.value(&outer, is_pure(&outer, state)) {
                        outer.update(value, Modifiers::NONE);
                    }
                    outer.commit();
                }
            }
            state.owner = None;
            state.run = None;
        }
        for (who, event, raw) in self.events {
            let owner = state.owner == Some(who);
            let other = outer.is_active() && state.owner.is_some_and(|owner| owner != who);
            if !scoped && ((ends && ender != Some(who)) || other) {
                held.retain(|(part, _)| *part != who);
                held.push((who, raw));
                if let Some(ctx) = &self.ctx {
                    order::turn_after(ctx, held_key(self.id, who), None, gesture);
                }
                continue;
            }
            let run = event.id();
            moved |= !matches!(event, EditEvent::Begin { .. });
            match event {
                // Inside a scope the parts report only updates, each a change of its own.
                event if scoped => {
                    if let Some(value) = event.value() {
                        outer.set(value.clone());
                    }
                }
                EditEvent::Begin { .. } => {
                    if !outer.is_active() {
                        outer.begin();
                        state.owner = Some(who);
                        state.pure = outer.id();
                    }
                }
                EditEvent::Update { value, modifiers, .. } => {
                    if !outer.is_active() {
                        outer.begin();
                        state.owner = Some(who);
                        state.pure = outer.id();
                    }
                    outer.update(value, modifiers);
                }
                EditEvent::Commit { value, changed, modifiers, .. } => match outer.start().cloned() {
                    None => outer.set(value),
                    Some(start) => {
                        let value = if !changed && is_pure(&outer, state) { start } else { value };
                        outer.update(value, modifiers);
                        outer.commit();
                    }
                },
                EditEvent::Cancel { reason, .. } => {
                    if owner {
                        outer.cancel(reason);
                    }
                }
                EditEvent::Set { value, .. } => outer.set(value),
            }
            if state.owner == Some(who)
                && let Some(run) = run
            {
                state.run = Some((who, run));
            }
        }
        for value in self.sets {
            if outer.is_active() && value != *outer.shown() {
                state.pure = None;
            }
            outer.set(value);
        }
        let owner_end = state.owner.filter(|_| outer.is_active()).and_then(|owner| {
            let (_, end) = self.drawn.into_iter().find(|(id, _)| *id == owner)?;
            Some((self.typing.contains(&owner), end?))
        });
        if let Some((typing, end)) = owner_end {
            // The owner's typing run under way: its ending event ends the composite's gesture.
            if typing && !matches!(end, End::Cancel) {
                outer.ended_by_part();
            }
            outer.interrupted_as(match &end {
                End::Cancel => Interrupted::Cancel,
                end => Interrupted::Commit(end.value(&outer, is_pure(&outer, state))),
            });
            state.end = Some(KeptEnd(Arc::new(end)));
        }
        state.active = outer.is_active() && !outer.is_ending();
        state.gesture = outer.id().or(gesture);
        state.held = held;
        if let Some(ctx) = &self.ctx {
            state.held_pass = ctx.cumulative_pass_nr();
            if moved || !state.held.is_empty() {
                ctx.request_repaint();
            }
        }
        outer
    }
}

/// The edit-order key of part `part`'s held event in composite `id`.
fn held_key(id: Id, part: Id) -> Id {
    id.with(("craft_ui::edit::parts::held", part))
}

/// Whether the composite's `outer` gesture runs from the composite's start ([`State::pure`]).
fn is_pure<V: Clone + PartialEq + Send + Sync + 'static, C>(outer: &Gesture<V>, state: &State<C>) -> bool {
    outer.id().is_some() && outer.id() == state.pure
}
