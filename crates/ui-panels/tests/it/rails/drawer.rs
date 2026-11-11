//! The drawer's own logic: how an edit changes the toolbar and what a drop on the rail asks for, and which of its tools
//! drag.

use craft_ui::rails::{DropAt, RailTool, ToolSlot, ToolbarEdit, apply_toolbar_edit};
use craft_ui::runtime::commands::EditCommands;
use craft_ui::runtime::drag;
use craft_ui::testing::{Fixture, stories, trial};
use egui::Vec2;

const UNDO: u8 = 1;
const REDO: u8 = 2;
const CUT: u8 = 3;

fn tool(id: u8) -> RailTool<u8> {
    let command = match id {
        UNDO => EditCommands::Undo,
        REDO => EditCommands::Redo,
        CUT => EditCommands::Cut,
        _ => EditCommands::Copy,
    };
    RailTool::new(id, command)
}

fn slot(id: u32, ids: &[u8]) -> ToolSlot<u8> {
    ToolSlot::new(id, ids.iter().map(|id| tool(*id)).collect())
}

/// The tool ids of each slot, in order.
fn layout(slots: &[ToolSlot<u8>]) -> Vec<(u32, Vec<u8>)> {
    slots.iter().map(|slot| (slot.id, slot.tools.iter().map(|tool| tool.id).collect())).collect()
}

#[trial]
fn a_tool_moved_down_its_own_slot_lands_at_the_index_after_it_leaves() {
    let slots = [slot(0, &[UNDO, REDO, CUT])];
    let edit = ToolbarEdit::move_to(UNDO, DropAt::into_slot(0, 3));
    assert_eq!(layout(&apply_toolbar_edit(&slots, &edit, &[])), [(0, vec![REDO, CUT, UNDO])], "the removal shifts the index");
}

/// A tool already on the toolbar is dimmed and does not drag; a tool that is not on it drags to the rail.
#[trial]
fn only_a_tool_off_the_toolbar_drags() {
    let entry = stories().into_iter().find(|entry| entry.story.name == "toolbar_drawer").expect("the drawer story");
    let mut fixture = Fixture::story(&entry.story);
    fixture.settle();
    let drags = |fixture: &mut Fixture, label: &str| {
        let row = fixture.by_label(label).unwrap_or_else(|| panic!("the {label} row"));
        let before = drag::started(fixture.ctx());
        fixture.drag_hold_at(row.center(), Vec2::new(0.0, 30.0));
        let started = drag::started(fixture.ctx()) > before;
        fixture.release_at(row.center() + Vec2::new(0.0, 30.0)).settle();
        started
    };
    assert!(!drags(&mut fixture, "Brush"), "Brush is on the toolbar");
    assert!(drags(&mut fixture, "Eraser"), "Eraser is not");
}
