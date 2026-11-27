//! The open state of a container's layer with no binding, kept in its opener's slot, and `Popup`. The slot is session
//! memory under the opener's id; whether it is open is also a cell on the opener's node (found through the window's
//! id map), so an opener that shows its expanded state (`opened_from`) runs again when its container opens or closes.

use super::opener::{Opener, Opening, Trigger, opener_anchor};
use super::place::{Anchor, Place};
use super::popups::{HOLD, PopupHandle, dwell_in};
use crate::contract::context::log_build_read;
use crate::contract::rt::ids;
use crate::contract::{Component, Context, Cx};
use crate::primitives::Id;
use crate::runtime::memory::{MemoryStage, sub};
use craft_ui_rt::{BuildCx, State};

/// Open state a container with no `open` binding keeps in its opener's slot.
#[derive(Clone, Copy, Default)]
struct OpenerOpen {
    open: bool,
    /// The frame time an opener's trigger was last applied at: a frame's build rounds apply it once.
    applied: Option<f64>,
    /// The opener that opened the layer by id and trigger, and how it fired.
    by: Option<(Id, Trigger, Opening)>,
}

/// The open state of the layer of a container with no binding hung from `first`, decided in a build of the
/// container's node `cx` (`stage` is its memory): the slot of its first opener, with the triggers of every opener
/// applied once per frame. A held `Trigger::Hold` opener opens it after `HOLD` seconds; the node builds again at that
/// deadline. A disabled opener that opened it closes it. A measuring run's stage writes nothing back.
///
/// While the layer is closed the node is a zero-size node in the owner's flow (`rt/container.rs` `show_on_layer`), so
/// the triggers and the hold timer run with no layer child. An opener that fired here opened the layer by that
/// activation: the caller claims it ([`Opener::claims_activation`], `dismissal::claim_activation`) so the layer chain
/// the opener sits in stays open.
pub(crate) fn opener_open(cx: &mut BuildCx<'_>, stage: &mut MemoryStage, first: &Opener) -> bool {
    let now = stage.ctx().time();
    let held = first.all().find(|opener| opener.trigger == Trigger::Hold).map(|opener| {
        // The dwell is timed under the opener's id in the container's node memory.
        let started = match dwell_in(stage, now, opener.id, "dwell", opener.pressed, HOLD) {
            Ok(started) => started,
            Err(due) => {
                cx.request_build_at(Some(due));
                false
            }
        };
        (opener, started)
    });
    let slot = stage.session::<OpenerOpen>(sub(first.id, "opens"));
    if let Some(opener) = first.all().find(|opener| opener.fired) {
        if slot.applied != Some(now) {
            // A context request opens, also over the open layer: a new right-click moves it to the pointer.
            slot.open = opener.trigger == Trigger::Context || !slot.open;
            slot.by = Some((opener.id, opener.trigger, opener.opening));
        }
        slot.applied = Some(now);
    } else if let Some((opener, true)) = held
        && !slot.open
    {
        slot.open = true;
        slot.by = Some((opener.id, opener.trigger, Opening::Plain));
    }
    if slot.open && !first.active(slot.by).0.enabled {
        slot.open = false;
    }
    if !slot.open {
        slot.by = None;
    }
    let open = slot.open;
    if !stage.is_peek()
        && let Some(cell) = open_cell(cx, first.id, open)
    {
        cell.set_if_changed(cx, open);
    }
    open
}

/// Whether the layer of the container hung from the opener `opener` is open, as its slot holds it, with no tracked
/// read: for the build that decides whether to show the layer.
pub(crate) fn slot_open(ctx: &Context, opener: Id) -> bool {
    MemoryStage::peek_session::<OpenerOpen>(ctx, sub(opener, "opens")).open
}

/// Where the layer of the container hung from `first` hangs, as its slot recorded the opener that opened it: that
/// opener's node and anchor (its own anchor, else the layer's `Pointer` or `Corner` place, else the point a secondary
/// click opened it at, else the opener's rect, a window rect).
pub(crate) fn hung_from(ctx: &Context, first: &Opener, place: &Place) -> (Id, Anchor) {
    let by = MemoryStage::peek_session::<OpenerOpen>(ctx, sub(first.id, "opens")).by;
    let (opener, opening) = first.active(by);
    let anchor = match opener_anchor(opener, opening, place) {
        Anchor::Response => Anchor::Rect(opener.rect),
        anchor => anchor,
    };
    (opener.id, anchor)
}

