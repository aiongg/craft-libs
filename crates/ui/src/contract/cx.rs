//! The context types (1.5): `CxCore`, `Cx`, `BodyCx`, `ItemCx`, `MeasureCx`, `PartsCx` and `PartBuilder`, and the
//! chapter traits implemented on them.

use super::{Component, EditResponse, Resolved, Show, Tooltip, Validity, ValueOf};
use crate::primitives::sensing::{PartDecl, PartResults};
use crate::primitives::{Adopted, PaintRecord, PartResult, TextEnv, TextLayout, TextRun};
use crate::runtime::commands::ResolvedCommand;
use crate::runtime::env::{Env, EnvView, Label};
use crate::runtime::focus::{self, FocusShape};
use crate::runtime::keys::Intents;
use crate::runtime::memory::{MemoryStage, StageAccess, sealed};
use crate::runtime::motion::{Driver, Drives};
use crate::runtime::{cursor, frame, pointer, wheel};
use crate::style::{ComponentStyle as _, Ink, Mix, StateSet, Stateful, UiTokens};
use craft_behavior::{
    CancelReason, Cursor, CursorIcon, CursorPriority, EditEvent, Flow, Gesture, GesturePhase, HitLevel, HitPriority, HitShape, Mixed, PartRef,
    PressCancel, PressPhase, PressState, Proposal, Roving, Sense, Value, Wheel,
};
use craft_keymap::Chord;
use craft_theme::Flags;
use egui::{Context, Id, Rect, Response, Ui, Vec2};
use std::cell::RefCell;
use std::fmt::Debug;
use std::hash::Hash;
use std::ops::Deref;
use std::sync::Arc;

/// How a component was activated this pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Activation {
    /// By `Intent::Activate` (Enter, Space or an AccessKit click) rather than a primary click.
    pub by_key: bool,
}

/// The non-generic part of every `Cx`; the step functions take this. It is a read-only output: component code
/// changes nothing here except through a chapter trait.
#[non_exhaustive]
pub struct CxCore<'a> {
    /// What the inherited context holds that component code may read.
    pub env: EnvView<'a>,
    /// `Common` after step 1.
    pub resolved: &'a Resolved<'a>,
    /// The component's id.
    pub id: Id,
    /// The allocated rect.
    pub rect: Rect,
    /// The rect minus padding, mirrored for right-to-left.
    pub content_rect: Rect,
    /// The rect intersected with the clip rect.
    pub visible: Rect,
    /// Start and end geometry of the rect.
    pub flow: Flow,
    /// The root widget's response.
    pub response: Response,
    /// The bound command, resolved at step 1.
    pub command: Option<Arc<ResolvedCommand>>,
    /// Written by step 12; read with `Styled::state`.
    pub(crate) state: StateSet,
    /// True once step 12 wrote `state` in this pass.
    pub(crate) stated: bool,
    /// Read with `Styled::ink`, valid from step 12.
    pub(crate) ink: Ink,
    /// An owned copy of the intents routed to this id.
    pub(crate) intents: Intents,
    pub(crate) memory: &'a mut MemoryStage,
    pub(crate) ctx: Context,
    pub(crate) paint: RefCell<PaintRecord>,
    pub(crate) results: PartResults,
    /// A face reads idle results for parts it has not registered.
    pub(crate) face_mode: bool,
    /// The active part while the component holds focus.
    pub(crate) active: Option<PartRef>,
    /// The arena root this component is a member of; `None` for a component with its own egui widget.
    pub(crate) member_of: Option<Id>,
    pub(crate) activation: Option<Activation>,
    /// For an arena root: the member activated this pass.
    pub(crate) activated_member: Option<Id>,
    pub(crate) press: PressState,
    pub(crate) cursor: Option<(Cursor, CursorPriority)>,
    /// The declared cursors of the parts hovered this pass, recorded at step 8 and written at step 16.
    pub(crate) part_cursors: Vec<CursorIcon>,
    pub(crate) claims: Vec<Chord>,
    pub(crate) reveal: Option<Rect>,
    pub(crate) press_cancel: Option<PressCancel>,
}

impl sealed::Sealed for CxCore<'_> {}

impl StageAccess for CxCore<'_> {
    fn stage(&self) -> &MemoryStage {
        self.memory
    }

    fn stage_mut(&mut self) -> &mut MemoryStage {
        self.memory
    }

    fn base(&self) -> Id {
        self.id
    }
}

