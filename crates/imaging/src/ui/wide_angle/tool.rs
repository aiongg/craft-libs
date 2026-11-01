//! [`ConstraintTool`]: Adaptive Wide Angle's Constraint tool (task WS-WA1).

use egui::{Key, Modifiers, Painter, Pos2, Shape, Stroke, vec2};

use craft_ui::behavior::nudge::NudgeKeys;
use craft_ui::canvas::kurbo::Point;
use craft_ui::canvas::{CanvasTool, CursorHost, Part, ToolAction, ToolContext, ToolCursor, ToolKey, ToolParts, ToolPointer};

use super::WideAngleLabels;
use super::session::{ConstraintOrientation, WideAngleConstraint, WideAngleConstraintEdit, WideAngleGeometry, WideAngleSession};
use super::style::WideAngleStyle;

/// Segments of the curve a constraint is drawn as (PhotoCraft's `Camera::arc(a, b, 24)`).
const ARC_SEGMENTS: usize = 24;

/// The state of the Constraint tool between passes: the drag in progress, the constraint picked
/// for the keyboard, the constraint a right-click opened the menu for, the edits not yet taken.
#[derive(Debug, Default)]
pub struct ConstraintTool {
    /// The drag's two ends in canvas units.
    drag: Option<(Point, Point)>,
    picked: Option<usize>,
    menu_for: Option<usize>,
    /// Where the right-click that chose `menu_for` landed, until the workspace opens the menu.
    menu_at: Option<Pos2>,
    edits: Vec<WideAngleConstraintEdit>,
}

impl ConstraintTool {
    pub fn new() -> Self {
        Self::default()
    }

    /// The tool for this pass over `session` (read only: edits are reported) and the app's
    /// geometry.
    pub fn pass<'a>(
        &'a mut self,
        session: &'a WideAngleSession,
        geometry: &'a dyn WideAngleGeometry,
        style: &'a WideAngleStyle,
        labels: &'a WideAngleLabels,
    ) -> ConstraintToolPass<'a> {
        ConstraintToolPass { tool: self, session, geometry, style, labels, canvas: None, scale: [1.0, 1.0], corrected: false }
    }

    pub fn take_edits(&mut self) -> Vec<WideAngleConstraintEdit> {
        std::mem::take(&mut self.edits)
    }

    /// A drag is drawing a constraint: the canvas shows the source, not the corrected preview, and
    /// the history waits.
    pub fn dragging(&self) -> bool {
        self.drag.is_some()
    }

    /// The constraint the keyboard acts on.
    pub fn picked(&self) -> Option<usize> {
        self.picked
    }

    /// The constraint the context menu is open for (the workspace draws the menu: Free,
    /// Horizontal, Vertical as radio items, then Delete).
    pub fn menu_for(&self) -> Option<usize> {
        self.menu_for
    }

    /// Where the menu should open, once: the right-click that chose [`menu_for`](Self::menu_for).
    pub fn take_menu_at(&mut self) -> Option<Pos2> {
        self.menu_at.take()
    }

    /// The menu closed (picked or dismissed).
    pub fn close_menu(&mut self) {
        self.menu_for = None;
        self.menu_at = None;
    }

    /// The session has `count` constraints after the edits of this pass or an undo: a picked
    /// constraint or a menu that points past the end is dropped, and so is a drag after an undo.
    pub fn sync(&mut self, count: usize) {
        self.picked = self.picked.filter(|&i| i < count);
        if self.menu_for.is_some_and(|i| i >= count) {
            self.close_menu();
        }
    }

    /// Drops the drag in progress (a tool change, an undo).
    pub fn drop_drag(&mut self) {
        self.drag = None;
    }
}

/// [`ConstraintTool`] as the `CanvasTool` of one pass (A).
///
/// - A drag draws a constraint from the press to the pointer (a 2 pt line in the accent while it
///   runs); on release it is added when longer than 6 screen points
///   ([`WideAngleConstraint::from_drag`](super::WideAngleConstraint::from_drag) with Shift at
///   the release).
/// - A secondary click opens the menu for the constraint whose curve passes nearest the click
///   within `wide_angle.hit` (10 points): `menu_for` is set; elsewhere nothing.
/// - Keyboard parts: each constraint is a part ("Constraint 2, Horizontal"); Page picks, arrows
///   move it one document pixel (Shift ten) as `WideAngleConstraintEdit::Move`, Delete and
///   Backspace delete it.
/// - Paint: each constraint as its curve ([`WideAngleGeometry::arc`], 24 segments; through
///   [`WideAngleGeometry::corrected`] over the corrected preview), `wide_angle.line` (2) in the
///   orientation's colour (accent, `status.warn.fg`, `status.bad.fg`) over the `.halo`, with end
///   dots of `wide_angle.end` (3.5).
pub struct ConstraintToolPass<'a> {
    tool: &'a mut ConstraintTool,
    session: &'a WideAngleSession,
    geometry: &'a dyn WideAngleGeometry,
    style: &'a WideAngleStyle,
    labels: &'a WideAngleLabels,
    /// The canvas image's size in canvas units; a press outside it is not the tool's.
    canvas: Option<[f64; 2]>,
    /// Document pixels per canvas unit, per axis.
    scale: [f64; 2],
    /// The canvas shows the corrected preview: the curves go through
    /// [`WideAngleGeometry::corrected`].
    corrected: bool,
}

