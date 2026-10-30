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
//!   and ends it as [`ending`] says, or lets [`GestureHost::end_frame`] do that. A typing run
//!   ends as a commit (closing a panel ends editing and keeps the text), with the control's last
//!   value even when no update reported it ([`unreported`]: a debounce still waiting, or
//!   [`on_release_only`](EditPolicy::on_release_only)).
//! - A gesture the host ends before its control does ([`GestureHost`] ending it for another
//!   control's gesture or change) is over: [`is_live`] is false for it, and its control, on its
//!   next [`load`](Gesture::load), carries the input on in a new gesture from the host's value.
//!   That gesture reports its [`Begin`](EditEvent::Begin) at its first change, so a drag or a
//!   draft that goes on after an options-bar change reaches the host as a gesture of its own,
//!   and nothing when it ends unchanged.
//! - [`Gesture::commit_on_interrupt`] makes every interruption of a control's gesture a commit,
//!   for work kept as it goes (a pen draft's anchors).
//! - An input that starts an edit in a composite or on a canvas overlay while another control's
//!   typing run has the focus asks the run to end and runs after the run's ending event has gone
//!   out (`edit::order`); a composite reports one edit at a time (`edit::parts`).
//! - [`scope`] is for dialogs: controls inside it report only [`Update`](EditEvent::Update)s,
//!   and the dialog owns one [`OuterGesture`].
//! - [`GestureHost`] turns the events into undo steps for an application that keeps undo marks
//!   ([`UndoMarks`]).
//!
//! Named `EditEvent` rather than `Edit`, which is [`crate::ime::Edit`].

mod gesture;
mod host;
mod live;
pub(crate) mod ops;
pub(crate) mod order;
pub(crate) mod parts;
mod story;
#[cfg(test)]
mod tests;

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use egui::{Context, Id, InnerResponse, Modifiers, Response, Ui, UiBuilder, UiStackInfo};

pub use gesture::Gesture;
pub(crate) use gesture::escape_pressed;
pub use host::{GestureHost, UndoMarks};
pub(crate) use story::{PolicyArgs, describe, policy, stories};

/// One gesture, unique within the process: a new gesture never reuses an earlier id, and a
/// gesture whose [`Begin`](EditEvent::Begin) goes out later has a greater id, across contexts and
/// memory resets alike (a gesture that carries on an input the host ended takes its id when its
/// `Begin` goes out, [`Gesture::id`]). A [`GestureHost`] relies on that to ignore the late events
/// of a gesture it ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GestureId(u64);

