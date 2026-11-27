//! The open state of a container's layer with no binding, kept in its opener's slot, and `Popup`.

use super::opener::{Opener, Opening, Trigger, opener_anchor};
use super::place::{Anchor, Place};
use super::popups::{HOLD, PopupHandle, dwell};
use crate::contract::cx::CxCore;
use crate::contract::{Component, Context, Cx};
use crate::primitives::Id;
use crate::runtime::memory::{MemoryStage, sub};

/// Open state a container with no `open` binding keeps in its opener's slot.
#[derive(Clone, Copy, Default)]
struct OpenerOpen {
    open: bool,
    /// The frame time an opener's trigger was last applied at: a frame's build rounds apply it once.
    applied: Option<f64>,
    /// The opener that opened the layer by id and trigger, and how it fired.
    by: Option<(Id, Trigger, Opening)>,
}

/// The open state of a container with no binding, in the run `core` of the container: the slot of its first opener,
/// with the triggers of every opener applied once per frame. A held `Trigger::Hold` opener opens it after `HOLD`
/// seconds; the container builds again at that deadline. A measuring run's stage writes nothing back.
///
/// An opener that fired here opened the layer by that activation: the caller claims it
/// ([`Opener::claims_activation`], `dismissal::claim_activation`) so the layer chain the opener sits in stays open.
pub(crate) fn opener_open(core: &mut CxCore<'_>, first: &Opener) -> bool {
    let now = core.context().time();
    let held = first.all().find(|opener| opener.trigger == Trigger::Hold).map(|opener| {
        // The dwell is timed under the opener's id in the container's node memory.
        let started = dwell(core, opener.id, "dwell", opener.pressed, HOLD);
        (opener, started)
    });
    let slot = core.memory.session::<OpenerOpen>(sub(first.id, "opens"));
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
    if !slot.open {
        slot.by = None;
    }
    slot.open
}

/// Where the layer of the container hung from `first` hangs, as its slot recorded the opener that opened it: that
/// opener's own anchor, else the layer's `Pointer` or `Corner` place, else the point a secondary click opened it at,
/// else the opener's rect (a window rect).
pub(crate) fn hung_from(stage: &mut MemoryStage, first: &Opener, place: &Place) -> (Id, Anchor) {
    let by = stage.session::<OpenerOpen>(sub(first.id, "opens")).by;
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
    MemoryStage::peek_session::<OpenerOpen>(ctx, sub(opener, "opens")).open
}

/// Writes the opener's slot: the container closed itself (Escape, a press outside, a row ran).
pub(crate) fn set_opener_open(stage: &mut MemoryStage, opener: &Opener, open: bool) {
    let slot = stage.session::<OpenerOpen>(sub(opener.id, "opens"));
    slot.open = open;
    if !open {
        slot.by = None;
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
        cx.core.keep_open = true;
    }

    /// Whether the layer `popup` shows first in this build: true in the build rounds of the frame it first shows,
    /// false in the frames after, so what a body resets on opening resets once.
    pub fn opening<C: Component>(popup: PopupHandle, cx: &Cx<'_, C>) -> bool {
        cx.core.popups.opening(popup.id())
    }
}
