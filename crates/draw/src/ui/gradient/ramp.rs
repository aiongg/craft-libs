//! [`GradientRamp`]: a gradient drawn across a rect over the checkerboard.

use craft_theme::Srgba;
use egui::emath::GuiRounding as _;
use egui::{Color32, Painter, Rect};

use super::stops::Sampler;
use super::value::{GradientStops, StopRow};
use craft_ui::controls::gradient_track::Rounded;
use craft_ui::theme::{ToEgui, UiTokens};

/// Even samples added to a stop bar's own, so a custom sampler's curve shows.
const EVEN_SAMPLES: usize = 64;

/// What a ramp draws with, from [`GradientRamp::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct GradientRampStyle {
    /// The checkerboard behind translucent parts: `swatch.checker.a` holds the corners,
    /// `swatch.checker.b` the other cells.
    pub checker: [Color32; 2],
    /// Side of a checkerboard cell, as a swatch's: `swatch.checker_cell`. The ramp rounds it so
    /// whole cells fill its height.
    pub cell: f32,
    /// The one-point edge round a stop bar's ramp: `gradient_editor.edge`.
    pub edge: Color32,
}

/// Paint for a gradient: a mesh across a rect, over the checkerboard where it is translucent.
/// The ramp has no edge; [`GradientSwatch`](super::GradientSwatch) draws a swatch's rings and the
/// stop bar the edge in [`GradientRampStyle::edge`].
#[derive(Clone, Copy, Debug)]
pub struct GradientRamp;

impl GradientRamp {
    pub fn style(tokens: &UiTokens) -> GradientRampStyle {
        let (s, g) = (&tokens.component.swatch, &tokens.component.gradient_editor);
        GradientRampStyle { checker: [s.checker_a.to_egui(), s.checker_b.to_egui()], cell: s.checker_cell, edge: g.edge.to_egui() }
    }

    /// The colours to draw `stops` with at the locations a mesh needs: the ends, every stop and
    /// midpoint of both rows, and even steps between, from `sampler`.
    pub fn colors(stops: &GradientStops, sampler: &Sampler) -> Vec<(f32, Srgba)> {
        let mut at = vec![0.0, 1.0];
        at.extend((1..EVEN_SAMPLES).map(|k| k as f32 / EVEN_SAMPLES as f32));
        let mut rows = vec![(StopRow::Color, stops.color.iter().map(|s| (s.location, s.midpoint)).collect::<Vec<_>>())];
        rows.push((StopRow::Opacity, stops.opacity.iter().map(|s| (s.location, s.midpoint)).collect()));
        for (_, list) in &rows {
            for (k, &(a, m)) in list.iter().enumerate() {
                at.push(a);
                if let Some(&(b, _)) = list.get(k + 1) {
                    at.push(a + m * (b - a));
                }
            }
        }
        at.retain(|x| x.is_finite());
        at.iter_mut().for_each(|x| *x = x.clamp(0.0, 1.0));
        at.sort_by(f32::total_cmp);
        at.dedup_by(|a, b| (*a - *b).abs() < 1e-4);
        at.into_iter().map(|x| (x, sampler(stops, x))).collect()
    }

    /// Paints `stops` across `rect`, coloured by `sampler`.
    pub fn paint(painter: &Painter, rect: Rect, stops: &GradientStops, sampler: &Sampler, tokens: &UiTokens) {
        Self::paint_colors(painter, rect, &Self::colors(stops, sampler), tokens);
    }

    /// Paints `colors` (location 0..=1, sorted) across `rect`, straight lines between them, for a
    /// host that samples its gradient itself; the end colours continue to the rect's ends.
    pub fn paint_colors(painter: &Painter, rect: Rect, colors: &[(f32, Srgba)], tokens: &UiTokens) {
        Self::paint_rounded(painter, rect, 0.0, colors, &Self::style(tokens));
    }

    /// [`paint_colors`](Self::paint_colors) in a rect with corners of `radius`.
    pub(crate) fn paint_rounded(painter: &Painter, rect: Rect, radius: f32, colors: &[(f32, Srgba)], style: &GradientRampStyle) {
        let rect = rect.round_to_pixels(painter.ctx().pixels_per_point());
        if colors.is_empty() || rect.width() <= 0.0 || rect.height() <= 0.0 {
            return;
        }
        let shape = Rounded { bar: rect, radius: radius.min(rect.height() / 2.0).min(rect.width() / 2.0) };
        let x_of = |location: f32| rect.left() + location.clamp(0.0, 1.0) * rect.width();
        let shown: Vec<(f32, Color32)> = colors.iter().map(|&(l, c)| (x_of(l), c.into())).collect();
        let color_at = |x: f32| -> Color32 {
            let (first, last) = (shown[0], shown[shown.len() - 1]);
            if x <= first.0 {
                return first.1;
            }
            if x >= last.0 {
                return last.1;
            }
            let k = shown.partition_point(|&(px, _)| px <= x).clamp(1, shown.len() - 1);
            let ((xa, ca), (xb, cb)) = (shown[k - 1], shown[k]);
            if xb <= xa { cb } else { ca.lerp_to_gamma(cb, (x - xa) / (xb - xa)) }
        };

        let mut xs = shape.samples();
        if colors.iter().any(|(_, c)| c.a < 1.0) {
            let rows = (rect.height() / style.cell).round().max(1.0);
            let cell = rect.height() / rows;
            let mut cells = xs.clone();
            cells.extend((1..(rect.width() / cell).ceil() as usize).map(|i| rect.left() + i as f32 * cell));
            let bands: Vec<(f32, f32)> = (0..rows as usize).map(|r| (rect.top() + r as f32 * cell, rect.top() + (r + 1) as f32 * cell)).collect();
            let cell_of = |x: f32| ((x - rect.left()) / cell).floor() as usize;
            painter.add(shape.mesh(cells, &bands, |_, mid, row| style.checker[(cell_of(mid) + row) % 2]));
        }
        xs.extend(shown.iter().map(|&(x, _)| x));
        painter.add(shape.mesh(xs, &[(rect.top(), rect.bottom())], |x, _, _| color_at(x)));
    }
}
