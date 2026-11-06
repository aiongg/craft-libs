//! What [`Checkbox`](super::Checkbox), the radio buttons of [`RadioGroup`](super::RadioGroup) and
//! [`RadioSet`](super::RadioSet), and [`Switch`](super::Switch) share: an indicator at the reading
//! start with the label after it, the indicator drawn on the sunken
//! [`Surface`](crate::primitives::Surface) in the check colours.

use std::sync::Arc;

use craft_theme::{
    CheckOffColors, CheckOnColors, CheckSwitchColors, Contrast, ControlSize, Rgba, States, ThumbColors, Tokens, TypeRole, identifiable_outline,
};
use egui::emath::GuiRounding as _;
use egui::text::{LayoutJob, TextWrapping};
use egui::{Align, Color32, CornerRadius, Galley, Pos2, Rect, Response, Stroke, StrokeKind, TextWrapMode, Ui, Vec2, pos2, vec2};

use super::TooltipExt as _;
use crate::direction;
use crate::primitives::{StateLayer, Surface, SurfaceEdge, SurfaceStyle};
use crate::theme::{ToEgui, UiTokens, role_format};

/// Share of the sunken fill an off indicator keeps while pressed.
const OFF_PRESSED_SHARE: f32 = 0.82;
/// How much of the accent fill an on indicator shows while pressed.
const ON_PRESSED_OPACITY: f32 = 0.75;
/// How far a disabled thumb's face moves from `surface.knob` toward `surface.panel`.
const KNOB_DISABLED: f64 = 0.5;

/// How much smaller than the standard icon a standard-size box or circle is.
const BOX_INSET: f32 = 2.0;
/// Stroke width of a check mark in box sides.
const MARK_RATIO: f32 = 0.125;
/// Diameter of a radio dot in box sides.
const DOT_RATIO: f32 = 0.4;
/// Width of a switch track in track heights.
const SWITCH_ASPECT: f32 = 2.0;

/// The sizes check boxes, radio buttons and switches share, at one control size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CheckSizes {
    /// Height of one line.
    pub height: f32,
    /// Side of the box or circle.
    pub box_size: f32,
    /// Between the box or circle and the label.
    pub box_gap: f32,
    /// Outline width, drawn inside.
    pub outline: f32,
    /// Corner radius of a check box.
    pub radius: f32,
    /// Stroke width of a check or dash.
    pub mark: f32,
    /// Diameter of a radio dot.
    pub dot: f32,
    /// Height of a switch track.
    pub track_height: f32,
    /// Width of a switch track.
    pub track_width: f32,
    /// Between a switch track's edge and its thumb.
    pub thumb_inset: f32,
    /// Between a switch track and its label.
    pub gap: f32,
}

impl CheckSizes {
    pub(crate) fn new(tokens: &Tokens, size: ControlSize) -> Self {
        let (control, icon) = match size {
            ControlSize::Sm => (tokens.size.control.sm, tokens.size.icon.sm),
            ControlSize::Md => (tokens.size.control.md, tokens.size.icon.md),
        };
        let box_size = match size {
            ControlSize::Sm => icon,
            ControlSize::Md => icon - BOX_INSET,
        };
        CheckSizes {
            height: control,
            box_size,
            box_gap: tokens.space.gap.stack,
            outline: tokens.stroke.medium,
            radius: tokens.shape.radius.sm,
            mark: box_size * MARK_RATIO,
            dot: box_size * DOT_RATIO,
            track_height: icon,
            track_width: icon * SWITCH_ASPECT,
            thumb_inset: tokens.space.gap.tight,
            gap: tokens.space.gap.inline,
        }
    }
}

/// The colours of check boxes, radio buttons and switches, per state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CheckColors {
    /// An indicator while off.
    pub off: CheckOffColors,
    /// A check box or radio button while on.
    pub on: CheckOnColors,
    /// A switch track while on.
    pub switch: CheckSwitchColors,
    /// A switch thumb.
    pub thumb: ThumbColors,
}

