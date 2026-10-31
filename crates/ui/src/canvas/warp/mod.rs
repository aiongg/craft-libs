//! [`WarpGrid`]: the mesh of a warp (PhotoCraft's Edit › Transform › Warp) on a canvas: patch
//! boundaries, anchors and their handles, dragged by the pointer and moved from the keyboard.

pub(crate) mod story;

use std::borrow::Cow;

use egui::{AsIdSalt, Context, CursorIcon, Id, IdSalt, Key, Modifiers, Painter, Pos2, Response, Shape, Stroke, Ui, vec2};
use kurbo::{BezPath, PathEl, Point, Rect, Vec2};

use super::claim::PrimaryPress;
use super::{CanvasView, Claim, Part, PartFocus, RulerUnits, hit, resize_cursor};
use crate::behavior::nudge;
use crate::edit::order::Turn;
use crate::edit::{EditEvent, EditPolicy, Gesture};
use crate::localized::words::format_in;
use crate::localized::{Labels, Word, Words};
use crate::primitives::{Handle, HandleShape, HandleStyle};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens};

/// A warp's control mesh: bicubic Bézier patches sharing their edges, as
/// `(3 columns + 1) × (3 rows + 1)` control points row by row, in document units.
///
/// `us` and `vs` are the patch boundaries in the mesh's global parameter space (PhotoCraft's
/// `BezierMesh::us` and `vs`): the knots of the columns and rows, increasing from 0 to 1, one more
/// than there are columns or rows. A patch's local `(u, v)` and the mesh's `(s, t)` are related by
/// them, `s = us[column] + u * (us[column + 1] - us[column])`; the patches need not be even.
///
/// A point on a row and a column that are both multiples of 3 is an anchor, where patches meet.
/// The other points on those rows and columns are the anchors' handles; the points inside a patch
/// shape its inside and are neither drawn nor dragged.
#[derive(Clone, Debug, PartialEq)]
pub struct WarpMesh {
    pub us: Vec<f64>,
    pub vs: Vec<f64>,
    pub points: Vec<Point>,
}

/// The side of its anchor a handle leaves on, in the mesh's own order: up and down along a
/// column, left and right along a row (in the document, unmirrored).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WarpSide {
    Up,
    Left,
    Right,
    Down,
}

impl WarpMesh {
    /// An unwarped mesh over `rect`: `columns` × `rows` patches with even knots.
    pub fn identity(rect: Rect, columns: usize, rows: usize) -> Self {
        let even = |n: usize| (0..=n).map(|i| i as f64 / n as f64).collect();
        Self::straight(rect, even(columns.max(1)), even(rows.max(1)))
    }

    /// An unwarped mesh over `rect` with patch boundaries `us` and `vs` (see [`WarpMesh`]), as
    /// PhotoCraft's `BezierMesh::fit` of the identity. `None` when the knots are not valid.
    pub fn identity_with_knots(rect: Rect, us: Vec<f64>, vs: Vec<f64>) -> Option<Self> {
        (Self::knots_valid(&us) && Self::knots_valid(&vs)).then(|| Self::straight(rect, us, vs))
    }

    /// The unwarped mesh over `rect` for valid knots.
    fn straight(rect: Rect, us: Vec<f64>, vs: Vec<f64>) -> Self {
        // Control points sit at thirds of each patch's span, which is exact for a straight mesh.
        let along = |knots: &[f64]| -> Vec<f64> {
            let mut out: Vec<f64> = knots.windows(2).flat_map(|k| (0..3).map(move |i| k[0] + (k[1] - k[0]) * i as f64 / 3.0)).collect();
            out.push(1.0);
            out
        };
        let (xs, ys) = (along(&us), along(&vs));
        let points = ys.iter().flat_map(|&v| xs.iter().map(move |&u| Point::new(rect.x0 + u * rect.width(), rect.y0 + v * rect.height()))).collect();
        Self { us, vs, points }
    }

    /// A mesh from PhotoCraft's parts: knots `us`, `vs` and the control points row by row. `None`
    /// when they do not make a [valid](Self::is_valid) mesh.
    pub fn new(us: Vec<f64>, vs: Vec<f64>, points: Vec<Point>) -> Option<Self> {
        let mesh = Self { us, vs, points };
        mesh.is_valid().then_some(mesh)
    }

    /// Whether `knots` are patch boundaries: at least two, from 0 to 1, strictly increasing.
    fn knots_valid(knots: &[f64]) -> bool {
        knots.len() >= 2 && knots[0] == 0.0 && knots[knots.len() - 1] == 1.0 && knots.windows(2).all(|k| k[1] > k[0])
    }

    /// The number of patch columns.
    pub fn columns(&self) -> usize {
        self.us.len().saturating_sub(1)
    }

    /// The number of patch rows.
    pub fn rows(&self) -> usize {
        self.vs.len().saturating_sub(1)
    }

    /// The mesh parameters `(s, t)` of the point `(u, v)` of patch (`column`, `row`).
    pub fn params(&self, column: usize, row: usize, u: f64, v: f64) -> (f64, f64) {
        (self.us[column] + u * (self.us[column + 1] - self.us[column]), self.vs[row] + v * (self.vs[row + 1] - self.vs[row]))
    }

    /// The patch under the mesh parameters `(s, t)` (each clamped to 0 to 1) and the parameters
    /// within it: the inverse of [`params`](Self::params). A parameter on a knot belongs to the
    /// patch before it, as in PhotoCraft's `BezierMesh::eval`.
    pub fn locate(&self, s: f64, t: f64) -> (usize, usize, f64, f64) {
        let at = |knots: &[f64], x: f64| {
            let x = if x.is_finite() { x.clamp(0.0, 1.0) } else { 0.0 };
            let k = knots[1..knots.len() - 1].partition_point(|&k| k < x);
            (k, (x - knots[k]) / (knots[k + 1] - knots[k]))
        };
        let ((column, u), (row, v)) = (at(&self.us, s), at(&self.vs, t));
        (column, row, u, v)
    }

    /// Control points per row: `3 columns + 1`.
    pub fn width(&self) -> usize {
        3 * self.columns() + 1
    }

    /// Control points per column: `3 rows + 1`.
    pub fn height(&self) -> usize {
        3 * self.rows() + 1
    }

    /// Whether the knots are valid (see [`WarpMesh`]) and there are as many points as the patches
    /// need.
    pub fn is_valid(&self) -> bool {
        Self::knots_valid(&self.us) && Self::knots_valid(&self.vs) && self.points.len() == self.width() * self.height()
    }

    /// The control point at column `i`, row `j`.
    pub fn point(&self, i: usize, j: usize) -> Point {
        self.points[j * self.width() + i]
    }

    /// The anchor the point `index` belongs to: its own index for an anchor, its anchor's for a
    /// handle, `None` for a point inside a patch.
    pub fn anchor_of(&self, index: usize) -> Option<usize> {
        let w = self.width();
        let (i, j) = (index % w, index / w);
        let near = |k: usize| match k % 3 {
            0 => k,
            1 => k - 1,
            _ => k + 1,
        };
        match (i % 3 == 0, j % 3 == 0) {
            (true, true) => Some(index),
            (true, false) => Some(near(j) * w + i),
            (false, true) => Some(j * w + near(i)),
            (false, false) => None,
        }
    }

