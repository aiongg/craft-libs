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
//!   owner not drawn, or a cancel as the gesture loaded), is held and takes its turn from the edit
//!   order (`edit::order`): it waits for the gesture's ending event to go out, and asks a typing
//!   run that has the focus to end. So the host gets the ending, then the next edit (a Set, or a
//!   gesture that part owns), and no part's value is made from the value before the ending: a
//!   screen reader's step on one part during another part's drag is its own Set after the
//!   release.
//! - A part pushed with its operations ([`Edits::push`]: what the user did, a step, a set, a move
//!   to a bound) is held as those operations, in the composite's queue in arrival order. A newer
//!   event never replaces an older one, except a gesture's Update or Commit, which replaces the
//!   queued Begin or Update of its run: a gesture's values are positions, not changes. When the
//!   queue's turn comes, [`Edits::replay`] applies its oldest entry onto the composite's value
//!   before the parts are drawn, and it goes out as its own Set, its own undo step: one entry a
//!   pass. An entry none of whose operations apply on that value is dropped, and the next goes
//!   out instead. The parts draw from and map onto [`Edits::value`], which holds it. While the
//!   queue holds entries, or one went out in the pass, the parts' new events join its tail, and
//!   a press waits (`order::keep`). An entry of a gesture (a field's typing run under way during
//!   another part's drag) begins the composite's gesture with its part as owner, and the entries
//!   after it wait for that gesture to end. A part's Cancel drops its run's entries. The queue is
//!   dropped in a pass the composite isn't drawn.
//! - A part pushed with its value ([`Edits::push_mapped`]) is held as its own value, and the part
//!   feeds it when it is drawn in the pass its turn comes, mapped onto the composite's value then.
//!   A part whose value holds several of the composite's values ([`Edits::push_from`]: a row's
//!   thumbs, a graph's points) applies only what its event changed from the value it was drawn
//!   from, onto its value of that pass ([`Rebase`]): a step on the gamma thumb keeps the black
//!   point a field scrub committed. An event whose change can't be told on that value is dropped
//!   rather than replayed whole, which would undo what went out meanwhile. While it waits, the
//!   part's newer events replace it. A part that isn't drawn in a pass drops its held event, and
//!   so does every part of a composite that isn't (a popover's picker that closes with the
//!   commit).
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

/// A part's event that waits for its turn, as the operations `O` the user did ([`Edits::push`]).
#[derive(Clone, Debug)]
pub(crate) struct Queued<O> {
    part: Id,
    kind: Kind,
    ops: Vec<O>,
}

impl<O> Queued<O> {
    /// The value this entry's operations make from `now`, each applied with `apply` onto the
    /// value the ones before it left: `None` when none applies. An operation that can't apply
    /// (its target isn't on that value) is skipped. A Begin has no operations and leaves `now`.
    fn apply<V: Clone>(&self, now: &V, apply: &mut impl FnMut(&O, &V) -> Option<V>) -> Option<V> {
        if self.ops.is_empty() {
            return matches!(self.kind, Kind::Begin { .. }).then(|| now.clone());
        }
        let mut value: Option<V> = None;
        for op in &self.ops {
            if let Some(next) = apply(op, value.as_ref().unwrap_or(now)) {
                value = Some(next);
            }
        }
        value
    }
}

/// A part's event without its value: what a [`Queued`] entry goes out as.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Kind {
    Set { modifiers: Modifiers },
    Begin { id: GestureId },
    Update { id: GestureId, modifiers: Modifiers },
    Commit { id: GestureId, changed: bool, modifiers: Modifiers },
}

impl Kind {
    /// `event`'s kind; `None` for a Cancel, which has nothing to replay.
    fn of<T>(event: &EditEvent<T>) -> Option<Self> {
        Some(match *event {
            EditEvent::Set { modifiers, .. } => Kind::Set { modifiers },
            EditEvent::Begin { id } => Kind::Begin { id },
            EditEvent::Update { id, modifiers, .. } => Kind::Update { id, modifiers },
            EditEvent::Commit { id, changed, modifiers, .. } => Kind::Commit { id, changed, modifiers },
            EditEvent::Cancel { .. } => return None,
        })
    }

