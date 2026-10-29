//! What [`Checkbox`](super::Checkbox), the radio buttons of [`RadioGroup`](super::RadioGroup) and
//! [`RadioSet`](super::RadioSet), and [`Switch`](super::Switch) share: an indicator at the reading
//! start with the label after it, the indicator drawn on the sunken
//! [`Surface`](crate::primitives::Surface) in the `check.*` colours.

use std::sync::Arc;

use craft_theme::{CheckOffColors, CheckOnColors, Rgba, States, TypeRole};
use egui::emath::GuiRounding as _;
use egui::text::{LayoutJob, TextWrapping};
use egui::{Align, Color32, CornerRadius, Galley, Pos2, Rect, Response, Stroke, StrokeKind, TextWrapMode, Ui, Vec2, pos2, vec2};

use super::TooltipExt as _;
use crate::direction;
use crate::primitives::{StateLayer, SurfaceEdge, SurfaceStyle};
use crate::theme::{ToEgui, UiTokens, role_format};

/// The colours of an indicator and its mark while off or while on, per state: `check.off.*` or
/// `check.on.*`.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct IndicatorColors {
    /// Inside of the box, circle or track.
    pub fill: States<Color32>,
    /// Outline of the box, circle or track, over a flat surface's border; a bevel keeps its
    /// colours.
    pub border: States<Color32>,
    /// Check, dash or dot; `None` while off, which has no mark. A switch draws none.
    pub mark: Option<States<Color32>>,
}

impl From<&CheckOffColors> for IndicatorColors {
    fn from(c: &CheckOffColors) -> Self {
        Self { fill: c.fill.to_egui(), border: c.border.to_egui(), mark: None }
    }
}

impl From<&CheckOnColors> for IndicatorColors {
    fn from(c: &CheckOnColors) -> Self {
        Self { fill: c.fill.to_egui(), border: c.border.to_egui(), mark: Some(c.mark.to_egui()) }
    }
}

/// An indicator's colours for `response` now, animated with the state layer.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Paint {
    pub fill: Color32,
    pub border: Color32,
    pub mark: Option<Color32>,
}

impl Paint {
    pub(crate) fn now(ui: &Ui, colors: &IndicatorColors, response: &Response) -> Self {
        Self {
            fill: StateLayer::color(ui, &colors.fill, response),
            border: StateLayer::color(ui, &colors.border, response),
            mark: colors.mark.as_ref().map(|mark| StateLayer::color(ui, mark, response)),
        }
    }
}

/// The label colours: `normal`, and `text.disabled` when disabled.
pub(crate) fn label_colors(tokens: &UiTokens, normal: Rgba) -> States<Color32> {
    let (normal, disabled) = (normal.to_egui(), tokens.color.text.disabled.to_egui());
    States::new(normal, normal, normal, disabled)
}

/// A check box or radio button in `rect`: the fill, then an outline of `width` drawn inside, in
/// the colours of `paint`. Flat in every theme.
pub(crate) fn box_shape(rect: Rect, radius: CornerRadius, width: f32, paint: Paint) -> egui::Shape {
    egui::epaint::RectShape::new(rect, radius, paint.fill, Stroke::new(width, paint.border), StrokeKind::Inside).into()
}

/// The sunken surface the indicator draws on, with `fill`, `border` (a flat edge's colour) and
/// `radius` in place of the surface's own.
pub(crate) fn indicator_shapes(ui: &Ui, surface: &SurfaceStyle, rect: Rect, radius: CornerRadius, paint: Paint) -> Vec<egui::Shape> {
    let mut style = *surface;
    style.fill = paint.fill;
    style.radius = radius;
    if let SurfaceEdge::Border(stroke) = style.edge {
        style.edge = SurfaceEdge::Border(Stroke::new(stroke.width, paint.border));
    }
    style.shapes(rect, ui.pixels_per_point())
}

