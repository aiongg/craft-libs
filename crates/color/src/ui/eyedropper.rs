//! The Eyedropper tool (F04.07): its options ([`EyedropperOptions`]: Sample Size, Sample, Show
//! Sampling Ring), its options bar ([`EyedropperOptionsBar`]), the sampling ring drawn while the
//! button is held ([`eyedropper_ring`]) and the canvas tool ([`EyedropperTool`]).
//!
//! The Sample Size is shared with the dialog droppers ([`DropperTool`](super::DropperTool)) and
//! the Info panel, as in Photoshop: the app keeps one [`EyedropperOptions`] and passes its `size`
//! to each.

use std::f32::consts::{PI, TAU};

use craft_theme::Srgba;
use egui::{Color32, Context, Painter, Pos2, Shape, Stroke, vec2};

use craft_ui::canvas::{CanvasTool, ToolAction, ToolContext, ToolCursor, ToolPointer};
use craft_ui::controls::{Checkbox, Select};
use craft_ui::icons::Icon;
use craft_ui::localized::SharedLabels;
use craft_ui::patterns::{CellWidth, OptionsBar, OptionsBarSlots};
use craft_ui::theme::{ToEgui, UiTokens};

craft_ui::words! {
    /// The Eyedropper's options words, English by default.
    #[non_exhaustive]
    pub struct EyedropperLabels {
        sample_size: "eyedropper.sampleSize" = "Sample Size:";
        size: "eyedropper.size" = "Size";
        point_sample: "eyedropper.pointSample" = "Point Sample";
        /// An N by N average; `{n}` is the side ("3 by 3 Average").
        average: "eyedropper.average" = "{n} by {n} Average";
        sample: "eyedropper.sample" = "Sample:";
        source: "eyedropper.source" = "Source";
        current_layer: "eyedropper.currentLayer" = "Current Layer";
        current_and_below: "eyedropper.currentAndBelow" = "Current & Below";
        all_layers: "eyedropper.allLayers" = "All Layers";
        all_layers_no_adjustments: "eyedropper.allLayersNoAdjustments" = "All Layers No Adjustments";
        current_and_below_no_adjustments: "eyedropper.currentAndBelowNoAdjustments" = "Current & Below No Adjustments";
        show_ring: "eyedropper.showRing" = "Show Sampling Ring";
        /// The bar's hint; `{key}` is the platform's name of Alt ("Option" on macOS).
        hint: "eyedropper.hint" = "Click to sample the foreground colour  ·  {key}-click for background";
    }
}

/// The Sample Size: one pixel or the average of a square round it. Photoshop's list and order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SampleSize {
    #[default]
    Point,
    Avg3,
    Avg5,
    Avg11,
    Avg31,
    Avg51,
    Avg101,
}

impl SampleSize {
    /// Every size, in the menu's order.
    pub const ALL: [SampleSize; 7] =
        [SampleSize::Point, SampleSize::Avg3, SampleSize::Avg5, SampleSize::Avg11, SampleSize::Avg31, SampleSize::Avg51, SampleSize::Avg101];

    /// The square's side in image pixels: 1, 3, 5, 11, 31, 51 or 101.
    pub fn pixels(self) -> u32 {
        match self {
            SampleSize::Point => 1,
            SampleSize::Avg3 => 3,
            SampleSize::Avg5 => 5,
            SampleSize::Avg11 => 11,
            SampleSize::Avg31 => 31,
            SampleSize::Avg51 => 51,
            SampleSize::Avg101 => 101,
        }
    }

    /// The menu item: "Point Sample", else [`EyedropperLabels::average`] with the side.
    // pc: crates/ui-egui/src/tools/eyedropper_ui.rs:13 `size_options`.
    pub fn label(self, labels: &EyedropperLabels) -> String {
        match self {
            SampleSize::Point => labels.point_sample.clone(),
            average => craft_i18n::fmt(&labels.average, &[("n", &average.pixels().to_string())]),
        }
    }
}

