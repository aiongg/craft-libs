//! The warp grid's story and the host the story and tests share: a mesh that applies the grid's
//! edits as PhotoCraft's Warp does in outline (an anchor carries its handles, a pull moves the
//! patch's control points so the surface point follows the pointer).

use egui::{Color32, Mesh, Ui};
use kurbo::{Point, Rect};

use super::{WarpEdit, WarpGrid, WarpMesh, WarpNames, WarpResponse, WarpSide, WarpSplit, cubic};
use crate::canvas::RulerUnits;
use crate::edit::EditEvent;
use crate::localized::LocaleNumbers;
use crate::story::{Args, Level, Story};

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Warp grid"];

pub(crate) fn stories() -> Vec<Story> {
    vec![Story {
        id: "primitives/warp-grid/warp",
        level: Level::Primitives,
        component: "Warp grid",
        name: "Warp",
        args: || {
            super::super::story::args().with_choice("grid", &["1 × 1", "3 × 3"], 0).with_bool("editable", true).with_choice(
                "split",
                &["Off", "Cross", "Vertical", "Horizontal"],
                0,
            )
        },
        show: show_warp,
    }]
}

/// The English names the story and tests use.
pub(crate) fn english_names() -> WarpNames<'static> {
    WarpNames::new(
        "Warp",
        |row, column| format!("Row {row}, column {column}"),
        |row, column, side| {
            let side = match side {
                WarpSide::Up => "up",
                WarpSide::Left => "left",
                WarpSide::Right => "right",
                WarpSide::Down => "down",
            };
            format!("Row {row}, column {column}, {side} handle")
        },
    )
}

/// A host's warp mesh, as the story and tests keep it: updates shown as they come.
#[derive(Clone, Debug)]
pub(crate) struct WarpHost {
    pub mesh: WarpMesh,
    /// The mesh at the press, which a drag's edits apply to and a cancel restores.
    pub before: Option<WarpMesh>,
}

impl WarpHost {
    pub(crate) fn new(mesh: WarpMesh) -> Self {
        Self { mesh, before: None }
    }

    pub(crate) fn apply(&mut self, response: &WarpResponse) {
        match &response.event {
            Some(EditEvent::Begin { .. }) => self.before = Some(self.mesh.clone()),
            Some(EditEvent::Update { value, .. }) => {
                if let Some(before) = &self.before {
                    self.mesh = edited(before, value);
                }
            }
            Some(EditEvent::Commit { value, .. }) => {
                let before = self.before.take().unwrap_or_else(|| self.mesh.clone());
                self.mesh = edited(&before, value);
            }
            Some(EditEvent::Set { value, .. }) => self.mesh = edited(&self.mesh, value),
            Some(EditEvent::Cancel { .. }) => {
                if let Some(before) = self.before.take() {
                    self.mesh = before;
                }
            }
            None => {}
        }
    }
}

fn bernstein(t: f64) -> [f64; 4] {
    let s = 1.0 - t;
    [s * s * s, 3.0 * s * s * t, 3.0 * s * t * t, t * t * t]
}

/// The surface point at the mesh parameters `(s, t)`.
pub(crate) fn eval(mesh: &WarpMesh, s: f64, t: f64) -> Point {
    let (column, row, u, v) = mesh.locate(s, t);
    let rows: [Point; 4] = std::array::from_fn(|b| cubic(std::array::from_fn(|a| mesh.point(3 * column + a, 3 * row + b)), u));
    cubic(rows, v)
}

/// The mesh parameters of the surface point at `p`, `None` off the surface: the nearest of a
/// sampled grid refined by Newton steps.
pub(crate) fn surface_at(mesh: &WarpMesh, p: Point) -> Option<(f64, f64)> {
    let n = 16 * mesh.columns().max(mesh.rows());
    let (mut s, mut t) = (0..=n)
        .flat_map(|i| (0..=n).map(move |j| (i as f64 / n as f64, j as f64 / n as f64)))
        .min_by(|a, b| eval(mesh, a.0, a.1).distance(p).total_cmp(&eval(mesh, b.0, b.1).distance(p)))?;
    let h = 1e-5;
    for _ in 0..12 {
        let q = eval(mesh, s, t);
        let ds = (eval(mesh, s + h, t) - eval(mesh, s - h, t)) / (2.0 * h);
        let dt = (eval(mesh, s, t + h) - eval(mesh, s, t - h)) / (2.0 * h);
        let det = ds.x * dt.y - ds.y * dt.x;
        if det.abs() < 1e-12 {
            break;
        }
        let r = p - q;
        s = (s + (r.x * dt.y - r.y * dt.x) / det).clamp(0.0, 1.0);
        t = (t + (ds.x * r.y - ds.y * r.x) / det).clamp(0.0, 1.0);
    }
    (eval(mesh, s, t).distance(p) < 1.0).then_some((s, t))
}

