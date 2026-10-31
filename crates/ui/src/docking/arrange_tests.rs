//! Arrangement rects and menu rows.

use egui::{Rect, pos2, vec2};

use super::arrange::{ArrangeLabels, Arrangement, arrange_rects, cascade_rects};

const AREA: Rect = Rect::from_min_max(pos2(10.0, 20.0), pos2(210.0, 220.0));

#[test]
// 2-up Vertical splits the area into two equal columns; 3-up Stacked gives one left cell and two right cells.
fn n_up_cells_split_the_area_as_photoshop() {
    let two = arrange_rects(Arrangement::TwoUpVertical, AREA, 5).unwrap();
    assert_eq!(two, vec![Rect::from_min_max(pos2(10.0, 20.0), pos2(110.0, 220.0)), Rect::from_min_max(pos2(110.0, 20.0), pos2(210.0, 220.0))]);
    let stacked = arrange_rects(Arrangement::ThreeUpStacked, AREA, 3).unwrap();
    assert_eq!(
        stacked,
        vec![
            Rect::from_min_max(pos2(10.0, 20.0), pos2(110.0, 220.0)),
            Rect::from_min_max(pos2(110.0, 20.0), pos2(210.0, 120.0)),
            Rect::from_min_max(pos2(110.0, 120.0), pos2(210.0, 220.0)),
        ]
    );
    assert_eq!(arrange_rects(Arrangement::SixUp, AREA, 6).unwrap().len(), 6);
    assert_eq!(arrange_rects(Arrangement::TileAllHorizontally, AREA, 4).unwrap()[3].min, pos2(10.0, 170.0));
    assert_eq!(arrange_rects(Arrangement::Tile, AREA, 5).unwrap().len(), 5);
    assert!(arrange_rects(Arrangement::FourUp, AREA, 1).is_none());
    assert!(arrange_rects(Arrangement::Cascade, AREA, 4).is_none());
}

#[test]
// Cascade steps each rect by CASCADE_STEP and wraps at the area's edge.
fn cascade_steps_and_wraps() {
    let rects = cascade_rects(AREA, vec2(120.0, 120.0), 4, 28.0);
    assert_eq!(rects[0].min, pos2(10.0, 20.0));
    assert_eq!(rects[1].min, pos2(38.0, 48.0));
    assert_eq!(rects[2].min, pos2(66.0, 76.0));
    // 94 + 120 > 210: both axes start over at the edge.
    assert_eq!(rects[3].min, pos2(10.0, 20.0));
    assert!(rects.iter().all(|r| r.size() == vec2(120.0, 120.0)));
}

#[test]
// With one document the n-up and Match rows are disabled and Float in Window is enabled.
fn menu_rows_are_enabled_per_document_count() {
    let entries = Arrangement::menu_entries(&ArrangeLabels::default(), 1, 1, 0);
    let enabled = |a: Arrangement| entries.iter().filter_map(|e| e.item()).find(|i| i.command() == Some(&a)).unwrap().is_enabled();
    assert!(!enabled(Arrangement::TwoUpVertical));
    assert!(!enabled(Arrangement::MatchAll));
    assert!(enabled(Arrangement::FloatInWindow));
    assert!(!enabled(Arrangement::ConsolidateAllToTabs));
    assert_eq!(Arrangement::from_command_id(Arrangement::FourUp.command_id()), Some(Arrangement::FourUp));
}