    /// The handles of the anchor `index`, each with its side, in the mesh's order.
    pub fn handles(&self, index: usize) -> Vec<(usize, WarpSide)> {
        let w = self.width();
        let (i, j) = (index % w, index / w);
        let mut out = Vec::with_capacity(4);
        if j > 0 {
            out.push((index - w, WarpSide::Up));
        }
        if i > 0 {
            out.push((index - 1, WarpSide::Left));
        }
        if i + 1 < w {
            out.push((index + 1, WarpSide::Right));
        }
        if j + 1 < self.height() {
            out.push((index + w, WarpSide::Down));
        }
        out
    }

    /// The anchors row by row.
    fn anchors(&self) -> impl Iterator<Item = usize> + '_ {
        let w = self.width();
        (0..self.height()).step_by(3).flat_map(move |j| (0..w).step_by(3).map(move |i| j * w + i))
    }

    /// The chain of cubic segments along control row `j`.
    fn row_path(&self, j: usize) -> BezPath {
        chain((0..self.width()).map(|i| self.point(i, j)).collect())
    }

    /// The chain of cubic segments along control column `i`.
    fn column_path(&self, i: usize) -> BezPath {
        chain((0..self.height()).map(|j| self.point(i, j)).collect())
    }

    /// The patch boundaries: each a chain of cubic segments along a row or a column of anchors.
    fn boundaries(&self) -> Vec<BezPath> {
        let rows = (0..self.height()).step_by(3).map(|j| self.row_path(j));
        let columns = (0..self.width()).step_by(3).map(|i| self.column_path(i));
        rows.chain(columns).collect()
    }

    /// The mesh with new lines of anchors cut through patch (`column`, `row`) at `(u, v)` (each 0
    /// to 1 across the patch): [`Vertical`](WarpSplit::Vertical) cuts the patch column at `u`,
    /// [`Horizontal`](WarpSplit::Horizontal) the patch row at `v` and [`Cross`](WarpSplit::Cross)
    /// both. The patches on a cut line are cut by de Casteljau, so the surface is unchanged, and
    /// the new knot is the mesh parameter of the cut ([`params`](Self::params)). A line whose
    /// patch index is past the mesh or whose parameter is not strictly between 0 and 1 is not
    /// cut.
    pub fn split(&self, split: WarpSplit, column: usize, row: usize, u: f64, v: f64) -> WarpMesh {
        if !self.is_valid() {
            return self.clone();
        }
        let mut out = self.clone();
        let inside = |x: f64| x > 0.0 && x < 1.0;
        if split != WarpSplit::Horizontal && column < self.columns() && inside(u) {
            out = out.split_column(column, u, self.params(column, 0, u, 0.0).0);
        }
        if split != WarpSplit::Vertical && row < self.rows() && inside(v) {
            out = out.split_row(row, v, self.params(0, row, 0.0, v).1);
        }
        out
    }

    /// A one-patch mesh with its rule-of-thirds lines made real: cut into 3 × 3 patches at a
    /// third and two thirds across each way, the surface unchanged. PhotoCraft does this before
    /// the first split of a custom warp so that the cut divides the grid on screen instead of
    /// replacing it. Any other mesh is returned as it is.
    pub fn commit_thirds(&self) -> WarpMesh {
        if !self.is_valid() || self.columns() != 1 || self.rows() != 1 {
            return self.clone();
        }
        // The second cut is at half of what is left after the first. The knots are 0, 1/3, 2/3, 1.
        self.split_column(0, 1.0 / 3.0, 1.0 / 3.0).split_column(1, 0.5, 2.0 / 3.0).split_row(0, 1.0 / 3.0, 1.0 / 3.0).split_row(1, 0.5, 2.0 / 3.0)
    }

    /// The de Casteljau cut of a cubic at `t`: the five new control points after its first.
    fn cut(p: [Point; 4], t: f64) -> [Point; 5] {
        let lerp = |a: Point, b: Point| Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t);
        let (a, b, c) = (lerp(p[0], p[1]), lerp(p[1], p[2]), lerp(p[2], p[3]));
        let (d, e) = (lerp(a, b), lerp(b, c));
        [a, d, lerp(d, e), e, c]
    }

    /// A new row of anchors through patch row `index` at `t`, at mesh parameter `knot`.
    fn split_row(&self, index: usize, t: f64, knot: f64) -> WarpMesh {
        let (w, h) = (self.width(), self.height());
        let mut rows: Vec<Vec<Point>> = (0..h).map(|j| (0..w).map(|i| self.point(i, j)).collect()).collect();
        let at = 3 * index;
        let cuts: Vec<[Point; 5]> = (0..w).map(|i| Self::cut(std::array::from_fn(|k| self.point(i, at + k)), t)).collect();
        let inserted: Vec<Vec<Point>> = (0..5).map(|k| cuts.iter().map(|c| c[k]).collect()).collect();
        rows.splice(at + 1..at + 3, inserted);
        let mut vs = self.vs.clone();
        vs.insert(index + 1, knot);
        WarpMesh { us: self.us.clone(), vs, points: rows.concat() }
    }

    /// A new column of anchors through patch column `index` at `t`, at mesh parameter `knot`.
    fn split_column(&self, index: usize, t: f64, knot: f64) -> WarpMesh {
        let (w, h) = (self.width(), self.height());
        let at = 3 * index;
        let mut points = Vec::with_capacity((w + 3) * h);
        for j in 0..h {
            let row: [Point; 4] = std::array::from_fn(|k| self.point(at + k, j));
            points.extend((0..=at).map(|i| self.point(i, j)));
            points.extend(Self::cut(row, t));
            points.extend((at + 3..w).map(|i| self.point(i, j)));
        }
        let mut us = self.us.clone();
        us.insert(index + 1, knot);
        WarpMesh { us, vs: self.vs.clone(), points }
    }

    /// The split a quick split (Alt or Ctrl with no tool set) makes at `(u, v)` of patch
    /// (`column`, `row`), PhotoCraft's rule: a [`Cross`](WarpSplit::Cross) in the open, and the
    /// perpendicular line when the pointer is close to an existing line of one direction. A
    /// one-patch mesh's rule-of-thirds lines count as existing lines. "Close" is 8 document units
    /// of the warp's source rectangle `bounds` across, within 2 to 12 % of the mesh.
    fn quick_split(&self, bounds: Rect, column: usize, row: usize, u: f64, v: f64) -> WarpSplit {
        let tolerance = |extent: f64| (8.0 / extent.abs().max(1.0)).clamp(0.02, 0.12);
        let thirds = self.columns() == 1 && self.rows() == 1;
        let near = |knots: &[f64], at: f64, tolerance: f64| {
            let inner = knots[1..knots.len() - 1].iter().copied();
            let thirds = thirds.then_some([1.0 / 3.0, 2.0 / 3.0]).into_iter().flatten();
            inner.chain(thirds).any(|k| (k - at).abs() < tolerance)
        };
        let (s, t) = self.params(column, row, u, v);
        match (near(&self.us, s, tolerance(bounds.width())), near(&self.vs, t, tolerance(bounds.height()))) {
            (true, false) => WarpSplit::Horizontal,
            (false, true) => WarpSplit::Vertical,
            _ => WarpSplit::Cross,
        }
    }

    /// The point of the surface nearest to document point `p`: its patch column and row, its
    /// parameters `(u, v)` in that patch and its distance from `p`.
    fn nearest(&self, p: Point) -> (usize, usize, f64, f64, f64) {
        const COARSE: usize = 8;
        const ROUNDS: usize = 10;
        let mut best = (0, 0, 0.0, 0.0, f64::INFINITY);
        for row in 0..self.rows() {
            for column in 0..self.columns() {
                let at = |u: f64, v: f64| self.patch_point(column, row, u, v).distance(p);
                let mut found = (0.5, 0.5, f64::INFINITY);
                for j in 0..=COARSE {
                    for i in 0..=COARSE {
                        let (u, v) = (i as f64 / COARSE as f64, j as f64 / COARSE as f64);
                        let d = at(u, v);
                        if d < found.2 {
                            found = (u, v, d);
                        }
                    }
                }
                let mut reach = 1.0 / COARSE as f64;
                for _ in 0..ROUNDS {
                    let (cu, cv) = (found.0, found.1);
                    for j in -2..=2 {
                        for i in -2..=2 {
                            let (u, v) = ((cu + f64::from(i) * reach / 2.0).clamp(0.0, 1.0), (cv + f64::from(j) * reach / 2.0).clamp(0.0, 1.0));
                            let d = at(u, v);
                            if d < found.2 {
                                found = (u, v, d);
                            }
                        }
                    }
                    reach /= 2.0;
                }
                if found.2 < best.4 {
                    best = (column, row, found.0, found.1, found.2);
                }
            }
        }
        best
    }

    /// The surface point of patch (`column`, `row`) at `(u, v)`.
    fn patch_point(&self, column: usize, row: usize, u: f64, v: f64) -> Point {
        let across: [Point; 4] = std::array::from_fn(|j| cubic(std::array::from_fn(|i| self.point(3 * column + i, 3 * row + j)), u));
        cubic(across, v)
    }

    /// A one-patch mesh's rule-of-thirds curves: the patch at a third and two thirds across, each
    /// way.
    fn thirds(&self) -> Vec<BezPath> {
        let curve = |q: [Point; 4]| {
            let mut path = BezPath::new();
            path.move_to(q[0]);
            path.curve_to(q[1], q[2], q[3]);
            path
        };
        let mut out = Vec::with_capacity(4);
        for t in [1.0 / 3.0, 2.0 / 3.0] {
            out.push(curve(std::array::from_fn(|j| cubic(std::array::from_fn(|i| self.point(i, j)), t))));
            out.push(curve(std::array::from_fn(|i| cubic(std::array::from_fn(|j| self.point(i, j)), t))));
        }
        out
    }
}

