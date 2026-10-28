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
//!   focus has left the field ([`Gesture::ended_by_part`]); if the composite stops being drawn,
//!   its gesture ends as that run would ([`Gesture::interrupted_with`]).
//! - A press on a part while another part's typing run has the focus waits for that run's event
//!   (`controls::slider::press_turn`); a click on a button part waits the same way
//!   ([`Edits::clicks`]).

use egui::{Context, Id};

use super::{EditEvent, EditResponse, Ending, Gesture, GestureId};
use crate::controls::slider::PRESS_WAIT;

/// How a typing field's run would end if it were interrupted now, as the composite's value: at
/// the start of the composite's gesture (an owner's `Commit` that changed nothing resets it
/// there), or with a value no update carried.
enum Draft<V> {
    Start,
    Value(V),
}

/// What a composite keeps between passes for its parts' edits, with clicks of type `C`. The
/// composite stores it with its own state.
#[derive(Clone, Debug)]
pub(crate) struct State<C> {
    /// The part whose gesture the composite's gesture is.
    owner: Option<Id>,
    /// The owner's own gesture, as its events named it.
    run: Option<(Id, GestureId)>,
    /// Clicks that wait for a typing run to end ([`Edits::clicks`]): the pass of the first, the
    /// last pass that held them, and the clicks.
    held: Option<(u64, u64, Vec<C>)>,
}

impl<C> Default for State<C> {
    fn default() -> Self {
        Self { owner: None, run: None, held: None }
    }
}

/// The parts' edits of a pass, in drawing order, as the composite's values `V`, and the clicks
/// of type `C` (the composite turns them into values once they no longer wait).
pub(crate) struct Edits<V, C = ()> {
    events: Vec<(Id, EditEvent<V>)>,
    /// The fields whose focus is a typing run's: the composite's gesture commits when it leaves
    /// them.
    typing: Vec<Id>,
    /// The owner's gesture as the last pass left it ([`State::run`]).
    run: Option<(Id, GestureId)>,
    /// The typing fields whose runs are under way, each with how it would end if the composite
    /// stopped being drawn (`None`: with the composite's own value).
    runs: Vec<(Id, Option<Draft<V>>)>,
    /// One-shot values to set, after the parts' events.
    sets: Vec<V>,
    clicks: Vec<C>,
}

impl<V: Clone + PartialEq + Send + Sync + 'static, C> Edits<V, C> {
    pub fn new(state: &State<C>) -> Self {
        Self { events: Vec::new(), typing: Vec::new(), run: state.run, runs: Vec::new(), sets: Vec::new(), clicks: Vec::new() }
    }

    /// A part's edit, its value mapped to the composite's.
    pub fn push<T: Clone>(&mut self, edit: &EditResponse<T>, map: impl FnOnce(T) -> V) {
        if let Some(event) = edit.event.clone() {
            self.events.push((edit.response.id, event.map(map)));
        }
    }

    /// [`push`](Edits::push) for a field whose focus is a typing run's, with how its run would
    /// end if interrupted ([`ending`](super::ending), [`unreported`](super::unreported)).
    pub fn typing<T: Clone + 'static>(&mut self, ctx: &Context, field: &EditResponse<T>, map: impl Fn(T) -> V) {
        let id = field.response.id;
        self.typing.push(id);
        let run = field.event.as_ref().and_then(EditEvent::id).or_else(|| self.run.filter(|(owner, _)| *owner == id).map(|(_, run)| run));
        if let Some(run) = run
            && super::is_live(ctx, run)
            && let Ending::Commit { changed } = super::ending(ctx, run)
        {
            let draft = match (changed, super::unreported::<T>(ctx, run)) {
                (false, _) => Some(Draft::Start),
                (true, Some(value)) => Some(Draft::Value(map(value))),
                // The composite's gesture has the field's last value.
                (true, None) => None,
            };
            self.runs.push((id, draft));
        }
        self.push(field, map);
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
    /// [`PRESS_WAIT`] passes later. The composite makes each click's value from the host's value
    /// at that point.
    pub fn clicks(&mut self, ctx: &Context, state: &mut State<C>, waits: bool) -> Vec<C> {
        let pass = ctx.cumulative_pass_nr();
        let held = state.held.take().filter(|(_, last, _)| last + 1 == pass);
        let (since, mut clicks) = held.map_or((pass, Vec::new()), |(since, _, clicks)| (since, clicks));
        clicks.append(&mut self.clicks);
        if waits && !clicks.is_empty() && pass < since + PRESS_WAIT {
            state.held = Some((since, pass, clicks));
            ctx.request_repaint();
            return Vec::new();
        }
        clicks
    }

    /// Turns the parts' gestures into the composite's `outer` gesture (see the module docs) and
    /// returns it; `scoped` when the composite is inside [`scope`](super::scope). Clicks not
    /// taken with [`clicks`](Edits::clicks) are dropped.
    pub fn feed(self, outer: Gesture<V>, state: &mut State<C>, scoped: bool) -> Gesture<V> {
        let mut outer = outer.focus_owners(&self.typing);
        for (who, event) in self.events {
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
        if let Some((_, draft)) = self.runs.into_iter().find(|(field, _)| state.owner == Some(*field)) {
            outer.ended_by_part();
            if let Some(value) = draft.and_then(|draft| match draft {
                Draft::Start => outer.start().cloned(),
                Draft::Value(value) => Some(value),
            }) {
                outer.interrupted_with(value);
            }
        }
        outer
    }
}
