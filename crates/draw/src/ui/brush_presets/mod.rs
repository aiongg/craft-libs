//! The Brush Preset picker, its cards and chip.
//!
//! - [`BrushPresetCards`]: a preset library as Photoshop's brush cards (tip, stroke, name), a
//!   `PresetBrowser` with the card as its thumbnail; [`BrushCardParts`], [`BrushCardLayout`] and
//!   [`BrushCardsState`] are its parts, geometry and state.
//! - [`BrushPresetPicker`]: the popover with Size, Hardness, search, the gear menu, the cards and
//!   the card scale; it reports [`BrushPick`]s and the size and hardness edits.
//! - [`BrushPresetChip`]: the options-bar chip that toggles the picker.
//! - [`BrushPickerSettings`]: the picker's persisted view (card parts and scale).
//!
//! The app paints the tip and stroke images from its engine; the library never sees an engine
//! type. Presets are keyed by the app's key type (PhotoCraft: `PresetKey<String>`, by name).
//!
//! ```no_run
//! use craft_draw::ui::brush_presets::{BrushPick, BrushPickerState, BrushPickerValues, BrushPresetChip, BrushPresetPicker};
//! use craft_ui::popover::PopoverAnchor;
//! use craft_ui::presets::PresetRow;
//! use craft_ui::presets::model::PresetKey;
//! # fn paint(_: &egui::Painter, _: egui::Rect) {}
//! # egui::__run_test_ui(|ui| {
//! # let (mut state, mut open, size) = (BrushPickerState::<PresetKey<String>>::new(), false, 30.0);
//! # let rows: Vec<PresetRow<PresetKey<String>>> = Vec::new();
//! let chip = BrushPresetChip::new("brush-chip").diameter(size).open(open).show(ui, paint);
//! if chip.clicked {
//!     open = !open;
//! }
//! let out = BrushPresetPicker::new("brush-picker", PopoverAnchor::Control(chip.response.clone()), &mut state, &rows)
//!     .toggle(chip.response.id)
//!     .values(BrushPickerValues { diameter: size, hardness: Some(0.8) })
//!     .show(ui, &mut open);
//! for pick in out.picks {
//!     match pick {
//!         BrushPick::List(_event) => { /* the app's preset commands */ }
//!         BrushPick::OpenSettings | BrushPick::NewPreset | BrushPick::Import => {}
//!         _ => {}
//!     }
//! }
//! # });
//! ```

mod card;
#[cfg(test)]
mod card_tests;
mod chip;
#[cfg(test)]
mod chip_tests;
mod labels;
mod picker;
#[cfg(test)]
mod picker_tests;
mod settings;
#[cfg(test)]
mod settings_tests;
pub(crate) mod stories;
mod style;

pub use card::{BrushCardLayout, BrushCardPart, BrushCardParts, BrushCardsState, BrushPresetCards, tip_thumb_scale};
pub use chip::{BrushPresetChip, BrushPresetChipResponse};
pub use labels::{BrushCardLabels, BrushPickerLabels};
pub use picker::{BrushPick, BrushPickerState, BrushPickerValues, BrushPresetPicker, BrushPresetPickerOutput};
pub use settings::BrushPickerSettings;
pub use style::{BrushCardStyle, BrushPickerStyle, BrushPresetChipStyle};
