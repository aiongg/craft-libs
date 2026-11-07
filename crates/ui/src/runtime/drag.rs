//! The drag session (8.4): one per context. It holds the payload, the preview, the held modifiers, the operation and the
//! mark of the drop target that would take the payload, as data. The drop-target registrations of a pass resolve in the
//! next pass, like the press arena's shapes. The ghost layer paints from this data (`contract::ghost`).

use crate::runtime::egui::areas::{self, Layer};
use crate::runtime::egui::{memory::Service, pass};
use crate::runtime::env::Label;
use crate::runtime::frame::PerViewport;
use crate::runtime::windows::{self, WindowHost};
use crate::runtime::{cursor, frame, pointer};
use craft_behavior::keys::drop_operation;
use craft_behavior::{Buttons, CancelReason, Cursor, CursorIcon, CursorPriority, DropOperation, PointerButton, PressCancel};
use craft_keymap::Mods;
use egui::{Context, Id, Pos2, Rect, ViewportId};
use std::any::{Any, TypeId};
use std::sync::Arc;

/// What the ghost layer paints beside the pointer during a drag.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub enum DragPreview {
    /// Nothing.
    #[default]
    None,
    /// A badge with a short text ("3 files").
    Badge(Label),
}

/// Where a drop would land, as geometry in the target's egui layer space.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum DropMark {
    /// An insertion line.
    #[non_exhaustive]
    Line {
        /// One end.
        from: Pos2,
        /// The other end.
        to: Pos2,
    },
    /// The payload goes into this rect.
    Into(Rect),
    /// This rect is split, the payload taking one side.
    Split(Rect),
}

impl DropMark {
    /// An insertion line from `from` to `to`.
    pub const fn line(from: Pos2, to: Pos2) -> Self {
        DropMark::Line { from, to }
    }

    /// The mark with every point passed through `map`.
    pub(crate) fn mapped(self, map: impl Fn(Pos2) -> Pos2) -> Self {
        let rect = |rect: Rect| Rect::from_two_pos(map(rect.min), map(rect.max));
        match self {
            DropMark::Line { from, to } => DropMark::Line { from: map(from), to: map(to) },
            DropMark::Into(target) => DropMark::Into(rect(target)),
            DropMark::Split(target) => DropMark::Split(rect(target)),
        }
    }
}

/// What a component registers with `Pointer::drop_target`.
pub trait DropTarget<P> {
    /// What a drop on this target carries to the component's caller.
    type Drop;

    /// `None` rejects. `at` is the pointer in the component's egui layer space, so a target picks its sub-area (a port,
    /// a split side) from it; `op` is the operation the held modifiers pick, which a target may refuse.
    fn accept(&self, payload: &P, at: Pos2, op: DropOperation) -> Option<Self::Drop>;

    /// Where `drop` would land, a value the ghost layer paints.
    fn mark(&self, drop: &Self::Drop) -> DropMark;
}

struct Session {
    source: Id,
    /// The viewport whose pass started the session and the button whose press it follows: only that viewport's end
    /// hook reads the button, so another window repainting mid-drag does not end the session. Its release ends the
    /// session, and its window loss cancels it.
    viewport: ViewportId,
    button: PointerButton,
    payload: Arc<dyn Any + Send + Sync>,
    preview: DragPreview,
    /// The mark of the target that would take a drop now, in screen space of the viewport that registered it.
    mark: Option<(ViewportId, DropMark)>,
    /// The root frame in which the source viewport saw the release while a target in another window held the
    /// drop: the session lives until that window's pass takes it.
    released: Option<u64>,
    /// A target took the drop.
    taken: bool,
}

/// A drop a target took: the source's `drag_ended` reads it in the next frame.
struct Ended {
    source: Id,
    payload: TypeId,
    op: DropOperation,
    frame: u64,
}

#[derive(Default)]
struct Board {
    session: Option<Session>,
    ended: Option<Ended>,
    /// The sessions started on this context, for tests.
    started: u64,
}

/// A target that accepted in one pass.
struct Registration {
    owner: Id,
    ordinal: u32,
    layer: Layer,
    mark: DropMark,
}

/// A viewport's registrations: this pass's, and the target that won last pass.
#[derive(Default)]
struct Targets {
    this: Vec<Registration>,
    winner: Option<(Id, u32)>,
}

fn board<R>(ctx: &Context, f: impl FnOnce(&mut Board) -> R) -> R {
    Service::<Board>::of(ctx).with(f)
}

/// Starts a session for `source` with `payload`. A running session stays: one per context.
pub(crate) fn start(ctx: &Context, source: Id, payload: Arc<dyn Any + Send + Sync>, preview: DragPreview) {
    board(ctx, |board| {
        if board.session.is_none() {
            let buttons = pointer::frame(ctx).buttons;
            let button = [PointerButton::Primary, PointerButton::Secondary, PointerButton::Middle, PointerButton::Extra1, PointerButton::Extra2]
                .into_iter()
                .find(|button| buttons.contains(Buttons::of(*button)))
                .unwrap_or(PointerButton::Primary);
            board.session =
                Some(Session { source, viewport: pass::current_viewport(ctx), button, payload, preview, mark: None, released: None, taken: false });
            board.ended = None;
            board.started += 1;
        }
    });
    crate::runtime::egui::repaint::request(ctx);
}

