//! [`SnapTargets`]: snapping (View › Snap / Snap To) and smart-guide geometry, egui-free.
//!
//! The app builds a target set once per gesture from its document (guides, grid, document bounds
//! and centre, layer edges and centres, selection edges); every pointer move asks it to snap a
//! point (marquee corners, crop handles, guide positions, pen anchors, transform handles) or a
//! moving rectangle (the Move tool, Free Transform drags). Distances are document units; the
//! [`ToolHost`](super::ToolHost) converts its 8 screen points with the zoom. Results carry
//! [`SnapLine`]s the host draws with [`SmartGuides`](super::SmartGuides).
//!
//! Snapping is on when View › Snap is; holding Ctrl while dragging turns it off, except when Ctrl
//! is the tool's own modifier. Smart guides follow View › Show › Smart Guides and also snap the
//! Move tool to other layers' edges and centres. The Rulers' `snap` closure and the crop host's
//! snap closures stay as they are; an app builds them from a target set here.

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use kurbo::{Point, Rect};

use super::guides::SmartLine;

/// Screen points within which a position snaps (Photoshop).
pub const SNAP_PX: f64 = 8.0;

/// What a target is, so an app can filter (Snap To › Guides only) and the lines can be drawn in
/// the smart-guide colour or the plain one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SnapKind {
    Guide,
    Grid,
    /// The document's edges.
    Bounds,
    /// The document's centre lines.
    Center,
    /// Another layer's (or page object's) edge: a smart guide.
    LayerEdge,
    /// Another layer's centre: a smart guide.
    LayerCenter,
    /// The selection's edges.
    Selection,
}

impl SnapKind {
    /// Drawn as a smart guide (magenta) rather than snapped silently.
    pub fn is_smart(self) -> bool {
        matches!(self, SnapKind::LayerEdge | SnapKind::LayerCenter)
    }
}

/// One snap position on one axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SnapTarget {
    /// Vertical line (snaps x) or horizontal line (snaps y).
    pub vertical: bool,
    /// The position along the snapped axis, document units.
    pub position: f64,
    pub kind: SnapKind,
    /// The extent of the target along its line, for drawing a smart guide between the two
    /// objects; `None` for a full-height line (guides, grid).
    pub span: Option<(f64, f64)>,
}

/// The line the UI draws for a snap that happened.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SnapLine {
    pub vertical: bool,
    pub position: f64,
    pub kind: SnapKind,
    /// From the snapped object to the target, along the line.
    pub span: Option<(f64, f64)>,
}

impl From<SnapLine> for SmartLine {
    // new: SmartLine is the drawn form; the conversion fills its ends from `span` or the view.
    fn from(line: SnapLine) -> Self {
        todo!()
    }
}

/// View › Snap To and View › Show › Smart Guides, as the app stores them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SnapOptions {
    /// View › Snap (the master switch).
    pub on: bool,
    pub guides: bool,
    pub grid: bool,
    pub layers: bool,
    pub document_bounds: bool,
    pub selection: bool,
    /// View › Show › Smart Guides.
    pub smart_guides: bool,
}

impl Default for SnapOptions {
    /// Photoshop's defaults: everything on.
    fn default() -> Self {
        Self { on: true, guides: true, grid: true, layers: true, document_bounds: true, selection: true, smart_guides: true }
    }
}

/// How the host snaps a tool's pointer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SnapGesture {
    /// The pointer itself snaps (marquee corner, pen anchor, crop handle).
    Point,
    /// A rectangle moves with the pointer and its edges and centre snap: `at_press` is the
    /// rectangle when the gesture began; the host snaps `at_press + (pointer - press)`.
    Rect { at_press: Rect },
    /// Only one axis snaps (a guide being dragged).
    Axis { vertical: bool },
}

/// The snap targets of one gesture.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SnapTargets {
    targets: Vec<SnapTarget>,
}

impl SnapTargets {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add one target.
    pub fn add(&mut self, target: SnapTarget) {
        self.targets.push(target);
    }

    /// Add a rectangle's four edges as `edge` targets and its two centre lines as `center`
    /// targets, each spanning the rectangle.
    // pc: crates/engine/src/snap.rs:163 `add_rect`.
    pub fn add_rect(&mut self, rect: Rect, edge: SnapKind, center: SnapKind) {
        todo!()
    }

    /// Guides as full-height targets, grid lines of `spacing` from `origin` over `bounds`, the
    /// document bounds and centre: what every app has. Layers and the selection are added by
    /// the app with [`add_rect`](Self::add_rect).
    // pc: crates/engine/src/snap.rs:113 `from_document` (the document part).
    pub fn with_document(
        self,
        options: &SnapOptions,
        bounds: Rect,
        guides: impl IntoIterator<Item = (bool, f64)>,
        grid: Option<(Point, kurbo::Vec2)>,
    ) -> Self {
        todo!()
    }

    /// Only the targets `keep` accepts (Snap To with one kind off).
    // pc: crates/engine/src/snap.rs:174 `filtered`.
    pub fn filtered(&self, keep: impl Fn(SnapKind) -> bool) -> SnapTargets {
        todo!()
    }

    pub fn is_empty(&self) -> bool {
        self.targets.is_empty()
    }

    /// The nearest target on one axis within `tolerance` of `value`.
    // pc: crates/engine/src/snap.rs:189 `nearest`.
    pub fn nearest(&self, vertical: bool, value: f64, tolerance: f64) -> Option<SnapTarget> {
        todo!()
    }

    /// `point` snapped on each axis independently, with the lines to draw.
    // pc: crates/engine/src/snap.rs:216 `snap_point`.
    pub fn snap_point(&self, point: Point, tolerance: f64) -> (Point, Vec<SnapLine>) {
        todo!()
    }

    /// The offset that moves `rect` so its nearest edge or centre meets a target on each axis,
    /// with the lines to draw. Edges win over centres at equal distance.
    // pc: crates/engine/src/snap.rs:233 `snap_rect`.
    pub fn snap_rect(&self, rect: Rect, tolerance: f64) -> (kurbo::Vec2, Vec<SnapLine>) {
        todo!()
    }

    /// `value` on one axis snapped to the nearest target within `tolerance`, for a guide drag
    /// or a ruler's snap closure.
    // pc: crates/ui-egui/src/canvas/snap_ui.rs:547 `snap_guide`.
    pub fn snap_axis(&self, vertical: bool, value: f64, tolerance: f64) -> (f64, Option<SnapLine>) {
        todo!()
    }
}

/// The bounding rectangle of `rects`, or `None` for none.
// pc: crates/engine/src/snap.rs:278 `union`.
pub fn union(rects: impl IntoIterator<Item = Rect>) -> Option<Rect> {
    todo!()
}
