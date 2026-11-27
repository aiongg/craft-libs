//! Each window's layer stack. A node puts a child on a layer with [`crate::BuildCx::layer`]: the
//! child paints, hit-tests and lays out under that layer, above everything on the layers below,
//! while it stays the building node's child for build and output (it goes when the node stops
//! showing it or goes itself). Layer children are placed after the window's layout, so an anchor
//! that moved in the same frame is where the child goes.

use craft_behavior::PointerButton;
use craft_keymap::chord::Mods;
use emath::{Align, Pos2, Rect, Vec2};

use crate::geom::Constraints;
use crate::id::NodeId;

/// The layers of a window, bottom to top. A modal layer with children blocks the pointer from the
/// layers below it; popups, menus and tooltips sit above it so a dialog can open them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Layer {
    /// The app root.
    Content,
    /// In-window floating panels and columns.
    Floating,
    /// Dialogs and modal sheets.
    Modal,
    /// Popovers, dropdown lists, pickers.
    Popup,
    /// Menus and submenus.
    Menu,
    Tooltip,
    /// The drag ghost and drop previews.
    Drag,
    /// Notifications.
    Toast,
}

impl Layer {
    pub const ALL: [Layer; 8] = [Layer::Content, Layer::Floating, Layer::Modal, Layer::Popup, Layer::Menu, Layer::Tooltip, Layer::Drag, Layer::Toast];

    pub(crate) fn index(self) -> u8 {
        self as u8
    }
}

/// Which side of the anchor a layer child goes on. When it does not fit there it flips to the
/// opposite side if that side has more room.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Below,
    Above,
    Right,
    Left,
}

/// Where a layer child goes, resolved after layout in window coordinates. The result is kept
/// inside the window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Placement {
    /// Top left at a window position.
    At(Pos2),
    /// Beside a node's window rect (the button that opened a menu). `align` places the child
    /// along the side: `Min` lines up the starts.
    Anchor { node: NodeId, side: Side, align: Align, gap: f32 },
    /// Beside a window rect that is not a node (a caret, a pointer position).
    Beside { rect: Rect, side: Side, align: Align, gap: f32 },
    /// Centred in the window.
    Center,
    /// Covers the window (a modal backdrop).
    Fill,
}

/// What a press outside a layer child does to it. "Outside" is outside the child and everything
/// it owns (its own layer children: a submenu is inside its menu), and outside its anchor node,
/// whose own press handler toggles it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OutsidePress {
    #[default]
    Ignore,
    /// The child gets [`crate::Event::Dismiss`]; the press goes on to what it hit.
    Dismiss,
    /// The child gets [`crate::Event::Dismiss`] and the press ends there (a menu: the click that
    /// closes it activates nothing).
    DismissAndConsume,
}

/// How a node shows a child on a layer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayerSpec {
    pub placement: Placement,
    pub outside_press: OutsidePress,
    /// The child is at least as wide as its anchor (a dropdown list under its field).
    pub match_anchor_width: bool,
    /// An Escape no focused node handled goes to the topmost such child as
    /// [`DismissReason::Escape`].
    pub escape: bool,
    /// Key focus moving from inside the child (or anything it owns) to a node outside it sends
    /// [`DismissReason::FocusLeft`].
    pub focus_left: bool,
    /// On the modal layer: the child blocks pointer and key input to every other window of the
    /// runtime while it is shown (an app-modal dialog). A press in a blocked window focuses the
    /// child's window.
    pub app_modal: bool,
    /// A press in another window of the runtime, or the app losing key focus (no window of the
    /// runtime focused at the next frame), sends [`DismissReason::Away`]: a popup or menu goes when
    /// the user turns to something else.
    pub away: bool,
    /// On the modal layer: the node below the modal that some input still reaches ([`ModalReach`]).
    pub reach: Option<ModalReach>,
    /// The node a child placed beside a rect hangs from (the item or part it opened from): a press
    /// on it is not outside the child, and focus moving onto it is not focus leaving. A
    /// [`Placement::Anchor`] child hangs from its node already.
    pub anchor: Option<NodeId>,
    /// The window rect the child stays inside (a shell's popup region below its bars); the whole
    /// window when `None`.
    pub bounds: Option<Rect>,
}

