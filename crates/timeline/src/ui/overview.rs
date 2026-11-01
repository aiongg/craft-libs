//! The whole timeline in miniature (N1.13).

use craft_ui::localized::Labels as _;
use craft_ui::primitives::Corner;
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _, scale};
use egui::accesskit::Role;
use egui::{AsIdSalt, Color32, IdSalt, Rect, Response, Sense, Stroke, StrokeKind, Ui, WidgetInfo, pos2, vec2};

use super::style::TimelineStyle;
use crate::time::{Tick, TimeRange, TimeView};

/// The overview shows this long past the content's end (SoundCraft `universe.rs:16`).
const TAIL_SECONDS: f64 = 10.0;
/// The room a run leaves above and below in its row, and the least width of a run
/// (SoundCraft `universe.rs:25`, `:26`). Drawing details of the miniature (decisions §3.4).
const RUN_INSET: f32 = 0.5;
const RUN_MIN_WIDTH: f32 = 1.0;

/// A run of touching clips of one colour on one track, in overview coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OverviewRun {
    pub rect: Rect,
    pub color: Color32,
}

/// The overview's blocks for `tracks` (each a list of clip ranges and colours, in time order)
/// over `total` time in `rect`: one row per track, each clip at least one point wide, and
/// neighbours of one colour that touch or overlap merged into one run, which covers exactly the
/// same pixels in the same order (a drum edit holds tens of thousands of clips under a point
/// wide). The app caches the result per document revision and rect.
// soundcraft: crates/ui-egui/src/edit_window/universe.rs:15 `universe_runs`.
pub fn overview_runs(tracks: &[Vec<(TimeRange, Color32)>], total: Tick, rect: Rect) -> Vec<OverviewRun> {
    let total = total.0.max(1) as f32;
    let row = (rect.height() / tracks.len().max(1) as f32).max(1.0);
    let mut runs = Vec::new();
    for (index, clips) in tracks.iter().enumerate() {
        let y = rect.min.y + index as f32 * row;
        let block = |from: f32, to: f32, color: Color32| OverviewRun {
            rect: Rect::from_min_max(pos2(from, y + RUN_INSET), pos2(to, y + row - RUN_INSET)),
            color,
        };
        let mut run: Option<(f32, f32, Color32)> = None;
        for &(range, color) in clips {
            let from = rect.min.x + range.start.0 as f32 / total * rect.width();
            let to = (rect.min.x + range.end().0 as f32 / total * rect.width()).max(from + RUN_MIN_WIDTH);
            run = match run {
                Some((a, b, held)) if held == color && from >= a && from <= b => Some((a, b.max(to), held)),
                Some((a, b, held)) => {
                    runs.push(block(a, b, held));
                    Some((from, to, color))
                }
                None => Some((from, to, color)),
            };
        }
        if let Some((a, b, held)) = run {
            runs.push(block(a, b, held));
        }
    }
    runs
}

/// Pro Tools' Universe (SoundCraft): the runs, the visible window as a box (1.5-point outline
/// in the `timeline.overview_view` colour), the playhead while playing; a click or drag centres
/// the window at the pointer, changing `view.start` directly. Total time is the content end
/// plus 10 seconds. Not a Tab stop (the panel scrolls by keys); AccessKit: an `Image` named by
/// [`TimelineLabels::overview`](super::TimelineLabels::overview).
pub struct TimelineOverview<'a> {
    id_salt: IdSalt,
    runs: &'a [OverviewRun],
    view: &'a mut TimeView,
    view_width: f32,
    end: Tick,
    playhead: Option<Tick>,
    labels: Option<super::TimelineLabels>,
}

impl<'a> TimelineOverview<'a> {
    /// `view_width`: the timeline's time area in points (for the box's width).
    pub fn new(id_salt: impl AsIdSalt, runs: &'a [OverviewRun], view: &'a mut TimeView, view_width: f32, end: Tick) -> Self {
        Self { id_salt: IdSalt::new(id_salt), runs, view, view_width, end, playhead: None, labels: None }
    }

    pub fn labels(mut self, labels: super::TimelineLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    pub fn playhead(mut self, t: Option<Tick>) -> Self {
        self.playhead = t;
        self
    }

    // soundcraft: crates/ui-egui/src/edit_window/universe.rs:60 `universe`.
    pub fn show(self, ui: &mut Ui) -> Response {
        let Self { id_salt, runs, view, view_width, end, playhead, labels } = self;
        let labels = labels.unwrap_or_else(|| super::TimelineLabels::of(ui));
        let tokens = Tokens::of(ui);
        let style = TimelineStyle::of(&tokens);
        let enabled = ui.is_enabled();
        let outer = ui.available_rect_before_wrap();
        let area = outer.shrink2(vec2(style.overview_inset_x, style.overview_inset_y));
        let id = ui.make_persistent_id(id_salt);
        let response = ui.interact(area, id, Sense::drag());
        ui.advance_cursor_after_rect(outer);
        response.widget_info(|| WidgetInfo::labeled(Role::Image, enabled, labels.overview.as_str()));

        let total = (end + Tick::from_seconds_f64(TAIL_SECONDS)).0.max(1) as f64;
        let to_fraction = |tick: Tick| (tick.0 as f64 / total) as f32;
        let shown_seconds = f64::from(view_width.max(style.overview_view_min)) / view.points_per_second.max(f64::EPSILON);
        let shown = to_fraction(Tick::from_seconds_f64(shown_seconds));
        if enabled
            && response.is_pointer_button_down_on()
            && let Some(pos) = response.interact_pointer_pos()
        {
            let centred = ((pos.x - area.min.x) / area.width().max(f32::EPSILON) - shown / 2.0).clamp(0.0, 1.0);
            view.start = Tick((f64::from(centred) * total).round() as i64);
        }

        let painter = ui.painter_at(outer);
        let color = &tokens.color;
        for run in runs {
            painter.rect_filled(run.rect, egui::CornerRadius::ZERO, run.color);
        }
        let first = to_fraction(view.start);
        let last = first + shown;
        let window = Rect::from_min_max(
            pos2(area.min.x + first * area.width(), area.min.y - style.overview_box_outset),
            pos2(area.min.x + last.min(1.0) * area.width(), area.max.y + style.overview_box_outset),
        );
        painter.rect_stroke(
            window,
            Corner::Control.radius(&tokens),
            Stroke::new(scale::Stroke::W1_5.width(ui.pixels_per_point()), color.overlay.line.to_egui()),
            StrokeKind::Outside,
        );
        if let Some(playhead) = playhead {
            let x = area.min.x + to_fraction(playhead) * area.width();
            painter.line_segment(
                [pos2(x, area.min.y), pos2(x, area.max.y)],
                Stroke::new(scale::Stroke::W1.width(ui.pixels_per_point()), color.accent.solid.to_egui()),
            );
        }
        response
    }
}
