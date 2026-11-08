//! Dock columns: the size rules of groups and columns, and the flyout of an iconic column.

use craft_behavior::dock::{Column, Group};
use craft_ui::docking::{DockHeights, GroupSize, IconicColumn, PanelSpec, fit_widths, group_heights};
use craft_ui::testing::memory::global;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const STRIP: f32 = 20.0;
const MIN: f32 = 60.0;
const ROOM: f32 = 400.0;

fn open(min: f32) -> GroupSize {
    GroupSize::new(min)
}

fn sum(heights: &[f32]) -> f32 {
    heights.iter().sum()
}

#[trial]
fn groups_with_a_saved_height_keep_it_and_the_unsized_group_takes_the_rest() {
    let mut saved = open(MIN);
    saved.saved = Some(120.0);
    let heights = group_heights(&[saved, open(MIN)], STRIP, ROOM);
    assert_eq!(heights[0], 120.0, "the resized group keeps its height");
    assert_eq!(heights[1], ROOM - 120.0, "the group never resized takes what is left");
}

#[trial]
fn a_minimized_group_takes_the_strip_only() {
    let mut minimized = open(MIN);
    minimized.minimized = true;
    let heights = group_heights(&[minimized, open(MIN)], STRIP, ROOM);
    assert_eq!(heights, vec![STRIP, ROOM - STRIP]);
}

#[trial]
fn default_heights_give_way_to_the_group_that_fills_before_it_gets_less() {
    let mut roomy = open(MIN);
    roomy.content = Some(DockHeights::new(250.0).compact(100.0));
    let mut filling = open(MIN);
    filling.content = Some(DockHeights::new(0.0).fill(200.0));
    let heights = group_heights(&[roomy, filling], STRIP, ROOM);
    assert_eq!(heights[1], 200.0, "the filling group gets its fill height");
    assert_eq!(heights[0], ROOM - 200.0, "the other group gave way from its default of 250");
    assert!(sum(&heights) <= ROOM, "the groups fit the room");
}

#[trial]
fn heights_shrink_towards_their_minimums_when_the_room_is_short() {
    let heights = group_heights(&[open(MIN), open(MIN), open(MIN)], STRIP, 2.0 * MIN);
    assert!(heights.iter().all(|height| *height >= MIN), "no group goes below its minimum");
}

#[trial]
fn columns_shrink_in_proportion_down_to_their_floors_and_iconic_columns_keep_their_width() {
    let columns = [(300.0, Some(100.0)), (50.0, None), (300.0, Some(200.0))];
    let widths = fit_widths(&columns, 500.0);
    assert_eq!(widths[1], 50.0, "an iconic column keeps its width");
    assert!(widths[0] >= 100.0 && widths[2] >= 200.0, "no column goes below its floor");
    assert!(sum(&widths) <= 500.0 + 0.5, "the columns fit the room");
}

fn spec(panel: &u8) -> PanelSpec {
    PanelSpec::new(format!("Panel {panel}"))
}

#[trial]
fn clicking_an_icon_opens_the_flyout_of_its_panel_and_escape_closes_it() {
    let changes: Rc<RefCell<Vec<Option<u8>>>> = Rc::default();
    let shown: Rc<Cell<Option<u8>>> = Rc::default();
    let (seen, group_active) = (Rc::clone(&changes), Rc::clone(&shown));
    let mut fixture = Fixture::new(move |ui| {
        let column = Column::new(vec![Group::new(vec![0, 1], 0)], 240.0);
        let out = IconicColumn::new("Panels", global("dock", "columns test"), &column, &spec)
            .show_flyouts(ui, |group, _, _, _| group_active.set(group.active_panel().copied()));
        if let Some(change) = out.flyout {
            seen.borrow_mut().push(change);
        }
    });
    fixture.settle();
    let icon = fixture.by_label("Panel 1").expect("the second panel has an icon");
    fixture.click(&icon);
    fixture.idle(2);
    assert_eq!(changes.borrow().as_slice(), [Some(1)], "the click opened the flyout of the clicked panel");
    assert_eq!(shown.get(), Some(1), "the flyout shows the group with the clicked panel active");
    fixture.key(Key::Escape, Modifiers::NONE);
    fixture.idle(2);
    assert_eq!(changes.borrow().as_slice(), [Some(1), None], "Escape closed the flyout");
}