impl GestureId {
    /// A new id, greater than every id made before it: for a gesture that an application's own
    /// control reports to a [`GestureHost`], and for tests that feed a host events.
    pub fn next() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        GestureId(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

/// `#3`, for logs.
impl std::fmt::Display for GestureId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

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

/// Constructors, for an application's own control that reports to a [`GestureHost`] and for
/// tests that feed one events; the variants are `#[non_exhaustive]`.
///
/// ```
/// use craft_ui::edit::{EditEvent, GestureHost, GestureId, UndoMarks};
/// use egui::Modifiers;
///
/// #[derive(Default)]
/// struct Doc { value: i32, undo: Vec<i32> }
/// impl UndoMarks for Doc {
///     type Mark = usize;
///     fn undo_mark(&self) -> usize { self.undo.len() }
///     fn revert_to(&mut self, mark: usize) { if let Some(&v) = self.undo.get(mark) { self.value = v; } self.undo.truncate(mark); }
///     fn squash_since(&mut self, mark: usize) { self.undo.truncate(mark + 1); }
/// }
///
/// let ctx = egui::Context::default();
/// let (mut doc, mut host) = (Doc::default(), GestureHost::new());
/// let apply = |doc: &mut Doc, v| { doc.undo.push(doc.value); doc.value = v; };
/// let id = GestureId::next();
/// for event in [EditEvent::begin(id), EditEvent::update(id, 1, Modifiers::NONE), EditEvent::update(id, 2, Modifiers::NONE), EditEvent::commit(id, 2, true, Modifiers::NONE)] {
///     host.handle(&ctx, &mut doc, event, apply);
/// }
/// assert_eq!((doc.value, doc.undo.len()), (2, 1));
/// host.handle(&ctx, &mut doc, EditEvent::set(5, Modifiers::SHIFT), apply);
/// assert_eq!((doc.value, doc.undo.len()), (5, 2));
/// ```
impl<T> EditEvent<T> {
    /// [`Begin`](EditEvent::Begin).
    pub fn begin(id: GestureId) -> Self {
        EditEvent::Begin { id }
    }

    /// [`Update`](EditEvent::Update).
    pub fn update(id: GestureId, value: T, modifiers: Modifiers) -> Self {
        EditEvent::Update { id, value, modifiers }
    }

    /// [`Commit`](EditEvent::Commit).
    pub fn commit(id: GestureId, value: T, changed: bool, modifiers: Modifiers) -> Self {
        EditEvent::Commit { id, value, changed, modifiers }
    }

    /// [`Cancel`](EditEvent::Cancel).
    pub fn cancel(id: GestureId, reason: CancelReason) -> Self {
        EditEvent::Cancel { id, reason }
    }

    /// [`Set`](EditEvent::Set).
    pub fn set(value: T, modifiers: Modifiers) -> Self {
        EditEvent::Set { value, modifiers }
    }
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
    /// The control's popup closed without a choice: a click outside it, a press on the control
    /// while it was open, or a screen reader's click on the open control.
    Dismissed,
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
/// [cancels on focus loss](Gesture::cancel_on_focus_loss); a drag cancels. A control that
/// [commits on interrupt](Gesture::commit_on_interrupt) always commits. A commit ends with the
/// control's last value: when no update reported it (a debounce still waiting,
/// [`on_release_only`](EditPolicy::on_release_only)), the host applies it first ([`unreported`]),
/// so the run is still one undo step with everything typed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Ending {
    /// The host keeps the control's last value as one undo step; `changed` is false when that
    /// value is the start value, so the host reverts instead.
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
    /// with the final value. A typing run interrupted before its commit (its control no longer
    /// drawn, another gesture begun) still ends with the typed value ([`Ending`]).
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
    /// one update, and a value still waiting at the end goes out with the commit, or with the
    /// [`Ending`] of an interrupted typing run. Timed by egui's
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

/// What every value control returns: its response, this frame's event, if any, and whether the
/// user submitted the value.
#[derive(Clone, Debug)]
#[non_exhaustive]
#[must_use = "the host applies the edit event"]
pub struct EditResponse<T> {
    pub response: Response,
    pub event: Option<EditEvent<T>>,
    /// The user pressed the control's submit key this frame (Enter in a text field), whether or
    /// not the value changed: a search's "find next", a dialog's OK. A change the key ends goes
    /// out as this frame's [`Commit`](EditEvent::Commit) (or [`Set`](EditEvent::Set)). Only a
    /// plain press counts: no modifier but Shift, and no repeat of a held key, so a dialog's OK
    /// fires once and Cmd/Ctrl+Enter stays free (a multi-line field's commit). Controls without
    /// a submit key never set it.
    pub submitted: bool,
    /// The user pressed Esc in the control this frame and the control passed the press on: it
    /// ended the control's own part (a text field cancels its typing run and leaves the field)
    /// and left the press in egui's input, so the controls drawn after it see it too. The host
    /// acts on the same press here: a dialog's Cancel, a tool dropping its operation. A press the
    /// control keeps (a search field clearing its text, an open list closing, an input method's
    /// composition) doesn't set it.
    pub escaped: bool,
}

impl<T> EditResponse<T> {
    pub fn new(response: Response, event: Option<EditEvent<T>>) -> Self {
        Self { response, event, submitted: false, escaped: false }
    }

    /// Whether the user submitted the value this frame.
    pub fn submitted(mut self, submitted: bool) -> Self {
        self.submitted = submitted;
        self
    }

    /// Whether the control passed an Esc press on to the host this frame.
    pub fn escaped(mut self, escaped: bool) -> Self {
        self.escaped = escaped;
        self
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
        EditResponse { response: self.response, event: self.event.map(|e| e.map(f)), submitted: self.submitted, escaped: self.escaped }
    }
}

/// Whether gesture `id`'s control was drawn in the current pass of its viewport. Call it in that
/// viewport, after the controls: a gesture that is not live is over, its control starts idle if
/// it comes back, and the host ends it as [`ending`] says ([`GestureHost::end_frame`] does that).
/// From another viewport, a gesture is live until its viewport finishes a pass without it. A
/// gesture the [`GestureHost`] ended before its control did is not live either.
pub fn is_live(ctx: &Context, id: GestureId) -> bool {
    match live::judge(ctx, id) {
        live::Judged::Live | live::Judged::Elsewhere { live: true } => true,
        live::Judged::Removed(_) | live::Judged::Elsewhere { live: false } => false,
    }
}

/// [`is_live`] before the current pass has drawn its controls: whether gesture `id`'s control was
/// drawn in the previous pass of its viewport or since. A composite asks it for a gesture it
/// replays before it draws its parts.
pub fn was_live(ctx: &Context, id: GestureId) -> bool {
    live::was_drawn(ctx, id)
}

/// How active gesture `id` ends if it is interrupted, as its control recorded it in its last
/// frame; [`Ending::Cancel`] for a gesture no control recorded.
pub fn ending(ctx: &Context, id: GestureId) -> Ending {
    live::ending(ctx, id).unwrap_or(Ending::Cancel)
}

/// The value an interrupted gesture `id` commits with that no update reported to the host: a
/// debounced value still waiting, or every change under
/// [`on_release_only`](EditPolicy::on_release_only) or of a gesture that
/// [holds its updates](Gesture::hold_updates). The host applies it before keeping the
/// gesture as one undo step. `None` when the host has seen the last value, when the gesture ends
/// as a cancel, or when `T` isn't the type the control reports its value in
/// ([`Gesture::finish_as`]).
pub fn unreported<T: Clone + 'static>(ctx: &Context, id: GestureId) -> Option<T> {
    live::unreported(ctx, id).and_then(|value| value.downcast_ref::<T>().cloned())
}

/// The active gesture whose control had the keyboard focus in its last pass of the current
/// viewport: a typing run, or a drag that took the focus with its press.
pub(crate) fn focused_gesture(ctx: &Context) -> Option<GestureId> {
    live::focused(ctx)
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
        let id = GestureId::next();
        live::mark(ctx, id, Ending::Cancel, None, false, None);
        Self { id }
    }

    pub fn id(self) -> GestureId {
        self.id
    }

    /// Marks the gesture live in this pass; call it every frame the dialog is shown.
    pub fn keep(self, ui: &Ui) {
        live::mark(ui.ctx(), self.id, Ending::Cancel, None, false, None);
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
