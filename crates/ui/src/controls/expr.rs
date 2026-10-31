//! Arithmetic typed into number fields, as in PhotoCraft (and Photoshop's fields): `+ - * / **`,
//! parentheses, `pi` and `tau`, and a unit after a number (`2in` in a pixel field is the pixels of
//! an inch at the document's resolution). Bounded: at most 1024 characters and 32 levels of
//! nesting, and only finite results; no names, no scripting.
//!
//! [`NumberField::expressions`](super::NumberField::expressions) reads typed text with
//! [`parse`] and [`relative`]; a length field reads it with [`parse_with_units`] through
//! [`NumberField::or_parse`](super::NumberField::or_parse). The text reaches these functions with
//! the region's decimal mark already `.` and its digits ASCII; `−` (U+2212) is a minus.
//!
//! ```
//! use craft_ui::controls::expr;
//! # fn demo() {
//! let half = expr::parse("(1920 - 40) / 2");
//! let doubled = expr::relative("*2", 150.0);
//! # let _ = (half, doubled);
//! # }
//! ```

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use crate::localized::units::{LengthUnit, UnitContext};

/// What a unit typed after a number means in a field.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExprUnits<'a> {
    /// The field's own unit; `None` for a field that is not a length or a percentage.
    pub unit: Option<LengthUnit>,
    /// The suffix the field shows; typed after a number it means the field's unit (`45°`).
    pub suffix: &'a str,
    /// Resolution, point size and what 100 % is.
    pub context: UnitContext,
}

/// The value of `text` as arithmetic, or `None` when it is not a complete expression with a finite
/// result. Precedence: unary `+ -` and `**` (right-associative) bind tighter than `* /`, which bind
/// tighter than `+ -`.
// pc: crates/ui-egui/src/widgets/numeric_expression.rs:6 `parse`, :72 `Parser::expression`.
pub fn parse(text: &str) -> Option<f64> {
    todo!()
}

/// [`parse`] where a number may carry a unit (`2in`, `5 mm`, `50%`, `12pica`; longest name
/// first, so `pica` wins over `pc`), converted into the field's unit. A unit that cannot convert
/// (a length in a percent field without an extent, any unit in a unitless field) makes the whole
/// text invalid.
// pc: crates/ui-egui/src/widgets/numeric_expression.rs:44 `parse_with_units`, :154 `Parser::unit`.
pub fn parse_with_units(text: &str, units: &ExprUnits<'_>) -> Option<f64> {
    todo!()
}

/// A relative entry applied to `current`: `*2` doubles it, `/4` quarters it. Only `*` and `/`
/// lead a relative entry: a leading `+` or `-` is the number's sign (typing `-10` sets −10), as in
/// PhotoCraft. `None` for anything else, so the field falls back to [`parse`].
// new: relative entry (Figma, Affinity); PhotoCraft has none.
pub fn relative(text: &str, current: f64) -> Option<f64> {
    todo!()
}
