//! Length units.

use super::units::{Length, LengthUnit, UnitContext};

// to_pixels then from_pixels returns the value for every unit at 300 ppi.
#[test]
fn every_unit_round_trips_through_pixels() {
    let context = UnitContext { pixels_per_inch: 300.0, extent: Some(800.0), ..UnitContext::default() };
    for unit in LengthUnit::ALL {
        let pixels = context.to_pixels(12.5, unit).unwrap();
        let back = context.from_pixels(pixels, unit).unwrap();
        assert!((back - 12.5).abs() < 1e-9, "{unit:?}: {back}");
    }
    assert_eq!(context.to_pixels(1.0, LengthUnit::Inches), Some(300.0));
}

// Percent without an extent is None; with extent 200, 50 % is 100 px.
#[test]
fn percent_converts_only_with_an_extent() {
    let none = UnitContext::default();
    assert_eq!(none.to_pixels(50.0, LengthUnit::Percent), None);
    assert_eq!(none.from_pixels(100.0, LengthUnit::Percent), None);
    let with = UnitContext { extent: Some(200.0), ..UnitContext::default() };
    assert_eq!(with.to_pixels(50.0, LengthUnit::Percent), Some(100.0));
    assert_eq!(with.from_pixels(100.0, LengthUnit::Percent), Some(50.0));
}

// Length::parse("2in") is 2 inches; "5 mm", "12pica", "50%" likewise; "2xx" is None.
#[test]
fn a_typed_suffix_names_its_unit() {
    let parse = |text| Length::parse(text, LengthUnit::Pixels);
    assert_eq!(parse("2in"), Some(Length { value: 2.0, unit: LengthUnit::Inches }));
    assert_eq!(parse("5 mm"), Some(Length { value: 5.0, unit: LengthUnit::Millimetres }));
    assert_eq!(parse("12pica"), Some(Length { value: 12.0, unit: LengthUnit::Picas }));
    assert_eq!(parse("50%"), Some(Length { value: 50.0, unit: LengthUnit::Percent }));
    assert_eq!(parse("-3.5"), Some(Length { value: -3.5, unit: LengthUnit::Pixels }));
    assert_eq!(parse("2xx"), None);
    assert_eq!(parse("2+3"), None);
    assert_eq!(parse("in"), None);
}

// points_per_inch 72.27 changes Points and Picas only.
#[test]
fn traditional_points_change_points_and_picas() {
    let postscript = UnitContext { pixels_per_inch: 300.0, ..UnitContext::default() };
    let traditional = UnitContext { points_per_inch: 72.27, ..postscript };
    for unit in LengthUnit::ALL {
        let (a, b) = (postscript.to_pixels(10.0, unit), traditional.to_pixels(10.0, unit));
        if matches!(unit, LengthUnit::Points | LengthUnit::Picas) {
            assert_ne!(a, b, "{unit:?}");
        } else {
            assert_eq!(a, b, "{unit:?}");
        }
    }
}