/// What component code reads and calls. It derefs to [`CxCore`] for reading only.
///
/// ```compile_fail,E0594
/// use craft_ui::contract::{Component, Cx};
/// fn update<C: Component>(cx: &mut Cx<'_, C>) {
///     cx.rect = egui::Rect::NOTHING;
/// }
/// ```
#[non_exhaustive]
pub struct Cx<'a, C: Component> {
    pub(crate) core: CxCore<'a>,
    /// The component's cached style.
    pub style: &'a C::Style,
    /// The shown copy of the value, shared with shown memory.
    pub(crate) shown: Option<Arc<ValueOf<C>>>,
    /// The binding holds a mixed value; `shown` is empty.
    pub(crate) mixed: bool,
    /// The edit events of this pass, in order; `applied` of them have reached the binding.
    pub(crate) events: Vec<EditEvent<ValueOf<C>>>,
    pub(crate) applied: usize,
    /// What the binding's check says about the shown value.
    pub(crate) validity: Validity<ValueOf<C>>,
}

impl<'a, C: Component> Deref for Cx<'a, C> {
    type Target = CxCore<'a>;

    fn deref(&self) -> &CxCore<'a> {
        &self.core
    }
}

impl<C: Component> sealed::Sealed for Cx<'_, C> {}

impl<C: Component> StageAccess for Cx<'_, C> {
    fn stage(&self) -> &MemoryStage {
        self.core.memory
    }

    fn stage_mut(&mut self) -> &mut MemoryStage {
        self.core.memory
    }

    fn base(&self) -> Id {
        self.core.id
    }
}

/// Pointer state and requests (chapter 8).
pub trait Pointer {
    /// The result of `part` this pass: idle for a part first registered this frame. A part `parts` did not
    /// declare is a debug panic.
    fn part(&self, part: impl Into<PartRef>) -> &PartResult;
    /// The component's own press.
    fn press(&self) -> &PressState;
    /// The pointer is over the component's own shapes.
    fn hit(&self) -> bool;
    /// Set only when the component's own shapes won the press and it was released as a click, or
    /// `Intent::Activate` was routed to it.
    fn activation(&self) -> Option<Activation>;
    /// For an arena root, the member activated this pass.
    fn activated_member(&self) -> Option<Id>;
    /// This pass's wheel share, once.
    fn take_wheel(&mut self) -> Option<Wheel>;
    /// Requests `cursor` at `priority`; the highest priority of the pass wins.
    fn cursor(&mut self, cursor: impl Into<Cursor>, priority: CursorPriority);
    /// Ends every captured press of this viewport.
    fn cancel_press(&mut self, reason: PressCancel);
}

impl Pointer for CxCore<'_> {
    fn part(&self, part: impl Into<PartRef>) -> &PartResult {
        let part = part.into();
        match self.results.get(&(part.key, part.index)) {
            Some(result) => result,
            None => {
                debug_assert!(self.face_mode || self.results.is_empty() && self.stated, "`parts` did not declare the part {part:?}");
                &PartResult::IDLE
            }
        }
    }

    fn press(&self) -> &PressState {
        &self.press
    }

    fn hit(&self) -> bool {
        self.response.hovered()
    }

    fn activation(&self) -> Option<Activation> {
        self.activation
    }

    fn activated_member(&self) -> Option<Id> {
        self.activated_member
    }

    fn take_wheel(&mut self) -> Option<Wheel> {
        wheel::take(&self.ctx, self.id)
    }

    fn cursor(&mut self, cursor: impl Into<Cursor>, priority: CursorPriority) {
        if self.cursor.as_ref().is_none_or(|(_, held)| priority > *held) {
            self.cursor = Some((cursor.into(), priority));
        }
    }

    fn cancel_press(&mut self, reason: PressCancel) {
        pointer::cancel_press(&self.ctx, reason);
        frame::note_cancelled_press(&self.ctx);
        self.press_cancel = Some(reason);
    }
}

impl<C: Component> Pointer for Cx<'_, C> {
    fn part(&self, part: impl Into<PartRef>) -> &PartResult {
        self.core.part(part)
    }

    fn press(&self) -> &PressState {
        self.core.press()
    }

    fn hit(&self) -> bool {
        self.core.hit()
    }

    fn activation(&self) -> Option<Activation> {
        self.core.activation()
    }

    fn activated_member(&self) -> Option<Id> {
        self.core.activated_member()
    }

    fn take_wheel(&mut self) -> Option<Wheel> {
        self.core.take_wheel()
    }

    fn cursor(&mut self, cursor: impl Into<Cursor>, priority: CursorPriority) {
        self.core.cursor(cursor, priority);
    }

    fn cancel_press(&mut self, reason: PressCancel) {
        self.core.cancel_press(reason);
    }
}

