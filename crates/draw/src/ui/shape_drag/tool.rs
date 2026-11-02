//! [`ShapeDragTool`]: the Shape tools' press, drag and release on the [`ToolHost`](craft_ui::canvas::ToolHost)
//! (task SH-T3).

use egui::{Key, Modifiers, Painter, Pos2, Shape};
use kurbo::{PathEl, Point, Rect, Vec2};

use super::{CreateShapeLabels, ShapeDragEvent, ShapeDragOptions, ShapeDragRules, ShapeDragToolLabels, ShapeFrame, ShapeKind};
use crate::geometry::shape::{RadialFrame, SIDES, ShapeGeometry, box_drag, line_drag};
use craft_ui::canvas::{CanvasTool, CursorHost, Readout, SnapGesture, ToolAction, ToolContext, ToolCursor, ToolKey, ToolNode, ToolPointer};
use craft_ui::edit::{CancelReason, EditEvent, GestureId};
use craft_ui::localized::Labels as _;

/// A Shape tool on the canvas: drag to draw, click to ask for a size. The app owns one value for
/// all Shape tools, sets [`options`](Self::options) (the kind from the selected tool) each pass,
/// hands it to `ToolHost::show` and drains [`take_events`](Self::take_events) after. The tool
/// never changes the document.
///
/// Pointer (Box frame, Photoshop's rules):
/// - The press starts at the snapped point (`SnapGesture::Point`); past the click slop
///   ([`ShapeDragStyle::click_slop`](super::ShapeDragStyle::click_slop) screen points, over the
///   zoom) the drag begins and each pass reports the shape it would draw; release commits it.
/// - A side under 1 px commits nothing: `Commit { changed: false }`.
/// - A press and release inside the slop is a [`Click`](ShapeDragEvent::Click) at the press.
/// - ⇧ makes a square or circle (a Line: 45° steps keeping its length); ⌥ draws from the centre.
///   Both may go down or up mid-drag; the shape follows the modifiers held now.
/// - Space held moves the shape being drawn without resizing it: the press point moves with the
///   pointer while Space is down, and the box snaps as a rectangle (`SnapGesture::Rect`).
/// - Esc cancels (the host calls [`cancel`](CanvasTool::cancel)); so does `ToolHost::cancel` on a
///   tool switch: `Cancel` goes out when a drag had begun.
///
/// Radial frame ([`ShapeDragRules::ILLUSTRATOR`] for Polygon and Star): the press is the centre;
/// ↑ and ↓ change the sides or points (3 to 100) while the button is down; Ctrl/Cmd holds a
/// star's inner radius; ⌥ straightens a star's shoulders (five points or more). ⇧ keeps the base
/// orientation.
///
/// Keyboard: with the canvas focused, Enter reports a `Click` at the view centre (the keyboard
/// route to Create Shape). The tool has no parts and no Tab stop of its own.
///
/// Paint: the outline of the shape being drawn, through the view, as a line over a halo
/// ([`ShapeDragStyle`](super::ShapeDragStyle)); none when [`preview`](Self::preview) is off (the
/// app previews the shape in the document from `Update` events).
#[derive(Debug, Default)]
pub struct ShapeDragTool {
    options: ShapeDragOptions,
    rules: ShapeDragRules,
    /// The tool paints the outline; on by default.
    preview: Option<bool>,
    labels: Option<ShapeDragToolLabels>,
    events: Vec<ShapeDragEvent>,
    drag: Option<Drag>,
}

/// A drag between passes.
#[derive(Clone, Debug)]
struct Drag {
    id: GestureId,
    /// The options of the press: a drag keeps its kind and values.
    options: ShapeDragOptions,
    /// The press as the pointer made it; the click slop is measured from here.
    origin: Point,
    /// The press, moved by Space.
    start: Point,
    /// The last pointer.
    at: Point,
    /// Past the click slop: `Begin` went out.
    began: bool,
    /// Where the pointer was when Space went down, while it is held.
    space_from: Option<Point>,
    /// The shape's box when Space went down, for the snap of the moving rectangle.
    space_rect: Option<Rect>,
    /// A radial star's inner radius held by Ctrl/Cmd.
    inner: Option<f64>,
    /// Sides or points as changed by ↑ and ↓ during this drag.
    sides: u32,
    /// The shape of the last `Update`, for the commit's `changed` and the paint.
    shape: Option<ShapeGeometry>,
}

impl ShapeDragTool {
    /// A tool with Photoshop's rules and default options.
    pub fn new() -> Self {
        Self::default()
    }

    /// The kind and options of this pass. A drag in progress keeps the kind it started with.
    pub fn options(&mut self, options: ShapeDragOptions) -> &mut Self {
        self.options = options;
        self
    }