impl CheckColors {
    pub(crate) fn new(tokens: &UiTokens) -> Self {
        let c = &tokens.color;
        let sunken = c.surface.sunken;
        // `text.faint` meets the text minimum on every surface and state fill, so the outline
        // meets the non-text minimum wherever the control sits; where a pinned `text.faint`
        // doesn't, and the fill doesn't either, the outline moves and `text.faint` stays.
        let high = tokens.appearance.contrast == Contrast::High;
        let rest_outline = identifiable_outline(c.text.faint, sunken, &[c.surface.panel, c.surface.raised], high);
        let outline = States::new(rest_outline, c.text.muted, c.text.muted, c.text.disabled);
        let solid = c.accent.solid;
        let switch = States::new(solid, solid, solid, c.text.disabled);
        let clear = Rgba::TRANSPARENT;
        let on_accent = c.text.on_accent;
        let [r, g, b, a] = sunken.0;
        let darker = |v: u8| (f32::from(v) * OFF_PRESSED_SHARE).round() as u8;
        let (face, edge) = (c.surface.knob, c.line.knob_edge);
        let disabled_face = mix(face, c.surface.panel, KNOB_DISABLED);
        let bevel = Surface::is_bevelled(tokens);
        let off_fill = States::new(sunken, sunken, Rgba([darker(r), darker(g), darker(b), a]), c.state.disabled_fill);
        let off = CheckOffColors { fill: off_fill, border: outline };
        CheckColors {
            off,
            on: if bevel {
                // The sunken face stays and the mark does the work of the accent fill.
                let mark = c.line.mark;
                CheckOnColors { fill: off_fill, border: outline, mark: States::new(mark, mark, mark, c.text.disabled) }
            } else {
                CheckOnColors {
                    fill: States::new(solid, solid, solid.faded(ON_PRESSED_OPACITY), c.text.disabled),
                    border: States::new(clear, clear, clear, c.text.disabled),
                    mark: States::new(on_accent, on_accent, on_accent, c.state.disabled_fill),
                }
            },
            switch: CheckSwitchColors { fill: switch, border: switch },
            thumb: ThumbColors { fill: States::new(face, face, face, disabled_face), border: States::new(edge, edge, edge, c.line.divider) },
        }
    }
}

/// Linear interpolation in OKLab, opaque.
fn mix(a: Rgba, b: Rgba, t: f64) -> Rgba {
    let (a, b) = (a.to_oklab(), b.to_oklab());
    let lerp = |x: f64, y: f64| x + (y - x) * t;
    craft_theme::Oklab { l: lerp(a.l, b.l), a: lerp(a.a, b.a), b: lerp(a.b, b.b) }.to_oklch().to_rgba()
}

/// The colours of an indicator and its mark while off or while on, per state.
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
    /// Width of what sits between the indicator and the label, gap after included; 0 for none.
    lead: f32,
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
        Self::layout_after(ui, tokens, layout, label, 0.0)
    }

    /// As [`layout`](Labelled::layout), with `lead` points (its gap to the label included) kept
    /// free between the indicator's gap and the label.
    pub(crate) fn layout_after(ui: &Ui, tokens: &UiTokens, layout: &Layout, label: &str, lead: f32) -> Self {
        let Layout { height, gap, label: role, indicator } = *layout;
        // The text colour depends on the state, known only after the interaction: painted later.
        let format = role_format(tokens, role, Color32::PLACEHOLDER);
        let line = tokens.typography.get(role).line_height;
        let room = (ui.available_width() - indicator.x - gap - lead).max(0.0);
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
        let width = indicator.x + lead + if label.is_empty() { 0.0 } else { gap + text.x };
        let size = vec2(width, height.max(text.y + (height - line).max(0.0)));
        Self { galley, indicator, gap, height, line, size, truncated, lead }
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
        let label_start = start + forward * (self.indicator.x + self.lead + self.gap);
        let left = if direction::is_rtl(ui) { label_start - text.x } else { label_start };
        (indicator, pos2(left, top).round_to_pixels(ppp))
    }

    /// The rect of the lead item (`side` square) in the element's `rect`: after the indicator and
    /// its gap, centred like the indicator.
    pub(crate) fn lead_rect(&self, ui: &Ui, rect: Rect, side: f32) -> Rect {
        let (indicator, _) = self.place(ui, rect);
        let forward = direction::forward(ui);
        let start = if direction::is_rtl(ui) { indicator.left() } else { indicator.right() } + forward * self.gap;
        let far = start + forward * side;
        Rect::from_x_y_ranges(start.min(far)..=start.max(far), indicator.center().y - side / 2.0..=indicator.center().y + side / 2.0)
            .round_to_pixels(ui.pixels_per_point())
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