/// The resolved look (chapter 4); every method reads step 12's output.
pub trait Styled {
    /// The state set.
    fn state(&self) -> &StateSet;
    /// The ink the surface reads in.
    fn ink(&self) -> Ink;
    /// The value of `look` for the component's state set.
    fn resolve<T: Mix>(&self, look: &Stateful<T>) -> T;
    /// The value of `look` for the state set of `part`.
    fn resolve_part<T: Mix>(&self, part: impl Into<PartRef>, look: &Stateful<T>) -> T;
}

impl CxCore<'_> {
    fn assert_stated(&self) {
        debug_assert!(self.stated, "the state set is written at step 12; it is not readable in `update` or `overlay`");
    }

    /// The state set of `part`: its press and hover, its own flags, and the focus flags only when it is the
    /// active part of the focused component.
    pub(crate) fn part_state(&self, part: PartRef) -> StateSet {
        let result = self.part(part);
        let mut flags = result.flags;
        if self.active == Some(part) && focus::is_focused(&self.ctx, self.id) {
            flags |= Flags::FOCUSED;
            if focus::focus_visible(&self.ctx, self.id) {
                flags |= Flags::FOCUS_VISIBLE;
            }
        }
        let pressed = result.press.phase.is_press();
        let state = if !self.resolved.enabled {
            craft_theme::State::Disabled
        } else if pressed {
            craft_theme::State::Pressed
        } else if result.hovered {
            craft_theme::State::Hover
        } else {
            craft_theme::State::Rest
        };
        let on = flags.intersects(Flags::ON);
        let t = craft_theme::StateMix::new(f32::from(u8::from(result.hovered)), f32::from(u8::from(pressed)), f32::from(u8::from(on)));
        StateSet::new(state, flags, t)
    }
}

impl Styled for CxCore<'_> {
    fn state(&self) -> &StateSet {
        self.assert_stated();
        &self.state
    }

    fn ink(&self) -> Ink {
        self.assert_stated();
        self.ink
    }

    fn resolve<T: Mix>(&self, look: &Stateful<T>) -> T {
        self.assert_stated();
        self.state.resolve(look)
    }

    fn resolve_part<T: Mix>(&self, part: impl Into<PartRef>, look: &Stateful<T>) -> T {
        self.assert_stated();
        self.part_state(part.into()).resolve(look)
    }
}

impl<C: Component> Styled for Cx<'_, C> {
    fn state(&self) -> &StateSet {
        self.core.state()
    }

    fn ink(&self) -> Ink {
        self.core.ink()
    }

    fn resolve<T: Mix>(&self, look: &Stateful<T>) -> T {
        self.core.resolve(look)
    }

    fn resolve_part<T: Mix>(&self, part: impl Into<PartRef>, look: &Stateful<T>) -> T {
        self.core.resolve_part(part, look)
    }
}

/// Focus and keys (chapter 9).
pub trait Focus {
    /// The intents routed to this component this pass: `cx.intents().take(Intent::Activate)`.
    fn intents(&mut self) -> &mut Intents;
    /// Claims `chord` for the next frames while this component is focused.
    fn claim(&mut self, chord: Chord);
    /// Whether `chord` arrived this pass for this component, consuming it.
    fn take_claimed(&mut self, chord: Chord) -> bool;
    /// The focused part while the component holds focus.
    fn active_part(&self) -> Option<PartRef>;
    /// The component holds key focus.
    fn is_focused(&self) -> bool;
    /// The component holds focus and shows it.
    fn focus_visible(&self) -> bool;
    /// Asks the nearest scroll owner to bring `rect` into view.
    fn reveal(&mut self, rect: Rect);
}

impl Focus for CxCore<'_> {
    fn intents(&mut self) -> &mut Intents {
        &mut self.intents
    }

    fn claim(&mut self, chord: Chord) {
        self.claims.push(chord);
    }

    fn take_claimed(&mut self, chord: Chord) -> bool {
        crate::runtime::keys::take_claimed(&self.ctx, self.id, chord)
    }

    fn active_part(&self) -> Option<PartRef> {
        self.active
    }

    fn is_focused(&self) -> bool {
        focus::is_focused(&self.ctx, self.id)
    }

    fn focus_visible(&self) -> bool {
        focus::focus_visible(&self.ctx, self.id)
    }

    fn reveal(&mut self, rect: Rect) {
        self.reveal = Some(rect);
    }
}

