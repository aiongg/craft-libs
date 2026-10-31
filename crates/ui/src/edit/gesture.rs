//! [`Gesture`]: the state machine a value control embeds to report [`EditEvent`]s.

use std::sync::Arc;

use egui::{Context, EventFilter, Id, Key, LayerId, Modifiers, Ui, ViewportId};

use super::{CancelAll, CancelReason, EditEvent, EditPolicy, Ending, GestureId, UpdateRate, in_scope, live};

/// The edit state of one value control: idle, or active with a start value and a [`GestureId`].
///
/// A control loads it every frame after interacting, feeds it its input, reads the value to
/// show, and finishes it to get the frame's event:
///
/// ```
/// use craft_ui::edit::{EditPolicy, EditResponse, Gesture};
/// use egui::{Sense, Ui, vec2};
///
/// /// A bare horizontal drag value.
/// fn drag_value(ui: &mut Ui, value: f32, policy: EditPolicy) -> EditResponse<f32> {
///     let (_, rect) = ui.allocate_space(vec2(120.0, 20.0));
///     let response = ui.interact(rect, ui.id().with("drag"), Sense::drag());
///     let mut gesture = Gesture::load(ui, response.id, &value, policy).enabled(response.enabled());
///     if response.drag_started() {
///         gesture.begin();
///     }
///     if response.dragged()
///         && let Some(anchor) = gesture.anchor().copied()
///     {
///         let moved = ui.input(|i| i.pointer.interact_pos().zip(i.pointer.press_origin()));
///         let delta = moved.map_or(0.0, |(pos, origin)| pos.x - origin.x);
///         gesture.update(anchor + delta, ui.input(|i| i.modifiers));
///     }
///     if response.drag_stopped() {
///         gesture.commit();
///     }
///     ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, gesture.shown().to_string(), Default::default(), ui.visuals().text_color());
///     let event = gesture.finish();
///     EditResponse::new(response, event)
/// }
/// # egui::__run_test_ui(|ui| { let _ = drag_value(ui, 1.0, EditPolicy::default()); });
/// ```
///
/// Lifecycle:
/// - [`begin`](Gesture::begin) on the start of a drag or the first keypress of a typing run;
///   the start value is the host's value.
/// - [`update`](Gesture::update) while active; updates go out at the policy's [`UpdateRate`],
///   none with [`on_release_only`](EditPolicy::on_release_only) or for a gesture that
///   [holds them](Gesture::hold_updates).
/// - [`commit`](Gesture::commit) on release or Enter, and on focus loss unless the control
///   [cancels on it](Gesture::cancel_on_focus_loss). The commit goes out in the same frame and
///   carries the final value, which an update may not have carried.
/// - [`load`](Gesture::load) cancels on Esc (and consumes it, so later widgets and the control's
///   own commit on egui's `drag_stopped` don't see it; a focused text field's typing run leaves
///   the press to the host instead, [`EditResponse::escaped`](super::EditResponse::escaped)) when
///   the policy is [`cancellable`](EditPolicy::cancellable) and no modal layer is above the
///   control; on [`cancel_all`](super::cancel_all); and when the host's value changed other than by the
///   gesture's updates ([`CancelReason::External`]); a host's rounding, clamping or mapping of an
///   update's value is not such a change for the `f64` controls (`load_normalised`).
///   [`enabled`](Gesture::enabled) cancels when the control is disabled.
/// - [`set`](Gesture::set) for a one-shot change; during a gesture it is an update.
/// - A control that is not drawn in a pass loses its gesture (see [`is_live`](super::is_live))
///   and starts idle when it is drawn again; the host ends the gesture as [`Ending`] says, a
///   typing run with the control's last value.
/// - A gesture the host ended before its control did (another control began a gesture or set a
///   value, [`GestureHost`](super::GestureHost)) is over: [`load`](Gesture::load) replaces it
///   with a new gesture of the same input (the same drag, typing run or draft), started from the
///   host's value. That gesture reports its [`Begin`](EditEvent::Begin) at its first change, with
///   an id it takes at that change ([`id`](Gesture::id)), and nothing if it ends unchanged, so
///   the input goes on and its later changes reach the host. Its [`start`](Gesture::start) and
///   [`shown`](Gesture::shown) value are the host's value, which holds the input's changes when
///   the host kept the gesture it ended and not when it reverted it (a drag's). So a drag
///   computes its value from the input instead: from [`anchor`](Gesture::anchor) and the
///   pointer's movement since the press, or, for a drag that adds each frame's movement to the
///   value, from the value it last set. A control whose commit also signals that the user
///   finished (a pen draft's Enter) can't rely on the commit for that: an unchanged carried-on
///   gesture reports nothing, so the control reports the finish another way.
///
/// At most one event goes out per frame: a gesture that begins and commits in one frame reports
/// [`Set`](EditEvent::Set) (nothing when the value didn't change). Inside a
/// [`scope`](super::scope) only updates go out.
///
/// The value type's `PartialEq` tells a change from none: an update that changes nothing, a
/// commit that changed nothing, and the host's value moving away from the one the gesture
/// reported (an outside change, which cancels it). So it must hold every value a gesture holds
/// equal to itself. A float that is never NaN while a gesture runs needs nothing: the
/// single-number controls begin no gesture from a value that isn't finite. A type that may hold
/// a NaN during a gesture (an unset value of a composite, a host's sample it carries along)
/// implements `PartialEq` with a NaN equal to another NaN; else its every gesture would read as
/// changed from outside and cancel. Debug builds assert it when a gesture begins and at each
/// update.
#[derive(Debug)]
#[must_use = "finish the gesture to store it and get the frame's event"]
pub struct Gesture<T> {
    ctx: Context,
    widget: Id,
    key: Id,
    layer: LayerId,
    /// The host's value this frame.
    host: T,
    policy: EditPolicy,
    scoped: bool,
    /// The widgets whose focus is the control's.
    owners: Vec<Id>,
    filter: Option<EventFilter>,
    cancel_on_focus_loss: bool,
    commit_on_interrupt: bool,
    pointer_selects: bool,
    enabled: bool,
    now: f64,
    stored: Stored<T>,
    /// A cancel to report this frame.
    cancelled: Option<Cancelled>,
    /// A gesture that ended this frame: its live entry goes.
    ended: Option<GestureId>,
    /// How an interruption ends the gesture, in place of the focus rule
    /// ([`interrupted_as`](Gesture::interrupted_as)).
    interrupted: Option<Interrupted<T>>,
    /// A part's run under way ends the gesture this frame ([`ended_by_part`](Gesture::ended_by_part)).
    by_part: bool,
}

