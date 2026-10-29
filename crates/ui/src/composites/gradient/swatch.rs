//! [`GradientSwatch`]: a well that shows a gradient.

use craft_theme::{ControlSize, Rgba, Srgba};
use egui::accesskit::{HasPopup, Role};
use egui::{CursorIcon, Response, Ui, vec2};

use super::ramp::GradientRamp;
use super::stops::{Sampler, default_sampler};
use super::value::{ColorStop, GradientStops, OpacityStop, StopRows};
use crate::controls::{SIZE_NAMES, Swatch, size_arg};
use crate::primitives::{Content, FocusRing, Gap, Interactive, Row, Stack, Text};
use crate::story::{Args, Level, Story};
use crate::theme::{Tokens, TokensOf};
use egui::{AsIdSalt, IdSalt};

/// A well that shows a gradient and reports a click, as [`Swatch`] does for a colour:
/// `if GradientSwatch::new("fill", "Gradient picker", &stops).show(ui).clicked() { open_editor }`.
///
/// The ramp is drawn over the checkerboard where it is translucent, inside the swatch's two
/// rings and corner radius, `swatch.size` high. The swatch edits nothing; a click is the host's
/// cue. AccessKit sees a button named by `label`, with `has_popup` when the host opens one.
#[derive(Clone)]
pub struct GradientSwatch<'a> {
    id_salt: IdSalt,
    label: String,
    stops: &'a GradientStops,
    rows: StopRows,
    sampler: Option<&'a Sampler>,
    width: Option<f32>,
    size: ControlSize,
    enabled: bool,
    popup: bool,
}

impl<'a> GradientSwatch<'a> {
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, stops: &'a GradientStops) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            label: label.into(),
            stops,
            rows: StopRows::Split,
            sampler: None,
            width: None,
            size: ControlSize::Md,
            enabled: true,
            popup: false,
        }
    }

    /// The row layout, for the default sampler. Default [`StopRows::Split`].
    pub fn rows(mut self, rows: StopRows) -> Self {
        self.rows = rows;
        self
    }

    /// Samples the gradient instead of the default [`default_sampler`].
    pub fn sampler(mut self, sampler: &'a Sampler) -> Self {
        self.sampler = Some(sampler);
        self
    }

    /// The swatch's width (PhotoCraft's options bar 96). Default: as wide as it is high.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Whether it takes clicks; a disabled swatch keeps its gradient and fades its rings.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Tells AccessKit that a click opens a popup (the host's editor).
    pub fn has_popup(mut self, popup: bool) -> Self {
        self.popup = popup;
        self
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        let tokens = Tokens::of(ui);
        let style = Swatch::style(&tokens, self.size);
        let ramp = GradientRamp::style(&tokens);
        let default = default_sampler(self.rows);
        let sampler: &Sampler = self.sampler.unwrap_or(&default);
        let response = Interactive::salted(self.id_salt, Role::Button, self.label.as_str())
            .state_layer(false)
            .focus_ring(false)
            .enabled(self.enabled)
            .show(ui, vec2(self.width.unwrap_or(style.side), style.side));
        if self.popup {
            ui.ctx().accesskit_node_builder(response.id, |node| node.set_has_popup(HasPopup::Dialog));
        }
        if ui.is_rect_visible(response.rect) {
            let colors = GradientRamp::colors(self.stops, sampler);
            let painter = ui.painter();
            GradientRamp::paint_rounded(painter, response.rect, f32::from(style.radius.nw), &colors, &ramp);
            painter.extend(Swatch::outline(&style, response.rect, false, self.enabled, ui.pixels_per_point()));
        }
        FocusRing::paint_radius(ui, response.rect, style.radius, &response);
        if self.enabled { response.on_hover_cursor(CursorIcon::PointingHand) } else { response }
    }
}

pub(crate) fn stories() -> Vec<Story> {
    vec![Story::new(
        "composites/gradient/swatch",
        Level::Composites,
        "GradientSwatch",
        "Swatch and ramp",
        || Args::new().with_choice("size", SIZE_NAMES, 1).with_number("width", 96.0, 24.0, 300.0).with_bool("enabled", true),
        show_swatch,
    )]
}

fn show_swatch(ui: &mut Ui, args: &mut Args) {
    let size = size_arg(args.choice("size"));
    let width = args.number("width") as f32;
    let enabled = args.bool("enabled");
    let color = |at: f32, [r, g, b]: [u8; 3]| ColorStop::new(at, Srgba::from(Rgba::rgb(r, g, b)));
    let mut centred = GradientStops::new(vec![color(0.0, [0xf5, 0xa5, 0x24]), color(1.0, [0x3e, 0x63, 0xdd])]);
    let mut skewed = centred.clone();
    skewed.color[0].midpoint = 0.15;
    centred.opacity = vec![OpacityStop::new(0.0, 0.0), OpacityStop::new(1.0, 1.0)];
    let combined = GradientStops::new(vec![
        ColorStop::new(0.0, Srgba::new(0.9, 0.28, 0.3, 1.0)),
        ColorStop::new(0.5, Srgba::new(0.9, 0.28, 0.3, 0.0)),
        ColorStop::new(1.0, Srgba::new(0.19, 0.64, 0.42, 1.0)),
    ]);
    let state_id = ui.scope_id().with("gradient-swatch-clicks");
    let mut clicks = ui.data(|d| d.get_temp::<u32>(state_id).unwrap_or_default());
    Stack::new(Gap::Stack).show(ui, |ui| {
        let tokens = Tokens::of(ui);
        let panel = ui.available_width().min(300.0);
        for (name, stops, rows) in [
            ("Fading in", &centred, StopRows::Split),
            ("Midpoint at 15 %", &skewed, StopRows::Split),
            ("Opacity in the colour", &combined, StopRows::Combined),
        ] {
            Row::new(Gap::Inline).show(ui, |ui| {
                if GradientSwatch::new(name, name, stops).rows(rows).width(width).size(size).enabled(enabled).has_popup(true).show(ui).clicked() {
                    clicks += 1;
                }
                ui.add(Text::new(name).content(Content::Muted));
            });
            // The ramp alone, as a stop bar and a thumbnail draw it.
            let (rect, _) = ui.allocate_exact_size(vec2(panel, 24.0), egui::Sense::hover());
            let sampler = default_sampler(rows);
            GradientRamp::paint(ui.painter(), rect, stops, &sampler, &tokens);
        }
        ui.add(Text::new(format!("Clicked {clicks} times")).content(Content::Muted));
    });
    ui.data_mut(|d| d.insert_temp(state_id, clicks));
}
