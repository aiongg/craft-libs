//! Invisible resize zones along an undecorated window's edges, and the end of a press that the
//! window manager took for an OS resize or move.

use egui::{CursorIcon, Id, LayerId, Order, PointerButton, Rect, ResizeDirection, Sense, Ui, ViewportCommand, ViewportId, pos2, vec2};

use super::TitleBarStyle;
use crate::theme::{Tokens, TokensOf as _};

/// The edge zones `e` thick and the corner zones `c` square of a window with content rect `w`: (zone, direction, cursor). Corners
/// come last so they win where they overlap the edges. Edges use the one-sided cursors
/// (`w-resize`, …), as GTK and KDE window borders do: some cursor themes ship no `ew-resize` /
/// `ns-resize`, and then X11 showed the core font's arrow and Wayland no resize cursor. Corners
/// keep the diagonal ones (`nwse-resize`), which those themes draw like the edges.
fn zones(w: Rect, e: f32, c: f32) -> [(Rect, ResizeDirection, CursorIcon); 8] {
    use ResizeDirection::*;
    [
        (Rect::from_min_max(w.left_top(), pos2(w.right(), w.top() + e)), North, CursorIcon::ResizeNorth),
        (Rect::from_min_max(pos2(w.left(), w.bottom() - e), w.right_bottom()), South, CursorIcon::ResizeSouth),
        (Rect::from_min_max(w.left_top(), pos2(w.left() + e, w.bottom())), West, CursorIcon::ResizeWest),
        (Rect::from_min_max(pos2(w.right() - e, w.top()), w.right_bottom()), East, CursorIcon::ResizeEast),
        (Rect::from_min_size(w.left_top(), vec2(c, c)), NorthWest, CursorIcon::ResizeNwSe),
        (Rect::from_min_size(w.right_top() - vec2(c, 0.0), vec2(c, c)), NorthEast, CursorIcon::ResizeNeSw),
        (Rect::from_min_size(w.left_bottom() - vec2(0.0, c), vec2(c, c)), SouthWest, CursorIcon::ResizeNeSw),
        (Rect::from_min_size(w.right_bottom() - vec2(c, c), vec2(c, c)), SouthEast, CursorIcon::ResizeNwSe),
    ]
}

/// The parts of `r` outside every rect of `holes`.
pub(crate) fn subtract(r: Rect, holes: &[Rect]) -> Vec<Rect> {
    let mut pieces = vec![r];
    for hole in holes {
        pieces = pieces
            .into_iter()
            .flat_map(|p| {
                let i = p.intersect(*hole);
                if !i.is_positive() {
                    return vec![p];
                }
                [
                    Rect::from_min_max(p.min, pos2(p.max.x, i.min.y)),
                    Rect::from_min_max(pos2(p.min.x, i.max.y), p.max),
                    Rect::from_min_max(pos2(p.min.x, i.min.y), pos2(i.min.x, i.max.y)),
                    Rect::from_min_max(pos2(i.max.x, i.min.y), pos2(p.max.x, i.max.y)),
                ]
                .into_iter()
                .filter(Rect::is_positive)
                .collect()
            })
            .collect();
    }
    pieces
}

/// One entry per OS window: a press belongs to one window.
fn handed_off_id(viewport: ViewportId) -> Id {
    Id::unique(("title_bar_resize_handed_off", viewport))
}

/// An OS resize or move started at `pass`; `released` is the pass whose input ended the press.
#[derive(Clone, Copy, Debug)]
struct HandOff {
    pass: u64,
    released: Option<u64>,
}

/// The OS took the press of the window being drawn at this pass: [`release_after_os_resize`]
/// ends it on the next input.
pub(crate) fn hand_off(ctx: &egui::Context) {
    let (pass, id) = (ctx.cumulative_pass_nr(), handed_off_id(ctx.viewport_id()));
    ctx.data_mut(|d| d.insert_temp(id, HandOff { pass, released: None }));
}

/// Move the window from this press. The window manager keeps the release of an OS move as it does
/// that of a resize, so egui kept the drag and every other move failed; the same hand-off ends it.
pub(crate) fn start_os_move(ctx: &egui::Context) {
    ctx.send_viewport_cmd(ViewportCommand::StartDrag);
    hand_off(ctx);
}

