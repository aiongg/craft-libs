//! `LayersSettings`, `LayersOptions`, `LayersOptionsDialog` and `paint_sample_thumbnail`: Layers
//! Panel Options (F11.03).
//!
//! `LayersSettings` is the panel's persisted section (`.settings(&mut ..)` on the panel);
//! `LayersOptions` is the value the dialog edits and returns (`.initial(..)`), held by the
//! settings.

//! design: allow(the sample thumbnail is original monochrome art drawn in fractions of its rect)

#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod dialog_tests;
#[cfg(test)]
mod tests;

use craft_ui::composites::GroupBox;
use craft_ui::controls::{Checkbox, RadioGroup, RadioSet};
use craft_ui::dialog::{Dialog, DialogId, DialogResult, DialogStack, DialogState, Footer, Outcome, StackDialog, Width, dialog_id};
use craft_ui::list::{ListRow, ListRowStyle, ThumbSize};
use craft_ui::localized::{SharedLabels as _, Words};
use craft_ui::primitives::Gap;
use craft_ui::theme::{Tokens, TokensOf as _};
use egui::{Color32, Context, Painter, Rect, Sense, Shape, Stroke, StrokeKind, Ui, pos2, vec2};

use super::panel::LayersStyle;
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
        Self { options: LayersOptions::default(), show_filters: true }
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
        Self {
            thumbnail_size: ThumbSize::Medium,
            thumbnail_contents: ThumbContents::EntireDocument,
            use_default_masks_on_fill_layers: true,
            expand_new_effects: true,
            add_copy_to_copied_layers_and_groups: true,
            preview_on_canvas_when_reordering: true,
            show_layer_mask_badges: false,
        }
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
        Self { id: dialog_id("layers.options"), draft: LayersOptions::default(), state: DialogState::new() }
    }

    /// Opens with `options`: the panel's current Panel Options.
    pub fn initial(mut self, options: LayersOptions) -> Self {
        self.draft = options;
        if matches!(self.draft.thumbnail_size, ThumbSize::Px(_)) {
            self.draft.thumbnail_size = ThumbSize::Medium;
        }
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
        let labels = LayersOptionsLabels::shared_ctx(ctx);
        let mut state = std::mem::take(&mut self.state);
        let shown = Dialog::new("layers.options", labels.title.as_str()).stack(stack).width(Width::Standard).footer(Footer::standard()).show(
            ctx,
            &mut state,
            &mut (),
            |ui, ()| self.body(ui, &labels),
        );
        self.state = state;
        match shown.outcome {
            Some(Outcome::Ok) => Some(DialogResult::Ok(self.draft.clone())),
            Some(Outcome::Cancel | Outcome::Close) => Some(DialogResult::Cancel),
            _ => None,
        }
    }

    fn confirm(&mut self, _ctx: &Context) -> Option<LayersOptions> {
        Some(self.draft.clone())
    }
}

impl LayersOptionsDialog {
    fn body(&mut self, ui: &mut Ui, labels: &LayersOptionsLabels) {
        let tokens = Tokens::of(ui);
        let words = Words::of_ctx(ui.ctx());
        let gap = Gap::Section.points(&tokens);
        let row = ListRow::<u8>::style(&tokens);
        let layers = LayersStyle::of(&tokens);
        let draft = &mut self.draft;
        GroupBox::new(labels.thumbnail_size.as_str()).show(ui, |ui| {
            let mut set = RadioSet::new("layers-options-size", labels.thumbnail_size.as_str(), &mut draft.thumbnail_size)
                .item(ThumbSize::None)
                .item(ThumbSize::Small)
                .item(ThumbSize::Medium)
                .item(ThumbSize::Large)
                .begin(ui);
            let mut picked = None;
            for (index, size) in ThumbSize::ALL.into_iter().enumerate() {
                let name = size.label(&words);
                ui.horizontal(|ui| {
                    if size == ThumbSize::None {
                        set.radio(ui, index, &name);
                        return;
                    }
                    let side = sample_side(&row, &layers, size);
                    set.radio_named(ui, index, "", &name).on_hover_text(name.as_ref());
                    ui.add_space(Gap::Inline.points(&tokens));
                    let (rect, response) = ui.allocate_exact_size(vec2(side, side), Sense::click());
                    paint_sample_thumbnail(ui.painter(), rect);
                    let description = labels.sample.as_str().replace("{side}", &format!("{side}"));
                    response.widget_info(|| egui::WidgetInfo::labeled(egui::accesskit::Role::Button, true, format!("{name}: {description}")));
                    if response.on_hover_text(name.as_ref()).clicked() {
                        picked = Some(size);
                    }
                });
            }
            let _ = set.end(ui);
            if let Some(size) = picked {
                draft.thumbnail_size = size;
            }
        });
        ui.add_space(gap);
        GroupBox::new(labels.thumbnail_contents.as_str()).show(ui, |ui| {
            let _ = RadioGroup::new("layers-options-contents", labels.thumbnail_contents.as_str(), &mut draft.thumbnail_contents)
                .item(ThumbContents::LayerBounds, labels.layer_bounds.as_str())
                .item(ThumbContents::EntireDocument, labels.entire_document.as_str())
                .show(ui);
        });
        ui.add_space(gap);
        let switches = [
            (&labels.default_masks, &mut draft.use_default_masks_on_fill_layers),
            (&labels.expand_new_effects, &mut draft.expand_new_effects),
            (&labels.add_copy, &mut draft.add_copy_to_copied_layers_and_groups),
            (&labels.preview_on_canvas, &mut draft.preview_on_canvas_when_reordering),
            (&labels.mask_badges, &mut draft.show_layer_mask_badges),
        ];
        for (label, value) in switches {
            let _ = Checkbox::bool(label.as_str(), value).show(ui);
        }
    }
}