/// The part kept in egui memory between frames.
#[derive(Clone, Debug)]
struct Stored<T> {
    phase: Phase<T>,
    /// A one-shot value not reported yet (it waits a frame when a cancel goes out first).
    set: Option<(T, Modifiers)>,
    /// The viewport and its pass that last loaded the gesture. A control missing from a pass has
    /// been removed: its state is dropped, and the host learns of it with
    /// [`is_live`](super::is_live).
    viewport: ViewportId,
    pass: u64,
    /// Whether the gesture set the focus filter with Esc in it, to take Esc out again.
    escape_locked: bool,
}

impl<T> Stored<T> {
    fn idle(viewport: ViewportId, pass: u64) -> Self {
        Self { phase: Phase::Idle, set: None, viewport, pass, escape_locked: false }
    }
}

#[derive(Clone, Debug)]
enum Phase<T> {
    Idle,
    Active(Active<T>),
    /// Cancelled while the host showed the gesture's value: show `start` until the host's value
    /// is `start` again, or moves away from `host`, its value at the cancel.
    Restoring {
        start: T,
        host: T,
    },
}

#[derive(Clone, Debug)]
struct Active<T> {
    id: GestureId,
    /// The input's first gesture ([`Gesture::input`]).
    input: GestureId,
    /// The [`CancelAll`] generation when the gesture began.
    generation: u64,
    start: T,
    /// The host's value when the input's first gesture began ([`Gesture::anchor`]).
    anchor: T,
    current: T,
    /// The value of the last update that went out (`start` before the first): what the host
    /// shows.
    reported: T,
    modifiers: Modifiers,
    /// Input time of the last change of `current`.
    changed_at: f64,
    /// Whether `Begin` went out.
    announced: bool,
    /// Committed this frame.
    ending: bool,
    /// Whether the control has had focus since the gesture began; losing it then ends the gesture.
    focused: bool,
    /// Whether the pointer has dragged the control during the gesture.
    dragged: bool,
    /// Reports no updates, whatever the policy ([`Gesture::hold_updates`]).
    held: bool,
    /// Carries on the input of a gesture the host ended: `Begin` waits for the first change.
    waits: bool,
    /// The `Begin` carries the value when the frame that begins the gesture already changed it
    /// ([`Gesture::begin_with_value`]).
    first_value: bool,
}