/// Ends a press the window manager took for an OS resize or move, in the window `raw` is the input
/// of. Call it from the app's `raw_input_hook` whenever the window is undecorated.
///
/// The compositor takes the button release of an OS resize or move (KDE Wayland, KWin on X11), so
/// egui kept the button down: the zones stopped hovering until the next click, and every other
/// resize or move failed. This ends the press on the input after the hand-off, but only while egui
/// still has it down and this input brings no release of its own: where the OS does send it
/// (Windows), nothing is added, so no second release can turn into a click.
pub fn release_after_os_resize(ctx: &egui::Context, raw: &mut egui::RawInput) {
    let viewport = raw.viewport_id;
    let Some(h) = ctx.data(|d| d.get_temp::<HandOff>(handed_off_id(viewport))) else { return };
    let pass = ctx.cumulative_pass_nr_for(viewport);
    if h.released.is_some() || h.pass >= pass {
        return;
    }
    let os_released = raw.events.iter().any(|e| matches!(e, egui::Event::PointerButton { button: PointerButton::Primary, pressed: false, .. }));
    let (latest, down, modifiers) = ctx.input_for(viewport, |i| (i.pointer.latest_pos(), i.pointer.primary_down(), i.modifiers));
    if let Some(pos) = latest.filter(|_| !os_released && down) {
        raw.events.push(egui::Event::PointerButton { pos, button: PointerButton::Primary, pressed: false, modifiers });
    }
    ctx.data_mut(|d| d.insert_temp(handed_off_id(viewport), HandOff { released: Some(pass), ..h }));
}

/// The compositor owns the pointer during an OS resize: it sets its own cursor, and egui-winit only
/// re-sends a cursor that changed, so afterwards the window kept the compositor's arrow until the
/// next press (KDE Wayland). On the first pointer motion after the hand-off, show the default
/// cursor for one frame so the resize cursor is sent again on the next.
pub(crate) fn reshow_cursor_after_os_resize(ctx: &egui::Context) {
    let id = handed_off_id(ctx.viewport_id());
    // Only motion after the press was ended means the compositor gave the pointer back.
    let Some(HandOff { released: Some(pass), .. }) = ctx.data(|d| d.get_temp::<HandOff>(id)) else { return };
    if pass < ctx.cumulative_pass_nr() && ctx.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::PointerMoved(_)))) {
        ctx.data_mut(|d| d.remove::<HandOff>(id));
        ctx.set_cursor_icon(CursorIcon::Default);
        ctx.request_repaint();
    }
}

/// The edge and corner zones alone, for windows that draw no title bar (an embedder hid its menu
/// bar on an undecorated window). Only `Integrated` frames get them, as with [`TitleBar`], and not
/// while maximized or in full screen. `native_hit_test` is true once the window procedure answers
/// the hit test itself. The zones still avoid other interactive widgets in the layer.
///
/// [`TitleBar`]: super::TitleBar
pub fn resize_zones_only(ui: &mut Ui, frame: &super::Frame, native_hit_test: bool) {
    let ctx = ui.ctx().clone();
    let (fullscreen, maximized) = ctx.input(|i| (i.viewport().fullscreen.unwrap_or(false), i.viewport().maximized.unwrap_or(false)));
    if !matches!(frame, super::Frame::Integrated(_)) || native_hit_test {
        return;
    }
    if !fullscreen && !maximized {
        let layer = ui.layer_id();
        let holes: Vec<Rect> =
            ctx.viewport(|v| v.this_pass.widgets.get_layer(layer).filter(|w| w.sense.interactive()).map(|w| w.interact_rect).collect());
        let style = TitleBarStyle::from_tokens(&Tokens::of(ui));
        resize_zones(ui, &holes, super::captions::window_scope(ctx.viewport_id()), &style);
    }
    reshow_cursor_after_os_resize(&ctx);
}

/// The resize zones above everything else, except over `holes` (the caption buttons and the
/// bar's controls), for the bar whose ids derive from `scope`. A press in one starts an OS resize
/// in that direction.
pub(crate) fn resize_zones(ui: &mut Ui, holes: &[Rect], scope: Id, style: &TitleBarStyle) {
    let ctx = ui.ctx().clone();
    let w = ctx.content_rect();
    let layer = LayerId::new(Order::Foreground, scope.with("resize"));
    let mut zui = ui.new_child(egui::UiBuilder::new().layer_id(layer).max_rect(w));
    // The child inherits the parent's clip, which inside a top panel is the panel: the zones below
    // the bar would never be hit.
    zui.set_clip_rect(w);
    for (i, (zone, dir, cursor)) in zones(w, style.edge, style.corner).into_iter().enumerate() {
        for (j, piece) in subtract(zone, holes).into_iter().enumerate() {
            // Drag only: the zones take no keyboard focus.
            let resp = zui.interact(piece, scope.with(("resize", i, j)), Sense::DRAG).on_hover_cursor(cursor);
            if resp.drag_started_by(PointerButton::Primary) {
                ctx.send_viewport_cmd(ViewportCommand::BeginResize(dir));
                hand_off(&ctx);
            }
        }
    }
}
