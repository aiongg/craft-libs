//! The brush cursor: the tip's outline as the user's cursor preferences ask for it.

use std::sync::Arc;

use egui::{Painter, Stroke, StrokeKind, Vec2};

use super::pixel_tip::pixel_cell;
use super::{BrushStyle, BrushTip, BrushTipOutline, TipPose};
use crate::canvas::{CanvasView, CursorOptions, PaintingCursor, ToolCursor};
use crate::icons::Icon;

/// The pointer of a brush tool: built each pass from the app's [`BrushTip`] and turned into a
/// [`ToolCursor`] the tool returns from [`CanvasTool::cursor`](crate::canvas::CanvasTool::cursor).
///
/// The rules, in order (Photoshop's Preferences › Cursors; PhotoCraft's `draw_canvas_tool_cursor`):
/// 1. Caps Lock with [`CursorOptions::caps_lock_precise`]: the precise crosshair
///    (`cursor.precise_reach`, no gap), whatever else is set.
/// 2. Standard: the tool's [`icon`](Self::icon), hotspot at its centre. Precise: the precise
///    crosshair.
/// 3. While [`painting`](Self::painting) with
///    [`CursorOptions::show_only_crosshair_while_painting`]: the precise crosshair (6, not
///    PhotoCraft's 5).
/// 4. A [`Pixels`](super::BrushTipOutline::Pixels) tip: [`ToolCursor::Hidden`] (the tool
///    paints the cell with [`paint_pixels`](Self::paint_pixels)), or the crosshair
///    `brush.tip_min_crosshair` with no gap when [`centre`](Self::centre) is on or the cell is
///    narrower than 6 points on screen.
/// 5. Otherwise Normal Brush Tip draws the 50 % outline (`normal_radius`, the `normal` rings),
///    Full Size Brush Tip the full reach (`full_radius`, the `full` rings), mapped through the
///    [`pose`](Self::pose) and then the view (zoom, rotation, flip, pixel aspect). A round tip
///    under an upright pose and an isotropic view is a [`ToolCursor::Circle`] (radius at least
///    1 point); under a skewed pose or a pixel aspect it is a [`ToolCursor::Outline`] ellipse
///    with the mapped axes' lengths; traced rings are a [`ToolCursor::Outline`]. With
///    [`inner_ring`](Self::inner_ring) a second ring is added.
/// 6. An outline whose farthest point is nearer than `brush.tip_min_radius` to the hotspot: the
///    crosshair `brush.tip_min_crosshair` with no gap.
///
/// The centre mark follows [`centre`](Self::centre). NaN, infinite, negative and zero sizes
/// never panic and never give a non-finite ring (they fall to rule 6).
#[derive(Clone, Debug)]
pub struct BrushCursor<'a> {
    tip: &'a BrushTip,
    pose: TipPose,
    centre: bool,
    painting: bool,
    icon: Icon,
    inner_ring: Option<f64>,
}

impl<'a> BrushCursor<'a> {
    /// The cursor of `tip`, upright, no centre mark, not painting, the Brush icon for Standard.
    pub fn new(tip: &'a BrushTip) -> Self {
        Self { tip, pose: TipPose::upright(), centre: false, painting: false, icon: Icon::Brush, inner_ring: None }
    }

    /// The pen's pose this pass ([`TipPose::pen`]); default upright.
    pub fn pose(mut self, pose: TipPose) -> Self {
        self.pose = pose;
        self
    }

    /// Whether the outline draws its centre mark: the app's rule (Show Crosshair in Brush Tip;
    /// Clone Stamp only while Alt is held; the Background Eraser always). Default off.
    pub fn centre(mut self, on: bool) -> Self {
        self.centre = on;
        self
    }

    /// Whether a stroke is being painted (for Show Only Crosshair While Painting). Default off.
    pub fn painting(mut self, on: bool) -> Self {
        self.painting = on;
        self
    }

