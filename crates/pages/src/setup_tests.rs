//! Paper sizes, margins and orientation.

use super::*;

#[test]
fn paper_names_match_either_way_round() {
    assert_eq!(paper_named(PAPER_SIZES, 792.0, 612.0).map(|p| p.name), Some("Letter"));
    assert_eq!(paper_named(PAPER_SIZES, 595.0, 842.0).map(|p| p.name), Some("A4"));
    assert_eq!(paper_named(PAPER_SIZES, 100.0, 100.0), None);
    assert_eq!(paper_named(PAPER_SIZES, 396.0, 612.0).map(|p| p.name), Some("Statement"), "the first of two names wins");
}

#[test]
fn margins_fit_and_presets_match() {
    let normal = Margins::uniform(72.0);
    assert!(normal.fit(612.0, 792.0, 0.0));
    assert!(!Margins { left: 300.0, right: 300.0, ..normal }.fit(612.0, 792.0, 0.0));
    assert!(!normal.fit(612.0, 792.0, 500.0), "the gutter counts");

    let moderate = Margins { top: 72.0, bottom: 72.0, left: 54.0, right: 54.0 };
    assert_eq!(MarginPreset::matching(&MARGIN_PRESETS, &moderate, false).map(|p| p.id), Some("moderate"));
    assert_eq!(MarginPreset::matching(&MARGIN_PRESETS, &moderate, true), None);
    let mirrored = MARGIN_PRESETS[4].margins;
    assert_eq!(MarginPreset::matching(&MARGIN_PRESETS, &mirrored, true).map(|p| p.id), Some("mirrored"));
    assert_eq!(MarginPreset::matching(&MARGIN_PRESETS, &mirrored, false), None);
}

#[test]
fn orientation_turns_size_and_margins() {
    let mut setup = PageSetup { margins: Margins { top: 10.0, bottom: 20.0, left: 30.0, right: 40.0 }, ..PageSetup::default() };
    setup.set_orientation(Orientation::Landscape);
    assert_eq!((setup.width, setup.height), (792.0, 612.0));
    assert_eq!(setup.margins, Margins { top: 30.0, bottom: 40.0, left: 20.0, right: 10.0 });
    setup.set_orientation(Orientation::Portrait);
    assert_eq!((setup.width, setup.height), (612.0, 792.0));
    assert_eq!(setup.margins, Margins { top: 10.0, bottom: 20.0, left: 30.0, right: 40.0 });

    setup.set_size(800.0, 600.0);
    assert_eq!(setup.orientation, Orientation::Landscape);
    setup.set_size(500.0, 500.0);
    assert_eq!(setup.orientation, Orientation::Landscape, "a square keeps the orientation");
    setup.set_size(500.0, 700.0);
    assert_eq!(setup.orientation, Orientation::Portrait);
}
