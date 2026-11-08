//! A number field is as wide as the widest value of its range.

use craft_ui::contract::{Bind, Props as _, Show as _};
use craft_ui::testing::{Frames, trial};
use craft_ui::text::NumberField;
use egui::{Rect, Ui};
use std::cell::Cell;
use std::ops::RangeInclusive;

/// A colour channel.
const CHANNEL: RangeInclusive<f64> = 0.0..=255.0;
/// A wide range.
const WIDE: RangeInclusive<f64> = 0.0..=100_000.0;

/// The field's rect and whether the text of its widest value fits in it, with the field holding that value.
fn holding_the_widest(range: &RangeInclusive<f64>) -> (Rect, bool) {
    let mut frames = Frames::new();
    let mut value = Some(*range.end());
    let rect = Cell::new(Rect::NOTHING);
    let mut show = |ui: &mut Ui| {
        rect.set(NumberField::new("Value", Bind::bound(&mut value)).range(range.clone()).id_salt("width").show(ui).response.rect);
    };
    frames.idle(4, &mut show);
    let text = format!("{}", range.end());
    let fits = frames.texts().iter().any(|run| run.text == text && !run.elided && rect.get().contains_rect(run.rect));
    (rect.get(), fits)
}

/// The width follows the range: a 0-255 channel is narrower than a 0-100000 field, and each holds its widest value
/// uncut.
#[trial]
fn a_number_field_is_as_wide_as_its_widest_value() {
    let (channel, channel_fits) = holding_the_widest(&CHANNEL);
    let (wide, wide_fits) = holding_the_widest(&WIDE);
    assert!(channel_fits, "255 fits its field: {channel:?}");
    assert!(wide_fits, "100000 fits its field: {wide:?}");
    assert!(channel.width() < wide.width(), "the channel field is narrower: {} against {}", channel.width(), wide.width());
}
