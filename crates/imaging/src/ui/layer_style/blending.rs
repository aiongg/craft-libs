//! The Blending Options page (task AD-9).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land (task AD-9)")]

use egui::Ui;

use craft_ui::controls::BlendChoice;
use craft_ui::edit::EditEvent;
use craft_ui::localized::LocaleNumbers;

use super::labels::{BlendIfBarLabels, LayerStyleLabels};
use crate::layer_style::Blending;

/// What the Blending Options page reads.
pub(super) struct BlendingInput<'a> {
    pub blending: &'a Blending,
    /// The Blend If entry shown.
    pub blend_if_channel: usize,
    /// The document's colour channels.
    pub channel_names: &'a [String],
    pub blend_modes: &'a [BlendChoice],
    pub labels: &'a LayerStyleLabels,
    pub bar_labels: &'a BlendIfBarLabels,
    pub numbers: &'a LocaleNumbers,
    pub enabled: bool,
}

/// What the Blending Options page reports in one pass.
#[derive(Default)]
pub(super) struct BlendingOutput {
    /// The page's values after an edit (slider drags as a gesture; every other control a `Set`).
    pub edit: Option<EditEvent<Blending>>,
    /// The Blend If select picked another entry (view state).
    pub blend_if_channel: Option<usize>,
}

/// Draws Photoshop's Blending Options page:
///
/// - General Blending: Blend Mode (`Select` over `input.blend_modes`; a mode the list lacks, such as an
///   imported Paint.NET mode, is added at the end so it still shows), Opacity 0..=100 %.
/// - Advanced Blending: Fill Opacity 0..=100 %; "Channels:" with one check per name in
///   `input.channel_names` (checked: the channel blends); Knockout (None, Shallow, Deep); the five
///   checks in Photoshop's order (Blend Interior Effects as Group, Blend Clipped Layers as Group,
///   Transparency Shapes Layer, Layer Mask Hides Effects, Vector Mask Hides Effects).
/// - Blend If (hidden with no entries): the "Blend If:" select ([`blend_if_entry_label`] names
///   the entries), then for This Layer and Underlying Layer a caption with the readout and a
///   [`BlendIfBar`](super::BlendIfBar).
///
/// Sections are divided by hairlines and headed by section labels (`type.label`).
pub(super) fn blending_page(ui: &mut Ui, input: &BlendingInput<'_>) -> BlendingOutput {
    // pc: crates/ui-egui/src/dialogs/layer_style.rs:1252-1350 `blending_page`, :1197 `KNOCKOUT`,
    // :1200-1206 `ADVANCED_CHECKS`.
    todo!()
}

/// The name of Blend If entry `index` when the style has `entries` entries and the document's
/// channels are `channel_names`: with as many entries as names (a one-channel document) the
/// name itself; otherwise entry 0 is `gray` and entry `i` is name `i − 1`.
pub(super) fn blend_if_entry_label(index: usize, entries: usize, channel_names: &[String], gray: &str) -> String {
    // pc: crates/ui-egui/src/dialogs/layer_style.rs:1317-1325 (the entry labels).
    todo!()
}
