//! The Styles page: the host's style presets in a [`PresetBrowser`] grid (task AD-8).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land (task AD-8)")]

use egui::{Painter, Rect, Ui};

use craft_ui::presets::{PresetRow, PresetState, ThumbInfo};

use super::labels::LayerStyleLabels;

/// The thumbnail painter of the Styles page: paints preset `info` into `rect`.
pub type StyleThumb<'a> = Box<dyn FnMut(&Painter, Rect, ThumbInfo<'_, String>) + 'a>;

/// The host's style presets for the Styles page: the preset browser's state and rows (groups
/// and presets keyed by the preset's name, as PhotoCraft keys them) and the thumbnail painter.
/// The page uses the library [`PresetBrowser`](craft_ui::presets::PresetBrowser) in grid view
/// at `layer_style.preset_thumb` (lead call L28).
pub struct StylePresets<'a> {
    pub(super) state: &'a mut PresetState<String>,
    pub(super) rows: &'a [PresetRow<String>],
    pub(super) thumb: StyleThumb<'a>,
}

impl<'a> StylePresets<'a> {
    pub fn new(
        state: &'a mut PresetState<String>,
        rows: &'a [PresetRow<String>],
        thumb: impl FnMut(&Painter, Rect, ThumbInfo<'_, String>) + 'a,
    ) -> Self {
        StylePresets { state, rows, thumb: Box::new(thumb) }
    }
}

/// What the Styles page asked for.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct StylesOutput {
    /// A click, Space or Enter on a preset: apply it to the draft (the host answers with
    /// [`LayerStyleDraft::apply_preset`](crate::layer_style::LayerStyleDraft::apply_preset);
    /// the document is untouched until OK).
    pub apply_style: Option<String>,
    /// New Style… was pressed.
    pub new_style: bool,
}

/// Draws the page body: the "New Style…" button at the title row's end, then the browser in a
/// `layer_style.presets_height` scroll area. A hovered thumbnail shows a 1.5 pt `accent.strong`
/// outline and its name as tooltip. Without presets the page shows only the button.
pub(super) fn styles_page(ui: &mut Ui, presets: Option<&mut StylePresets<'_>>, labels: &LayerStyleLabels) -> StylesOutput {
    // pc: crates/ui-egui/src/dialogs/layer_style.rs:943-978 (title, New Style…, groups of 32
    // thumbnails, hover outline, click applies).
    todo!()
}
