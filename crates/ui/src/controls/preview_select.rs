//! Painted previews for a [`Select`](super::Select): line styles drawn as samples, and how much of
//! an option's text shows beside its preview ([`SelectText`]).
//!
//! A preview select is a `Select` with [`row_content`](super::Select::row_content) and
//! [`text_shown`](super::Select::text_shown):
//!
//! ```ignore
//! Select::new("underline", "Underline", &mut style)
//!     .rows(LineStyle::UNDERLINES.len(), |i| LineStyle::UNDERLINES[i], |i| LineStyle::UNDERLINES[i].label())
//!     .row_content(vec2(160.0, 22.0), |p, r, s| paint_line_style(p, r, *s, ink, &LineWeights::UNDERLINE))
//!     .text_shown(SelectText::Never)
//!     .show(ui);
//! ```

use egui::{Color32, Painter, Rect, Shape, Stroke, pos2};

use crate::canvas::crisp;
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
        self.word().english
    }

    /// The keyed word of [`label`](Self::label), for `Words::get`.
    pub fn word(self) -> Word {
        LINE_STYLE_WORDS[self as usize]
    }
}

/// The names of [`LineStyle::label`], keyed `lineStyle.<name>`, in the order of the variants.
pub(crate) const LINE_STYLE_WORDS: &[Word] = &[
    Word::keyed("lineStyle.single", "Single"),
    Word::keyed("lineStyle.double", "Double"),
    Word::keyed("lineStyle.thick", "Thick"),
    Word::keyed("lineStyle.dotted", "Dotted"),
    Word::keyed("lineStyle.dashed", "Dashed"),
    Word::keyed("lineStyle.dotDash", "Dot-dash"),
    Word::keyed("lineStyle.dotDotDash", "Dot-dot-dash"),
    Word::keyed("lineStyle.wave", "Wave"),
    Word::keyed("lineStyle.thickThin", "Thick - Thin"),
    Word::keyed("lineStyle.thinThick", "Thin - Thick"),
    Word::keyed("lineStyle.thinThin", "Thin - Thin"),
    Word::keyed("lineStyle.thickThick", "Thick - Thick"),
];

/// The weights of [`paint_line_style`]'s samples, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineWeights {
    /// The solid line.
    pub line: f32,
    /// A dash's thickness.
    pub dash: f32,
    /// A dash's length.
    pub dash_length: f32,
    /// The gap after a dash.
    pub dash_gap: f32,
    /// A dot's radius.
    pub dot_radius: f32,
    /// The gap after a dot.
    pub dot_gap: f32,
}

impl LineWeights {
    /// WordCraft's underline samples: 1 point lines, dashes 6 long, dots of radius 0.75.
    // wordcraft: crates/ui-egui/src/ribbon/underline.rs:55 paint_underline
    pub const UNDERLINE: Self = Self { line: 1.0, dash: 1.0, dash_length: 6.0, dash_gap: 4.0, dot_radius: 0.75, dot_gap: 2.0 };
    /// DesignCraft's Stroke Type samples: a 5 point solid line, 4 point dashes 6 long every 9,
    /// dots of radius 2 every 6.
    // designcraft: crates/ui-egui/src/panels/properties/object.rs:258 paint_stroke_type
    pub const STROKE: Self = Self { line: 5.0, dash: 4.0, dash_length: 6.0, dash_gap: 3.0, dot_radius: 2.0, dot_gap: 2.0 };
}

/// A sample of `style` across `rect` at its middle, in `color`, with the lines and patterns of
/// `weights`: dashes and dots as the weights give them, a sine wave of amplitude 1.6 and period 6
/// for `Wave`; the two-line styles 6 apart (DesignCraft's `paint_stroke_type`, 3 above and below
/// the middle), `Double` 3 apart and `Thick` 2.5 points. Horizontal lines cover whole device
/// pixels ([`crisp`]), so thin lines stay sharp at any scale.
// wordcraft: crates/ui-egui/src/ribbon/underline.rs:55 paint_underline
// designcraft: crates/ui-egui/src/panels/properties/object.rs:258 paint_stroke_type
pub fn paint_line_style(painter: &Painter, rect: Rect, style: LineStyle, color: Color32, weights: &LineWeights) {
    let ppp = painter.pixels_per_point();
    let y = rect.center().y;
    let (x0, x1) = (rect.left(), rect.right());
    let line = |y: f32, width: f32| crisp(painter, pos2(x0, y), pos2(x1, y), Stroke::new(width, color));
    // Repeats (length, gap) runs along the line; a length of 0 is a dot.
    let pattern = |runs: &[(f32, f32)]| {
        let dot_y = (y * ppp).round() / ppp;
        let radius = weights.dot_radius;
        let mut x = x0;
        loop {
            for &(length, gap) in runs {
                if x >= x1 {
                    return;
                }
                if length == 0.0 {
                    painter.circle_filled(pos2(x + radius, dot_y), radius, color);
                    x += 2.0 * radius + gap;
                } else {
                    crisp(painter, pos2(x, y), pos2((x + length).min(x1), y), Stroke::new(weights.dash, color));
                    x += length + gap;
                }
            }
        }
    };
    let pair = |upper: f32, lower: f32| {
        line(y - 3.0, upper);
        line(y + 3.0, lower);
    };
    let dash = weights.dash_length;
    match style {
        LineStyle::Solid => line(y, weights.line),
        LineStyle::Double => {
            line(y - 1.5, weights.line);
            line(y + 1.5, weights.line);
        }
        LineStyle::Thick => line(y, 2.5),
        LineStyle::Dotted => pattern(&[(0.0, weights.dot_gap)]),
        LineStyle::Dashed => pattern(&[(dash, weights.dash_gap)]),
        LineStyle::DotDash => pattern(&[(dash, 3.0), (0.0, 3.0)]),
        LineStyle::DotDotDash => pattern(&[(dash, 3.0), (0.0, 2.5), (0.0, 3.0)]),
        LineStyle::Wave => {
            let wave_y = (y * ppp).round() / ppp;
            let points = (0..=((x1 - x0).max(0.0) as usize))
                .map(|i| pos2(x0 + i as f32, wave_y + 1.6 * (i as f32 * std::f32::consts::TAU / 6.0).sin()))
                .collect();
            painter.add(Shape::line(points, Stroke::new(weights.line, color)));
        }
        LineStyle::ThickThin => pair(3.0, 1.0),
        LineStyle::ThinThick => pair(1.0, 3.0),
        LineStyle::ThinThin => pair(1.0, 1.0),
        LineStyle::ThickThick => pair(2.5, 2.5),
    }
}
