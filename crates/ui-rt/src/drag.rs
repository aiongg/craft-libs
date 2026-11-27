//! Drag sessions: a payload carried from a press to a drop target, in the same window or another
//! one, and the files of an OS drag. One session runs at a time for the whole app.
//!
//! A press owner starts a session with [`crate::EventCx::start_drag`]. From then on every pointer
//! sample of that press finds the window under the pointer (from the windows' desktop positions
//! and their stacking order, looking through a window the drag moves along), hit-tests that
//! window's drop shapes, and offers the payload to the shape's node and its ancestors until one
//! accepts. The ghost, when the source gave one, sits on the drag layer of the window under the
//! pointer and moves without repainting. A release drops on the accepting node; Escape, a lost
//! window focus or the source going away cancels. The source hears how it ended.
//!
//! OS file drags ([`crate::FileDrag`]) run as sessions with no source and an [`OsFiles`]
//! payload, so a target takes files the way it takes any payload.

use std::any::Any;
use std::rc::Rc;

use craft_behavior::keys::drop_operation;
use craft_behavior::{CopyKeys, CursorIcon, DropOperation, PressCancel};
use craft_keymap::chord::Mods;
use emath::{Pos2, Vec2};

use crate::build::BuildCx;
use crate::element::{Element, ElementBox};
use crate::event::{Event, Handled};
use crate::geom::Constraints;
use crate::hit::{Hit, HitKind, contains, hit_records};
use crate::id::{ChildKey, NodeId, WindowId};
use crate::layer::{Layer, LayerSpec, Placement};
use crate::layout::LayoutCx;
use crate::paint::PaintCx;
use crate::platform::{Cursor, FileDrag, OsFile, OsFiles};
use crate::runtime::Runtime;
use crate::tree::Node;

/// A drag's payload, of any type. Targets look for the types they take with [`DragData::get`].
#[derive(Clone)]
pub struct DragData(Rc<dyn Any>);

impl DragData {
    pub fn new<P: 'static>(payload: P) -> DragData {
        DragData(Rc::new(payload))
    }

    pub fn get<P: 'static>(&self) -> Option<&P> {
        self.0.downcast_ref::<P>()
    }

    pub fn is<P: 'static>(&self) -> bool {
        self.0.is::<P>()
    }

    /// The files of an OS drag.
    pub fn files(&self) -> Option<&OsFiles> {
        self.get::<OsFiles>()
    }
}

impl PartialEq for DragData {
    fn eq(&self, other: &DragData) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl std::fmt::Debug for DragData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DragData")
    }
}

/// What shows beside the pointer during a drag: `size` logical points painted by `paint` in its
/// own coordinates, its top left at the pointer plus `offset`. It records no hit shapes, so it
/// never hides a target.
#[derive(Clone)]
pub struct DragGhost {
    pub size: Vec2,
    pub offset: Vec2,
    pub paint: Rc<dyn Fn(&mut PaintCx<'_>)>,
}

impl DragGhost {
    pub fn new(size: Vec2, offset: Vec2, paint: impl Fn(&mut PaintCx<'_>) + 'static) -> DragGhost {
        DragGhost { size, offset, paint: Rc::new(paint) }
    }
}

/// What [`crate::EventCx::start_drag`] starts.
pub struct DragStart {
    pub payload: DragData,
    pub ghost: Option<DragGhost>,
    /// A window that moves along with the pointer (a floating panel window being dragged): the
    /// pointer is never "over" it, so the targets under it take the drop.
    pub moving: Option<WindowId>,
}

impl DragStart {
    pub fn new<P: 'static>(payload: P) -> DragStart {
        DragStart { payload: DragData::new(payload), ghost: None, moving: None }
    }

    #[must_use]
    pub fn ghost(mut self, ghost: DragGhost) -> DragStart {
        self.ghost = Some(ghost);
        self
    }

