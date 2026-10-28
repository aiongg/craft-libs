//! The paths canvas overlays edit and draw: knots, subpaths, the host's selection and the edits
//! the overlays report. The host converts its own path type to these each frame and applies the
//! edits to its own.

use egui::Color32;
use kurbo::{BezPath, CubicBez, Point, Vec2};

/// One point of a path: its anchor and its two direction handles, in document units. A handle at
/// its anchor is retracted.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Knot {
    pub anchor: Point,
    /// The handle of the segment arriving at the anchor.
    pub in_ctrl: Point,
    /// The handle of the segment leaving the anchor.
    pub out_ctrl: Point,
    /// A smooth point keeps its handles collinear when one turns (the host's rule).
    pub smooth: bool,
}

impl Knot {
    /// A corner point with both handles retracted.
    pub fn corner(anchor: Point) -> Self {
        Self { anchor, in_ctrl: anchor, out_ctrl: anchor, smooth: false }
    }

    /// A smooth point whose out handle is `out` and whose in handle mirrors it.
    pub fn smooth(anchor: Point, out: Point) -> Self {
        Self { anchor, in_ctrl: anchor - (out - anchor), out_ctrl: out, smooth: true }
    }

    /// The handle on `side`.
    pub fn handle(&self, side: HandleSide) -> Point {
        match side {
            HandleSide::In => self.in_ctrl,
            HandleSide::Out => self.out_ctrl,
        }
    }

    /// Whether the handle on `side` lies on the anchor.
    pub fn retracted(&self, side: HandleSide) -> bool {
        self.handle(side) == self.anchor
    }
}

/// One subpath: its knots in order, open or closed.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EditSubpath {
    pub knots: Vec<Knot>,
    pub closed: bool,
}

impl EditSubpath {
    /// The number of segments: one between each pair of neighbouring knots, and one from the last
    /// knot back to the first when closed.
    pub fn segment_count(&self) -> usize {
        match self.knots.len() {
            0 | 1 => 0,
            n if self.closed => n,
            n => n - 1,
        }
    }

    /// The knots a segment joins: the segment `i` leaves knot `i` for the next one.
    pub fn segment_ends(&self, i: usize) -> Option<(usize, usize)> {
        (i < self.segment_count()).then(|| (i, (i + 1) % self.knots.len()))
    }

    /// The segment leaving knot `i`, as a cubic in document units.
    pub fn segment(&self, i: usize) -> Option<CubicBez> {
        let (a, b) = self.segment_ends(i)?;
        let (a, b) = (&self.knots[a], &self.knots[b]);
        Some(CubicBez::new(a.anchor, a.out_ctrl, b.in_ctrl, b.anchor))
    }

    /// Whether a segment is a straight line: both its handles retracted.
    pub fn is_straight(&self, i: usize) -> bool {
        self.segment_ends(i).is_some_and(|(a, b)| self.knots[a].retracted(HandleSide::Out) && self.knots[b].retracted(HandleSide::In))
    }

    /// The subpath as a kurbo path.
    pub fn bez_path(&self) -> BezPath {
        let mut path = BezPath::new();
        let Some(first) = self.knots.first() else { return path };
        path.move_to(first.anchor);
        for i in 0..self.segment_count() {
            if let Some(c) = self.segment(i) {
                path.curve_to(c.p1, c.p2, c.p3);
            }
        }
        if self.closed {
            path.close_path();
        }
        path
    }
}

/// A path the overlays show: the host's key for it, its subpaths and its colour.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EditPath {
    /// The host's key, stable across frames.
    pub key: u64,
    pub subpaths: Vec<EditSubpath>,
    /// The layer colour its outline, anchors and handles are drawn in (DesignCraft, VectorCraft);
    /// `overlay.stroke` without one.
    pub color: Option<Color32>,
}

impl EditPath {
    /// The knot `at` refers to.
    pub fn knot(&self, at: KnotRef) -> Option<&Knot> {
        self.subpaths.get(at.subpath)?.knots.get(at.knot)
    }

    /// The number of anchors over all subpaths.
    pub fn anchor_count(&self) -> usize {
        self.subpaths.iter().map(|s| s.knots.len()).sum()
    }

    /// Every knot's reference, in path order.
    pub fn knot_refs(&self) -> impl Iterator<Item = KnotRef> + '_ {
        self.subpaths.iter().enumerate().flat_map(|(subpath, s)| (0..s.knots.len()).map(move |knot| KnotRef { subpath, knot }))
    }
}

/// A knot of a path, or the segment leaving it: its subpath and its index there.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct KnotRef {
    pub subpath: usize,
    pub knot: usize,
}

impl KnotRef {
    pub fn new(subpath: usize, knot: usize) -> Self {
        Self { subpath, knot }
    }
}

/// The host's path selection: the path the overlay edits, its selected anchors and a selected
/// segment (whose two handles show).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PathSelection {
    pub path: Option<u64>,
    pub anchors: Vec<KnotRef>,
    /// The segment leaving this knot.
    pub segment: Option<KnotRef>,
}

impl PathSelection {
    /// Whether `knot` of `path` is a selected anchor.
    pub fn has_anchor(&self, path: u64, knot: KnotRef) -> bool {
        self.path == Some(path) && self.anchors.contains(&knot)
    }
}

/// Which of a knot's two direction handles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HandleSide {
    In,
    Out,
}

/// An edit of a path, the value of the overlays' edit events. A drag reports the whole move from
/// its press: the host applies each one to the path it had at the press.
#[derive(Clone, Debug, PartialEq)]
pub enum PathEdit {
    /// Move anchors with their handles by `by`, in document units.
    MoveAnchors { path: u64, anchors: Vec<KnotRef>, by: Vec2 },
    /// Move a direction handle to `to`. A smooth knot keeps its other handle collinear, unless
    /// `alone`, which also makes the knot a corner (the host's rule).
    MoveHandle { path: u64, knot: KnotRef, side: HandleSide, to: Point, alone: bool },
    /// Bend the segment leaving `segment` so its point at `t` moves by `by`, the nearer handle
    /// more (PhotoCraft's `bend_segment`).
    BendSegment { path: u64, segment: KnotRef, t: f64, by: Vec2 },
    /// Convert Point: `None` makes a corner with retracted handles; `Some(out)` a smooth point
    /// whose out handle is at `out` and whose in handle mirrors it.
    Convert { path: u64, knot: KnotRef, out: Option<Point> },
}

impl PathEdit {
    /// The edit's path.
    pub fn path(&self) -> u64 {
        match *self {
            PathEdit::MoveAnchors { path, .. }
            | PathEdit::MoveHandle { path, .. }
            | PathEdit::BendSegment { path, .. }
            | PathEdit::Convert { path, .. } => path,
        }
    }
}
