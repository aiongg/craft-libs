//! [`SliceBar`]: the Slice tools' options bar cells (task SH-T9).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{AsIdSalt, IdSalt, Ui};

use super::{SliceCommand, SliceLabels, SliceMode, SliceOrigin, SliceToolSettings};
use craft_ui::canvas::Constraint;
use craft_ui::edit::EditEvent;
use craft_ui::localized::units::LengthUnit;

/// What [`SliceBar::show`] reports.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
#[must_use = "the app stores the style and runs the command"]
pub struct SliceBarOutput {
    /// The Style, Width or Height changed (`ConstraintFieldsOutput::event`): tool settings, no
    /// undo step. A style pick resets the numbers (Fixed Size 64 × 64, else 1 × 1).
    pub style: Option<EditEvent<Constraint>>,
    /// A unit picked from a Fixed Size field's menu.
    pub unit_picked: Option<LengthUnit>,
    /// A button: Slices From Guides (Slice); Promote, Divide…, Delete, Slice Options… and the Hide
    /// Auto Slices check box (Select).
    pub command: Option<SliceCommand>,
}

/// The cells a Slice tool adds to the options bar.
///
/// - Slice: the kernel's `ConstraintFields` (Style, Width, swap, Height) over
///   [`SliceToolSettings::style`], then "Slices From Guides".
/// - Select: Promote (enabled for a selected auto or layer slice), Divide…, Delete (enabled for a
///   selected slice), "Hide Auto Slices" check box, "Slice Options…".
/// - Keyboard: each cell is a Tab stop in the bar's order.
pub struct SliceBar<'a> {
    id_salt: IdSalt,
    mode: SliceMode,
    settings: &'a SliceToolSettings,
    selected: Option<SliceOrigin>,
    labels: Option<SliceLabels>,
}

impl<'a> SliceBar<'a> {
    /// The bar of `mode` with `settings`.
    pub fn new(id_salt: impl AsIdSalt, mode: SliceMode, settings: &'a SliceToolSettings) -> Self {
        SliceBar { id_salt: IdSalt::new(id_salt), mode, settings, selected: None, labels: None }
    }

    /// The selected slice's origin, or `None` without a selection: enables Promote, Divide…,
    /// Delete and Slice Options….
    pub fn selected(mut self, origin: Option<SliceOrigin>) -> Self {
        self.selected = origin;
        self
    }

    /// The words; the installed ones by default.
    pub fn labels(mut self, labels: SliceLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Adds the cells to the bar's row `ui`.
    pub fn show(self, ui: &mut Ui) -> SliceBarOutput {
        // pc: crates/ui-egui/src/tools/slice_ui.rs:404-472 `options_bar` (Style :414-417 with the
        // 64 × 64 / 1 × 1 reset, the swap button :433, Slices From Guides :440, Promote :447,
        // Divide… :450, Delete :453, Hide Auto Slices :460, Slice Options… :462).
        // new: the Style cells are `craft_ui::canvas::ConstraintFields` (K13).
        todo!()
    }
}