/// The cubic segments through `points`, the first the start and every three after the next
/// segment's controls and end.
fn chain(points: Vec<Point>) -> BezPath {
    let mut path = BezPath::new();
    path.move_to(points[0]);
    for &[a, b, c] in points[1..].as_chunks::<3>().0 {
        path.curve_to(a, b, c);
    }
    path
}

/// The cubic Bézier through control points `p` at `t`.
fn cubic(p: [Point; 4], t: f64) -> Point {
    let s = 1.0 - t;
    let b = [s * s * s, 3.0 * s * s * t, 3.0 * s * t * t, t * t * t];
    Point::new(p.iter().zip(b).map(|(p, b)| p.x * b).sum(), p.iter().zip(b).map(|(p, b)| p.y * b).sum())
}

/// Which lines a split puts through a point of a warp mesh ([`WarpGrid::split`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WarpSplit {
    /// Both lines through the point: a column and a row of anchors.
    Cross,
    /// A line down the mesh, along its columns: it cuts a patch column in two and adds a column
    /// of anchors.
    Vertical,
    /// A line across the mesh, along its rows: it cuts a patch row in two and adds a row of
    /// anchors.
    Horizontal,
}

/// A warp drag or nudge, in document units. A drag's `by` is the whole move from its press, which
/// the host applies to the mesh it had at the press; a nudge's (a [`Set`](EditEvent::Set)) is
/// one step from the mesh it has.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WarpEdit {
    /// Move the control point `index` (an anchor carries its handles along: the host's rule).
    Point { index: usize, by: Vec2 },
    /// Pull the surface at the host's parameters `(s, t)` ([`WarpGrid::surface`]).
    Surface { s: f64, t: f64, by: Vec2 },
    /// Insert lines of anchors: the host's mesh becomes [`WarpMesh::split`] of itself with these
    /// arguments. Always a [`Set`](EditEvent::Set).
    ///
    /// `thirds` is set when the cut is of a one-patch mesh that PhotoCraft's custom warp first
    /// divides at its thirds ([`WarpMesh::commit_thirds`]): the host applies that to its mesh
    /// before `split`, and `column`, `row`, `u` and `v` name a patch of the result.
    Split { split: WarpSplit, column: usize, row: usize, u: f64, v: f64, thirds: bool },
}

/// The AccessKit names of a warp mesh's parts. [`WarpNames::english`] is English, and a host passes
/// its own translated text or [`WarpNames::of`] for the words installed in the context.
pub struct WarpNames<'a> {
    /// The group of parts ("Warp").
    pub group: String,
    /// The name of the anchor at a row and column (from 1): "Row 2, column 3".
    pub anchor: Box<dyn Fn(usize, usize) -> String + 'a>,
    /// The name of a handle of the anchor at a row and column (from 1): "Row 2, column 3, right
    /// handle".
    pub handle: Box<dyn Fn(usize, usize, WarpSide) -> String + 'a>,
}

/// Group, the anchor's name with `{row}` and `{column}`, then a handle's name for the up, left,
/// right and down side.
const WORDS: [Word; 6] = [
    Word::keyed("warp.group", "Warp"),
    Word::keyed("warp.anchor", "Row {row}, column {column}"),
    Word::keyed("warp.handleUp", "Row {row}, column {column}, up handle"),
    Word::keyed("warp.handleLeft", "Row {row}, column {column}, left handle"),
    Word::keyed("warp.handleRight", "Row {row}, column {column}, right handle"),
    Word::keyed("warp.handleDown", "Row {row}, column {column}, down handle"),
];

impl Labels for WarpNames<'static> {
    const WORDS: &'static [Word] = &WORDS;

    fn english() -> Self {
        Self::localized_in(Self::new("", |_, _| String::new(), |_, _, _| String::new()), &Words::english())
    }

    fn localized_in(self, words: &Words) -> Self {
        let group = words.get(WORDS[0]);
        match words.translator() {
            Some(words) => {
                let handles = words.clone();
                Self::new(
                    group,
                    move |row, column| anchor_name(Some(&words), row, column),
                    move |row, column, side| handle_name(Some(&handles), row, column, side),
                )
            }
            None => Self::new(group, |row, column| anchor_name(None, row, column), |row, column, side| handle_name(None, row, column, side)),
        }
    }
}

/// The name of the anchor at a row and column in `words` (English when `None`).
fn anchor_name(words: Option<&Words>, row: usize, column: usize) -> String {
    format_in(words, WORDS[1], &[("row", &row.to_string()), ("column", &column.to_string())])
}

/// The name of a handle of the anchor at a row and column in `words` (English when `None`).
fn handle_name(words: Option<&Words>, row: usize, column: usize, side: WarpSide) -> String {
    let word = match side {
        WarpSide::Up => WORDS[2],
        WarpSide::Left => WORDS[3],
        WarpSide::Right => WORDS[4],
        WarpSide::Down => WORDS[5],
    };
    format_in(words, word, &[("row", &row.to_string()), ("column", &column.to_string())])
}