    #[must_use]
    pub fn moving(mut self, window: WindowId) -> DragStart {
        self.moving = Some(window);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropPhase {
    /// The drag is over a drop shape of this node or of a descendant that did not accept.
    Over,
    /// The drag left the node, another node accepted it, or it was cancelled.
    Leave,
    Drop,
}

/// A drag offered to, leaving or dropped on a target.
#[derive(Clone, Debug, PartialEq)]
pub struct DropInput {
    pub phase: DropPhase,
    pub data: DragData,
    /// The pointer in the target's window coordinates; `None` for an OS drop that came with no
    /// position, offered to window targets ([`crate::PaintCx::window_drop_target`]).
    pub pos: Option<Pos2>,
    /// The keys held, which pick the drop operation ([`crate::EventCx::drop_operation`]).
    pub mods: Mods,
    /// The drop shape under the pointer; `None` for a window target.
    pub hit: Option<Hit>,
}

/// How a drag ended, as its source hears it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DragOutcome {
    Dropped {
        target: NodeId,
        window: WindowId,
        op: DropOperation,
    },
    /// Released where no target took it.
    NotDropped,
    Cancelled(PressCancel),
}

/// The running drag as build and paint read it from [`crate::Env::drag`]. It changes when the
/// drag starts, moves into another window, changes target or operation, and ends; never on a move
/// alone.
#[derive(Clone, Debug, PartialEq)]
pub struct DragInfo {
    pub data: DragData,
    /// `None` for OS files.
    pub source: Option<NodeId>,
    /// The window under the pointer.
    pub window: Option<WindowId>,
    pub target: Option<NodeId>,
    pub op: Option<DropOperation>,
}

/// Where the running drag's pointer is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DragPointer {
    /// The window under the pointer and the pointer in its coordinates.
    pub over: Option<(WindowId, Pos2)>,
    /// On the virtual desktop in physical pixels, when the platform told where the drag's window
    /// is.
    pub desktop: Option<Pos2>,
}

pub(crate) struct Session {
    data: DragData,
    source: Option<NodeId>,
    /// The window whose press drives the drag, or the window the OS drags files over.
    pub(crate) origin: WindowId,
    /// The driving press's pointer; `None` for OS files.
    pub(crate) pointer: Option<u64>,
    ghost: Option<DragGhost>,
    moving: Option<WindowId>,
    mods: Mods,
    /// The pointer's last position in `origin`.
    last: Option<Pos2>,
    over: Option<(WindowId, Pos2)>,
    target: Option<(NodeId, DropOperation)>,
    /// What the node handling an offer accepted with.
    accepted: Option<(NodeId, DropOperation)>,
    ghost_in: Option<WindowId>,
    /// An OS drop came with no position and no window target took it: it drops at the next
    /// pointer position in its window.
    waiting_drop: bool,
    /// Started inside the source's handler: the first offer runs once the handler returned.
    pending: bool,
    /// The source node went away: the session ends at the next safe point, unheard.
    orphaned: bool,
}

/// The ghost's node among a drag layer's children.
const GHOST_KEY: u32 = 0x6057;

struct GhostElement(DragGhost);

impl Element for GhostElement {
    type Output = ();
    type Local = ();

    fn props_key(&self) -> u64 {
        0
    }

    fn init(&self, _: &mut BuildCx<'_>) {}

    fn layout(&self, _: &mut (), _: &mut LayoutCx<'_>, c: Constraints) -> Vec2 {
        c.constrain(self.0.size)
    }

    fn paint(&self, _: &(), cx: &mut PaintCx<'_>) {
        (self.0.paint)(cx);
    }
}

impl Runtime {
    pub(crate) fn start_drag(&mut self, source: NodeId, start: DragStart) -> bool {
        if self.drag.is_some() {
            return false;
        }
        let Some(window) = self.tree.window_of(source) else { return false };
        let Some(press) = self.windows.get(&window).and_then(|w| w.input.press_state()) else {
            return false;
        };
        self.drag = Some(Session {
            data: start.payload,
            source: Some(source),
            origin: window,
            pointer: Some(press.0),
            ghost: start.ghost,
            moving: start.moving,
            mods: press.2,
            last: Some(press.1),
            over: None,
            target: None,
            accepted: None,
            ghost_in: None,
            waiting_drop: false,
            pending: true,
            orphaned: false,
        });
        self.publish_drag();
        true
    }

    /// The running drag, without recording a read.
    pub fn drag(&self) -> Option<DragInfo> {
        self.drag_info()
    }

    pub(crate) fn drag_info(&self) -> Option<DragInfo> {
        self.drag.as_ref().map(|s| DragInfo {
            data: s.data.clone(),
            source: s.source,
            window: s.over.map(|o| o.0),
            target: s.target.map(|t| t.0),
            op: s.target.map(|t| t.1),
        })
    }

    pub(crate) fn drag_pointer(&self) -> Option<DragPointer> {
        let s = self.drag.as_ref()?;
        Some(DragPointer { over: s.over, desktop: s.last.and_then(|p| self.to_desktop(s.origin, p)) })
    }

