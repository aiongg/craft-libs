//! [`VectorscopePlot`]: graticule, targets and the hover target ([`scope_point`]).

use craft_theme::{Mode, TypeRole};
use craft_ui::localized::{Labels as _, LocaleNumbers};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _, tokens_in_mode};
use egui::{Align2, AsIdSalt, Color32, IdSalt, Mesh, Pos2, Rect, Response, Role, Sense, Shape, Stroke, Ui, WidgetInfo, vec2};

use super::labels::ScopePanelLabels;
use super::panel::ScopePanel;

/// The vectorscope's counts for one pass: a hue-major grid of `hues × saturations` cells
/// ([`Vectorscope::counts`](crate::scopes::Vectorscope::counts)) and the sample under the pointer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VectorscopeFrame<'a> {
    /// `hues × saturations` counts, hue-major.
    pub counts: &'a [u32],
    /// Hue cells ([`HUES`](crate::scopes::HUES)).
    pub hues: usize,
    /// Saturation cells ([`SATURATIONS`](crate::scopes::SATURATIONS)).
    pub saturations: usize,
    /// The hue (turns) and saturation (0..=1) of the pixel under the pointer: the hover target.
    pub hover: Option<[f32; 2]>,
}

impl VectorscopeFrame<'_> {
    /// The AccessKit value of the plot, [`ScopePanelLabels::vectorscope_value`] filled from the
    /// counts, the numbers in `numbers`'s digits ([`Vectorscope::summary`](crate::scopes::Vectorscope::summary)'s rules); `None`
    /// when nothing was counted.
    pub fn summary(&self, labels: &ScopePanelLabels, numbers: &LocaleNumbers) -> Option<String> {
        // new: spec AccessKit "Vectorscope".
        if self.saturations < 2 || self.hues == 0 {
            return None;
        }
        let mut peak_saturation: Option<f32> = None;
        let (mut sin, mut cos) = (0.0f64, 0.0f64);
        for (index, count) in self.counts.iter().enumerate().filter(|(_, count)| **count > 0) {
            let hue = (index / self.saturations) as f32 / self.hues as f32;
            let saturation = (index % self.saturations) as f32 / (self.saturations - 1) as f32;
            peak_saturation = Some(peak_saturation.map_or(saturation, |peak| peak.max(saturation)));
            let weight = f64::from(*count) * f64::from(saturation);
            let angle = f64::from(hue) * std::f64::consts::TAU;
            sin += weight * angle.sin();
            cos += weight * angle.cos();
        }
        let peak_saturation = peak_saturation?;
        let mean_hue = if sin == 0.0 && cos == 0.0 { 0.0 } else { sin.atan2(cos).to_degrees().rem_euclid(360.0) };
        let saturation = craft_i18n::ltr(&numbers.format(f64::from(peak_saturation) * 100.0, 0..=0));
        let hue = craft_i18n::ltr(&numbers.format(mean_hue, 0..=0));
        Some(craft_i18n::fmt(&labels.vectorscope_value, &[("saturation", &saturation), ("hue", &hue)]))
    }
}

/// The radius of the rings' outer edge in a vectorscope drawn in `rect`: half the shorter side
/// less `margin` (the vector margin), at least 1.
fn ring_radius(rect: Rect, margin: f32) -> f32 {
    (rect.width().min(rect.height()) * 0.5 - margin).max(1.0)
}

/// Where hue `hue` (turns) at `saturation` (0..=1) sits in a vectorscope drawn in `rect`: the
/// radius is half the shorter side less `margin` (the vector margin), red at the 3 o'clock position when `red_right`, else
/// at −100°, hue turning counter-clockwise. The cloud, the targets and the hover target use it.
pub fn scope_point(rect: Rect, margin: f32, red_right: bool, hue: f32, saturation: f32) -> Pos2 {
    // pc: crates/ui-egui/src/dialogs/camera_raw_scope_ui.rs:443 `scope_point`.
    let radius = ring_radius(rect, margin);
    let offset = if red_right { 0.0 } else { -100.0f32.to_radians() };
    let angle = offset - hue * std::f32::consts::TAU;
    rect.center() + vec2(angle.cos(), angle.sin()) * saturation * radius
}