    /// Which product's rules the drag follows; Photoshop's by default.
    pub fn rules(&mut self, rules: ShapeDragRules) -> &mut Self {
        self.rules = rules;
        self
    }

    /// Whether the tool paints the outline being drawn; on by default.
    pub fn preview(&mut self, on: bool) -> &mut Self {
        self.preview = Some(on);
        self
    }

    /// The words; the installed ones by default.
    pub fn labels(&mut self, labels: ShapeDragToolLabels) -> &mut Self {
        self.labels = Some(labels);
        self
    }

    /// The kind this pass draws (the options bar shows its fields).
    pub fn kind(&self) -> ShapeKind {
        self.options.kind
    }

    /// The events since the last call, in order.
    pub fn take_events(&mut self) -> Vec<ShapeDragEvent> {
        std::mem::take(&mut self.events)
    }

    /// Whether the drag draws a Polygon or Star from its centre.
    fn radial(&self, kind: ShapeKind) -> bool {
        self.rules.frame(kind) == ShapeFrame::Radial
    }

    /// The shape a drag from `start` to `at` with `modifiers` draws under the options and rules,
    /// or `None` when a side is under 1 px: [`box_drag`](crate::geometry::shape::box_drag) for
    /// the box kinds (Rectangle with four equal radii when `corner_radius` > 0, Rounded Rectangle
    /// always with them, Triangle a 3-sided polygon, Polygon and Star with the options' sides and
    /// ratio, Custom a `Custom` box keeping proportions with ⇧),
    /// [`line_drag`](crate::geometry::shape::line_drag) for a Line (weight at least 1), and
    /// [`RadialFrame::drag`](crate::geometry::shape::RadialFrame::drag) in the radial frame.
    fn shape(&self, drag: &Drag, modifiers: Modifiers) -> Option<ShapeGeometry> {
        // pc: crates/ui-egui/src/tools/vector_ui/shapes.rs:50-82 `shape_geometry`.
        // inv-vectorcraft: crates/tools/src/shape.rs:131-145 (the radial kinds).
        let o = &drag.options;
        let radii = [o.corner_radius.max(0.0); 4];
        if o.kind == ShapeKind::Line {
            let to = line_drag(drag.start, drag.at, modifiers.shift)?;
            return Some(ShapeGeometry::Line { from: drag.start, to, weight: o.line_weight.max(1.0) });
        }
        if self.radial(o.kind) {
            let frame = RadialFrame::drag(drag.start, drag.at, modifiers.shift)?;
            let star_ratio = if o.kind == ShapeKind::Star {
                match drag.inner {
                    Some(inner) => frame.ratio_holding(inner),
                    None => (self.rules.straight_shoulders && modifiers.alt)
                        .then(|| RadialFrame::straight_shoulders(drag.sides))
                        .flatten()
                        .unwrap_or(o.star_ratio),
                }
            } else {
                1.0
            };
            return Some(frame.geometry(drag.sides, star_ratio));
        }
        let rect = box_drag(drag.start, drag.at, modifiers.shift, modifiers.alt)?;
        Some(match o.kind {
            ShapeKind::Rectangle if o.corner_radius > 0.0 => ShapeGeometry::Rect { rect, radii },
            ShapeKind::Rectangle => ShapeGeometry::Rect { rect, radii: [0.0; 4] },
            ShapeKind::RoundedRectangle => ShapeGeometry::Rect { rect, radii },
            ShapeKind::Ellipse => ShapeGeometry::Ellipse { rect },
            ShapeKind::Triangle => ShapeGeometry::Polygon { rect, sides: 3, star_ratio: 1.0 },
            ShapeKind::Polygon => ShapeGeometry::Polygon { rect, sides: drag.sides, star_ratio: 1.0 },
            ShapeKind::Star => ShapeGeometry::Polygon { rect, sides: drag.sides, star_ratio: o.star_ratio },
            ShapeKind::Custom | ShapeKind::Line => ShapeGeometry::Custom { rect, keep_proportions: modifiers.shift },
        })
    }

    /// What a commit carries when the drag is under a pixel: an empty shape of the kind, with
    /// `changed` false.
    fn empty(drag: &Drag) -> ShapeGeometry {
        if drag.options.kind == ShapeKind::Line {
            ShapeGeometry::Line { from: drag.start, to: drag.at, weight: drag.options.line_weight.max(1.0) }
        } else {
            ShapeGeometry::Rect { rect: Rect::from_points(drag.start, drag.at), radii: [0.0; 4] }
        }
    }

