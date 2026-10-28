//! A slider track drawn as a colour gradient ([`Slider::track_colors`](super::Slider::track_colors)),
//! over the checkerboard where a stop is translucent.

use craft_theme::Srgba;
use egui::{Color32, Mesh, Rect, Ui, pos2};

use super::slider::{SliderStyle, Track};

/// Points sampled along each rounded end of the track.
const CORNER_SAMPLES: usize = 8;

/// Paints the track as the gradient through `stops`, spread evenly from where the thumb's centre
/// is at the minimum to where it is at the maximum, so the colour under the thumb is the colour
/// of its value; past those points the end colours continue to the track's ends. The track keeps
/// its shape and edge; translucent stops show the swatch checkerboard (`checker`) under them.
pub(super) fn paint(ui: &Ui, style: &SliderStyle, track: &Track, stops: &[Srgba], checker: [Color32; 2]) {
    let ppp = ui.pixels_per_point();
    let bar = track.bar(ppp);
    if stops.is_empty() || bar.width() <= 0.0 || bar.height() <= 0.0 {
        return;
    }
    let radius = f32::from(style.track.radius.nw).min(bar.height() / 2.0).min(bar.width() / 2.0);
    let shape = Rounded { bar, radius };
    let colors: Vec<Color32> = stops.iter().map(|&s| s.into()).collect();
    let (x0, x1) = track.ends();
    let stop_x = |i: usize| if colors.len() < 2 { x0 } else { egui::lerp(x0..=x1, i as f32 / (colors.len() - 1) as f32) };
    let color_at = |x: f32| -> Color32 {
        if colors.len() < 2 || x <= x0 {
            return colors[0];
        }
        if x >= x1 {
            return colors[colors.len() - 1];
        }
        let t = (x - x0) / (x1 - x0) * (colors.len() - 1) as f32;
        let i = (t.floor() as usize).min(colors.len() - 2);
        colors[i].lerp_to_gamma(colors[i + 1], t - i as f32)
    };

    let mut xs = shape.samples();
    xs.extend((0..colors.len()).map(stop_x));
    let painter = ui.painter();
    if stops.iter().any(|s| s.a < 1.0) {
        // Two rows of square cells.
        let cell = bar.height() / 2.0;
        let mut cells = xs.clone();
        let count = (bar.width() / cell).ceil() as usize;
        cells.extend((1..count).map(|i| bar.left() + i as f32 * cell));
        let rows = [(bar.top(), bar.top() + cell), (bar.top() + cell, bar.bottom())];
        let cell_of = |x: f32| ((x - bar.left()) / cell).floor() as usize;
        painter.add(shape.mesh(cells, &rows, |_, mid, row| checker[(cell_of(mid) + row) % 2]));
    }
    painter.add(shape.mesh(xs, &[(bar.top(), bar.bottom())], |x, _, _| color_at(x)));
    let mut edge = style.track;
    edge.fill = Color32::TRANSPARENT;
    painter.extend(edge.shapes(bar, ppp));
}

/// A horizontal bar whose ends are rounded with `radius`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Rounded {
    pub(crate) bar: Rect,
    pub(crate) radius: f32,
}

impl Rounded {
    /// The x positions that trace the rounded ends: the bar's ends and points along each corner.
    pub(crate) fn samples(&self) -> Vec<f32> {
        let mut xs = vec![self.bar.left(), self.bar.right()];
        if self.radius > 0.0 {
            for i in 1..=CORNER_SAMPLES {
                let d = self.radius * i as f32 / CORNER_SAMPLES as f32;
                xs.extend([self.bar.left() + d, self.bar.right() - d]);
            }
        }
        xs
    }

    /// The bar's top and bottom at `x`.
    fn span_at(&self, x: f32) -> (f32, f32) {
        let (left, right) = (self.bar.left() + self.radius, self.bar.right() - self.radius);
        let d = if x < left {
            left - x
        } else if x > right {
            x - right
        } else {
            0.0
        };
        let r = self.radius;
        let inset = if d > 0.0 { r - (r * r - d.min(r) * d.min(r)).sqrt() } else { 0.0 };
        (self.bar.top() + inset, self.bar.bottom() - inset)
    }

    /// A mesh over the bar in vertical slices between the positions `xs` (sorted here) and the
    /// horizontal `rows`, cut to the rounded outline. `color(x, mid, row)` colours the vertex at
    /// `x` of the slice whose centre is `mid`.
    pub(crate) fn mesh(&self, mut xs: Vec<f32>, rows: &[(f32, f32)], color: impl Fn(f32, f32, usize) -> Color32) -> Mesh {
        xs.retain(|x| x.is_finite());
        xs.iter_mut().for_each(|x| *x = x.clamp(self.bar.left(), self.bar.right()));
        xs.sort_by(f32::total_cmp);
        xs.dedup_by(|a, b| (*a - *b).abs() < 1e-3);
        let mut mesh = Mesh::default();
        for (row, &(top, bottom)) in rows.iter().enumerate() {
            for pair in xs.windows(2) {
                let (a, b) = (pair[0], pair[1]);
                let mid = (a + b) / 2.0;
                let cut = |x: f32| {
                    let (t, btm) = self.span_at(x);
                    let t = t.max(top);
                    (t, btm.min(bottom).max(t))
                };
                let ((at, ab), (bt, bb)) = (cut(a), cut(b));
                if ab <= at && bb <= bt {
                    continue;
                }
                let base = mesh.vertices.len() as u32;
                let (ca, cb) = (color(a, mid, row), color(b, mid, row));
                mesh.colored_vertex(pos2(a, at), ca);
                mesh.colored_vertex(pos2(b, bt), cb);
                mesh.colored_vertex(pos2(b, bb), cb);
                mesh.colored_vertex(pos2(a, ab), ca);
                mesh.add_triangle(base, base + 1, base + 2);
                mesh.add_triangle(base, base + 2, base + 3);
            }
        }
        mesh
    }
}