/// Which layers the Eyedropper samples. Photoshop's list and order; the document engine reads
/// the [`id`](Self::id).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SampleSource {
    Current,
    CurrentAndBelow,
    #[default]
    All,
    AllNoAdjustments,
    CurrentAndBelowNoAdjustments,
}

impl SampleSource {
    /// Every source, in the menu's order.
    pub const ALL: [SampleSource; 5] = [
        SampleSource::Current,
        SampleSource::CurrentAndBelow,
        SampleSource::All,
        SampleSource::AllNoAdjustments,
        SampleSource::CurrentAndBelowNoAdjustments,
    ];

    /// PhotoCraft's `sampleLayer` id: `current`, `currentAndBelow`, `all`, `allNoAdjustments`,
    /// `currentAndBelowNoAdjustments`.
    pub fn id(self) -> &'static str {
        match self {
            SampleSource::Current => "current",
            SampleSource::CurrentAndBelow => "currentAndBelow",
            SampleSource::All => "all",
            SampleSource::AllNoAdjustments => "allNoAdjustments",
            SampleSource::CurrentAndBelowNoAdjustments => "currentAndBelowNoAdjustments",
        }
    }

    /// The menu item.
    // pc: crates/ui-egui/src/tools/eyedropper_ui.rs:26 `layer_options`.
    pub fn label(self, labels: &EyedropperLabels) -> String {
        match self {
            SampleSource::Current => &labels.current_layer,
            SampleSource::CurrentAndBelow => &labels.current_and_below,
            SampleSource::All => &labels.all_layers,
            SampleSource::AllNoAdjustments => &labels.all_layers_no_adjustments,
            SampleSource::CurrentAndBelowNoAdjustments => &labels.current_and_below_no_adjustments,
        }
        .clone()
    }
}

/// The Eyedropper's options, kept by the app (a settings section or its tool state).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct EyedropperOptions {
    pub size: SampleSize,
    pub source: SampleSource,
    /// Show Sampling Ring: the ring while the button is held. Off with the Precise cursor
    /// preference whatever this says.
    pub show_ring: bool,
}

impl Default for EyedropperOptions {
    fn default() -> Self {
        EyedropperOptions { size: SampleSize::Point, source: SampleSource::All, show_ring: true }
    }
}

/// The Eyedropper's options bar body on [`OptionsBar`](craft_ui::patterns::OptionsBar): "Sample
/// Size:" and its [`Select`](craft_ui::controls::Select) (130 points wide in PhotoCraft),
/// "Sample:" and its Select (190 points), the Show Sampling Ring check box, a divider, then the
/// hint in faint text. Edits write `options` directly.
pub struct EyedropperOptionsBar<'a> {
    options: &'a mut EyedropperOptions,
}

impl<'a> EyedropperOptionsBar<'a> {
    pub fn new(options: &'a mut EyedropperOptions) -> EyedropperOptionsBar<'a> {
        EyedropperOptionsBar { options }
    }

    /// Fills the bar's body slot.
    // pc: crates/ui-egui/src/tools/eyedropper_ui.rs:42 `options` (the widths are the field sizes
    // of `OptionsBarStyle::fields`, the nearest of Md and Lg).
    pub fn add_to(self, slots: &mut OptionsBarSlots<'a>) {
        let options = self.options;
        slots.body(move |ui| {
            let labels = EyedropperLabels::shared(ui);
            OptionsBar::cell(ui, &labels.sample_size, CellWidth::Md, |ui, _| {
                let mut select = Select::new("eyedropper-size", labels.size.clone(), &mut options.size);
                for size in SampleSize::ALL {
                    select = select.option(size, size.label(&labels));
                }
                let _ = select.show(ui);
            });
            OptionsBar::cell(ui, &labels.sample, CellWidth::Lg, |ui, _| {
                let mut select = Select::new("eyedropper-source", labels.source.clone(), &mut options.source);
                for source in SampleSource::ALL {
                    select = select.option(source, source.label(&labels));
                }
                let _ = select.show(ui);
            });
            let _ = Checkbox::bool(labels.show_ring.clone(), &mut options.show_ring).show(ui);
            OptionsBar::divider(ui);
            let key = if ui.ctx().os() == egui::os::OperatingSystem::Mac { "Option" } else { "Alt" };
            OptionsBar::hint(ui, &craft_i18n::fmt(&labels.hint, &[("key", key)]));
        });
    }
}

/// The sampling ring's geometry and colours, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SamplingRingStyle {
    /// The ring's outer radius: 105 (Photoshop's ring is about 210 points across).
    pub outer_radius: f32,
    /// Each band's width: 17 (the grey outer band and the coloured inner band).
    pub band: f32,
    /// The outer band: grey 127, opaque.
    pub grey: Color32,
    /// The hairlines on both edges, one physical pixel: black at alpha 160 outside, white at alpha
    /// 160 inside it.
    pub dark: Color32,
    pub light: Color32,
}

