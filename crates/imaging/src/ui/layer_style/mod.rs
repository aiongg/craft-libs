//! The Layer Style dialog body ([`LayerStyleDialog`]): the effect list, the Styles page, the
//! Blending Options page with its [`BlendIfBar`]s, and the effect pages drawn from the
//! [`EffectSpec`](crate::layer_style::EffectSpec) tables, with [`ContourField`]s for contours.
//!
//! The dialog reads a [`LayerStyleDraft`](crate::layer_style::LayerStyleDraft) (the model in
//! [`crate::layer_style`], without egui). The host owns the dialog frame, the draft, the engine
//! and the preview; the dialog reports the edits and asks for colours, presets and contour
//! files. The edits' value is the [`LayerStyle`](crate::layer_style::LayerStyle) alone; the page,
//! the Preview check and the Blend If channel are reported beside it
//! ([`LayerStyleOutput::page`] and the other two), so the host's undo never moves them.
//!
//! ```no_run
//! use craft_imaging::layer_style::LayerStyleDraft;
//! use craft_imaging::ui::layer_style::LayerStyleDialog;
//! use craft_ui::edit::EditEvent;
//! # fn demo(ui: &mut egui::Ui, draft: &mut LayerStyleDraft, channels: &[String], modes: &[craft_ui::controls::BlendChoice], numbers: &craft_ui::localized::LocaleNumbers) {
//! let out = LayerStyleDialog::new("layer-style", draft, channels, numbers).blend_modes(modes).show(ui);
//! out.apply_view(draft); // the page, Preview and Blend If channel are the draft's view state
//! match out.edit.event {
//!     Some(EditEvent::Cancel { .. }) => { /* restore the style kept at Begin */ }
//!     Some(event) => {
//!         if let Some(value) = event.value() {
//!             draft.style = value.clone();
//!         }
//!     }
//!     None => {}
//! }
//! if let Some(request) = out.pick_color {
//!     // Open the colour picker; its answer: LayerStyleDialog::set_color(draft, &request.key, colour).
//! #   let _ = request;
//! }
//! # }
//! ```

mod blend_if;
#[cfg(test)]
mod blend_if_tests;
mod blending;
mod contour;
#[cfg(test)]
mod contour_tests;
mod dialog;
#[cfg(test)]
mod dialog_tests;
mod labels;
mod list;
mod page;
#[cfg(test)]
mod page_tests;
pub(crate) mod stories;
mod style;
mod styles;

pub use blend_if::BlendIfBar;
pub use contour::{ContourAction, ContourEditorDialog, ContourField, ContourFieldResponse, ContourRequest};
pub use dialog::{LayerStyleDialog, LayerStyleOutput};
pub use labels::{BlendIfBarLabels, ContourFieldLabels, LayerStyleLabels};
pub use page::PatternChoice;
pub use style::{BlendIfBarStyle, ContourFieldStyle, LayerStyleDialogStyle};
pub use styles::{StylePresets, StyleThumb};

pub use crate::layer_style::LayerStylePage;