    /// The tool's icon for the Standard painting cursor. Default [`Icon::Brush`].
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = icon;
        self
    }

    /// A second ring at `radius` document units inside the outline, mapped like it (LightCraft's
    /// feather core).
    pub fn inner_ring(mut self, radius: f64) -> Self {
        self.inner_ring = Some(radius);
        self
    }

    /// The pointer for this pass, by the rules above. `caps_lock` is
    /// [`ToolContext::caps_lock`](crate::canvas::ToolContext::caps_lock).
    pub fn cursor(&self, style: &BrushStyle, prefs: &CursorOptions, view: &CanvasView, caps_lock: bool) -> ToolCursor {
        // pc: crates/ui-egui/src/canvas.rs:1455, crates/ui-egui/src/tools/brush_cursor.rs:102 `cursor`.
        let precise = ToolCursor::Crosshair { length: style.cursor.precise_reach, gap: 0.0 };
        let min_crosshair = ToolCursor::Crosshair { length: style.tip_min_crosshair, gap: 0.0 };
        // Rules 1 and 2.
        if (caps_lock && prefs.caps_lock_precise) || matches!(prefs.painting, PaintingCursor::Standard | PaintingCursor::Precise) {
            return ToolCursor::brush(&style.cursor, prefs, 0.0, 0.0, self.icon, caps_lock);
        }
        // Rule 3.
        if self.painting && prefs.show_only_crosshair_while_painting {
            return precise;
        }
        let full = prefs.painting == PaintingCursor::FullSizeBrushTip;
        // Rule 5.
        let unit = sanitize(self.tip.diameter) / 2.0;
        let axes_for = |radius: f64| [view.map_vec(self.pose.x_axis * radius), view.map_vec(self.pose.y_axis * radius)].map(|v| Vec2::new(v.x, v.y));
        let unit_axes = axes_for(unit);
        let (rings, round) = match &self.tip.outline {
            BrushTipOutline::Round { normal_radius, full_radius } => {
                let radius = sanitize(if full { *full_radius } else { *normal_radius });
                let axes = axes_for(radius);
                (vec![ellipse(axes)], Some(axes))
            }
            BrushTipOutline::Traced { normal, full: reach } => {
                let rings = if full { reach } else { normal };
                let mapped = rings.rings().iter().map(|ring| ring.iter().map(|p| unit_axes[0] * p[0] + unit_axes[1] * p[1]).collect()).collect();
                (mapped, None)
            }
            // Rule 4. A cell narrower than the precise reach, or not finite, shows the crosshair.
            BrushTipOutline::Pixels { cell, .. } => {
                let corners = view.quad(*cell);
                let (min, max) = corners.iter().fold((corners[0], corners[0]), |(lo, hi), p| (lo.min(*p), hi.max(*p)));
                let side = (max.x - min.x).min(max.y - min.y);
                let narrow = matches!(side.partial_cmp(&style.cursor.precise_reach), None | Some(std::cmp::Ordering::Less));
                return if self.centre || narrow { min_crosshair } else { ToolCursor::Hidden };
            }
        };
        // Rule 6, also for a non-finite outline.
        let extent = match round {
            Some(axes) => semi_major(axes),
            None => rings.iter().flatten().fold(0.0f32, |m, p| m.max(p.length())),
        };
        let finite = rings.iter().flatten().all(|p| p.x.is_finite() && p.y.is_finite());
        if !finite || !extent.is_finite() || extent < style.tip_min_radius {
            return min_crosshair;
        }
        let mut rings = rings;
        if let Some(radius) = self.inner_ring {
            let inner = ellipse(axes_for(sanitize(radius)));
            if inner.iter().all(|p| p.x.is_finite() && p.y.is_finite()) {
                if let Some(axes) = round {
                    rings = vec![ellipse(axes), inner];
                } else {
                    rings.push(inner);
                }
            }
        } else if let Some([x, y]) = round {
            // A round tip stays a plain circle while the mapped axes are equal and square.
            let anisotropic = (x.length_sq() - y.length_sq()).abs() > 0.001 || x.dot(y).abs() > 0.001;
            if !anisotropic {
                return ToolCursor::Circle { radius: x.length().max(1.0), centre: self.centre };
            }
        }
        ToolCursor::Outline { rings: Arc::from(rings), centre: self.centre }
    }

    /// Paints a [`Pixels`](super::BrushTipOutline::Pixels) tip's cell on the pixel grid: its
    /// screen rect through `view` with every edge rounded to a physical pixel
    /// (`painter.ctx().pixels_per_point()`), then, one physical pixel wide, the cursor halo just
    /// outside and the cursor line just inside: a square cell as two rect strokes, a round one as
    /// two circles (`r + px / 2` and `r − px / 2`). Nothing for another outline or a non-finite
    /// cell. The tool calls it from [`CanvasTool::paint`](crate::canvas::CanvasTool::paint).
    pub fn paint_pixels(&self, painter: &Painter, view: &CanvasView, style: &BrushStyle) {
        // pc: crates/ui-egui/src/canvas.rs:1475-1496, crates/ui-egui/src/canvas/cursors.rs:19.
        let BrushTipOutline::Pixels { cell, round } = &self.tip.outline else { return };
        if ![cell.x0, cell.y0, cell.x1, cell.y1].iter().all(|v| v.is_finite()) {
            return;
        }
        let ppp = painter.ctx().pixels_per_point();
        let ppp = if ppp.is_finite() && ppp > 0.0 { ppp } else { 1.0 };
        let px = 1.0 / ppp;
        let rect = pixel_cell(view, *cell, ppp);
        let (halo, line) = (Stroke::new(px, style.cursor.halo), Stroke::new(px, style.cursor.line));
        if *round {
            let r = rect.width() / 2.0;
            painter.circle_stroke(rect.center(), r + px / 2.0, halo);
            painter.circle_stroke(rect.center(), (r - px / 2.0).max(0.0), line);
        } else {
            painter.rect_stroke(rect, 0.0, halo, StrokeKind::Outside);
            painter.rect_stroke(rect, 0.0, line, StrokeKind::Inside);
        }
    }
}

/// `v` if it is a finite size, else 0; negative sizes count as 0.
fn sanitize(v: f64) -> f64 {
    if v.is_finite() { v.max(0.0) } else { 0.0 }
}

/// Points of the ellipse with semi-axis vectors `axes`.
fn ellipse(axes: [Vec2; 2]) -> Vec<Vec2> {
    const SEGMENTS: usize = 64;
    (0..SEGMENTS)
        .map(|i| {
            let (sin, cos) = (std::f32::consts::TAU * i as f32 / SEGMENTS as f32).sin_cos();
            axes[0] * cos + axes[1] * sin
        })
        .collect()
}

/// The farthest distance from the centre of the ellipse with semi-axis vectors `axes`.
fn semi_major([x, y]: [Vec2; 2]) -> f32 {
    let (a, b, d) = (x.length_sq(), y.length_sq(), x.dot(y));
    ((a + b) / 2.0 + (((a - b) / 2.0).powi(2) + d * d).sqrt()).sqrt()
}
