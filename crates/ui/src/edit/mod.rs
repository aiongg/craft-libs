//! The edit lifecycle: how a value control reports a change to the application.
//!
//! A value control (slider, drag value, text field, colour picker) returns an [`EditResponse`]:
//! its [`Response`] and at most one [`EditEvent`] per frame. A continuous change is a gesture:
//! [`Begin`](EditEvent::Begin), [`Update`](EditEvent::Update)s, then
//! [`Commit`](EditEvent::Commit) or [`Cancel`](EditEvent::Cancel). A one-shot change (a toggle,
//! an arrow-key nudge, a wheel step, Enter) is a [`Set`](EditEvent::Set).
//!
//! The application owns the value and its "before" state. It applies every update before the
//! next frame; a value that changes any other way during a gesture cancels it
//! ([`CancelReason::External`]). The widget keeps only its start value, which it shows after a
//! cancel until the application echoes the restored value.
//!
//! - Controls embed a [`Gesture`], the state machine that turns their input into events.
//! - [`EditPolicy`] says how often a control reports updates and whether Esc cancels.
//! - [`cancel_all`] cancels every active gesture, e.g. on a document switch or an undo.
//! - A gesture whose control is no longer drawn is over; the host learns of it with [`is_live`]
//!   and ends it as [`ending`] says, or lets [`GestureHost::end_frame`] do that.
//! - [`scope`] is for dialogs: controls inside it report only [`Update`](EditEvent::Update)s,
//!   and the dialog owns one [`OuterGesture`].
//! - [`GestureHost`] turns the events into undo steps for an application that keeps undo marks
//!   ([`UndoMarks`]).
//!
//! Named `EditEvent` rather than `Edit`, which is [`crate::ime::Edit`].

mod gesture;
mod host;
mod live;
mod story;
#[cfg(test)]
mod tests;

use std::time::Duration;

use egui::{Context, Id, InnerResponse, Modifiers, Response, Ui, UiBuilder, UiStackInfo};

pub use gesture::Gesture;
pub use host::{GestureHost, UndoMarks};
pub(crate) use story::{RATES, describe, policy, stories};

/// One gesture, unique within its [`Context`]: a new gesture never reuses an earlier id, and a
/// later gesture has a greater id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GestureId(u64);

impl GestureId {
    /// The next unused id of `ctx`.
    fn next(ctx: &Context) -> Self {
        ctx.data_mut(|d| {
            let next = d.get_temp_mut_or_default::<NextGestureId>(Id::unique(NEXT_GESTURE_ID));
            next.0 += 1;
            GestureId(next.0)
        })
    }
}

/// `#3`, for logs.
impl std::fmt::Display for GestureId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

#[derive(Clone, Copy, Default)]
struct NextGestureId(u64);

const NEXT_GESTURE_ID: &str = "craft_ui::edit::next_gesture_id";

/// What a value control reports in a frame.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum EditEvent<T> {
    /// A gesture started; the host takes its "before" state (e.g. an undo mark).
    #[non_exhaustive]
    Begin { id: GestureId },
    /// The gesture's value changed, at the policy's [`UpdateRate`]; the host previews it.
    #[non_exhaustive]
    Update { id: GestureId, value: T, modifiers: Modifiers },
    /// The gesture ended with `value`, which the host applies: an update may not have carried it
    /// (a debounced value, or the change of the commit frame). `changed` is false when `value`
    /// equals the start value, so the host makes no undo step. `modifiers` are the ones held at
    /// the end.
    #[non_exhaustive]
    Commit { id: GestureId, value: T, changed: bool, modifiers: Modifiers },
    /// The gesture was abandoned; the host restores its "before" state.
    Cancel { id: GestureId, reason: CancelReason },
    /// A one-shot change outside a gesture: a toggle, an arrow-key nudge, a wheel step, Enter.
    #[non_exhaustive]
    Set { value: T, modifiers: Modifiers },
}

impl<T> EditEvent<T> {
    /// The gesture the event belongs to; `None` for [`Set`](EditEvent::Set).
    pub fn id(&self) -> Option<GestureId> {
        match self {
            EditEvent::Begin { id } | EditEvent::Update { id, .. } | EditEvent::Commit { id, .. } | EditEvent::Cancel { id, .. } => Some(*id),
            EditEvent::Set { .. } => None,
        }
    }

    /// The value the event carries, if any.
    pub fn value(&self) -> Option<&T> {
        match self {
            EditEvent::Update { value, .. } | EditEvent::Commit { value, .. } | EditEvent::Set { value, .. } => Some(value),
            EditEvent::Begin { .. } | EditEvent::Cancel { .. } => None,
        }
    }

    /// The same event with its value converted, e.g. from a control's value to a document field.
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> EditEvent<U> {
        match self {
            EditEvent::Begin { id } => EditEvent::Begin { id },
            EditEvent::Update { id, value, modifiers } => EditEvent::Update { id, value: f(value), modifiers },
            EditEvent::Commit { id, value, changed, modifiers } => EditEvent::Commit { id, value: f(value), changed, modifiers },
            EditEvent::Cancel { id, reason } => EditEvent::Cancel { id, reason },
            EditEvent::Set { value, modifiers } => EditEvent::Set { value: f(value), modifiers },
        }
    }
}

