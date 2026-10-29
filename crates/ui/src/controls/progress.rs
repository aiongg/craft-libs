//! [`Progress`]: a horizontal progress bar.

use egui::accesskit::Role;
use egui::emath::GuiRounding as _;
use egui::epaint::RectShape;
use egui::{Color32, CornerRadius, Rect, Response, Sense, Shape, Ui, Widget, WidgetInfo, pos2, vec2};

use crate::direction;
use crate::primitives::{Surface, SurfaceEdge, SurfaceKind, SurfaceStyle};
use crate::story::{Args, Level, Story, variant_cluster, variant_list};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens, corner_radius};

/// Seconds for the indeterminate segment to cross the track (PhotoCraft's `jobs_ui::bar`, 0.8 turns
/// a second).
const SWEEP: f64 = 1.25;
/// The indeterminate segment's length, as a fraction of the track.
const SEGMENT: f32 = 0.3;
/// Where the still indeterminate segment starts, as a fraction of the track.
const STILL_START: f32 = 0.3;
/// Dash and gap lengths of the still segment, in track heights.
const DASH: f32 = 2.0;
const DASH_GAP: f32 = 1.0;

/// A progress bar's size and colours, from [`Progress::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ProgressStyle {
    /// Default length: `progress.width`.
    pub width: f32,
    /// Thickness: `progress.track_height`.
    pub height: f32,
    /// `progress.radius`, on the track and the fill.
    pub radius: CornerRadius,
    /// The sunken surface with `progress.track` and `progress.radius`.
    pub track: SurfaceStyle,
    /// `progress.fill`.
    pub fill: Color32,
}

/// What a bar shows.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Amount {
    /// A fraction of the track; one that is not a number counts as none.
    Fraction(f32),
    Indeterminate,
}

/// A horizontal progress bar: `ui.add(Progress::new(0.4).label("Export"))`.
///
/// The fill grows from the reading start, so it mirrors in a right-to-left interface, and is never
/// narrower than the bar is thick unless it is empty. The bar is `progress.width` long unless
/// [`width`](Progress::width) says otherwise. [`Progress::indeterminate`]
/// moves a segment along the track; with reduced motion the segment stands still and is drawn as
/// dashes, so it cannot be mistaken for a determinate fill. AccessKit sees a progress indicator
/// with the fraction as its value between 0 and 1 (none when indeterminate).
#[derive(Clone, Debug)]
pub struct Progress {
    amount: Amount,
    label: Option<String>,
    width: Option<f32>,
}

impl Progress {
    /// A bar `fraction` full, clamped to 0..=1.
    pub fn new(fraction: f32) -> Self {
        Self { amount: Amount::Fraction(fraction), label: None, width: None }
    }

    pub fn indeterminate() -> Self {
        Self { amount: Amount::Indeterminate, label: None, width: None }
    }

    /// The accessible name.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// The bar's length instead of 120 points.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    pub fn style(tokens: &UiTokens) -> ProgressStyle {
        let p = &tokens.component.progress;
        let radius = corner_radius(p.radius);
        let mut track = Surface::style(tokens, SurfaceKind::Sunken);
        track.fill = p.track.to_egui();
        track.radius = radius;
        if let SurfaceEdge::Border(stroke) = &mut track.edge {
            stroke.color = p.edge.to_egui();
        }
        ProgressStyle { width: p.width, height: p.track_height, radius, track, fill: p.fill.to_egui() }
    }

    /// `fraction` as drawn and reported: clamped to 0..=1, and 0 when it is not a number.
    pub fn clamped(fraction: f32) -> f32 {
        if fraction.is_nan() { 0.0 } else { fraction.clamp(0.0, 1.0) }
    }

    /// The shapes of a bar over `rect`: the track, then the fill. `spans` are the filled parts as
    /// fractions of the track from the reading start, in reading order.
    pub fn shapes(style: &ProgressStyle, rect: Rect, spans: &[(f32, f32)], rtl: bool, pixels_per_point: f32) -> Vec<Shape> {
        let mut out = style.track.shapes(rect, pixels_per_point);
        let rect = rect.round_to_pixels(pixels_per_point);
        for &(from, to) in spans {
            let (a, b) = if rtl {
                (rect.right() - to * rect.width(), rect.right() - from * rect.width())
            } else {
                (rect.left() + from * rect.width(), rect.left() + to * rect.width())
            };
            let fill = Rect::from_min_max(pos2(a, rect.top()), pos2(b, rect.bottom())).round_to_pixels(pixels_per_point);
            if fill.width() > 0.0 {
                out.push(Shape::Rect(RectShape::filled(fill, style.radius, style.fill)));
            }
        }
        out
    }
}