/// The one exception to "a modal layer blocks the input below it": a dialog that samples a canvas
/// (a colour picker's eyedropper, a levels dialog's view navigation) lets some input through to
/// that node. The wheel and middle presses always reach it, and so do presses of the listed
/// buttons with at least the listed modifiers held. Only where the node is the topmost shape below
/// the modal: something that covers it keeps the input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModalReach {
    pub node: NodeId,
    pub presses: [Option<(PointerButton, Mods)>; 4],
}

impl ModalReach {
    /// The wheel and middle presses on `node`.
    pub const fn view(node: NodeId) -> ModalReach {
        ModalReach { node, presses: [None; 4] }
    }

    /// Also presses of `button` with at least `mods` held. A fifth gesture is dropped.
    #[must_use]
    pub fn with_press(mut self, button: PointerButton, mods: Mods) -> ModalReach {
        if !self.presses.contains(&Some((button, mods)))
            && let Some(free) = self.presses.iter_mut().find(|p| p.is_none())
        {
            *free = Some((button, mods));
        }
        self
    }

    /// Whether a press of `button` with `held` down reaches the node.
    pub fn admits_press(&self, button: PointerButton, held: Mods) -> bool {
        button == PointerButton::Middle
            || self.presses.iter().flatten().any(|&(b, need)| {
                b == button && (!need.cmd || held.cmd) && (!need.ctrl || held.ctrl) && (!need.alt || held.alt) && (!need.shift || held.shift)
            })
    }
}

impl LayerSpec {
    pub const fn new(placement: Placement) -> LayerSpec {
        LayerSpec {
            placement,
            outside_press: OutsidePress::Ignore,
            match_anchor_width: false,
            escape: false,
            focus_left: false,
            app_modal: false,
            away: false,
            reach: None,
            anchor: None,
            bounds: None,
        }
    }

    /// The child hangs from `node` ([`LayerSpec::anchor`]).
    pub const fn hung_from(mut self, node: Option<NodeId>) -> LayerSpec {
        self.anchor = node;
        self
    }

    /// The child stays inside `bounds` ([`LayerSpec::bounds`]).
    pub const fn within(mut self, bounds: Option<Rect>) -> LayerSpec {
        self.bounds = bounds;
        self
    }

    pub const fn dismiss_on_away(mut self) -> LayerSpec {
        self.away = true;
        self
    }

    pub const fn reach(mut self, reach: Option<ModalReach>) -> LayerSpec {
        self.reach = reach;
        self
    }

    pub const fn dismiss_on_escape(mut self) -> LayerSpec {
        self.escape = true;
        self
    }

    pub const fn dismiss_on_focus_left(mut self) -> LayerSpec {
        self.focus_left = true;
        self
    }

    pub const fn app_modal(mut self) -> LayerSpec {
        self.app_modal = true;
        self
    }

    pub const fn outside_press(mut self, outside: OutsidePress) -> LayerSpec {
        self.outside_press = outside;
        self
    }

    pub const fn match_anchor_width(mut self) -> LayerSpec {
        self.match_anchor_width = true;
        self
    }

    pub(crate) fn anchor_node(&self) -> Option<NodeId> {
        match self.placement {
            Placement::Anchor { node, .. } => Some(node),
            _ => self.anchor,
        }
    }

    /// The constraints the child is laid out with, given its anchor's window rect.
    pub fn constraints(&self, viewport: Vec2, anchor: Option<Rect>) -> Constraints {
        let viewport = self.bounds.map_or(viewport, |b| b.size().min(viewport).max(Vec2::ZERO));
        match self.placement {
            Placement::Fill => Constraints::tight(viewport),
            _ => {
                let mut c = Constraints::loose(viewport);
                if self.match_anchor_width
                    && let Some(a) = anchor
                {
                    c.min.x = a.width().min(viewport.x);
                }
                c
            }
        }
    }

