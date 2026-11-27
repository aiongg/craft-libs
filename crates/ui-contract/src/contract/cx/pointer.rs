//! The `Pointer` trait: presses, hover, drags and cursors, on `CxCore` and `Cx`.

use super::context::{Activation, Cx, CxCore};
use crate::contract::rt::drag::DragSource;
use crate::contract::rt::node::NodeInput;
use crate::contract::{Component, Hover};
use crate::primitives::Id;
use crate::primitives::PartResult;
use crate::runtime::drag::{DragPreview, DropTarget, Typed, WindowDropTarget};
use craft_behavior::{Cursor, CursorPriority, DropOperation, MultiTap, PartKey, PartRef, PressCancel, PressState, Wheel};
use craft_ui_rt::{Pos2, Vec2};
use std::any::{Any, TypeId};
use std::ops::Range;
use std::sync::Arc;

/// Pointer state and requests (chapter 8).
pub trait Pointer {
    /// The result of `part` this pass: idle for a part first registered this frame. A part `parts` did not
    /// declare is a debug panic.
    fn part(&self, part: impl Into<PartRef>) -> &PartResult;
    /// The result of `part` this pass, `None` when `parts` did not declare it: for parts that exist only under a
    /// condition.
    fn part_if_declared(&self, part: impl Into<PartRef>) -> Option<&PartResult>;
    /// Where the pointer is over `part` this pass, in window coordinates; `None` when `part` is not hovered.
    /// The read marks the part as following the pointer (`pointer::follows_pointer`): a memoized view around the
    /// component runs on each move over the part, and the pointer-move gate runs a pass for it. A component that only
    /// asks whether the part is hovered reads `PartResult::hovered`.
    fn hover_pos(&self, part: impl Into<PartRef>) -> Option<Pos2>;
    /// The component's own press.
    fn press(&self) -> &PressState;
    /// The pointer is over the component's own shapes.
    fn hit(&self) -> bool;
    /// The pointer's stay over the component: over it, entered, left, and clicked while it has stayed over.
    fn hover(&self) -> Hover;
    /// Set only when the component's own shapes won the press and it was released as a click, or
    /// `Intent::Activate` was routed to it.
    fn activation(&self) -> Option<Activation>;
    /// For an arena root, the member activated this pass.
    fn activated_member(&self) -> Option<Id>;
    /// This pass's wheel share, once, with the pointer it came at (`Wheel::at`) in window coordinates (8.3).
    fn take_wheel(&mut self) -> Option<Wheel>;
    /// This pass's tap, once, when it lies in this component's rect and this component (an arena root with its members)
    /// wins a press there.
    fn take_multi_tap(&mut self) -> Option<MultiTap>;
    /// Feeds `pan`, the line pan this pass gave, to this component's wheel ease and returns the eased step for this
    /// pass. `Vec2::ZERO` on a pass with no line pan lets owed motion keep paying.
    fn ease_wheel(&mut self, pan: Vec2) -> Vec2;
    /// Drops this component's wheel ease and the motion it owes.
    fn stop_wheel_ease(&mut self);
    /// Requests `cursor` at `priority`; the highest priority of the pass wins.
    fn cursor(&mut self, cursor: impl Into<Cursor>, priority: CursorPriority);
    /// Ends every captured press of this viewport.
    fn cancel_press(&mut self, reason: PressCancel);
    /// Starts the drag session with `payload`; the context holds one session at a time, and a running one stays.
    fn start_drag<P: Clone + Send + Sync + 'static>(&mut self, payload: P, preview: DragPreview);
    /// Starts the drag session with `payload`, which also carries the payloads `also`, one per type: targets of any of
    /// these types see the one drag.
    fn start_drag_also<P: Clone + Send + Sync + 'static>(
        &mut self,
        payload: P,
        also: Vec<Arc<dyn std::any::Any + Send + Sync>>,
        preview: DragPreview,
    );
    /// The active drag payload of type `P` and the pointer position in window coordinates (8.3), from any
    /// viewport.
    fn dragging<P: Send + Sync + 'static>(&self) -> Option<(Arc<P>, Pos2)>;
    /// Registers `target` over this component's rect for this pass, in `update`. Returns the drop released over it this
    /// pass, when it was the topmost accepting target under the pointer last pass and accepts now.
    fn drop_target<P: Send + Sync + 'static, T: DropTarget<P>>(&mut self, target: &T) -> Option<T::Drop>;
    /// Registers `target` over the whole window of the running viewport for this pass, in `update`; a target over a
    /// component beats it wherever that accepts. By default the payload is read without a pointer position; a target
    /// that requires one waits for it so the target at that position can take the drop.
    fn window_drop_target<P: Send + Sync + 'static, T: WindowDropTarget<P>>(&mut self, target: &T) -> Option<T::Drop>;
    /// The operation of a drop of this component's payload, in the pass after the drop; `None` while dragging, when the
    /// drag was cancelled and when no target took it.
    fn drag_ended<P: Send + Sync + 'static>(&self) -> Option<DropOperation>;
    /// The visible range the scroll part `key` moved to this pass (thumb or end drag, page press, or an AccessKit click
    /// on the track, which pages toward the track's centre).
    fn scroll_moved(&self, key: PartKey) -> Option<Range<f64>>;
}

