//! `quantity::Reader`: the one number and unit parser (18.3): locale digits, unit spellings, expressions and
//! relative input.

use crate::codec::NotAValue;

/// First code point of each supported digit block; each block has ten consecutive digits.
const DIGIT_BLOCKS: [u32; 5] = [0x30, 0x660, 0x6F0, 0x966, 0xFF10];

/// The value of a decimal digit of any supported script.
fn digit(c: char) -> Option<u32> {
    DIGIT_BLOCKS.iter().find_map(|&zero| (zero..zero + 10).contains(&(c as u32)).then(|| c as u32 - zero))
}

/// A unit spelling and the factor that turns its quantity into the base unit.
#[derive(Clone, Debug, PartialEq)]
struct Unit {
    spelling: String,
    factor: f64,
}

/// Reads numbers, expressions and unit quantities written in one locale.
///
/// Grammar: `expr = term (('+' | '-') term)*`, `term = factor (('*' | '/') factor)*`,
/// `factor = ('-' | '+')? (number unit? | '(' expr ')')`. Text that starts with `+`, `*` or `/` is relative: it
/// applies to the start value (`+5` adds 5, `*2` doubles). A leading `-` is a negative number.
#[derive(Clone, Debug, PartialEq)]
pub struct Reader {
    decimal: char,
    group: Option<char>,
    units: Vec<Unit>,
}

impl Default for Reader {
    fn default() -> Self {
        Reader { decimal: '.', group: None, units: Vec::new() }
    }
}

impl Reader {
    /// A reader for plain numbers written with `.` as the decimal separator.
    pub fn new() -> Self {
        Self::default()
    }

    /// The decimal separator.
    #[must_use]
    pub fn decimal(mut self, decimal: char) -> Self {
        self.decimal = decimal;
        self
    }

    /// The digit-group separator, which the reader skips between digits.
    #[must_use]
    pub fn group(mut self, group: char) -> Self {
        self.group = Some(group);
        self
    }

    /// A unit spelling (case-insensitive) and its factor to the base unit.
    #[must_use]
    pub fn unit(mut self, spelling: &str, factor: f64) -> Self {
        self.units.push(Unit { spelling: spelling.to_lowercase(), factor });
        self
    }

    /// The decimal separator the reader reads and writes.
    pub fn decimal_separator(&self) -> char {
        self.decimal
    }

    /// The value of `text`. `start` is the value relative input applies to.
    pub fn read(&self, text: &str, start: Option<f64>) -> Result<f64, NotAValue> {
        let chars: Vec<char> = text.chars().filter(|c| !c.is_whitespace()).collect();
        let mut parse = Parse { reader: self, chars: &chars, at: 0 };
        let value = match (chars.first(), start) {
            (Some('+' | '*' | '/'), Some(start)) => parse.expr_from(start)?,
            _ => parse.expr()?,
        };
        (parse.at == chars.len() && value.is_finite()).then_some(value).ok_or(NotAValue)
    }
}

struct Parse<'a> {
    reader: &'a Reader,
    chars: &'a [char],
    at: usize,
}

impl Parse<'_> {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.at).copied()
    }

    fn expr(&mut self) -> Result<f64, NotAValue> {
        let first = self.term()?;
        self.sum(first)
    }

    /// An expression whose left operand is `start`, written without it.
    fn expr_from(&mut self, start: f64) -> Result<f64, NotAValue> {
        let term = self.product(start)?;
        self.sum(term)
    }

    fn sum(&mut self, mut left: f64) -> Result<f64, NotAValue> {
        while let Some(op @ ('+' | '-')) = self.peek() {
            self.at += 1;
            let right = self.term()?;
            left = if op == '+' { left + right } else { left - right };
        }
        Ok(left)
    }

    fn term(&mut self) -> Result<f64, NotAValue> {
        let first = self.factor()?;
        self.product(first)
    }

    fn product(&mut self, mut left: f64) -> Result<f64, NotAValue> {
        // A bare relative `+5` has no product to run; the sum loop takes it.
        while let Some(op @ ('*' | '/')) = self.peek() {
            self.at += 1;
            let right = self.factor()?;
            left = if op == '*' { left * right } else { left / right };
        }
        Ok(left)
    }

    fn factor(&mut self) -> Result<f64, NotAValue> {
        match self.peek() {
            Some('-') => {
                self.at += 1;
                self.factor().map(|value| -value)
            }
            Some('+') => {
                self.at += 1;
                self.factor()
            }
            Some('(') => {
                self.at += 1;
                let value = self.expr()?;
                (self.peek() == Some(')')).then(|| self.at += 1).ok_or(NotAValue)?;
                Ok(value)
            }
            _ => self.quantity(),
        }
    }

    fn quantity(&mut self) -> Result<f64, NotAValue> {
        let number = self.number()?;
        let rest: String = self.chars[self.at..].iter().take_while(|c| c.is_alphabetic()).collect();
        if rest.is_empty() {
            return Ok(number);
        }
        let lowered = rest.to_lowercase();
        let unit = self.reader.units.iter().find(|unit| unit.spelling == lowered).ok_or(NotAValue)?;
        self.at += rest.chars().count();
        Ok(number * unit.factor)
    }

    fn number(&mut self) -> Result<f64, NotAValue> {
        let mut text = String::new();
        let mut seen_decimal = false;
        while let Some(c) = self.peek() {
            if let Some(d) = digit(c) {
                text.push(char::from_digit(d, 10).ok_or(NotAValue)?);
            } else if c == self.reader.decimal && !seen_decimal {
                seen_decimal = true;
                text.push('.');
            } else if Some(c) == self.reader.group && !seen_decimal && !text.is_empty() {
            } else {
                break;
            }
            self.at += 1;
        }
        if text.is_empty() || text == "." {
            return Err(NotAValue);
        }
        text.parse().map_err(|_| NotAValue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expressions_units_and_relative_input_read_through_one_grammar() {
        let reader = Reader::new().decimal(',').group('.').unit("in", 72.0).unit("pt", 1.0);
        assert_eq!(reader.read("2+3*4", None), Ok(14.0));
        assert_eq!(reader.read("(2+3)*4", None), Ok(20.0));
        assert_eq!(reader.read("1.234,5", None), Ok(1234.5), "group separator skipped, comma is the decimal");
        assert_eq!(reader.read("1in + 6pt", None), Ok(78.0));
        assert_eq!(reader.read("-3", Some(10.0)), Ok(-3.0), "a leading minus is a negative number");
        assert_eq!(reader.read("+5", Some(10.0)), Ok(15.0));
        assert_eq!(reader.read("*2", Some(10.0)), Ok(20.0));
        assert_eq!(reader.read("٣٤", None), Ok(34.0), "Arabic-Indic digits");
        for bad in ["", "abc", "1 +", "2px", "1/0", "(1"] {
            assert_eq!(reader.read(bad, Some(1.0)), Err(NotAValue), "{bad:?} is not a value");
        }
    }
}