    /// Takes the pointer at `at` with `modifiers`: begins the drag past the click slop, moves the
    /// press with Space, and reports the shape when it changed.
    fn update(&mut self, at: Point, modifiers: Modifiers, cx: &mut ToolContext<'_>) {
        let Some(mut drag) = self.drag.take() else { return };
        if !at.is_finite() {
            self.drag = Some(drag);
            return;
        }
        if drag.space_from.is_some() {
            drag.start += space_move(drag.at, at);
        }
        drag.at = at;
        if !drag.began {
            if at.distance(drag.origin) <= cx.view.doc_len(Self::style(cx.tokens).click_slop) {
                self.drag = Some(drag);
                return;
            }
            drag.began = true;
            self.events.push(ShapeDragEvent::Gesture(EditEvent::begin(drag.id)));
        }
        // Ctrl/Cmd holds the star's inner radius as it was when the key went down.
        let holds = self.rules.hold_inner && modifiers.command && drag.options.kind == ShapeKind::Star && self.radial(ShapeKind::Star);
        if !holds {
            drag.inner = None;
        } else if drag.inner.is_none() {
            drag.inner = match &drag.shape {
                Some(ShapeGeometry::RadialPolygon { radius, star_ratio, .. }) => Some(radius * star_ratio),
                _ => RadialFrame::drag(drag.start, at, modifiers.shift).map(|f| f.radius * drag.options.star_ratio),
            };
        }
        let shape = self.shape(&drag, modifiers);
        if let Some(shape) = &shape {
            if drag.shape.as_ref() != Some(shape) {
                self.events.push(ShapeDragEvent::Gesture(EditEvent::update(drag.id, shape.clone(), modifiers)));
            }
            let size = shape.bounds();
            let labels = CreateShapeLabels::of_ctx(&cx.canvas.ctx);
            let (w, h) = (cx.numbers.format(size.width(), 0..=0), cx.numbers.format(size.height(), 0..=0));
            cx.readout(Readout::new("craft_draw::shape_drag").row(labels.width, w).row(labels.height, h));
        }
        drag.shape = shape;
        self.drag = Some(drag);
    }
}

impl CanvasTool for ShapeDragTool {
    fn cursor(&self, cx: &ToolContext<'_>) -> ToolCursor {
        // new: the crosshair (`ToolCursor::Crosshair` at the cursor style's length and gap), as the
        // marquee tools show it.
        let (length, gap) = CursorHost::style(cx.tokens).crosshair;
        ToolCursor::Crosshair { length, gap }
    }

