//! Tests of the stroke style.

use super::*;

fn with(f: impl FnOnce(&mut StrokeStyle)) -> StrokeStyle {
    let mut s = StrokeStyle::default();
    f(&mut s);
    s
}

// Width, opacity, miter limit 1–500, offset ±1,000,000, 33 entries, a length over 10000, NaN, and
// a pattern total below 0.01 each give their `StrokeError`; PhotoCraft's limit values pass.
#[test]
fn stroke_validate_rejects_what_photocraft_rejects() {
    assert_eq!(StrokeStyle::default().validate(), Ok(()));
    let limits = with(|s| {
        s.width = 1_000_000.0;
        s.opacity = 0.0;
        s.miter_limit = 500.0;
        s.dash_offset = -1_000_000.0;
        s.dashes = vec![10_000.0; 32];
    });
    assert_eq!(limits.validate(), Ok(()));
    assert_eq!(with(|s| s.width = -1.0).validate(), Err(StrokeError::Width));
    assert_eq!(with(|s| s.width = 1_000_001.0).validate(), Err(StrokeError::Width));
    assert_eq!(with(|s| s.width = f32::NAN).validate(), Err(StrokeError::Width));
    assert_eq!(with(|s| s.opacity = 1.01).validate(), Err(StrokeError::Opacity));
    assert_eq!(with(|s| s.miter_limit = 0.99).validate(), Err(StrokeError::MiterLimit));
    assert_eq!(with(|s| s.miter_limit = 500.5).validate(), Err(StrokeError::MiterLimit));
    assert_eq!(with(|s| s.dash_offset = 1_000_001.0).validate(), Err(StrokeError::DashOffset));
    assert_eq!(with(|s| s.dashes = vec![1.0; 33]).validate(), Err(StrokeError::DashPattern));
    assert_eq!(with(|s| s.dashes = vec![10_001.0, 1.0]).validate(), Err(StrokeError::DashPattern));
    assert_eq!(with(|s| s.dashes = vec![f32::NAN, 1.0]).validate(), Err(StrokeError::DashPattern));
    assert_eq!(with(|s| s.dashes = vec![0.0, 0.005]).validate(), Err(StrokeError::DashTotal));
    assert_eq!(with(|s| s.dashes = vec![0.0, 0.01]).validate(), Ok(()));
}

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

// JSON round-trips a non-default style; `{}` deserialises to the default.
#[test]
fn stroke_serde_round_trips_and_defaults() {
    let custom = with(|s| {
        s.align = StrokeAlign::Outside;
        s.dashes = vec![1.0, 2.0];
        s.dash_unit = DashUnit::Document;
    });
    let json = serde_json::to_string(&custom).unwrap();
    assert_eq!(serde_json::from_str::<StrokeStyle>(&json).unwrap(), custom);
    assert_eq!(serde_json::from_str::<StrokeStyle>("{}").unwrap(), StrokeStyle::default());
}

// `to_kurbo` multiplies dashes and offset by the width under `Widths`, keeps them under
// `Document`, and carries cap, join and miter limit.
#[test]
fn stroke_to_kurbo_scales_dashes_by_width() {
    let mut s = with(|s| {
        s.width = 4.0;
        s.dashes = vec![2.0, 1.0];
        s.dash_offset = 0.5;
        s.cap = LineCap::Square;
        s.join = LineJoin::Round;
        s.miter_limit = 7.0;
    });
    let k = s.to_kurbo();
    assert_eq!(k.width, 4.0);
    assert_eq!(k.dash_pattern.as_slice(), &[8.0, 4.0]);
    assert_eq!(k.dash_offset, 2.0);
    assert_eq!(k.miter_limit, 7.0);
    assert_eq!((k.start_cap, k.end_cap), (kurbo::Cap::Square, kurbo::Cap::Square));
    assert_eq!(k.join, kurbo::Join::Round);
    s.dash_unit = DashUnit::Document;
    let k = s.to_kurbo();
    assert_eq!(k.dash_pattern.as_slice(), &[2.0, 1.0]);
    assert_eq!(k.dash_offset, 0.5);
}
