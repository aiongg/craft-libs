//! [`AdjustSliders`]: a table of slider rows for the adjustments that are only sliders (Exposure,
//! Vibrance, Posterize, VectorCraft's colour effects), with Photoshop's double-click to default
//! and PhotoCraft's rounding (task AD-3).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::ops::RangeInclusive;

use craft_theme::Srgba;
use egui::{AsIdSalt, IdSalt, Ui};

use craft_ui::edit::{EditPolicy, EditResponse};
use craft_ui::localized::{LocaleNumbers, Unit};

use super::{AdjustContext, AdjustLabels};

/// A slider row's track.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum AdjustTrack {
    /// The slider's own track.
    #[default]
    Plain,
    /// A colour track through these stops, start to end (an image tone, not chrome), drawn only
    /// while [`AdjustContext::dynamic_color_sliders`] is on; the plain track otherwise.
    Colors(Vec<Srgba>),
}

/// One row of an [`AdjustSliders`] table.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct AdjustField {
    /// The host's key for the value (the engine parameter's name); the library never reads it.
    pub key: &'static str,
    /// The row's caption, translated.
    pub label: String,
    pub range: RangeInclusive<f64>,
    /// The value a double-click on the slider sets.
    pub default: f64,
    /// The field's unit ("{n} %", "{n}°"), none by default.
    pub unit: Option<Unit>,
    /// Decimal places kept; `None` rounds as [`round`](Self::round) says.
    pub decimals: Option<usize>,
    pub track: AdjustTrack,
}

impl AdjustField {
    /// A row with a plain track, no unit and the range's own rounding.
    pub fn new(key: &'static str, label: impl Into<String>, range: RangeInclusive<f64>, default: f64) -> Self {
        Self { key, label: label.into(), range, default, unit: None, decimals: None, track: AdjustTrack::Plain }
    }

    /// The field's unit.
    pub fn unit(mut self, unit: Unit) -> Self {
        self.unit = Some(unit);
        self
    }

    /// Decimal places the value keeps.
    pub fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = Some(decimals);
        self
    }

    /// The row's track.
    pub fn track(mut self, track: AdjustTrack) -> Self {
        self.track = track;
        self
    }

    /// The value the row stores for `x` (lead call L25, PhotoCraft's `row_value`): rounded to
    /// `decimals` places when set (at most 6), else to 0.01 for a range 10 wide or less, else to
    /// a whole number.
    pub fn round(&self, x: f64) -> f64 {
        // pc: crates/ui-egui/src/dialogs/adjust_editors/basic.rs:12-22 `row_value`.
        todo!()
    }

    /// Exposure's rows: Exposure −20..20 (2 places), Offset −0.5..0.5 (4 places), Gamma Correction
    /// 0.01..9.99 (default 1); keys `exposure`, `offset`, `gamma`. Exposure's wheel step is 0.01.
    pub fn exposure(labels: &AdjustLabels) -> Vec<AdjustField> {
        // pc: crates/ui-egui/src/dialogs/adjust_editors.rs:146-154 (the Exposure table).
        todo!()
    }

    /// Vibrance's rows: Vibrance and Saturation, −100..100 %, default 0; keys `vibrance`,
    /// `saturation`.
    pub fn vibrance(labels: &AdjustLabels) -> Vec<AdjustField> {
        // pc: crates/ui-egui/src/dialogs/adjust_editors.rs:155.
        todo!()
    }

    /// Posterize's row: Levels 2..255, default 4; key `levels`.
    pub fn posterize(labels: &AdjustLabels) -> Vec<AdjustField> {
        // pc: crates/ui-egui/src/dialogs/adjust_editors.rs:161.
        todo!()
    }
}

/// A table of [`SliderRow`](craft_ui::composites::SliderRow)s over the host's values, one per
/// [`AdjustField`] (same index).
///
/// - Each row: caption, slider and number field; the colour track when the field has one and the
///   context's dynamic tracks are on.
/// - Double-click on a slider: the field's default, one `Set` (Photoshop).
/// - Mouse wheel over a slider: steps it in a dialog ([`AdjustContext::wheel`]), by 1 or the
///   field's smallest decimal step (Exposure 0.01); nothing in the Properties panel.
/// - Every value the table reports is rounded with [`AdjustField::round`].
/// - Keyboard and AccessKit: each row's field and slider as [`SliderRow`](craft_ui::composites::SliderRow)
///   has them (labelled by the caption, arrows, Shift ×10, Page Up and Down).
#[must_use = "show the table with `show`"]
pub struct AdjustSliders<'a> {
    id_salt: IdSalt,
    fields: &'a [AdjustField],
    values: &'a [f64],
    numbers: &'a LocaleNumbers,
    context: AdjustContext,
    policy: EditPolicy,
    enabled: bool,
}

impl<'a> AdjustSliders<'a> {
    /// The table of `fields` over `values` (same length).
    pub fn new(id_salt: impl AsIdSalt, fields: &'a [AdjustField], values: &'a [f64], numbers: &'a LocaleNumbers) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            fields,
            values,
            numbers,
            context: AdjustContext::dialog(true),
            policy: EditPolicy::default(),
            enabled: true,
        }
    }

    /// Dialog or Properties panel; a dialog by default.
    pub fn context(mut self, context: AdjustContext) -> Self {
        self.context = context;
        self
    }

    /// The rows' edit policy.
    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Enables or disables every row.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Draws the rows. The edit carries every value of the table (the changed one rounded), so
    /// the host stores it as is.
    pub fn show(self, ui: &mut Ui) -> EditResponse<Vec<f64>> {
        // pc: crates/ui-egui/src/dialogs/adjust_editors/basic.rs:24-37 `sliders` (`RowGestures::reset`
        // to the default, `row_value`), :46-50 `track_stops`.
        // lib: `SliderRow::new(..).default_value(..).wheel(context.wheel()).track_colors(..)`.
        todo!()
    }
}