fn operation(ctx: &Context, mods: Mods) -> DropOperation {
    drop_operation(windows::os_of(ctx), mods)
}

/// The active drag payload of type `P`, from any viewport. For code without a `Cx`.
pub fn active<P: Send + Sync + 'static>(ctx: &Context) -> Option<Arc<P>> {
    board(ctx, |board| board.session.as_ref().map(|session| session.payload.clone()))?.downcast::<P>().ok()
}

/// How many sessions have started on this context.
#[doc(hidden)]
pub fn started(ctx: &Context) -> u64 {
    board(ctx, |board| board.started)
}

/// The mark of the target that would take a drop now.
#[doc(hidden)]
pub fn mark(ctx: &Context) -> Option<DropMark> {
    board(ctx, |board| board.session.as_ref().and_then(|session| session.mark.map(|(_, mark)| mark)))
}

/// Whether a session runs.
pub(crate) fn running(ctx: &Context) -> bool {
    board(ctx, |board| board.session.is_some())
}

/// Ends the session without a drop; the source's press ends with it, so nobody gets a drop.
pub fn cancel(ctx: &Context, reason: CancelReason) {
    if board(ctx, |board| board.session.take()).is_none() {
        return;
    }
    let press = match reason {
        CancelReason::Escape => PressCancel::Escape,
        CancelReason::Removed => PressCancel::Removed,
        _ => PressCancel::FocusLost,
    };
    pointer::cancel_press(ctx, press);
    frame::note_cancelled_press(ctx);
}

/// The begin hook: the window losing focus ends the session.
pub(crate) fn begin(ctx: &Context) {
    if source_viewport(ctx) == Some(pass::current_viewport(ctx)) && pointer::pass_pointer(ctx).focus_lost {
        cancel(ctx, CancelReason::Blur);
    }
}

/// The viewport the running session started in.
fn source_viewport(ctx: &Context) -> Option<ViewportId> {
    board(ctx, |board| board.session.as_ref().map(|session| session.viewport))
}

/// The pointer in `viewport` for a drag that started in another window: the desktop pointer mapped into this
/// window, when it lies inside. `None` without a session, in the source window, or outside this window.
pub(crate) fn cross_window_pointer(ctx: &Context, viewport: ViewportId) -> Option<Pos2> {
    let source = source_viewport(ctx)?;
    (source != viewport).then(|| WindowHost::of(ctx).pointer_in(viewport)).flatten()
}

/// The running session's payload of type `P` and the pointer in screen space.
pub(crate) fn dragging<P: Send + Sync + 'static>(ctx: &Context) -> Option<(Arc<P>, Pos2)> {
    let payload = active::<P>(ctx)?;
    Some((payload, pointer::frame(ctx).pos?))
}

/// Records that `owner`'s `ordinal`th target on `layer` accepts at the pointer, with its mark in layer space.
pub(crate) fn register(ctx: &Context, owner: Id, layer: Layer, mark: DropMark) {
    let mark = match areas::layer_map(ctx, layer) {
        Some(map) => mark.mapped(|point| map.to_screen(point)),
        None => mark,
    };
    PerViewport::<Targets>::with(ctx, pass::current_viewport(ctx), |targets| {
        let ordinal = targets.this.iter().filter(|registration| registration.owner == owner).count() as u32;
        targets.this.push(Registration { owner, ordinal, layer, mark });
    });
}

/// The ordinal the next registration of `owner` gets in this pass.
pub(crate) fn next_ordinal(ctx: &Context, owner: Id) -> u32 {
    PerViewport::<Targets>::with(ctx, pass::current_viewport(ctx), |targets| {
        targets.this.iter().filter(|registration| registration.owner == owner).count() as u32
    })
}

/// The operation the held modifiers pick now.
pub(crate) fn current_operation(ctx: &Context) -> DropOperation {
    operation(ctx, pointer::frame(ctx).mods)
}

/// Whether the primary button was released in this pass.
fn released(ctx: &Context) -> bool {
    let viewport = pass::current_viewport(ctx);
    pointer::frame(ctx).release == Some(PointerButton::Primary)
        || board(ctx, |board| board.session.as_ref().is_some_and(|session| session.released.is_some() && session.viewport != viewport))
}

/// Takes the drop for `owner`'s `ordinal`th target when it won last pass and the primary button was released in this
/// one: the drop ends the session's chance of another target taking it, and the source reads `op` in the next frame.
pub(crate) fn take_drop<P: 'static>(ctx: &Context, owner: Id, ordinal: u32, op: DropOperation) -> bool {
    let won = PerViewport::<Targets>::with(ctx, pass::current_viewport(ctx), |targets| targets.winner == Some((owner, ordinal)));
    if !won || !released(ctx) {
        return false;
    }
    let frame = pass::root_frame_nr(ctx);
    board(ctx, |board| {
        let Some(session) = board.session.as_mut() else { return false };
        if session.taken || board.ended.as_ref().is_some_and(|ended| ended.frame == frame) {
            return false;
        }
        session.taken = true;
        board.ended = Some(Ended { source: session.source, payload: TypeId::of::<P>(), op, frame });
        true
    })
}

