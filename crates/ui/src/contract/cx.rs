//! The context types (1.5): `CxCore`, `Cx`, `BodyCx`, `ItemCx`, `MeasureCx`, `PartsCx` and `PartBuilder`, and the
//! chapter traits implemented on them.

use super::label::{self, LeafLabel};
use super::run::ShowAt;
use super::{Component, EditResponse, Resolved, Show, SideLabel, Tooltip, Validity, ValueOf};
use crate::primitives::sensing::{PartDecl, PartResults};
use crate::primitives::{Adopted, PaintRecord, PartResult, TextEnv, TextLayout, TextRun};
use crate::runtime::access::{self, FieldLabel, Politeness};
use crate::runtime::commands::{CommandAccess, CommandChain, Commands, ResolvedCommand};
use crate::runtime::drag::{self, DragPreview, DropTarget, WindowDropTarget};
use crate::runtime::egui::{alloc, areas};
use crate::runtime::env::{Env, EnvView, Label};
use crate::runtime::focus::{self, FocusShape};
use crate::runtime::keys::{Intents, MenuKey};
use crate::runtime::memory::{MemoryStage, StageAccess, sealed};
use crate::runtime::motion::{Driver, Drives};
use crate::runtime::overlay::CloseReason;
use crate::runtime::{cursor, frame, pointer, wheel};
use crate::style::{ComponentStyle as _, Ink, Mix, StateSet, Stateful, UiTokens};
use craft_behavior::{
    Axis, CancelReason, Cursor, CursorIcon, CursorPriority, DropOperation, EditEvent, Flow, Gesture, GesturePhase, HitLevel, HitPriority, HitShape,
    Mixed, PartKey, PartRef, PressCancel, PressPhase, PressState, Proposal, Roving, ScrollSpan, Sense, Value, Wheel,
};
use craft_i18n::{Format, WordSet};
use craft_keymap::Chord;
use craft_theme::Flags;
use egui::{Context, Id, Pos2, Rect, Response, Ui, Vec2};
use std::cell::RefCell;
use std::fmt::Debug;
use std::hash::Hash;
use std::ops::{Deref, Range};
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
    /// What focus holds of this component, read once at step 8.
    pub(crate) focus: crate::runtime::focus::Facts,
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
    /// The visible range each scroll part moved to this pass, from step 8's results.
    pub(crate) scroll_moves: Vec<(PartKey, Range<f64>)>,
    /// The parts that show the value, whose double click resets it like the root's.
    pub(crate) value_parts: Vec<(PartKey, Option<u64>)>,
    /// The value parts that declared `own_reset`.
    pub(crate) own_reset_parts: Vec<(PartKey, Option<u64>)>,
    /// The part a reset targeted that declared `own_reset`; step 9 leaves its value to `update`.
    pub(crate) reset_part: Option<PartRef>,
    /// The sense the root's egui widget carries, for the forwarding widget of a transformed body.
    pub(crate) root_sense: Sense,
    /// The member of the container's roving group Tab arrives at, set by the group's pick.
    pub(crate) roving_arrival: Option<Id>,
    /// The events and visible range the item step left for `collection_response`.
    pub(crate) collection_events: Option<Box<dyn std::any::Any>>,
    pub(crate) visible_rows: Range<usize>,
    /// What the collection's props allow, for `collection_response`.
    pub(crate) gestures: super::collection::Gestures,
    /// The text part's result of this pass, from step 8b.
    pub(crate) texts: Option<(PartKey, Box<dyn std::any::Any>)>,
    /// Where the text part reserved the surface in the painter, so the field's surface paints beneath its text.
    pub(crate) surface_slot: Option<egui::layers::ShapeIdx>,
    /// The editor part as step 8b set it up.
    pub(crate) editor: Option<super::editor::EditorInfo>,
    /// The editor part's inputs of this pass, until `editor_input` takes them.
    pub(crate) editor_inputs: Vec<craft_behavior::EditorInput>,
    /// The text part's draft is not a value of its codec.
    pub(crate) text_invalid: bool,
    /// The row and column counts of the collection, which the item step records for its node.
    pub(crate) grid: crate::runtime::access::GridPlace,
    /// The text part's completion list is open: Escape belongs to its popup, not to the component's gesture.
    pub(crate) escape_to_list: bool,
    /// The text facts step 16 gives AccessKit, from the text adapter or the editor's `editor_state`.
    pub(crate) text_access: RefCell<Option<crate::runtime::access::text::TextFacts>>,
    /// Enter was pressed in the text part.
    pub(crate) submitted: bool,
    /// Escape abandoned the text part's edit.
    pub(crate) escaped: bool,
}

