//! The window facts and requests that both `Ui` and a hook's `CxCore` reach (contract-rt 15.9): one function each over
//! the build that runs, so a tracked read subscribes only the node that reads it.

use craft_behavior::{Buttons, OwnerClass, PointerButton};
use craft_ui_rt::{BuildCx, InteractFact};

use super::layers::LayerChildOf;
use crate::contract::context::Context;
use crate::primitives::Id;
use crate::runtime::focus::PickedIn;
use crate::runtime::overlay::{OpenEntry, OpenFilter};
use crate::runtime::pointer::PointerFrame;

/// The window's pointer as a tool reads it: its position, the buttons held, the modifiers and the frame time, with the
/// theme's hit `slop` and `target`. Tracked through the window's pointer cell, so the reader runs again at every
/// pointer move. Presses, releases, wheel and gestures reach the node they target as its input, so the frame's event
/// fields are empty.
pub(crate) fn pointer(cx: &mut BuildCx<'_>, ctx: &Context, slop: f32, target: f32) -> PointerFrame {
    let facts = cx.env().pointer.get(cx);
    let buttons = [PointerButton::Primary, PointerButton::Secondary, PointerButton::Middle, PointerButton::Extra1, PointerButton::Extra2]
        .into_iter()
        .filter(|button| facts.held(*button))
        .fold(Buttons::empty(), |held, button| held | Buttons::of(button));
    PointerFrame::held(facts.pos, buttons, ctx.modifiers(), cx.now(), slop, target)
}

/// The focused stop of the window `cx` builds in, from the focused node's kept facts. Tracked through the window's
/// focused-node cell: the reader runs again when focus moves, and at no other input.
pub(crate) fn focused_stop(cx: &mut BuildCx<'_>) -> Option<crate::runtime::focus::StopRec> {
    let node = cx.env().focused.get(cx)?;
    let rt = cx.runtime();
    let kept = rt
        .local::<super::node::ComponentNode>(node)
        .or_else(|| rt.local::<super::node::ScopeNode>(node))
        .or_else(|| rt.local::<super::overlay::LayerScope>(node).map(|lk| &lk.kept))?;
    let keys = kept.focus.stop?;
    Some(crate::runtime::focus::StopRec::of_kept(kept.id, keys, kept.focus.press_only))
}

/// The top open layer of the window `cx` builds in that takes keys, if any. Tracked through the open-layer cell.
pub(crate) fn top_layer(cx: &mut BuildCx<'_>) -> Option<OpenEntry> {
    super::layers::track(cx);
    let open = super::layers::open(cx.runtime(), cx.window());
    open.into_iter().rev().find(|open| super::layers::counts(&open.entry, OpenFilter::Keys)).map(|open| open.entry)
}

/// The id of the focus scope of the top layer that takes keys, if one is open: the scope a newly opened layer nests
/// in. Tracked through the open-layer cell.
pub(crate) fn open_focus_scope(cx: &mut BuildCx<'_>) -> Option<Id> {
    top_layer(cx).filter(|entry| entry.scoped).map(|entry| entry.id)
}

/// Whether the node `cx` builds lies in the top modal layer. Tracked through the open-layer cell.
pub(crate) fn in_top_modal(cx: &mut BuildCx<'_>) -> bool {
    super::layers::track(cx);
    super::layers::in_top_modal(cx.runtime(), cx.node())
}

/// Whether the layer `id` is the top modal layer of the window `cx` builds in. Tracked through the open-layer cell.
pub(crate) fn is_top_modal(cx: &mut BuildCx<'_>, id: Id) -> bool {
    super::layers::track(cx);
    super::layers::top_modal(cx.runtime(), cx.window()).is_some_and(|top| top.entry.id == id)
}

/// The menu layer the node `cx` builds sits in, if it sits in one.
pub(crate) fn running_menu(cx: &BuildCx<'_>) -> Option<Id> {
    let of = cx.provided::<LayerChildOf>()?;
    (of.1 == Some(OwnerClass::Menu)).then_some(of.0)
}

/// Whether the focused node sits in a menu layer and its focus shows (the keyboard drives the menu). Tracked through
/// the window's focused-node cell and the focused node's `FocusVisible`.
pub(crate) fn keyboard_in_menu(cx: &mut BuildCx<'_>) -> bool {
    let Some(focused) = cx.env().focused.get(cx) else { return false };
    let in_menu = cx.provided_above::<LayerChildOf>(focused).is_some_and(|of| of.1 == Some(OwnerClass::Menu));
    in_menu && cx.read_interaction(focused, InteractFact::FocusVisible)
}

/// Whether the focused node took the focus as its layer or scope appeared (a menu's first row as the menu showed) and
/// has kept it since: no key, press or request moved it. A menu row reads it to tell a key that reached the row its
/// menu's opening focused from a key on a row the user or assistive technology chose. Tracked through the window's
/// focused-node cell.
pub(crate) fn focus_came_with_open(cx: &mut BuildCx<'_>) -> bool {
    let Some(focused) = cx.env().focused.get(cx) else { return false };
    cx.runtime().local::<super::node::ComponentNode>(focused).is_some_and(|kept| kept.focus.entered)
}

/// Moves focus back out of the layer a pick was made in, before the pick's command runs, so the command reaches the
/// stop focus returns to: the layer's focus return (its opener), as its closing would, and past a menu bar that F10 or
/// a lone Alt focused, to the stop the bar took the focus from. Nothing when the layer has no live node.
pub(crate) fn return_from_pick(cx: &mut BuildCx<'_>, picked: PickedIn) {
    let layer = match picked {
        PickedIn::Menus(layer) | PickedIn::Layer(layer) => layer,
    };
    let Some(node) = super::ids::node_of(cx, layer) else { return };
    let rt = cx.runtime();
    // A menu pick walks out of every menu under the first: the outermost layer child that has a return names it.
    let mut to = rt.focus_return(cx.window(), node);
    if matches!(picked, PickedIn::Menus(_)) {
        while let Some(at) = to {
            match super::layers::layer_child_of(rt, at).and_then(|child| rt.focus_return(cx.window(), child)) {
                Some(next) => to = Some(next),
                None => break,
            }
        }
    }
    if let Some(to) = to {
        let to = super::node::MenuBar::past_bar(cx, to);
        cx.focus_node(to);
    }
}

/// A request a hook made that changes the window (`CxCore::return_from_pick`, `CxCore::close_layer`): the run records
/// it, and the driver applies it after the run, in order, as it applies focus requests.
#[derive(Clone, Copy)]
pub(crate) enum LayerAsk {
    ReturnFromPick(PickedIn),
    Close(Id, crate::runtime::overlay::CloseReason),
}

impl LayerAsk {
    pub(crate) fn apply(self, cx: &mut BuildCx<'_>) {
        match self {
            LayerAsk::ReturnFromPick(picked) => return_from_pick(cx, picked),
            LayerAsk::Close(id, why) => super::overlay::close_layer(cx, id, why),
        }
    }
}