impl Widget for Progress {
    fn ui(self, ui: &mut Ui) -> Response {
        let tokens = Tokens::of(ui);
        let style = Progress::style(&tokens);
        let width = self.width.unwrap_or(style.width);
        let (rect, response) = ui.allocate_exact_size(vec2(width, style.height), Sense::hover());
        let value = match self.amount {
            Amount::Fraction(f) => Some(f64::from(Progress::clamped(f))),
            Amount::Indeterminate => None,
        };
        response.widget_info(|| {
            let mut info = match &self.label {
                Some(label) => WidgetInfo::labeled(Role::ProgressIndicator, ui.is_enabled(), label),
                None => WidgetInfo::new(Role::ProgressIndicator),
            };
            info.value = value;
            info
        });
        ui.ctx().accesskit_node_builder(response.id, |node| {
            if value.is_some() {
                node.set_min_numeric_value(0.0);
                node.set_max_numeric_value(1.0);
            } else {
                node.set_busy();
            }
        });
        if ui.is_rect_visible(rect) {
            let still = tokens.appearance.reduced_motion;
            let spans = match self.amount {
                Amount::Fraction(f) => match Progress::clamped(f) {
                    f if f > 0.0 => vec![(0.0, f.max(style.height / rect.width().max(1.0)).min(1.0))],
                    _ => Vec::new(),
                },
                Amount::Indeterminate if still => dashes(rect.width(), style.height),
                Amount::Indeterminate => {
                    let phase = (ui.input(|i| i.time) % SWEEP / SWEEP) as f32;
                    let start = -SEGMENT + phase * (1.0 + SEGMENT);
                    vec![(start.max(0.0), (start + SEGMENT).min(1.0))]
                }
            };
            ui.painter().extend(Progress::shapes(&style, rect, &spans, direction::is_rtl(ui), ui.pixels_per_point()));
            if matches!(self.amount, Amount::Indeterminate) && !still {
                ui.ctx().request_repaint();
            }
        }
        response
    }
}

/// The still indeterminate segment: dashes across `STILL_START..STILL_START + SEGMENT` of a track
/// `width` wide.
fn dashes(width: f32, height: f32) -> Vec<(f32, f32)> {
    let (dash, gap) = (DASH * height / width.max(1.0), DASH_GAP * height / width.max(1.0));
    let end = STILL_START + SEGMENT;
    let mut out = Vec::new();
    let mut at = STILL_START;
    while at < end {
        out.push((at, (at + dash).min(end)));
        at += dash + gap;
    }
    out
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Progress"];

pub(crate) fn stories() -> Vec<Story> {
    vec![
        Story::new(
            "controls/progress/values",
            Level::Controls,
            "Progress",
            "Values",
            || Args::new().with_number("fraction", 0.4, 0.0, 1.0),
            show_values,
        ),
        Story::new("controls/progress/indeterminate", Level::Controls, "Progress", "Indeterminate", Args::new, show_indeterminate),
    ]
}

/// Empty, the args' fraction, and full.
fn show_values(ui: &mut Ui, args: &mut Args) {
    let chosen = args.number("fraction") as f32;
    variant_list(ui, "progress-values", |form| {
        for (name, fraction) in [("Empty", 0.0), ("Chosen", chosen), ("Full", 1.0)] {
            variant_cluster(form, &format!("{:.0}%", fraction * 100.0), |ui| {
                ui.add(Progress::new(fraction).label(name));
            });
        }
    });
}

fn show_indeterminate(ui: &mut Ui, _: &mut Args) {
    variant_list(ui, "progress-indeterminate", |form| {
        variant_cluster(form, "Working", |ui| {
            ui.add(Progress::indeterminate().label("Working"));
        });
    });
}