/// `mesh` with `edit` applied.
pub(crate) fn edited(mesh: &WarpMesh, edit: &WarpEdit) -> WarpMesh {
    let mut out = mesh.clone();
    let w = mesh.width();
    match *edit {
        WarpEdit::Point { index, by } if index < mesh.points.len() => {
            out.points[index] += by;
            if mesh.anchor_of(index) == Some(index) {
                // An anchor carries its handles and the twist points beside it.
                let (i, j) = ((index % w) as isize, (index / w) as isize);
                for (di, dj) in [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)] {
                    let (x, y) = (i + di, j + dj);
                    if (0..w as isize).contains(&x) && (0..mesh.height() as isize).contains(&y) {
                        out.points[y as usize * w + x as usize] += by;
                    }
                }
            }
        }
        WarpEdit::Point { .. } => {}
        WarpEdit::Split { split, column, row, u, v, thirds } => {
            let base = if thirds { mesh.commit_thirds() } else { mesh.clone() };
            return base.split(split, column, row, u, v);
        }
        WarpEdit::Surface { s, t, by } => {
            // The least change of the patch's control points that moves the surface point by `by`.
            let (column, row, u, v) = mesh.locate(s, t);
            let (bu, bv) = (bernstein(u), bernstein(v));
            let sum: f64 = bu.iter().flat_map(|a| bv.iter().map(move |b| (a * b).powi(2))).sum();
            for (b, wv) in bv.iter().enumerate() {
                for (a, wu) in bu.iter().enumerate() {
                    out.points[(3 * row + b) * w + 3 * column + a] += by * (wu * wv / sum);
                }
            }
        }
    }
    out
}

/// The story's picture before any warp.
const PICTURE: Rect = Rect::new(160.0, 100.0, 480.0, 300.0);

/// A checked picture warped by the mesh, a 1 × 1 (rule-of-thirds lines) or 3 × 3 grid over it.
fn show_warp(ui: &mut Ui, args: &mut Args) {
    let key = ui.scope_id().with("warp-story");
    let grid = args.choice("grid").to_owned();
    let size = if grid == "3 × 3" { 3 } else { 1 };
    let mut host = ui
        .data(|d| d.get_temp::<(String, WarpHost)>(key))
        .filter(|(g, _)| *g == grid)
        .map_or_else(|| WarpHost::new(WarpMesh::identity(PICTURE, size, size)), |(_, host)| host);
    let canvas = super::super::story::document(ui, args, "Document");
    let view = &canvas.view;
    paint_picture(ui, view, &host.mesh);
    let px = RulerUnits::pixels(LocaleNumbers::from_tag("en").ungrouped());
    let mesh = host.mesh.clone();
    let r = WarpGrid::new("story-warp", &mesh, PICTURE, english_names())
        .editable(args.bool("editable"))
        .split(match args.choice("split") {
            "Cross" => Some(WarpSplit::Cross),
            "Vertical" => Some(WarpSplit::Vertical),
            "Horizontal" => Some(WarpSplit::Horizontal),
            _ => None,
        })
        .surface(|p| surface_at(&mesh, p))
        .units(&px, &px)
        .show(ui, &canvas.response, view);
    host.apply(&r);
    ui.data_mut(|d| d.insert_temp(key, (grid, host)));
}

/// The picture through the mesh: a checker of the mesh parameters, sampled finely.
fn paint_picture(ui: &Ui, view: &crate::canvas::CanvasView, mesh: &WarpMesh) {
    const N: u32 = 24;
    let at: Vec<egui::Pos2> = (0..=N)
        .flat_map(|j| (0..=N).map(move |i| (f64::from(i) / f64::from(N), f64::from(j) / f64::from(N))))
        .map(|(s, t)| view.to_screen(eval(mesh, s, t)))
        .collect();
    let colors = [Color32::from_rgba_unmultiplied(70, 130, 180, 220), Color32::from_rgba_unmultiplied(230, 160, 60, 220)];
    let row = N + 1;
    let mut cells = Mesh::default();
    for j in 0..N {
        for i in 0..N {
            let color = colors[((i / 4 + j / 4) % 2) as usize];
            let base = cells.vertices.len() as u32;
            for k in [j * row + i, j * row + i + 1, (j + 1) * row + i + 1, (j + 1) * row + i] {
                cells.colored_vertex(at[k as usize], color);
            }
            cells.add_triangle(base, base + 1, base + 2);
            cells.add_triangle(base, base + 2, base + 3);
        }
    }
    ui.painter_at(view.rect).add(cells);
}
