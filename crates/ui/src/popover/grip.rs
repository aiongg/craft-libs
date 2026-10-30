use egui::{CursorIcon, Id, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2, pos2, vec2};

use crate::direction;
use crate::theme::{ToEgui, Tokens, TokensOf};

/// The corner grip of a resizable popover: three diagonal ticks in a square at the bottom trailing
/// corner of a frame (bottom right, bottom left when mirrored). Pointer only: the keyboard has no
/// resize, and the grip has no AccessKit node.
///
/// The grip reports how far the frame should grow; the owner keeps the size and its limits.
#[must_use = "show the grip with `show`"]
pub struct ResizeGrip {
    id: Id,
}

/// What [`ResizeGrip::show`] returns.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct GripResponse {
    pub response: Response,
    /// The press began this frame.
    pub started: bool,
    /// How far the pointer has moved since the press, as growth: right and down in a left-to-right
    /// layout, left and down mirrored. `None` while not dragged.
    pub growth: Option<Vec2>,
}

impl ResizeGrip {
    /// A grip with `id`, which the owner makes unique (`popover_id.with("grip")`).
    pub fn new(id: Id) -> Self {
        Self { id }
    }

    /// The grip's square in `frame`.
    pub fn rect(ui: &Ui, frame: Rect) -> Rect {
        let size = Tokens::of(ui).component.popover.grip_size;
        let rtl = direction::is_rtl(ui);
        let corner = if rtl { frame.left_bottom() } else { frame.right_bottom() };
        let min = if rtl { pos2(corner.x, corner.y - size) } else { pos2(corner.x - size, corner.y - size) };
        Rect::from_min_size(min, vec2(size, size))
    }

    /// Draws the grip in the corner of `frame` and senses its drag.
    pub fn show(self, ui: &mut Ui, frame: Rect) -> GripResponse {
        let rtl = direction::is_rtl(ui);
        let parts = Tokens::of(ui).component.popover;
        let rect = Self::rect(ui, frame);
        let response = ui.interact(rect, self.id, Sense::DRAG);
        if response.hovered() || response.dragged() {
            ui.ctx().set_cursor_icon(if rtl { CursorIcon::ResizeNeSw } else { CursorIcon::ResizeNwSe });
        }
        let stroke = Stroke::new(1.0, parts.grip_tick.to_egui());
        // Ticks run from the edge toward the corner, 3 points in from both edges.
        let inset = 3.0;
        let corner = if rtl { pos2(rect.left() + inset, rect.bottom() - inset) } else { pos2(rect.right() - inset, rect.bottom() - inset) };
        let side = if rtl { 1.0 } else { -1.0 };
        for k in 1..=3 {
            let d = k as f32 * parts.grip_pitch;
            let from: Pos2 = pos2(corner.x + side * d, corner.y);
            let to: Pos2 = pos2(corner.x, corner.y - d);
            ui.painter().line_segment([from, to], stroke);
        }
        let growth = response.total_drag_delta().filter(|_| response.dragged()).map(|d| if rtl { vec2(-d.x, d.y) } else { d });
        GripResponse { started: response.drag_started(), growth, response }
    }
}
