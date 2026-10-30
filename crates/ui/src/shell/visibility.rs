//! Which slots show: the host's flags, the body-only mode, the Tab reveal and the edge reveal.

use egui::{Context, Event, Key, Popup, RawInput, Rect, pos2};

use super::ids::{State, WindowKey, set_state, state};
use super::slot::{Edge, Slot};
use crate::direction;

/// What the window shows besides its body.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BodyMode {
    /// The slots show as their `visible` flags say.
    #[default]
    Normal,
    /// No slot shows, floating windows and document tabs hide, and the backdrop is black. A
    /// reveal shows the slots built with [`Slot::reveal`].
    Only,
}

/// The edges with an edge reveal, indexing [`State::edges`].
const REVEALING: [Edge; 3] = [Edge::Top, Edge::Start, Edge::End];

pub(super) fn index(edge: Edge) -> Option<usize> {
    REVEALING.iter().position(|e| *e == edge)
}

/// Does `slot` on `edge` show?
pub(super) fn shows(slot: &Slot, edge: Edge, mode: BodyMode, allow_reveal: bool, state: &State) -> bool {
    match mode {
        BodyMode::Normal => slot.visible,
        BodyMode::Only => allow_reveal && slot.reveal && (state.tab || index(edge).is_some_and(|i| state.edges[i])),
    }
}

/// Turns the Tab reveal of `window` on or off. Ignored (and false) unless the window's last pass
/// was in [`BodyMode::Only`] with reveal allowed. Returns whether the reveal is now on.
pub(super) fn toggle(ctx: &Context, window: &WindowKey) -> bool {
    let mut s = state(ctx, window);
    if !(s.only && s.allow_reveal) {
        return false;
    }
    s.tab = !s.tab;
    if s.tab {
        s.edges = [false; 3];
    }
    let on = s.tab;
    set_state(ctx, window, s);
    ctx.request_repaint();
    on
}

pub(super) fn clear(ctx: &Context, window: &WindowKey) {
    let mut s = state(ctx, window);
    if s.tab || s.edges != [false; 3] {
        s.tab = false;
        s.edges = [false; 3];
        set_state(ctx, window, s);
        ctx.request_repaint();
    }
}

pub(super) fn active(ctx: &Context, window: &WindowKey) -> bool {
    state(ctx, window).tab
}

/// True while Tab should toggle the chrome: no control has keyboard focus, or the body's canvas
/// has it. Inside a control Tab moves focus as usual. False while something else owns the
/// keyboard: an egui modal, an app-modal window, a menu or a popup list.
pub fn tab_toggles_chrome(ctx: &Context) -> bool {
    if ctx.memory(|m| m.top_modal_layer().is_some())
        || crate::viewport::ViewportWindow::blocks_main(ctx)
        || crate::menu::any_open(ctx)
        || crate::popup_list::any_open(ctx)
    {
        return false;
    }
    let focused = ctx.memory(|m| m.focused());
    focused.is_none() || focused == super::ids::canvas(ctx)
}

/// For the host's `raw_input_hook`, before the pass runs: when the input holds a Tab press
/// without modifiers and [`tab_toggles_chrome`], removes the Tab key events (egui moves focus on
/// Tab at the start of the pass, so a later `consume_key` is too late) and returns true; the host
/// then calls [`AppShell::toggle_reveal`](super::AppShell::toggle_reveal) in body-only mode, or
/// flips its own flags otherwise.
pub fn take_tab_toggle(ctx: &Context, raw: &mut RawInput) -> bool {
    let is_tab = |e: &Event| matches!(e, Event::Key { key: Key::Tab, .. });
    let pressed = raw.events.iter().any(|e| matches!(e, Event::Key { key: Key::Tab, pressed: true, modifiers, .. } if modifiers.is_none()));
    if !pressed || !tab_toggles_chrome(ctx) {
        return false;
    }
    raw.events.retain(|e| !is_tab(e));
    true
}

/// Updates the Tab and edge reveal at the start of a pass and returns the state to draw with.
/// `window_rect` is the shell's rect; `edge_px` the zone's width.
pub(super) fn begin_pass(ctx: &Context, window: &WindowKey, mode: BodyMode, allow_reveal: bool, window_rect: Rect, edge_px: f32) -> State {
    let mut s = state(ctx, window);
    let only = mode == BodyMode::Only;
    if !only || !allow_reveal {
        // Reveal ends with body-only mode.
        s.tab = false;
        s.edges = [false; 3];
    } else if s.tab {
        s.edges = [false; 3];
    } else {
        // A held button or an open popup freezes the edge reveal: it neither starts nor ends.
        let frozen = ctx.input(|i| i.pointer.any_down()) || Popup::is_any_open(ctx) || crate::popup_list::any_open(ctx);
        if !frozen {
            let pointer = ctx.input(|i| i.pointer.hover_pos()).filter(|p| window_rect.contains(*p));
            for (i, edge) in REVEALING.into_iter().enumerate() {
                let zone = zone(ctx, window_rect, edge, edge_px);
                let in_zone = pointer.is_some_and(|p| zone.contains(p));
                let over_slot = s.edges[i] && pointer.is_some_and(|p| s.shown.iter().any(|(e, r)| *e == edge && r.contains(p)));
                s.edges[i] = in_zone || over_slot;
            }
        }
    }
    s.only = only;
    s.allow_reveal = allow_reveal;
    let before = state(ctx, window);
    if before.tab != s.tab || before.edges != s.edges {
        // The layout changes now; the slots' rects are measured on the next pass.
        ctx.request_repaint();
    }
    s
}

/// The strip of `edge_px` along an edge of the window.
fn zone(ctx: &Context, window: Rect, edge: Edge, edge_px: f32) -> Rect {
    match edge {
        Edge::Top => Rect::from_min_max(window.min, pos2(window.max.x, window.min.y + edge_px)),
        Edge::Bottom => Rect::from_min_max(pos2(window.min.x, window.max.y - edge_px), window.max),
        Edge::Start | Edge::End => {
            let at_left = (edge == Edge::Start) != direction::is_rtl(ctx);
            if at_left {
                Rect::from_min_max(window.min, pos2(window.min.x + edge_px, window.max.y))
            } else {
                Rect::from_min_max(pos2(window.max.x - edge_px, window.min.y), window.max)
            }
        }
    }
}