impl<'a> WarpNames<'a> {
    pub fn new(
        group: impl Into<String>,
        anchor: impl Fn(usize, usize) -> String + 'a,
        handle: impl Fn(usize, usize, WarpSide) -> String + 'a,
    ) -> Self {
        Self { group: group.into(), anchor: Box::new(anchor), handle: Box::new(handle) }
    }
}

/// A warp mesh's look and reach, from [`WarpGrid::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct WarpStyle {
    /// Patch boundaries: `transform.line` of `overlay.line`, bare.
    pub line: Stroke,
    /// A one-patch mesh's rule-of-thirds lines: `transform.warp_minor` of `overlay.line` at 0.7
    /// alpha.
    pub minor: Stroke,
    /// Anchors: [`Handle::style`] `Square` at `transform.warp_anchor`.
    pub anchor: HandleStyle,
    /// Handle ends: [`Handle::style`] `Round` at `transform.warp_control`.
    pub control: HandleStyle,
    /// The arm from an anchor to a shown handle: `transform.warp_minor` of `overlay.handle_stroke`.
    pub arm: Stroke,
    /// The preview of a split line: `transform.line` of `overlay.stroke`.
    pub split: Stroke,
    /// Under the split preview, so it reads on any image: `overlay.halo`, `handle.halo` wider on
    /// each side.
    pub split_halo: Stroke,
    /// Distance from an anchor or a handle within which a press takes it: `transform.hit`.
    pub hit: f32,
    /// Distance from the pointer within which a handle and its arm show, measured to the handle
    /// or its anchor: `transform.warp_reveal`.
    pub reveal: f32,
    /// Largest distance between a curve and the lines it is drawn with: `bezier.flatten`.
    pub flatten: f32,
}

/// What [`WarpGrid::show`] reports.
#[derive(Clone, Debug, PartialEq)]
#[must_use = "apply the warp edits to the host's mesh"]
pub struct WarpResponse {
    /// The grid's id, also its Tab stop's.
    pub id: Id,
    /// A drag (Begin on the first move, Update, Commit on release; Esc cancels it) or an arrow
    /// key's Set.
    pub event: Option<EditEvent<WarpEdit>>,
    /// The control point the keys act on, picked by the pointer or the Page keys.
    pub picked: Option<usize>,
    /// Whether the grid owns the current press: the host skips its tool input.
    pub claimed: bool,
}

/// The host's surface parameters under a document point ([`WarpGrid::surface`]).
type Surface<'a> = Box<dyn Fn(Point) -> Option<(f64, f64)> + 'a>;

/// What a press drags.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WarpTarget {
    /// The anchor or handle with this index.
    Point(usize),
    /// The surface at these parameters ([`WarpGrid::surface`]).
    Surface(f64, f64),
}

/// A press the host-driven calls track, kept by the host from [`WarpGrid::press`] to
/// [`WarpGrid::release`]. [`show`](WarpGrid::show) keeps the same value in the context's memory
/// under the grid's id.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WarpPress {
    target: WarpTarget,
    /// The press in canvas space.
    origin: Pos2,
    began: bool,
    /// The edit the drag last produced.
    last: Option<WarpEdit>,
}

impl WarpPress {
    /// What the press drags.
    pub fn target(&self) -> WarpTarget {
        self.target
    }

    /// Whether the drag has begun (the pointer moved off the press).
    pub fn began(&self) -> bool {
        self.began
    }
}

/// What [`WarpGrid::track`] reports for one pointer position.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WarpTrack {
    /// The drag began with this position: the host takes its "before" state.
    pub begun: bool,
    /// The edit of the whole move from the press, once the drag has begun.
    pub edit: Option<WarpEdit>,
}

/// What [`WarpGrid::release`] reports.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WarpRelease {
    /// The drag ended with this edit (the last one [`WarpGrid::track`] gave).
    Commit(WarpEdit),
    /// The press never became a drag.
    Nothing,
}

/// A split press ([`WarpGrid::split_press`]), kept by the host until [`WarpGrid::split_release`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WarpSplitPress {
    /// A quick split: no tool was set at the press.
    quick: bool,
}

/// A press the grid owns, kept between passes.
#[derive(Clone, Copy, Debug)]
struct Drag {
    press: PrimaryPress,
    core: WarpPress,
    /// Esc ended it: the rest of the press does nothing.
    cancelled: bool,
    /// The press waits for another control's typing run to end ([`Claim::turn`]); its target is
    /// the one at the press and is made again when its turn comes.
    waits: bool,
}

/// The mesh of a warp over a canvas: patch boundaries, anchors and their handles, dragged by the
/// pointer and moved from the keyboard as PhotoCraft's Warp has them. The grid draws, senses and
/// reports [`WarpEdit`]s; the host moves its mesh (an anchor carrying its handles, a pull of the
/// surface) and passes it back each frame.
///
/// ```
/// use craft_ui::canvas::{CanvasView, WarpGrid, WarpMesh, WarpNames, kurbo::{Point, Rect}};
/// # egui::__run_test_ui(|ui| {
/// let (rect, canvas) = ui.allocate_exact_size(egui::vec2(400.0, 300.0), egui::Sense::click_and_drag());
/// let view = CanvasView::new(rect, Point::new(200.0, 150.0), 1.0);
/// let mesh = WarpMesh::identity(Rect::new(100.0, 80.0, 300.0, 220.0), 1, 1);
/// let names = WarpNames::new("Warp", |row, column| format!("Row {row}, column {column}"), |row, column, side| {
///     format!("Row {row}, column {column}, {side:?} handle")
/// });
/// let r = WarpGrid::new("warp", &mesh, Rect::new(100.0, 80.0, 300.0, 220.0), names).show(ui, &canvas, &view);
/// if !r.claimed {
///     // the tool's input
/// }
/// # });
/// ```
///
/// - Look: patch boundaries a bare `transform.line`; a one-patch mesh also its rule-of-thirds
///   lines. Anchors always show; a handle and its arm only within `transform.warp_reveal` of the
///   pointer (from the handle or its anchor), while its anchor or one of its handles is dragged,
///   or while one of its anchor's parts is picked and the grid has the keyboard.
/// - Pointer (screen points): the nearest anchor or handle within `transform.hit` drags that
///   point ([`WarpEdit::Point`]); elsewhere on the surface ([`surface`](Self::surface)) the press
///   pulls it ([`WarpEdit::Surface`]). A drag begins on the first move; Esc cancels it and the
///   rest of the press does nothing. The host shows the grid before its guides and tool input and
///   skips the tool while [`claimed`](WarpResponse::claimed).
/// - Keyboard: one Tab stop ([`PartFocus`]). Page Down and Page Up pick the next and previous
///   part (anchors row by row, each followed by its handles), Home and End the first and last; the
///   arrows move the picked point (⇧ ten steps).
/// - Nothing mirrors in a right-to-left interface: the mesh is the document's.
pub struct WarpGrid<'a> {
    id_salt: IdSalt,
    mesh: &'a WarpMesh,
    bounds: Rect,
    preset: bool,
    names: WarpNames<'a>,
    editable: bool,
    surface: Option<Surface<'a>>,
    units: Option<(&'a RulerUnits, &'a RulerUnits)>,
    zero: Point,
    steps: (f64, f64),
    split: Option<WarpSplit>,
    modifiers: Option<Modifiers>,
    policy: EditPolicy,
}

