//! Length units.

// to_pixels then from_pixels returns the value for every unit at 300 ppi.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn every_unit_round_trips_through_pixels() {
    todo!()
}

// Percent without an extent is None; with extent 200, 50 % is 100 px.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn percent_converts_only_with_an_extent() {
    todo!()
}

// Length::parse("2in") is 2 inches; "5 mm", "12pica", "50%" likewise; "2xx" is None.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn a_typed_suffix_names_its_unit() {
    todo!()
}

// points_per_inch 72.27 changes Points and Picas only.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn traditional_points_change_points_and_picas() {
    todo!()
}
