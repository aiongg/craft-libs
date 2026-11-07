//! `Codec`: how a text part turns a value into text and text into a value (18.2), with the library codecs of this
//! crate.

use crate::quantity::Reader;
use std::borrow::Cow;

/// The text is not a value of the codec's type.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NotAValue;

/// Where the caret goes when the part takes focus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Caret {
    /// The whole text is selected.
    All,
    /// The caret sits after the last character.
    End,
}

/// A value's text form.
pub trait Codec {
    /// The value the text stands for.
    type Value: Clone + PartialEq + Send + Sync + 'static;

    /// Every draft is a value (plain text): a draft applies as typed and never rejects.
    const DRAFTS_ARE_VALUES: bool = false;

    /// The text of `v`.
    fn format(&self, v: &Self::Value) -> String;

    /// The value of `text`; `start` is the value when editing began, for relative input.
    fn parse(&self, text: &str, start: &Self::Value) -> Result<Self::Value, NotAValue>;

    /// Whether `text` applies as soon as it is typed rather than on commit.
    fn applies_as_typed(&self, _text: &str) -> bool {
        false
    }

    /// Whether the codec takes `c` as typed.
    fn filter(&self, _c: char) -> bool {
        true
    }

    /// The text the field shows after committing `v`, which was parsed from `text`.
    fn normalise(&self, _text: &str, v: &Self::Value) -> String {
        self.format(v)
    }

    /// Where the caret goes when the part takes focus with this text.
    fn caret_on_focus(&self, _text: &str) -> Caret {
        Caret::All
    }

    /// The widest typical text, for the field's ideal width. Codecs with bounded text only; `PlainText` returns
    /// empty text, so a plain field never sizes from what the user typed.
    fn sample(&self, current: &Self::Value) -> Cow<'_, str> {
        self.format(current).into()
    }
}

impl<C: Codec + ?Sized> Codec for &C {
    type Value = C::Value;
    const DRAFTS_ARE_VALUES: bool = C::DRAFTS_ARE_VALUES;

    fn format(&self, v: &Self::Value) -> String {
        (**self).format(v)
    }

    fn parse(&self, text: &str, start: &Self::Value) -> Result<Self::Value, NotAValue> {
        (**self).parse(text, start)
    }

    fn applies_as_typed(&self, text: &str) -> bool {
        (**self).applies_as_typed(text)
    }

    fn filter(&self, c: char) -> bool {
        (**self).filter(c)
    }

    fn normalise(&self, text: &str, v: &Self::Value) -> String {
        (**self).normalise(text, v)
    }

    fn caret_on_focus(&self, text: &str) -> Caret {
        (**self).caret_on_focus(text)
    }

    fn sample(&self, current: &Self::Value) -> Cow<'_, str> {
        (**self).sample(current)
    }
}

/// Plain text: every draft is a value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlainText;

impl Codec for PlainText {
    type Value = String;
    const DRAFTS_ARE_VALUES: bool = true;

    fn format(&self, v: &String) -> String {
        v.clone()
    }

    fn parse(&self, text: &str, _start: &String) -> Result<String, NotAValue> {
        Ok(text.to_owned())
    }

    fn applies_as_typed(&self, _text: &str) -> bool {
        true
    }

    fn caret_on_focus(&self, _text: &str) -> Caret {
        Caret::End
    }

    fn sample(&self, _current: &String) -> Cow<'_, str> {
        Cow::Borrowed("")
    }
}

/// An optional number: empty text is `None`; anything else reads through a [`Reader`].
#[derive(Clone, Debug, PartialEq)]
pub struct NumberCodec {
    reader: Reader,
    decimals: usize,
}

/// The widest number a field is sized for.
const NUMBER_SAMPLE: &str = "0000.00";

impl NumberCodec {
    /// A codec reading with `reader` and writing at most `decimals` decimal places.
    pub fn new(reader: Reader, decimals: usize) -> Self {
        NumberCodec { reader, decimals }
    }

    /// The most decimal places the codec writes.
    pub fn decimals(&self) -> usize {
        self.decimals
    }

    /// The reader the codec parses with.
    pub fn reader(&self) -> &Reader {
        &self.reader
    }
}

impl Default for NumberCodec {
    fn default() -> Self {
        NumberCodec::new(Reader::new(), 3)
    }
}

impl Codec for NumberCodec {
    type Value = Option<f64>;

    fn format(&self, v: &Option<f64>) -> String {
        let Some(number) = v else { return String::new() };
        let fixed = format!("{number:.*}", self.decimals);
        let trimmed = if fixed.contains('.') { fixed.trim_end_matches('0').trim_end_matches('.') } else { &fixed };
        trimmed.replace('.', &self.reader.decimal_separator().to_string())
    }

    fn parse(&self, text: &str, start: &Option<f64>) -> Result<Option<f64>, NotAValue> {
        if text.trim().is_empty() {
            return Ok(None);
        }
        self.reader.read(text, *start).map(Some)
    }

    fn sample(&self, _current: &Option<f64>) -> Cow<'_, str> {
        Cow::Borrowed(NUMBER_SAMPLE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_codec_round_trips_and_rejects() {
        let codec = NumberCodec::new(Reader::new().decimal(','), 2);
        assert_eq!(codec.format(&Some(1.5)), "1,5");
        assert_eq!(codec.format(&Some(2.0)), "2");
        assert_eq!(codec.format(&None), "");
        assert_eq!(codec.parse("1,25", &None), Ok(Some(1.25)));
        assert_eq!(codec.parse("", &Some(3.0)), Ok(None));
        assert_eq!(codec.parse("x", &None), Err(NotAValue));
        assert_eq!(codec.parse("*2", &Some(4.0)), Ok(Some(8.0)));
    }
}
