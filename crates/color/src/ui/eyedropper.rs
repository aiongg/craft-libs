//! The Eyedropper tool (F04.07): its options ([`EyedropperOptions`]: Sample Size, Sample, Show
//! Sampling Ring), its options bar ([`EyedropperOptionsBar`]), the sampling ring drawn while the
//! button is held ([`eyedropper_ring`]) and the canvas tool ([`EyedropperTool`]).
//!
//! The Sample Size is shared with the dialog droppers ([`DropperTool`](super::DropperTool)) and
//! the Info panel, as in Photoshop: the app keeps one [`EyedropperOptions`] and passes its `size`
//! to each.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_theme::Srgba;
use egui::{Color32, Context, Painter, Pos2};

use craft_ui::patterns::OptionsBarSlots;

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
        todo!()
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
        todo!()
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
        todo!()
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
    todo!()
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
// needs: S3-canvas tool_host — `impl CanvasTool for EyedropperTool` (`press` returns
// `ToolAction::Claimed`; `drag`/`release` sample at `ToolPointer::doc` with
// `ToolPointer::modifiers.alt`; `paint` draws `eyedropper_ring` while held; `cursor` is the
// pipette `ToolCursor` with its tip on the pixel).
pub struct EyedropperTool<'a> {
    options: EyedropperOptions,
    sample: Box<dyn FnMut([f64; 2], SampleSize, SampleSource) -> Option<Srgba> + 'a>,
    held: Option<(Pos2, Srgba)>,
    picked: Option<EyedropperPick>,
}

impl<'a> EyedropperTool<'a> {
    pub fn new(options: EyedropperOptions, sample: impl FnMut([f64; 2], SampleSize, SampleSource) -> Option<Srgba> + 'a) -> EyedropperTool<'a> {
        EyedropperTool { options, sample: Box::new(sample), held: None, picked: None }
    }

    /// Samples at document point `doc` (screen point `screen` for the ring): what the tool's press
    /// and drag do.
    // pc: crates/ui-egui/src/canvas/cursors.rs:199 `eyedropper_color` (size and source from the
    // options), :60 `eyedropper_ring_colors`.
    pub fn sample_at(&mut self, ctx: &Context, doc: [f64; 2], screen: Pos2, background: bool) {
        todo!()
    }

    /// The pick of the gesture, once.
    pub fn take(&mut self) -> Option<EyedropperPick> {
        self.picked.take()
    }
}
