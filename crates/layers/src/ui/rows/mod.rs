//! `BlendRows`: Blend mode, Opacity, Lock and Fill as one block (F11.04).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod tests;

use craft_ui::edit::EditEvent;
use egui::{AsIdSalt, IdSalt, Response, Ui};

use super::tree::{BlendChoice, LayerProperties, LockKind};

/// An edit of the active layer's properties, which the app applies to every selected layer (the
/// Background left out of a multi-layer change, as PhotoCraft).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum LayerPropertyEdit {
    /// The blend mode, by the app's key: a pick, an arrow key or a wheel notch over the closed
    /// select, each a `Set`.
    Blend(EditEvent<String>),
    /// The mode under the pointer in the open blend list (`None` when it leaves the list): the
    /// app previews it on the canvas. Not an edit.
    BlendHover(Option<String>),
    /// Opacity, 0 to 100: a scrub or slider drag is a gesture (`Begin`, `Update`, `Commit` or
    /// `Cancel`, Esc restores), typing, an arrow step and a wheel step are `Set`.
    Opacity(EditEvent<f64>),
    /// Fill, 0 to 100, as Opacity.
    Fill(EditEvent<f64>),
    /// A lock button: the lock `kind` asks to be `on`.
    Lock {
        /// The lock.
        kind: LockKind,
        /// The state asked for.
        on: bool,
    },
}

craft_ui::words! {
    /// The words of the blend, opacity, lock and fill rows.
    pub struct BlendRowsLabels {
        /// The blend select's name for a screen reader.
        blend_mode: "layers.blendMode" = "Blend Mode";
        opacity: "layers.opacity" = "Opacity:";
        fill: "layers.fill" = "Fill:";
        lock: "layers.lock" = "Lock:";
        /// The lock buttons' tooltips and names, in [`LockKind::ALL`] order.
        lock_transparency: "layers.lockTransparency" = "Lock transparent pixels";
        lock_pixels: "layers.lockPixels" = "Lock image pixels";
        lock_position: "layers.lockPosition" = "Lock position";
        lock_artboard: "layers.lockArtboard" = "Prevent auto-nesting in and out of Artboards and Frames";
        lock_all: "layers.lockAll" = "Lock all";
        /// Why Lock transparent pixels is disabled, for a screen reader.
        transparency_included: "layers.lockTransparencyIncluded" = "Included in Lock image pixels";
    }
}

/// Blend mode, Opacity, Lock and Fill as one block: the two rows above the Layers panel's list,
/// usable alone (a Properties panel).
///
/// ```ignore
/// let tree = DocLayers::new(&doc);
/// let props = tree.properties();
/// let modes = tree.blend_modes(props.as_ref().is_some_and(|p| p.group), props.as_ref().map_or("normal", |p| &p.blend_key));
/// for edit in BlendRows::new("blend-rows", props.as_ref(), &modes).show(ui).events {
///     app.set_properties(edit); // every selected layer
/// }
/// ```
///
/// - **Row 1**: the blend [`Select`](craft_ui::controls::Select) (Small) with `blend_modes` and
///   their separators, then "Opacity:" as a scrub label and a Small
///   [`NumberField`](craft_ui::controls::NumberField) 0 to 100 % with its pop-up slider,
///   `layers.percent_field_width` wide. A wheel notch over the closed select steps one mode
///   (`Set` each); hovering a mode in the open list reports `BlendHover`. The "Opacity:" label
///   hides when `available < label + field + 2 gaps + 16 + layers.blend.min_width`, and the
///   select takes the rest of the row (at least 40).
/// - **Row 2** (with [`locks`](Self::locks)): "Lock:", five Small toggle
///   [`IconButton`](craft_ui::controls::IconButton)s in [`LockKind::ALL`] order (Lock image pixels
///   disables Lock transparent pixels), then "Fill:" and its field as Opacity's, right-aligned in
///   one column with it. The "Lock:" label drops before the Fill label would run over the lock
///   icons.
/// - **States**: with `properties` `None` the rows show Normal, 100 %, unlocked, all greyed. With
///   `editable` false (the Background) the select, Opacity and Fill grey; the locks stay.
/// - **Values**: in the pass after an edit the app passes back its rounded copy of the value
///   (the shared rule L19); in other passes a changed value is an outside change.
/// - **Keyboard**: each control is a Tab stop; the fields take arrows (with Shift by 10) and
///   typing; Esc during a scrub cancels it.
/// - **AccessKit**: a `ComboBox` "Blend Mode"; spin buttons "Opacity" and "Fill" with the value
///   and the % unit; the lock buttons are toggle buttons named by their tooltip with their pressed
///   state, the disabled one described by [`BlendRowsLabels::transparency_included`].
/// - Right to left, the rows mirror; number text keeps its digits' direction.
#[must_use = "show the rows with `show`"]
pub struct BlendRows<'a> {
    id_salt: IdSalt,
    properties: Option<&'a LayerProperties>,
    blend_modes: &'a [BlendChoice],
    locks: bool,
    labels: Option<BlendRowsLabels>,
}

/// What [`BlendRows::show`] returns.
#[derive(Debug)]
#[must_use = "the host applies the edits"]
pub struct BlendRowsOutput {
    /// The edits of this pass, in order.
    pub events: Vec<LayerPropertyEdit>,
    /// The block.
    pub response: Response,
}

impl<'a> BlendRows<'a> {
    /// The rows for the active layer's `properties` (`None`: no active layer), offering
    /// `blend_modes`; `id_salt` keeps the fields' gestures.
    pub fn new(id_salt: impl AsIdSalt, properties: Option<&'a LayerProperties>, blend_modes: &'a [BlendChoice]) -> Self {
        Self { id_salt: IdSalt::new(id_salt), properties, blend_modes, locks: true, labels: None }
    }

    /// Shows the Lock and Fill row (default true).
    pub fn locks(mut self, locks: bool) -> Self {
        self.locks = locks;
        self
    }

    /// The rows' words; the installed words by default.
    pub fn labels(mut self, labels: BlendRowsLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Draws the rows over the available width.
    pub fn show(self, ui: &mut Ui) -> BlendRowsOutput {
        // pc: crates/ui-egui/src/panels/layers.rs:104-235 (blend and Opacity row, Lock and Fill row);
        // pc: crates/ui-egui/src/panels/layers.rs:51-102 `inactive_blend_rows` (no layer).
        todo!()
    }
}

/// The width of the blend select and whether the "Opacity:" label shows, for an `available`
/// row width, the label's width, the field's width (`layers.percent_field_width` plus the pop-up
/// arrow), the gap and the select's least width (`layers.blend.min_width`, 80): the label shows
/// while `available >= label + field + 2 × gap + 16 + min_blend`; the select gets
/// `available - field - gap - (label + gap + 16 when shown)`, at least 40.
pub(crate) fn opacity_row_layout(available: f32, label_width: f32, field_width: f32, gap: f32, min_blend: f32) -> (f32, bool) {
    // pc: crates/ui-egui/src/panels/layers.rs:39-46 `opacity_row_layout`.
    todo!()
}
