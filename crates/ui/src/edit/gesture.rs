//! [`Gesture`]: the state machine a value control embeds to report [`EditEvent`]s.

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
///     let mut gesture = Gesture::load(ui, response.id, &value, policy);
///     if response.drag_started() {
///         gesture.begin();
///     }
///     if response.dragged()
///         && let Some(start) = gesture.start().copied()
///     {
///         let moved = ui.input(|i| i.pointer.interact_pos().zip(i.pointer.press_origin()));
///         let delta = moved.map_or(0.0, |(pos, origin)| pos.x - origin.x);
///         gesture.update(start + delta, ui.input(|i| i.modifiers));
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
///   none with [`on_release_only`](EditPolicy::on_release_only).
/// - [`commit`](Gesture::commit) on release or Enter, and on focus loss unless the control
///   [cancels on it](Gesture::cancel_on_focus_loss). The commit goes out in the same frame and
///   carries the final value, which an update may not have carried.
/// - [`load`](Gesture::load) cancels on Esc (and consumes it, so later widgets and the control's
///   own commit on egui's `drag_stopped` don't see it) when the policy is
///   [`cancellable`](EditPolicy::cancellable) and no modal layer is above the control; on
///   [`cancel_all`](super::cancel_all); and when the host's value changed other than by the
///   gesture's updates ([`CancelReason::External`]).
/// - [`set`](Gesture::set) for a one-shot change; during a gesture it is an update.
/// - A control that is not drawn in a pass loses its gesture (see [`is_live`](super::is_live))
///   and starts idle when it is drawn again.
///
/// At most one event goes out per frame: a gesture that begins and commits in one frame reports
/// [`Set`](EditEvent::Set) (nothing when the value didn't change). Inside a
/// [`scope`](super::scope) only updates go out.
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
    filter: EventFilter,
    cancel_on_focus_loss: bool,
    now: f64,
    stored: Stored<T>,
    /// A cancel to report this frame.
    cancelled: Option<Cancelled>,
    /// A gesture that ended this frame: its live entry goes.
    ended: Option<GestureId>,
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
    /// Whether the gesture set the focus filter with Esc in it, to put the control's own back.
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
    /// The [`CancelAll`] generation when the gesture began.
    generation: u64,
    start: T,
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
}

#[derive(Clone, Copy, Debug)]
struct Cancelled {
    id: GestureId,
    reason: CancelReason,
    announced: bool,
}

const STATE: &str = "craft_ui::edit::gesture";

