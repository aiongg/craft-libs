//! `LayersSettings`, `LayersOptions`, `LayersOptionsDialog` and `paint_sample_thumbnail`: Layers
//! Panel Options (F11.03).
//!
//! `LayersSettings` is the panel's persisted section (`.settings(&mut ..)` on the panel);
//! `LayersOptions` is the value the dialog edits and returns (`.initial(..)`), held by the
//! settings.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod dialog_tests;
#[cfg(test)]
mod tests;

use craft_ui::dialog::{DialogId, DialogResult, DialogStack, DialogState, StackDialog};
use craft_ui::list::{ListRowStyle, ThumbSize};
use egui::{Context, Painter, Rect};

use super::thumbs::ThumbContents;

/// The Layers panel's persisted section: the Panel Options and the panel menu's Show Filters.
/// Every field has a serde default, so a section saved by an older version loads.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct LayersSettings {
    /// The Panel Options dialog's value.
    pub options: LayersOptions,
    /// The panel menu's Filter Options › Show: the filter bar shows (default true).
    pub show_filters: bool,
}

impl Default for LayersSettings {
    /// [`LayersOptions::default`], filters shown.
    fn default() -> Self {
        // pc: crates/ui-egui/src/state/panel_state.rs:41-52 `LayersPanelOptions::defaults` (show_filters true).
        todo!()
    }
}

/// Layers Panel Options: what the dialog edits and returns. Every field has a serde default.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct LayersOptions {
    /// Thumbnail Size: [`ThumbSize::None`], `Small`, `Medium` (default) or `Large`; the row
    /// height follows the Size preset's `list.*` tokens. `Px` is not offered and reads as
    /// `Medium`.
    pub thumbnail_size: ThumbSize,
    /// Thumbnail Contents: Layer Bounds or Entire Document (default). The panel menu's Clip
    /// Thumbnails to Layer Bounds is the same setting; it is part of every
    /// [`LayerThumb`](super::thumbs::LayerThumb) key of a pixel thumbnail.
    pub thumbnail_contents: ThumbContents,
    /// Use Default Masks on Fill Layers (default true): the app reads it when it makes a fill
    /// layer.
    pub use_default_masks_on_fill_layers: bool,
    /// Expand New Effects (default true): the app opens a layer's effects when it adds one.
    pub expand_new_effects: bool,
    /// Add "copy" to Copied Layers and Groups (default true): the app reads it when it
    /// duplicates.
    pub add_copy_to_copied_layers_and_groups: bool,
    /// Preview on canvas when reordering layers (default true): the panel reports
    /// [`LayersOutput::drop_preview`](super::panel::LayersOutput::drop_preview) only while it is on.
    pub preview_on_canvas_when_reordering: bool,
    /// Show layer mask badges (default false): a small badge on each mask thumbnail's corner
    /// (a pen for a vector mask, a dot for a pixel mask).
    pub show_layer_mask_badges: bool,
}

impl Default for LayersOptions {
    /// Photoshop's defaults: Medium, Entire Document, the first four switches on, badges off.
    fn default() -> Self {
        // pc: crates/ui-egui/src/state/panel_state.rs:41-52 `LayersPanelOptions::defaults`.
        todo!()
    }
}

craft_ui::words! {
    /// The words of Layers Panel Options. The size names are [`ThumbSize::label`]'s.
    pub struct LayersOptionsLabels {
        title: "layers.options.title" = "Layers Panel Options";
        thumbnail_size: "layers.options.thumbnailSize" = "Thumbnail Size";
        /// A size radio's description for a screen reader: `{side}` is the sample's side in points.
        sample: "layers.options.sample" = "{side} by {side} point thumbnails";
        thumbnail_contents: "layers.options.thumbnailContents" = "Thumbnail Contents";
        layer_bounds: "layers.options.layerBounds" = "Layer Bounds";
        entire_document: "layers.options.entireDocument" = "Entire Document";
        default_masks: "layers.options.defaultMasks" = "Use Default Masks on Fill Layers";
        expand_new_effects: "layers.options.expandNewEffects" = "Expand New Effects";
        add_copy: "layers.options.addCopy" = "Add “copy” to Copied Layers and Groups";
        preview_on_canvas: "layers.options.previewOnCanvas" = "Preview on canvas when reordering layers";
        mask_badges: "layers.options.maskBadges" = "Show layer mask badges";
    }
}