/// Why a gesture was cancelled.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CancelReason {
    /// The user pressed Esc during the gesture.
    Escape,
    /// The control lost keyboard focus, for a control that cancels on it
    /// ([`Gesture::cancel_on_focus_loss`]).
    FocusLost,
    /// The host switched documents ([`cancel_all`]).
    DocumentSwitched,
    /// The user asked to undo during the gesture ([`cancel_all`]).
    UndoDuringGesture,
    /// The control was not drawn in a pass, so no control is left to end the gesture
    /// ([`is_live`], [`GestureHost::end_frame`]).
    Removed,
    /// The host's value changed during the gesture other than by its updates (another control
    /// or a command set it). The control shows the host's value; the change stays.
    External,
    /// The control was disabled during the gesture ([`Gesture::enabled`]).
    Disabled,
}

/// How a gesture ends when it is interrupted: its control is no longer drawn, or another gesture
/// begins at the host. It follows the control's focus-loss rule: a typing run (a gesture whose
/// control has the focus, not a drag) commits unless the control
/// [cancels on focus loss](Gesture::cancel_on_focus_loss); a drag cancels. A debounced value the
/// host hasn't seen yet is lost.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Ending {
    /// The host keeps the value it has seen as one undo step; `changed` is false when that value
    /// is the start value, so the host reverts instead.
    Commit { changed: bool },
    /// The host reverts the gesture.
    Cancel,
}

/// How a control reports a gesture; the host chooses it per control. Built from
/// [`EditPolicy::default`] with the builder methods.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct EditPolicy {
    /// How often [`Update`](EditEvent::Update)s go out during a gesture.
    pub update: UpdateRate,
    /// What a preview costs the host. Controls pass it on; the host picks its preview quality.
    pub cost: Cost,
    /// Whether Esc cancels a gesture. Without it, Esc ends a drag as a commit.
    pub cancellable: bool,
    /// No [`Update`](EditEvent::Update)s: the host sees only the [`Commit`](EditEvent::Commit)
    /// with the final value.
    pub on_release_only: bool,
}

impl Default for EditPolicy {
    /// Realtime updates of a cheap preview, cancellable with Esc.
    fn default() -> Self {
        Self { update: UpdateRate::Realtime, cost: Cost::Cheap, cancellable: true, on_release_only: false }
    }
}

impl EditPolicy {
    /// Sets [`update`](EditPolicy::update).
    #[must_use]
    pub fn update(mut self, update: UpdateRate) -> Self {
        self.update = update;
        self
    }

    /// Sets [`cost`](EditPolicy::cost).
    #[must_use]
    pub fn cost(mut self, cost: Cost) -> Self {
        self.cost = cost;
        self
    }

    /// Sets [`cancellable`](EditPolicy::cancellable).
    #[must_use]
    pub fn cancellable(mut self, cancellable: bool) -> Self {
        self.cancellable = cancellable;
        self
    }

    /// Sets [`on_release_only`](EditPolicy::on_release_only).
    #[must_use]
    pub fn on_release_only(mut self, on_release_only: bool) -> Self {
        self.on_release_only = on_release_only;
        self
    }
}

/// How often a gesture reports its value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum UpdateRate {
    /// An update in every frame the value changes (egui delivers input once per frame).
    Realtime,
    /// An update once the value has held still for the duration; changes within it coalesce into
    /// one update, and a value still waiting at the end goes out with the commit. Timed by egui's
    /// input time ([`egui::InputState::time`]), so the integration sets [`egui::RawInput::time`]
    /// every frame (eframe does).
    Debounced(Duration),
}

/// What a preview of the value costs the host: a hint for its preview quality.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Cost {
    /// A full-quality preview every update is affordable.
    #[default]
    Cheap,
    /// The host should preview at reduced quality during the gesture and render fully on commit.
    Expensive,
}

/// What every value control returns: its response and this frame's event, if any.
#[derive(Clone, Debug)]
#[non_exhaustive]
#[must_use = "the host applies the edit event"]
pub struct EditResponse<T> {
    pub response: Response,
    pub event: Option<EditEvent<T>>,
}

impl<T> EditResponse<T> {
    pub fn new(response: Response, event: Option<EditEvent<T>>) -> Self {
        Self { response, event }
    }

    /// The value of a finished change that the host keeps as an undo step: a
    /// [`Set`](EditEvent::Set), or a [`Commit`](EditEvent::Commit) that changed the value.
    pub fn committed(&self) -> Option<&T> {
        match &self.event {
            Some(EditEvent::Set { value, .. } | EditEvent::Commit { value, changed: true, .. }) => Some(value),
            _ => None,
        }
    }

    /// The same response with the event's value converted ([`EditEvent::map`]).
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> EditResponse<U> {
        EditResponse { response: self.response, event: self.event.map(|e| e.map(f)) }
    }
}

