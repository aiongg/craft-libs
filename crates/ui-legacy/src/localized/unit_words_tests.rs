//! Unit words.

use std::sync::Arc;

use super::{LocaleNumbers, UnitKind, Units, Words};

fn english() -> Units {
    Units::new(Arc::new(LocaleNumbers::from_tag("en")), Arc::new(Words::english()))
}

// Each kind writes its English template with default digits; a field unit reads the same word.
#[test]
fn english_units_write_their_templates() {
    let units = english();
    assert_eq!(units.pixels(36.6), "37 px");
    assert_eq!(units.percent(12.5), "12.5%");
    assert_eq!(units.percent(100.0), "100%");
    assert_eq!(units.degrees(45.0), "45°");
    assert_eq!(units.points(9.0), "9 pt");
    assert_eq!(units.times(1.5), "1.5×");
    let field = units.unit(UnitKind::Percent);
    assert_eq!((field.prefix(), field.suffix()), ("", "%"));
}

// A translated word and Arabic-Indic digits both reach the text; a catalog row without `{n}` falls back to the English template.
#[test]
fn a_translated_word_and_locale_digits_reach_the_text() {
    let words = Words::new(|word| match word.english {
        "{n} px" => Some("{n} بكسل".to_owned()),
        "{n}%" => Some("٪{n}".to_owned()),
        "{n}°" => Some("degrees".to_owned()),
        _ => None,
    });
    let units = Units::new(Arc::new(LocaleNumbers::from_tag("ar-EG")), Arc::new(words));
    // An isolate may surround the number in right-to-left text.
    let (pixels, percent) = (units.pixels(37.0), units.percent(50.0));
    assert!(pixels.contains("٣٧") && pixels.contains("بكسل") && !pixels.contains("37"), "{pixels:?}");
    assert!(percent.contains("٥٠") && percent.starts_with('٪'), "{percent:?}");
    assert_eq!(units.degrees(45.0), "٤٥°");
    assert_eq!(units.unit(UnitKind::Percent).prefix(), "٪");
}

// A length in a unit reads its word with the unit's own digits, in the user's language; a field of that unit takes the same word.
#[test]
fn a_length_is_a_word_with_a_placeholder_for_each_unit() {
    use super::units::LengthUnit;
    let units = english();
    assert_eq!(units.length(LengthUnit::Inches, 0.5), "0.5 in");
    assert_eq!(units.length(LengthUnit::Points, 12.0), "12 pt");
    assert_eq!(units.length(LengthUnit::Centimetres, 2.54), "2.54 cm");
    assert_eq!(units.length(LengthUnit::Picas, 1.0), "1 pica");
    let words = Words::new(|word| (word.english == "{n} in").then(|| "{n} po".to_owned()));
    let french = Units::new(Arc::new(LocaleNumbers::from_tag("fr")), Arc::new(words));
    assert_eq!(french.length(LengthUnit::Inches, 1.0), "1 po");
    assert_eq!(french.unit(LengthUnit::Inches.kind()).suffix(), " po");
}
