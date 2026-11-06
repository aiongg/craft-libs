//! Arithmetic in number fields.

use super::expr::{ExprUnits, parse, parse_with_units, relative};
use crate::localized::units::{LengthUnit, UnitContext};

fn units(unit: Option<LengthUnit>, suffix: &str, pixels_per_inch: f64, extent: Option<f64>) -> ExprUnits<'_> {
    ExprUnits { unit, suffix, context: UnitContext { pixels_per_inch, extent, ..UnitContext::default() } }
}

// "2+3*4" is 14, "(2+3)*4" 20, "-2**2" -4, "2**3**2" 512.
#[test]
fn precedence_and_parentheses_follow_arithmetic() {
    for (text, want) in
        [("2+3*4", 14.0), ("(2+3)*4", 20.0), ("-2**2", -4.0), ("2**3**2", 512.0), ("−12 + .5", -11.5), ("10%3", 1.0), ("pi*2", std::f64::consts::TAU)]
    {
        assert_eq!(parse(text), Some(want), "{text}");
    }
}

// "1in" in a pixel field at 300 ppi is 300; "pica" wins over "pc".
#[test]
fn a_unit_converts_into_the_fields_unit() {
    let px = units(Some(LengthUnit::Pixels), "px", 300.0, None);
    for (text, want) in [("1in", 300.0), ("2.54cm+10", 310.0), ("1pica", 50.0), ("1pc", 50.0), ("72pt", 300.0)] {
        let got = parse_with_units(text, &px);
        assert!(got.is_some_and(|v| (v - want).abs() < 1e-9), "{text}: {got:?}");
    }
    let degrees = units(None, "°", 72.0, None);
    assert_eq!(parse_with_units("45°*2", &degrees), Some(90.0));
    assert_eq!(parse_with_units("2in", &degrees), None, "a unitless field takes no unit");
}

// "10px" in a percent field with no extent is None.
#[test]
fn a_length_in_a_percent_field_without_an_extent_is_invalid() {
    let percent = units(Some(LengthUnit::Percent), "%", 72.0, None);
    assert_eq!(parse_with_units("10px", &percent), None);
    assert_eq!(parse_with_units("50%", &percent), Some(50.0));
    let with_extent = units(Some(LengthUnit::Percent), "%", 72.0, Some(600.0));
    assert_eq!(parse_with_units("300px", &with_extent), Some(50.0));
}

// relative("*2", 150) is 300, "/4" 37.5; "+10" and "-10" are None.
#[test]
fn relative_entries_scale_the_current_value() {
    assert_eq!(relative("*2", 150.0), Some(300.0));
    assert_eq!(relative("/4", 150.0), Some(37.5));
    assert_eq!(relative("/0", 150.0), None);
    assert_eq!(relative("+10", 150.0), None);
    assert_eq!(relative("-10", 150.0), None);
    assert_eq!(relative("5", 150.0), None);
}

// 1025 characters or 33 nested parentheses give None; "1/0" gives None.
#[test]
fn long_or_deep_text_is_refused() {
    assert_eq!(parse(&"1".repeat(1025)), None);
    assert_eq!(parse(&format!("{}1{}", "(".repeat(33), ")".repeat(33))), None);
    assert_eq!(parse(&format!("{}1{}", "(".repeat(20), ")".repeat(20))), Some(1.0));
    assert_eq!(parse("1/0"), None);
}