/// The colour of hue `hue` (turns) at `saturation`, at the brightness of the dark data colours
/// (`level`, 0..=1): PhotoCraft's `scope_hue`.
fn scope_hue(hue: f32, saturation: f32, level: f32) -> Color32 {
    // pc: crates/ui-egui/src/theme.rs:806 `scope_hue`.
    let channel = |offset: f32| {
        let k = (offset + hue * 6.0).rem_euclid(6.0);
        (level * (1.0 - saturation * k.min(4.0 - k).clamp(0.0, 1.0)) * 255.0) as u8
    };
    Color32::from_rgb(channel(5.0), channel(3.0), channel(1.0))
}

/// The vectorscope plot, usable alone (EffectCraft, LightCraft) or under [`ScopePanel`](super::ScopePanel).
///
/// **What the user sees.** On `data.scope`, rounded `shape.radius.control`: rings at 0.25, 0.5,
/// 0.75 and 1 of the radius in `text.faint` × the vector ring alpha, 0.7 pt; spokes to R, Y,
/// G, C, B, M in `text.faint` × 0.4, 0.6 pt, each ending in a 2 pt dot and its letter in
/// `scope_hue(h, 1)` (`data.*`); the skin-tone line at 23° in `text.muted` (unless off); the
/// rings, spokes and skin-tone line read the dark mode's `text.faint` and `text.muted`, as the
/// plot is dark in every theme; the cloud
/// as square dots of half-side `radius / (saturations − 1)` clamped to `scopes.vector_dot_min..=
/// scopes.vector_dot_max`, coloured `scope_hue(h, s)` at alpha `0.2 + 0.8 × sqrt(count / peak)`;
/// the hover target as a 5 pt circle and a 9 pt cross, a 3 pt `data.scope` stroke under a 1.2 pt
/// white one. It never mirrors (`Unmirrored::Graph`).
///
/// **AccessKit.** An `Image` named [`ScopePanelLabels::vectorscope`] with the value text of
/// [`VectorscopeFrame::summary`]. A right-click on it opens the host's scope menu (the panel's).
#[must_use = "show the plot with `show`"]
pub struct VectorscopePlot<'a> {
    pub(super) id_salt: IdSalt,
    pub(super) frame: &'a VectorscopeFrame<'a>,
    pub(super) red_right: bool,
    pub(super) skin_tone: bool,
    pub(super) height: Option<f32>,
    pub(super) labels: Option<ScopePanelLabels>,
}

impl<'a> VectorscopePlot<'a> {
    /// A plot of `frame`; red at −100° and the skin-tone line on by default.
    pub fn new(id_salt: impl AsIdSalt, frame: &'a VectorscopeFrame<'a>) -> Self {
        Self { id_salt: IdSalt::new(id_salt), frame, red_right: false, skin_tone: true, height: None, labels: None }
    }

    /// Show Red at 3 o'clock.
    pub fn red_right(mut self, red_right: bool) -> Self {
        self.red_right = red_right;
        self
    }

    /// Show Skin Tone Indicator.
    pub fn skin_tone(mut self, skin_tone: bool) -> Self {
        self.skin_tone = skin_tone;
        self
    }

