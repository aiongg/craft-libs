//! A composite's parts as one edit: the plumbing that turns the gestures of a composite's inner
//! controls (a colour picker's area, sliders and fields; a gradient editor's bar and fields) into
//! one [`Gesture`] of the composite's value.
//!
//! Each pass the composite draws its parts from its gesture's [`shown`](Gesture::shown) value and
//! collects their events in drawing order, as its own value ([`Edits::push`], [`Edits::typing`]),
//! with the one-shot values it sets ([`Edits::set`]) and the clicks that wait for a typing run
//! ([`Edits::click`], [`Edits::clicks`]). [`Edits::feed`] then gives them to its gesture:
//!
//! - The part whose Begin (or first Update) starts the gesture owns it; its Commit commits and
//!   its Cancel cancels. An owner's Commit that changed nothing ends the gesture at its start.
//!   Other parts' values join the gesture; Sets are [`set`](Gesture::set).
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
//! - In a pass where the composite's gesture ends (the owner's Commit or Cancel, the owner not
//!   drawn, or a cancel as the gesture loaded), the other parts' events wait for the next pass,
//!   held as the parts' own values, and the clicks wait with them ([`Edits::clicks`]). Each part
//!   that is drawn again feeds its held event first, mapped onto the value the ending left, so
//!   the host gets the ending, then the next edit (a Set, or a gesture that part owns), and no
//!   part's value is made from the value before the ending: a screen reader's step on one part
//!   in the pass another part's typing run commits on Enter steps the committed value. A part
//!   that isn't drawn again drops its held event. The composite's own one-shot values
//!   ([`Edits::set`]) are made from the value at the pass start, so they go only into a gesture
//!   that goes on.
//! - A press on a part while another part's typing run has the focus waits for that run's event
//!   (`controls::slider::press_turn`); a click on a button part waits the same way
//!   ([`Edits::clicks`]).

use std::any::Any;
use std::fmt;
use std::sync::Arc;

use egui::{Context, Id, Modifiers};

use super::gesture::Interrupted;
use super::{CancelReason, EditEvent, EditResponse, Ending, Gesture, GestureId};
use crate::controls::slider::PRESS_WAIT;

/// How a part's gesture under way would end if the part stopped being drawn now, as the
/// composite's value.
#[derive(Clone)]
enum End<V> {
    /// A commit that changed nothing: the composite's gesture ends at its start.
    Start,
    /// A commit with the part's last value, which the composite's gesture has.
    Current,
    /// A commit with a value no update carried.
    Value(V),
    Cancel,
}

impl<V: Clone + PartialEq + Send + Sync + 'static> End<V> {
    /// The value a commit ends the composite's `outer` gesture with, when not its current one.
    fn value(&self, outer: &Gesture<V>) -> Option<V> {
        match self {
            End::Start => outer.start().cloned(),
            End::Value(value) => Some(value.clone()),
            End::Current | End::Cancel => None,
        }
    }
}

/// A part's event, as its own value, kept between passes whatever its type.
type Raw = Arc<dyn Any + Send + Sync>;

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

/// What a composite keeps between passes for its parts' edits, with clicks of type `C`. The
/// composite stores it with its own state.
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
    /// The parts' events that wait for the pass after the one their gesture ended in.
    late: Vec<(Id, Late)>,
    /// Clicks that wait for a typing run to end ([`Edits::clicks`]): the pass of the first, the
    /// last pass that held them, and the clicks.
    held: Option<(u64, u64, Vec<C>)>,
}

impl<C> Default for State<C> {
    fn default() -> Self {
        Self { owner: None, run: None, end: None, active: false, late: Vec::new(), held: None }
    }
}

/// A part's held event ([`State::late`]).
#[derive(Clone)]
struct Late(Raw);

impl fmt::Debug for Late {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Late")
    }
}

/// The parts' edits of a pass, in drawing order, as the composite's values `V`, and the clicks
/// of type `C` (the composite turns them into values once they no longer wait).
pub(crate) struct Edits<V, C = ()> {
    /// Each with the part's own event, held if the composite's gesture ends first.
    events: Vec<(Id, EditEvent<V>, Late)>,
    /// The events held from the last pass, fed when their parts are pushed.
    late: Vec<(Id, Late)>,
    /// The owner, and whether the composite's gesture went on, as the last pass left them.
    owner: Option<Id>,
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
    /// One-shot values to set, after the parts' events.
    sets: Vec<V>,
    clicks: Vec<C>,
}

