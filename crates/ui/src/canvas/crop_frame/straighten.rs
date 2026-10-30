//! Straighten: a line drawn along something that should be level or plumb gives the angle that
//! levels it (PhotoCraft's `tools/crop_straighten.rs`).

use egui::{Context, CursorIcon, Id, Key, Modifiers, Pos2, Response, Shape, Ui};
use kurbo::{Point, Rect, Size};

use super::geometry::normalized_angle;
use super::{CropBox, CropFrame, CropStyle};
use crate::canvas::claim::PrimaryPress;
use crate::canvas::transform_box::fixed;
use crate::canvas::{CanvasView, Claim, Readout};

/// The line being drawn, in screen points, kept between passes.
#[derive(Clone, Copy, Debug)]
struct Line {
    press: PrimaryPress,
    to: Pos2,
}

/// What one pass of the Straighten line gave.
pub(super) struct Pass {
    /// The line owns the current press: the frame's own gestures skip it.
    pub claimed: bool,
    /// The released line's angle.
    pub straightened: Option<f64>,
}

/// The frame angle (degrees, clockwise on the page, in -45..=45) that makes the line from `a` to
/// `b` level, or plumb when it is nearer vertical; `None` for a point or a line that is not
/// finite.
pub(super) fn level_angle(a: Point, b: Point) -> Option<f64> {
    let d = b - a;
    if !(d.x.is_finite() && d.y.is_finite()) || d.hypot() == 0.0 {
        return None;
    }
    // The line has no direction: fold it into -90..=90, then into the nearer axis.
    let mut deg = d.y.atan2(d.x).to_degrees();
    if deg > 90.0 {
        deg -= 180.0;
    } else if deg <= -90.0 {
        deg += 180.0;
    }
    if deg > 45.0 {
        deg -= 90.0;
    } else if deg <= -45.0 {
        deg += 90.0;
    }
    Some(deg + 0.0)
}

impl CropBox {
    /// The largest frame with this one's proportions that fits a canvas of `canvas_size` at
    /// `angle` degrees, centred on the canvas (the host applies it as one box step after
    /// Straighten). `None` when a size or the angle is not a positive finite number.
    pub fn fitted(&self, canvas_size: Size, angle: f64) -> Option<CropBox> {
        let r = self.rect.abs();
        let (w, h, cw, ch) = (r.width(), r.height(), canvas_size.width, canvas_size.height);
        if ![w, h, cw, ch, angle].iter().all(|v| v.is_finite()) || w <= 0.0 || h <= 0.0 || cw <= 0.0 || ch <= 0.0 {
            return None;
        }
        let (s, c) = angle.to_radians().sin_cos();
        let (s, c) = (s.abs(), c.abs());
        // The turned frame's bounds, scaled by `k`, must fit the canvas on both axes.
        let k = (cw / (w * c + h * s)).min(ch / (w * s + h * c));
        if !k.is_finite() || k <= 0.0 {
            return None;
        }
        let (hw, hh, cx, cy) = (w * k / 2.0, h * k / 2.0, cw / 2.0, ch / 2.0);
        Some(CropBox { rect: Rect::new(cx - hw, cy - hh, cx + hw, cy + hh), angle: normalized_angle(angle) })
    }
}

impl CropFrame<'_> {
    /// One pass of Straighten: a press on the canvas while it is armed starts a line, which
    /// follows the pointer and, on release, gives the angle that levels it when it is at least
    /// `crop.straighten_min` screen points long. Esc drops the line.
    pub(super) fn straighten_pass(&self, ui: &Ui, canvas: &Response, view: &CanvasView, style: &CropStyle, id: Id, busy: bool) -> Pass {
        let ctx = ui.ctx();
        let key = line_key(id);
        let mut line: Option<Line> = ctx.data(|d| d.get_temp(key)).filter(|l: &Line| l.press.holds(ctx));
        if line.is_none() && self.straighten && !busy && ui.is_enabled() {
            line = self.line_press(ctx, canvas, view, id);
        }
        let mut pass = Pass { claimed: line.is_some(), straightened: None };
        let Some(mut l) = line else {
            ctx.data_mut(|d| d.remove::<Line>(key));
            return pass;
        };
        let pointer = ctx.input(|i| i.pointer.latest_pos());
        if let Some(p) = pointer {
            l.to = p;
        }
        let dropped = ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
        let down = ctx.input(|i| i.pointer.primary_down());
        let from = l.press.origin;
        if dropped || !down {
            ctx.data_mut(|d| d.remove::<Line>(key));
            if !dropped && (l.to - from).length() >= style.straighten_min {
                pass.straightened = level_angle(view.to_doc(from), view.to_doc(l.to));
            }
            return pass;
        }
        ctx.data_mut(|d| d.insert_temp(key, l));
        ctx.set_cursor_icon(CursorIcon::Crosshair);
        self.paint_line(ui, view, style, id, from, l.to);
        pass.claimed = true;
        pass
    }

    /// A press that starts a line, claimed for the frame.
    fn line_press(&self, ctx: &Context, canvas: &Response, view: &CanvasView, id: Id) -> Option<Line> {
        if !ctx.input(|i| i.pointer.primary_pressed()) {
            return None;
        }
        let press = PrimaryPress::current(ctx)?;
        if !(view.rect.contains(press.origin) && canvas.contains_pointer()) || !Claim::take(ctx, canvas.id, id) {
            return None;
        }
        Some(Line { press, to: press.origin })
    }

    /// The line over its halo, and its angle beside the pointer.
    fn paint_line(&self, ui: &Ui, view: &CanvasView, style: &CropStyle, id: Id, from: Pos2, to: Pos2) {
        let painter = ui.painter_at(view.rect);
        painter.add(Shape::line_segment([from, to], style.straighten_halo));
        painter.add(Shape::line_segment([from, to], style.straighten));
        let (Some(labels), Some((ux, _))) = (&self.readout, self.units) else { return };
        let d = to - from;
        if d == egui::Vec2::ZERO {
            return;
        }
        // Counter-clockwise on screen, as a protractor reads, from -180 (shown as 180) to 180.
        let deg = fixed(normalized_angle(f64::from(-d.y).atan2(f64::from(d.x)).to_degrees()), 1);
        let deg = if deg == -180.0 { 180.0 } else { deg };
        let angle = format!("{}{}{}", labels.degrees.prefix(), ux.numbers.format(deg, 1..=1), labels.degrees.suffix());
        Readout::new(("craft_ui::canvas::CropFrame::straighten", id)).row(labels.angle.clone(), angle).show(ui.ctx(), to);
    }
}

fn line_key(id: Id) -> Id {
    id.with("craft_ui::canvas::CropFrame::straighten")
}

#[cfg(test)]
#[path = "straighten_tests.rs"]
mod tests;