    /// The plot's height; without a call 30 % of the available height clamped to
    /// `scopes.vector_height_min..=scopes.vector_height_max` (PhotoCraft's Camera Raw header).
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// The words; the installed words' [`ScopePanelLabels`] without a call.
    pub fn labels(mut self, labels: ScopePanelLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Draws the plot across the available width.
    pub fn show(self, ui: &mut Ui) -> Response {
        // pc: crates/ui-egui/src/dialogs/camera_raw_scope_ui.rs:450-496 `vectorscope`, :511 (height rule).
        let tokens = Tokens::of(ui);
        // The plot is dark in every theme, so it draws the dark mode's data colours.
        let dark = tokens_in_mode(ui.ctx(), Mode::Dark);
        let style = ScopePanel::style(&tokens);
        let labels = self.labels.unwrap_or_else(|| ScopePanelLabels::of(ui));
        let frame = self.frame;
        let height = self.height.unwrap_or_else(|| (ui.available_height() * 0.3).clamp(style.vector_height[0], style.vector_height[1]));
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
        let response = ui.interact(rect, ui.make_persistent_id(self.id_salt), Sense::click());
        let value = frame.summary(&labels, &LocaleNumbers::of(ui));
        response.widget_info(|| {
            let mut info = WidgetInfo::labeled(Role::Image, ui.is_enabled(), &labels.vectorscope);
            info.current_text_value.clone_from(&value);
            info
        });
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let red_right = self.red_right;
        let point = |hue: f32, saturation: f32| scope_point(rect, style.vector_margin, red_right, hue, saturation);
        let center = rect.center();
        let radius = ring_radius(rect, style.vector_margin);
        let scope = dark.color.data.scope.to_egui();
        let level = f32::from(dark.color.data.red.to_egui().r()) / 255.0;
        let (faint, muted) = (dark.color.text.faint.to_egui(), dark.color.text.muted.to_egui());
        let painter = ui.painter();
        painter.rect_filled(rect, tokens.radius.control, scope);
        for ring in [0.25, 0.5, 0.75, 1.0] {
            painter.circle_stroke(center, radius * ring, Stroke::new(0.7, faint.gamma_multiply(style.vector_ring_alpha)));
        }
        let mut mesh = Mesh::default();
        let peak = frame.counts.iter().copied().max().unwrap_or(1).max(1) as f32;
        if frame.saturations > 1 && frame.hues > 0 {
            let dot = (radius / (frame.saturations - 1) as f32).clamp(style.vector_dot[0], style.vector_dot[1]);
            for (index, count) in frame.counts.iter().enumerate().filter(|(_, count)| **count > 0) {
                let hue = (index / frame.saturations) as f32 / frame.hues as f32;
                let saturation = (index % frame.saturations) as f32 / (frame.saturations - 1) as f32;
                let color = scope_hue(hue, saturation, level).gamma_multiply(0.2 + 0.8 * (*count as f32 / peak).sqrt());
                mesh.add_colored_rect(Rect::from_center_size(point(hue, saturation), vec2(dot * 2.0, dot * 2.0)), color);
            }
        }
        painter.add(Shape::mesh(mesh));
        let letters = tokens.font(TypeRole::Caption);
        for (index, letter) in ["R", "Y", "G", "C", "B", "M"].into_iter().enumerate() {
            let hue = index as f32 / 6.0;
            let color = scope_hue(hue, 1.0, level);
            painter.line_segment([center, point(hue, 1.0)], Stroke::new(0.6, faint.gamma_multiply(0.4)));
            painter.circle_filled(point(hue, 1.0), 2.0, color);
            painter.text(point(hue, 1.11), Align2::CENTER_CENTER, letter, letters.clone(), color);
        }
        if self.skin_tone {
            painter.line_segment([center, point(23.0 / 360.0, 1.0)], Stroke::new(1.0, muted));
        }
        if let Some([hue, saturation]) = frame.hover {
            let at = point(hue, saturation);
            let painter = painter.with_clip_rect(rect);
            let white = craft_theme::Rgba::WHITE.to_egui();
            for stroke in [Stroke::new(3.0, scope), Stroke::new(1.2, white)] {
                painter.circle_stroke(at, 5.0, stroke);
                painter.line_segment([at - vec2(9.0, 0.0), at + vec2(9.0, 0.0)], stroke);
                painter.line_segment([at - vec2(0.0, 9.0), at + vec2(0.0, 9.0)], stroke);
            }
        }
        response
    }
}
