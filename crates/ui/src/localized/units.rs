//! Length units shared by every field, ruler and dialog that shows a length: [`LengthUnit`]
//! (Photoshop's Preferences › Units & Rulers list), a [`Length`] in one, and the [`UnitContext`]
//! that converts between units (the document's resolution, the point size, what 100 % is).
//!
//! ```
//! use craft_ui::localized::units::{Length, LengthUnit, UnitContext};
//! let at_300 = UnitContext { pixels_per_inch: 300.0, ..UnitContext::default() };
//! let inch = Length { value: 1.0, unit: LengthUnit::Inches };
//! # let _ = (inch, at_300);
//! ```

/// Centimetres per inch.
pub(crate) const CM_PER_INCH: f64 = 2.54;
/// Points per inch: PostScript points, which a pica is 12 of.
pub(crate) const POINTS_PER_INCH: f64 = 72.0;

/// A length unit of a size field. Pixels and percent are the pixel units; the rest are physical
/// and convert through the resolution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum LengthUnit {
    Pixels,
    Percent,
    Inches,
    Centimetres,
    Millimetres,
    Points,
    Picas,
}

impl LengthUnit {
    /// Every unit, in menu order.
    pub const ALL: [LengthUnit; 7] = [
        LengthUnit::Pixels,
        LengthUnit::Percent,
        LengthUnit::Inches,
        LengthUnit::Centimetres,
        LengthUnit::Millimetres,
        LengthUnit::Points,
        LengthUnit::Picas,
    ];

    pub fn is_pixel_unit(self) -> bool {
        matches!(self, LengthUnit::Pixels | LengthUnit::Percent)
    }

    /// Decimal places a field in this unit shows.
    pub fn decimals(self) -> usize {
        match self {
            LengthUnit::Pixels => 0,
            LengthUnit::Points | LengthUnit::Percent => 1,
            LengthUnit::Millimetres | LengthUnit::Centimetres | LengthUnit::Picas => 2,
            LengthUnit::Inches => 3,
        }
    }

    /// The unit a typed suffix names (`px`, `in` or `"`, `cm`, `mm`, `pt`, `pc` or `pica`, `%` and
    /// their long forms), in any case.
    pub fn from_suffix(suffix: &str) -> Option<LengthUnit> {
        Some(match suffix.to_ascii_lowercase().as_str() {
            "px" | "pixel" | "pixels" => LengthUnit::Pixels,
            "in" | "inch" | "inches" | "\"" => LengthUnit::Inches,
            "cm" | "centimeter" | "centimeters" | "centimetre" | "centimetres" => LengthUnit::Centimetres,
            "mm" | "millimeter" | "millimeters" | "millimetre" | "millimetres" => LengthUnit::Millimetres,
            "pt" | "point" | "points" => LengthUnit::Points,
            "pc" | "pica" | "picas" => LengthUnit::Picas,
            "%" | "percent" => LengthUnit::Percent,
            _ => return None,
        })
    }

    /// Text typed in a field of this unit, with an optional unit suffix of its own (`2in` in a
    /// pixel field), as the value in this unit.
    pub(crate) fn parse_typed(self, text: &str, original_pixels: f64, pixels_per_inch: f64, points_per_inch: f64) -> Option<f64> {
        let text = text.trim();
        let end = text.find(|c: char| !(c.is_ascii_digit() || matches!(c, '.' | ',' | '-' | '+'))).unwrap_or(text.len());
        let (number, suffix) = text.split_at(end);
        let value: f64 = number.trim().replace(',', ".").parse().ok().filter(|v: &f64| v.is_finite())?;
        let suffix = suffix.trim();
        if suffix.is_empty() {
            return Some(value);
        }
        let typed = LengthUnit::from_suffix(suffix)?;
        let pixels = typed.to_pixels(value, original_pixels, pixels_per_inch, points_per_inch);
        Some(self.from_pixels(pixels, original_pixels, pixels_per_inch, points_per_inch))
    }

    /// `pixels` in this unit, given the original pixel size (for percent), the resolution and
    /// the points in an inch (72 for PostScript points).
    // pc: crates/ui-egui/src/dialogs/sizing.rs:120 `to_unit`.
    pub fn from_pixels(self, pixels: f64, original_pixels: f64, pixels_per_inch: f64, points_per_inch: f64) -> f64 {
        let resolution = pixels_per_inch.max(1.0);
        match self {
            LengthUnit::Pixels => pixels,
            LengthUnit::Percent => pixels / original_pixels.max(1.0) * 100.0,
            LengthUnit::Inches => pixels / resolution,
            LengthUnit::Centimetres => pixels / resolution * CM_PER_INCH,
            LengthUnit::Millimetres => pixels / resolution * CM_PER_INCH * 10.0,
            LengthUnit::Points => pixels / resolution * points_per_inch,
            LengthUnit::Picas => pixels / resolution * points_per_inch / 12.0,
        }
    }

    /// A value in this unit as pixels.
    // pc: crates/ui-egui/src/dialogs/sizing.rs:132 `from_unit`.
    pub fn to_pixels(self, value: f64, original_pixels: f64, pixels_per_inch: f64, points_per_inch: f64) -> f64 {
        let resolution = pixels_per_inch.max(1.0);
        match self {
            LengthUnit::Pixels => value,
            LengthUnit::Percent => value * original_pixels / 100.0,
            LengthUnit::Inches => value * resolution,
            LengthUnit::Centimetres => value / CM_PER_INCH * resolution,
            LengthUnit::Millimetres => value / (CM_PER_INCH * 10.0) * resolution,
            LengthUnit::Points => value / points_per_inch * resolution,
            LengthUnit::Picas => value / points_per_inch * 12.0 * resolution,
        }
    }
}