impl<C: Component> Focus for Cx<'_, C> {
    fn intents(&mut self) -> &mut Intents {
        self.core.intents()
    }

    fn claim(&mut self, chord: Chord) {
        self.core.claim(chord);
    }

    fn take_claimed(&mut self, chord: Chord) -> bool {
        self.core.take_claimed(chord)
    }

    fn active_part(&self) -> Option<PartRef> {
        self.core.active_part()
    }

    fn is_focused(&self) -> bool {
        self.core.is_focused()
    }

    fn focus_visible(&self) -> bool {
        self.core.focus_visible()
    }

    fn reveal(&mut self, rect: Rect) {
        self.core.reveal(rect);
    }
}

impl CxCore<'_> {
    /// The resolved label as the name an edit event carries.
    pub(crate) fn label_arc(&self) -> Arc<str> {
        Arc::from(self.resolved.label)
    }

    /// A press on the component or one of its parts is down: the press that carries a gesture.
    pub(crate) fn press_held(&self) -> bool {
        let down = |phase: &PressPhase| matches!(phase, PressPhase::Down { .. } | PressPhase::DragStart { .. } | PressPhase::Drag { .. });
        down(&self.press.phase) || self.results.values().any(|result| down(&result.press.phase))
    }

    /// The reason a press of the component or one of its parts was cancelled in this pass.
    pub(crate) fn press_cancelled(&self) -> Option<PressCancel> {
        let reason = |phase: &PressPhase| match phase {
            PressPhase::Cancel { reason, .. } => Some(*reason),
            _ => None,
        };
        reason(&self.press.phase).or_else(|| self.results.values().find_map(|result| reason(&result.press.phase)))
    }
}

/// Values and edits (chapter 10). `value`, `validity`, `gesture`, `adopt` and `edit_response` take `&self`, so `paint`
/// can call them.
pub trait Edit {
    /// The component's value type.
    type Value: Value;

    /// The shown value: the running gesture's proposal, else the binding's. `None` for a mixed binding, a kept
    /// binding with nothing in it, and a component with no value.
    fn value(&self) -> Option<&Self::Value>;

    /// What the binding's check says about the shown value.
    fn validity(&self) -> Validity<Self::Value>;

    /// Where this pass's gesture stands.
    fn gesture(&self) -> Option<GesturePhase>;

    /// Maps the child's edit onto a clone of this component's shown value and queues the resulting event, following the
    /// child's, for step 15.
    fn adopt<V: Value>(&self, child: &EditResponse<V>, map: impl FnOnce(&V, &mut Self::Value));

    /// The `EditResponse` a value component returns from `finish`.
    fn edit_response(&self) -> EditResponse<Self::Value>;

    /// Sets the running gesture's value, beginning a gesture if none runs and the press that carries it has had no
    /// gesture cancelled.
    fn propose(&mut self, v: Self::Value);

    /// A one-shot change outside a gesture.
    fn set(&mut self, v: Self::Value);

    /// Opens a gesture.
    fn begin(&mut self);

    /// Commits the proposed value.
    fn commit(&mut self);

    /// Cancels the running gesture.
    fn cancel(&mut self, reason: CancelReason);
}

/// The key of the gesture slot under a component's base id.
pub(crate) fn gesture_key(id: Id) -> Id {
    crate::runtime::memory::sub(id, "edit")
}

impl<C: Component> Cx<'_, C> {
    fn gesture_slot(&mut self) -> &mut Gesture<ValueOf<C>> {
        let id = gesture_key(self.core.id);
        self.core.memory.shown(id)
    }

    /// The value before an edit: the shown value, or mixed.
    fn before(&self) -> Mixed<ValueOf<C>> {
        match (&self.shown, self.mixed) {
            (Some(value), false) => Mixed::Same((**value).clone()),
            _ => Mixed::Mixed,
        }
    }

    /// An edit of a read-only component reaches nothing.
    fn refuses_edits(&self) -> bool {
        self.core.resolved.read_only
    }

    /// Records the gesture an adopted event belongs to, as `begin`, `propose`, `commit` and `cancel` do for the
    /// component's own.
    pub(crate) fn absorb(&mut self, event: &EditEvent<ValueOf<C>>) {
        let dead = self.core.press_held();
        let slot = self.gesture_slot();
        match event {
            EditEvent::Begin { id, before, .. } => {
                *slot = slot.ended(false);
                slot.id = *id;
                slot.before = Some(before.clone());
            }
            EditEvent::Update { value, .. } => slot.proposal = Some(value.clone()),
            EditEvent::Commit { .. } | EditEvent::Set { .. } => *slot = slot.ended(false),
            EditEvent::Cancel { .. } => *slot = slot.ended(dead),
            _ => {}
        }
    }
}

