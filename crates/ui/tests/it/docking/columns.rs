//! Dock columns: the size rules of groups and columns, and the flyout of an iconic column.

use craft_behavior::Proposal;
use craft_behavior::dock::{Column, Group};
use craft_ui::buttons::Button;
use craft_ui::contract::{Bind, Props as _, Show as _, ShowContainer as _};
use craft_ui::docking::{DockHeights, GroupAt, GroupSize, IconicColumn, PanelSpec, fit_widths, group_heights};
use craft_ui::sections::Panel;
use craft_ui::testing::layout::Block;
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

/// Shows the column with `open` as its bound flyout and returns the panel the flyout shows.
fn run(open: &mut Option<u8>, shown: &Cell<Option<u8>>, ui: &mut egui::Ui) {
    let column = Column::new(vec![Group::new(vec![0, 1], 0)]);
    IconicColumn::new("Panels", global("dock", "columns test"), &column, Bind::bound(open), &spec).show_flyouts(ui, |group, _, _, _| {
        shown.set(group.active_panel().copied());
        None
    });
}

#[trial]
fn clicking_an_icon_writes_the_bound_flyout_and_escape_writes_none() {
    let open: Rc<RefCell<Option<u8>>> = Rc::default();
    let shown: Rc<Cell<Option<u8>>> = Rc::default();
    let (bound, group_active) = (Rc::clone(&open), Rc::clone(&shown));
    let mut fixture = Fixture::new(move |ui| run(&mut bound.borrow_mut(), &group_active, ui));
    fixture.settle();
    let icon = fixture.by_label("Panel 1").expect("the second panel has an icon");
    fixture.click(&icon);
    fixture.idle(2);
    assert_eq!(*open.borrow(), Some(1), "the click wrote the clicked panel");
    assert_eq!(shown.get(), Some(1), "the flyout shows the group with the clicked panel active");
    fixture.key(Key::Escape, Modifiers::NONE);
    fixture.idle(2);
    assert_eq!(*open.borrow(), None, "Escape wrote None");
}

#[trial]
fn writing_the_bound_flyout_opens_the_same_flyout_as_a_click_and_writing_it_again_keeps_it_open() {
    let open: Rc<RefCell<Option<u8>>> = Rc::new(RefCell::new(Some(1)));
    let shown: Rc<Cell<Option<u8>>> = Rc::default();
    let (bound, group_active) = (Rc::clone(&open), Rc::clone(&shown));
    let mut fixture = Fixture::new(move |ui| run(&mut bound.borrow_mut(), &group_active, ui));
    fixture.settle();
    assert_eq!(shown.get(), Some(1), "the bound panel's flyout is open");
    *open.borrow_mut() = Some(1);
    fixture.idle(2);
    assert_eq!(shown.get(), Some(1), "writing the same panel again keeps it open");
    assert_eq!(*open.borrow(), Some(1));
}

/// A panel that opens at half the room: its flyout shows the panel's title row and the button under it whole.
fn roomy_spec(panel: &u8) -> PanelSpec {
    spec(panel).heights(DockHeights::new(ROOM / 2.0))
}

/// The panel asks for a height, so the flyout does not open at the least height, where the panel's body would clip
/// the button and a press on the hidden half would land on the panel.
#[trial]
fn a_button_in_the_flyout_takes_its_click_and_leaves_the_flyout_open() {
    let open: Rc<RefCell<Option<u8>>> = Rc::new(RefCell::new(Some(0)));
    let clicks: Rc<Cell<u32>> = Rc::default();
    let (bound, counted) = (Rc::clone(&open), Rc::clone(&clicks));
    let mut fixture =
        Fixture::new(move |ui| {
            let column = Column::new(vec![Group::new(vec![0, 1], 0)]);
            let counted = Rc::clone(&counted);
            IconicColumn::new("Panels", global("dock", "flyout button test"), &column, Bind::bound(&mut bound.borrow_mut()), &roomy_spec)
                .show_flyouts(ui, move |_, at, ui, _| {
                    let GroupAt::Show(rect) = at else { return None };
                    Panel::new("Panel").show_at(ui, rect, |ui, _| {
                        if craft_ui::testing::contract::clicked(&Button::new("Apply").show(ui)) {
                            counted.set(counted.get() + 1);
                        }
                    });
                    None
                });
        });
    fixture.settle();
    let apply = fixture.by_label("Apply").expect("the open flyout shows its button");
    fixture.click(&apply);
    fixture.idle(2);
    assert_eq!(clicks.get(), 1, "the click reached the button");
    assert_eq!(*open.borrow(), Some(0), "a click inside the flyout leaves it open");
}

/// A panel body taller than the flyout's least height opens a flyout that shows all of it: the view measures the
/// group at the flyout's width, and the flyout takes that height in a window with room.
#[trial]
fn a_flyout_is_as_tall_as_its_group_measures() {
    const BLOCK: egui::Vec2 = egui::Vec2::new(40.0, 30.0);
    const BLOCKS: usize = 8;
    let body = |ui: &mut egui::Ui| {
        let mut last = egui::Rect::NOTHING;
        for salt in 0..BLOCKS {
            last = Block::new(BLOCK).id_salt(salt).show(ui).rect;
        }
        last
    };
    let seen: Rc<Cell<(egui::Rect, egui::Rect)>> = Rc::new(Cell::new((egui::Rect::NOTHING, egui::Rect::NOTHING)));
    let shown = Rc::clone(&seen);
    let mut fixture = Fixture::new(move |ui| {
        let column = Column::new(vec![Group::new(vec![0, 1], 0)]);
        let shown = Rc::clone(&shown);
        IconicColumn::new("Panels", global("dock", "measured flyout test"), &column, Bind::given(Some(0)), &spec).show_flyouts(
            ui,
            move |_, at, ui, _| match at {
                GroupAt::Measure(width) => Some(Panel::new("Panel").measure_in(ui, Proposal::Width(width), |ui, _| body(ui)).y),
                GroupAt::Show(rect) => {
                    let mut last = egui::Rect::NOTHING;
                    let panel = Panel::new("Panel").show_at(ui, rect, |ui, _| last = body(ui)).output.response.rect;
                    shown.set((panel, last));
                    None
                }
            },
        );
    });
    fixture.settle();
    let (panel, last) = seen.get();
    assert!(panel.contains_rect(last), "the flyout's panel shows its whole body: {panel:?} {last:?}");
}