    pub(crate) fn accept_drop(&mut self, node: NodeId, op: DropOperation) {
        if let Some(s) = self.drag.as_mut() {
            s.accepted = Some((node, op));
        }
    }

    pub(crate) fn drop_operation(&self, copy: CopyKeys) -> DropOperation {
        self.drag.as_ref().map_or(DropOperation::Move, |s| drop_operation(self.os, s.mods, copy))
    }

    /// Whether the running drag follows `pointer`'s press in `window`.
    pub(crate) fn drag_follows(&self, window: WindowId, pointer: u64) -> bool {
        self.drag.as_ref().is_some_and(|s| s.origin == window && s.pointer == Some(pointer))
    }

    /// Whether the running drag is an OS file drag over `window`.
    pub(crate) fn file_drag_in(&self, window: WindowId) -> bool {
        self.drag.as_ref().is_some_and(|s| s.origin == window && s.pointer.is_none())
    }

    /// The pointer of the running drag's press moved to `pos` (`origin` coordinates) with `mods`.
    pub(crate) fn drag_moved(&mut self, pos: Pos2, mods: Mods) {
        let Some(s) = self.drag.as_mut() else { return };
        s.last = Some(pos);
        s.mods = mods;
        if s.pending {
            return;
        }
        let origin = s.origin;
        self.update_drag(origin, pos);
        if self.drag.as_ref().is_some_and(|s| s.waiting_drop) {
            self.finish_drag();
        }
    }

    /// The held keys changed: the offer runs again, so the operation follows them.
    pub(crate) fn drag_mods(&mut self, mods: Mods) {
        let Some(s) = self.drag.as_mut().filter(|s| s.mods != mods) else { return };
        s.mods = mods;
        if let (false, Some(last)) = (s.pending, s.last) {
            let origin = s.origin;
            self.update_drag(origin, last);
        }
    }

    /// Runs the offer a session started inside a handler waits for, and ends an orphaned one.
    /// Called after each routed event and each frame.
    pub(crate) fn settle_drag(&mut self) {
        let Some(s) = self.drag.as_mut() else { return };
        if s.orphaned {
            self.end_drag(None);
            return;
        }
        if s.pending {
            s.pending = false;
            if let Some(last) = s.last {
                let origin = s.origin;
                self.update_drag(origin, last);
            }
        }
    }

    /// The pointer is at `pos` in `window`: finds the window under it, offers the payload there,
    /// tells a target that lost the drag, and moves the ghost.
    fn update_drag(&mut self, window: WindowId, pos: Pos2) {
        let Some(s) = self.drag.as_ref() else { return };
        let over = self.locate(window, pos, s.moving);
        let target = over.and_then(|(w, p)| self.offer(w, Some(p)));
        let Some(s) = self.drag.as_mut() else { return };
        s.over = over;
        let old = std::mem::replace(&mut s.target, target);
        let data = s.data.clone();
        let mods = s.mods;
        if let Some((node, _)) = old.filter(|o| Some(o.0) != target.map(|t| t.0)) {
            self.deliver(node, node, &Event::Drop(DropInput { phase: DropPhase::Leave, data, pos: None, mods, hit: None }));
        }
        self.place_ghost(over);
        self.publish_drag();
    }

    /// Offers the payload at `pos` in `window`: to the drop shape's node and its ancestors, then
    /// to the window targets. Returns the node that accepted and its operation.
    fn offer(&mut self, window: WindowId, pos: Option<Pos2>) -> Option<(NodeId, DropOperation)> {
        let s = self.drag.as_mut()?;
        s.accepted = None;
        let (data, mods) = (s.data.clone(), s.mods);
        let w = self.windows.get(&window)?;
        let hit = pos.and_then(|p| hit_records(&self.tree, w, p, &|h| h.kind == HitKind::Drop)).map(|(hit, _)| hit);
        if let Some(hit) = hit {
            let ev = Event::Drop(DropInput { phase: DropPhase::Over, data: data.clone(), pos, mods, hit: Some(hit) });
            if let Some(node) = self.dispatch(hit.node, &ev, true) {
                return Some((node, self.accepted_op(node)));
            }
        }
        let window_targets: Vec<NodeId> = self.windows.get(&window).map(|w| w.window_drops.iter().rev().copied().collect()).unwrap_or_default();
        for node in window_targets {
            let ev = Event::Drop(DropInput { phase: DropPhase::Over, data: data.clone(), pos, mods, hit: None });
            if self.deliver(node, node, &ev) == Handled::Yes {
                return Some((node, self.accepted_op(node)));
            }
        }
        None
    }