/// The side of a size's sample in the dialog: twice the row thumbnail of `size` at the current
/// Size preset (`layers.options.sample_scale` × [`ListRowStyle::thumb_side`]); 0 for `None`.
pub(crate) fn sample_side(row: &ListRowStyle, layers: &LayersStyle, size: ThumbSize) -> f32 {
    layers.sample_scale * row.thumb_side(size)
}

/// Paints the dialog's sample thumbnail into `rect`: PhotoCraft's own monochrome miniature
/// landscape (light sky, shaded hills, a meadow with daisies), original art, clipped to `rect`.
/// It scales with `rect` and never mirrors.
pub fn paint_sample_thumbnail(painter: &Painter, rect: Rect) {
    let painter = painter.with_clip_rect(rect);
    let side = rect.width();
    let point = |x: f32, y: f32| pos2(rect.min.x + x * side, rect.min.y + y * side);
    const BANDS: u32 = 22;
    for n in 0..BANDS {
        let y0 = n as f32 / BANDS as f32;
        let y1 = (n + 1) as f32 / BANDS as f32;
        painter.rect_filled(Rect::from_min_max(point(0.0, y0), point(1.0, y1)), 0.0, Color32::from_gray((119.0 + y0 * 30.0) as u8));
    }
    painter.add(Shape::convex_polygon(
        vec![point(0.0, 0.60), point(0.22, 0.36), point(0.46, 0.56), point(0.72, 0.33), point(1.0, 0.62)],
        Color32::from_gray(80),
        Stroke::NONE,
    ));
    painter.add(Shape::convex_polygon(
        vec![point(0.0, 0.76), point(0.32, 0.60), point(0.57, 0.69), point(1.0, 0.51), point(1.0, 1.0), point(0.0, 1.0)],
        Color32::from_gray(64),
        Stroke::NONE,
    ));
    for n in 0..16 {
        let x = ((n * 43 + 13) % 97) as f32 / 96.0;
        let y = 0.78 + ((n * 11 % 19) as f32) / 100.0;
        painter.line_segment([point(x, 1.0), point((x + 0.02).min(1.0), y)], Stroke::new(side * 0.01, Color32::from_gray(104)));
    }
    // A daisy with layered petals and a dark centre disc, the same composition at every size.
    let center = point(0.49, 0.57);
    let unit = side * 0.34;
    for n in 0..15 {
        let theta = (n as f32 + 0.5) * std::f32::consts::TAU / 15.0;
        let direction = vec2(theta.cos(), theta.sin());
        let normal = vec2(-theta.sin(), theta.cos());
        let mut outline = Vec::with_capacity(20);
        for i in 0..=9 {
            let t = i as f32 / 9.0;
            let base = center + direction * unit * (0.22 + 0.83 * t);
            outline.push(base + normal * (unit * 0.22 * (std::f32::consts::PI * t).sin()));
        }
        for i in (0..=9).rev() {
            let t = i as f32 / 9.0;
            let base = center + direction * unit * (0.22 + 0.83 * t);
            outline.push(base - normal * (unit * 0.22 * (std::f32::consts::PI * t).sin()));
        }
        let fill = if n % 3 == 0 { Color32::from_gray(213) } else { Color32::from_gray(238) };
        painter.add(Shape::convex_polygon(outline, fill, Stroke::new(0.3, Color32::from_gray(155))));
    }
    painter.circle_filled(center, unit * 0.27, Color32::from_gray(33));
    painter.circle_filled(center - vec2(unit * 0.065, unit * 0.055), unit * 0.16, Color32::from_gray(55));
    for n in 0..16 {
        let theta = n as f32 * std::f32::consts::TAU / 16.0;
        let r = unit * if n % 2 == 0 { 0.16 } else { 0.21 };
        painter.circle_filled(center + vec2(theta.cos(), theta.sin()) * r, (side * 0.008).max(0.3), Color32::from_gray(119));
    }
    painter.rect_stroke(rect, 0.0, Stroke::new(1.0, Color32::from_gray(92)), StrokeKind::Inside);
}
