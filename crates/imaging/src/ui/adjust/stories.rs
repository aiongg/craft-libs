//! Stories of the adjustment editors (task ST-AD1): a stub, not registered. Registering it (task
//! RG2) adds these stories to `adjust::stories()` (today the Levels editor's) and the new
//! components to `adjust::COMPONENTS`, and regenerates the registry dump; until then the gallery
//! would run the `todo!()` bodies.
//!
//! Histograms, spectra and samples are fakes made by the story (a ramp histogram, a hue rotation
//! for the adjusted spectrum), so every state shows without an engine. Every string goes through
//! story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `imaging/adjust/levels-panel` | RGB with a fake histogram, Auto pressed, a picker armed, a grey document |
//! | `imaging/adjust/curves-panel` | Points; pencil with Smooth; the overlays on the composite; an isolated mask channel |
//! | `imaging/adjust/sliders` | Exposure, Vibrance, Posterize, and a VectorCraft-style table with colour tracks |
//! | `imaging/adjust/brightness-contrast` | Normal and Use Legacy |
//! | `imaging/adjust/threshold` | The marker on a fake histogram |
//! | `imaging/adjust/hue-saturation` | Master, a range with its thumbs, Colorize, a range wrapping through 0° |
//! | `imaging/adjust/color-balance` | The three tones |
//! | `imaging/adjust/black-white` | Default and tinted |
//! | `imaging/adjust/photo-filter` | A preset and a custom colour |
//! | `imaging/adjust/channel-mixer` | Colour, Monochrome, Total over 100 % |
//! | `imaging/adjust/selective-color` | Relative, Absolute, Neutrals |
//! | `imaging/adjust/properties-panel` | The same editors with `AdjustContext::properties` (no wheel) and an `AdjustSession` Reset |
//!
//! `COMPONENTS` for RG2: `LevelsPanel`, `CurvesPanel`, `TonePickerRow`, `AdjustSliders`,
//! `BrightnessContrastEditor`, `ThresholdEditor`, `HueSaturationEditor`, `ColorBalanceEditor`,
//! `BlackWhiteEditor`, `PhotoFilterEditor`, `ChannelMixerEditor`, `SelectiveColorEditor`.

#![expect(dead_code, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::story::Story;

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    // new: see the module's table.
    todo!()
}
