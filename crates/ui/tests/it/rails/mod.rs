//! The rail's own logic: which tool a slot shows, what the rail remembers, where a drop goes, and the flyout's hold
//! and click.

mod drawer;
mod panel_rail;

use craft_ui::contract::{Role, Show as _};
use craft_ui::rails::{DropAt, FlyoutButton, LastUsed, RailTool, ToolSlot, column_toggle_icon, drop_in_slot, remembered, shown_tool, visible_slots};
use craft_ui::runtime::commands::EditCommands;
use craft_ui::testing::{Fixture, trial};
use std::cell::RefCell;
use std::rc::Rc;

const UNDO: u8 = 1;
const REDO: u8 = 2;
const CUT: u8 = 3;
/// More than the 0.5 s a press must be held to open the flyout.
const HELD: f64 = 0.7;

fn tools(ids: &[u8]) -> Vec<RailTool<u8>> {
    let command = |id: u8| match id {
        UNDO => EditCommands::Undo,
        REDO => EditCommands::Redo,
        _ => EditCommands::Cut,
    };
    ids.iter().map(|id| RailTool::new(*id, command(*id))).collect()
}

fn refs(tools: &[RailTool<u8>]) -> Vec<&RailTool<u8>> {
    tools.iter().collect()
}

#[trial]
fn hidden_tools_leave_their_slot_and_an_emptied_slot_goes() {
    let slots = [ToolSlot::new(0, tools(&[UNDO, REDO])), ToolSlot::new(1, tools(&[CUT]))];
    let visible = visible_slots(&slots, &[REDO, CUT]);
    assert_eq!(visible.len(), 1, "the slot with no visible tool is dropped");
    let (index, kept) = &visible[0];
    assert_eq!(*index, 0);
    assert_eq!(kept.iter().map(|tool| tool.id).collect::<Vec<_>>(), [UNDO], "the slot lists only its visible tools");
}

#[trial]
fn a_slot_shows_the_current_tool_then_the_last_used_then_the_first_enabled() {
    let all = tools(&[UNDO, REDO, CUT]);
    let slot = refs(&all);
    let mut last = LastUsed::default();
    let always = |_: &RailTool<u8>| true;
    assert_eq!(shown_tool(&99, 0, &slot, &last, always).map(|tool| tool.id), Some(UNDO), "the first enabled tool without a memory");
    last.set(0, CUT);
    assert_eq!(shown_tool(&99, 0, &slot, &last, always).map(|tool| tool.id), Some(CUT), "the remembered tool");
    assert_eq!(shown_tool(&REDO, 0, &slot, &last, always).map(|tool| tool.id), Some(REDO), "the current tool wins over the memory");
    let not_cut = |tool: &RailTool<u8>| tool.id != CUT;
    assert_eq!(shown_tool(&99, 0, &slot, &last, not_cut).map(|tool| tool.id), Some(UNDO), "a disabled memory is skipped");
    let none = |_: &RailTool<u8>| false;
    assert_eq!(shown_tool(&99, 0, &slot, &last, none).map(|tool| tool.id), Some(UNDO), "with nothing enabled the first tool shows");
}

#[trial]
fn the_slot_that_holds_the_current_tool_remembers_it() {
    let slots = [ToolSlot::new(4, tools(&[UNDO, REDO])), ToolSlot::new(5, tools(&[CUT]))];
    let last = LastUsed::default();
    let next = remembered(&slots, &REDO, &last).expect("the memory changes");
    assert_eq!(next.get(4), Some(&REDO));
    assert_eq!(next.get(5), None, "another slot keeps nothing");
    assert!(remembered(&slots, &REDO, &next).is_none(), "an unchanged memory reports nothing");
    assert!(remembered(&slots, &99, &last).is_none(), "a tool no slot holds changes nothing");
}

#[trial]
fn a_drop_joins_the_slot_in_its_middle_and_makes_a_slot_at_its_edges() {
    assert_eq!(drop_in_slot(0.1, 7, 2, 3), DropAt::new_slot(2), "the leading edge goes before the slot");
    assert_eq!(drop_in_slot(0.9, 7, 2, 3), DropAt::new_slot(3), "the trailing edge goes after it");
    assert_eq!(drop_in_slot(0.5, 7, 2, 3), DropAt::into_slot(7, 3), "the middle appends to the slot's tools");
}

#[trial]
fn the_column_toggle_points_towards_the_canvas_when_widening() {
    assert_eq!(column_toggle_icon(1, true, false), "chevrons_right", "a left rail widens to the right");
    assert_eq!(column_toggle_icon(2, true, false), "chevrons_left", "and narrows back to the left");
    assert_eq!(column_toggle_icon(1, false, false), "chevrons_left", "a right rail widens to the left");
    assert_eq!(column_toggle_icon(1, true, true), "chevrons_left", "directional icons mirror in a right-to-left interface");
}

struct Rig {
    fixture: Fixture,
    selected: Rc<RefCell<Vec<u8>>>,
}

fn rig(hold_selects: bool) -> Rig {
    let selected = Rc::new(RefCell::new(Vec::new()));
    let seen = selected.clone();
    let mut fixture = Fixture::new(move |ui| {
        let all = tools(&[UNDO, REDO, CUT]);
        let out = FlyoutButton::new(&all, &UNDO).hold_selects(hold_selects).show(ui);
        if let Some(tool) = out.select {
            seen.borrow_mut().push(tool);
        }
    });
    fixture.settle();
    Rig { fixture, selected }
}

fn button(rig: &Rig) -> craft_ui::testing::Target {
    rig.fixture.by_role(Role::BUTTON).into_iter().next().expect("the flyout button shows")
}

#[trial]
fn a_click_selects_the_shown_tool_and_holding_opens_the_flyout() {
    let mut clicked_rig = rig(true);
    let target = button(&clicked_rig);
    clicked_rig.fixture.click(&target);
    clicked_rig.fixture.idle(3);
    assert_eq!(*clicked_rig.selected.borrow(), [UNDO], "a click selects the shown tool");
    assert!(clicked_rig.fixture.by_role(Role::MENU).is_empty(), "a click opens no flyout");

    let mut held = rig(true);
    let target = button(&held);
    held.fixture.press(&target);
    held.fixture.advance(HELD);
    held.fixture.idle(3);
    assert!(!held.fixture.by_role(Role::MENU).is_empty(), "a held press opens the flyout");
}

#[trial]
fn a_hold_that_does_not_select_only_opens_the_flyout() {
    let mut held = rig(false);
    let target = button(&held);
    held.fixture.press(&target);
    held.fixture.advance(HELD);
    held.fixture.idle(3);
    held.fixture.release(&target);
    held.fixture.idle(3);
    assert!(held.selected.borrow().is_empty(), "releasing after the hold selects nothing when the hold does not select");
}
