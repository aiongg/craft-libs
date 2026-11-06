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

use kurbo::{Point, Rect, Vec2};

use super::guides::SmartLine;

/// Screen points within which a position snaps (Photoshop).
pub const SNAP_PX: f64 = 8.0;

/// What a target is, so an app can filter (Snap To › Guides only) and the lines can be drawn in
/// the smart-guide colour or the plain one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SnapKind {
    #[allow(missing_docs)]
    Guide,
    #[allow(missing_docs)]
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
    /// What the target is.
    pub kind: SnapKind,
    /// The extent of the target along its line, for drawing a smart guide between the two
    /// objects; `None` for a full-height line (guides, grid).
    pub span: Option<(f64, f64)>,
}

/// The line the UI draws for a snap that happened.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SnapLine {
    /// Whether the line is vertical (snaps x) or horizontal (snaps y).
    pub vertical: bool,
    /// The position along the snapped axis, in document units.
    pub position: f64,
    /// What the target was.
    pub kind: SnapKind,
    /// From the snapped object to the target, along the line.
    pub span: Option<(f64, f64)>,
}

/// Half the length, in document units, of a line without a span. The smart-guide painter clips
/// to the view, so the line reads as running across all of it.
const FULL_EXTENT: f64 = 1.0e6;

impl From<SnapLine> for SmartLine {
    /// The line along `span`, or across the whole view when there is none.
    // new: SmartLine is the drawn form.
    fn from(line: SnapLine) -> Self {
        let (from, to) = line.span.unwrap_or((-FULL_EXTENT, FULL_EXTENT));
        let (a, b) = if line.vertical {
            (Point::new(line.position, from), Point::new(line.position, to))
        } else {
            (Point::new(from, line.position), Point::new(to, line.position))
        };
        SmartLine { a, b, extension: false }
    }
}

/// View › Snap To and View › Show › Smart Guides, as the app stores them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SnapOptions {
    /// View › Snap (the master switch).
    pub on: bool,
    #[allow(missing_docs)]
    pub guides: bool,
    #[allow(missing_docs)]
    pub grid: bool,
    #[allow(missing_docs)]
    pub layers: bool,
    #[allow(missing_docs)]
    pub document_bounds: bool,
    #[allow(missing_docs)]
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
    Rect {
        /// The rectangle when the gesture began.
        at_press: Rect,
    },
    /// Only one axis snaps (a guide being dragged).
    Axis {
        /// Whether the snapping axis is x (a vertical guide).
        vertical: bool,
    },
}

/// The snap targets of one gesture.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SnapTargets {
    targets: Vec<SnapTarget>,
}

/// Distances closer than this are equal.
const TIE: f64 = 1e-9;

/// The line through `target` reaching the snapped object at `a..b` along the line; a target
/// without a span stays full height.
fn line(target: &SnapTarget, a: f64, b: f64) -> SnapLine {
    let span = target.span.map(|(lo, hi)| (lo.min(a).min(b), hi.max(a).max(b)));
    SnapLine { vertical: target.vertical, position: target.position, kind: target.kind, span }
}

