//! [`ContourField`]: an effect's Contour row, its preset picker and the Contour Editor (task
//! AD-10).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land (task AD-10)")]

use egui::{AsIdSalt, IdSalt, Response, Ui};

use craft_ui::composites::Curve;
use craft_ui::edit::EditEvent;

use super::labels::ContourFieldLabels;
use crate::layer_style::Contour;

/// What the picker's menu or the editor asked of the host's contour library. The library itself
/// (and its `.shc` files) is the host's.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum ContourAction {
    /// Load Contours…: add a file's contours to the library.
    Load,
    /// Replace Contours…: replace the library with a file's.
    Replace,
    /// Save Contours…: save the library (only offered when it has contours).
    Save,
    /// Delete Contour on the host's contour at this index of
    /// [`user_contours`](ContourField::user_contours).
    Delete(usize),
    /// New… in the editor: the edited curve under the name the user typed, to add to the library.
    New(Contour),
}

/// What a [`ContourField`] reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct ContourFieldResponse {
    /// The thumbnail.
    pub response: Response,
    /// The contour's edits: a preset or library pick is one `Set`; the Contour Editor is one
    /// gesture from opening (`Begin`) through its edits (`Update`, so the preview follows) to OK
    /// (`Commit`) or Cancel and Esc (`Cancel`: the host restores the contour it had at `Begin`).
    /// An edited built-in preset is named "Custom" ([`Contour::edited`]).
    pub edit: Option<EditEvent<Contour>>,
    /// The Anti-aliased check's new value, when clicked.
    pub anti_alias: Option<bool>,
    /// A library request from the picker's menu or the editor.
    pub action: Option<ContourAction>,
}

/// An effect's Contour row: the label, a thumbnail of the curve (a click opens the Contour
/// Editor), the preset picker's arrow and, for effects that have one, the Anti-aliased check.
///
/// - Picker (the arrow; a popover): the built-in presets ([`Contour::presets`], Photoshop's
///   order) then the host's contours, as `layer_style.contour_cell` cells with the curve drawn in
///   each and the name as tooltip; arrows move, Enter or a click picks (one `Set`) and closes. A
///   menu at its end offers Load Contours…, Replace Contours…, Save Contours… (disabled with no
///   host contours); a host contour's context menu offers Delete Contour.
/// - Contour Editor (a modal over the dialog): Preset select (the built-in names), a
///   `layer_style.contour_graph` square `CurveEditor` with corner points (levels 0..=255, at most
///   [`Contour::MAX_POINTS`] points), Input and Output fields in percent and a Corner check for
///   the selected point, OK and Cancel, New… (asks a name, then [`ContourAction::New`]), Load…
///   and Save… (as the picker's menu).
/// - AccessKit: the thumbnail is a `Button` named [`ContourFieldLabels::editor`] whose value is
///   the contour's name; the arrow a `Button` named [`ContourFieldLabels::presets`] with a popup.
#[must_use = "show the field with `show`"]
pub struct ContourField<'a> {
    id_salt: IdSalt,
    label: String,
    contour: &'a Contour,
    anti_alias: Option<bool>,
    user_contours: &'a [Contour],
    labels: Option<ContourFieldLabels>,
    enabled: bool,
}

impl<'a> ContourField<'a> {
    /// The row named `label` ("Contour", "Gloss Contour"; empty for Bevel & Emboss's contour
    /// under its check) over `contour`.
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, contour: &'a Contour) -> Self {
        ContourField {
            id_salt: IdSalt::new(id_salt),
            label: label.into(),
            contour,
            anti_alias: None,
            user_contours: &[],
            labels: None,
            enabled: true,
        }
    }

    /// Shows the Anti-aliased check with this value; hidden by default.
    pub fn anti_alias(mut self, on: Option<bool>) -> Self {
        self.anti_alias = on;
        self
    }

    /// The host's contour library, listed after the built-in presets.
    pub fn user_contours(mut self, contours: &'a [Contour]) -> Self {
        self.user_contours = contours;
        self
    }

    /// The words; the installed ones by default.
    pub fn labels(mut self, labels: ContourFieldLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Draws the row, and the picker and the editor while open (their state is the field's, in
    /// egui memory under its id).
    pub fn show(self, ui: &mut Ui) -> ContourFieldResponse {
        // pc: crates/ui-egui/src/dialogs/layer_style/contour.rs:93-222 `row` (thumbnail 36, arrow,
        // picker grid of 40 cells, Load/Replace/Save Contours…, Delete Contour, Anti-aliased),
        // :323-516 `editor` (Preset select, 220 graph, Input/Output, Corner, OK, Cancel, New…, Load…,
        // Save…), :306-320 `follow_corners`.
        todo!()
    }

    /// `contour` as the curve the editor edits: points scaled to levels 0..=255, corner flags
    /// kept.
    pub fn to_curve(contour: &Contour) -> Curve {
        // pc: crates/ui-egui/src/dialogs/layer_style/contour.rs:29-37 `levels_of`.
        todo!()
    }

    /// The editor's `curve` back as a contour named `name`: points scaled to 0..=1 and validated
    /// by [`Contour::from_points`]; `None` for a drawn (`Curve::Map`) or invalid curve.
    pub fn from_curve(curve: &Curve, name: &str) -> Option<Contour> {
        // pc: crates/ui-egui/src/dialogs/layer_style/contour.rs:518-525 `shown_contour`.
        todo!()
    }
}