impl<C: Component> Cx<'_, C> {
    /// The part a reset aimed at, when it declared `own_reset`; `update` proposes the value with that part at its
    /// default. Reads `None` when the reset was the whole binding's, which step 9 applied.
    pub fn reset_part(&self) -> Option<PartRef> {
        self.core.reset_part
    }

    /// Asks the platform to open `url`, in a new tab where the platform has tabs and `new_tab` is set.
    pub fn open_url(&mut self, url: &str, new_tab: bool) {
        crate::runtime::egui::output::open_url(&self.core.ctx, url, new_tab);
    }

    /// Makes this component a member of the roving group of the body it is shown in: its stop joins the group and
    /// Tab visits the group once. A component that never calls it keeps its own Tab stop.
    pub fn join_roving(&mut self) {
        if let Some(group) = self.core.env.roving() {
            super::steps::join_group(&self.core.ctx, group, self.core.id);
        }
    }

    /// Names the member of this container's roving group (`ContainerProps.roving`) that Tab arrives at; the
    /// first member when none is named.
    pub fn roving_arrival(&mut self, member: Id) {
        self.core.roving_arrival = Some(member);
    }

    /// Moves focus to one of this component's focusable parts, in this pass: the rest of the pass reads the new
    /// focus. For `update` after a press on the component or while it holds focus; it never takes focus from elsewhere.
    pub fn focus_part(&mut self, part: impl Into<PartRef>) {
        let part = part.into();
        debug_assert!(
            self.core.focus.focused || self.core.press_held() || self.core.results.values().any(|result| result.press.phase.is_press()),
            "`focus_part` is for a component that holds focus or was just pressed; `update` never steals focus"
        );
        focus::request(&self.core.ctx, focus::FocusTarget::Part(self.core.id, part));
        self.core.sync_focus();
    }

    /// Moves focus to the component's own stop (not a part), under the rule of `focus_part`.
    pub fn focus_own(&mut self) {
        debug_assert!(
            self.core.focus.focused || self.core.press_held(),
            "`focus_own` is for a component that holds focus or was just pressed; `update` never steals focus"
        );
        focus::request(&self.core.ctx, focus::FocusTarget::Id(self.core.id));
        self.core.sync_focus();
    }
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
    /// The result of `part` this pass, `None` when `parts` did not declare it: for parts that exist only under a
    /// condition.
    fn part_if_declared(&self, part: impl Into<PartRef>) -> Option<&PartResult>;
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
    /// Starts the drag session with `payload`; the context holds one session at a time, and a running one stays.
    fn start_drag<P: Clone + Send + Sync + 'static>(&mut self, payload: P, preview: DragPreview);
    /// The active drag payload of type `P` and the pointer position in this component's egui layer space (8.3), from any
    /// viewport.
    fn dragging<P: Send + Sync + 'static>(&self) -> Option<(Arc<P>, Pos2)>;
    /// Registers `target` over this component's rect for this pass, in `update`. Returns the drop released over it this
    /// pass, when it was the topmost accepting target under the pointer last pass and accepts now.
    fn drop_target<P: Send + Sync + 'static, T: DropTarget<P>>(&mut self, target: &T) -> Option<T::Drop>;
    /// Registers `target` over the whole window of the running viewport for this pass, in `update`; a target over a
    /// component beats it wherever that accepts. The payload is read without a pointer position, so files hovered
    /// before the pointer has a position reach it. OS file drops (`OsFiles`) land on it wherever no other target
    /// takes them.
    fn window_drop_target<P: Send + Sync + 'static, T: WindowDropTarget<P>>(&mut self, target: &T) -> Option<T::Drop>;
    /// The operation of a drop of this component's payload, in the pass after the drop; `None` while dragging, when the
    /// drag was cancelled and when no target took it.
    fn drag_ended<P: Send + Sync + 'static>(&self) -> Option<DropOperation>;
    /// The visible range the scroll part `key` moved to this pass (thumb or end drag, page press).
    fn scroll_moved(&self, key: PartKey) -> Option<Range<f64>>;
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

    fn part_if_declared(&self, part: impl Into<PartRef>) -> Option<&PartResult> {
        let part = part.into();
        self.results.get(&(part.key, part.index))
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

    fn start_drag<P: Clone + Send + Sync + 'static>(&mut self, payload: P, preview: DragPreview) {
        drag::start(&self.ctx, self.id, Arc::new(payload), preview);
    }

    fn dragging<P: Send + Sync + 'static>(&self) -> Option<(Arc<P>, Pos2)> {
        let (payload, at) = drag::dragging::<P>(&self.ctx)?;
        Some((payload, areas::layer_map(&self.ctx, self.response.layer_id).map_or(at, |map| map.to_layer(at))))
    }

    fn drop_target<P: Send + Sync + 'static, T: DropTarget<P>>(&mut self, target: &T) -> Option<T::Drop> {
        let (payload, at) = self.dragging::<P>()?;
        // Only where the pointer is over this component's visible shapes on its own layer: not under a layer above, not
        // in a clipped-out part.
        let screen = areas::layer_map(&self.ctx, self.response.layer_id).map_or(at, |map| map.to_screen(at));
        if !self.visible.contains(at) || areas::layer_at(&self.ctx, screen) != Some(self.response.layer_id) {
            return None;
        }
        let op = drag::current_operation(&self.ctx);
        let drop = target.accept(&payload, at, op)?;
        let ordinal = drag::next_ordinal(&self.ctx, self.id);
        drag::register(&self.ctx, self.id, self.response.layer_id, target.mark(&drop), false);
        drag::take_drop::<P>(&self.ctx, self.id, ordinal, op).then_some(drop)
    }

    fn window_drop_target<P: Send + Sync + 'static, T: WindowDropTarget<P>>(&mut self, target: &T) -> Option<T::Drop> {
        let payload = drag::active::<P>(&self.ctx)?;
        let op = drag::current_operation(&self.ctx);
        let drop = target.accept(&payload, op)?;
        let ordinal = drag::next_ordinal(&self.ctx, self.id);
        drag::register(&self.ctx, self.id, self.response.layer_id, target.mark(&drop), true);
        drag::take_drop::<P>(&self.ctx, self.id, ordinal, op).then_some(drop)
    }

    fn drag_ended<P: Send + Sync + 'static>(&self) -> Option<DropOperation> {
        drag::ended::<P>(&self.ctx, self.id)
    }

    fn scroll_moved(&self, key: PartKey) -> Option<Range<f64>> {
        self.scroll_moves.iter().find(|(moved, _)| *moved == key).map(|(_, range)| range.clone())
    }
}