#[derive(Clone, Copy, Debug)]
struct Cancelled {
    id: GestureId,
    reason: CancelReason,
    announced: bool,
}

const STATE: &str = "craft_ui::edit::gesture";

/// Whether `host` is `reported`, an `f64` an update sent, as a host that keeps the value in `f32`
/// shows it: equal to `f32` precision, a few `f32` ulps apart at most. That covers a host that
/// stores the `f32` rounding of it, also through a linear scale (stored 0..1, shown ×100), and
/// moves the host's value by nothing but its own storage, not an outside change. A host that
/// rounds to a step or to display decimals differs by more, and is one.
pub(crate) fn equals_to_f32(host: f64, reported: f64) -> bool {
    host == reported
        || host.is_finite() && reported.is_finite() && (host - reported).abs() <= 2.0 * f64::from(f32::EPSILON) * host.abs().max(reported.abs())
}

/// Whether `host` is the host's own normalisation of `reported`, the value of an update: a host
/// that rounds to a step, clamps to a range or maps the value (a log scale) stores the nearest
/// value it accepts, and `start`, the value the gesture began from, is one it accepted. So a
/// normalised value is no farther from `reported` than `start` is, where an outside change that
/// lands farther off is not one. Covers [`equals_to_f32`]'s hosts too.
fn normalised_by_host(host: &f64, reported: &f64, start: &f64) -> bool {
    let (host, reported, start) = (*host, *reported, *start);
    equals_to_f32(host, reported)
        || host.is_finite() && reported.is_finite() && start.is_finite() && (host - reported).abs() <= (start - reported).abs()
}

impl Gesture<f64> {
    /// [`load`](Gesture::load) for a control whose host may store a normalised form of the
    /// values it receives: the host's value is not an outside change when it is
    /// `normalised_by_host` from the last update's value.
    pub(crate) fn load_normalised(ui: &Ui, id: Id, value: f64, policy: EditPolicy) -> Self {
        Self::load_with(ui, id, &value, policy, true, Some(normalised_by_host))
    }
}