    /// The child's top left for `size`, given its anchor's window rect (`None` when the anchor
    /// node is gone: the child stays where it is, `current`).
    pub fn place(&self, size: Vec2, viewport: Vec2, anchor: Option<Rect>, current: Vec2) -> Vec2 {
        if let Some(bounds) = self.bounds {
            // Placed inside the bounds as inside a window of their size, then moved back.
            let origin = bounds.min.to_vec2();
            let inner = LayerSpec { bounds: None, placement: shifted(self.placement, -origin), ..*self };
            let anchor = anchor.map(|a| a.translate(-origin));
            return inner.place(size, bounds.size().min(viewport).max(Vec2::ZERO), anchor, current - origin) + origin;
        }
        let pos = match self.placement {
            Placement::At(p) => p.to_vec2(),
            Placement::Center => (viewport - size) / 2.0,
            Placement::Fill => Vec2::ZERO,
            Placement::Anchor { side, align, gap, .. } => match anchor {
                Some(rect) => beside(rect, side, align, gap, size, viewport),
                None => current,
            },
            Placement::Beside { rect, side, align, gap } => beside(rect, side, align, gap, size, viewport),
        };
        let max = (viewport - size).max(Vec2::ZERO);
        Vec2::new(pos.x.clamp(0.0, max.x), pos.y.clamp(0.0, max.y)).round()
    }
}

/// `placement` with its window positions moved by `by`.
fn shifted(placement: Placement, by: Vec2) -> Placement {
    match placement {
        Placement::At(p) => Placement::At(p + by),
        Placement::Beside { rect, side, align, gap } => Placement::Beside { rect: rect.translate(by), side, align, gap },
        other => other,
    }
}

fn beside(rect: Rect, side: Side, align: Align, gap: f32, size: Vec2, viewport: Vec2) -> Vec2 {
    let room = |s: Side| match s {
        Side::Below => viewport.y - rect.max.y - gap,
        Side::Above => rect.min.y - gap,
        Side::Right => viewport.x - rect.max.x - gap,
        Side::Left => rect.min.x - gap,
    };
    let opposite = match side {
        Side::Below => Side::Above,
        Side::Above => Side::Below,
        Side::Right => Side::Left,
        Side::Left => Side::Right,
    };
    let need = match side {
        Side::Below | Side::Above => size.y,
        Side::Right | Side::Left => size.x,
    };
    let side = if room(side) < need && room(opposite) > room(side) { opposite } else { side };
    let along = |start: f32, end: f32, extent: f32| match align {
        Align::Min => start,
        Align::Center => (start + end - extent) / 2.0,
        Align::Max => end - extent,
    };
    match side {
        Side::Below => Vec2::new(along(rect.min.x, rect.max.x, size.x), rect.max.y + gap),
        Side::Above => Vec2::new(along(rect.min.x, rect.max.x, size.x), rect.min.y - gap - size.y),
        Side::Right => Vec2::new(rect.max.x + gap, along(rect.min.y, rect.max.y, size.y)),
        Side::Left => Vec2::new(rect.min.x - gap - size.x, along(rect.min.y, rect.max.y, size.y)),
    }
}

/// Why a layer child is asked to go.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DismissReason {
    /// A press outside it ([`OutsidePress`]).
    OutsidePress,
    /// Escape reached no focused node that handled it ([`LayerSpec::escape`]).
    Escape,
    /// Key focus left it ([`LayerSpec::focus_left`]).
    FocusLeft,
    /// The user turned to another window or app ([`LayerSpec::away`]).
    Away,
}

#[cfg(test)]
mod tests {
    use super::*;
    use emath::pos2;

    #[test]
    fn a_menu_flips_above_its_anchor_when_there_is_no_room_below() {
        let viewport = Vec2::new(400.0, 300.0);
        let anchor = Rect::from_min_max(pos2(10.0, 260.0), pos2(90.0, 280.0));
        let spec = LayerSpec::new(Placement::Anchor { node: NodeId(9), side: Side::Below, align: Align::Min, gap: 2.0 });
        let at = spec.place(Vec2::new(120.0, 100.0), viewport, Some(anchor), Vec2::ZERO);
        assert_eq!(at, Vec2::new(10.0, 158.0));
        // Room below: it stays below, pushed inside the window on the right.
        let anchor = Rect::from_min_max(pos2(350.0, 20.0), pos2(390.0, 40.0));
        let at = spec.place(Vec2::new(120.0, 100.0), viewport, Some(anchor), Vec2::ZERO);
        assert_eq!(at, Vec2::new(280.0, 42.0));
    }
}