impl LengthUnit {
    /// The suffix a field shows after a number in this unit (`px`, `%`, `in`, `cm`, `mm`, `pt`,
    /// `pica`), as Photoshop's fields.
    pub fn suffix(self) -> &'static str {
        match self {
            LengthUnit::Pixels => "px",
            LengthUnit::Percent => "%",
            LengthUnit::Inches => "in",
            LengthUnit::Centimetres => "cm",
            LengthUnit::Millimetres => "mm",
            LengthUnit::Points => "pt",
            LengthUnit::Picas => "pica",
        }
    }

    /// The unit's name in the user's language, for unit menus.
    pub fn label(self, labels: &LengthUnitLabels) -> &str {
        match self {
            LengthUnit::Pixels => &labels.pixels,
            LengthUnit::Percent => &labels.percent,
            LengthUnit::Inches => &labels.inches,
            LengthUnit::Centimetres => &labels.centimetres,
            LengthUnit::Millimetres => &labels.millimetres,
            LengthUnit::Points => &labels.points,
            LengthUnit::Picas => &labels.picas,
        }
    }
}

/// What a typed or shown length means in a document: the resolution, the point size and, for a
/// field that is both a length and a percentage, the length 100 % stands for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UnitContext {
    /// The document's resolution. 72 by default.
    pub pixels_per_inch: f64,
    /// Points per inch: 72 (PostScript, the default) or 72.27 (traditional), Preferences ›
    /// Units & Rulers › Point/Pica Size.
    pub points_per_inch: f64,
    /// The pixels 100 % stands for (Free Transform's W and H, Image Size's original size), or
    /// `None` where lengths and percentages do not convert.
    pub extent: Option<f64>,
}

impl Default for UnitContext {
    fn default() -> Self {
        Self { pixels_per_inch: 72.0, points_per_inch: POINTS_PER_INCH, extent: None }
    }
}

impl UnitContext {
    /// `value` in `unit` as pixels, or `None` for a percentage without an
    /// [`extent`](Self::extent) or a value that is not a finite number.
    ///
    /// A resolution or point size that is not a positive number reads as 72, and 100 % is the
    /// magnitude of the extent.
    // pc: crates/ui-egui/src/widgets/numeric_expression.rs:154 `Parser::unit`.
    pub fn to_pixels(&self, value: f64, unit: LengthUnit) -> Option<f64> {
        let (extent, pixels_per_inch, points_per_inch) = self.basis(unit)?;
        Some(unit.to_pixels(value, extent, pixels_per_inch, points_per_inch)).filter(|v| v.is_finite())
    }

    /// `pixels` in `unit`, or `None` as for [`to_pixels`](Self::to_pixels).
    pub fn from_pixels(&self, pixels: f64, unit: LengthUnit) -> Option<f64> {
        let (extent, pixels_per_inch, points_per_inch) = self.basis(unit)?;
        Some(unit.from_pixels(pixels, extent, pixels_per_inch, points_per_inch)).filter(|v| v.is_finite())
    }

    /// The extent, resolution and point size a conversion in `unit` uses, or `None` for a
    /// percentage without a usable extent.
    fn basis(&self, unit: LengthUnit) -> Option<(f64, f64, f64)> {
        let positive = |v: f64| if v.is_finite() && v > 0.0 { v } else { 72.0 };
        let extent = match (unit, self.extent) {
            (LengthUnit::Percent, extent) => extent.map(f64::abs).filter(|e| e.is_finite() && *e > 1e-9)?,
            (_, extent) => extent.unwrap_or(1.0),
        };
        Some((extent, positive(self.pixels_per_inch), positive(self.points_per_inch)))
    }

    /// `length` in `unit`.
    pub fn convert(&self, length: Length, unit: LengthUnit) -> Option<f64> {
        self.from_pixels(self.to_pixels(length.value, length.unit)?, unit)
    }
}

/// A number in a length unit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Length {
    pub value: f64,
    pub unit: LengthUnit,
}

impl Length {
    /// Text such as `2in`, `5 mm`, `50%` or a bare `12` (in `default_unit`), with the region's
    /// decimal mark already normalised to `.`. `None` when the text is not one number with an
    /// optional unit (arithmetic is [`controls::expr`](crate::controls::expr)'s).
    // pc: crates/ui-egui/src/widgets/units.rs:77 `unit_of_suffix`, :110 `parse_num`.
    pub fn parse(text: &str, default_unit: LengthUnit) -> Option<Length> {
        let text = text.trim();
        let digits = text.strip_prefix(['-', '+', '\u{2212}']).unwrap_or(text);
        let end = digits.find(|c: char| !(c.is_ascii_digit() || c == '.')).unwrap_or(digits.len());
        let (magnitude, suffix) = digits.split_at(end);
        let sign = text.len() - digits.len();
        let number = text.get(..sign + magnitude.len())?;
        let value: f64 = number.replace('\u{2212}', "-").parse().ok().filter(|v: &f64| v.is_finite())?;
        let suffix = suffix.trim();
        let unit = if suffix.is_empty() { default_unit } else { LengthUnit::from_suffix(suffix)? };
        Some(Length { value, unit })
    }
}

crate::words! {
    /// The names of the length units, for unit menus (rulers, fields with a unit menu).
    #[non_exhaustive]
    pub struct LengthUnitLabels {
        pixels: "units.pixels" = "Pixels";
        percent: "units.percent" = "Percent";
        inches: "units.inches" = "Inches";
        centimetres: "units.centimetres" = "Centimeters";
        millimetres: "units.millimetres" = "Millimeters";
        points: "units.points" = "Points";
        picas: "units.picas" = "Picas";
    }
}