    /// The part's gesture this event belongs to; `None` for a Set.
    fn run(self) -> Option<GestureId> {
        match self {
            Kind::Set { .. } => None,
            Kind::Begin { id } | Kind::Update { id, .. } | Kind::Commit { id, .. } => Some(id),
        }
    }

    /// The event of this kind with `value` (which a Begin doesn't carry).
    fn event<V>(self, value: V) -> EditEvent<V> {
        match self {
            Kind::Set { modifiers } => EditEvent::set(value, modifiers),
            Kind::Begin { id } => EditEvent::begin(id),
            Kind::Update { id, modifiers } => EditEvent::update(id, value, modifiers),
            Kind::Commit { id, changed, modifiers } => EditEvent::commit(id, value, changed, modifiers),
        }
    }
}

/// What a composite keeps between passes for its parts' edits, with its parts' operations of type
/// `O` and its own inputs of type `C`. The composite stores it with its own state.
#[derive(Clone, Debug)]
pub(crate) struct State<O, C = ()> {
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
    /// carried on from the host's value (which has another id), nor one a queued part event
    /// began. The owner's start is then the composite's start.
    pure: Option<GestureId>,
    /// The parts' events held as values ([`Edits::push_mapped`]) that wait for their turn.
    held: Vec<(Id, Held)>,
    /// The parts' operations that wait for their turn, oldest first ([`Edits::push`]).
    queue: Vec<Queued<O>>,
    /// The pass that left `held` and `queue`: they are dropped in any pass but the next.
    pass: u64,
    /// Whether the queue has its place in the edit order.
    placed: bool,
    /// The composite's inputs that wait for their turn ([`Edits::inputs`]), and the pass that
    /// held them.
    inputs: Option<(u64, Vec<C>)>,
}

impl<O, C> State<O, C> {
    /// Gives the queue as a pass left it its place in the edit order, under `key`. `entered`: it
    /// has one in this pass ([`Edits::replay`]); else its entries were held in this pass and wait
    /// for `gesture`. `began`: the composite's gesture an entry that went out in this pass began,
    /// which the entries after it wait for.
    fn place_queue(&self, ctx: &Context, key: Id, entered: bool, began: Option<GestureId>, gesture: Option<GestureId>) {
        if self.queue.is_empty() {
            return;
        }
        if !entered {
            if order::turn_after(ctx, key, None, gesture) == Turn::Now {
                order::keep(ctx, key, None);
            }
        } else if began.is_some() {
            order::keep(ctx, key, began);
        }
    }
}