impl<'a> WarpGrid<'a> {
    /// The host's `mesh` over the source rectangle `bounds` (the box the warp was made on, which
    /// the quick split's reach is measured against), named `names` in the AccessKit tree.
    pub fn new(id_salt: impl AsIdSalt, mesh: &'a WarpMesh, bounds: Rect, names: WarpNames<'a>) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            mesh,
            bounds,
            preset: false,
            names,
            editable: true,
            surface: None,
            units: None,
            zero: Point::ZERO,
            steps: (nudge::STEP, nudge::BIG_STEP),
            split: None,
            modifiers: None,
            policy: EditPolicy::default(),
        }
    }

    /// Whether the mesh's points show and can be dragged; true by default. False draws the patch
    /// boundaries only (PhotoCraft's Warp on type, which takes only the warp styles).
    pub fn editable(mut self, editable: bool) -> Self {
        self.editable = editable;
        self
    }

    /// The host's surface parameters `(s, t)` under a document point, `None` off the surface: a
    /// press there pulls the surface. Without it only the points are dragged.
    pub fn surface(mut self, surface: impl Fn(Point) -> Option<(f64, f64)> + 'a) -> Self {
        self.surface = Some(Box::new(surface));
        self
    }

    /// Whether the mesh is a preset fitted as one patch (PhotoCraft's Arc, Flag and the other
    /// styles), false by default. A split of a custom one-patch mesh first makes its
    /// rule-of-thirds lines real ([`WarpMesh::commit_thirds`]); a preset's is cut as it is.
    pub fn preset(mut self, preset: bool) -> Self {
        self.preset = preset;
        self
    }

    /// The ruler units of the document's x and y axes, for the parts' AccessKit values. Without
    /// them the parts have no value.
    pub fn units(mut self, x: &'a RulerUnits, y: &'a RulerUnits) -> Self {
        self.units = Some((x, y));
        self
    }

    /// The document point the rulers count from, for the AccessKit values.
    pub fn zero(mut self, zero: Point) -> Self {
        self.zero = zero;
        self
    }

    /// The arrow keys' move, in document units, plain and with Shift (1 and 10).
    pub fn steps(mut self, step: f64, big_step: f64) -> Self {
        self.steps = (step, big_step);
        self
    }

    /// The split tool, when one is active (PhotoCraft's Split Warp): a hover over the surface
    /// previews the new lines through the patch under the pointer ([`Cross`](WarpSplit::Cross)
    /// both), and a press and release inserts them where the pointer is released, as one
    /// [`WarpEdit::Split`] [`Set`](EditEvent::Set). While a tool is set presses neither drag
    /// points nor pull the surface; the pointer is the resize arrow across a single line and the
    /// cross hair for a cross, and a pointer on an existing line previews and inserts nothing
    /// there. With a tool set and an anchor focused, Enter splits the anchor's patch (the one
    /// after it, the last one at the mesh's end) at its middle.
    ///
    /// With no tool (`None`) Alt or Ctrl held at the press is a quick split: the tool is chosen
    /// from the pointer by PhotoCraft's rule (a cross in the open, the perpendicular line close
    /// to an existing one).
    pub fn split(mut self, tool: Option<WarpSplit>) -> Self {
        self.split = tool;
        self
    }

    /// The modifiers every test of the grid reads (the quick split, the drag) in place of the
    /// input state: a touch host's sticky modifiers.
    pub fn modifiers(mut self, modifiers: Modifiers) -> Self {
        self.modifiers = Some(modifiers);
        self
    }

    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn style(tokens: &UiTokens) -> WarpStyle {
        let (t, o) = (&tokens.component.transform, &tokens.color.overlay);
        let stroke = o.line.to_egui();
        WarpStyle {
            line: Stroke::new(t.line, stroke),
            minor: Stroke::new(t.warp_minor, stroke.gamma_multiply(0.7)),
            anchor: HandleStyle { size: t.warp_anchor, ..Handle::style(tokens, HandleShape::Square) },
            control: HandleStyle { size: t.warp_control, ..Handle::style(tokens, HandleShape::Round) },
            arm: Stroke::new(t.warp_minor, o.handle_stroke.to_egui()),
            split: Stroke::new(t.line, o.stroke.to_egui()),
            split_halo: Stroke::new(t.line + 2.0 * tokens.component.handle.halo, o.halo.to_egui()),
            hit: t.hit,
            reveal: t.warp_reveal,
            flatten: tokens.component.bezier.flatten,
        }
    }

    /// Draws the mesh over `view` and senses through the host's `canvas` response.
    pub fn show(self, ui: &mut Ui, canvas: &Response, view: &CanvasView) -> WarpResponse {
        let ctx = ui.ctx().clone();
        let id = ui.make_persistent_id(self.id_salt);
        let style = WarpGrid::style(&Tokens::of(ui));
        let mut out = WarpResponse { id, event: None, picked: None, claimed: false };
        if !self.mesh.is_valid() {
            return out;
        }
        let editable = self.editable && ui.is_enabled();
        let screen: Vec<Pos2> = self.mesh.points.iter().map(|&p| view.to_screen(p)).collect();
        let picked_key = id.with("craft_ui::canvas::WarpGrid::picked");
        let mut picked: Option<usize> =
            ctx.data(|d| d.get_temp(picked_key)).filter(|&i: &usize| editable && i < screen.len() && self.mesh.anchor_of(i).is_some());

        // A press whose end the grid missed is dropped, its gesture with it.
        let mut drag: Option<Drag> =
            ctx.data(|d| d.get_temp(drag_key(id))).filter(|d: &Drag| d.press.holds(&ctx) || (d.waits && Claim::is_owner(&ctx, canvas.id, id)));
        let waited = drag.is_some_and(|d| d.waits);
        // The pass in which the press starts: its pass, or the one its turn comes in.
        let mut starts = false;
        if let Some(d) = drag
            && d.waits
        {
            drag = match Claim::turn(&ctx, canvas.id, id) {
                Turn::Waits => drag,
                Turn::Runs => {
                    starts = true;
                    let target = self.target_at(view, &screen, &style, d.press.origin);
                    Some(Drag {
                        core: WarpPress { target: target.unwrap_or(d.core.target), ..d.core },
                        cancelled: target.is_none(),
                        waits: false,
                        ..d
                    })
                }
                Turn::Now => None,
            };
        }
        let mods = self.modifiers.unwrap_or_else(|| ctx.input(|i| i.modifiers));
        let splitting = self.splitting(mods);
        if drag.is_none() && !Claim::keeps_waiting(&ctx, canvas.id, id, waited) && editable && !splitting {
            drag = self.press_pass(&ctx, canvas, view, &style, id, mods);
            starts = drag.is_some_and(|d| !d.waits);
        }
        if starts && let Some(Drag { core: WarpPress { target: WarpTarget::Point(index), .. }, cancelled: false, .. }) = drag {
            picked = Some(index);
        }
        let reported = Gesture::<WarpEdit>::reported(ui, id).map(|(_, value)| value);
        let mut gesture = Gesture::load(ui, id, &reported.unwrap_or(IDLE), self.policy).focus_owners(&[]).enabled(editable);
        if let Some(d) = &mut drag {
            if d.core.began && !gesture.is_active() {
                // Esc, `cancel_all` or a missed pass ended it; the press stays claimed.
                d.cancelled = true;
            }
            if !d.core.began && !d.cancelled && !d.waits && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
                d.cancelled = true;
            }
        }
        if let Some(d) = drag.take() {
            drag = if d.waits { Some(d) } else { self.track_pass(&ctx, view, d, &mut gesture) };
        }

        let parts = if editable { self.parts(&screen, &style, picked) } else { Vec::new() };
        let focus = PartFocus::new(id, &self.names.group).parts(&parts).picked(picked.map(|i| i as u64)).focusable(editable).show(ui, canvas);
        if let Some(pick) = focus.pick {
            picked = pick.map(|k| k as usize);
        }
        if focus.focused
            && editable
            && drag.is_none()
            && let Some(index) = picked
            && let Some(by) = nudge::read(ui, self.steps.0, self.steps.1)
        {
            gesture.set(WarpEdit::Point { index, by });
        }
        if focus.focused
            && editable
            && drag.is_none()
            && let Some(tool) = self.split
            && let Some(index) = picked
            && let Some(anchor) = self.mesh.anchor_of(index)
            && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter))
        {
            // The anchor's corner of the mesh the cut is made on (a one-patch mesh's committed
            // thirds put it at the same corner of the 3 × 3).
            let (base, thirds) = self.split_base();
            let (w, scale) = (self.mesh.width(), if thirds { 3 } else { 1 });
            let (row, column) = ((anchor / w / 3 * scale).min(base.rows() - 1), (anchor % w / 3 * scale).min(base.columns() - 1));
            gesture.set(WarpEdit::Split { split: tool, column, row, u: 0.5, v: 0.5, thirds });
        }
        let preview = if editable { self.split_pass(&ctx, canvas, view, id, mods, &mut gesture) } else { None };
        if editable && !splitting {
            self.cursor(&ctx, canvas, view, &screen, &style, drag);
        }

        out.event = gesture.finish();
        out.claimed = Claim::is_owner(&ctx, canvas.id, id);
        out.picked = picked;
        let held = drag.filter(|d| d.waits || ctx.input(|i| i.pointer.primary_down()));
        ctx.data_mut(|d| {
            match held {
                Some(drag) => {
                    d.insert_temp(drag_key(id), drag);
                }
                None => d.remove::<Drag>(drag_key(id)),
            }
            match picked {
                Some(index) => {
                    d.insert_temp(picked_key, index);
                }
                None => d.remove::<usize>(picked_key),
            }
        });
        let dragged = held.and_then(|d| match d.core.target {
            WarpTarget::Point(index) if !d.cancelled => self.mesh.anchor_of(index),
            _ => None,
        });
        let kept = picked.filter(|_| focus.focused).and_then(|i| self.mesh.anchor_of(i));
        self.paint(ui, view, &screen, &style, editable, [dragged, kept]);
        if let Some(at) = preview {
            for line in at.lines(self.mesh) {
                for stroke in [style.split_halo, style.split] {
                    stroke_path(&ui.painter_at(view.rect), view.affine() * line.clone(), f64::from(style.flatten), stroke);
                }
            }
        }
        if focus.keyboard
            && let Some(index) = picked
        {
            let size = if self.mesh.anchor_of(index) == Some(index) { style.anchor.size } else { style.control.size };
            PartFocus::ring(&ui.painter_at(view.rect), egui::Rect::from_center_size(screen[index], vec2(size, size)));
        }
        out
    }

    /// One pass of the split tool or a quick split: takes a press on the surface (which waits
    /// for another control's typing run to end, like a point's press), inserts at its release,
    /// sets the pointer and returns what the pointer would insert.
    fn split_pass(
        &self,
        ctx: &Context,
        canvas: &Response,
        view: &CanvasView,
        id: Id,
        mods: Modifiers,
        gesture: &mut Gesture<WarpEdit>,
    ) -> Option<WarpSplitAt> {
        let key = id.with("craft_ui::canvas::WarpGrid::split_press");
        let (pointer, pressed, released) = ctx.input(|i| (i.pointer.latest_pos(), i.pointer.primary_pressed(), i.pointer.primary_released()));
        let mut press: Option<SplitPress> =
            ctx.data(|d| d.get_temp(key)).filter(|p: &SplitPress| p.press.holds(ctx) || (p.waits && Claim::is_owner(ctx, canvas.id, id)));
        let waited = press.is_some_and(|p| p.waits);
        if let Some(p) = press
            && p.waits
        {
            press = match Claim::turn(ctx, canvas.id, id) {
                Turn::Waits => Some(p),
                Turn::Runs => Some(SplitPress { waits: false, ..p }),
                Turn::Now => None,
            };
        }
        let quick = self.split.is_none();
        if press.is_none()
            && !Claim::keeps_waiting(ctx, canvas.id, id, waited)
            && pressed
            && let Some(current) = PrimaryPress::current(ctx)
            && view.rect.contains(current.origin)
            && canvas.contains_pointer()
            && let Some(core) = self.split_press(view, current.origin, mods)
            && Claim::take(ctx, canvas.id, id)
        {
            let waits = Claim::turn(ctx, canvas.id, id) == Turn::Waits;
            press = Some(SplitPress { press: current, core, waits, released: None });
        }
        if let Some(p) = &mut press {
            if !p.waits && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
                press = None;
            } else if released && p.released.is_none() {
                p.released = Some(pointer.unwrap_or(p.press.origin));
            }
        }
        if let Some(p) = press
            && !p.waits
            && let Some(end) = p.released
        {
            if let Some(edit) = self.split_release(view, &p.core, end) {
                gesture.set(edit);
            }
            press = None;
        }
        ctx.data_mut(|d| match press {
            Some(p) => {
                d.insert_temp(key, p);
            }
            None => d.remove::<SplitPress>(key),
        });
        let live = press.is_some() || !quick || mods.alt || mods.command;
        let over = canvas.hover_pos().or(pointer).filter(|p| live && view.rect.contains(*p) && canvas.contains_pointer());
        let quick = press.map_or(quick, |p| p.core.quick);
        let hover = over.and_then(|p| self.split_at(view, quick, p)).filter(|_| press.is_some() || Claim::owner(ctx, canvas.id).is_none());
        if let Some(split) = &hover {
            ctx.set_cursor_icon(split.cursor);
        }
        hover
    }

    /// The mesh a split cuts and whether its thirds were made real first.
    fn split_base(&self) -> (Cow<'a, WarpMesh>, bool) {
        let thirds = !self.preset && self.mesh.columns() == 1 && self.mesh.rows() == 1;
        (if thirds { Cow::Owned(self.mesh.commit_thirds()) } else { Cow::Borrowed(self.mesh) }, thirds)
    }

    /// The split press a button pressed at canvas point `origin` with `modifiers` held starts:
    /// with a tool set ([`split`](Self::split)) any press on the surface, else (a quick split) one
    /// with ⌥ or ⌘ held. A press off the surface splits at the nearest point of it. `None` where
    /// every line of the split lies on an existing one. The host keeps the press until [`split_release`](Self::split_release).
    pub fn split_press(&self, view: &CanvasView, origin: Pos2, modifiers: Modifiers) -> Option<WarpSplitPress> {
        let mods = self.modifiers.unwrap_or(modifiers);
        let quick = self.split.is_none();
        if !self.editable || !self.mesh.is_valid() || (quick && !(mods.alt || mods.command)) {
            return None;
        }
        self.split_at(view, quick, origin).map(|_| WarpSplitPress { quick })
    }

    /// The edit a split press inserts when the button is released at canvas point `end`: always a
    /// [`Set`](EditEvent::Set). A point off the surface splits at the nearest point of it. `None`
    /// where every line lies on an existing one.
    pub fn split_release(&self, view: &CanvasView, press: &WarpSplitPress, end: Pos2) -> Option<WarpEdit> {
        self.split_at(view, press.quick, end).map(|at| at.edit())
    }

    /// What the split would insert for a pointer at canvas point `p`, for the preview line and
    /// the cursor; `press` is the split press under way, if any, else a quick split is the one
    /// `modifiers` start.
    pub fn split_hit(&self, view: &CanvasView, press: Option<&WarpSplitPress>, p: Pos2, modifiers: Modifiers) -> Option<WarpSplitAt> {
        let mods = self.modifiers.unwrap_or(modifiers);
        let quick = self.split.is_none();
        if !self.editable || !self.mesh.is_valid() || (press.is_none() && quick && !(mods.alt || mods.command)) {
            return None;
        }
        self.split_at(view, press.map_or(quick, |p| p.quick), p)
    }

    /// What the split would insert for a pointer at screen point `p`: the armed tool's lines, or
    /// with `quick` the quick split's. A point off the surface splits at the nearest point of it.
    /// `None` where every line of the split lies on an existing one; a cross with one line on an existing one inserts the other alone.
    fn split_at(&self, view: &CanvasView, quick: bool, p: Pos2) -> Option<WarpSplitAt> {
        const EDGE: f64 = 0.01;
        let doc = view.to_doc(p);
        let (mesh, thirds) = self.split_base();
        let (column, row, u, v, _) = mesh.nearest(doc);
        let kind = self.split.filter(|_| !quick).unwrap_or_else(|| mesh.quick_split(self.bounds, column, row, u, v));
        let free = |c: f64| (EDGE..=1.0 - EDGE).contains(&c);
        let split = match (kind != WarpSplit::Horizontal && free(u), kind != WarpSplit::Vertical && free(v)) {
            (true, true) => WarpSplit::Cross,
            (true, false) => WarpSplit::Vertical,
            (false, true) => WarpSplit::Horizontal,
            (false, false) => return None,
        };
        // The way a line moves as its parameter grows, for the arrow across it.
        let step = 1e-3;
        let across = |along_u: bool| {
            let at = |d: f64| if along_u { mesh.patch_point(column, row, d, v) } else { mesh.patch_point(column, row, u, d) };
            let c = if along_u { u } else { v };
            at((c + step).min(1.0)) - at((c - step).max(0.0))
        };
        let cursor = match kind {
            WarpSplit::Cross => CursorIcon::Crosshair,
            WarpSplit::Vertical => resize_cursor(view, across(true)),
            WarpSplit::Horizontal => resize_cursor(view, across(false)),
        };
        Some(WarpSplitAt { split, column, row, u, v, thirds, cursor })
    }

    /// Whether a press with `modifiers` held (or the grid's own, [`modifiers`](Self::modifiers))
    /// is a split press: a tool is set, or ⌥ or ⌘ is held with none.
    fn splitting(&self, modifiers: Modifiers) -> bool {
        let mods = self.modifiers.unwrap_or(modifiers);
        self.editable && (self.split.is_some() || mods.alt || mods.command)
    }

    /// What a press at canvas point `point` with `modifiers` held would drag: the nearest anchor
    /// or handle within `style.hit`, else the surface under it ([`surface`](Self::surface));
    /// `None` where the grid is not editable, the point is off the surface, or the press is a
    /// split press ([`split_press`](Self::split_press)).
    pub fn hit(&self, view: &CanvasView, style: &WarpStyle, point: Pos2, modifiers: Modifiers) -> Option<WarpTarget> {
        if !self.editable || !self.mesh.is_valid() || self.splitting(modifiers) {
            return None;
        }
        let screen: Vec<Pos2> = self.mesh.points.iter().map(|&p| view.to_screen(p)).collect();
        self.target_at(view, &screen, style, point)
    }

    /// A press at canvas point `origin` with `modifiers` held: the press to track, or `None`
    /// where [`hit`](Self::hit) finds no target.
    pub fn press(&self, view: &CanvasView, style: &WarpStyle, origin: Pos2, modifiers: Modifiers) -> Option<WarpPress> {
        let target = self.hit(view, style, origin, modifiers)?;
        Some(WarpPress { target, origin, began: false, last: None })
    }

    /// The pointer at canvas point `pointer`: begins the drag on the first move off the press and
    /// gives the edit of the whole move from it. The host applies it to the mesh it had at the
    /// press.
    pub fn track(&self, view: &CanvasView, press: &mut WarpPress, pointer: Pos2) -> WarpTrack {
        self.track_with(view, press, pointer, &mut || true)
    }

    /// [`track`](Self::track) where beginning the drag may be refused: `begin` answers whether it
    /// began.
    fn track_with(&self, view: &CanvasView, press: &mut WarpPress, pointer: Pos2, begin: &mut dyn FnMut() -> bool) -> WarpTrack {
        let mut track = WarpTrack { begun: false, edit: None };
        if !press.began && pointer != press.origin {
            press.began = begin();
            track.begun = press.began;
        }
        if press.began {
            let by = view.to_doc(pointer) - view.to_doc(press.origin);
            let edit = match press.target {
                WarpTarget::Point(index) => WarpEdit::Point { index, by },
                WarpTarget::Surface(s, t) => WarpEdit::Surface { s, t, by },
            };
            press.last = Some(edit);
            track.edit = Some(edit);
        }
        track
    }

    /// The button released: the end of the drag, if it began.
    pub fn release(&self, press: &WarpPress) -> WarpRelease {
        match press.last.filter(|_| press.began) {
            Some(edit) => WarpRelease::Commit(edit),
            None => WarpRelease::Nothing,
        }
    }

    /// A press on the canvas read from the input: the nearest point within reach, else the
    /// surface under it, claimed for the grid.
    fn press_pass(&self, ctx: &Context, canvas: &Response, view: &CanvasView, style: &WarpStyle, id: Id, mods: Modifiers) -> Option<Drag> {
        if !ctx.input(|i| i.pointer.primary_pressed()) {
            return None;
        }
        let press = PrimaryPress::current(ctx)?;
        if !(view.rect.contains(press.origin) && canvas.contains_pointer()) {
            return None;
        }
        let core = self.press(view, style, press.origin, mods)?;
        if !Claim::take(ctx, canvas.id, id) {
            return None;
        }
        let waits = Claim::turn(ctx, canvas.id, id) == Turn::Waits;
        Some(Drag { press, core, cancelled: false, waits })
    }

    /// What a press at screen point `p` would drag.
    fn target_at(&self, view: &CanvasView, screen: &[Pos2], style: &WarpStyle, p: Pos2) -> Option<WarpTarget> {
        let nearest = (0..screen.len())
            .filter(|&i| self.mesh.anchor_of(i).is_some())
            .map(|i| (i, hit::point(p, screen[i])))
            .filter(|&(_, d)| d < style.hit)
            .min_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((index, _)) = nearest {
            return Some(WarpTarget::Point(index));
        }
        let (s, t) = self.surface.as_ref()?(view.to_doc(p))?;
        Some(WarpTarget::Surface(s, t))
    }

    /// One pass of a press the grid owns: the press that goes on, if it does.
    fn track_pass(&self, ctx: &Context, view: &CanvasView, mut d: Drag, gesture: &mut Gesture<WarpEdit>) -> Option<Drag> {
        let (pointer, down, input_mods) = ctx.input(|i| (i.pointer.latest_pos(), i.pointer.primary_down(), i.modifiers));
        let mods = self.modifiers.unwrap_or(input_mods);
        if d.cancelled {
            return down.then_some(d);
        }
        if let Some(pointer) = pointer {
            let track = self.track_with(view, &mut d.core, pointer, &mut || {
                gesture.begin();
                gesture.is_active()
            });
            if let Some(edit) = track.edit {
                gesture.update(edit, mods);
            }
        }
        if down {
            return Some(d);
        }
        if let WarpRelease::Commit(_) = self.release(&d.core) {
            gesture.commit();
        }
        None
    }

    /// The pointer (PhotoCraft's): a cross hair over a point, a hand over the surface.
    fn cursor(&self, ctx: &Context, canvas: &Response, view: &CanvasView, screen: &[Pos2], style: &WarpStyle, drag: Option<Drag>) {
        let target = match drag {
            Some(d) => (!d.cancelled).then_some(d.core.target),
            None if Claim::owner(ctx, canvas.id).is_none() => canvas.hover_pos().and_then(|p| self.target_at(view, screen, style, p)),
            None => None,
        };
        match target {
            Some(WarpTarget::Point(_)) => ctx.set_cursor_icon(CursorIcon::Crosshair),
            Some(WarpTarget::Surface(..)) => ctx.set_cursor_icon(if drag.is_some() { CursorIcon::Grabbing } else { CursorIcon::Grab }),
            None => {}
        }
    }

    /// The keyboard parts and their AccessKit nodes: anchors row by row, each followed by its
    /// handles.
    fn parts(&self, screen: &[Pos2], style: &WarpStyle, picked: Option<usize>) -> Vec<Part> {
        let w = self.mesh.width();
        let reach = vec2(2.0 * style.hit, 2.0 * style.hit);
        let value = |index: usize| {
            let p = self.mesh.points[index];
            self.units.map(|(x, y)| format!("{}, {}", x.format_value(p.x, self.zero.x), y.format_value(p.y, self.zero.y)))
        };
        let part = |index: usize, name: String| Part {
            key: index as u64,
            name,
            value: value(index),
            rect: egui::Rect::from_center_size(screen[index], reach),
            selected: picked == Some(index),
        };
        let mut parts = Vec::new();
        for anchor in self.mesh.anchors() {
            let (row, column) = (anchor / w / 3 + 1, anchor % w / 3 + 1);
            parts.push(part(anchor, (self.names.anchor)(row, column)));
            for (index, side) in self.mesh.handles(anchor) {
                parts.push(part(index, (self.names.handle)(row, column, side)));
            }
        }
        parts
    }

    /// Draws the patch boundaries, a one-patch mesh's thirds and, when `editable`, the anchors and
    /// the handles the pointer is near or whose anchor is in `kept`.
    fn paint(&self, ui: &Ui, view: &CanvasView, screen: &[Pos2], style: &WarpStyle, editable: bool, kept: [Option<usize>; 2]) {
        let painter = ui.painter_at(view.rect);
        let to_screen = view.affine();
        let tolerance = f64::from(style.flatten);
        if self.mesh.columns() == 1 && self.mesh.rows() == 1 {
            for path in self.mesh.thirds() {
                stroke_path(&painter, to_screen * path, tolerance, style.minor);
            }
        }
        for path in self.mesh.boundaries() {
            stroke_path(&painter, to_screen * path, tolerance, style.line);
        }
        if !editable {
            return;
        }
        let pointer = ui.input(|i| i.pointer.hover_pos()).filter(|p| view.rect.contains(*p));
        let near = |p: Pos2| pointer.is_some_and(|h| h.distance(p) <= style.reveal);
        let ppp = ui.pixels_per_point();
        let mut ends = Vec::new();
        for anchor in self.mesh.anchors() {
            let a = screen[anchor];
            for (index, _) in self.mesh.handles(anchor) {
                let p = screen[index];
                if near(p) || near(a) || kept.contains(&Some(anchor)) {
                    painter.line_segment([a, p], style.arm);
                    ends.push(p);
                }
            }
        }
        for anchor in self.mesh.anchors() {
            painter.extend(Handle::shapes_with(&style.anchor, HandleShape::Square, screen[anchor], ppp));
        }
        for p in ends {
            painter.extend(Handle::shapes_with(&style.control, HandleShape::Round, p, ppp));
        }
    }
}