impl SamplingRingStyle {
    /// The ring from the theme's `eyedropper` tokens; the hairlines are fixed.
    pub fn of(tokens: &UiTokens) -> SamplingRingStyle {
        let ring = &tokens.component.eyedropper;
        SamplingRingStyle { outer_radius: ring.ring_radius, band: ring.ring_band, grey: ring.ring_grey.to_egui(), ..SamplingRingStyle::default() }
    }
}

impl Default for SamplingRingStyle {
    fn default() -> Self {
        SamplingRingStyle {
            outer_radius: 105.0,
            band: 17.0,
            grey: Color32::from_gray(127),
            dark: Color32::from_black_alpha(160),
            light: Color32::from_white_alpha(160),
        }
    }
}

/// Draws the sampling ring centred on `center`: the inner band's upper half (screen angles π to
/// 2π) is `new`, the sampled colour, its lower half `current`, the foreground; the outer band is
/// grey, with dark and light hairlines on both edges so it reads on any image. Draw it only while
/// the button is held and [`EyedropperOptions::show_ring`] is on; the pipette cursor stays at the
/// centre.
// pc: crates/ui-egui/src/canvas/cursors.rs:73 `eyedropper_ring` (r_out 105, band 17, r_in = r_out
// - 2 band; the upper half as a 48-segment `Shape::line`, the lower a `circle_stroke`).
pub fn eyedropper_ring(painter: &Painter, center: Pos2, new: Color32, current: Color32, style: &SamplingRingStyle) {
    const SEGMENTS: usize = 48;
    let band = style.band;
    let inner = style.outer_radius - 2.0 * band;
    // A stroke of width `w` centred on radius `r` covers `r - w / 2 ..= r + w / 2`.
    let arc = |radius: f32, from: f32, to: f32| -> Vec<Pos2> {
        (0..=SEGMENTS)
            .map(|i| {
                let a = from + (to - from) * i as f32 / SEGMENTS as f32;
                center + vec2(a.cos(), a.sin()) * radius
            })
            .collect()
    };
    // Screen y grows down, so PI..TAU is the upper half and 0..PI the lower; the halves meet at
    // 3 and 9 o'clock, as in Photoshop.
    painter.add(Shape::line(arc(inner + band / 2.0, PI, TAU), Stroke::new(band, new)));
    painter.add(Shape::line(arc(inner + band / 2.0, 0.0, PI), Stroke::new(band, current)));
    painter.add(Shape::closed_line(arc(inner + band * 1.5, 0.0, TAU)[..SEGMENTS].to_vec(), Stroke::new(band, style.grey)));
    let px = 1.0 / painter.ctx().pixels_per_point().max(1.0);
    for (radius, color) in [
        (style.outer_radius + px / 2.0, style.dark),
        (style.outer_radius + px * 1.5, style.light),
        (inner - px / 2.0, style.dark),
        (inner - px * 1.5, style.light),
    ] {
        painter.add(Shape::closed_line(arc(radius, 0.0, TAU)[..SEGMENTS].to_vec(), Stroke::new(px, color)));
    }
}