impl<T: Clone + PartialEq + Send + Sync + 'static> Gesture<T> {
    /// The gesture state of the control with widget id `id` (its [`egui::Response::id`]), whose
    /// host value is `value`. Applies a pending [`cancel_all`](super::cancel_all), an external
    /// change of the value, and Esc.
    pub fn load(ui: &Ui, id: Id, value: &T, policy: EditPolicy) -> Self {
        Self::load_with(ui, id, value, policy, true, None)
    }

    /// [`load`](Gesture::load) for a control that leaves Esc to the host: the press cancels as it
    /// does there but stays in the input, so the controls after it and the host see it too (a
    /// focused text field's typing run, whose Esc also closes a dialog).
    pub(crate) fn load_passing_escape(ui: &Ui, id: Id, value: &T, policy: EditPolicy) -> Self {
        Self::load_with(ui, id, value, policy, false, None)
    }

    fn load_with(ui: &Ui, id: Id, value: &T, policy: EditPolicy, take_escape: bool, echo: Option<fn(&T, &T, &T) -> bool>) -> Self {
        let ctx = ui.ctx().clone();
        let key = id.with(STATE);
        let (viewport, pass) = (ctx.viewport_id(), ctx.cumulative_pass_nr());
        let stored = match ctx.data(|d| d.get_temp::<Stored<T>>(key)) {
            Some(stored) if stored.viewport == viewport && stored.pass + 1 >= pass => Stored { pass, ..stored },
            _ => Stored::idle(viewport, pass),
        };
        let now = ctx.input(|i| i.time);
        let mut gesture = Self {
            ctx,
            widget: id,
            key,
            layer: ui.layer_id(),
            host: value.clone(),
            policy,
            scoped: in_scope(ui),
            owners: vec![id],
            filter: None,
            cancel_on_focus_loss: false,
            commit_on_interrupt: false,
            pointer_selects: false,
            enabled: true,
            now,
            stored,
            cancelled: None,
            ended: None,
            interrupted: None,
            by_part: false,
        };
        gesture.end_restoring();
        gesture.apply_cancel_all();
        gesture.carry_on_ended();
        gesture.apply_external(echo);
        gesture.apply_escape(take_escape);
        gesture
    }

    /// Whether losing keyboard focus cancels the gesture ([`CancelReason::FocusLost`]) rather
    /// than committing it; off by default. It also decides how an interrupted typing run ends
    /// ([`Ending`]).
    pub fn cancel_on_focus_loss(mut self, cancel: bool) -> Self {
        self.cancel_on_focus_loss = cancel;
        self
    }

    /// Whether an interrupted gesture ends as a commit, whatever the focus rule of [`Ending`] says:
    /// its control is no longer drawn, or the host ends it because another gesture begins or a
    /// value is set ([`GestureHost`](super::GestureHost)). Off by default. For a control whose
    /// gesture is user work kept as it goes, such as a pen draft's anchors (switching tools
    /// finishes the path). It replaces nothing else: Esc, focus loss and the control's own
    /// commit and cancel still end the gesture as the control says.
    pub fn commit_on_interrupt(mut self, commit: bool) -> Self {
        self.commit_on_interrupt = commit;
        self
    }

    /// Whether a pointer drag on the control selects (a text field's text) rather than changes the
    /// value; off by default. Such a drag doesn't make the gesture a drag, so an interrupted
    /// typing run still ends as a commit ([`Ending`]).
    pub fn pointer_selects(mut self, selects: bool) -> Self {
        self.pointer_selects = selects;
        self
    }

    /// The control's own focus filter (the keys it keeps while focused). Whenever a
    /// [focus owner](Gesture::focus_owners) has focus, [`finish`](Gesture::finish) sets this
    /// filter on it, with Esc added while a cancellable gesture is active outside a
    /// [`scope`](super::scope). Without a filter of its own the control keeps egui's, and the
    /// gesture only adds Esc while such a gesture is active and takes it out once after. A control
    /// wrapping a widget that sets its own filter (egui's `TextEdit` does) passes that widget's
    /// filter here and finishes after showing it.
    pub fn event_filter(mut self, filter: EventFilter) -> Self {
        self.filter = Some(filter);
        self
    }

    /// The widgets whose keyboard focus is the control's; the widget `id` given to
    /// [`load`](Gesture::load) by default. A control whose gesture lives on one widget while the
    /// focus goes to others (a range slider's track and its thumbs) names them here. The focus
    /// rules apply to whichever of them has focus: the end on focus loss (moving the focus between
    /// owners is no loss), the [focus filter](Gesture::event_filter), and how an interrupted
    /// gesture ends ([`Ending`]).
    pub fn focus_owners(mut self, owners: &[Id]) -> Self {
        self.owners = owners.to_vec();
        self
    }

    /// Whether the control is enabled; on by default. Disabling the control cancels its active
    /// gesture ([`CancelReason::Disabled`]); a disabled control begins no gesture and sets no
    /// value.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        if !enabled {
            self.cancel(CancelReason::Disabled);
        }
        self
    }

    /// The policy the control was given.
    pub fn policy(&self) -> EditPolicy {
        self.policy
    }

    /// Whether a gesture is active.
    pub fn is_active(&self) -> bool {
        matches!(self.stored.phase, Phase::Active(_))
    }

    /// The active gesture's id: the one its events go out with. A gesture that carries on an
    /// input the host ended takes its id at its first change, the one its
    /// [`Begin`](EditEvent::Begin) goes out with, so the id is greater than that of every gesture
    /// the host has ended before ([`GestureHost`](super::GestureHost)).
    pub fn id(&self) -> Option<GestureId> {
        match &self.stored.phase {
            Phase::Active(a) => Some(a.id),
            _ => None,
        }
    }

    /// The active gesture of the control with widget id `id` as its last pass left it, read before
    /// the control loads it: the gesture's id and the value of the last update that went out (the
    /// start before the first). For a control that works out which host value it gives
    /// [`load`](Gesture::load) as unmoved.
    pub fn reported(ui: &Ui, id: Id) -> Option<(GestureId, T)> {
        let (viewport, pass) = (ui.ctx().viewport_id(), ui.ctx().cumulative_pass_nr());
        let stored = ui.data(|d| d.get_temp::<Stored<T>>(id.with(STATE))).filter(|s| s.viewport == viewport && s.pass + 1 >= pass)?;
        match stored.phase {
            Phase::Active(a) => Some((a.id, a.reported)),
            _ => None,
        }
    }

    /// The active gesture's start value: the host's value when it began. For a gesture that
    /// carries on an input the host ended, that is the host's value when the host ended it, not
    /// the value the input began from ([`anchor`](Gesture::anchor)).
    pub fn start(&self) -> Option<&T> {
        match &self.stored.phase {
            Phase::Active(a) => Some(&a.start),
            _ => None,
        }
    }

    /// The value the input under way began from: the host's value when its first gesture began.
    /// It stays when a gesture the host ended carries the input on, where
    /// [`start`](Gesture::start) becomes the host's value. A drag that computes its value from
    /// the pointer's movement since the press starts from it, so the value follows the pointer
    /// whether the host kept or reverted the gesture it ended.
    pub fn anchor(&self) -> Option<&T> {
        match &self.stored.phase {
            Phase::Active(a) => Some(&a.anchor),
            _ => None,
        }
    }

    /// The input under way: the id of its first gesture, the same when a gesture the host ended
    /// carries the input on. For a control that keeps track of its own input (a press until its
    /// release) across carry-ons.
    pub(crate) fn input(&self) -> Option<GestureId> {
        match &self.stored.phase {
            Phase::Active(a) => Some(a.input),
            _ => None,
        }
    }

    /// The value the control shows: the gesture's value while active, the start value after a
    /// cancel until the host echoes it, an unreported one-shot value, or else the host's value.
    pub fn shown(&self) -> &T {
        match &self.stored.phase {
            Phase::Active(a) => &a.current,
            Phase::Restoring { start, .. } => start,
            Phase::Idle => self.stored.set.as_ref().map_or(&self.host, |(value, _)| value),
        }
    }

    /// Starts a gesture from the host's value; nothing while one is active or the control is
    /// disabled.
    pub fn begin(&mut self) {
        if self.is_active() || !self.enabled {
            return;
        }
        debug_assert!(equals_itself(&self.host), "{}", NOT_ITSELF);
        let generation = CancelAll::get(&self.ctx).generation;
        let id = GestureId::next();
        self.stored.phase = Phase::Active(Active {
            id,
            input: id,
            generation,
            start: self.host.clone(),
            anchor: self.host.clone(),
            current: self.host.clone(),
            reported: self.host.clone(),
            modifiers: Modifiers::NONE,
            changed_at: self.now,
            announced: false,
            ending: false,
            focused: self.focused_owner().is_some(),
            dragged: self.dragged(),
            held: false,
            waits: false,
            first_value: false,
        });
    }

    /// [`begin`](Gesture::begin) for a drag whose first frame already moves the value: if the
    /// value differs from the start when the `Begin` goes out, the `Begin` carries it, so the
    /// host shows the first move in that frame. Still one event per frame. The `Begin` of a
    /// gesture inside a part of a composite carries the value for the part, and the composite
    /// begins its own gesture with it in the same frame.
    pub fn begin_with_value(&mut self) {
        self.begin();
        if let Phase::Active(a) = &mut self.stored.phase {
            a.first_value = true;
        }
    }

    /// Whether the active gesture holds its updates: while it does it reports none, whatever the
    /// policy says, as [`on_release_only`](EditPolicy::on_release_only) does, and its commit
    /// carries its value; once it stops, the value not reported yet goes out as an update. For a
    /// gesture whose values in between may mean nothing to the host, such as a number field's
    /// typing run (`-` and `5+` aren't numbers yet), while the control's other gestures report at
    /// the policy's rate. Lasts until the gesture ends or the next call.
    pub fn hold_updates(&mut self, hold: bool) {
        if let Phase::Active(a) = &mut self.stored.phase {
            a.held = hold;
        }
    }

    /// How the active gesture ends if it is interrupted after this frame (its control stops being
    /// drawn, or the host ends it), in place of the focus rule of [`Ending`] and
    /// [`commit_on_interrupt`](Gesture::commit_on_interrupt). For a composite whose gesture is a
    /// part's: as that part's gesture would end, with its value as the composite's
    /// ([`unreported`](super::unreported)). The composite gives it again each frame.
    pub(crate) fn interrupted_as(&mut self, interrupted: Interrupted<T>) {
        if self.is_active() {
            self.interrupted = Some(interrupted);
        }
    }

    /// For a composite whose gesture is a part's typing run, while that run is under way in this
    /// frame: losing the focus doesn't end the gesture, the part's ending event does. The part
    /// sees the focus go in the next frame when the control that took it is drawn after it.
    pub(crate) fn ended_by_part(&mut self) {
        self.by_part = true;
    }

    /// The gesture's new value and the modifiers held; nothing while idle or once committed.
    pub fn update(&mut self, value: T, modifiers: Modifiers) {
        if let Phase::Active(a) = &mut self.stored.phase
            && !a.ending
        {
            debug_assert!(equals_itself(&value), "{}", NOT_ITSELF);
            a.modifiers = modifiers;
            if a.current != value {
                a.current = value;
                a.changed_at = self.now;
                // A carried-on gesture announces itself in the frame of a change. An id taken
                // earlier could be older than a gesture the host ended meanwhile, and the host
                // would ignore this one.
                if a.waits && !a.announced {
                    a.id = GestureId::next();
                }
            }
        }
    }

    /// Ends the gesture with its current value.
    pub fn commit(&mut self) {
        if let Phase::Active(a) = &mut self.stored.phase {
            a.ending = true;
        }
    }

    /// Abandons the gesture. When the host shows the gesture's value, the control shows the
    /// start value until the host echoes it.
    pub fn cancel(&mut self, reason: CancelReason) {
        let phase = std::mem::replace(&mut self.stored.phase, Phase::Idle);
        match phase {
            Phase::Active(a) => {
                self.cancelled = Some(Cancelled { id: a.id, reason, announced: a.announced });
                self.ended = Some(a.id);
                if a.announced && !self.scoped && self.host == a.reported && self.host != a.start {
                    self.stored.phase = Phase::Restoring { start: a.start, host: self.host.clone() };
                }
            }
            other => self.stored.phase = other,
        }
    }

    /// A one-shot change (toggle, arrow-key nudge, wheel step, Enter): a [`Set`](EditEvent::Set)
    /// with the modifiers held, when idle and `value` differs from the host's; an update during a
    /// gesture; nothing while the control is disabled.
    pub fn set(&mut self, value: T) {
        if !self.enabled {
            return;
        }
        match &self.stored.phase {
            Phase::Active(a) if a.ending => {}
            Phase::Active(a) => {
                let modifiers = a.modifiers;
                self.update(value, modifiers);
            }
            Phase::Idle | Phase::Restoring { .. } => {
                debug_assert!(equals_itself(&value), "{}", NOT_ITSELF);
                self.stored.phase = Phase::Idle;
                let modifiers = self.ctx.input(|i| i.modifiers);
                self.stored.set = (value != self.host).then_some((value, modifiers));
            }
        }
    }

    /// Whether the gesture ends in this frame: it committed, or a cancel goes out.
    pub(crate) fn is_ending(&self) -> bool {
        self.cancelled.is_some() || matches!(&self.stored.phase, Phase::Active(a) if a.ending)
    }

    /// Stores the state and returns this frame's event.
    pub fn finish(self) -> Option<EditEvent<T>> {
        self.finish_as(|value| value)
    }

    /// [`finish`](Gesture::finish) for a control whose events carry its value as another type
    /// (a number field's blank `None` reported as the host's value): `report` maps this frame's
    /// event, and the value an interrupted gesture commits with that no update reported, so the
    /// host gets that value in the type of the events it handles ([`unreported`](super::unreported)).
    pub fn finish_as<U: Clone + Send + Sync + 'static>(mut self, report: impl Fn(T) -> U) -> Option<EditEvent<U>> {
        self.apply_focus();
        let event = self.resolve();
        self.schedule();
        self.lock_focus();
        self.record_live(&report);
        let Self { ctx, key, stored, .. } = self;
        ctx.data_mut(|d| d.insert_temp(key, stored));
        event.map(|event| event.map(report))
    }

    /// Leaves the restoring state once the host has acted on the cancel.
    fn end_restoring(&mut self) {
        if let Phase::Restoring { start, host } = &self.stored.phase
            && (self.host == *start || self.host != *host)
        {
            self.stored.phase = Phase::Idle;
        }
    }

    fn apply_cancel_all(&mut self) {
        let all = CancelAll::get(&self.ctx);
        if let Phase::Active(a) = &self.stored.phase
            && all.generation > a.generation
            && let Some(reason) = all.reason
        {
            self.cancel(reason);
        }
    }

    /// Replaces a gesture the host ended with a new one of the same input, started from the host's
    /// value; its `Begin` waits for its first change, which gives it its id
    /// ([`update`](Gesture::update)). The input, its anchor, the focus, the drag and held updates
    /// go on. Runs after
    /// [`apply_cancel_all`](Gesture::apply_cancel_all): an input that a `cancel_all` stops
    /// doesn't go on.
    fn carry_on_ended(&mut self) {
        let generation = CancelAll::get(&self.ctx).generation;
        let Phase::Active(a) = &mut self.stored.phase else { return };
        if !a.announced || !live::ended(&self.ctx, a.id) {
            return;
        }
        debug_assert!(equals_itself(&self.host), "{}", NOT_ITSELF);
        live::forget(&self.ctx, a.id);
        *a = Active {
            id: GestureId::next(),
            generation,
            start: self.host.clone(),
            current: self.host.clone(),
            reported: self.host.clone(),
            changed_at: self.now,
            announced: false,
            ending: false,
            waits: true,
            ..a.clone()
        };
    }

    /// Cancels when the host's value is no longer the one the gesture reported.
    fn apply_external(&mut self, echo: Option<fn(&T, &T, &T) -> bool>) {
        if let Phase::Active(a) = &self.stored.phase
            && self.host != a.reported
            && !echo.is_some_and(|echo| echo(&self.host, &a.reported, &a.start))
        {
            self.cancel(CancelReason::External);
        }
    }

    /// Cancels on Esc, unless a modal layer is above the control: Esc then closes the modal.
    /// `take` takes the press out of the input.
    fn apply_escape(&mut self, take: bool) {
        let armed = matches!(&self.stored.phase, Phase::Active(a) if !a.ending) && self.policy.cancellable && !self.scoped;
        if !armed || !self.ctx.memory(|m| m.is_above_modal_layer(self.layer)) {
            return;
        }
        let pressed = match take {
            true => self.ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)),
            false => self.ctx.input(escape_pressed),
        };
        if pressed {
            self.cancel(CancelReason::Escape);
        }
    }

    /// Whether the pointer drags the control's value.
    fn dragged(&self) -> bool {
        !self.pointer_selects && self.ctx.is_being_dragged(self.widget)
    }

    /// The focus owner that has focus.
    fn focused_owner(&self) -> Option<Id> {
        self.ctx.memory(|m| self.owners.iter().copied().find(|&id| m.has_focus(id)))
    }

    /// Ends the gesture when the control has lost the focus it had during it.
    fn apply_focus(&mut self) {
        let focused = self.focused_owner().is_some();
        let Phase::Active(a) = &mut self.stored.phase else { return };
        if focused || a.ending || self.by_part {
            a.focused |= focused;
            return;
        }
        if a.focused {
            match self.cancel_on_focus_loss {
                true => self.cancel(CancelReason::FocusLost),
                false => self.commit(),
            }
        }
    }

    /// The one event of this frame.
    fn resolve(&mut self) -> Option<EditEvent<T>> {
        if let Some(c) = self.cancelled.take()
            && c.announced
            && !self.scoped
        {
            return Some(EditEvent::Cancel { id: c.id, reason: c.reason });
        }
        if let Some((value, modifiers)) = self.stored.set.take() {
            return Some(match self.scoped {
                true => EditEvent::Update { id: GestureId::next(), value, modifiers },
                false => EditEvent::Set { value, modifiers },
            });
        }
        let Phase::Active(a) = &mut self.stored.phase else { return None };
        if a.ending {
            return self.end();
        }
        if !a.announced {
            if a.waits && a.current == a.start {
                return None;
            }
            a.announced = true;
            if !self.scoped {
                let value = (a.first_value && a.current != a.start).then(|| a.current.clone());
                if value.is_some() {
                    a.reported = a.current.clone();
                }
                return Some(EditEvent::Begin { id: a.id, value });
            }
        }
        let due = match self.policy.update {
            UpdateRate::Debounced(wait) => self.now - a.changed_at >= wait.as_secs_f64(),
            UpdateRate::Realtime => true,
        };
        if self.policy.on_release_only || a.held || a.current == a.reported || !due {
            return None;
        }
        a.reported = a.current.clone();
        Some(EditEvent::Update { id: a.id, value: a.current.clone(), modifiers: a.modifiers })
    }

    /// The committed gesture's event: its commit, or a set when it began this frame; inside a
    /// scope, an update with a value the host hasn't seen.
    fn end(&mut self) -> Option<EditEvent<T>> {
        let Phase::Active(a) = std::mem::replace(&mut self.stored.phase, Phase::Idle) else { return None };
        self.ended = Some(a.id);
        let changed = a.current != a.start;
        let modifiers = self.ctx.input(|i| i.modifiers);
        match (self.scoped, a.announced) {
            (true, _) => (a.current != a.reported).then_some(EditEvent::Update { id: a.id, value: a.current, modifiers: a.modifiers }),
            (false, false) => changed.then_some(EditEvent::Set { value: a.current, modifiers }),
            (false, true) => Some(EditEvent::Commit { id: a.id, value: a.current, changed, modifiers }),
        }
    }

    /// Asks for the frame in which the next pending event goes out.
    fn schedule(&self) {
        if self.stored.set.is_some() {
            self.ctx.request_repaint();
            return;
        }
        let Phase::Active(a) = &self.stored.phase else { return };
        if self.policy.on_release_only || a.held || a.current == a.reported {
            return;
        }
        match self.policy.update {
            UpdateRate::Debounced(wait) => {
                let left = (wait.as_secs_f64() - (self.now - a.changed_at)).max(0.0);
                self.ctx.request_repaint_after_secs(left as f32);
            }
            UpdateRate::Realtime => self.ctx.request_repaint(),
        }
    }

    /// Sets the control's own filter on the focused owner, with Esc while a cancellable gesture is
    /// active, so Esc cancels the gesture instead of dropping the focus.
    fn lock_focus(&mut self) {
        let owner = self.focused_owner();
        let lock = owner.is_some() && self.is_active() && self.policy.cancellable && !self.scoped;
        if let Some(owner) = owner
            && (lock || self.filter.is_some() || self.stored.escape_locked)
        {
            let own = self.filter.unwrap_or_default();
            let filter = EventFilter { escape: own.escape || lock, ..own };
            self.ctx.memory_mut(|m| m.set_focus_lock_filter(owner, filter));
        }
        self.stored.escape_locked = lock;
    }

    /// Records the active gesture live in this pass with how it ends if interrupted (a commit
    /// carries the value no update reported) and whether it is a typing run, or records that one
    /// ended. A gesture waiting for its first change isn't recorded until its `Begin` goes out.
    ///
    /// A typing run has the focus, isn't a pointer drag, and ends by the focus rule when the
    /// focus leaves: not a gesture that [commits on interrupt](Gesture::commit_on_interrupt), and
    /// not a composite's, which ends as its part's does ([`interrupted_as`](Gesture::interrupted_as)).
    fn record_live<U: Clone + Send + Sync + 'static>(&mut self, report: &impl Fn(T) -> U) {
        let dragged = self.dragged();
        let focus = self.focused_owner();
        let focused = focus.is_some();
        let (cancel_on_focus_loss, commit_on_interrupt) = (self.cancel_on_focus_loss, self.commit_on_interrupt);
        let by_focus = !commit_on_interrupt && self.interrupted.is_none();
        if let Some(id) = self.ended.filter(|&id| self.id() != Some(id)) {
            live::finish(&self.ctx, id);
        }
        match &mut self.stored.phase {
            // A gesture the host hasn't seen begin has nothing to end.
            Phase::Active(a) if !a.announced => a.dragged |= dragged,
            Phase::Active(a) => {
                a.dragged |= dragged;
                let commits = match &self.interrupted {
                    Some(Interrupted::Commit(_)) => true,
                    Some(Interrupted::Cancel) => false,
                    None => commit_on_interrupt || (a.focused && !a.dragged && !cancel_on_focus_loss),
                };
                let (ending, unreported) = match commits {
                    true => {
                        let last = match &self.interrupted {
                            Some(Interrupted::Commit(Some(value))) => value,
                            _ => &a.current,
                        };
                        let unreported = (*last != a.reported).then(|| Arc::new(report(last.clone())) as live::Value);
                        (Ending::Commit { changed: *last != a.start }, unreported)
                    }
                    false => (Ending::Cancel, None),
                };
                let typing = focus.filter(|_| by_focus && a.focused && !a.dragged);
                live::mark(&self.ctx, a.id, ending, unreported, focused, typing);
            }
            Phase::Idle | Phase::Restoring { .. } => {}
        }
    }
}

/// How a composite's gesture ends if it is interrupted, as the part that owns it would end
/// ([`Gesture::interrupted_as`]).
#[derive(Clone, Debug)]
pub(crate) enum Interrupted<T> {
    /// A commit, with this value in place of the gesture's own when given.
    Commit(Option<T>),
    Cancel,
}

/// Whether `value` equals itself, as the value of every gesture must ([`Gesture`]).
#[expect(clippy::eq_op, reason = "the check is whether equality is reflexive")]
fn equals_itself<T: PartialEq>(value: &T) -> bool {
    value == value
}

const NOT_ITSELF: &str =
    "a gesture's value must equal itself: a value type that may hold a NaN during a gesture compares it equal to another NaN (see `Gesture`)";

/// Whether this pass's input has a press of Esc that `consume_key(Modifiers::NONE, Key::Escape)`
/// would take.
pub(crate) fn escape_pressed(input: &egui::InputState) -> bool {
    input.events.iter().any(
        |event| matches!(event, egui::Event::Key { key: Key::Escape, pressed: true, modifiers, .. } if modifiers.matches_logically(Modifiers::NONE)),
    )
}
