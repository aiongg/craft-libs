//! The rename field: `TextField` with the look sheet's field surface (3.11).

use crate::text_field::TextFieldStyle;
use craft_theme::FontSlot;
use craft_ui_rt::{FontSpec, Margin, Rgba, Stroke};

use crate::look::Look;

/// craft leaves egui's own caret on its text fields: 2 points wide in #c0deff, reaching 1.5
/// points above and below the line of text.
const CARET: Rgba = Rgba::rgb(0xc0, 0xde, 0xff);
const CARET_WIDTH: f32 = 2.0;
const CARET_OVERHANG: f32 = 1.5;
/// craft blinks the caret itself: shown, then hidden, 0.53 s each from its last move.
const CARET_BLINK: f64 = 0.53;

/// craft's `TextField`: the text at the font's own line height, selected in `accent.soft`.
pub fn rename_style(look: &Look) -> TextFieldStyle {
    TextFieldStyle {
        font: FontSpec { slot: FontSlot::Ui, size: look.body_size },
        line_height: None,
        text: look.text,
        fill: look.sunken,
        stroke: Some(Stroke::new(look.thin, look.control_edge)),
        focus_stroke: Some(Stroke::new(look.thin, look.focus)),
        radius: look.radius_control,
        padding: Margin::new(look.pad, 0.0, look.pad, 0.0),
        height: look.control,
        selection: look.accent_soft,
        caret: CARET,
        caret_width: CARET_WIDTH,
        caret_overhang: CARET_OVERHANG,
        caret_blink: CARET_BLINK,
    }
}