/// A split press the grid owns, kept between passes.
#[derive(Clone, Copy, Debug)]
struct SplitPress {
    press: PrimaryPress,
    core: WarpSplitPress,
    /// The press waits for another control's typing run to end ([`Claim::turn`]); the insert is
    /// made against the host's mesh when its turn comes.
    waits: bool,
    /// Where the button was released while the press waited.
    released: Option<Pos2>,
}

/// The lines the pointer would insert ([`WarpGrid::split_hit`]).
#[derive(Clone, Copy, Debug)]
pub struct WarpSplitAt {
    split: WarpSplit,
    column: usize,
    row: usize,
    u: f64,
    v: f64,
    thirds: bool,
    cursor: CursorIcon,
}

impl WarpSplitAt {
    /// The pointer for the split: a cross hair for a cross, else the arrow across its line.
    pub fn cursor(&self) -> CursorIcon {
        self.cursor
    }

    /// The edit the split inserts.
    pub fn edit(&self) -> WarpEdit {
        WarpEdit::Split { split: self.split, column: self.column, row: self.row, u: self.u, v: self.v, thirds: self.thirds }
    }

    /// The new lines of `mesh` (the host's mesh), in document units.
    pub fn lines(&self, mesh: &WarpMesh) -> Vec<BezPath> {
        let base = if self.thirds { mesh.commit_thirds() } else { mesh.clone() };
        let cut = base.split(self.split, self.column, self.row, self.u, self.v);
        let column = || cut.column_path(3 * self.column + 3);
        let row = || cut.row_path(3 * self.row + 3);
        match self.split {
            WarpSplit::Vertical => vec![column()],
            WarpSplit::Horizontal => vec![row()],
            WarpSplit::Cross => vec![column(), row()],
        }
    }
}

/// The gesture's host value while idle: an edit that moves nothing, so a key's Set always differs.
const IDLE: WarpEdit = WarpEdit::Point { index: usize::MAX, by: Vec2::ZERO };

fn drag_key(id: Id) -> Id {
    id.with("craft_ui::canvas::WarpGrid::drag")
}

/// Strokes a screen-space path of open curves as polylines within `tolerance` of them.
fn stroke_path(painter: &Painter, path: BezPath, tolerance: f64, stroke: Stroke) {
    let mut lines: Vec<Vec<Pos2>> = Vec::new();
    let pos = |p: Point| Pos2::new(p.x as f32, p.y as f32);
    kurbo::flatten(&path, tolerance, |el| match el {
        PathEl::MoveTo(p) => lines.push(vec![pos(p)]),
        PathEl::LineTo(p) => {
            if let Some(points) = lines.last_mut() {
                points.push(pos(p));
            }
        }
        _ => {}
    });
    painter.extend(lines.into_iter().filter(|l| l.len() > 1).map(|l| Shape::line(l, stroke)));
}

#[cfg(test)]
mod tests;
