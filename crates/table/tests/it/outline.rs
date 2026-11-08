//! The outline bands.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use craft_table::ui::outline::{LANE_MIN_ZOOM, OutlineAction, OutlineAxis, OutlineBand, OutlineGroup, bands, lane};
use craft_table::ui::{GridLayout, LineIndex, SheetGeometry, SheetMetrics};
use craft_ui::contract::Show as _;
use craft_ui::testing::{Fixture, trial};
use egui::{Pos2, Rect, Vec2, vec2};

const LANE: f32 = 16.0;
const SLACK: f32 = 4.0;
const ROW_HEIGHT: f32 = 20.0;

fn geometry(origin: Pos2, levels: (u8, u8), frozen: (u32, u32), hidden: &BTreeMap<u32, f32>) -> SheetGeometry {
    let rows = LineIndex::new(hidden, ROW_HEIGHT, 1000);
    let columns = LineIndex::new(&BTreeMap::new(), 80.0, 100);
    let mut layout = GridLayout::new(Rect::from_min_size(origin, vec2(500.0, 400.0)), rows, columns);
    layout.zoom = 1.0;
    layout.frozen = frozen;
    layout.scroll = Vec2::ZERO;
    layout.headings = true;
    layout.bands = bands(levels, true, 1.0, LANE, SLACK);
    layout.right_to_left = false;
    SheetGeometry::new(layout, SheetMetrics::new(ROW_HEIGHT, 8.0, 6.0))
}

/// The band thickness is a lane per level plus one, and a lane stops shrinking below the least zoom.
#[trial]
fn band_thickness_counts_a_lane_per_level_plus_one() {
    let step = lane(LANE, 1.0);
    assert_eq!(bands((0, 0), true, 1.0, LANE, SLACK), Vec2::ZERO);
    assert_eq!(bands((2, 0), true, 1.0, LANE, SLACK), vec2(3.0 * step + SLACK, 0.0));
    assert!(bands((3, 0), true, 1.0, LANE, SLACK).x > bands((2, 0), true, 1.0, LANE, SLACK).x);
    assert_eq!(lane(LANE, 0.1), lane(LANE, LANE_MIN_ZOOM), "lanes stop shrinking at the least zoom");
    assert_eq!(lane(LANE, 2.0), (LANE * 2.0).round());
    let corner = bands((2, 0), false, 1.0, LANE, SLACK);
    assert_eq!(corner.y, step + SLACK, "level buttons keep a corner when the sheet has no headings");
}

/// Rows 4 to 6 (one-based) in a group, its summary row below.
fn rows() -> OutlineAxis {
    OutlineAxis { levels: 1, groups: vec![OutlineGroup { start: 3, end: 5, level: 1, collapsed: false, summary_before: false }] }
}

/// A click on a level button and on a group's button reports their actions.
#[trial]
fn buttons_report_levels_and_toggles() {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let sink = seen.clone();
    let mut fixture = Fixture::new(move |ui| {
        let geometry = geometry(ui.available_rect_before_wrap().min, (1, 0), (0, 0), &BTreeMap::new());
        let none = OutlineAxis::default();
        let out = OutlineBand::new("Outline", &rows(), &none, &geometry).show_at(ui, OutlineBand::area(&geometry));
        sink.borrow_mut().extend(out.actions);
    });
    fixture.room(vec2(520.0, 420.0));
    fixture.settle();
    let level = fixture.by_label("Level 2").expect("the second level button");
    fixture.click(&level);
    let toggle = fixture.by_label("Hide Detail, rows 4\u{2013}6").expect("the group's button");
    fixture.click(&toggle);
    let actions = seen.borrow();
    assert_eq!(actions[0], OutlineAction::Level { rows: true, level: 2 });
    assert!(matches!(actions[1], OutlineAction::Toggle { rows: true, group } if group == rows().groups[0]));
    assert_eq!(actions.len(), 2);
}

/// A group whose first line is hidden and whose other lines are frozen keeps its bracket: a vertical line in its
/// lane, apart from the band's edge.
#[trial]
fn a_group_hidden_at_its_start_in_a_frozen_pane_keeps_its_bracket() {
    let group = OutlineGroup { start: 1, end: 2, level: 1, collapsed: false, summary_before: false };
    let axis = OutlineAxis { levels: 1, groups: vec![group] };
    let edge = Rc::new(RefCell::new(0.0_f32));
    let band_edge = edge.clone();
    let mut fixture = Fixture::new(move |ui| {
        let geometry = geometry(ui.available_rect_before_wrap().min, (1, 0), (3, 0), &BTreeMap::from([(1, 0.0)]));
        *band_edge.borrow_mut() = geometry.cells().left() - geometry.header_size().x;
        let none = OutlineAxis::default();
        OutlineBand::new("Outline", &axis, &none, &geometry).show_at(ui, OutlineBand::area(&geometry));
    });
    fixture.room(vec2(520.0, 420.0));
    fixture.settle();
    let edge = *edge.borrow();
    let brackets =
        fixture.shape_bounds().iter().filter(|bounds| bounds.width() < 4.0 && bounds.height() > 4.0 && bounds.center().x < edge - 1.0).count();
    assert_eq!(brackets, 1, "one bracket in the lane");
}