impl Pointer for CxCore<'_> {
    /// Step 12 leaves the report; a run that stopped before it (a sizing pass) reports no stay, so its output never
    /// repeats an `entered` or `left` the real pass reported.
    fn hover(&self) -> Hover {
        if !self.stated || self.face_mode {
            return Hover::default();
        }
        self.memory.recall::<Hover>(crate::contract::steps::ShownState::hover_slot(self.id))
    }

    fn part(&self, part: impl Into<PartRef>) -> &PartResult {
        let part = part.into();
        match self.results.get(&(part.key, part.index)) {
            Some(result) => result,
            None => {
                // A run that stopped before step 7 has no parts: its core is blank, so every part reads idle.
                debug_assert!(
                    self.face_mode || !self.parted || self.is_measuring() || self.results.is_empty() && self.stated,
                    "`parts` did not declare the part {part:?}"
                );
                &PartResult::IDLE
            }
        }
    }

    fn part_if_declared(&self, part: impl Into<PartRef>) -> Option<&PartResult> {
        let part = part.into();
        self.results.get(&(part.key, part.index))
    }

    fn hover_pos(&self, part: impl Into<PartRef>) -> Option<Pos2> {
        let result = self.part_if_declared(part)?;
        result.hover_pos
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
        if self.run.wheel_taken.replace(true) {
            return None;
        }
        // The steps queued since the last run, folded into one share.
        let mut steps = self.run.input.iter().filter_map(|input| match input {
            NodeInput::Wheel(wheel) => Some(*wheel),
            _ => None,
        });
        let mut share = steps.next()?;
        for step in steps {
            share.scroll += step.scroll;
            share.lines += step.lines;
            share.touch_pan += step.touch_pan;
            share.zoom *= step.zoom;
            share.touch_zoom *= step.touch_zoom;
            share.rotation += step.rotation;
            share.touch_rotation += step.touch_rotation;
            (share.phase, share.at, share.mods, share.gathered) = (step.phase, step.at, step.mods, step.gathered);
        }
        Some(share)
    }

    fn take_multi_tap(&mut self) -> Option<MultiTap> {
        let _ = self;
        todo!("multi-finger taps: ui-rt's touch tracker queues a NodeInput on the node under the fingers, INT-1c")
    }

    fn ease_wheel(&mut self, pan: Vec2) -> Vec2 {
        let slot = crate::runtime::memory::sub(self.id, "wheel_ease");
        let mut ease = self.memory.recall::<Option<craft_behavior::WheelEase>>(slot);
        if pan == Vec2::ZERO && ease.is_none() {
            return Vec2::ZERO;
        }
        let now = self.context().time();
        let running = ease.get_or_insert_default();
        running.feed(pan, now);
        let step = running.step(now);
        // While motion is owed the node runs in the next frame to pay it out; the ease is kept with the node.
        self.run.wake.set(running.owes());
        if !self.is_measuring() {
            *self.memory.shown::<Option<craft_behavior::WheelEase>>(slot) = ease;
        }
        step
    }

    fn stop_wheel_ease(&mut self) {
        let slot = crate::runtime::memory::sub(self.id, "wheel_ease");
        if !self.is_measuring() {
            *self.memory.shown::<Option<craft_behavior::WheelEase>>(slot) = None;
        }
        self.run.wake.set(false);
    }

    fn cursor(&mut self, cursor: impl Into<Cursor>, priority: CursorPriority) {
        if self.cursor.as_ref().is_none_or(|(_, held)| priority > *held) {
            self.cursor = Some((cursor.into(), priority));
        }
    }

    fn cancel_press(&mut self, reason: PressCancel) {
        // The driver ends the node's press with ui-rt's press machine after the run.
        self.press_cancel = Some(reason);
    }

    fn start_drag<P: Clone + Send + Sync + 'static>(&mut self, payload: P, preview: DragPreview) {
        // The driver starts ui-rt's payload session from the node's press after the run.
        *self.run.drag_start.borrow_mut() = Some(DragSource::new(payload, preview));
    }

    fn start_drag_also<P: Clone + Send + Sync + 'static>(
        &mut self,
        payload: P,
        also: Vec<Arc<dyn std::any::Any + Send + Sync>>,
        preview: DragPreview,
    ) {
        let _ = (payload, also, preview);
        todo!("drags that carry extra payloads: ui-rt's DragStart takes one payload; the session needs the `also` list beside it, INT-1c")
    }

    fn dragging<P: Send + Sync + 'static>(&self) -> Option<(Arc<P>, Pos2)> {
        let (payload, at) = self.dragging_of(TypeId::of::<P>())?;
        Some((payload.downcast::<P>().ok()?, at))
    }

    fn drop_target<P: Send + Sync + 'static, T: DropTarget<P>>(&mut self, target: &T) -> Option<T::Drop> {
        self.drop_target_of(TypeId::of::<P>(), &Typed::new(target))
    }

    fn window_drop_target<P: Send + Sync + 'static, T: WindowDropTarget<P>>(&mut self, target: &T) -> Option<T::Drop> {
        if target.requires_pointer() && self.dragging::<P>().is_none() {
            return None;
        }
        todo!("window-wide drop targets: a drop rule on the window root's node in ui-rt's routing (Component::drop_target with window reach), INT-1c")
    }

    fn drag_ended<P: Send + Sync + 'static>(&self) -> Option<DropOperation> {
        // The end of the node's own drag reaches its next run; the node starts drags of one payload, so the type is
        // the one it started with.
        self.run.input.iter().rev().find_map(|input| match input {
            NodeInput::DragEnd(craft_ui_rt::DragOutcome::Dropped { op, .. }) => Some(*op),
            _ => None,
        })
    }

    fn scroll_moved(&self, key: PartKey) -> Option<Range<f64>> {
        let moves = self.scroll_moves.iter().find(|moved| moved.key == key)?;
        moves.pressed.clone().or_else(|| moves.clicked.clone().filter(|_| self.intents.access_click(key.into())))
    }
}