/// The operation of the drop of `source`'s payload of type `P` that the previous frame took.
pub(crate) fn ended<P: 'static>(ctx: &Context, source: Id) -> Option<DropOperation> {
    let frame = pass::root_frame_nr(ctx);
    board(ctx, |board| {
        board
            .ended
            .as_ref()
            .filter(|ended| ended.source == source && ended.payload == TypeId::of::<P>() && ended.frame + 1 == frame)
            .map(|ended| ended.op)
    })
}

/// The end hook: resolves this pass's registrations for the next pass, records the mark, requests the cursor,
/// and ends the session once its button is up. The session ends in the source window; when a target in another
/// window holds the drop, it waits there for one more frame so that window's pass takes it.
pub(crate) fn end_pass(ctx: &Context) {
    let viewport = pass::current_viewport(ctx);
    let winner = PerViewport::<Targets>::with(ctx, viewport, |targets| {
        // The topmost layer in the order the overlay host handed egui wins, then the target registered last: a target
        // shown later lies over an enclosing one.
        let best = targets
            .this
            .iter()
            .enumerate()
            .max_by_key(|(at, registration)| (areas::layer_position(ctx, registration.layer), *at))
            .map(|(_, registration)| registration);
        let best = best.map(|registration| ((registration.owner, registration.ordinal), registration.mark));
        targets.winner = best.map(|(key, _)| key);
        targets.this.clear();
        best.map(|(_, mark)| mark)
    });
    let op = current_operation(ctx);
    let over_this_window = pointer::frame(ctx).pos.is_some();
    let primary_down = pointer::pass_pointer(ctx).primary_down;
    let buttons = pointer::frame(ctx).buttons;
    let held = board(ctx, |board| board.session.as_ref().map(|session| session.button))
        .is_none_or(|button| if button == PointerButton::Primary { primary_down } else { buttons.contains(Buttons::of(button)) });
    let frame = pass::root_frame_nr(ctx);
    let source = source_viewport(ctx);
    let is_source = source == Some(viewport);
    if is_source {
        windows::note_source_pointer(ctx, viewport, pointer::frame(ctx).pos);
    }
    let foreign_target = is_source && !held && foreign_winner(ctx, viewport);
    let running = board(ctx, |board| {
        if let Some(session) = &mut board.session {
            let expired = session.released.is_some_and(|released| frame > released + 1);
            let ended_here = is_source && !held && session.released.is_none();
            if expired || (ended_here && !foreign_target) {
                board.session = None;
            } else if ended_here {
                session.released = Some(frame);
            }
        }
        if let Some(session) = &mut board.session {
            match winner {
                Some(mark) => session.mark = Some((viewport, mark)),
                None if session.mark.is_some_and(|(marked, _)| marked == viewport) => session.mark = None,
                None => {}
            }
        }
        board.session.is_some()
    });
    if !running {
        windows::clear_source_pointer(ctx);
    }
    if running && is_source {
        // The windows the pointer may be over resolve their targets again.
        for other in pass::live_viewports(ctx).into_iter().filter(|other| *other != viewport) {
            crate::runtime::egui::repaint::request_of(ctx, other);
        }
    }
    if running && over_this_window {
        match winner {
            None => cursor::request(ctx, Cursor::System(CursorIcon::NotAllowed), CursorPriority::Drag),
            Some(_) => {
                let icon = match op {
                    DropOperation::Copy => CursorIcon::Copy,
                    DropOperation::Link => CursorIcon::Alias,
                    _ => CursorIcon::Grabbing,
                };
                cursor::request(ctx, Cursor::System(icon), CursorPriority::DropTarget);
            }
        }
    }
}

/// Whether a viewport other than `viewport` has a target that took the pointer in its last pass.
fn foreign_winner(ctx: &Context, viewport: ViewportId) -> bool {
    pass::live_viewports(ctx)
        .into_iter()
        .filter(|other| *other != viewport)
        .any(|other| PerViewport::<Targets>::with(ctx, other, |targets| targets.winner.is_some()))
}

/// What the ghost layer paints: the session's preview and the mark of the target that would take a drop.
pub(crate) struct Ghost {
    pub preview: DragPreview,
    pub mark: Option<DropMark>,
}

/// The running session's ghost data.
pub(crate) fn ghost(ctx: &Context) -> Option<Ghost> {
    let viewport = pass::current_viewport(ctx);
    board(ctx, |board| {
        board.session.as_ref().map(|session| Ghost {
            preview: session.preview.clone(),
            mark: session.mark.and_then(|(marked, mark)| (marked == viewport).then_some(mark)),
        })
    })
}
