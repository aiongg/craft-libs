//! [`PatternFillEditor`] (task SH-T12).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{AsIdSalt, IdSalt, Painter, Rect, Ui};

use super::{PatternFillLabels, PatternFillOptions, PatternSwatch};
use craft_ui::edit::EditResponse;

/// The app's painter of a swatch's tile.
type Thumbnails<'a> = dyn FnMut(&Painter, Rect, &PatternSwatch) + 'a;

/// One pattern fill editor body.
///
/// Bitmap mode ([`PatternFillOptions::colors`] `None`, Photoshop's Pattern Fill):
/// - The pattern field: S6a-presets' `PresetPicker` (a 40 × 40 thumbnail field opening a grid of
///   every swatch, names in tooltips; "No patterns" when the list is empty), with "Snap to Origin"
///   beside it, which sets the phase to (0, 0).
/// - "Angle": `AngleDial` row, −180 to 180. "Scale": a percent `NumberField`,
///   [`PatternFillOptions::SCALE`]. "Link with Layer" check box.
///
/// Preset mode (`colors` set, PowerPoint's Pattern fill): a grid of the swatches (each painted by
/// the host in the two colours; the selected one and the hovered or focused one ringed in
/// `accent.solid`), then Foreground and Background colour chips.
///
/// Every field reports through one [`EditResponse<PatternFillOptions>`]: a pick, a check box,
/// Snap to Origin and a typed value are `Set`; a scrub or a dial drag is a gesture. Keyboard: Tab
/// through the fields; the grid takes arrows. Right to left the rows mirror.
pub struct PatternFillEditor<'a> {
    id_salt: IdSalt,
    options: &'a PatternFillOptions,
    swatches: &'a [PatternSwatch],
    thumbnails: Option<Box<Thumbnails<'a>>>,
    labels: Option<PatternFillLabels>,
}

impl<'a> PatternFillEditor<'a> {
    /// The editor of `options`, offering `swatches`.
    pub fn new(id_salt: impl AsIdSalt, options: &'a PatternFillOptions, swatches: &'a [PatternSwatch]) -> Self {
        PatternFillEditor { id_salt: IdSalt::new(id_salt), options, swatches, thumbnails: None, labels: None }
    }

    /// Paints a swatch's tile into a rect (the picker's cells and field, the preset grid). Without
    /// it the cells show the swatch's name.
    pub fn thumbnails(mut self, paint: impl FnMut(&Painter, Rect, &PatternSwatch) + 'a) -> Self {
        self.thumbnails = Some(Box::new(paint));
        self
    }

    /// The words; the installed ones by default.
    pub fn labels(mut self, labels: PatternFillLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    pub fn show(self, ui: &mut Ui) -> EditResponse<PatternFillOptions> {
        // pc: crates/ui-egui/src/dialogs/pattern_fill_ui.rs:159-190 `body` (picker and Snap to Origin
        // :162-172, Angle :174-177, Scale :178-184, Link with Layer :185-189).
        // inv-deckcraft: crates/ui-egui/src/panels/fillui.rs:478-515 `pattern_editor` (22 × 22 cells,
        // 2 pt accent ring on the selected or hovered, Foreground and Background).
        todo!()
    }
}