/// The Layers Panel Options dialog, a [`StackDialog`] whose result is the edited
/// [`LayersOptions`].
///
/// ```ignore
/// dialogs.open(LayersOptionsDialog::new().initial(prefs.layers.options.clone()));
/// // later
/// if let Some(DialogResult::Ok(options)) = dialogs.take::<LayersOptionsDialog>() { prefs.layers.options = options; }
/// ```
///
/// - Layout (`Width::Standard`, the library footer with OK and Cancel in the platform order): a
///   [`GroupBox`](craft_ui::composites::GroupBox) "Thumbnail Size" with a
///   [`RadioGroup`](craft_ui::controls::RadioGroup) of None (a text label), then Small, Medium and
///   Large, each with a real-size sample ([`paint_sample_thumbnail`]) at twice the row thumbnail
///   (`layers.options.sample_scale` × `list.thumb.*`: 36, 48, 96 at Compact); a click on a
///   sample picks its size. A group box "Thumbnail Contents" with Layer Bounds and Entire
///   Document; then the five switches as [`Checkbox`](craft_ui::controls::Checkbox)es, in
///   PhotoCraft's order.
/// - Keyboard: the radio groups take arrows; Tab walks the groups and checkboxes; Enter is OK,
///   Esc is Cancel (the library dialog's keys).
/// - AccessKit: the dialog frame's roles; the size radios are named "None", "Small", "Medium",
///   "Large" and described by [`LayersOptionsLabels::sample`].
/// - Returns `Ok(options)` on OK, `Cancel` on Cancel or Esc; nothing is applied while it is open.
pub struct LayersOptionsDialog {
    id: DialogId,
    draft: LayersOptions,
    state: DialogState,
}

impl LayersOptionsDialog {
    /// The dialog at Photoshop's defaults ([`LayersOptions::default`]).
    pub fn new() -> Self {
        // new: `dialog_id("layers.options")`, draft `LayersOptions::default()`, a new `DialogState`.
        todo!()
    }

    /// Opens with `options`: the panel's current Panel Options.
    pub fn initial(mut self, options: LayersOptions) -> Self {
        self.draft = options;
        self
    }
}

impl Default for LayersOptionsDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl StackDialog for LayersOptionsDialog {
    type Result = LayersOptions;

    fn id(&self) -> DialogId {
        self.id
    }

    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<LayersOptions>> {
        // pc: crates/ui-egui/src/panels/layers_panel_ui.rs:197-250 `options_body` (group boxes 308
        // wide, samples 38, 68, 96, the five checkboxes); the dialog itself as `NewGuideDialog::show`
        // (`craft_ui::dialog::guides`), with `Width::Standard` and `Footer::standard()`.
        todo!()
    }

    fn confirm(&mut self, ctx: &Context) -> Option<LayersOptions> {
        // new: OK without drawing: the draft as it is.
        todo!()
    }
}

/// The side of a size's sample in the dialog: twice the row thumbnail of `size` at the current
/// Size preset (`layers.options.sample_scale` × [`ListRowStyle::thumb_side`]); 0 for `None`.
pub(crate) fn sample_side(style: &ListRowStyle, size: ThumbSize) -> f32 {
    // pc: crates/ui-egui/src/panels/layers_panel_ui.rs:210-212 (38, 68, 96: about twice 18, 34, 48).
    todo!()
}

/// Paints the dialog's sample thumbnail into `rect`: PhotoCraft's own monochrome miniature
/// landscape (light sky, shaded hills, a meadow with daisies), original art, clipped to `rect`.
/// It scales with `rect` and never mirrors.
pub fn paint_sample_thumbnail(painter: &Painter, rect: Rect) {
    // pc: crates/ui-egui/src/panels/layers_panel_ui.rs:114-174 `paint_flower_example`.
    todo!()
}