/// Whether the container hung from the opener `opener` (the id of its response, [`Overlay::opened_by`]) is open, as
/// its slot holds it: an opener reports its expanded state by it.
///
/// [`Overlay::opened_by`]: super::Overlay::opened_by
pub fn opened_from(ctx: &Context, opener: Id) -> bool {
    let open = MemoryStage::peek_session::<OpenerOpen>(ctx, sub(opener, "opens")).open;
    // Inside a build, a tracked read of the cell on the opener's node.
    log_build_read(move |cx| {
        if let Some(cell) = open_cell(cx, opener, open) {
            cell.get(cx);
        }
    });
    open
}

/// The cell on the node of `opener` (found through the window's id map) that holds whether the container hung from it
/// is open, made holding `open` on first use; `None` while no node shows `opener`.
fn open_cell(cx: &mut BuildCx<'_>, opener: Id, open: bool) -> Option<State<bool>> {
    let node = ids::node_of(cx, opener)?;
    cx.slot_of(node, ("craft-opener-open", opener.value()), || open)
}

/// The slot of `opener` holds `open`: when the running build ends, the cell on the opener's node takes it, and its
/// readers run again if it moved.
fn note_open(opener: Id, open: bool) {
    log_build_read(move |cx| {
        if let Some(cell) = open_cell(cx, opener, open) {
            cell.set_if_changed(cx, open);
        }
    });
}

/// Writes the opener's slot from the owner's build `cx`: the container's layer closed (Escape, a press outside, a row
/// ran).
pub(crate) fn set_opener_open(cx: &mut BuildCx<'_>, ctx: &Context, opener: &Opener, open: bool) {
    let mut stage = MemoryStage::open(ctx, opener.id);
    let slot = stage.session::<OpenerOpen>(sub(opener.id, "opens"));
    slot.open = open;
    if !open {
        slot.by = None;
    }
    stage.write_back();
    if let Some(cell) = open_cell(cx, opener.id, open) {
        cell.set_if_changed(cx, open);
    }
}

/// The open state of a container's layer that has an opener and no binding, driven from the opener's side.
pub struct Popup;

impl Popup {
    /// Opens the container hung from `opener` (a [`PopupHandle`] or the id of the opener's response).
    pub fn open<C: Component>(opener: impl Into<Id>, cx: &mut Cx<'_, C>) {
        Self::write(opener.into(), cx, Some(true));
    }

    /// Closes the container hung from `opener`.
    pub fn close<C: Component>(opener: impl Into<Id>, cx: &mut Cx<'_, C>) {
        Self::write(opener.into(), cx, Some(false));
    }

    /// Opens the container hung from `opener` when it is closed, else closes it.
    pub fn toggle<C: Component>(opener: impl Into<Id>, cx: &mut Cx<'_, C>) {
        Self::write(opener.into(), cx, None);
    }

    /// Writes the slot. The component that writes it opened or closed the layer by its own activation, which closes
    /// no layer chain it sits in (`Cx::keep_open`).
    fn write<C: Component>(opener: Id, cx: &mut Cx<'_, C>, to: Option<bool>) {
        let slot = cx.core.memory.session::<OpenerOpen>(sub(opener, "opens"));
        slot.open = to.unwrap_or(!slot.open);
        slot.by = None;
        let open = slot.open;
        if !cx.core.memory.is_peek() {
            note_open(opener, open);
        }
        cx.core.keep_open = true;
    }

    /// Whether the layer `popup` shows first in this build: true in the build rounds of the frame it first shows,
    /// false in the frames after, so what a body resets on opening resets once.
    pub fn opening<C: Component>(popup: PopupHandle, cx: &Cx<'_, C>) -> bool {
        cx.core.popups.opening(popup.id())
    }
}
