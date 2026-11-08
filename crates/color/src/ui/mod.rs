//! The colour widgets.
//!
//! - Fields and strips of a colour model: [`ColorArea`], [`ColorStrip`], [`ColorRamp`], [`ColorDisc`], [`ColorWheel`].
//! - The picker: [`ColorPicker`], its popover and dialog, the hex and readout fields, the choice list and the droppers.
//! - Wells and paints: [`SwatchPair`], [`PaintChip`], [`PaintList`], [`SwatchPalette`], [`SwatchTable`].
//! - Panels: [`ColorPanel`], [`HarmonyPanel`], [`SwatchesPanel`].
//! - Sampling: the eyedropper, colour samplers and the screen picker.
//! - Imaging prompts: [`LutPicker`] and [`ProfileMismatchDialog`].
//!
//! Colour edits are values of a colour model ([`ModelValues`](craft_theme::ModelValues)) or of [`PickedColor`], and
//! report through the edit lifecycle of the contract.

mod area;
mod channels;
mod choice;
mod disc;
mod droppers;
mod eyedropper;
mod harmony;
mod hex;
mod lut_picker;
mod paint_chip;
mod pair;
mod palette;
mod panel;
mod picker;
mod picker_dialog;
mod popover;
mod profile_prompts;
mod ramp;
mod readout;
mod samplers;
mod screen_picker;
mod strip;
mod swatches;
#[cfg(feature = "screen-testing")]
pub mod testing;
mod wheel;

pub use crate::AreaMode;
pub use area::ColorArea;
pub use choice::{ChosenColor, ColorChoice, NamedColor};
pub use disc::{ColorDisc, ColorDiscOutput, DiscMove};
pub use droppers::{DropperButton, DropperKind, DropperTool};
pub use eyedropper::{EyedropperBar, EyedropperOptions, EyedropperPick, EyedropperTool, SampleSize, SampleSource};
pub use harmony::{
    HarmonyCommand, HarmonyMenu, HarmonyMenuAction, HarmonyPanel, HarmonyPanelOutput, HarmonyPanelState, HarmonyPicked, HarmonyWheel,
    HarmonyWheelOutput, HarmonyWords,
};
pub use hex::{HexPairField, PairCodec};
pub use lut_picker::{LutChoice, LutPicker, LutPickerOutput, LutWords};
pub use paint_chip::{
    FillKind, GradientPreview, ImagePreview, Paint, PaintChip, PaintChipOutput, PaintEditor, PaintLink, PaintLinkKind, PaintList, PaintListEvent,
    PaintListOptions, PaintListOutput, PaintRow, PaintWords,
};
pub use pair::{PairWell, SwatchPair, SwatchPairOutput, Well};
pub use palette::{SwatchPalette, SwatchTable, SwatchTableOutput};
pub use panel::{ColorPanel, ColorPanelCommand, ColorPanelMenu, ColorPanelMode, ColorPanelOutput, ColorPanelRamp, ColorPanelState, ColorPanelWords, PanelTarget};
pub use picker::{
    ColorPicker, ColorPickerLayout, ColorPickerOutput, ColorPickerWords, ColorWarning, ColorWarningChip, PickedColor, ScreenPickGesture,
};
pub use picker_dialog::{ColorPickerDialog, ColorPickerDialogWords, ColorPickerMemory, ColorPickerResult};
pub use popover::{ColorPopover, ColorPopoverOutput};
pub use profile_prompts::{ProfileAction, ProfileAnswer, ProfileMismatchDialog, ProfileMode, ProfileQuestion, ProfileStatus, ProfileWords};
pub use ramp::{ColorRamp, ColorRampOutput, RampKind, RampPick};
pub use readout::ColorReadout;
pub use samplers::{
    ColorSamplerMarks, ColorSamplerTool, ColorSamplers, MAX_SAMPLERS, SamplerError, SamplerOptions, SamplerReadout, SamplerReadouts,
};
pub use screen_picker::{CaptureSource, LOUPE_PIXELS, LOUPE_ZOOM, ScreenColorPicker, ScreenPick, StillCapture};
pub use strip::ColorStrip;
pub use swatches::{
    MAX_GROUPS, MAX_SWATCHES, SwatchAt, SwatchLibrary, SwatchesCommand, SwatchesMenu, SwatchesMenuOutput, SwatchesPanel, SwatchesPanelOutput,
    SwatchesState, SwatchesWords,
};
pub use wheel::{WheelGeometry, WheelPart};