impl<T: Clone + PartialEq + Send + Sync + 'static> Gesture<T> {
    /// The gesture state of the control with widget id `id` (its [`egui::Response::id`]), whose
    /// host value is `value`. Applies a pending [`cancel_all`](super::cancel_all), an external
    /// change of the value, and Esc.
    pub fn load(ui: &Ui, id: Id, value: &T, policy: EditPolicy) -> Self {
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
            filter: EventFilter::default(),
            cancel_on_focus_loss: false,
            now,
            stored,
            cancelled: None,
            ended: None,
        };
        gesture.end_restoring();
        gesture.apply_cancel_all();
        gesture.apply_external();
        gesture.apply_escape();
        gesture
    }

    /// Whether losing keyboard focus cancels the gesture ([`CancelReason::FocusLost`]) rather
    /// than committing it; off by default. It also decides how an interrupted typing run ends
    /// ([`Ending`]).
    pub fn cancel_on_focus_loss(mut self, cancel: bool) -> Self {
        self.cancel_on_focus_loss = cancel;
        self
    }

    /// The control's own focus filter (the keys it keeps while focused). While a cancellable
    /// gesture is active and the control has focus, [`finish`](Gesture::finish) sets this
    /// filter with Esc added, and puts it back without Esc when the gesture ends; otherwise it
    /// leaves the focus filter alone. A control wrapping a widget that sets its own filter (egui's
    /// `TextEdit` does) passes that widget's filter here and finishes after showing it.
    pub fn event_filter(mut self, filter: EventFilter) -> Self {
        self.filter = filter;
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

    /// The active gesture's id.
    pub fn id(&self) -> Option<GestureId> {
        match &self.stored.phase {
            Phase::Active(a) => Some(a.id),
            _ => None,
        }
    }

    /// The active gesture's start value: the host's value when it began.
    pub fn start(&self) -> Option<&T> {
        match &self.stored.phase {
            Phase::Active(a) => Some(&a.start),
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

    /// Starts a gesture from the host's value; nothing while one is active.
    pub fn begin(&mut self) {
        if self.is_active() {
            return;
        }
        let generation = CancelAll::get(&self.ctx).generation;
        self.stored.phase = Phase::Active(Active {
            id: GestureId::next(&self.ctx),
            generation,
            start: self.host.clone(),
            current: self.host.clone(),
            reported: self.host.clone(),
            modifiers: Modifiers::NONE,
            changed_at: self.now,
            announced: false,
            ending: false,
            focused: self.ctx.memory(|m| m.has_focus(self.widget)),
            dragged: self.ctx.is_being_dragged(self.widget),
        });
    }

    /// The gesture's new value and the modifiers held; nothing while idle or once committed.
    pub fn update(&mut self, value: T, modifiers: Modifiers) {
        if let Phase::Active(a) = &mut self.stored.phase
            && !a.ending
        {
            a.modifiers = modifiers;
            if a.current != value {
                a.current = value;
                a.changed_at = self.now;
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
    /// gesture.
    pub fn set(&mut self, value: T) {
        match &self.stored.phase {
            Phase::Active(a) if a.ending => {}
            Phase::Active(a) => {
                let modifiers = a.modifiers;
                self.update(value, modifiers);
            }
            Phase::Idle | Phase::Restoring { .. } => {
                self.stored.phase = Phase::Idle;
                let modifiers = self.ctx.input(|i| i.modifiers);
                self.stored.set = (value != self.host).then_some((value, modifiers));
            }
        }
    }

    /// Stores the state and returns this frame's event.
    pub fn finish(mut self) -> Option<EditEvent<T>> {
        self.apply_focus();
        let event = self.resolve();
        self.schedule();
        self.lock_focus();
        self.record_live();
        let Self { ctx, key, stored, .. } = self;
        ctx.data_mut(|d| d.insert_temp(key, stored));
        event
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

    /// Cancels when the host's value is no longer the one the gesture reported.
    fn apply_external(&mut self) {
        if let Phase::Active(a) = &self.stored.phase
            && self.host != a.reported
        {
            self.cancel(CancelReason::External);
        }
    }

    /// Cancels on Esc, unless a modal layer is above the control: Esc then closes the modal.
    fn apply_escape(&mut self) {
        let armed = matches!(&self.stored.phase, Phase::Active(a) if !a.ending) && self.policy.cancellable && !self.scoped;
        if armed && self.ctx.memory(|m| m.is_above_modal_layer(self.layer)) && self.ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            self.cancel(CancelReason::Escape);
        }
    }

    /// Ends the gesture when the control has lost the focus it had during it.
    fn apply_focus(&mut self) {
        let focused = self.ctx.memory(|m| m.has_focus(self.widget));
        let Phase::Active(a) = &mut self.stored.phase else { return };
        if focused || a.ending {
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
                true => EditEvent::Update { id: GestureId::next(&self.ctx), value, modifiers },
                false => EditEvent::Set { value, modifiers },
            });
        }
        let Phase::Active(a) = &mut self.stored.phase else { return None };
        if a.ending {
            return self.end();
        }
        if !a.announced {
            a.announced = true;
            if !self.scoped {
                return Some(EditEvent::Begin { id: a.id });
            }
        }
        let due = match self.policy.update {
            UpdateRate::Debounced(wait) => self.now - a.changed_at >= wait.as_secs_f64(),
            UpdateRate::Realtime => true,
        };
        if self.policy.on_release_only || a.current == a.reported || !due {
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
        if self.policy.on_release_only || a.current == a.reported {
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

    /// Keeps the focus on Esc while a cancellable gesture is active, so Esc cancels the gesture
    /// instead of dropping the focus; puts the control's own filter back after.
    fn lock_focus(&mut self) {
        let focused = self.ctx.memory(|m| m.has_focus(self.widget));
        let lock = focused && self.is_active() && self.policy.cancellable && !self.scoped;
        if lock || (focused && self.stored.escape_locked) {
            let filter = EventFilter { escape: self.filter.escape || lock, ..self.filter };
            self.ctx.memory_mut(|m| m.set_focus_lock_filter(self.widget, filter));
        }
        self.stored.escape_locked = lock;
    }

    /// Records the active gesture live in this pass with how it ends if interrupted, or drops
    /// the entry of one that ended.
    fn record_live(&mut self) {
        let dragged = self.ctx.is_being_dragged(self.widget);
        let cancel_on_focus_loss = self.cancel_on_focus_loss;
        match &mut self.stored.phase {
            Phase::Active(a) => {
                a.dragged |= dragged;
                let ending = match a.focused && !a.dragged && !cancel_on_focus_loss {
                    true => Ending::Commit { changed: a.reported != a.start },
                    false => Ending::Cancel,
                };
                live::mark(&self.ctx, a.id, ending);
            }
            Phase::Idle | Phase::Restoring { .. } => {
                if let Some(id) = self.ended {
                    live::forget(&self.ctx, id);
                }
            }
        }
    }
}