impl ConstraintToolPass<'_> {
    /// The canvas image is `canvas` pixels and the document `document` pixels: the pointer and
    /// the curves convert between them. Default: the same.
    pub fn image(mut self, canvas: [u32; 2], document: [u32; 2]) -> Self {
        let ratio = |document: u32, canvas: u32| if canvas == 0 { 1.0 } else { f64::from(document) / f64::from(canvas) };
        self.canvas = Some([f64::from(canvas[0]), f64::from(canvas[1])]);
        self.scale = [ratio(document[0], canvas[0]), ratio(document[1], canvas[1])];
        self
    }

    /// Whether the canvas shows the corrected preview (default: the source).
    pub fn corrected(mut self, on: bool) -> Self {
        self.corrected = on;
        self
    }

    fn to_document(&self, p: Point) -> Point {
        Point::new(p.x * self.scale[0], p.y * self.scale[1])
    }

    /// The curve of the scene line `a` to `b` in canvas units: the lens arc, through the solved
    /// mesh while the corrected preview shows (the unmapped arc while no solve is ready).
    fn curve(&self, a: [f64; 2], b: [f64; 2]) -> Vec<Point> {
        let arc = self.geometry.arc(self.session, a, b, ARC_SEGMENTS);
        let mapped = self.corrected.then(|| arc.iter().map(|&q| self.geometry.corrected(self.session, q)).collect::<Option<Vec<_>>>()).flatten();
        mapped.unwrap_or(arc).into_iter().map(|q| Point::new(q.x / self.scale[0], q.y / self.scale[1])).collect()
    }

    /// The constraint whose curve passes nearest `at` (screen) within `wide_angle.hit`.
    fn nearest(&self, cx: &ToolContext<'_>, at: Pos2) -> Option<usize> {
        // pc: crates/ui-egui/src/dialogs/wide_angle_ui.rs:310-322 (nearest vertex of the 24-segment
        // arc; here the nearest point of the drawn segments, so a click between vertices hits).
        self.session
            .constraints
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let points: Vec<Pos2> = self.curve(c.a, c.b).into_iter().map(|q| cx.view.to_screen(q)).collect();
                let near = match points.as_slice() {
                    [only] => only.distance(at),
                    _ => points.windows(2).map(|w| segment_distance(at, w[0], w[1])).fold(f32::MAX, f32::min),
                };
                (i, near)
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .filter(|&(_, near)| near < self.style.hit)
            .map(|(i, _)| i)
    }

    fn orientation_color(&self, orientation: ConstraintOrientation) -> egui::Color32 {
        match orientation {
            ConstraintOrientation::Free => self.style.free,
            ConstraintOrientation::Horizontal => self.style.horizontal,
            ConstraintOrientation::Vertical => self.style.vertical,
        }
    }

    fn orientation_label(&self, orientation: ConstraintOrientation) -> &str {
        match orientation {
            ConstraintOrientation::Free => &self.labels.free,
            ConstraintOrientation::Horizontal => &self.labels.horizontal,
            ConstraintOrientation::Vertical => &self.labels.vertical,
        }
    }

    /// `points` with the halo under it, as a polyline.
    fn line(&self, painter: &Painter, points: Vec<Pos2>, color: egui::Color32) {
        painter.add(Shape::line(points.clone(), Stroke::new(self.style.line + self.style.halo_width, self.style.halo)));
        painter.add(Shape::line(points, Stroke::new(self.style.line, color)));
    }
}

