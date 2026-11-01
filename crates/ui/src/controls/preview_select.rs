#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
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

use egui::{Color32, Painter, Rect};

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
        todo!()
    }
}

/// A sample of `style` across `rect` at its middle, in `color`, 1 point lines unless the style
/// says otherwise: dots of radius 0.75 every 1.5 + gap points, dashes from (length, gap) runs
/// (WordCraft's `pattern`), a sine wave 1.5 high for `Wave`; the two-line styles 6 apart
/// (DesignCraft's `paint_stroke_type`, 3 above and below the middle). The line sits on a pixel
/// centre so 1-point lines stay sharp.
// wordcraft: crates/ui-egui/src/ribbon/underline.rs:55 paint_underline
// designcraft: crates/ui-egui/src/panels/properties/object.rs:258 paint_stroke_type
pub fn paint_line_style(painter: &Painter, rect: Rect, style: LineStyle, color: Color32) {
    todo!()
}
