//! Arithmetic typed into number fields, as in PhotoCraft (and Photoshop's fields): `+ - * / %`
//! (remainder), `^` and `**` (power), parentheses, `pi` and `tau`, and a unit after a number
//! (`2in` in a pixel field is the pixels of an inch at the document's resolution). Bounded: at most 1024 characters and 32 levels of
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

/// The longest text read, in bytes.
const MAX_LEN: usize = 1024;
/// The deepest nesting of parentheses, signs and operands.
const MAX_DEPTH: u8 = 32;

/// Unit names a number may carry, longest first so `pica` wins over `pc`.
const UNIT_NAMES: [(&str, LengthUnit); 8] = [
    ("pica", LengthUnit::Picas),
    ("px", LengthUnit::Pixels),
    ("pt", LengthUnit::Points),
    ("pc", LengthUnit::Picas),
    ("in", LengthUnit::Inches),
    ("cm", LengthUnit::Centimetres),
    ("mm", LengthUnit::Millimetres),
    ("%", LengthUnit::Percent),
];

/// The value of `text` as arithmetic, or `None` when it is not a complete expression with a finite
/// result. Precedence: unary `+ -` and `**` (right-associative) bind tighter than `* /`, which bind
/// tighter than `+ -`.
// pc: crates/ui-egui/src/widgets/numeric_expression.rs:6 `parse`, :72 `Parser::expression`.
pub fn parse(text: &str) -> Option<f64> {
    evaluate(text, None)
}

/// [`parse`] where a number may carry a unit (`2in`, `5 mm`, `50%`, `12pica`; longest name
/// first, so `pica` wins over `pc`), converted into the field's unit. A unit that cannot convert
/// (a length in a percent field without an extent, any unit in a unitless field) makes the whole
/// text invalid.
// pc: crates/ui-egui/src/widgets/numeric_expression.rs:44 `parse_with_units`, :154 `Parser::unit`.
pub fn parse_with_units(text: &str, units: &ExprUnits<'_>) -> Option<f64> {
    evaluate(text, Some(units))
}

/// A relative entry applied to `current`: `*2` doubles it, `/4` quarters it. Only `*` and `/`
/// lead a relative entry: a leading `+` or `-` is the number's sign (typing `-10` sets −10), as in
/// PhotoCraft. `None` for anything else, so the field falls back to [`parse`].
// new: relative entry (Figma, Affinity); PhotoCraft has none.
pub fn relative(text: &str, current: f64) -> Option<f64> {
    let text = text.trim_start();
    let operand = parse(text.get(1..)?)?;
    let value = match text.chars().next()? {
        '*' => current * operand,
        '/' if operand != 0.0 => current / operand,
        _ => return None,
    };
    value.is_finite().then_some(value)
}

fn evaluate(text: &str, units: Option<&ExprUnits<'_>>) -> Option<f64> {
    if text.len() > MAX_LEN {
        return None;
    }
    let normalized = text.replace('\u{2212}', "-");
    let mut parser = Parser { rest: &normalized, units };
    let value = parser.expression(0, 0)?;
    parser.space();
    (parser.rest.is_empty() && value.is_finite()).then_some(value)
}

struct Parser<'a> {
    rest: &'a str,
    units: Option<&'a ExprUnits<'a>>,
}

impl Parser<'_> {
    fn space(&mut self) {
        self.rest = self.rest.trim_start();
    }

    fn take(&mut self, token: &str) -> bool {
        self.space();
        if let Some(rest) = self.rest.strip_prefix(token) {
            self.rest = rest;
            true
        } else {
            false
        }
    }

    fn expression(&mut self, minimum: u8, depth: u8) -> Option<f64> {
        if depth >= MAX_DEPTH {
            return None;
        }
        self.space();
        let mut left = if self.take("+") {
            self.expression(3, depth + 1)?
        } else if self.take("-") {
            -self.expression(3, depth + 1)?
        } else if self.take("(") {
            let value = self.expression(0, depth + 1)?;
            if !self.take(")") {
                return None;
            }
            value
        } else if self.take("pi") {
            std::f64::consts::PI
        } else if self.take("tau") {
            std::f64::consts::TAU
        } else {
            self.number()?
        };
        loop {
            self.space();
            let (token, precedence, right_minimum) = if self.rest.starts_with("**") {
                ("**", 3, 3)
            } else {
                match self.rest.chars().next() {
                    Some('+') => ("+", 1, 2),
                    Some('-') => ("-", 1, 2),
                    Some('*') => ("*", 2, 3),
                    Some('/') => ("/", 2, 3),
                    Some('%') => ("%", 2, 3),
                    Some('^') => ("^", 3, 3),
                    _ => break,
                }
            };
            if precedence < minimum {
                break;
            }
            if !self.take(token) {
                return None;
            }
            let right = self.expression(right_minimum, depth + 1)?;
            left = match token {
                "+" => left + right,
                "-" => left - right,
                "*" => left * right,
                "/" if right != 0.0 => left / right,
                "%" if right != 0.0 => left % right,
                "^" | "**" => left.powf(right),
                _ => return None,
            };
            if !left.is_finite() {
                return None;
            }
        }
        left.is_finite().then_some(left)
    }

    fn number(&mut self) -> Option<f64> {
        let mut length = 0;
        let mut exponent = false;
        for (offset, ch) in self.rest.char_indices() {
            let accepted = ch.is_ascii_digit()
                || ch == '.'
                || (!exponent && matches!(ch, 'e' | 'E'))
                || (matches!(ch, '+' | '-') && self.rest.get(..offset).is_some_and(|s| s.ends_with(['e', 'E'])));
            if !accepted {
                break;
            }
            exponent |= matches!(ch, 'e' | 'E');
            length = offset + ch.len_utf8();
        }
        let value = self.rest.get(..length)?.parse::<f64>().ok()?;
        self.rest = self.rest.get(length..)?;
        let value = match self.units {
            Some(units) => self.unit(value, units)?,
            None => value,
        };
        value.is_finite().then_some(value)
    }

    /// The unit after a number, if any, converting `value` into the field's unit.
    fn unit(&mut self, value: f64, units: &ExprUnits<'_>) -> Option<f64> {
        let rest = self.rest.trim_start();
        let typed = UNIT_NAMES.iter().find(|(name, _)| rest.starts_with(name)).filter(|(name, _)| {
            // `%` followed by an operand is the remainder operator (`10%3`), not percent.
            *name != "%"
                || rest.get(1..).map(str::trim_start).and_then(|r| r.chars().next()).is_none_or(|c| matches!(c, ')' | '+' | '-' | '*' | '/' | '^'))
        });
        let Some(&(name, typed)) = typed else {
            // The field's own suffix (`45°`), when it is no operator or number of its own.
            let own = units.suffix;
            let operand = |c: char| c.is_ascii_digit() || matches!(c, '.' | '(' | ')' | '+' | '-' | '*' | '/' | '%' | '^');
            if !own.is_empty() && !own.starts_with(operand) && rest.starts_with(own) {
                self.rest = rest.get(own.len()..)?;
            }
            return Some(value);
        };
        self.rest = rest.get(name.len()..)?;
        let field = units.unit?;
        if field == typed {
            return Some(value);
        }
        // Lengths convert through pixels; a percentage needs the context's extent.
        units.context.from_pixels(units.context.to_pixels(value, typed)?, field)
    }
}
