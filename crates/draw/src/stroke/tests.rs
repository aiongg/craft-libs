//! Tests of the stroke style (task PE-A1).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Width, opacity, miter limit 1–500, offset ±1,000,000, 33 entries, a length over 10000, NaN, and
// a pattern total below 0.01 each give their `StrokeError`; PhotoCraft's limit values pass.
#[test]
#[ignore = "skeleton"]
fn stroke_validate_rejects_what_photocraft_rejects() {
    todo!()
}

// `line_style` from dashes and cap: [] Solid, [4, 2] Dashed, [0, 2] with round caps Dotted,
// [0, 2] with butt caps Custom; `with_line_style` round-trips each preset and zeroes the offset.
#[test]
#[ignore = "skeleton"]
fn stroke_line_style_names_solid_dashed_dotted_and_custom() {
    todo!()
}

// A ten-entry pattern: `with_dash_pairs` of three pairs edits pairs 1–3 and leaves pairs 4 and 5;
// an odd pattern's last pair has no gap.
#[test]
#[ignore = "skeleton"]
fn stroke_dash_pairs_keep_unshown_entries() {
    todo!()
}

// Default equals PhotoCraft's `ShapeStroke::default()` options (width 3, centre, butt, miter,
// limit 100, solid, opacity 1); JSON round-trips; `{}` deserialises to the default.
#[test]
#[ignore = "skeleton"]
fn stroke_serde_round_trips_and_defaults() {
    todo!()
}

// `to_kurbo` multiplies dashes and offset by the width under `Widths`, keeps them under
// `Document`, and carries cap, join and miter limit.
#[test]
#[ignore = "skeleton"]
fn stroke_to_kurbo_scales_dashes_by_width() {
    todo!()
}
