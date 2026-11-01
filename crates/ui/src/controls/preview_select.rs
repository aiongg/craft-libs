//! Painted previews for a [`Select`](super::Select): line styles drawn as samples, and how much of
//! an option's text shows beside its preview ([`SelectText`]).
//!
//! A preview select is a `Select` with [`row_content`](super::Select::row_content) and
//! [`text_shown`](super::Select::text_shown):
//!
//! ```ignore
//! Select::new("underline", "Underline", &mut style)
//!     .rows(LineStyle::UNDERLINES.len(), |i| LineStyle::UNDERLINES[i], |i| LineStyle::UNDERLINES[i].label())
//!     .row_content(vec2(160.0, 22.0), |p, r, s| paint_line_style(p, r, *s, ink))
//!     .text_shown(SelectText::Never)
//!     .show(ui);
//! ```

use egui::{Color32, Painter, Rect, Shape, Stroke, pos2};

use crate::localized::Word;

/// Where a select shows its options' text beside their [row content](super::Select::row_content).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SelectText {
    /// In the list and in the closed box (every select's default).
    #[default]
    Always,
    /// In the list only; the closed box shows the preview alone (DesignCraft's Stroke Type and
    /// corner shape boxes, InDesign's).
    InList,
    /// Nowhere: rows and box are previews (WordCraft's underline styles, Word's). The text stays
    /// the option's accessible name and its tooltip, and type-ahead and search still use it.
    Never,
}

/// A line's look, for underline, stroke and border style lists.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LineStyle {
    Solid,
    /// Two thin lines 3 points apart.
    Double,
    /// 2.5 points wide.
    Thick,
    Dotted,
    Dashed,
    DotDash,
    DotDotDash,
    Wave,
    /// Two lines, the upper 3 wide and the lower 1 (InDesign's Thick - Thin).
    ThickThin,
    ThinThick,
    ThinThin,
    ThickThick,
}

impl LineStyle {
    /// WordCraft's Underline list, in its order (Word's).
    pub const UNDERLINES: [LineStyle; 8] = [
        LineStyle::Solid,
        LineStyle::Double,
        LineStyle::Thick,
        LineStyle::Dotted,
        LineStyle::Dashed,
        LineStyle::DotDash,
        LineStyle::DotDotDash,
        LineStyle::Wave,
    ];

    /// DesignCraft's Stroke Type list, in its order (InDesign's).
    pub const STROKES: [LineStyle; 7] = [
        LineStyle::Solid,
        LineStyle::Dashed,
        LineStyle::Dotted,
        LineStyle::ThickThin,
        LineStyle::ThinThick,
        LineStyle::ThinThin,
        LineStyle::ThickThick,
    ];

    /// The English name the word tables key ("Single" for an underline's `Solid`, "Dot-dash",
    /// "Thick - Thin"): WordCraft's and DesignCraft's names.
    // wordcraft: crates/ui-egui/src/ribbon/underline.rs:5 UNDERLINE_STYLES; designcraft: crates/ui-egui/src/panels/properties/object.rs:236 TYPES
    pub fn label(self) -> &'static str {
        LINE_STYLE_WORDS[self as usize].english
    }
}

/// The names of [`LineStyle::label`], plain words looked up by their English text, in the order of
/// the variants.
pub(crate) const LINE_STYLE_WORDS: &[Word] = &[
    Word::plain("Single"),
    Word::plain("Double"),
    Word::plain("Thick"),
    Word::plain("Dotted"),
    Word::plain("Dashed"),
    Word::plain("Dot-dash"),
    Word::plain("Dot-dot-dash"),
    Word::plain("Wave"),
    Word::plain("Thick - Thin"),
    Word::plain("Thin - Thick"),
    Word::plain("Thin - Thin"),
    Word::plain("Thick - Thick"),
];

/// A sample of `style` across `rect` at its middle, in `color`, 1 point lines unless the style
/// says otherwise: dots of radius 0.75 every 1.5 + gap points, dashes from (length, gap) runs
/// (WordCraft's `pattern`), a sine wave of amplitude 1.6 and period 6 for `Wave`; the two-line styles 6 apart
/// (DesignCraft's `paint_stroke_type`, 3 above and below the middle). The line sits on a pixel
/// centre so 1-point lines stay sharp.
// wordcraft: crates/ui-egui/src/ribbon/underline.rs:55 paint_underline
// designcraft: crates/ui-egui/src/panels/properties/object.rs:258 paint_stroke_type
pub fn paint_line_style(painter: &Painter, rect: Rect, style: LineStyle, color: Color32) {
    let y = rect.center().y.round() + 0.5;
    let (x0, x1) = (rect.left(), rect.right());
    let line = |y: f32, width: f32| {
        painter.line_segment([pos2(x0, y), pos2(x1, y)], Stroke::new(width, color));
    };
    // Repeats (length, gap) runs along the line; a length of 0 is a dot.
    let pattern = |runs: &[(f32, f32)]| {
        let mut x = x0;
        loop {
            for &(length, gap) in runs {
                if x >= x1 {
                    return;
                }
                if length == 0.0 {
                    painter.circle_filled(pos2(x + 0.75, y), 0.75, color);
                    x += 1.5 + gap;
                } else {
                    painter.line_segment([pos2(x, y), pos2((x + length).min(x1), y)], Stroke::new(1.0, color));
                    x += length + gap;
                }
            }
        }
    };
    let pair = |upper: f32, lower: f32| {
        line(y - 3.0, upper);
        line(y + 3.0, lower);
    };
    match style {
        LineStyle::Solid => line(y, 1.0),
        LineStyle::Double => {
            line(y - 1.5, 1.0);
            line(y + 1.5, 1.0);
        }
        LineStyle::Thick => line(y, 2.5),
        LineStyle::Dotted => pattern(&[(0.0, 2.0)]),
        LineStyle::Dashed => pattern(&[(6.0, 4.0)]),
        LineStyle::DotDash => pattern(&[(6.0, 3.0), (0.0, 3.0)]),
        LineStyle::DotDotDash => pattern(&[(6.0, 3.0), (0.0, 2.5), (0.0, 3.0)]),
        LineStyle::Wave => {
            let points =
                (0..=((x1 - x0).max(0.0) as usize)).map(|i| pos2(x0 + i as f32, y + 1.6 * (i as f32 * std::f32::consts::TAU / 6.0).sin())).collect();
            painter.add(Shape::line(points, Stroke::new(1.0, color)));
        }
        LineStyle::ThickThin => pair(3.0, 1.0),
        LineStyle::ThinThick => pair(1.0, 3.0),
        LineStyle::ThinThin => pair(1.0, 1.0),
        LineStyle::ThickThick => pair(2.5, 2.5),
    }
}
