//! Straighten: a line drawn along something that should be level or plumb gives the angle that
//! levels it (PhotoCraft's `tools/crop_straighten.rs`).

use egui::{Context, CursorIcon, Id, Key, Modifiers, Painter, Pos2, Response, Shape, Ui};
use kurbo::{Point, Rect, Size};

use super::geometry::normalized_angle;
use super::{CropBox, CropFrame, CropLabels, CropStyle};
use crate::ui::transform::transform_box::fixed;
use craft_ui::canvas::claim::PrimaryPress;
use craft_ui::canvas::quad::BoxPart;
use craft_ui::canvas::{CanvasView, Claim, Readout, RulerUnits};

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

impl CropBox {
    /// The frame angle (degrees, clockwise on the page, in -45..=45) that makes the line from `a` to
    /// `b` level, or plumb when it is nearer vertical; `None` for a point or a line that is not
    /// finite.
    pub fn level_angle(a: Point, b: Point) -> Option<f64> {
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
        let mods = self.mods(ctx.input(|i| i.modifiers));
        if let Some(p) = pointer {
            l.to = self.snap_point(view, p, mods);
        }
        let dropped = ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
        let down = ctx.input(|i| i.pointer.primary_down());
        let from = self.snap_point(view, l.press.origin, mods);
        if dropped || !down {
            ctx.data_mut(|d| d.remove::<Line>(key));
            if !dropped {
                pass.straightened = Self::line_angle(view, style, from, l.to);
            }
            return pass;
        }
        ctx.data_mut(|d| d.insert_temp(key, l));
        ctx.set_cursor_icon(CursorIcon::Crosshair);
        self.paint_line(&ui.painter_at(view.rect), style, id, from, l.to, self.readout_values());
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

    /// Canvas point `p` moved by the host's [`snap`](Self::snap), for an end point of a Straighten
    /// line. The point is the same gesture a frame corner is: the snap sees a zero-size frame at
    /// the point and its bottom-right corner ([`BoxPart::Corner`] 2), so it moves both axes onto
    /// its targets.
    pub(super) fn snap_point(&self, view: &CanvasView, p: Pos2, mods: Modifiers) -> Pos2 {
        let Some(snap) = self.snap.as_ref() else { return p };
        let at = view.to_doc(p);
        let by = snap(BoxPart::Corner(2), &CropBox::upright(Rect::from_points(at, at)), mods);
        if by.x.is_finite() && by.y.is_finite() { view.to_screen(at + by) } else { p }
    }

    /// The angle a Straighten line from canvas point `from` to `to` gives, when it is at least
    /// `crop.straighten_min` screen points long.
    pub(super) fn line_angle(view: &CanvasView, style: &CropStyle, from: Pos2, to: Pos2) -> Option<f64> {
        if (to - from).length() < style.straighten_min {
            return None;
        }
        CropBox::level_angle(view.to_doc(from), view.to_doc(to))
    }

    /// The line over its halo, and its angle beside the pointer.
    pub(super) fn paint_line(&self, painter: &Painter, style: &CropStyle, id: Id, from: Pos2, to: Pos2, readout: Option<(&CropLabels, &RulerUnits)>) {
        painter.add(Shape::line_segment([from, to], style.straighten_halo));
        painter.add(Shape::line_segment([from, to], style.straighten));
        let Some((labels, ux)) = readout else { return };
        let d = to - from;
        if d == egui::Vec2::ZERO {
            return;
        }
        // Counter-clockwise on screen, as a protractor reads, from -180 (shown as 180) to 180.
        let deg = fixed(normalized_angle(f64::from(-d.y).atan2(f64::from(d.x)).to_degrees()), 1);
        let deg = if deg == -180.0 { 180.0 } else { deg };
        let angle = format!("{}{}{}", labels.degrees.prefix(), ux.numbers.format(deg, 1..=1), labels.degrees.suffix());
        Readout::new(("craft_ui::canvas::CropFrame::straighten", id)).row(labels.angle.clone(), angle).show(painter.ctx(), to);
    }
}

fn line_key(id: Id) -> Id {
    id.with("craft_ui::canvas::CropFrame::straighten")
}

#[cfg(test)]
#[path = "straighten_tests.rs"]
mod tests;
