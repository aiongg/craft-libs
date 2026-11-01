//! The brush cursor: the tip's outline as the user's cursor preferences ask for it.
#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::Painter;

use super::{BrushStyle, BrushTip, TipPose};
use crate::canvas::{CanvasView, CursorOptions, ToolCursor};
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
        // pc: crates/ui-egui/src/canvas.rs:1455 (the painting-cursor match: ⇪, Standard, Precise,
        // painting && show_only_crosshair_while_painting, Pencil cell, outline);
        // crates/ui-egui/src/tools/brush_cursor.rs:102 `cursor` (anisotropic axes → ellipse; extent < 3 →
        // crosshair 4/0; circle radius max 1), :95 `on_screen`, :48 `tip_axes`. Rules 1–2 are
        // `ToolCursor::brush`'s.
        todo!()
    }

    /// Paints a [`Pixels`](super::BrushTipOutline::Pixels) tip's cell on the pixel grid: its
    /// screen rect through `view` with every edge rounded to a physical pixel
    /// (`painter.ctx().pixels_per_point()`), then, one physical pixel wide, the cursor halo just
    /// outside and the cursor line just inside: a square cell as two rect strokes, a round one as
    /// two circles (`r + px / 2` and `r − px / 2`). Nothing for another outline or a non-finite
    /// cell. The tool calls it from [`CanvasTool::paint`](crate::canvas::CanvasTool::paint).
    pub fn paint_pixels(&self, painter: &Painter, view: &CanvasView, style: &BrushStyle) {
        // pc: crates/ui-egui/src/canvas.rs:1475-1496 (dark α 160 Outside and light α 230 Inside,
        // 1 / ppp wide); crates/ui-egui/src/canvas/cursors.rs:19 `pencil_cursor_rect` (snap to 1 / ppp).
        todo!()
    }
}