impl<V: Clone + PartialEq + Send + Sync + 'static, C> Edits<V, C> {
    pub fn new(state: &State<C>) -> Self {
        Self {
            events: Vec::new(),
            late: state.late.clone(),
            owner: state.owner,
            active: state.active,
            ctx: None,
            typing: Vec::new(),
            run: state.run,
            drawn: Vec::new(),
            sets: Vec::new(),
            clicks: Vec::new(),
        }
    }

    /// A part's edit, its value mapped to the composite's. Called for every part drawn, also
    /// without an event.
    pub fn push<T: Clone + Send + Sync + 'static>(&mut self, edit: &EditResponse<T>, map: impl Fn(T) -> V) {
        self.push_as(&edit.response.ctx, edit.response.id, edit.event.as_ref(), map);
    }

    /// [`push`](Edits::push) for a part whose edit carries another widget's response (a
    /// popover's picker, reported on its swatch): `id` is the part's own.
    pub fn push_as<T: Clone + Send + Sync + 'static>(&mut self, ctx: &Context, id: Id, event: Option<&EditEvent<T>>, map: impl Fn(T) -> V) {
        let run = event.and_then(EditEvent::id).or_else(|| self.run.filter(|(owner, _)| *owner == id).map(|(_, run)| run));
        let end = run.filter(|&run| super::is_live(ctx, run)).map(|run| match super::ending(ctx, run) {
            Ending::Commit { changed: false } => End::Start,
            Ending::Commit { changed: true } => super::unreported::<T>(ctx, run).map_or(End::Current, |value| End::Value(map(value))),
            Ending::Cancel => End::Cancel,
        });
        self.drawn.push((id, end));
        self.ctx.get_or_insert_with(|| ctx.clone());
        if let Some(i) = self.late.iter().position(|(part, _)| *part == id) {
            let (_, late) = self.late.remove(i);
            if let Some(held) = late.0.downcast_ref::<EditEvent<T>>() {
                self.events.push((id, held.clone().map(&map), late.clone()));
            }
        }
        if let Some(event) = event {
            self.events.push((id, event.clone().map(&map), Late(Arc::new(event.clone()))));
        }
    }

    /// [`push`](Edits::push) for a field whose focus is a typing run's.
    pub fn typing<T: Clone + Send + Sync + 'static>(&mut self, ctx: &Context, field: &EditResponse<T>, map: impl Fn(T) -> V) {
        self.typing.push(field.response.id);
        self.push_as(ctx, field.response.id, field.event.as_ref(), map);
    }

    /// A one-shot value, set after the parts' events in the order given.
    pub fn set(&mut self, value: V) {
        self.sets.push(value);
    }

    /// A click on a part that sets a value through the composite (a compare swatch's half, a
    /// delete button); [`clicks`](Edits::clicks) says when it runs.
    pub fn click(&mut self, click: C) {
        self.clicks.push(click);
    }

    /// The clicks that run in this pass: those held from earlier passes, then this pass's. While
    /// `waits` (another part's gesture had the focus as the composite's pass began,
    /// [`focused_gesture`](super::focused_gesture): a typing run) a click waits as a press on a
    /// part does (`controls::slider::press_turn`): it took the focus, the field ends its run and
    /// the event goes out, and the click runs in the first pass without such a gesture, at most
    /// [`PRESS_WAIT`] passes later. A click also waits in a pass where the composite's gesture
    /// ends (see the module docs). The composite makes each click's value from the host's value
    /// at that point.
    pub fn clicks(&mut self, ctx: &Context, state: &mut State<C>, waits: bool) -> Vec<C> {
        let pass = ctx.cumulative_pass_nr();
        let held = state.held.take().filter(|(_, last, _)| last + 1 == pass);
        let (since, mut clicks) = held.map_or((pass, Vec::new()), |(since, _, clicks)| (since, clicks));
        clicks.append(&mut self.clicks);
        let ends = self.active && self.owner.is_some_and(|owner| !self.is_drawn(owner) || self.owner_ends(owner));
        if (waits || ends) && !clicks.is_empty() && pass < since + PRESS_WAIT {
            state.held = Some((since, pass, clicks));
            ctx.request_repaint();
            return Vec::new();
        }
        clicks
    }

    fn is_drawn(&self, part: Id) -> bool {
        self.drawn.iter().any(|(id, _)| *id == part)
    }

    /// Whether `owner`'s event of this pass ends its gesture.
    fn owner_ends(&self, owner: Id) -> bool {
        self.events.iter().any(|(id, event, _)| *id == owner && matches!(event, EditEvent::Commit { .. } | EditEvent::Cancel { .. }))
    }

    /// Turns the parts' gestures into the composite's `outer` gesture (see the module docs) and
    /// returns it; `scoped` when the composite is inside [`scope`](super::scope). Clicks not
    /// taken with [`clicks`](Edits::clicks) are dropped.
    pub fn feed(self, outer: Gesture<V>, state: &mut State<C>, scoped: bool) -> Gesture<V> {
        let mut outer = outer.focus_owners(&self.typing);
        let ended = state.end.take();
        // The owner wasn't drawn: the composite's gesture ends in this pass.
        let gone = outer.is_active() && state.owner.is_some_and(|owner| !self.is_drawn(owner));
        // The owner whose event ends the composite's gesture in this pass: the other parts'
        // events wait for the next.
        let ender = state.owner.filter(|&owner| !scoped && outer.is_active() && !gone && self.owner_ends(owner));
        let holds = !scoped && (gone || outer.is_ending() || ender.is_some());
        state.late.clear();
        if gone {
            // A gesture no control recorded is cancelled (`edit::ending`).
            match ended.and_then(|end| end.get::<V>()).unwrap_or(End::Cancel) {
                End::Cancel => outer.cancel(CancelReason::Removed),
                end => {
                    if let Some(value) = end.value(&outer) {
                        outer.update(value, Modifiers::NONE);
                    }
                    outer.commit();
                }
            }
            state.owner = None;
            state.run = None;
        }
        for (who, event, late) in self.events {
            if holds && ender != Some(who) {
                state.late.push((who, late));
                continue;
            }
            let run = event.id();
            let owner = state.owner == Some(who);
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
                    }
                }
                EditEvent::Update { value, modifiers, .. } => {
                    if !outer.is_active() {
                        outer.begin();
                        state.owner = Some(who);
                    }
                    outer.update(value, modifiers);
                }
                EditEvent::Commit { value, changed, modifiers, .. } => match outer.start().cloned() {
                    None => outer.set(value),
                    Some(start) if owner => {
                        outer.update(if changed { value } else { start }, modifiers);
                        outer.commit();
                    }
                    Some(_) => outer.update(value, modifiers),
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
                end => Interrupted::Commit(end.value(&outer)),
            });
            state.end = Some(KeptEnd(Arc::new(end)));
        }
        state.active = outer.is_active() && !outer.is_ending();
        if let Some(ctx) = self.ctx.as_ref().filter(|_| !state.late.is_empty()) {
            ctx.request_repaint();
        }
        outer
    }
}