impl SnapTargets {
    /// No targets.
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
        let r = rect.abs();
        let (xs, ys) = ((r.y0, r.y1), (r.x0, r.x1));
        for (vertical, position, kind, span) in [
            (true, r.x0, edge, xs),
            (true, r.x1, edge, xs),
            (true, (r.x0 + r.x1) / 2.0, center, xs),
            (false, r.y0, edge, ys),
            (false, r.y1, edge, ys),
            (false, (r.y0 + r.y1) / 2.0, center, ys),
        ] {
            self.add(SnapTarget { vertical, position, kind, span: Some(span) });
        }
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
        grid: Option<(Point, Vec2)>,
    ) -> Self {
        if !options.on {
            return self;
        }
        let mut this = self;
        if options.guides {
            for (vertical, position) in guides {
                this.add(SnapTarget { vertical, position, kind: SnapKind::Guide, span: None });
            }
        }
        if options.grid
            && let Some((origin, step)) = grid
        {
            let lines = |origin: f64, step: f64, lo: f64, hi: f64| {
                let first = ((lo - origin) / step).ceil() as i64;
                let last = ((hi - origin) / step).floor() as i64;
                (first..=last).map(move |k| origin + k as f64 * step)
            };
            let r = bounds.abs();
            if step.x.is_finite() && step.x > 0.0 {
                for position in lines(origin.x, step.x, r.x0, r.x1) {
                    this.add(SnapTarget { vertical: true, position, kind: SnapKind::Grid, span: None });
                }
            }
            if step.y.is_finite() && step.y > 0.0 {
                for position in lines(origin.y, step.y, r.y0, r.y1) {
                    this.add(SnapTarget { vertical: false, position, kind: SnapKind::Grid, span: None });
                }
            }
        }
        if options.document_bounds {
            this.add_rect(bounds, SnapKind::Bounds, SnapKind::Center);
        }
        this
    }

    /// Only the targets `keep` accepts (Snap To with one kind off).
    // pc: crates/engine/src/snap.rs:174 `filtered`.
    pub fn filtered(&self, keep: impl Fn(SnapKind) -> bool) -> SnapTargets {
        SnapTargets { targets: self.targets.iter().copied().filter(|t| keep(t.kind)).collect() }
    }

    /// Whether there is no target.
    pub fn is_empty(&self) -> bool {
        self.targets.is_empty()
    }

    /// The nearest target on one axis within `tolerance` of `value`.
    // pc: crates/engine/src/snap.rs:189 `nearest`.
    pub fn nearest(&self, vertical: bool, value: f64, tolerance: f64) -> Option<SnapTarget> {
        let mut best: Option<(f64, SnapTarget)> = None;
        for target in self.targets.iter().filter(|t| t.vertical == vertical) {
            let distance = (target.position - value).abs();
            if distance > tolerance {
                continue;
            }
            // A guide wins a tie, as in Photoshop.
            let better = best.is_none_or(|(d, t)| {
                distance < d - TIE || ((distance - d).abs() <= TIE && target.kind == SnapKind::Guide && t.kind != SnapKind::Guide)
            });
            if better {
                best = Some((distance, *target));
            }
        }
        best.map(|(_, target)| target)
    }

    /// `point` snapped on each axis independently, with the lines to draw.
    // pc: crates/engine/src/snap.rs:216 `snap_point`.
    pub fn snap_point(&self, point: Point, tolerance: f64) -> (Point, Vec<SnapLine>) {
        let mut out = point;
        let mut lines = Vec::new();
        if let Some(t) = self.nearest(true, point.x, tolerance) {
            out.x = t.position;
            lines.push(line(&t, point.y, point.y));
        }
        if let Some(t) = self.nearest(false, point.y, tolerance) {
            out.y = t.position;
            lines.push(line(&t, out.x, out.x));
        }
        (out, lines)
    }

    /// The offset that moves `rect` so its nearest edge or centre meets a target on each axis,
    /// with the lines to draw. Edges win over centres at equal distance.
    // pc: crates/engine/src/snap.rs:233 `snap_rect`.
    pub fn snap_rect(&self, rect: Rect, tolerance: f64) -> (Vec2, Vec<SnapLine>) {
        let mut delta = Vec2::ZERO;
        let mut lines = Vec::new();
        let rect = rect.abs();
        for vertical in [true, false] {
            let (lo, hi, other) = if vertical { (rect.x0, rect.x1, (rect.y0, rect.y1)) } else { (rect.y0, rect.y1, (rect.x0, rect.x1)) };
            // Edges before the centre, so an edge wins a tie.
            let anchors = [lo, hi, (lo + hi) / 2.0];
            let mut best: Option<f64> = None;
            for anchor in anchors {
                if let Some(t) = self.nearest(vertical, anchor, tolerance) {
                    let d = t.position - anchor;
                    if best.is_none_or(|b| d.abs() < b.abs() - TIE) {
                        best = Some(d);
                    }
                }
            }
            let Some(d) = best else { continue };
            if vertical {
                delta.x = d;
            } else {
                delta.y = d;
            }
            for anchor in anchors {
                let moved = anchor + d;
                let hit = self
                    .targets
                    .iter()
                    .filter(|t| t.vertical == vertical && (t.position - moved).abs() < 1e-6)
                    .min_by_key(|t| u8::from(t.kind != SnapKind::Guide));
                if let Some(t) = hit {
                    lines.push(line(t, other.0, other.1));
                }
            }
        }
        (delta, lines)
    }

    /// `value` on one axis snapped to the nearest target within `tolerance`, for a guide drag
    /// or a ruler's snap closure.
    // pc: crates/ui-egui/src/canvas/snap_ui.rs:547 `snap_guide`.
    pub fn snap_axis(&self, vertical: bool, value: f64, tolerance: f64) -> (f64, Option<SnapLine>) {
        match self.nearest(vertical, value, tolerance) {
            Some(t) => (t.position, Some(SnapLine { vertical, position: t.position, kind: t.kind, span: t.span })),
            None => (value, None),
        }
    }
}

/// The bounding rectangle of `rects`, or `None` for none.
// pc: crates/engine/src/snap.rs:278 `union`.
pub fn union(rects: impl IntoIterator<Item = Rect>) -> Option<Rect> {
    rects.into_iter().reduce(|a, b| a.union(b))
}