    fn accepted_op(&self, node: NodeId) -> DropOperation {
        self.drag.as_ref().and_then(|s| s.accepted).filter(|a| a.0 == node).map_or(DropOperation::Move, |a| a.1)
    }

    /// Ends the session at its last position: the target gets the drop, the source the outcome.
    pub(crate) fn finish_drag(&mut self) {
        let Some(s) = self.drag.as_ref() else { return };
        let outcome = match s.target.and_then(|(node, op)| Some((node, op, self.tree.window_of(node)?))) {
            Some((node, op, window)) => {
                let pos = s.over.filter(|o| o.0 == window).map(|o| o.1);
                let ev = Event::Drop(DropInput { phase: DropPhase::Drop, data: s.data.clone(), pos, mods: s.mods, hit: None });
                match self.deliver(node, node, &ev) {
                    Handled::Yes => DragOutcome::Dropped { target: node, window, op },
                    Handled::No => DragOutcome::NotDropped,
                }
            }
            None => DragOutcome::NotDropped,
        };
        if let Some(s) = self.drag.as_mut() {
            s.target = None;
        }
        self.end_drag(Some(outcome));
    }

    /// Cancels the running drag: its target hears `Leave`, its source the reason.
    pub(crate) fn cancel_drag(&mut self, reason: PressCancel) {
        self.end_drag(Some(DragOutcome::Cancelled(reason)));
    }

    fn end_drag(&mut self, outcome: Option<DragOutcome>) {
        self.place_ghost(None);
        let Some(s) = self.drag.take() else { return };
        if let Some((node, _)) = s.target {
            let ev = Event::Drop(DropInput { phase: DropPhase::Leave, data: s.data.clone(), pos: None, mods: s.mods, hit: None });
            self.deliver(node, node, &ev);
        }
        if let (Some(source), Some(outcome)) = (s.source, outcome)
            && !s.orphaned
        {
            self.deliver(source, source, &Event::DragEnd(outcome));
        }
        self.publish_drag();
    }

    /// A node went away: a session it started ends unheard, a target it was is dropped.
    pub(crate) fn forget_drag_node(&mut self, id: NodeId) {
        let Some(s) = self.drag.as_mut() else { return };
        if s.source == Some(id) {
            s.orphaned = true;
        }
        if s.target.is_some_and(|t| t.0 == id) {
            s.target = None;
        }
    }

    /// Routes an OS file drag over `window`.
    pub(crate) fn file_drag(&mut self, window: WindowId, ev: &FileDrag) {
        if self.drag.as_ref().is_some_and(|s| s.pointer.is_some()) {
            return;
        }
        match ev {
            FileDrag::Enter { files, pos } => {
                self.end_drag(None);
                self.begin_files(window, files.clone(), *pos);
            }
            FileDrag::Over(pos) => {
                if self.file_drag_in(window) {
                    let mods = self.drag.as_ref().map_or(Mods::NONE, |s| s.mods);
                    self.drag_moved(*pos, mods);
                }
            }
            FileDrag::Leave => {
                if self.file_drag_in(window) {
                    self.end_drag(None);
                }
            }
            FileDrag::Drop { files, pos } => {
                if !self.file_drag_in(window) {
                    self.end_drag(None);
                    self.begin_files(window, Vec::new(), *pos);
                }
                let Some(s) = self.drag.as_mut() else { return };
                s.data = DragData::new(OsFiles { files: files.clone() });
                if let Some(p) = *pos {
                    s.last = Some(p);
                }
                let last = s.last;
                match last {
                    Some(last) => {
                        self.update_drag(window, last);
                        self.finish_drag();
                    }
                    None => {
                        let target = self.offer(window, None);
                        let Some(s) = self.drag.as_mut() else { return };
                        match target {
                            Some(t) => {
                                s.target = Some(t);
                                self.finish_drag();
                            }
                            None => s.waiting_drop = true,
                        }
                    }
                }
            }
        }
    }

    fn begin_files(&mut self, window: WindowId, files: Vec<OsFile>, pos: Option<Pos2>) {
        let last = pos.or_else(|| self.windows.get(&window).and_then(|w| w.input.hover_pos()));
        self.drag = Some(Session {
            data: DragData::new(OsFiles { files }),
            source: None,
            origin: window,
            pointer: None,
            ghost: None,
            moving: None,
            mods: Mods::NONE,
            last,
            over: None,
            target: None,
            accepted: None,
            ghost_in: None,
            waiting_drop: false,
            pending: false,
            orphaned: false,
        });
        match last {
            Some(p) => self.update_drag(window, p),
            None => self.publish_drag(),
        }
    }

