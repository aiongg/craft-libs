//! [`AdjustmentKind`] and the Adjustments panel: a grid of icon buttons, one per adjustment.

use craft_behavior::IconId;
use craft_ui::contract::{NoValue, Outcome, Pattern};
use craft_ui::runtime::env::Label;
use egui::{Response, Ui};

use super::AdjustWords;

/// One of the adjustments the Adjustments panel offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AdjustmentKind {
    /// Brightness/Contrast.
    BrightnessContrast,
    /// Levels.
    Levels,
    /// Curves.
    Curves,
    /// Exposure.
    Exposure,
    /// Vibrance.
    Vibrance,
    /// Hue/Saturation.
    HueSaturation,
    /// Color Balance.
    ColorBalance,
    /// Black & White.
    BlackWhite,
    /// Photo Filter.
    PhotoFilter,
    /// Channel Mixer.
    ChannelMixer,
    /// Invert.
    Invert,
    /// Posterize.
    Posterize,
    /// Threshold.
    Threshold,
    /// Gradient Map.
    GradientMap,
    /// Selective Color.
    SelectiveColor,
    /// Color Lookup.
    ColorLookup,
}

impl AdjustmentKind {
    /// Every kind, in the panel's order.
    pub const ALL: [AdjustmentKind; 16] = [
        AdjustmentKind::BrightnessContrast,
        AdjustmentKind::Levels,
        AdjustmentKind::Curves,
        AdjustmentKind::Exposure,
        AdjustmentKind::Vibrance,
        AdjustmentKind::HueSaturation,
        AdjustmentKind::ColorBalance,
        AdjustmentKind::BlackWhite,
        AdjustmentKind::PhotoFilter,
        AdjustmentKind::ChannelMixer,
        AdjustmentKind::Invert,
        AdjustmentKind::Posterize,
        AdjustmentKind::Threshold,
        AdjustmentKind::GradientMap,
        AdjustmentKind::SelectiveColor,
        AdjustmentKind::ColorLookup,
    ];

    /// The kind's key (`"brightnessContrast"`).
    pub fn key(self) -> &'static str {
        let _ = self;
        todo!("pn.C44")
    }

    /// The kind's icon from the catalog.
    pub fn icon(self) -> IconId {
        let _ = self;
        todo!("pn.C44")
    }

    /// The kind's title.
    pub fn label(self, words: &AdjustWords) -> Label {
        let _ = (self, words);
        todo!("pn.C44")
    }
}

/// What the Adjustments panel reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct AdjustmentsPanelOutput {
    /// The panel's response.
    pub response: Response,
    /// The kind the user picked this pass.
    pub picked: Option<AdjustmentKind>,
}

impl Outcome for AdjustmentsPanelOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// A grid of icon buttons in one roving group; the app picks which kinds it offers (default all).
#[must_use = "show the panel with `show`"]
#[derive(Pattern)]
#[allow(dead_code)] // P3(panels-w11): pn.C44
pub struct AdjustmentsPanel<'a> {
    label: Label,
    kinds: &'a [AdjustmentKind],
}

impl<'a> AdjustmentsPanel<'a> {
    /// A panel named `label` offering every kind.
    pub fn new(label: impl Into<Label>) -> Self {
        let _ = label.into();
        todo!("pn.C44")
    }

    /// Offers only `kinds`.
    #[must_use]
    pub fn kinds(mut self, kinds: &'a [AdjustmentKind]) -> Self {
        let _ = (&mut self, kinds);
        todo!("pn.C44")
    }

    /// Draws the grid and reports the picked kind.
    pub fn show(self, ui: &mut Ui) -> AdjustmentsPanelOutput {
        let _ = (self, ui);
        todo!("pn.C44")
    }
}