/// An indicator of `indicator` size and a label, laid out in the room the `Ui` has.
pub(crate) struct Labelled {
    galley: Arc<Galley>,
    indicator: Vec2,
    gap: f32,
    /// The control height: one line's element height.
    height: f32,
    /// Height of the label's first line.
    line: f32,
    /// The whole element: indicator, gap and label.
    pub size: Vec2,
    truncated: bool,
}

/// What [`Labelled::layout`] needs from a control's style.
pub(crate) struct Layout {
    pub height: f32,
    pub gap: f32,
    pub label: TypeRole,
    pub indicator: Vec2,
}

impl Labelled {
    /// Lays out `label` after the indicator. The label wraps, is cut with an ellipsis or runs on
    /// as the `Ui`'s wrap mode says; at least one line is `height` tall with the indicator and the
    /// first line centred in it.
    pub(crate) fn layout(ui: &Ui, tokens: &UiTokens, layout: &Layout, label: &str) -> Self {
        let Layout { height, gap, label: role, indicator } = *layout;
        // The text colour depends on the state, known only after the interaction: painted later.
        let format = role_format(tokens, role, Color32::PLACEHOLDER);
        let line = tokens.typography.get(role).line_height;
        let room = (ui.available_width() - indicator.x - gap).max(0.0);
        let layout = |wrap: TextWrapping| {
            let mut job = LayoutJob::single_section(label.to_owned(), format.clone());
            job.wrap = wrap;
            job.halign = if direction::is_rtl(ui) { Align::RIGHT } else { Align::LEFT };
            ui.painter().layout_job(job)
        };
        let full = layout(TextWrapping::no_max_width());
        let (galley, truncated) = match ui.wrap_mode() {
            _ if full.size().x <= room => (full, false),
            TextWrapMode::Wrap => (layout(TextWrapping::wrap_at_width(room)), false),
            TextWrapMode::Truncate => (layout(TextWrapping::truncate_at_width(room)), true),
            TextWrapMode::Extend => (full, false),
        };
        let text = galley.size();
        let width = indicator.x + if label.is_empty() { 0.0 } else { gap + text.x };
        let size = vec2(width, height.max(text.y + (height - line).max(0.0)));
        Self { galley, indicator, gap, height, line, size, truncated }
    }

    /// The indicator's rect in the element's `rect`, at the reading start, and where the label's
    /// top-left corner goes. One line is centred in `rect`; with more, the indicator and the first
    /// line are centred in the top `height`.
    pub(crate) fn place(&self, ui: &Ui, rect: Rect) -> (Rect, Pos2) {
        let ppp = ui.pixels_per_point();
        let text = self.galley.size();
        let (center_y, top) = if self.galley.rows.len() > 1 {
            let center = rect.top() + self.height / 2.0;
            (center, center - self.line / 2.0)
        } else {
            (rect.center().y, rect.center().y - text.y / 2.0)
        };
        let (start, forward) = (direction::start_x(ui, rect), direction::forward(ui));
        let indicator = Rect::from_center_size(pos2(start + forward * self.indicator.x / 2.0, center_y), self.indicator).round_to_pixels(ppp);
        let label_start = start + forward * (self.indicator.x + self.gap);
        let left = if direction::is_rtl(ui) { label_start - text.x } else { label_start };
        (indicator, pos2(left, top).round_to_pixels(ppp))
    }

    /// Paints the label with its top-left corner at `pos`.
    pub(crate) fn paint_label(&self, ui: &Ui, pos: Pos2, color: Color32) {
        if self.galley.is_empty() {
            return;
        }
        // A right-aligned galley's rows sit left of its origin.
        ui.painter().galley(pos - self.galley.rect.min.to_vec2(), Arc::clone(&self.galley), color);
    }

    /// `response` with the whole label in a tooltip when it was cut.
    pub(crate) fn tooltip(&self, response: Response, label: &str) -> Response {
        if self.truncated { response.tooltip(label) } else { response }
    }
}
