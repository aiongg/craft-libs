//! The Brush Preset picker's persisted view ([`BrushPickerSettings`]).

use std::ops::RangeInclusive;

use serde::{Deserialize, Serialize};

use super::card::{BrushCardParts, BrushCardsState};

/// What the picker remembers across restarts: which card parts show and the card scale. The app
/// saves it in its settings (serde) and passes it back at launch; the search, open groups and an
/// open rename are not saved.
///
/// The app saves it when [`BrushPresetPickerOutput::settings`](super::BrushPresetPickerOutput::settings)
/// is set, and restores it with [`BrushCardsState::restore`].
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BrushPickerSettings {
    /// Which parts the cards show.
    pub parts: BrushCardParts,
    /// The card scale.
    pub scale: f32,
}

impl Default for BrushPickerSettings {
    fn default() -> Self {
        Self { parts: BrushCardParts::ALL, scale: 1.0 }
    }
}

impl BrushPickerSettings {
    /// The footer slider's range: under 0.15 the card padding and rows stop making sense.
    pub const SCALE_RANGE: RangeInclusive<f32> = 0.15..=2.0;

    /// The settings made safe to show: the scale clamped to [`SCALE_RANGE`](Self::SCALE_RANGE)
    /// (one that isn't a finite number gives 1), and every part on when none is.
    pub fn restored(self) -> Self {
        // pc: crates/ui-egui/src/tools/brush_picker.rs:134-155 `restore` (clamp to SCALE_RANGE; non-finite keeps the default; none on gives all)
        let scale = if self.scale.is_finite() { self.scale.clamp(*Self::SCALE_RANGE.start(), *Self::SCALE_RANGE.end()) } else { 1.0 };
        Self { parts: self.parts.normalised(), scale }
    }
}

impl<K> BrushCardsState<K> {
    /// The part of this state that persists.
    pub fn settings(&self) -> BrushPickerSettings {
        // new: the parts and the scale, as stored.
        BrushPickerSettings { parts: self.parts, scale: self.scale }
    }

    /// Sets the parts and scale from `settings`, [`restored`](BrushPickerSettings::restored)
    /// first.
    pub fn restore(&mut self, settings: &BrushPickerSettings) {
        // pc: crates/ui-egui/src/tools/brush_picker.rs:134-155 `restore`
        let BrushPickerSettings { parts, scale } = settings.restored();
        self.parts = parts;
        self.scale = scale;
    }
}
