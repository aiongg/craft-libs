//! [`ShapeDragTool`]: the Shape tools' press, drag and release on the [`ToolHost`](craft_ui::canvas::ToolHost)
//! (task SH-T3).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::Painter;
use kurbo::{Point, Vec2};

use super::{ShapeDragEvent, ShapeDragNames, ShapeDragOptions, ShapeDragRules, ShapeKind};
use crate::geometry::shape::ShapeGeometry;
use craft_ui::canvas::{CanvasTool, SnapGesture, ToolAction, ToolContext, ToolCursor, ToolKey, ToolPointer};
use craft_ui::edit::GestureId;

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
    names: Option<ShapeDragNames>,
    events: Vec<ShapeDragEvent>,
    drag: Option<Drag>,
}

/// A drag between passes.
#[derive(Clone, Debug)]
struct Drag {
    id: GestureId,
    /// The press, moved by Space.
    start: Point,
    /// The last pointer.
    at: Point,
    /// Past the click slop: `Begin` went out.
    began: bool,
    /// Where the pointer was when Space went down, while it is held.
    space_from: Option<Point>,
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
    pub fn names(&mut self, names: ShapeDragNames) -> &mut Self {
        self.names = Some(names);
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

    /// The shape a drag from `start` to `at` with `modifiers` draws under the options and rules,
    /// or `None` when a side is under 1 px: [`box_drag`](crate::geometry::shape::box_drag) for
    /// the box kinds (Rectangle with four equal radii when `corner_radius` > 0, Rounded Rectangle
    /// always with them, Triangle a 3-sided polygon, Polygon and Star with the options' sides and
    /// ratio, Custom a `Custom` box keeping proportions with ⇧),
    /// [`line_drag`](crate::geometry::shape::line_drag) for a Line (weight at least 1), and
    /// [`RadialFrame::drag`](crate::geometry::shape::RadialFrame::drag) in the radial frame.
    fn shape(&self, drag: &Drag, modifiers: egui::Modifiers) -> Option<ShapeGeometry> {
        // pc: crates/ui-egui/src/tools/vector_ui/shapes.rs:50-82 `shape_geometry`.
        // inv-vectorcraft: crates/tools/src/shape.rs:131-145 (the radial kinds).
        todo!()
    }
}

impl CanvasTool for ShapeDragTool {
    fn cursor(&self, cx: &ToolContext<'_>) -> ToolCursor {
        // new: the crosshair (`ToolCursor::Crosshair` at the cursor style's length and gap), as the
        // marquee tools show it.
        todo!()
    }

    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        // pc: crates/ui-egui/src/canvas/gesture.rs:84 (a shape drag that stays inside
        // `shape_dialog::is_click` opens the dialog).
        // new: claims the press, starts a `Drag` with a new `GestureId`, emits nothing yet.
        todo!()
    }

    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/vector_ui/shapes.rs:85-96 (Space moves the box, which snaps).
        // new: past `click_slop / zoom` emits `Begin` then `Update` with `shape`; Space moves
        // `start` by the pointer's move; a readout of W and H through `cx.readout`.
        todo!()
    }

    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/vector_ui/shapes.rs:99-112 `finish_shape`;
        // crates/ui-egui/src/dialogs/shape_dialog.rs:50-53 `is_click`.
        // new: `Commit { changed: shape.is_some() }` after a begun drag, else `Click`.
        todo!()
    }

    fn cancel(&mut self, cx: &mut ToolContext<'_>) {
        // new: `Cancel { reason: Escape }` when a drag had begun; drops the drag.
        todo!()
    }

    fn key(&mut self, key: &ToolKey, cx: &mut ToolContext<'_>) -> bool {
        // inv-vectorcraft: crates/tools/src/shape.rs:5-7 (↑/↓ change the count during a drag).
        // new: Enter without a drag reports `Click` at the view centre (keyboard route, lead call
        // 11); Space and the modifiers re-run `drag` with the last pointer.
        todo!()
    }

    fn paint(&self, painter: &Painter, cx: &ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/vector_ui/shapes.rs:148-166 `draw_shape_preview` (the path over
        // a halo, :164 `contrast_path`).
        // new: `ShapeGeometry::to_path` mapped through `cx.view`, drawn with `ShapeDragTool::style`.
        todo!()
    }

    fn snap_gesture(&self) -> Option<SnapGesture> {
        // pc: crates/ui-egui/src/tools/vector_ui/shapes.rs:85-87 (`shape_bounds` snaps while Space
        // repositions). new: `Point` while sizing, `Rect { at_press }` while Space moves the shape.
        todo!()
    }
}

/// The move of a Space drag from `from` to `to`, in document units.
fn space_move(from: Point, to: Point) -> Vec2 {
    to - from
}