/// The distance from `p` to the segment `a`..`b`.
fn segment_distance(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let length_squared = ab.length_sq();
    if length_squared <= f32::EPSILON {
        return p.distance(a);
    }
    let t = ((p - a).dot(ab) / length_squared).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

impl CanvasTool for ConstraintToolPass<'_> {
    fn cursor(&self, cx: &ToolContext<'_>) -> ToolCursor {
        // new: the crosshair (PhotoCraft shows the default arrow).
        let (length, gap) = CursorHost::style(cx.tokens).crosshair;
        ToolCursor::Crosshair { length, gap }
    }

    fn press(&mut self, pointer: &ToolPointer, _cx: &mut ToolContext<'_>) -> ToolAction {
        // pc: crates/ui-egui/src/dialogs/wide_angle_ui.rs:289-290 (drag start). PhotoCraft's
        // interaction rect is the image, so a press on the pasteboard is not the tool's.
        let inside = self.canvas.is_none_or(|[w, h]| (0.0..=w).contains(&pointer.doc.x) && (0.0..=h).contains(&pointer.doc.y));
        if !inside {
            return ToolAction::Ignored;
        }
        self.tool.drag = Some((pointer.doc, pointer.doc));
        ToolAction::Claimed
    }

    fn drag(&mut self, pointer: &ToolPointer, _cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/dialogs/wide_angle_ui.rs:291-295.
        if let Some(drag) = self.tool.drag.as_mut() {
            drag.1 = pointer.doc;
        }
    }

    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/dialogs/wide_angle_ui.rs:297-308 (over 6 px; Shift orients).
        let Some((a, _)) = self.tool.drag.take() else { return };
        let b = pointer.doc;
        if cx.view.to_screen(a).distance(cx.view.to_screen(b)) > self.style.min_drag {
            let constraint = WideAngleConstraint::from_drag(self.to_document(a), self.to_document(b), pointer.modifiers.shift);
            self.tool.edits.push(WideAngleConstraintEdit::Add(constraint));
        } else if let Some(index) = self.nearest(cx, pointer.screen) {
            // new: a click on a constraint picks it for the keyboard.
            self.tool.picked = Some(index);
        }
    }

    fn each_pass(&mut self, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/dialogs/wide_angle_ui.rs:310-322 (a secondary click on
        // `cx.canvas` opens the menu for the constraint whose curve passes within 10 px).
        if !cx.canvas.secondary_clicked() {
            return;
        }
        let Some(at) = cx.canvas.interact_pointer_pos() else { return };
        match self.nearest(cx, at) {
            Some(index) => {
                self.tool.menu_for = Some(index);
                self.tool.menu_at = Some(at);
            }
            None => self.tool.close_menu(),
        }
    }

    fn cancel(&mut self, _cx: &mut ToolContext<'_>) {
        // new: Esc or a lost pointer drops the drag.
        self.tool.drag = None;
    }

    fn key(&mut self, key: &ToolKey, _cx: &mut ToolContext<'_>) -> bool {
        // new: arrows move and Delete removes the picked constraint (lead call 49).
        let count = self.session.constraints.len();
        let Some(index) = self.tool.picked.filter(|&i| i < count) else { return false };
        if !key.pressed || self.tool.drag.is_some() {
            return false;
        }
        if matches!(key.key, Key::Delete | Key::Backspace) && key.modifiers.matches_exact(Modifiers::NONE) {
            self.tool.edits.push(WideAngleConstraintEdit::Delete(index));
            self.tool.picked = (count > 1).then(|| index.min(count - 2));
            return true;
        }
        let Some(nudge) = NudgeKeys::new().from_key(key) else { return false };
        self.tool.edits.push(WideAngleConstraintEdit::Move { index, by: [nudge.by.x, nudge.by.y] });
        true
    }

    fn paint(&self, painter: &Painter, cx: &ToolContext<'_>) {
        // pc: crates/ui-egui/src/dialogs/wide_angle_ui.rs:331-356 (curves, colours, end dots).
        let screen = |points: Vec<Point>| -> Vec<Pos2> { points.into_iter().map(|q| cx.view.to_screen(q)).collect() };
        for constraint in &self.session.constraints {
            let points = screen(self.curve(constraint.a, constraint.b));
            let color = self.orientation_color(constraint.orientation);
            let ends = [points.first().copied(), points.last().copied()];
            self.line(painter, points, color);
            for end in ends.into_iter().flatten() {
                painter.circle_filled(end, self.style.end + self.style.halo_width / 2.0, self.style.halo);
                painter.circle_filled(end, self.style.end, color);
            }
        }
        if let Some((a, b)) = self.tool.drag {
            let (a, b) = (self.to_document(a), self.to_document(b));
            let points = screen(
                self.geometry
                    .arc(self.session, [a.x, a.y], [b.x, b.y], ARC_SEGMENTS)
                    .into_iter()
                    .map(|q| Point::new(q.x / self.scale[0], q.y / self.scale[1]))
                    .collect(),
            );
            self.line(painter, points, self.style.free);
        }
    }

    fn parts(&self, cx: &ToolContext<'_>) -> Option<ToolParts> {
        // new: constraints as keyboard parts (lead call 49).
        if self.session.constraints.is_empty() {
            return None;
        }
        let size = 2.0 * self.style.hit;
        let parts = self
            .session
            .constraints
            .iter()
            .enumerate()
            .map(|(index, constraint)| {
                let curve = self.curve(constraint.a, constraint.b);
                let middle = curve.get(curve.len() / 2).map_or(Pos2::ZERO, |&q| cx.view.to_screen(q));
                let name = craft_i18n::fmt(
                    &self.labels.constraint_part,
                    &[("n", &cx.numbers.format(index as f64 + 1.0, 0..=0)), ("orientation", self.orientation_label(constraint.orientation))],
                );
                Part {
                    key: index as u64,
                    name,
                    value: None,
                    rect: egui::Rect::from_center_size(middle, vec2(size, size)),
                    selected: self.tool.picked == Some(index),
                }
            })
            .collect();
        Some(ToolParts::new(self.labels.constraint_lines.clone(), parts))
    }

    fn picked(&self) -> Option<u64> {
        self.tool.picked.map(|i| i as u64)
    }

    fn pick(&mut self, key: Option<u64>, _cx: &mut ToolContext<'_>) {
        self.tool.picked = key.and_then(|k| usize::try_from(k).ok());
    }
}