impl<O, C> Default for State<O, C> {
    fn default() -> Self {
        Self {
            owner: None,
            run: None,
            end: None,
            active: false,
            gesture: None,
            pure: None,
            held: Vec::new(),
            queue: Vec::new(),
            pass: 0,
            placed: false,
            inputs: None,
        }
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

/// What a part's event of a pass waits as if [`Edits::feed`] holds it.
enum Hold<O> {
    /// A value ([`Edits::push_mapped`]).
    Value(Held),
    /// A part's operations of this pass ([`Edits::push`]).
    Ops(Queued<O>),
    /// The queue's entry [`Edits::replay`] took in this pass: it goes back to the queue's head.
    Replayed(Queued<O>),
    /// Nothing: a Cancel.
    Nothing,
}

/// The parts' edits of a pass, in drawing order, as the composite's values `V`, the operations
/// `O` they report, and the composite's inputs of type `C` (the composite turns them into values
/// once their turn comes).
pub(crate) struct Edits<V, O, C = ()> {
    /// The composite.
    id: Id,
    /// Each with the part's own event, and what it waits as if it isn't the gesture's to take.
    events: Vec<(Id, EditEvent<V>, Hold<O>)>,
    /// The value events held from the last pass, fed when their parts are pushed and their turn
    /// has come.
    held: Vec<(Id, Held)>,
    /// The value events held that wait on.
    waiting: Vec<(Id, Held)>,
    /// The queue as the last pass left it, then with the entry `replay` took out and the
    /// operations that joined it in this pass.
    queue: Vec<Queued<O>>,
    /// The pass that left `held` and `queue`, and whether they were checked against this pass.
    pass: u64,
    checked: bool,
    /// The queue's place in the edit order as the last pass left it, and whether it has one in
    /// this pass.
    placed: bool,
    entered: bool,
    /// The composite's value as the replay and the events pushed so far that go out in this pass
    /// leave it.
    value: V,
    /// Whether the composite is inside [`scope`](super::scope).
    scoped: bool,
    /// The composite's gesture as loaded, and whether it was ending.
    loaded: Option<GestureId>,
    ending: bool,
    /// An entry of the queue went out in this pass, and the part and run whose gesture it begins.
    drained: bool,
    replayed: Option<(Id, GestureId)>,
    /// The composite's gesture, and whether it went on, as the last pass left them.
    gesture: Option<GestureId>,
    active: bool,
    /// The context, once a part is pushed.
    ctx: Option<Context>,
    /// The fields whose focus is a typing run's: the composite's gesture commits when it leaves
    /// them.
    typing: Vec<Id>,
    /// The owner's gesture as the last pass left it ([`State::run`]), and its part.
    run: Option<(Id, GestureId)>,
    owner: Option<Id>,
    /// The parts drawn in this pass, each with how its gesture under way would end if it stopped
    /// being drawn.
    drawn: Vec<(Id, Option<End<V>>)>,
    /// Values to set, after the parts' events.
    sets: Vec<V>,
    inputs: Vec<C>,
}

impl<V: Clone + PartialEq + Send + Sync + 'static, O: Clone + Send + Sync + 'static, C> Edits<V, O, C> {
    /// The edits of composite `id` (its widget id) in this pass. `outer` is the composite's
    /// gesture as loaded; `scoped` when the composite is inside [`scope`](super::scope).
    pub fn new(id: Id, state: &State<O, C>, outer: &Gesture<V>, scoped: bool) -> Self {
        Self {
            id,
            events: Vec::new(),
            held: state.held.clone(),
            waiting: Vec::new(),
            queue: state.queue.clone(),
            pass: state.pass,
            checked: false,
            placed: state.placed,
            entered: false,
            value: outer.shown().clone(),
            scoped,
            loaded: outer.id(),
            ending: outer.is_ending(),
            drained: false,
            replayed: None,
            gesture: state.gesture,
            active: state.active,
            ctx: None,
            typing: Vec::new(),
            run: state.run,
            owner: state.owner,
            drawn: Vec::new(),
            sets: Vec::new(),
            inputs: Vec::new(),
        }
    }

    /// The composite's gesture while another part owns it: what a part's held event waits for.
    fn owned(&self) -> Option<GestureId> {
        self.gesture.filter(|_| self.active)
    }

    /// Keeps `ctx`, and drops what the last pass held unless that was the pass before this one.
    fn arrive(&mut self, ctx: &Context) {
        if !self.checked {
            self.checked = true;
            if self.pass + 1 != ctx.cumulative_pass_nr() {
                self.held.clear();
                self.queue.clear();
            }
        }
        self.ctx.get_or_insert_with(|| ctx.clone());
    }

    /// The queue's turn: when it has come, its oldest entry that applies goes out in this pass as
    /// the part's event, made with `apply` from the composite's value (see the module docs).
    /// Called once a pass, before the first part is drawn, by a composite whose parts report
    /// operations.
    #[allow(dead_code, reason = "the composites push operations from DS-13ol T2 on")]
    pub fn replay(&mut self, ctx: &Context, mut apply: impl FnMut(&O, &V) -> Option<V>) {
        self.arrive(ctx);
        let key = queue_key(self.id);
        if self.queue.is_empty() {
            if self.placed {
                order::leave(ctx, key);
            }
            return;
        }
        self.entered = true;
        // The composite's gesture under way: the owner's, or one an entry of the queue began.
        if self.loaded.is_some() {
            order::keep(ctx, key, self.loaded);
            return;
        }
        if order::turn_after(ctx, key, None, self.owned()) == Turn::Waits {
            return;
        }
        while !self.queue.is_empty() {
            let entry = self.queue.remove(0);
            let Some(value) = entry.apply(&self.value, &mut apply) else { continue };
            if let Kind::Begin { id } | Kind::Update { id, .. } = entry.kind {
                self.replayed = Some((entry.part, id));
            }
            self.events.push((entry.part, entry.kind.event(value.clone()), Hold::Replayed(entry)));
            self.value = value;
            self.drained = true;
            // No input runs before the queue's next entry, nor in this pass.
            order::keep(ctx, key, None);
            return;
        }
        self.entered = false;
    }

    /// The composite's value the parts draw from and map onto: the replay's, then each event of
    /// this pass pushed so far that goes out in this pass.
    #[allow(dead_code, reason = "the composites push operations from DS-13ol T2 on")]
    pub fn value(&self) -> &V {
        &self.value
    }

    /// A part's edit, with the operations `ops` its event made. Called for every part drawn, also
    /// without an event. A Set's value is `ops` applied to [`value`](Edits::value) with `apply`
    /// (a Set with none: its value with `map`); a gesture's event's is `map(part value,
    /// value())`. An event that goes out in this pass leaves [`value`](Edits::value) at its
    /// value; one that waits joins the queue as `ops`.
    #[allow(dead_code, reason = "the composites push operations from DS-13ol T2 on")]
    pub fn push<T: Clone + Send + Sync + 'static>(
        &mut self,
        edit: &EditResponse<T>,
        ops: impl IntoIterator<Item = O>,
        mut apply: impl FnMut(&O, &V) -> Option<V>,
        map: impl Fn(T, &V) -> V,
    ) {
        let (ctx, part) = (&edit.response.ctx, edit.response.id);
        let base = self.value.clone();
        let map = |value: T| map(value, &base);
        self.record(ctx, part, edit.event.as_ref(), &map);
        let Some(event) = &edit.event else { return };
        let Some(kind) = Kind::of(event) else {
            // A Cancel: the run's events that wait never went out.
            let run = event.id();
            self.queue.retain(|q| q.part != part || q.kind.run() != run);
            if self.goes_out(part, run, self.events.len())
                && let EditEvent::Cancel { id, reason } = *event
            {
                self.events.push((part, EditEvent::Cancel { id, reason }, Hold::Nothing));
            }
            return;
        };
        let queued = Queued { part, kind, ops: ops.into_iter().collect() };
        if !self.goes_out(part, kind.run(), self.events.len()) {
            debug_assert!(!queued.ops.is_empty() || matches!(kind, Kind::Begin { .. }), "a held part event needs its operations");
            self.join(queued);
            return;
        }
        let value = match kind {
            Kind::Begin { id } => {
                self.events.push((part, EditEvent::begin(id), Hold::Ops(queued)));
                return;
            }
            Kind::Set { .. } if !queued.ops.is_empty() => queued.apply(&self.value, &mut apply),
            _ => event.value().cloned().map(map),
        };
        // A Set none of whose operations applies changes nothing.
        let Some(value) = value else { return };
        self.value = value.clone();
        let event = kind.event(value);
        self.events.push((part, event, Hold::Ops(queued)));
    }

    /// [`push`](Edits::push) for a field whose focus is a typing run's.
    #[allow(dead_code, reason = "the composites push operations from DS-13ol T2 on")]
    pub fn typing<T: Clone + Send + Sync + 'static>(
        &mut self,
        field: &EditResponse<T>,
        ops: impl IntoIterator<Item = O>,
        apply: impl FnMut(&O, &V) -> Option<V>,
        map: impl Fn(T, &V) -> V,
    ) {
        self.typing.push(field.response.id);
        self.push(field, ops, apply, map);
    }

