//! [`AdjustSliders`]: a table of slider rows for the adjustments that are only sliders (Exposure,
//! Vibrance, Posterize, VectorCraft's colour effects), with Photoshop's double-click to default
//! and PhotoCraft's rounding (task AD-3).

use std::ops::RangeInclusive;

use craft_theme::Srgba;
use egui::{AsIdSalt, IdSalt, Ui};

use craft_ui::composites::SliderRow;
use craft_ui::edit::{EditEvent, EditPolicy, EditResponse};
use craft_ui::localized::{LocaleNumbers, Unit};

use super::{AdjustContext, AdjustLabels};

/// The percent unit of the sliders that show one.
pub(super) const PERCENT: &str = "{n} %";

/// The most decimal places a field keeps.
const MAX_DECIMALS: usize = 6;

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
        match self.decimals {
            Some(places) => {
                let scale = 10f64.powi(places.min(MAX_DECIMALS) as i32);
                (x * scale).round() / scale
            }
            None if self.range.end() - self.range.start() <= 10.0 => (x * 100.0).round() / 100.0,
            None => x.round(),
        }
    }

    /// The row's step: the smallest decimal step of its places, else 0.01 for a range 10 wide or
    /// less and 1 above. Every value the row reports is on it, so [`round`](Self::round) leaves
    /// a reported value as it is.
    fn step(&self) -> f64 {
        match self.decimals {
            Some(places) => 10f64.powi(-(places.min(MAX_DECIMALS) as i32)),
            None if self.range.end() - self.range.start() <= 10.0 => 0.01,
            None => 1.0,
        }
    }

    /// How far one wheel notch moves the slider: the row's step, but not finer than 0.01 (a
    /// notch of 0.0001 across Offset's range would not move the thumb).
    fn wheel_step(&self) -> f64 {
        self.step().max(0.01)
    }

    /// `x` held in the row's range.
    pub(super) fn clamp(&self, x: f64) -> f64 {
        x.max(*self.range.start()).min(*self.range.end())
    }

    /// Exposure's rows: Exposure −20..20 (2 places), Offset −0.5..0.5 (4 places), Gamma Correction
    /// 0.01..9.99 (default 1); keys `exposure`, `offset`, `gamma`. Exposure's wheel step is 0.01.
    pub fn exposure(labels: &AdjustLabels) -> Vec<AdjustField> {
        // pc: crates/ui-egui/src/dialogs/adjust_editors.rs:146-154 (the Exposure table).
        vec![
            AdjustField::new("exposure", labels.exposure.as_str(), -20.0..=20.0, 0.0).decimals(2),
            AdjustField::new("offset", labels.offset.as_str(), -0.5..=0.5, 0.0).decimals(4),
            AdjustField::new("gamma", labels.gamma_correction.as_str(), 0.01..=9.99, 1.0),
        ]
    }

    /// Vibrance's rows: Vibrance and Saturation, −100..100 %, default 0; keys `vibrance`,
    /// `saturation`.
    pub fn vibrance(labels: &AdjustLabels) -> Vec<AdjustField> {
        // pc: crates/ui-egui/src/dialogs/adjust_editors.rs:155.
        vec![
            AdjustField::new("vibrance", labels.vibrance.as_str(), -100.0..=100.0, 0.0).unit(Unit::new(PERCENT)),
            AdjustField::new("saturation", labels.saturation.as_str(), -100.0..=100.0, 0.0).unit(Unit::new(PERCENT)),
        ]
    }

    /// Posterize's row: Levels 2..255, default 4; key `levels`.
    pub fn posterize(labels: &AdjustLabels) -> Vec<AdjustField> {
        // pc: crates/ui-egui/src/dialogs/adjust_editors.rs:161.
        vec![AdjustField::new("levels", labels.levels.as_str(), 2.0..=255.0, 4.0)]
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
        let enabled = self.enabled && ui.is_enabled();
        let all: Vec<f64> = self.fields.iter().enumerate().map(|(i, f)| f.clamp(self.values.get(i).copied().unwrap_or(f.default))).collect();
        let mut events = Vec::new();
        let (mut submitted, mut escaped) = (false, false);
        let group = ui.scope(|ui| {
            for (i, field) in self.fields.iter().enumerate() {
                let mut row = SliderRow::new((self.id_salt, field.key), field.label.as_str(), all[i], field.range.clone(), self.numbers)
                    .default_value(field.default)
                    .step(field.step())
                    .wheel(self.context.wheel())
                    .wheel_step(field.wheel_step())
                    .policy(self.policy)
                    .enabled(enabled);
                if let Some(unit) = &field.unit {
                    row = row.unit(unit.clone());
                }
                if let Some(places) = field.decimals {
                    row = row.decimals(places);
                }
                if let AdjustTrack::Colors(stops) = &field.track
                    && let Some(stops) = self.context.track(stops)
                {
                    row = row.track_colors(stops.iter().copied());
                }
                let shown = row.show(ui);
                submitted |= shown.field.submitted || shown.slider.submitted;
                escaped |= shown.field.escaped || shown.slider.escaped;
                events.extend([shown.field.event, shown.slider.event].into_iter().flatten().map(|event| (i, event)));
            }
        });
        let picked = pick(events, |(_, event)| is_ending(event));
        let event = picked.map(|(i, event)| {
            event.map(|value| {
                let mut table = all.clone();
                table[i] = self.fields[i].round(value);
                table
            })
        });
        let mut response = group.response;
        if event.as_ref().is_some_and(|event| event.value().is_some()) {
            response.mark_changed();
        }
        EditResponse::new(response, event).submitted(submitted).escaped(escaped)
    }
}

/// Whether `event` ends a gesture or is a one-shot change: the events that must not be dropped
/// for a begin or an update of another control in the same pass.
pub(super) fn is_ending<T>(event: &EditEvent<T>) -> bool {
    matches!(event, EditEvent::Commit { .. } | EditEvent::Cancel { .. } | EditEvent::Set { .. })
}

/// The one event of several controls in a pass: the first that ends a gesture, else the first.
/// Two controls report in one pass only when the user moves from one to the other (a field's
/// commit on focus loss beside the next control's begin); the next pass reports the later one's
/// update.
pub(super) fn pick<T>(events: Vec<T>, ending: impl Fn(&T) -> bool) -> Option<T> {
    let at = events.iter().position(ending).unwrap_or(0);
    events.into_iter().nth(at)
}
