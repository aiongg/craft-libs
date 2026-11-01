//! Photoshop's adjustments as editors: each takes a plain value and reports an
//! [`EditResponse`](craft_ui::edit::EditResponse) of it, and the same editor serves the Image ›
//! Adjustments dialog and the Properties panel ([`AdjustContext`]). The app converts values to its
//! engine's parameters; the library has no JSON.
//!
//! - Tone: [`LevelsEditor`] (one channel's levels over its histogram), [`LevelsPanel`] and
//!   [`CurvesPanel`] around it and the kernel's curve editor, with the channels
//!   ([`ToneChannels`]) and the point pickers ([`TonePickerRow`], [`TonePickTool`]).
//! - Slider sets: [`AdjustSliders`] (Exposure, Vibrance, Posterize, any table),
//!   [`BrightnessContrastEditor`], [`ThresholdEditor`], [`ColorBalanceEditor`],
//!   [`PhotoFilterEditor`], [`HueSaturationEditor`], [`BlackWhiteEditor`],
//!   [`ChannelMixerEditor`], [`SelectiveColorEditor`].
//! - The Properties panel's session ([`AdjustSession`]).
//!
//! Gradient Map is `craft_draw::ui::gradient::map`, beside the gradient editor it builds on.
//!
//! ```no_run
//! use craft_imaging::ui::adjust::{AdjustContext, LevelsPanel, Levels, ToneChannels, ToneSpace};
//! use craft_ui::edit::EditEvent;
//! # fn demo(ui: &mut egui::Ui, levels: &mut Vec<Levels>, numbers: &craft_ui::localized::LocaleNumbers) {
//! let out = LevelsPanel::new("adj-levels", levels, ToneChannels::of(ToneSpace::Rgb, false), numbers)
//!     .context(AdjustContext::dialog(true))
//!     .show(ui);
//! match out.edit.event {
//!     Some(EditEvent::Update { value, .. }) => { /* preview `value` for `out.channel` */ }
//!     Some(EditEvent::Commit { value, .. } | EditEvent::Set { value, .. }) => levels[out.channel] = value,
//!     _ => {}
//! }
//! # }
//! ```

mod basic;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod basic_tests;
mod context;
mod curves_panel;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod curves_panel_tests;
mod hue_saturation;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod hue_saturation_tests;
mod labels;
mod levels_editor;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod levels_editor_tests;
mod levels_panel;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod levels_panel_tests;
mod mixers;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod mixers_tests;
mod session;
#[cfg(test)]
mod session_tests;
mod sliders;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod sliders_tests;
pub(crate) mod stories;
mod style;
#[cfg(test)]
mod thumb_row_tests;
mod tone;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod tone_tests;

pub use basic::{
    BrightnessContrast, BrightnessContrastEditor, ColorBalance, ColorBalanceEditor, ColorBalanceTone, PhotoFilter, PhotoFilterEditor,
    PhotoFilterOutput, PhotoFilterPreset, ThresholdEditor,
};
pub use context::{AdjustContext, ToneChannel, ToneChannels, ToneSpace};
pub use curves_panel::{CurvesMode, CurvesPanel, CurvesPanelOutput, ToneCurves};
pub use hue_saturation::{HueRangeValue, HueSaturation, HueSaturationEditor};
pub use labels::AdjustLabels;
pub use levels_editor::{Levels, LevelsClip, LevelsEditor, LevelsLabels, LevelsStyle};
pub use levels_panel::{LevelsPanel, LevelsPanelOutput};
pub use mixers::{
    BlackWhite, BlackWhiteEditor, BlackWhiteOutput, ChannelMixer, ChannelMixerEditor, SelectiveColor, SelectiveColorEditor, SelectiveMethod,
};
pub use session::AdjustSession;
pub use sliders::{AdjustField, AdjustSliders, AdjustTrack};
pub use style::AdjustStyle;
pub use tone::{TonePick, TonePickTool, TonePicker, TonePickerResponse, TonePickerRow};

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &[
    "LevelsEditor",
    "LevelsPanel",
    "CurvesPanel",
    "AdjustSliders",
    "BrightnessContrastEditor",
    "ThresholdEditor",
    "HueSaturationEditor",
    "ColorBalanceEditor",
    "BlackWhiteEditor",
    "PhotoFilterEditor",
    "ChannelMixerEditor",
    "SelectiveColorEditor",
    "AdjustSession",
];

/// The Levels editor's story and the other editors' stories.
pub fn stories() -> Vec<craft_ui::story::Story> {
    let mut all = levels_editor::stories();
    all.extend(stories::stories());
    all
}