    /// Adds `entry` to the queue's tail; an Update or Commit takes the place of the queued Begin
    /// or Update of its part's run.
    fn join(&mut self, entry: Queued<O>) {
        if let Kind::Update { id: run, .. } | Kind::Commit { id: run, .. } = entry.kind
            && let Some(at) =
                self.queue.iter().position(|q| q.part == entry.part && matches!(q.kind, Kind::Begin { id } | Kind::Update { id, .. } if id == run))
        {
            self.queue[at] = entry;
            return;
        }
        self.queue.push(entry);
    }

    /// Records that `part` is drawn, and how its gesture under way would end.
    fn record<T: Clone + Send + Sync + 'static>(&mut self, ctx: &Context, part: Id, event: Option<&EditEvent<T>>, map: impl Fn(T) -> V) {
        self.arrive(ctx);
        let run = event.and_then(EditEvent::id).or_else(|| self.run.filter(|(owner, _)| *owner == part).map(|(_, run)| run));
        let end = run.filter(|&run| super::is_live(ctx, run)).map(|run| match super::ending(ctx, run) {
            Ending::Commit { changed } => End::Commit { value: super::unreported::<T>(ctx, run).map(&map), changed },
            Ending::Cancel => End::Cancel,
        });
        self.drawn.push((part, end));
    }

    /// A part's edit, its value mapped to the composite's. Called for every part drawn, also
    /// without an event. For a part with one of the composite's values (a slider, a dial): a
    /// held event's value is mapped onto the composite's value of the pass its turn comes.
    pub fn push_mapped<T: Clone + Send + Sync + 'static>(&mut self, edit: &EditResponse<T>, map: impl Fn(T) -> V) {
        self.push_as(&edit.response.ctx, edit.response.id, edit.event.as_ref(), map);
    }

    /// [`push_mapped`](Edits::push_mapped) for a part whose value holds several of the
    /// composite's values (a graph's points), drawn from `drawn` in this pass. A held event
    /// applies only what it changed from the value its part was drawn from, onto `drawn` of the
    /// pass its turn comes ([`Rebase`]), or is dropped.
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

    /// [`push_mapped`](Edits::push_mapped) for a part whose edit carries another widget's
    /// response (a popover's picker, reported on its swatch): `id` is the part's own.
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
        self.record(ctx, id, event, &map);
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
                    self.events.push((id, event.map(&map), Hold::Value(held.clone())));
                }
            }
        }
        if let (Some(event), Some(new)) = (event, new) {
            self.events.push((id, event.clone().map(&map), Hold::Value(new)));
        }
    }

    /// [`push_mapped`](Edits::push_mapped) for a field whose focus is a typing run's.
    pub fn typing_mapped<T: Clone + Send + Sync + 'static>(&mut self, ctx: &Context, field: &EditResponse<T>, map: impl Fn(T) -> V) {
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
    pub fn inputs(&mut self, ctx: &Context, state: &mut State<O, C>) -> Vec<C> {
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

    /// Whether `part`'s event of this pass goes out in this pass, as far as the parts pushed so
    /// far tell, rather than waiting for its turn ([`feed`](Edits::feed) holds it). A part drawn
    /// after it draws from and maps onto the value that event makes only when it does
    /// ([`Running`]); else onto the composite's shown value. False when the part has no event.
    pub fn applies(&self, part: Id) -> bool {
        let Some(at) = self.events.iter().rposition(|(id, ..)| *id == part) else { return false };
        self.goes_out(part, self.events[at].1.id(), at)
    }

    /// Whether an event of `part` (of its gesture `run`, if any) pushed after the first `at`
    /// events goes out in this pass.
    fn goes_out(&self, part: Id, run: Option<GestureId>, at: usize) -> bool {
        if self.scoped {
            return true;
        }
        // The run whose gesture the queue's entry begins in this pass.
        if run.is_some_and(|run| self.replayed == Some((part, run))) {
            return true;
        }
        if self.ending {
            return false;
        }
        if self.loaded.is_some() {
            return self.owner == Some(part);
        }
        // The queue drains one entry a pass: a new event joins its tail.
        if self.drained || !self.queue.is_empty() {
            return false;
        }
        // Idle: the first part whose event begins a gesture owns it, and the events of the other
        // parts after it wait.
        let begins = |event: &EditEvent<V>| matches!(event, EditEvent::Begin { .. } | EditEvent::Update { .. });
        !self.events[..at].iter().any(|(id, event, _)| *id != part && begins(event))
    }

    fn is_drawn(&self, part: Id) -> bool {
        self.drawn.iter().any(|(id, _)| *id == part)
    }

    /// Whether `owner`'s event of this pass ends its gesture.
    fn owner_ends(&self, owner: Id) -> bool {
        self.events.iter().any(|(id, event, _)| *id == owner && matches!(event, EditEvent::Commit { .. } | EditEvent::Cancel { .. }))
    }

    /// Turns the parts' gestures into the composite's `outer` gesture (see the module docs) and
    /// returns it. Inputs not taken with [`inputs`](Edits::inputs) are dropped.
    pub fn feed(self, outer: Gesture<V>, state: &mut State<O, C>) -> Gesture<V> {
        let scoped = self.scoped;
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
        // Operations held although `push` or `replay` let them go: they wait first.
        let mut back = Vec::new();
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
        for (who, event, hold) in self.events {
            let owner = state.owner == Some(who);
            let other = outer.is_active() && state.owner.is_some_and(|owner| owner != who);
            if !scoped && ((ends && ender != Some(who)) || other) {
                match hold {
                    Hold::Value(raw) => {
                        held.retain(|(part, _)| *part != who);
                        held.push((who, raw));
                        if let Some(ctx) = &self.ctx {
                            order::turn_after(ctx, held_key(self.id, who), None, gesture);
                        }
                    }
                    Hold::Ops(entry) | Hold::Replayed(entry) => back.push(entry),
                    Hold::Nothing => {}
                }
                continue;
            }
            // A gesture the queue's entry begins doesn't run from the composite's start.
            let replayed = matches!(hold, Hold::Replayed(_));
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
                        state.pure = outer.id().filter(|_| !replayed);
                    }
                }
                EditEvent::Update { value, modifiers, .. } => {
                    if !outer.is_active() {
                        outer.begin();
                        state.owner = Some(who);
                        state.pure = outer.id().filter(|_| !replayed);
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
        back.extend(self.queue);
        state.queue = back;
        state.placed = self.drained || !state.queue.is_empty();
        if let Some(ctx) = &self.ctx {
            let began = outer.id().filter(|_| self.drained);
            state.place_queue(ctx, queue_key(self.id), self.entered, began, gesture);
            state.pass = ctx.cumulative_pass_nr();
            if moved || !state.held.is_empty() || !state.queue.is_empty() {
                ctx.request_repaint();
            }
        }
        outer
    }
}

/// The composite's value as the parts pushed so far in a pass leave it. A part pushed after them
/// maps onto it, so every part's event in a frame counts, each from the value the ones before it
/// left: a screen reader's steps on two fields of one row in one frame both reach the host. A
/// part's value is in it only when the part's event goes out in this pass
/// ([`Edits::applies`]); one that waits for its turn isn't.
pub(crate) struct Running<V> {
    value: V,
}

impl<V: Clone + PartialEq + Send + Sync + 'static> Running<V> {
    /// From `value`, the composite's value the first part is drawn from.
    pub fn new(value: V) -> Self {
        Self { value }
    }

    /// The value the next part maps onto.
    pub fn value(&self) -> &V {
        &self.value
    }

    /// After `part` is pushed into `edits`: its event's value, when that goes out in this pass.
    pub fn after<O: Clone + Send + Sync + 'static, C>(&mut self, edits: &Edits<V, O, C>, part: Id) {
        if edits.applies(part)
            && let Some((_, event, _)) = edits.events.iter().rfind(|(id, ..)| *id == part)
            && let Some(value) = event.value()
        {
            self.value = value.clone();
        }
    }
}

/// The edit-order key of part `part`'s held value event in composite `id`.
fn held_key(id: Id, part: Id) -> Id {
    id.with(("craft_ui::edit::parts::held", part))
}

/// The edit-order key of composite `id`'s queue of held operations.
fn queue_key(id: Id) -> Id {
    id.with("craft_ui::edit::parts::queue")
}

/// Whether the composite's `outer` gesture runs from the composite's start ([`State::pure`]).
fn is_pure<V: Clone + PartialEq + Send + Sync + 'static, O, C>(outer: &Gesture<V>, state: &State<O, C>) -> bool {
    outer.id().is_some() && outer.id() == state.pure
}
