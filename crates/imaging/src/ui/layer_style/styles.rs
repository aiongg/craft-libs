//! The Styles page: the host's style presets in a [`PresetBrowser`] grid (task AD-8).

use egui::{Painter, Rect, Ui};

use craft_ui::controls::{Button, ButtonVariant};
use craft_ui::presets::{PresetBrowser, PresetEvent, PresetRow, PresetState, PresetView, ThumbInfo};
use craft_ui::theme::{Tokens, TokensOf as _};

use super::LayerStyleDialog;
use super::dialog::title_row;
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

/// Draws the page: the title with the "New Style…" button at its end, then the browser in a
/// `layer_style.presets_height` scroll area. A hovered thumbnail shows a 1.5 pt `accent.strong`
/// outline and its name as tooltip. Without presets the page shows only the button.
pub(super) fn styles_page(ui: &mut Ui, presets: Option<&mut StylePresets<'_>>, labels: &LayerStyleLabels) -> StylesOutput {
    // pc: crates/ui-egui/src/dialogs/layer_style.rs:943-978 (title, New Style…, groups of 32
    // thumbnails, hover outline, click applies).
    // lib: the groups and thumbnails are `PresetBrowser` in grid view; its rows draw the hover
    // ring and tooltip, so the page keeps none of its own.
    let style = LayerStyleDialog::style(&Tokens::of(ui));
    let new_style = title_row(ui, &labels.styles, |ui| {
        ui.add(Button::new(labels.new_style.as_str()).variant(ButtonVariant::Secondary).id_salt("new-style")).clicked()
    });
    let mut out = StylesOutput { apply_style: None, new_style };
    if let Some(presets) = presets {
        let StylePresets { state, rows, thumb } = presets;
        let thumb_side = style.preset_thumb;
        let browser = PresetBrowser::new("presets", labels.styles.as_str(), state, rows)
            .view_lock(PresetView::Grid)
            .thumb_range(thumb_side..=thumb_side)
            .context_menu(false)
            .max_height(style.presets_height)
            .shrink_to_content(true);
        let shown = browser.show(ui, |painter, rect, info| thumb(painter, rect, info));
        for event in shown.events {
            if let PresetEvent::Select { key, .. } | PresetEvent::Activate { key, .. } = event {
                out.apply_style = Some(key);
                break;
            }
        }
    }
    out
}
