//! The drawer's own logic: how an edit changes the toolbar and what a drop on the rail asks for.

use craft_ui::rails::{DrawerGroup, DropAt, RailTool, ToolDrag, ToolDropped, ToolSlot, ToolbarEdit, apply_toolbar_edit, drop_edit};
use craft_ui::runtime::commands::EditCommands;
use craft_ui::testing::trial;

const UNDO: u8 = 1;
const REDO: u8 = 2;
const CUT: u8 = 3;
const COPY: u8 = 4;

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
fn a_tool_added_as_a_new_slot_gets_the_next_free_slot_id_at_the_place() {
    let slots = [slot(0, &[UNDO]), slot(5, &[REDO])];
    let edit = ToolbarEdit::Add { tool: tool(CUT), at: DropAt::new_slot(1) };
    assert_eq!(layout(&apply_toolbar_edit(&slots, &edit, &[])), [(0, vec![UNDO]), (6, vec![CUT]), (5, vec![REDO])]);
}

#[trial]
fn a_tool_moved_down_its_own_slot_lands_at_the_index_after_it_leaves() {
    let slots = [slot(0, &[UNDO, REDO, CUT])];
    let edit = ToolbarEdit::Move { tool: UNDO, at: DropAt::into_slot(0, 3) };
    assert_eq!(layout(&apply_toolbar_edit(&slots, &edit, &[])), [(0, vec![REDO, CUT, UNDO])], "the removal shifts the index");
}

#[trial]
fn a_tool_moved_out_of_its_only_slot_takes_the_slot_with_it() {
    let slots = [slot(0, &[UNDO]), slot(1, &[REDO])];
    let edit = ToolbarEdit::Move { tool: UNDO, at: DropAt::into_slot(1, 0) };
    assert_eq!(layout(&apply_toolbar_edit(&slots, &edit, &[])), [(1, vec![UNDO, REDO])], "the slot the tool left is gone");
}

#[trial]
fn removing_the_last_tool_of_a_slot_removes_the_slot_and_reset_restores_the_defaults() {
    let defaults = [slot(0, &[UNDO, REDO])];
    let slots = [slot(0, &[UNDO]), slot(1, &[REDO])];
    let removed = apply_toolbar_edit(&slots, &ToolbarEdit::Remove { tool: REDO }, &defaults);
    assert_eq!(layout(&removed), [(0, vec![UNDO])]);
    assert_eq!(layout(&apply_toolbar_edit(&removed, &ToolbarEdit::Reset, &defaults)), layout(&defaults));
}

#[trial]
fn a_move_of_a_tool_that_is_not_on_the_toolbar_changes_nothing() {
    let slots = [slot(0, &[UNDO])];
    let edit = ToolbarEdit::Move { tool: COPY, at: DropAt::new_slot(0) };
    assert_eq!(layout(&apply_toolbar_edit(&slots, &edit, &[])), layout(&slots));
}

#[trial]
fn a_drop_on_the_rail_asks_for_the_edit_its_drag_implies() {
    let groups = [DrawerGroup::new("Edit", vec![tool(CUT), tool(COPY)])];
    let toolbar = [slot(0, &[UNDO, REDO]), slot(1, &[CUT])];

    let from_drawer = ToolDropped { drag: ToolDrag::drawer(0, 1), at: DropAt::new_slot(2) };
    let Some(ToolbarEdit::Add { tool, at, .. }) = drop_edit(&from_drawer, &groups, &toolbar) else {
        panic!("a drawer row dropped on the rail adds its tool")
    };
    assert_eq!((tool.id, at), (COPY, DropAt::new_slot(2)));

    let elsewhere = ToolDropped { drag: ToolDrag::rail(0, 1), at: DropAt::into_slot(1, 0) };
    let Some(ToolbarEdit::Move { tool, at, .. }) = drop_edit(&elsewhere, &groups, &toolbar) else {
        panic!("a rail tool dropped on another slot moves")
    };
    assert_eq!((tool, at), (REDO, DropAt::into_slot(1, 0)));

    let own_slot = ToolDropped { drag: ToolDrag::rail(0, 1), at: DropAt::into_slot(0, 0) };
    assert!(drop_edit(&own_slot, &groups, &toolbar).is_none(), "a tool dropped on its own slot stays");

    let gone = ToolDropped { drag: ToolDrag::drawer(3, 0), at: DropAt::new_slot(0) };
    assert!(drop_edit(&gone, &groups, &toolbar).is_none(), "a drag whose row is gone asks for nothing");
}
