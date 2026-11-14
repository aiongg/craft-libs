//! Color Range: the Select menu's dialog that selects by sampled colours, colour families or tonal ranges.

mod dialog;
mod hosted;
mod labels;
mod value;

pub use dialog::{ColorRangeDialog, ColorRangeOutput};
pub use hosted::HostedColorRange;
pub use labels::ColorRangeWords;
pub use value::{ColorRangeSelect, ColorRangeValue, ColorSamples, RangeControls, SampleSign, SampleTool};
