//! Tests of the stroke style.

use super::*;

fn with(f: impl FnOnce(&mut StrokeStyle)) -> StrokeStyle {
    let mut s = StrokeStyle::default();
    f(&mut s);
    s
}

// Width, opacity, miter limit 1–500, offset ±1,000,000, 33 entries, a length over 10000, NaN, and
// a pattern total below 0.01 each give their `StrokeError`; PhotoCraft's limit values pass.

// `line_style` from dashes and cap: [] Solid, [4, 2] Dashed, [0, 2] with round caps Dotted,
// [0, 2] with butt caps Custom; `with_line_style` round-trips each preset and zeroes the offset.
#[test]
fn stroke_line_style_names_solid_dashed_dotted_and_custom() {
    assert_eq!(StrokeStyle::default().line_style(), LineStyle::Solid);
    assert_eq!(with(|s| s.dashes = vec![4.0, 2.0]).line_style(), LineStyle::Dashed);
    let dotted = with(|s| {
        s.dashes = vec![0.0, 2.0];
        s.cap = LineCap::Round;
    });
    assert_eq!(dotted.line_style(), LineStyle::Dotted);
    assert_eq!(with(|s| s.dashes = vec![0.0, 2.0]).line_style(), LineStyle::Custom);

    let base = with(|s| {
        s.dash_offset = 5.0;
        s.dashes = vec![1.0, 1.0];
        s.join = LineJoin::Bevel;
    });
    for preset in [LineStyle::Solid, LineStyle::Dashed, LineStyle::Dotted] {
        let out = base.with_line_style(preset);
        assert_eq!(out.line_style(), preset);
        assert_eq!(out.dash_offset, 0.0);
        assert_eq!(out.join, LineJoin::Bevel);
    }
    assert_eq!(base.with_line_style(LineStyle::Custom), base);
}

// A ten-entry pattern: `with_dash_pairs` of three pairs edits pairs 1–3 and leaves pairs 4 and 5;
// an odd pattern's last pair has no gap.
#[test]
fn stroke_dash_pairs_keep_unshown_entries() {
    let s = with(|s| s.dashes = (1..=10).map(|n| n as f32).collect());
    let mut pairs = s.dash_pairs();
    assert_eq!(pairs.len(), 5);
    assert_eq!(pairs[4], (9.0, Some(10.0)));
    pairs.truncate(3);
    pairs[0] = (20.0, None);
    pairs[2] = (50.0, Some(60.0));
    let out = s.with_dash_pairs(&pairs);
    assert_eq!(out.dashes, vec![20.0, 2.0, 3.0, 4.0, 50.0, 60.0, 7.0, 8.0, 9.0, 10.0]);

    let odd = with(|s| s.dashes = vec![3.0, 1.0, 5.0]);
    assert_eq!(odd.dash_pairs(), vec![(3.0, Some(1.0)), (5.0, None)]);
    assert_eq!(odd.with_dash_pairs(&[(3.0, Some(1.0)), (6.0, None)]).dashes, vec![3.0, 1.0, 6.0]);
    assert_eq!(odd.with_dash_pairs(&[(3.0, None)]).dashes, vec![3.0, 1.0, 5.0]);
    let appended = odd.with_dash_pairs(&[(3.0, Some(1.0)), (5.0, Some(2.0)), (7.0, Some(8.0))]);
    assert_eq!(appended.dashes, vec![3.0, 1.0, 5.0, 2.0, 7.0, 8.0]);
    let one = with(|s| s.dashes = vec![4.0, 2.0]);
    assert_eq!(one.with_dash_pairs(&[(4.0, None)]).dashes, vec![4.0]);
}

// Default equals PhotoCraft's `ShapeStroke::default()` options (width 3, centre, butt, miter,
// limit 100, solid, opacity 1); JSON round-trips; `{}` deserialises to the default.

// `to_kurbo` multiplies dashes and offset by the width under `Widths`, keeps them under
// `Document`, and carries cap, join and miter limit.
