//! The effect tables: which fields an effect page shows, in which order, with which ranges, and
//! the effect's factory defaults.

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land (task AD-7)")]

use std::borrow::Cow;

use super::model::{EffectKind, StyleParams};

/// A slider field's unit, shown after its number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FieldUnit {
    /// "%".
    Percent,
    /// "px".
    Pixels,
    /// "°".
    Degrees,
}

/// One entry of a choice field: the value key stored in the parameters and the English label.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ChoiceOption {
    /// Stored as `StyleValue::Text` (`"softer"`).
    pub value: Cow<'static, str>,
    /// The English label; the dialog shows the `layerStyle.choice.*` word for the built-in
    /// tables.
    pub label: Cow<'static, str>,
}

/// How a field is edited.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum FieldKind {
    /// A slider row over `min..=max` in `unit`; values are whole numbers.
    Slider {
        min: f64,
        max: f64,
        unit: FieldUnit,
    },
    /// A colour swatch; a click asks the app for a colour (`ColorRequest`).
    Color,
    /// A blend-mode select over the host's blend modes.
    Blend,
    /// A select over `options`; the first is the fallback for a missing value.
    Choice(Cow<'static, [ChoiceOption]>),
    /// Gradient Overlay's Method: Smooth, Perceptual, Linear, Classic, and Custom first while the
    /// stored value names none of them (an imported curve kept until the user picks).
    GradientMethod,
    Check,
    /// An angle dial and field, −180..=180°. With the effect's `useGlobalLight` on, it edits the
    /// shared light angle.
    Angle,
    /// A pattern from the host's pattern list; the first entry when the value is empty.
    Pattern,
    /// A contour thumbnail with the preset picker and the Contour Editor; `anti_alias` is the
    /// key of the effect's Anti-aliased check shown beside it, when the effect has one.
    Contour {
        anti_alias: Option<Cow<'static, str>>,
    },
}

/// One row of an effect page.
#[derive(Clone, Debug, PartialEq)]
pub struct EffectField {
    /// The parameter key (`"distance"`).
    pub key: Cow<'static, str>,
    /// The English label ("Distance"); empty for a row that continues the one above (Bevel &
    /// Emboss's contour under its Contour check). The dialog shows the `layerStyle.field.*` word
    /// for the built-in tables.
    pub label: Cow<'static, str>,
    pub kind: FieldKind,
}

/// An effect's page: its fields in order and its factory defaults. An app with effects of its
/// own builds its specs by hand; Photoshop's ten come from [`EffectSpec::photoshop`].
#[derive(Clone, Debug, PartialEq)]
pub struct EffectSpec {
    pub kind: EffectKind,
    /// The English name ("Drop Shadow"); the dialog shows the `layerStyle.effect.*` word for the
    /// built-in kinds.
    pub label: Cow<'static, str>,
    pub fields: Vec<EffectField>,
    /// The values a new instance starts from.
    pub defaults: StyleParams,
}