impl<C: Component> Edit for Cx<'_, C> {
    type Value = ValueOf<C>;

    fn value(&self) -> Option<&ValueOf<C>> {
        self.shown.as_deref()
    }

    fn validity(&self) -> Validity<ValueOf<C>> {
        self.validity.clone()
    }

    fn gesture(&self) -> Option<GesturePhase> {
        if let Some(phase @ (GesturePhase::Commit | GesturePhase::Cancel)) = self.events.last().and_then(EditEvent::phase) {
            return Some(phase);
        }
        if !self.core.memory.recall::<Gesture<ValueOf<C>>>(gesture_key(self.core.id)).is_running() {
            return None;
        }
        let began = self.events.iter().any(|event| matches!(event, EditEvent::Begin { .. }));
        let updated = self.events.iter().any(|event| matches!(event, EditEvent::Update { .. }));
        Some(if began && !updated { GesturePhase::Begin } else { GesturePhase::Update })
    }

    fn adopt<V: Value>(&self, child: &EditResponse<V>, map: impl FnOnce(&V, &mut ValueOf<C>)) {
        let (Some(event), Some(mut mine)) = (child.event.as_ref(), self.shown.as_deref().cloned()) else { return };
        let before = Mixed::Same(mine.clone());
        if let Some(value) = event.value() {
            map(value, &mut mine);
        }
        let adopted = match event {
            EditEvent::Begin { id, label, doc_key, .. } => EditEvent::begin(*id, before, label.clone(), *doc_key),
            EditEvent::Update { id, .. } => EditEvent::update(*id, mine),
            EditEvent::Commit { id, .. } => EditEvent::commit(*id, before, mine),
            EditEvent::Cancel { id, reason, .. } => EditEvent::cancel(*id, *reason, Some(before)),
            EditEvent::Set { id, label, doc_key, .. } => EditEvent::set(*id, before, mine, label.clone(), *doc_key),
            _ => return,
        };
        self.core.paint.borrow_mut().adopted.push(Adopted::new(adopted));
    }

    fn edit_response(&self) -> EditResponse<ValueOf<C>> {
        let event = self.events.iter().cloned().fold(None, EditEvent::fold);
        EditResponse::new(self.core.response.clone(), event, self.validity.clone())
    }

    fn propose(&mut self, v: ValueOf<C>) {
        if self.refuses_edits() {
            return;
        }
        let before = self.before();
        let (label, doc_key) = (self.core.label_arc(), self.core.resolved.doc_key);
        let slot = self.gesture_slot();
        // A gesture cancelled under its press stays cancelled until the release.
        if slot.dead {
            return;
        }
        let started = !slot.is_running();
        if started {
            slot.id = slot.id.next();
            slot.before = Some(before.clone());
        }
        slot.proposal = Some(v.clone());
        let id = slot.id;
        if started {
            self.events.push(EditEvent::begin(id, before, label, doc_key));
        }
        self.events.push(EditEvent::update(id, v.clone()));
        self.shown = Some(Arc::new(v));
        self.mixed = false;
    }

    fn set(&mut self, v: ValueOf<C>) {
        if self.refuses_edits() {
            return;
        }
        let before = self.before();
        let (label, doc_key) = (self.core.label_arc(), self.core.resolved.doc_key);
        let id = self.gesture_slot().id;
        self.events.push(EditEvent::set(id, before, v.clone(), label, doc_key));
        self.shown = Some(Arc::new(v));
        self.mixed = false;
    }

    fn begin(&mut self) {
        if self.refuses_edits() {
            return;
        }
        let before = self.before();
        let (label, doc_key) = (self.core.label_arc(), self.core.resolved.doc_key);
        let slot = self.gesture_slot();
        if !slot.is_running() && !slot.dead {
            slot.id = slot.id.next();
            slot.before = Some(before.clone());
            let id = slot.id;
            self.events.push(EditEvent::begin(id, before, label, doc_key));
        }
    }

    fn commit(&mut self) {
        let slot = self.gesture_slot();
        if !slot.is_running() {
            return;
        }
        let ended = slot.ended(false);
        let running = std::mem::replace(slot, ended);
        let (id, Some(before)) = (running.id, running.before) else { return };
        match (running.proposal, before) {
            (Some(after), before) => self.events.push(EditEvent::commit(id, before, after)),
            (None, Mixed::Same(held)) => self.events.push(EditEvent::commit(id, Mixed::Same(held.clone()), held)),
            (None, Mixed::Mixed) => self.events.push(EditEvent::cancel(id, CancelReason::Dismissed, Some(Mixed::Mixed))),
        }
    }

    fn cancel(&mut self, reason: CancelReason) {
        let dead = self.core.press_held();
        let slot = self.gesture_slot();
        if !slot.is_running() {
            return;
        }
        let ended = slot.ended(dead);
        let running = std::mem::replace(slot, ended);
        let restored = running.before.clone().filter(|_| !reason.document_moved());
        self.events.push(EditEvent::cancel(running.id, reason, restored.clone()));
        if let Some(before) = restored {
            self.shown = match before {
                Mixed::Same(value) => Some(Arc::new(value)),
                Mixed::Mixed => None,
            };
            self.mixed = self.shown.is_none();
        }
    }
}

