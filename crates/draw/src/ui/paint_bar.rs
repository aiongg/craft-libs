//! [`PaintBar`]: the options bar members of the Brush, Pencil, Eraser and Mixer Brush tools after the preset picker.

use craft_behavior::{EditEvents, Value};
use craft_theme::Srgba;
use craft_ui::contract::Entry;
use craft_ui::text::RangeAlert;
use egui::{Response, Ui};

mod words;

pub use words::PaintBarWords;

/// The painting tools the bar serves.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PaintTool {
    /// The Brush tool.
    Brush,
    /// The Pencil tool.
    Pencil,
    /// The Eraser tool.
    Eraser,
    /// The Mixer Brush tool.
    MixerBrush,
}

/// The bar's values.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct PaintValues<M> {
    /// The brush size.
    pub size: f64,
    /// The brush hardness.
    pub hardness: f64,
    /// The opacity.
    pub opacity: f64,
    /// The flow.
    pub flow: f64,
    /// The stroke smoothing.
    pub smoothing: Smoothing,
    /// Pressure controls the opacity.
    pub pressure_opacity: bool,
    /// Pressure controls the size.
    pub pressure_size: bool,
    /// Build-up (airbrush) is on.
    pub build_up: bool,
    /// The blend mode, as the app's blend key.
    pub mode: M,
    /// The eraser erases to history.
    pub erase_to_history: bool,
    /// The pencil erases where it starts on the foreground colour.
    pub auto_erase: bool,
    /// The Mixer Brush's values.
    pub mixer: MixerValues,
}

/// The stroke smoothing options.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Smoothing {
    /// The amount, 0 to 1.
    pub amount: f64,
    /// Pulled string mode.
    pub pulled_string: bool,
    /// The stroke catches up when the pointer pauses.
    pub catch_up: bool,
    /// The stroke catches up at the end.
    pub catch_up_on_end: bool,
    /// The amount adjusts for zoom.
    pub adjust_for_zoom: bool,
}

/// The Mixer Brush's values.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct MixerValues {
    /// Load the brush after each stroke.
    pub load_after_stroke: bool,
    /// Clean the brush after each stroke.
    pub clean_after_stroke: bool,
    /// The wetness.
    pub wet: f64,
    /// The load.
    pub load: f64,
    /// The mix.
    pub mix: f64,
    /// The flow.
    pub flow: f64,
    /// Sample all layers.
    pub sample_all_layers: bool,
    /// The index into [`MIXER_COMBINATIONS`].
    pub combination: usize,
    /// The colour the brush is loaded with, if any.
    pub load_color: Option<Srgba>,
    /// Only solid colours load.
    pub solid_colors_only: bool,
}

/// What the Eraser tool erases with.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum EraserMode {
    /// A brush.
    #[default]
    Brush,
    /// A pencil.
    Pencil,
    /// A block.
    Block,
}

/// The mixer combinations as (wet, load, mix); `None` is Custom.
pub const MIXER_COMBINATIONS: [Option<(f64, f64, f64)>; 13] = [
    None,
    Some((0.0, 25.0, 25.0)),
    Some((0.0, 50.0, 50.0)),
    Some((0.0, 75.0, 75.0)),
    Some((25.0, 25.0, 25.0)),
    Some((25.0, 50.0, 50.0)),
    Some((25.0, 75.0, 75.0)),
    Some((50.0, 25.0, 25.0)),
    Some((50.0, 50.0, 50.0)),
    Some((50.0, 75.0, 75.0)),
    Some((75.0, 25.0, 25.0)),
    Some((75.0, 50.0, 50.0)),
    Some((75.0, 75.0, 75.0)),
];

/// The index of the first combination of each group, as fractions 0..1 of the list.
pub const MIXER_GROUP_STARTS: [usize; 4] = [0, 1, 4, 7];

/// The bar's members for one painting tool.
#[must_use = "show the bar with `show`"]
#[derive(craft_ui::contract::Pattern)]
#[pattern(probe(PaintBar<'static, u32>))]
pub struct PaintBar<'a, M: Value> {
    tool: PaintTool,
    values: &'a mut PaintValues<M>,
    eraser_mode: Option<&'a mut EraserMode>,
    max_size: f64,
    modes: &'a [Entry<M>],
}

/// What [`PaintBar::show`] reports.
#[non_exhaustive]
pub struct PaintBarOutput<M> {
    /// The bar's response.
    pub response: Response,
    /// The edits of the values, one gesture each.
    pub edits: EditEvents<PaintValues<M>>,
    /// The Load button was pressed.
    pub mixer_load: bool,
    /// The Clean button was pressed.
    pub mixer_clean: bool,
    /// A value was typed out of range.
    pub out_of_range: Option<RangeAlert>,
}

impl<'a, M: Value> PaintBar<'a, M> {
    /// The bar of `tool` over `values`.
    pub fn new(tool: PaintTool, values: &'a mut PaintValues<M>) -> Self {
        PaintBar { tool, values, eraser_mode: None, max_size: 5000.0, modes: &[] }
    }

    /// The Eraser's mode, shown for the Eraser tool.
    pub fn eraser_mode(mut self, mode: &'a mut EraserMode) -> Self {
        self.eraser_mode = Some(mode);
        self
    }

    /// The largest brush size.
    pub fn max_size(mut self, max: f64) -> Self {
        self.max_size = max;
        self
    }

    /// The blend modes the Mode list offers.
    pub fn modes(mut self, modes: &'a [Entry<M>]) -> Self {
        self.modes = modes;
        self
    }

    /// Shows the members.
    pub fn show(self, ui: &mut Ui) -> PaintBarOutput<M> {
        let PaintBar { tool, values, eraser_mode, max_size, modes } = self;
        let _ = (ui, tool, values, eraser_mode, max_size, modes);
        todo!("pn.C18")
    }
}