/// What an [`EyedropperTool`] picked.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EyedropperPick {
    pub color: Srgba,
    /// Alt was held: the colour is for the background.
    pub background: bool,
}

/// The Eyedropper on the canvas: a press samples the app's `sample` function at the document
/// point with the options' size and source, a drag samples at every move (the ring follows while
/// [`show_ring`](EyedropperOptions::show_ring) is on), the release keeps the last sample; Alt
/// sets the background. Off the image or over transparency nothing changes. The app reads
/// [`take`](Self::take) after the tool host's pass.
///
/// The ring's lower half shows the current foreground colour: set it with
/// [`foreground`](Self::foreground) (black until then).
pub struct EyedropperTool<'a> {
    options: EyedropperOptions,
    sample: Box<dyn FnMut([f64; 2], SampleSize, SampleSource) -> Option<Srgba> + 'a>,
    held: Option<(Pos2, Srgba)>,
    picked: Option<EyedropperPick>,
    foreground: Srgba,
}

impl<'a> EyedropperTool<'a> {
    pub fn new(options: EyedropperOptions, sample: impl FnMut([f64; 2], SampleSize, SampleSource) -> Option<Srgba> + 'a) -> EyedropperTool<'a> {
        EyedropperTool { options, sample: Box::new(sample), held: None, picked: None, foreground: Srgba::rgb(0.0, 0.0, 0.0) }
    }

    /// The foreground colour the ring shows beside the sampled one.
    pub fn foreground(mut self, color: Srgba) -> EyedropperTool<'a> {
        self.foreground = color;
        self
    }

    /// Samples at document point `doc` (screen point `screen` for the ring): what the tool's press
    /// and drag do.
    // pc: crates/ui-egui/src/canvas/cursors.rs:199 `eyedropper_color` (size and source from the
    // options), :60 `eyedropper_ring_colors`.
    pub fn sample_at(&mut self, ctx: &Context, doc: [f64; 2], screen: Pos2, background: bool) {
        match (self.sample)(doc, self.options.size, self.options.source) {
            Some(color) => {
                self.held = Some((screen, color));
                self.picked = Some(EyedropperPick { color, background });
            }
            // Off the image or over transparency: no ring, and the last pick stands.
            None => self.held = None,
        }
        ctx.request_repaint();
    }

    #[cfg(test)]
    pub(super) fn ring_shown(&self) -> bool {
        self.held.is_some()
    }

    /// The pick of the gesture, once.
    pub fn take(&mut self) -> Option<EyedropperPick> {
        self.picked.take()
    }
}

impl CanvasTool for EyedropperTool<'_> {
    fn cursor(&self, _cx: &ToolContext<'_>) -> ToolCursor {
        // The pipette's tip is at the bottom left of its 24-unit icon.
        ToolCursor::Tool { icon: Icon::Eyedropper, hotspot: vec2(2.0, 22.0), badge: None }
    }

    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        self.picked = None;
        self.sample_at(&cx.canvas.ctx, [pointer.doc.x, pointer.doc.y], pointer.screen, pointer.modifiers.alt);
        ToolAction::Claimed
    }

    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        self.sample_at(&cx.canvas.ctx, [pointer.doc.x, pointer.doc.y], pointer.screen, pointer.modifiers.alt);
    }

    fn release(&mut self, _pointer: &ToolPointer, _cx: &mut ToolContext<'_>) {
        self.held = None;
    }

    fn cancel(&mut self, _cx: &mut ToolContext<'_>) {
        self.held = None;
        self.picked = None;
    }

    fn paint(&self, painter: &Painter, cx: &ToolContext<'_>) {
        if let (true, Some((screen, new))) = (self.options.show_ring, self.held) {
            eyedropper_ring(painter, screen, Color32::from(new), Color32::from(self.foreground), &SamplingRingStyle::of(cx.tokens));
        }
    }

    fn autoscrolls(&self) -> bool {
        false
    }
}