impl EffectSpec {
    /// Photoshop's table and factory defaults for `kind`, as PhotoCraft has them. Keys and labels,
    /// ranges in the field's unit (Slider min..max unit):
    ///
    /// - Drop Shadow: blend Blend Mode; color Color; opacity Opacity 0..100 %; angle Angle;
    ///   useGlobalLight Use Global Light; distance Distance 0..300 px; spread Spread 0..100 %;
    ///   size Size 0..250 px; contour Contour (anti-alias key `antiAlias`); noise Noise 0..100 %;
    ///   knocksOut Layer Knocks Out Drop Shadow. Defaults: Multiply, #000000, 75, 120, true, 5, 0,
    ///   5, Linear, antiAlias false, 0, true.
    /// - Inner Shadow: as Drop Shadow with choke Choke 0..100 % in place of Spread and no
    ///   knocksOut. Defaults: Multiply, #000000, 75, 120, true, 5, choke 0, 5, Linear, false, 0.
    /// - Outer Glow (Technique before Spread and Range after Contour, Photoshop's order, #6698):
    ///   blend; opacity 0..100 %; color; technique Technique (softer Softer, precise Precise);
    ///   spread 0..100 %; size 0..250 px; contour; range Range 1..100 %; noise 0..100 %.
    ///   Defaults: Screen, 75, #ffffbe, softer, 0, 5, Linear, false, range 50, 0.
    /// - Inner Glow: blend; opacity; color; technique; source Source (edge Edge, center Center);
    ///   choke 0..100 %; size 0..250 px; contour; range 1..100 %; noise 0..100 %. Defaults:
    ///   Screen, 75, #ffffbe, softer, edge, 0, 5, Linear, false, 50, 0.
    /// - Stroke: size Size 1..250 px; position Position (outside Outside, inside Inside, center
    ///   Center); blend; opacity 0..100 %; overprint Overprint; color. Defaults: 1, inside,
    ///   Normal, 100, false, #000000.
    /// - Color Overlay: blend; color; opacity 0..100 %. Defaults: Normal, #ff0000, 100.
    /// - Gradient Overlay: blend; opacity; from From (Color); to To (Color); reverse Reverse; style
    ///   Style (linear Linear, radial Radial, angle Angle, reflected Reflected, diamond Diamond);
    ///   angle; scale Scale 10..150 %; method Method (GradientMethod). Defaults: Normal, 100,
    ///   #000000, #ffffff, false, linear, 90, 100, custom.
    /// - Pattern Overlay: blend; opacity; pattern Pattern; angle; scale Scale 1..1000 %; link Link
    ///   with Layer. Defaults: Normal, 100, "", 0, 100, true.
    /// - Bevel & Emboss: style Style (inner Inner Bevel, outer Outer Bevel, emboss Emboss, pillow
    ///   Pillow Emboss); technique Technique (smooth Smooth, chiselHard Chisel Hard, chiselSoft
    ///   Chisel Soft); depth Depth 1..1000 %; direction Direction (up Up, down Down); size 0..250
    ///   px; soften Soften 0..16 px; angle; useGlobalLight; altitude Altitude 0..90 °;
    ///   glossContour Gloss Contour (anti-alias `glossAntiAlias`); highlightBlend Highlight Mode;
    ///   highlightColor Highlight Color; highlightOpacity Highlight Opacity 0..100 %; shadowBlend
    ///   Shadow Mode; shadowColor Shadow Color; shadowOpacity Shadow Opacity 0..100 %; contour
    ///   Contour (Check); contourName "" (Contour, anti-alias `contourAntiAlias`); contourRange
    ///   Range 1..100 %. Defaults: inner, smooth, 100, up, 5, 0, 120, true, 30, Linear, false,
    ///   Screen, #ffffff, 75, Multiply, #000000, 75, false, Linear, 50, false.
    /// - Satin: blend; color; opacity; angle; distance Distance 0..250 px; size 0..250 px;
    ///   contour; invert Invert. Defaults: Multiply, #000000, 50, 19, 11, 14, Linear, antiAlias
    ///   true, invert true.
    ///
    /// The defaults name blend modes by Photoshop's English names (`"Multiply"`, `"Screen"`,
    /// `"Normal"`), as PhotoCraft's engine stores them; a host whose `BlendChoice` keys differ
    /// maps them when it builds the draft. Contours default to
    /// [`Contour::linear`](super::Contour::linear).
    pub fn photoshop(kind: EffectKind) -> EffectSpec {
        // pc: crates/ui-egui/src/dialogs/layer_style.rs:90-216 `spec` (fields, ranges, units);
        // crates/engine/src/layer_style.rs:82-110 `effect_defaults` (defaults).
        todo!()
    }

    /// The field with `key`.
    pub fn field(&self, key: &str) -> Option<&EffectField> {
        self.fields.iter().find(|f| f.key == key)
    }
}