    /// Shows the ghost on the drag layer of the window under the pointer, at the pointer.
    fn place_ghost(&mut self, at: Option<(WindowId, Pos2)>) {
        let Some(s) = self.drag.as_mut() else { return };
        let Some(ghost) = s.ghost.clone() else { return };
        let old = std::mem::replace(&mut s.ghost_in, at.map(|a| a.0));
        if let Some(old) = old.filter(|o| Some(*o) != at.map(|a| a.0))
            && let Some(id) = self.ghost_id(old)
        {
            self.remove_subtree(id);
        }
        let Some((window, pos)) = at else { return };
        let Some(id) = self.ghost_id(window) else { return };
        let spec = LayerSpec::new(Placement::At(pos + ghost.offset));
        let Some(w) = self.windows.get_mut(&window) else { return };
        let layer = w.layer(Layer::Drag);
        if w.layered.insert(id, spec) != Some(spec) {
            w.relayout = true;
        }
        if let std::collections::hash_map::Entry::Vacant(slot) = self.tree.nodes.entry(id) {
            let mut node = Node::new(Some(layer), None, window, 1, Some(Box::new(ElementBox::new(GhostElement(ghost)))), 0);
            node.boundary = true;
            slot.insert(node);
            if let Some(l) = self.tree.nodes.get_mut(&layer) {
                l.children.push(id);
            }
            self.tree.mark_build(id);
        }
    }

    fn ghost_id(&self, window: WindowId) -> Option<NodeId> {
        self.windows.get(&window).map(|w| w.layer(Layer::Drag).child(ChildKey::Fixed(GHOST_KEY)))
    }

    /// Writes the drag's reader view and has every window pick its cursor again.
    fn publish_drag(&mut self) {
        let info = self.drag_info();
        let cell = self.drag_cell();
        cell.set_if_changed(&mut self.store, info);
        for w in self.windows.values_mut() {
            w.input.cursor_check = true;
        }
    }

    /// The cursor the running drag shows in `window`, when the pointer is over it.
    pub(crate) fn drag_cursor(&self, window: WindowId) -> Option<Cursor> {
        let s = self.drag.as_ref()?;
        if s.over.map(|o| o.0) != Some(window) || s.pending {
            return None;
        }
        let icon = match s.target.map(|t| t.1) {
            None => CursorIcon::NotAllowed,
            Some(DropOperation::Copy) => CursorIcon::Copy,
            Some(DropOperation::Link) => CursorIcon::Alias,
            Some(_) => CursorIcon::Grabbing,
        };
        Some(Cursor::Icon(icon))
    }

    /// `pos` in `window` on the virtual desktop, in physical pixels.
    pub(crate) fn to_desktop(&self, window: WindowId, pos: Pos2) -> Option<Pos2> {
        let w = self.windows.get(&window)?;
        let origin = w.position?;
        Some(origin + pos.to_vec2() * *self.store.peek(w.env.scale))
    }

    pub(crate) fn desktop_to_window(&self, window: WindowId, desktop: Pos2) -> Option<Pos2> {
        let w = self.windows.get(&window)?;
        let origin = w.position?;
        Some(((desktop - origin) / *self.store.peek(w.env.scale)).to_pos2())
    }

    /// The topmost window, other than `exclude`, under `pos` of window `from`, and the point in
    /// its coordinates. Windows whose desktop position is unknown are found only as `from`.
    pub fn window_at(&self, from: WindowId, pos: Pos2, exclude: Option<WindowId>) -> Option<(WindowId, Pos2)> {
        self.locate(from, pos, exclude)
    }

    fn locate(&self, from: WindowId, pos: Pos2, exclude: Option<WindowId>) -> Option<(WindowId, Pos2)> {
        let desktop = self.to_desktop(from, pos);
        for &w in self.z_order.iter().rev() {
            if Some(w) == exclude {
                continue;
            }
            let Some(win) = self.windows.get(&w) else { continue };
            let rect = emath::Rect::from_min_size(Pos2::ZERO, win.viewport);
            let local = if w == from { Some(pos) } else { desktop.and_then(|d| self.desktop_to_window(w, d)) };
            if let Some(local) = local.filter(|p| contains(rect, *p)) {
                return Some((w, local));
            }
        }
        None
    }
}