/// The label a component shows (chapter 19).
// F3-LATER: words (F4 adds `words::<L>()`)
pub trait WordSetCx {
    /// The label as given, a cheap clone.
    fn label(&self) -> Label;
}

impl WordSetCx for CxCore<'_> {
    fn label(&self) -> Label {
        self.resolved.label_value.clone()
    }
}

impl<C: Component> WordSetCx for Cx<'_, C> {
    fn label(&self) -> Label {
        self.core.label()
    }
}

/// What a body closure and the header, footer and empty hooks read: a view over the pipeline's memory stage
/// keyed by the body's base id.
#[non_exhaustive]
pub struct BodyCx<'a> {
    /// The body's base id: `sub(id, "body")`.
    pub id: Id,
    /// What the inherited context holds that component code may read.
    pub env: EnvView<'a>,
    /// The container's `Common` after step 1.
    pub resolved: &'a Resolved<'a>,
    /// The body's rect.
    pub rect: Rect,
    pub(crate) memory: &'a mut MemoryStage,
}

impl<'a> BodyCx<'a> {
    pub(crate) fn new(id: Id, env: EnvView<'a>, resolved: &'a Resolved<'a>, rect: Rect, memory: &'a mut MemoryStage) -> Self {
        memory.claim(id);
        BodyCx { id, env, resolved, rect, memory }
    }
}

impl sealed::Sealed for BodyCx<'_> {}

impl StageAccess for BodyCx<'_> {
    fn stage(&self) -> &MemoryStage {
        self.memory
    }

    fn stage_mut(&mut self) -> &mut MemoryStage {
        self.memory
    }

    fn base(&self) -> Id {
        self.id
    }
}

impl WordSetCx for BodyCx<'_> {
    fn label(&self) -> Label {
        self.resolved.label_value.clone()
    }
}

/// Rich tooltip content an item stored.
pub(crate) type ItemTip<'r> = Box<dyn FnOnce(&mut Ui) + 'r>;

/// What the item closure reads for one item: a view over the pipeline's memory stage keyed by the item.
#[non_exhaustive]
pub struct ItemCx<'a, 'r, K> {
    /// The item's key.
    pub key: &'a K,
    /// The item's position in the source.
    pub at: usize,
    /// The item's rect.
    pub rect: Rect,
    /// What the inherited context holds that component code may read.
    pub env: EnvView<'a>,
    /// Base id of the item's stage: `sub(item_id, "item")`.
    pub(crate) id: Id,
    pub(crate) memory: &'a mut MemoryStage,
    pub(crate) tip: Option<ItemTip<'r>>,
}

impl<'r, K> ItemCx<'_, 'r, K> {
    /// Rich tooltip content for the item; it may borrow app data.
    pub fn tip(&mut self, tip: impl FnOnce(&mut Ui) + 'r) {
        self.tip = Some(Box::new(tip));
    }
}

impl<K> sealed::Sealed for ItemCx<'_, '_, K> {}

