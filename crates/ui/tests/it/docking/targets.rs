//! `DockTargets`: the drop-zone registry resolves a pointer to the spot a drop would land on.

use craft_behavior::{DockSide, GroupId};
use craft_ui::docking::targets::{EDGE_BAND, SIDE_EDGE_ZONE_DEPTH};
use craft_ui::docking::{DockSpot, DockTargets, ZoneRule};
use craft_ui::testing::trial;
use egui::{Pos2, Rect, Vec2};

const GROUP: GroupId = GroupId::new(4);
const LAYOUT: Rect = Rect { min: Pos2::new(0.0, 0.0), max: Pos2::new(400.0, 300.0) };
const STRIP_HEIGHT: f32 = 24.0;

fn group_rect() -> Rect {
    LAYOUT
}

fn strip() -> Rect {
    Rect::from_min_size(group_rect().min, Vec2::new(group_rect().width(), STRIP_HEIGHT))
}

fn body() -> Rect {
    Rect::from_min_max(Pos2::new(group_rect().left(), strip().bottom()), group_rect().max)
}

/// One group the way group rendering registers it: the strip, the sides and the middle of the body.
fn registered(rule: ZoneRule) -> DockTargets {
    let mut targets = DockTargets::new();
    for (side, rect) in rule.zones(body()) {
        targets.add(DockSpot::Group { group: GROUP, side }, rect);
    }
    let line = Rect::from_min_size(strip().min, Vec2::new(2.0, STRIP_HEIGHT));
    targets.add(DockSpot::Tab { group: GROUP, index: 0, line }, strip());
    targets
}

#[trial]
fn a_pointer_resolves_to_a_tab_insertion_a_group_side_or_the_group() {
    for rule in [ZoneRule::Fraction, ZoneRule::Depth] {
        let targets = registered(rule);
        assert!(matches!(targets.at(strip().center()), Some(DockSpot::Tab { group: GROUP, index: 0, .. })), "{rule:?}: the strip");
        let left = Pos2::new(body().left() + 2.0, body().center().y);
        assert_eq!(targets.at(left), Some(DockSpot::Group { group: GROUP, side: DockSide::Start }), "{rule:?}: the start side");
        let bottom = Pos2::new(body().center().x, body().bottom() - 2.0);
        assert_eq!(targets.at(bottom), Some(DockSpot::Group { group: GROUP, side: DockSide::Bottom }), "{rule:?}: the bottom side");
        assert_eq!(targets.at(body().center()), Some(DockSpot::Group { group: GROUP, side: DockSide::Center }), "{rule:?}: the middle");
    }
}

#[trial]
fn the_side_zones_of_a_body_do_not_overlap_and_stay_inside_it() {
    for rule in [ZoneRule::Fraction, ZoneRule::Depth] {
        let zones = rule.zones(body());
        for (at, (_, rect)) in zones.iter().enumerate() {
            assert!(body().contains_rect(*rect), "{rule:?}: zone {at} stays inside the body");
            for (_, other) in &zones[at + 1..] {
                assert!(!rect.shrink(0.01).intersects(*other), "{rule:?}: zone {at} does not overlap another");
            }
        }
    }
    let depth = ZoneRule::Depth.zones(body())[0].1.width();
    assert!(depth <= SIDE_EDGE_ZONE_DEPTH, "a side zone is at most the named depth");
}

#[trial]
fn an_edge_band_beats_a_group_side_near_the_layouts_border() {
    let mut targets = registered(ZoneRule::Fraction);
    let band = Rect::from_min_max(LAYOUT.min, Pos2::new(LAYOUT.left() + EDGE_BAND, LAYOUT.bottom()));
    targets.add(DockSpot::Edge(DockSide::Start), band);
    let near = Pos2::new(LAYOUT.left() + 2.0, body().center().y);
    assert_eq!(targets.at(near), Some(DockSpot::Edge(DockSide::Start)));
    let inside = Pos2::new(LAYOUT.left() + EDGE_BAND + 2.0, body().center().y);
    assert_eq!(targets.at(inside), Some(DockSpot::Group { group: GROUP, side: DockSide::Start }), "past the band the group's side");
}

#[trial]
fn a_raised_layer_and_a_blocked_rect_hide_the_spots_below() {
    let mut targets = registered(ZoneRule::Fraction);
    let window = Rect::from_center_size(body().center(), Vec2::splat(80.0));
    targets.raise();
    targets.add(DockSpot::Blocked, window);
    assert_eq!(targets.at(window.center()), None, "a floating window that takes no drop hides the group under it");
    assert!(targets.at(strip().center()).is_some(), "outside the window the group still takes the drop");
}

#[trial]
fn clearing_the_registry_empties_it_between_passes() {
    let mut targets = registered(ZoneRule::Fraction);
    assert!(targets.at(body().center()).is_some());
    targets.clear();
    assert!(targets.is_empty() && targets.at(body().center()).is_none());
}
