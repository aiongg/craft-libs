//! The edit lifecycle: how a value control reports a change to the application.
//!
//! A value control (slider, drag value, text field, colour picker) returns an [`EditResponse`]:
//! its [`Response`] and at most one [`EditEvent`] per frame. A continuous change is a gesture:
//! [`Begin`](EditEvent::Begin), [`Update`](EditEvent::Update)s, then
//! [`Commit`](EditEvent::Commit) or [`Cancel`](EditEvent::Cancel). A one-shot change (a toggle,
//! an arrow-key nudge, a wheel step, Enter) is a [`Set`](EditEvent::Set).
//!
//! The application owns the value and its "before" state. The widget keeps only its start value,
//! which it shows after a cancel until the application echoes the restored value.
//!
//! - Controls embed a [`Gesture`], the state machine that turns their input into events.
//! - [`EditPolicy`] says how often a control reports updates and whether Esc cancels.
//! - [`cancel_all`] cancels every active gesture, e.g. on a document switch or an undo.
//! - [`scope`] is for dialogs: controls inside it report only [`Update`](EditEvent::Update)s,
//!   and the dialog owns one outer gesture.
//! - [`GestureHost`] turns the events into undo steps for an application that keeps undo marks
//!   ([`UndoMarks`]).
//!
//! Named `EditEvent` rather than `Edit`, which is [`crate::ime::Edit`].

mod gesture;
mod host;
mod story;
#[cfg(test)]
mod tests;

use std::time::Duration;

use egui::{Context, Id, InnerResponse, Modifiers, Response, Ui, UiBuilder, UiStackInfo};

pub use gesture::Gesture;
pub use host::{GestureHost, UndoMarks};
pub(crate) use story::stories;

/// One gesture, unique within its [`Context`]: a new gesture never reuses an earlier id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
pub enum EditEvent<T> {
    /// A gesture started; the host takes its "before" state (e.g. an undo mark).
    Begin { id: GestureId },
    /// The gesture's value changed, at the policy's [`UpdateRate`]; the host previews it.
    Update { id: GestureId, value: T, modifiers: Modifiers },
    /// The gesture ended with `value`. `changed` is false when `value` equals the start value,
    /// so the host makes no undo step. With updates on, the last `Update` carried `value`.
    Commit { id: GestureId, value: T, changed: bool },
    /// The gesture was abandoned; the host restores its "before" state.
    Cancel { id: GestureId, reason: CancelReason },
    /// A one-shot change outside a gesture: a toggle, an arrow-key nudge, a wheel step, Enter.
    Set { value: T },
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
            EditEvent::Update { value, .. } | EditEvent::Commit { value, .. } | EditEvent::Set { value } => Some(value),
            EditEvent::Begin { .. } | EditEvent::Cancel { .. } => None,
        }
    }

    /// The same event with its value converted, e.g. from a control's value to a document field.
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> EditEvent<U> {
        match self {
            EditEvent::Begin { id } => EditEvent::Begin { id },
            EditEvent::Update { id, value, modifiers } => EditEvent::Update { id, value: f(value), modifiers },
            EditEvent::Commit { id, value, changed } => EditEvent::Commit { id, value: f(value), changed },
            EditEvent::Cancel { id, reason } => EditEvent::Cancel { id, reason },
            EditEvent::Set { value } => EditEvent::Set { value: f(value) },
        }
    }
}

/// Why a gesture was cancelled.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
}

/// How a control reports a gesture; the host chooses it per control.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

/// How often a gesture reports its value.
///
/// egui delivers input once per frame and a control reports at most one event per frame, so
/// `Realtime` and `PerFrame` produce the same events; they tell the host whether the value must
/// show in the frame it arrives (`Realtime`) or may show a frame later (`PerFrame`, e.g. a
/// preview rendered on a worker).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateRate {
    /// An update in every frame the value changes, applied by the host in that frame.
    Realtime,
    /// An update in every frame the value changes; the host may apply it a frame late.
    PerFrame,
    /// An update once the value has held still for the duration (input time,
    /// [`egui::InputState::time`]); changes within it coalesce into one update. The last value
    /// always goes out as an update before the commit.
    Debounced(Duration),
}

/// What a preview of the value costs the host: a hint for its preview quality.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Cost {
    /// A full-quality preview every update is affordable.
    #[default]
    Cheap,
    /// The host should preview at reduced quality during the gesture and render fully on commit.
    Expensive,
}

/// What every value control returns: its response and this frame's event, if any.
#[derive(Clone, Debug)]
#[must_use = "the host applies the edit event"]
pub struct EditResponse<T> {
    pub response: Response,
    pub event: Option<EditEvent<T>>,
}

impl<T> EditResponse<T> {
    /// The value of a finished change that the host keeps as an undo step: a
    /// [`Set`](EditEvent::Set), or a [`Commit`](EditEvent::Commit) that changed the value.
    pub fn committed(&self) -> Option<&T> {
        match &self.event {
            Some(EditEvent::Set { value } | EditEvent::Commit { value, changed: true, .. }) => Some(value),
            _ => None,
        }
    }

    /// The same response with the event's value converted ([`EditEvent::map`]).
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> EditResponse<U> {
        EditResponse { response: self.response, event: self.event.map(|e| e.map(f)) }
    }
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

/// The [`UiStackInfo`] tag [`scope`] sets.
const SCOPE_TAG: &str = "craft_ui::edit::scope";

/// Shows a dialog's controls: inside, every control reports only
/// [`Update`](EditEvent::Update)s (a commit or a one-shot change becomes an update with the
/// value), and Esc is left to the dialog. The dialog owns one outer gesture: it begins when the
/// dialog opens, commits on OK and cancels on Cancel or Esc.
pub fn scope<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
    ui.scope_builder(UiBuilder::new().ui_stack_info(UiStackInfo::default().with_tag(SCOPE_TAG)), add_contents)
}

/// Whether `ui` is inside a [`scope`].
pub fn in_scope(ui: &Ui) -> bool {
    ui.stack().iter().any(|node| node.tags().contains(SCOPE_TAG))
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Edit lifecycle"];