impl<K> StageAccess for ItemCx<'_, '_, K> {
    fn stage(&self) -> &MemoryStage {
        self.memory
    }

    fn stage_mut(&mut self) -> &mut MemoryStage {
        self.memory
    }

    fn base(&self) -> Id {
        self.id
    }
}

/// What `measure` reads: step 5's view of the environment, the style, the resolved props and the shown value.
#[non_exhaustive]
pub struct MeasureCx<'a, C: Component> {
    /// What the inherited context holds that component code may read.
    pub env: EnvView<'a>,
    /// The component's cached style.
    pub style: &'a C::Style,
    /// `Common` after step 1.
    pub resolved: &'a Resolved<'a>,
    /// The size the parent proposes.
    pub proposal: Proposal,
    /// The width the parent offers.
    pub room: Option<f32>,
    pub(crate) value: Option<&'a ValueOf<C>>,
    pub(crate) ui: &'a Ui,
    pub(crate) tokens: &'a UiTokens,
    pub(crate) ink: Ink,
    pub(crate) id: Id,
    pub(crate) enabled_size: (bool, craft_theme::ControlSize),
}

impl<'a, C: Component> MeasureCx<'a, C> {
    /// The shown value; it has the stage's lifetime, so a borrow of `cx` is not held.
    pub fn value(&self) -> Option<&'a ValueOf<C>> {
        self.value
    }
}

impl<C: Component> MeasureCx<'_, C> {
    /// The label as given.
    pub fn label(&self) -> Label {
        self.resolved.label_value.clone()
    }

    /// Lays `run` out in the room the parent offers, as `Draw::text` will paint it.
    pub fn layout(&self, run: &TextRun<'_>) -> TextLayout {
        let env = TextEnv { tokens: self.tokens, direction: self.env.direction(), ink: crate::style::ink_color(self.tokens, self.ink) };
        run.layout(self.ui.ctx(), &env, self.room.unwrap_or(f32::INFINITY))
    }

    /// Measures a child under the composite's resolved `enabled` and `size`, as `PaintUi::child` will show it.
    pub fn child<D: Show>(&self, key: impl Hash + Debug, child: &mut D, proposal: Proposal) -> Vec2 {
        let id = self.id.with(key);
        super::measure_under(child, self.ui, Some(id), proposal, Some(self.enabled_size))
    }

    /// The icon side of the component's size.
    pub fn icon(&self) -> f32 {
        crate::style::control_metrics(self.tokens, self.resolved.size).icon
    }

    /// The gap between the component's items.
    pub fn gap(&self) -> f32 {
        self.style.metrics().gap
    }
}

/// A part as `PartsCx::part` declares it; the builder sets its flags.
pub struct PartBuilder<'a> {
    decl: &'a mut PartDecl,
    declared: &'a mut PartExtras,
}

/// The parts a component declared, each with what it declares beyond the sensing core's `PartDecl`.
pub(crate) type Decls = Vec<(PartDecl, PartExtras)>;

/// What a part declares beyond the sensing core's `PartDecl`.
#[derive(Clone, Debug, Default)]
pub(crate) struct PartExtras {
    pub focusable: bool,
    pub exclusive: bool,
    pub focus_shape: Option<FocusShape>,
    pub drag_cursor: Option<CursorIcon>,
    pub tooltip: Option<Tooltip>,
    pub pointer_only: Option<&'static str>,
}

impl PartBuilder<'_> {
    /// What the part senses; hover only by default.
    pub fn sense(self, sense: Sense) -> Self {
        self.decl.sense = sense;
        self
    }

    /// The part's level and sub-level in the resolve order.
    pub fn priority(self, priority: HitPriority) -> Self {
        self.decl.priority = priority;
        self
    }

    /// The cursor while the pointer is over the part.
    pub fn cursor(self, cursor: CursorIcon) -> Self {
        self.decl.cursor = Some(cursor);
        self
    }

    /// The cursor while the part is dragged.
    pub fn drag_cursor(self, cursor: CursorIcon) -> Self {
        self.declared.drag_cursor = Some(cursor);
        self
    }

    /// The part is a focus stop.
    pub fn focusable(self) -> Self {
        self.declared.focusable = true;
        self
    }

    /// The part joins the exclusive hit list, written per egui layer and read next frame. The list feeds only the
    /// Windows hit-test sink; presses inside the app on the part follow the press-target rule.
    pub fn exclusive(self) -> Self {
        self.declared.exclusive = true;
        self
    }

    /// The shape the focus ring takes around the part.
    pub fn focus_shape(self, shape: FocusShape) -> Self {
        self.declared.focus_shape = Some(shape);
        self
    }

    /// Value flags of the part, merged into its state at step 12 and its sub-node at step 16.
    pub fn flags(self, flags: Flags) -> Self {
        self.decl.flags |= flags;
        self
    }

    /// The part sits above the body or items in the stacking order and is painted by `paint_over`.
    pub fn over(self) -> Self {
        self.decl.over = true;
        self
    }

    /// The part's tooltip.
    pub fn tooltip(self, tooltip: impl Into<Tooltip>) -> Self {
        self.declared.tooltip = Some(tooltip.into());
        self
    }

    /// A part with no focus stop names its keyboard path: the intent or command that does the same.
    pub fn pointer_only(self, reason: &'static str) -> Self {
        self.declared.pointer_only = Some(reason);
        self
    }
}

