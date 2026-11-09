//! `ReadingMeasure`: a reading column's width follows the text size, and the least measure is shorter than the target.

use craft_i18n::Direction;
use craft_theme::{Contrast, Mode, TypeRole};
use craft_ui::primitives::ReadingMeasure;
use craft_ui::style::Ink;
use craft_ui::testing::style::{paint_frame, tokens};
use craft_ui::testing::{Frames, trial};

/// The measure for `role`, laid out the way a painter lays text out.
fn measure(role: TypeRole) -> ReadingMeasure {
    let mut frames = Frames::new();
    let tokens = tokens(Mode::Light, Contrast::Standard);
    let mut found = None;
    paint_frame(&mut frames, &tokens, Direction::Ltr, Ink::OnPanel, |draw| {
        found = Some(ReadingMeasure::of(role, |run| draw.layout(run, f32::INFINITY)));
    });
    found.expect("the frame painted")
}