impl<C: Component> Pointer for Cx<'_, C> {
    fn part(&self, part: impl Into<PartRef>) -> &PartResult {
        self.core.part(part)
    }

    fn part_if_declared(&self, part: impl Into<PartRef>) -> Option<&PartResult> {
        self.core.part_if_declared(part)
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

    fn start_drag<P: Clone + Send + Sync + 'static>(&mut self, payload: P, preview: DragPreview) {
        self.core.start_drag(payload, preview);
    }

    fn dragging<P: Send + Sync + 'static>(&self) -> Option<(Arc<P>, Pos2)> {
        self.core.dragging::<P>()
    }

    fn drop_target<P: Send + Sync + 'static, T: DropTarget<P>>(&mut self, target: &T) -> Option<T::Drop> {
        self.core.drop_target(target)
    }

    fn window_drop_target<P: Send + Sync + 'static, T: WindowDropTarget<P>>(&mut self, target: &T) -> Option<T::Drop> {
        self.core.window_drop_target(target)
    }

    fn drag_ended<P: Send + Sync + 'static>(&self) -> Option<DropOperation> {
        self.core.drag_ended::<P>()
    }

    fn scroll_moved(&self, key: PartKey) -> Option<Range<f64>> {
        self.core.scroll_moved(key)
    }
}

/// The resolved look (chapter 4); every method reads step 12's output.
pub trait Styled {
    /// The state set.
    fn state(&self) -> &StateSet;
    /// The root's own state: the hover and press of the root without those of its parts, for a control whose parts
    /// each paint their own state (a split button's main part).
    fn root_state(&self) -> StateSet;
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

    /// The active part, read again when focus was written since step 8.
    pub(crate) fn active_now(&self) -> Option<PartRef> {
        if focus::writes() == self.focus.at {
            return self.active;
        }
        focus::facts(&self.ctx, self.id).part.filter(|part| self.results.contains_key(&(part.key, part.index)))
    }

    /// Reads the focus facts again when focus was written since they were read, so a request or a layer that took
    /// focus during this pass shows in the steps after it.
    pub(crate) fn sync_focus(&mut self) {
        if focus::writes() != self.focus.at {
            self.focus = focus::facts(&self.ctx, self.id);
            self.active = self.focus.part.filter(|part| self.results.contains_key(&(part.key, part.index)));
        }
    }