/// Step 7's context: the step-5 read view plus the rect geometry and the part builders.
#[non_exhaustive]
pub struct PartsCx<'a, C: Component> {
    /// The rect minus padding, mirrored for right-to-left.
    pub content_rect: Rect,
    /// Start and end geometry of the rect.
    pub flow: Flow,
    pub(crate) decls: &'a mut Decls,
    pub(crate) rovings: &'a mut Vec<Roving>,
    pub(crate) last_state: (bool, bool),
    pub(crate) measure: MeasureCx<'a, C>,
}

impl<'a, C: Component> Deref for PartsCx<'a, C> {
    type Target = MeasureCx<'a, C>;

    fn deref(&self) -> &MeasureCx<'a, C> {
        &self.measure
    }
}

impl<C: Component> PartsCx<'_, C> {
    /// Last pass's pointer-over state of this component, so `parts` can register hover-revealed parts.
    pub fn hovered(&self) -> bool {
        self.last_state.0
    }

    /// Last pass's focus-visible state of this component.
    pub fn focus_visible(&self) -> bool {
        self.last_state.1
    }

    /// Declares a part; a repeated part is named with its index: `cx.part(POINT.at(id), shape)`.
    pub fn part(&mut self, part: impl Into<PartRef>, shape: impl Into<HitShape>) -> PartBuilder<'_> {
        let part = part.into();
        let mut decl = PartDecl::new(part.key, shape, Sense::HOVER);
        decl.index = part.index;
        decl.priority = HitPriority::new(HitLevel::Body, 0);
        let at = self.decls.len();
        self.decls.push((decl, PartExtras::default()));
        let (decl, declared) = &mut self.decls[at];
        PartBuilder { decl, declared }
    }

    /// Declares a roving group over part and child keys.
    pub fn roving(&mut self, roving: Roving) {
        self.rovings.push(roving);
    }
}

/// The cursor requests of the hovered parts' declared cursors, in registration order.
pub(crate) fn declared_cursor(core: &CxCore<'_>) -> Option<CursorIcon> {
    core.part_cursors.first().copied()
}

/// Writes the cursor a component asked for into the cursor service.
pub(crate) fn write_cursor(core: &CxCore<'_>) {
    if let Some((cursor, priority)) = core.cursor.clone() {
        cursor::request(&core.ctx, cursor, priority);
    } else if let Some(icon) = declared_cursor(core) {
        cursor::request(&core.ctx, Cursor::System(icon), CursorPriority::Part);
    }
}

/// An env derived for children `PaintUi` builds: the composite's resolved `enabled` and `size`.
pub(crate) fn child_env(env: &Env, enabled: bool, size: craft_theme::ControlSize) -> Env {
    env.derive(|scoped| {
        scoped.enabled &= enabled;
        scoped.size = size;
    })
}

impl Drives for CxCore<'_> {
    fn driver(&self) -> Driver<'_> {
        Driver { stage: self.memory, base: self.id, policy: self.env.motion() }
    }
}

impl<C: Component> Drives for Cx<'_, C> {
    fn driver(&self) -> Driver<'_> {
        self.core.driver()
    }
}

impl Drives for BodyCx<'_> {
    fn driver(&self) -> Driver<'_> {
        Driver { stage: self.memory, base: self.id, policy: self.env.motion() }
    }
}

impl<K> Drives for ItemCx<'_, '_, K> {
    fn driver(&self) -> Driver<'_> {
        Driver { stage: self.memory, base: self.id, policy: self.env.motion() }
    }
}