/// Whether gesture `id`'s control was drawn in the current pass of its viewport. Call it in that
/// viewport, after the controls: a gesture that is not live is over, its control starts idle if
/// it comes back, and the host ends it as [`ending`] says ([`GestureHost::end_frame`] does that).
/// From another viewport, a gesture is live until its viewport finishes a pass without it.
pub fn is_live(ctx: &Context, id: GestureId) -> bool {
    match live::judge(ctx, id) {
        live::Judged::Live | live::Judged::Elsewhere { live: true } => true,
        live::Judged::Removed(_) | live::Judged::Elsewhere { live: false } => false,
    }
}

/// How active gesture `id` ends if it is interrupted, as its control recorded it in its last
/// frame; [`Ending::Cancel`] for a gesture no control recorded.
pub fn ending(ctx: &Context, id: GestureId) -> Ending {
    live::ending(ctx, id).unwrap_or(Ending::Cancel)
}

/// Every active gesture of `ctx` reports [`Cancel`](EditEvent::Cancel) with `reason` on its next
/// frame. The host calls it on a document switch, or on undo during a gesture (instead of
/// undoing a step). Gestures that begin after the call are not affected.
pub fn cancel_all(ctx: &Context, reason: CancelReason) {
    ctx.data_mut(|d| {
        let all = d.get_temp_mut_or_default::<CancelAll>(Id::unique(CANCEL_ALL));
        all.generation += 1;
        all.reason = Some(reason);
    });
}

/// The [`cancel_all`] calls so far: a gesture that began at an earlier generation is cancelled.
#[derive(Clone, Copy, Default)]
struct CancelAll {
    generation: u64,
    reason: Option<CancelReason>,
}

impl CancelAll {
    fn get(ctx: &Context) -> Self {
        ctx.data(|d| d.get_temp::<CancelAll>(Id::unique(CANCEL_ALL))).unwrap_or_default()
    }
}

const CANCEL_ALL: &str = "craft_ui::edit::cancel_all";

/// A dialog's gesture: it begins when the dialog opens, and the dialog commits it on OK and
/// cancels it on Cancel or Esc, through [`GestureHost::begin_outer`], [`GestureHost::commit`]
/// and [`GestureHost::cancel`]. The controls inside the dialog's [`scope`] report only updates.
///
/// The dialog calls [`keep`](OuterGesture::keep) every frame it is shown; a dialog that is no
/// longer shown is removed, and the host reverts its gesture.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[must_use]
pub struct OuterGesture {
    id: GestureId,
}

impl OuterGesture {
    /// A new gesture, live in the current pass.
    pub fn begin(ctx: &Context) -> Self {
        let id = GestureId::next(ctx);
        live::mark(ctx, id, Ending::Cancel);
        Self { id }
    }

    pub fn id(self) -> GestureId {
        self.id
    }

    /// Marks the gesture live in this pass; call it every frame the dialog is shown.
    pub fn keep(self, ui: &Ui) {
        live::mark(ui.ctx(), self.id, Ending::Cancel);
    }
}

/// The [`UiStackInfo`] tag [`scope`] sets.
const SCOPE_TAG: &str = "craft_ui::edit::scope";

/// Shows a dialog's controls: inside, every control reports only
/// [`Update`](EditEvent::Update)s (a commit or a one-shot change becomes an update with the
/// value), and Esc is left to the dialog, which owns one [`OuterGesture`]. Popups and menus
/// opened inside `add_contents` are inside too.
pub fn scope<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
    let _depth = ScopeDepth::enter(ui.ctx());
    ui.scope_builder(UiBuilder::new().ui_stack_info(UiStackInfo::default().with_tag(SCOPE_TAG)), add_contents)
}

/// Whether `ui` is inside a [`scope`]: in its `Ui`, or in any layer shown while it runs.
pub fn in_scope(ui: &Ui) -> bool {
    ScopeDepth::get(ui.ctx()) > 0 || ui.stack().iter().any(|node| node.tags().contains(SCOPE_TAG))
}

/// How many [`scope`] closures are running: popups and menus have a `Ui` stack of their own, so
/// the tag alone doesn't reach them.
struct ScopeDepth {
    ctx: Context,
}

impl ScopeDepth {
    fn enter(ctx: &Context) -> Self {
        ctx.data_mut(|d| *d.get_temp_mut_or_default::<usize>(Id::unique(SCOPE_DEPTH)) += 1);
        Self { ctx: ctx.clone() }
    }

    fn get(ctx: &Context) -> usize {
        ctx.data(|d| d.get_temp::<usize>(Id::unique(SCOPE_DEPTH))).unwrap_or_default()
    }
}

impl Drop for ScopeDepth {
    fn drop(&mut self) {
        self.ctx.data_mut(|d| {
            let depth = d.get_temp_mut_or_default::<usize>(Id::unique(SCOPE_DEPTH));
            *depth = depth.saturating_sub(1);
        });
    }
}

const SCOPE_DEPTH: &str = "craft_ui::edit::scope_depth";

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Edit lifecycle"];