    fn press(&mut self, pointer: &ToolPointer, _cx: &mut ToolContext<'_>) -> ToolAction {
        // pc: crates/ui-egui/src/canvas/gesture.rs:84 (a shape drag that stays inside
        // `shape_dialog::is_click` opens the dialog).
        // new: claims the press, starts a `Drag` with a new `GestureId`, emits nothing yet.
        let options = self.options;
        self.drag = Some(Drag {
            id: GestureId::next(),
            options,
            origin: pointer.doc,
            start: pointer.doc,
            at: pointer.doc,
            began: false,
            space_from: None,
            space_rect: None,
            inner: None,
            sides: options.sides.clamp(*SIDES.start(), *SIDES.end()),
            shape: None,
        });
        ToolAction::Claimed
    }

    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/vector_ui/shapes.rs:85-96 (Space moves the box, which snaps).
        // new: past `click_slop / zoom` emits `Begin` then `Update` with `shape`; Space moves
        // `start` by the pointer's move; a readout of W and H through `cx.readout`.
        self.update(pointer.doc, pointer.modifiers, cx);
    }

    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/vector_ui/shapes.rs:99-112 `finish_shape`;
        // crates/ui-egui/src/dialogs/shape_dialog.rs:50-53 `is_click`.
        // new: `Commit { changed: shape.is_some() }` after a begun drag, else `Click`.
        self.update(pointer.doc, pointer.modifiers, cx);
        let Some(drag) = self.drag.take() else { return };
        let event = if drag.began {
            let changed = drag.shape.is_some();
            let value = drag.shape.clone().unwrap_or_else(|| Self::empty(&drag));
            ShapeDragEvent::Gesture(EditEvent::commit(drag.id, value, changed, pointer.modifiers))
        } else {
            ShapeDragEvent::Click { at: drag.origin, kind: drag.options.kind, modifiers: pointer.modifiers }
        };
        self.events.push(event);
    }

    fn cancel(&mut self, reason: CancelReason, _cx: &mut ToolContext<'_>) {
        // new: `Cancel` with the reason given when a drag had begun; drops the drag.
        if let Some(drag) = self.drag.take()
            && drag.began
        {
            self.events.push(ShapeDragEvent::Gesture(EditEvent::cancel(drag.id, reason)));
        }
    }

    fn key(&mut self, key: &ToolKey, cx: &mut ToolContext<'_>) -> bool {
        // inv-vectorcraft: crates/tools/src/shape.rs:5-7 (↑/↓ change the count during a drag).
        // new: Enter without a drag reports `Click` at the view centre (keyboard route, lead call
        // 11); Space and the modifiers re-run `drag` with the last pointer.
        match key.key {
            Key::Space => {
                let Some(drag) = self.drag.as_mut() else { return false };
                if !key.pressed {
                    drag.space_from = None;
                    drag.space_rect = None;
                } else if drag.space_from.is_none() {
                    drag.space_from = Some(drag.at);
                    drag.space_rect = Some(drag.shape.as_ref().map_or_else(|| Rect::from_points(drag.start, drag.at), ShapeGeometry::bounds));
                }
                true
            }
            Key::ArrowUp | Key::ArrowDown if key.pressed && self.rules.arrow_sides => {
                let Some(drag) = self.drag.as_mut() else { return false };
                if !matches!(drag.options.kind, ShapeKind::Polygon | ShapeKind::Star) || !self.rules.radial_polygons {
                    return false;
                }
                let sides = if key.key == Key::ArrowUp { drag.sides.saturating_add(1) } else { drag.sides.saturating_sub(1) };
                drag.sides = sides.clamp(*SIDES.start(), *SIDES.end());
                let at = drag.at;
                self.update(at, key.modifiers, cx);
                true
            }
            Key::Enter if key.pressed && !key.repeat && self.drag.is_none() => {
                let at = cx.view.to_doc(cx.view.rect.center());
                self.events.push(ShapeDragEvent::Click { at, kind: self.options.kind, modifiers: key.modifiers });
                true
            }
            _ => false,
        }
    }

    fn paint(&self, painter: &Painter, cx: &ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/vector_ui/shapes.rs:148-166 `draw_shape_preview` (the path over
        // a halo, :164 `contrast_path`).
        // new: `ShapeGeometry::to_path` mapped through `cx.view`, drawn with `ShapeDragTool::style`.
        let Some(shape) = self.drag.as_ref().filter(|d| d.began).and_then(|d| d.shape.as_ref()) else { return };
        if self.preview == Some(false) {
            return;
        }
        let style = Self::style(cx.tokens);
        let mut runs: Vec<(Vec<Pos2>, bool)> = Vec::new();
        kurbo::flatten(shape.to_path(cx.view.doc_len(0.25)), cx.view.doc_len(0.25), |el| match el {
            PathEl::MoveTo(p) => runs.push((vec![cx.view.to_screen(p)], false)),
            PathEl::LineTo(p) => {
                if let Some((points, _)) = runs.last_mut() {
                    points.push(cx.view.to_screen(p));
                }
            }
            PathEl::ClosePath => {
                if let Some((_, closed)) = runs.last_mut() {
                    *closed = true;
                }
            }
            _ => {}
        });
        for stroke in [style.halo, style.line] {
            for (points, closed) in &runs {
                painter.add(if *closed { Shape::closed_line(points.clone(), stroke) } else { Shape::line(points.clone(), stroke) });
            }
        }
    }

    fn snap_gesture(&self) -> Option<SnapGesture> {
        // pc: crates/ui-egui/src/tools/vector_ui/shapes.rs:85-87 (`shape_bounds` snaps while Space
        // repositions). new: `Point` while sizing, `Rect { at_press }` while Space moves the shape.
        let Some(Drag { space_from: Some(from), space_rect: Some(rect), origin, .. }) = &self.drag else {
            return Some(SnapGesture::Point);
        };
        Some(SnapGesture::Rect { at_press: *rect + (*origin - *from) })
    }

    fn node(&self, cx: &ToolContext<'_>) -> Option<ToolNode> {
        let drag = self.drag.as_ref().filter(|d| d.began)?;
        let bounds = drag.shape.as_ref()?.bounds();
        let labels = self.labels.clone().unwrap_or_else(|| ShapeDragToolLabels::of_ctx(&cx.canvas.ctx));
        let (w, h) = (cx.numbers.format(bounds.width(), 0..=0), cx.numbers.format(bounds.height(), 0..=0));
        Some(ToolNode::new(craft_i18n::fmt(&labels.drawing, &[("width", &w), ("height", &h)])).value(labels.hint.clone()).bounds(bounds))
    }
}

/// The move of a Space drag from `from` to `to`, in document units.
fn space_move(from: Point, to: Point) -> Vec2 {
    to - from
}