    /// The state set of `part`: its press and hover, its own flags, and the focus flags only when it is the
    /// active part of the focused component.
    pub(crate) fn part_state(&self, part: PartRef) -> StateSet {
        let result = self.part(part);
        let mut flags = result.flags;
        if self.active_now() == Some(part) && focus::is_focused(&self.ctx, self.id) {
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

    fn root_state(&self) -> StateSet {
        self.assert_stated();
        let hovered = self.response.hovered() && !self.results.values().any(|result| result.hovered);
        let pressed = self.press.phase.is_press();
        let state = if !self.resolved.enabled {
            craft_theme::State::Disabled
        } else if pressed {
            craft_theme::State::Pressed
        } else if hovered {
            craft_theme::State::Hover
        } else {
            craft_theme::State::Rest
        };
        StateSet::new(
            state,
            self.state.flags,
            craft_theme::StateMix::new(f32::from(u8::from(hovered)), f32::from(u8::from(pressed)), self.state.t.on),
        )
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

    fn root_state(&self) -> StateSet {
        self.core.root_state()
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
    /// Registers that this component is a menu bar that takes the menu-bar keys of the next input hook (9.7).
    fn register_menu_bar(&mut self);
    /// The menu-bar keys of the pass: F10 or a lone Alt press focus the bar, Alt and a letter open a menu.
    fn take_menu_keys(&mut self) -> Vec<MenuKey>;
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
        self.active_now()
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

    fn register_menu_bar(&mut self) {
        focus::register_menu_bar(&self.ctx);
    }

    fn take_menu_keys(&mut self) -> Vec<MenuKey> {
        crate::runtime::keys::take_menu_keys(&self.ctx)
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

    fn register_menu_bar(&mut self) {
        self.core.register_menu_bar();
    }

    fn take_menu_keys(&mut self) -> Vec<MenuKey> {
        self.core.take_menu_keys()
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
    /// child's, for step 15. A composite whose own value is mixed or not yet shown maps the edit onto `start` instead;
    /// its events then begin from a mixed value.
    fn adopt<V: Value>(&self, child: &EditResponse<V>, start: &Self::Value, map: impl FnOnce(&V, &mut Self::Value));

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

    /// An edit of a read-only or disabled component reaches nothing.
    fn refuses_edits(&self) -> bool {
        self.core.resolved.read_only || !self.core.resolved.enabled
    }

    /// Turns an adopted child event into the composite's own and records its gesture, as `begin`, `propose`, `commit`
    /// and `cancel` do for the component's own. The composite's gesture takes a fresh id; its `Commit` and `Cancel`
    /// carry the value the gesture began with, not the latest proposal. An event with no gesture of the composite's
    /// to end is dropped.
    pub(crate) fn absorb(&mut self, event: EditEvent<ValueOf<C>>) -> Option<EditEvent<ValueOf<C>>> {
        let dead = self.core.press_held();
        let fresh = crate::runtime::edit::next_gesture_id(&self.core.ctx);
        let slot = self.gesture_slot();
        match event {
            EditEvent::Begin { before, label, doc_key, .. } => {
                *slot = slot.ended(false);
                slot.id = fresh;
                slot.before = Some(before.clone());
                Some(EditEvent::begin(fresh, before, label, doc_key))
            }
            EditEvent::Update { value, .. } => {
                slot.proposal = Some(value.clone());
                Some(EditEvent::update(slot.id, value))
            }
            EditEvent::Commit { before, after, .. } => {
                let running = slot.is_running();
                let began = slot.before.clone().filter(|_| running);
                let id = if running { slot.id } else { fresh };
                *slot = slot.ended(false);
                Some(EditEvent::commit(id, began.unwrap_or(before), after))
            }
            EditEvent::Cancel { reason, .. } => {
                if !slot.is_running() {
                    return None;
                }
                let id = slot.id;
                let began = slot.before.clone().filter(|_| !reason.document_moved());
                *slot = slot.ended(dead);
                if let Some(before) = began.clone() {
                    self.shown = match before {
                        Mixed::Same(value) => Some(Arc::new(value)),
                        Mixed::Mixed => None,
                    };
                    self.mixed = self.shown.is_none();
                }
                Some(EditEvent::cancel(id, reason, began))
            }
            EditEvent::Set { before, after, label, doc_key, .. } => {
                *slot = slot.ended(false);
                Some(EditEvent::set(fresh, before, after, label, doc_key))
            }
            _ => None,
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

    fn adopt<V: Value>(&self, child: &EditResponse<V>, start: &ValueOf<C>, map: impl FnOnce(&V, &mut ValueOf<C>)) {
        let Some(event) = child.event.as_ref() else { return };
        let before = self.before();
        let mut mine = match &before {
            Mixed::Same(value) => value.clone(),
            Mixed::Mixed => start.clone(),
        };
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
        let mut response = EditResponse::new(self.core.response.clone(), event, self.validity.clone());
        response.escaped = self.core.escaped || matches!(self.events.last(), Some(EditEvent::Cancel { reason: CancelReason::Escape, .. }));
        response.submitted = self.core.submitted;
        response.shown = match (&self.shown, self.mixed) {
            (_, true) => Some(Mixed::Mixed),
            (Some(value), false) => Some(Mixed::Same(Arc::clone(value))),
            (None, false) => None,
        };
        response
    }

    fn propose(&mut self, v: ValueOf<C>) {
        if self.refuses_edits() {
            return;
        }
        let before = self.before();
        let (label, doc_key) = (self.core.label_arc(), self.core.resolved.doc_key);
        let fresh = crate::runtime::edit::next_gesture_id(&self.core.ctx);
        let slot = self.gesture_slot();
        // A gesture cancelled under its press stays cancelled until the release.
        if slot.dead {
            return;
        }
        let started = !slot.is_running();
        if started {
            slot.id = fresh;
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
        // A `Set` has its own id: a running gesture's id stays that gesture's, which the host ends when it applies the `Set`.
        let id = crate::runtime::edit::next_gesture_id(&self.core.ctx);
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
        let fresh = crate::runtime::edit::next_gesture_id(&self.core.ctx);
        let slot = self.gesture_slot();
        if !slot.is_running() && !slot.dead {
            slot.id = fresh;
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

impl<C: Component> CommandAccess for Cx<'_, C> {
    fn command_ctx(&self) -> &Context {
        &self.core.ctx
    }

    fn command_chain(&self) -> &CommandChain {
        self.core.env.command_chain()
    }

    fn command_owner(&self) -> Option<Id> {
        Some(self.core.id)
    }
}

impl<C: Component> Commands for Cx<'_, C> {}

impl CommandAccess for BodyCx<'_> {
    fn command_ctx(&self) -> &Context {
        self.memory.ctx()
    }

    fn command_chain(&self) -> &CommandChain {
        self.env.command_chain()
    }

    fn command_owner(&self) -> Option<Id> {
        None
    }
}

impl Commands for BodyCx<'_> {}

impl<K> CommandAccess for ItemCx<'_, '_, K> {
    fn command_ctx(&self) -> &Context {
        self.memory.ctx()
    }

    fn command_chain(&self) -> &CommandChain {
        self.env.command_chain()
    }

    fn command_owner(&self) -> Option<Id> {
        None
    }
}

impl<K> Commands for ItemCx<'_, '_, K> {}

/// The label a component shows and the built-in text it reads (chapter 19).
pub trait WordSetCx {
    /// The label as given, a cheap clone.
    fn label(&self) -> Label;

    /// The word set `L` in the language of the scope; built once per installed `Words`.
    fn words<L: WordSet + Send + Sync + 'static>(&self) -> Arc<L>;

    /// How the scope writes numbers: locale digits, separators and grouping.
    fn format(&self) -> &Format;
}

impl WordSetCx for CxCore<'_> {
    fn label(&self) -> Label {
        self.resolved.label_value.clone()
    }

    fn words<L: WordSet + Send + Sync + 'static>(&self) -> Arc<L> {
        self.env.words()
    }

    fn format(&self) -> &Format {
        self.env.format()
    }
}

impl<C: Component> WordSetCx for Cx<'_, C> {
    fn label(&self) -> Label {
        self.core.label()
    }

    fn words<L: WordSet + Send + Sync + 'static>(&self) -> Arc<L> {
        self.core.words()
    }

    fn format(&self) -> &Format {
        self.core.format()
    }
}

impl<C: Component> WordSetCx for MeasureCx<'_, C> {
    fn label(&self) -> Label {
        self.resolved.label_value.clone()
    }

    fn words<L: WordSet + Send + Sync + 'static>(&self) -> Arc<L> {
        self.env.words()
    }

    fn format(&self) -> &Format {
        self.env.format()
    }
}

/// Live announcements and the enclosing field label (chapter 12).
pub trait Accessible {
    /// Announces transient text; it is flushed at frame end as one status node and returned in `FrameOutput`.
    fn announce(&mut self, text: impl Into<String>, politeness: Politeness);

    /// The label of the enclosing field container, when the component sits in one.
    fn field_label(&self) -> Option<FieldLabel>;
}

impl Accessible for CxCore<'_> {
    fn announce(&mut self, text: impl Into<String>, politeness: Politeness) {
        access::announce(&self.ctx, text, politeness);
    }

    fn field_label(&self) -> Option<FieldLabel> {
        access::scope::field_label(&self.ctx)
    }
}

impl<C: Component> Accessible for Cx<'_, C> {
    fn announce(&mut self, text: impl Into<String>, politeness: Politeness) {
        self.core.announce(text, politeness);
    }

    fn field_label(&self) -> Option<FieldLabel> {
        self.core.field_label()
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
    open: &'a mut bool,
    gate: Option<&'a mut bool>,
    close: Option<CloseReason>,
}

impl<'a> BodyCx<'a> {
    pub(crate) fn new(
        id: Id,
        env: EnvView<'a>,
        resolved: &'a Resolved<'a>,
        rect: Rect,
        memory: &'a mut MemoryStage,
        open: &'a mut bool,
        gate: Option<&'a mut bool>,
    ) -> Self {
        memory.claim(id);
        BodyCx { id, env, resolved, rect, memory, open, gate, close: None }
    }

    /// Whether the body is open.
    pub fn is_open(&self) -> bool {
        *self.open
    }

    /// The open state: a title row's disclosure binds to it, and step 15 writes a change to its owner.
    pub fn open(&mut self) -> &mut bool {
        self.open
    }

    /// The gate's shown copy, for a container whose value is a bool: a title row's check box binds to it, and
    /// step 15 applies a change. Under a gate that is off the body is disabled.
    pub fn gate(&mut self) -> Option<&mut bool> {
        self.gate.as_deref_mut()
    }

    /// Asks the container to close, as `CloseReason::Requested`: step 15 closes it.
    pub fn request_close(&mut self) {
        self.request_close_as(CloseReason::Requested);
    }

    /// Asks the container to close and tells it why (an OK button is `Activated`, Enter is `Enter`): step 15
    /// closes it and `Container::closed` gets `why`. The first request of a pass keeps its reason.
    pub fn request_close_as(&mut self, why: CloseReason) {
        self.close.get_or_insert(why);
    }

    pub(crate) fn close_requested(&self) -> Option<CloseReason> {
        self.close
    }

    /// Runs the label step (B2, 16.3) for one leaf child in the body's flow: `label` sits beside the child or above
    /// it, with the icon and detail of `side`, and the child takes the width the label leaves. The row is as high as
    /// the placement measured: the child (plus the label line when the label stacks), and the label when it is taller.
    /// `ui` is the body's `Ui`; `key` makes the child's id.
    pub fn labelled<C: Show>(&mut self, ui: &mut Ui, key: impl Hash + Debug, label: Label, side: SideLabel, child: C) -> C::Output {
        let id = self.id.with(key);
        let env = Env::of(ui);
        let mut child = child;
        let under = Some((self.resolved.enabled, self.resolved.size));
        let ideal = super::measure_under(&mut child, &*ui, Some(id), Proposal::Ideal, under);
        let avail = ui.available_rect_before_wrap();
        // The child's own height: a stacked label pushes the control down, and the allocation below follows the placement.
        let row = Rect::from_min_size(avail.min, Vec2::new(avail.width(), ideal.y));
        let at = LeafLabel {
            id,
            owner: self.resolved.id,
            parent: None,
            row,
            text: &label,
            side: &side,
            enabled: self.resolved.enabled,
            size: self.resolved.size,
        };
        let placed = label::leaf(ui, &env, &at, &mut child);
        let control = Rect::from_min_size(placed.control.min, Vec2::new(placed.control.width(), ideal.y));
        let output = super::run::show_leaf(super::Props::id(child, id), ui, ShowAt::Rect(control), None);
        let bottom = control.bottom().max(placed.label.bottom());
        placed.finish(ui.ctx());
        alloc::space(ui, Vec2::new(avail.width(), (bottom - row.top()).max(ideal.y)));
        output
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

    fn words<L: WordSet + Send + Sync + 'static>(&self) -> Arc<L> {
        self.env.words()
    }

    fn format(&self) -> &Format {
        self.env.format()
    }
}

impl<K> WordSetCx for ItemCx<'_, '_, K> {
    /// An item has no label of its own: the empty label.
    fn label(&self) -> Label {
        Label::default()
    }

    fn words<L: WordSet + Send + Sync + 'static>(&self) -> Arc<L> {
        self.env.words()
    }

    fn format(&self) -> &Format {
        self.env.format()
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
    /// Selection, lead, open and renaming state of the row.
    pub row: super::RowState,
    /// The nesting depth, 0 for a root.
    pub depth: usize,
    /// The item's place in the set.
    pub set: crate::runtime::access::SetPosition,
    /// The row's zero-based index in the collection.
    pub row_index: usize,
    /// The number of rows in the collection.
    pub row_count: usize,
    /// The number of columns the rows show, 0 without columns.
    pub column_count: usize,
    /// What the inherited context holds that component code may read.
    pub env: EnvView<'a>,
    /// Base id of the item's stage: `sub(item_id, "item")`.
    pub(crate) id: Id,
    pub(crate) memory: &'a mut MemoryStage,
    pub(crate) tip: Option<ItemTip<'r>>,
    pub(crate) cells: Vec<(craft_behavior::ColumnKey, Rect)>,
    pub(crate) grid: Option<&'a craft_behavior::GridView>,
}

impl<'r, K> ItemCx<'_, 'r, K> {
    /// The visible columns of the row with their rects.
    pub fn cells(&self) -> &[(craft_behavior::ColumnKey, Rect)] {
        &self.cells
    }

    /// The zero-based position of column `key` among the columns the row shows.
    pub fn column_index(&self, key: &craft_behavior::ColumnKey) -> Option<usize> {
        self.cells.iter().position(|(column, _)| column == key)
    }

    /// The rect of column `key` in this row.
    pub fn cell(&self, key: &craft_behavior::ColumnKey) -> Option<Rect> {
        self.cells.iter().find(|(column, _)| column == key).map(|(_, rect)| *rect)
    }

    /// The sections of a sheet layout.
    pub fn grid(&self) -> Option<&craft_behavior::GridView> {
        self.grid
    }

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
    /// The bound command as step 1 resolved it, with the scoped bindings of the scopes above the component.
    pub fn command(&self) -> Option<&ResolvedCommand> {
        self.resolved.command
    }

    /// The label as given.
    pub fn label(&self) -> Label {
        self.resolved.label_value.clone()
    }

    /// Lays `run` out in the room the parent offers, as `Draw::text` will paint it.
    pub fn layout(&self, run: &TextRun<'_>) -> TextLayout {
        self.layout_at(run, self.room.unwrap_or(f32::INFINITY))
    }

    /// Lays `run` out `width` wide, for a text that paints in a band narrower than the room.
    pub fn layout_at(&self, run: &TextRun<'_>, width: f32) -> TextLayout {
        let env = TextEnv { tokens: self.tokens, direction: self.env.direction(), ink: crate::style::ink_color(self.tokens, self.ink) };
        run.layout(self.ui.ctx(), &env, width)
    }

    /// Measures a child under the composite's resolved `enabled` and `size`, as `PaintUi::child` will show it.
    pub fn child<D: Show>(&self, key: impl Hash + Debug, child: &mut D, proposal: Proposal) -> Vec2 {
        let id = self.id.with(key);
        super::measure_under(child, self.ui, Some(id), proposal, Some(self.enabled_size))
    }

    /// The size `face` has when `PaintUi::face` shows it as the look of a part: steps 1, 2 and 4 of the face run
    /// under the composite's resolved `enabled` and `size`, with no proposal.
    pub fn face<S: Show>(&self, mut face: S) -> Vec2 {
        let id = self.id.with("face");
        super::measure_under(&mut face, self.ui, Some(id), Proposal::Ideal, Some(self.enabled_size))
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
    pub keyboard_via: Option<PartKey>,
    pub value: bool,
    pub own_reset: bool,
    pub selected: bool,
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

    /// The part is the arrival of the component's part roving group: Tab and Shift+Tab into the group land here, not
    /// on the first member. The last part declared `selected` wins.
    pub fn selected(self) -> Self {
        self.declared.selected = true;
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

    /// The part shows the component's value: a double click on it resets the value, as one on the root does. A
    /// stepper button or a scroll thumb is no value part.
    pub fn value_part(self) -> Self {
        self.declared.value = true;
        self
    }

    /// A reset aimed at the part (Reset while it is the focused part, a double click on it) does not reset the
    /// binding: `Cx::reset_part` names the part and `update` proposes the value with that part at its own default.
    /// Implies `value_part`.
    pub fn own_reset(self) -> Self {
        self.declared.value = true;
        self.declared.own_reset = true;
        self
    }

    /// A part with no focus stop names its keyboard path: the intent or command that does the same.
    pub fn pointer_only(self, reason: &'static str) -> Self {
        self.declared.pointer_only = Some(reason);
        self
    }

    /// A part with no focus stop of its own does the same as the focusable part `part` of this component, which
    /// carries the keyboard path.
    pub fn keyboard_via(self, part: PartKey) -> Self {
        self.declared.keyboard_via = Some(part);
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
    pub(crate) scrolls: &'a mut Vec<ScrollDecl>,
    pub(crate) text: &'a mut super::text::TextStep,
    pub(crate) last_state: (bool, bool),
    pub(crate) measure: MeasureCx<'a, C>,
    /// A collection's rows as the item step places them this pass.
    pub(crate) items: Option<std::rc::Rc<dyn std::any::Any>>,
}

impl<C: super::Collection> PartsCx<'_, C> {
    /// The rect of the item `key` this pass, in the coordinates the item step lays rows out in (a transformed
    /// collection's, before the transform). `None` for an item the step does not build: neither in view nor kept.
    pub fn rect_of(&self, key: &C::Key) -> Option<Rect> {
        let rows = self.items.as_ref()?.downcast_ref::<super::items::Rows<C::Key>>()?;
        rows.rects.iter().find(|(held, _)| held == key).map(|(_, rect)| *rect)
    }

    /// How far the rows are scrolled this pass.
    pub fn offset(&self) -> craft_behavior::ScrollPos {
        self.items.as_ref().and_then(|rows| rows.downcast_ref::<super::items::Rows<C::Key>>()).map_or_else(Default::default, |rows| rows.offset)
    }
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
        if part.key == PartKey::CELL
            && let Some(column) = part.index
        {
            decl.flags |= super::items::rows::cell_flags(self.measure.ui.ctx(), self.measure.id, column);
        }
        let at = self.decls.len();
        self.decls.push((decl, PartExtras::default()));
        let (decl, declared) = &mut self.decls[at];
        PartBuilder { decl, declared }
    }

    /// Declares a roving group over part and child keys.
    pub fn roving(&mut self, roving: Roving) {
        self.rovings.push(roving);
    }

    /// Declares one axis's scroll bar: the track as part `key` and the thumb as part `key.at(THUMB)`, inside
    /// `rect`. The pipeline runs thumb drag, end drag when the span resizes its ends, and page press on the
    /// track; `Pointer::scroll_moved(key)` reads the range the part moved to.
    pub fn scroll(&mut self, key: PartKey, rect: Rect, axis: Axis, span: ScrollSpan) {
        let rtl = self.flow.is_rtl();
        let min_thumb = crate::style::scroll_bar_look(self.measure.tokens).min_thumb;
        let (at, len) = craft_behavior::scroll::thumb(&span, track_length(rect, axis), min_thumb);
        let thumb = thumb_rect(rect, axis, rtl, at, len);
        self.part(key, rect)
            .sense(Sense::CLICK)
            .priority(HitPriority::new(HitLevel::Body, 0))
            .pointer_only("Page Up and Page Down scroll the region");
        self.part(key.at(THUMB), thumb)
            .sense(Sense::DRAG)
            .priority(HitPriority::new(HitLevel::Body, 1))
            .pointer_only("Page Up and Page Down scroll the region");
        self.scrolls.push(ScrollDecl { key, axis, rtl, track: rect, thumb, span, min_thumb });
    }
}

/// The index of a scroll bar's thumb part.
pub(crate) const THUMB: u64 = 0;

/// One axis's scroll bar, declared by `PartsCx::scroll`.
pub(crate) struct ScrollDecl {
    pub key: PartKey,
    pub axis: Axis,
    pub rtl: bool,
    pub track: Rect,
    pub thumb: Rect,
    pub span: ScrollSpan,
    pub min_thumb: f32,
}

fn track_length(rect: Rect, axis: Axis) -> f32 {
    match axis {
        Axis::X => rect.width(),
        _ => rect.height(),
    }
}

/// The thumb `at` points from the start edge of `track`, `len` long; the start edge of the x axis is the right
/// edge under right-to-left.
pub(crate) fn thumb_rect(track: Rect, axis: Axis, rtl: bool, at: f32, len: f32) -> Rect {
    match axis {
        Axis::X if rtl => Rect::from_min_size(egui::pos2(track.max.x - at - len, track.min.y), Vec2::new(len, track.height())),
        Axis::X => Rect::from_min_size(egui::pos2(track.min.x + at, track.min.y), Vec2::new(len, track.height())),
        _ => Rect::from_min_size(egui::pos2(track.min.x, track.min.y + at), Vec2::new(track.width(), len)),
    }
}

/// The ranges this pass's scroll-part presses moved, from step 8's results.
pub(crate) fn scroll_moves(decls: &[ScrollDecl], results: &PartResults) -> Vec<(PartKey, Range<f64>)> {
    let mut moves = Vec::new();
    for decl in decls {
        let sign = if decl.axis == Axis::X && decl.rtl { -1.0 } else { 1.0 };
        let along = |delta: Vec2| sign * if decl.axis == Axis::X { delta.x } else { delta.y };
        let length = track_length(decl.track, decl.axis);
        let thumb = results.get(&(decl.key, Some(THUMB))).map(|result| result.press.phase);
        let track = results.get(&(decl.key, None)).map(|result| result.press.phase);
        let moved = match (thumb, track) {
            (Some(PressPhase::Drag { delta, .. }), _) => Some(craft_behavior::scroll::drag(&decl.span, length, decl.min_thumb, along(delta))),
            (_, Some(PressPhase::Down { start, .. })) => {
                let at = if decl.axis == Axis::X { start.origin.x } else { start.origin.y };
                let thumb_at = if decl.axis == Axis::X { decl.thumb.center().x } else { decl.thumb.center().y };
                Some(craft_behavior::scroll::page(&decl.span, sign * (at - thumb_at) > 0.0))
            }
            _ => None,
        };
        if let Some(range) = moved {
            moves.push((decl.key, range));
        }
    }
    moves
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
    env.derive_local(|scoped| {
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
