//! The brush Size row ([`BrushSizeField`]): a value field over a log slider.
#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::edit::EditResponse;
use egui::{AsIdSalt, IdSalt, Ui};

use super::labels::BrushSettingsLabels;

/// The brush Size row of Brush Tip Shape and of the Brush Preset picker's head: the label and a
/// [`NumberField`](craft_ui::controls::NumberField) (px, whole pixels, 1 to the maximum, the
/// field's default width) on one line, a [`Slider`](craft_ui::controls::Slider) with
/// [`SliderScale::Log`](craft_ui::controls::SliderScale::Log) under it, and with
/// [`restore`](Self::restore) Photoshop's "Restore original size" icon button (sampled tips)
/// before the field.
///
/// The field and the slider show one value: a slider drag rounds to whole pixels and stays one
/// gesture (`Begin`, `Update`, `Commit`) while the app writes the rounded value back; a typed
/// value is a `Set`; restore is a `Set` of the original size. Esc during a drag cancels.
///
/// AccessKit: the field is a `SpinButton` named by the label, the slider a `Slider` of the same
/// name, the restore button a `Button` "Restore original size".
#[must_use = "show the field with `show`"]
pub struct BrushSizeField {
    id_salt: IdSalt,
    label: String,
    diameter: f64,
    max: f64,
    restore: Option<f64>,
    labels: Option<BrushSettingsLabels>,
}

impl BrushSizeField {
    /// A Size row named `label` for `diameter` pixels, 1 to `max` (the app's engine limit).
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, diameter: f64, max: f64) -> Self {
        Self { id_salt: IdSalt::new(id_salt), label: label.into(), diameter, max, restore: None, labels: None }
    }

    /// Shows the "Restore original size" button, which sets `original` (clamped to 1 to the
    /// maximum). `None`: no button.
    pub fn restore(mut self, original: Option<f64>) -> Self {
        self.restore = original;
        self
    }

    /// The words; the installed language's by default.
    pub fn labels(mut self, labels: BrushSettingsLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Shows the row and reports the new diameter.
    pub fn show(self, ui: &mut Ui) -> EditResponse<f64> {
        // pc: crates/ui-egui/src/panels/brush_sections.rs:129-150 `size_row` (field 74 wide, restore button, log slider, 4 pt after)
        // pc: crates/ui-egui/src/tools/brush_picker.rs:232-247 (the picker head's Size: field and log slider)
        todo!()
    }
}