impl CxCore<'_> {
    /// The active drag payload whose type is `payload` and the pointer in window coordinates.
    pub(crate) fn dragging_of(&self, payload: TypeId) -> Option<(Arc<dyn Any + Send + Sync>, Pos2)> {
        let _ = payload;
        todo!("the running drag session's payload and pointer as a tracked read the driver hands the run (BuildCx drag session), INT-1c")
    }

    /// `Pointer::drop_target` for the payload whose type is `payload_type`, chosen at run time: a collection takes the
    /// payload types its drop kinds name. The target lies over this component's visible shapes on its own layer.
    pub(crate) fn drop_target_of<T: DropTarget<dyn Any + Send + Sync>>(&mut self, payload_type: TypeId, target: &T) -> Option<T::Drop> {
        let _ = (payload_type, target);
        todo!(
            "drop targets chosen in a run: the node's DropRule (Component::drop_target) takes the payload in ui-rt's routing and its run reads NodeInput::Drop, INT-1c"
        )
    }
}

impl<C: Component> Pointer for Cx<'_, C> {
    fn hover(&self) -> Hover {
        self.core.hover()
    }

    fn part(&self, part: impl Into<PartRef>) -> &PartResult {
        self.core.part(part)
    }

    fn part_if_declared(&self, part: impl Into<PartRef>) -> Option<&PartResult> {
        self.core.part_if_declared(part)
    }

    fn hover_pos(&self, part: impl Into<PartRef>) -> Option<Pos2> {
        self.core.hover_pos(part)
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

    fn take_multi_tap(&mut self) -> Option<MultiTap> {
        self.core.take_multi_tap()
    }

    fn ease_wheel(&mut self, pan: Vec2) -> Vec2 {
        self.core.ease_wheel(pan)
    }

    fn stop_wheel_ease(&mut self) {
        self.core.stop_wheel_ease();
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

    fn start_drag_also<P: Clone + Send + Sync + 'static>(
        &mut self,
        payload: P,
        also: Vec<Arc<dyn std::any::Any + Send + Sync>>,
        preview: DragPreview,
    ) {
        self.core.start_drag_also(payload, also, preview);
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

impl<C: Component> Cx<'_, C> {
    /// The value a primary-button sweep sets on this toggle in this run, `None` when no sweep reached it. `on` is the
    /// value the toggle shows. Only toggles that return their value from `Component::sweep_value` and share a layout
    /// take part; the press owner's run gets the value first, each toggle the sweep crosses gets it once. A control
    /// that uses this senses both clicks and drags.
    pub fn swept_toggle(&self, on: Option<bool>) -> Option<bool> {
        self.core.run.swept.filter(|&swept| on != Some(swept))
    }
}

/// The cursor requests of the hovered parts' declared cursors, in registration order.
pub(crate) fn declared_cursor(core: &CxCore<'_>) -> Option<Cursor> {
    core.part_cursors.first().cloned()
}

/// The cursor the run asked for (`Pointer::cursor`), else the first hovered part's declared one: the node keeps it
/// (`Kept.cursor`), so the pointer shows it without a run.
pub(crate) fn run_cursor(core: &CxCore<'_>) -> Option<Cursor> {
    core.cursor.clone().map(|(cursor, _)| cursor).or_else(|| declared_cursor(core))
}
