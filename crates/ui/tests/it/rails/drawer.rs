//! The drawer's own logic: how an edit changes the toolbar and what a drop on the rail asks for.

use craft_ui::rails::{
    DrawerGroup, DropAt, LastUsed, RailTool, ToolDrag, ToolRail, ToolSlot, ToolbarDrawer, ToolbarEdit, apply_toolbar_edit, drop_edit,
};
use craft_ui::runtime::commands::EditCommands;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers, Rect, pos2, vec2};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

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
    let edit = ToolbarEdit::add(tool(CUT), DropAt::new_slot(1));
    assert_eq!(layout(&apply_toolbar_edit(&slots, &edit, &[])), [(0, vec![UNDO]), (6, vec![CUT]), (5, vec![REDO])]);
}

#[trial]
fn a_tool_moved_down_its_own_slot_lands_at_the_index_after_it_leaves() {
    let slots = [slot(0, &[UNDO, REDO, CUT])];
    let edit = ToolbarEdit::move_to(UNDO, DropAt::into_slot(0, 3));
    assert_eq!(layout(&apply_toolbar_edit(&slots, &edit, &[])), [(0, vec![REDO, CUT, UNDO])], "the removal shifts the index");
}

#[trial]
fn a_tool_moved_out_of_its_only_slot_takes_the_slot_with_it() {
    let slots = [slot(0, &[UNDO]), slot(1, &[REDO])];
    let edit = ToolbarEdit::move_to(UNDO, DropAt::into_slot(1, 0));
    assert_eq!(layout(&apply_toolbar_edit(&slots, &edit, &[])), [(1, vec![UNDO, REDO])], "the slot the tool left is gone");
}

#[trial]
fn removing_the_last_tool_of_a_slot_removes_the_slot_and_reset_restores_the_defaults() {
    let defaults = [slot(0, &[UNDO, REDO])];
    let slots = [slot(0, &[UNDO]), slot(1, &[REDO])];
    let removed = apply_toolbar_edit(&slots, &ToolbarEdit::remove(REDO), &defaults);
    assert_eq!(layout(&removed), [(0, vec![UNDO])]);
    assert_eq!(layout(&apply_toolbar_edit(&removed, &ToolbarEdit::Reset, &defaults)), layout(&defaults));
}

#[trial]
fn a_move_of_a_tool_that_is_not_on_the_toolbar_changes_nothing() {
    let slots = [slot(0, &[UNDO])];
    let edit = ToolbarEdit::move_to(COPY, DropAt::new_slot(0));
    assert_eq!(layout(&apply_toolbar_edit(&slots, &edit, &[])), layout(&slots));
}

#[trial]
fn a_drop_on_the_rail_asks_for_the_edit_its_drag_implies() {
    let groups = [DrawerGroup::new("Edit", vec![tool(CUT), tool(COPY)])];
    let toolbar = [slot(0, &[UNDO, REDO]), slot(1, &[CUT])];

    let Some(ToolbarEdit::Add { tool, at, .. }) = drop_edit(ToolDrag::drawer(0, 1), DropAt::new_slot(2), &groups, &toolbar) else {
        panic!("a drawer row dropped on the rail adds its tool")
    };
    assert_eq!((tool.id, at), (COPY, DropAt::new_slot(2)));

    let Some(ToolbarEdit::Move { tool, at, .. }) = drop_edit(ToolDrag::rail(0, 1), DropAt::into_slot(1, 0), &groups, &toolbar) else {
        panic!("a rail tool dropped on another slot moves")
    };
    assert_eq!((tool, at), (REDO, DropAt::into_slot(1, 0)));

    assert!(drop_edit(ToolDrag::rail(0, 1), DropAt::into_slot(0, 0), &groups, &toolbar).is_none(), "a tool dropped on its own slot stays");
    assert!(drop_edit(ToolDrag::drawer(3, 0), DropAt::new_slot(0), &groups, &toolbar).is_none(), "a drag whose row is gone asks for nothing");
}

/// An open drawer beside a rail at the left edge, over two groups of two tools each and an empty toolbar. `closed` is
/// set when the drawer reports it closed (the host then stops showing it); `edits` collects the edits it reports.
fn drawer() -> (Fixture, Rc<Cell<bool>>, Rc<RefCell<Vec<ToolbarEdit<u8>>>>) {
    let closed = Rc::new(Cell::new(false));
    let edits = Rc::new(RefCell::new(Vec::new()));
    let (seen_closed, seen_edits) = (closed.clone(), edits.clone());
    let mut fixture = Fixture::new(move |ui| {
        if seen_closed.get() {
            return;
        }
        let groups = [DrawerGroup::new("History", vec![tool(UNDO), tool(REDO)]), DrawerGroup::new("Clipboard", vec![tool(CUT), tool(COPY)])];
        let rail = Rect::from_min_size(pos2(0.0, 0.0), vec2(40.0, 400.0));
        let out = ToolbarDrawer::new(rail, &groups, &[], None).show(ui);
        seen_edits.borrow_mut().extend(out.edits);
        seen_closed.set(out.closed);
    });
    fixture.room(vec2(800.0, 600.0));
    fixture.settle();
    (fixture, closed, edits)
}

#[trial]
fn a_press_outside_the_drawer_closes_it() {
    let (mut fixture, closed, _) = drawer();
    assert!(fixture.by_label("Undo").is_some(), "the drawer lists its tools");
    fixture.click_at(pos2(780.0, 580.0));
    assert!(closed.get(), "a press away from the drawer and the rail closes it");
}

#[trial]
fn down_from_the_last_tool_of_a_group_lands_on_the_first_tool_of_the_next() {
    let (mut fixture, _, edits) = drawer();
    let redo = fixture.by_label("Redo").expect("the last tool of the first group is listed");
    fixture.click(&redo);
    fixture.key(Key::ArrowDown, Modifiers::NONE);
    fixture.key(Key::Enter, Modifiers::NONE);
    let added: Vec<u8> =
        edits.borrow().iter().filter_map(|edit| if let ToolbarEdit::Add { tool, .. } = edit { Some(tool.id) } else { None }).collect();
    assert_eq!(added, [CUT], "the second group's heading row is skipped");
}

#[trial]
fn a_drawer_row_dragged_onto_the_rail_drops_as_a_drawer_tool() {
    let edits = Rc::new(RefCell::new(Vec::new()));
    let seen = edits.clone();
    let rail_at = Rc::new(Cell::new(Rect::NOTHING));
    let rail_seen = rail_at.clone();
    let mut fixture = Fixture::new(move |ui| {
        let groups = [DrawerGroup::new("History", vec![tool(UNDO), tool(REDO)]), DrawerGroup::new("Clipboard", vec![tool(CUT), tool(COPY)])];
        let toolbar = [slot(0, &[UNDO])];
        let last_used = LastUsed::default();
        let rail = ToolRail::new("Tools", &toolbar, &UNDO, &last_used).editing().show_rail(ui, |_, _| {});
        rail_seen.set(rail.response.rect);
        let out = ToolbarDrawer::new(rail.response.rect, &groups, &toolbar, rail.dropped).show(ui);
        seen.borrow_mut().extend(out.edits);
    });
    fixture.room(vec2(800.0, 600.0));
    fixture.settle();
    let row = fixture.by_label("Cut").expect("the drawer lists Cut");
    let by = rail_at.get().center() - row.center();
    fixture.drag(&row, by);
    let added: Vec<u8> =
        edits.borrow().iter().filter_map(|edit| if let ToolbarEdit::Add { tool, .. } = edit { Some(tool.id) } else { None }).collect();
    assert_eq!(added, [CUT], "the drop on the rail adds the dragged drawer tool");
}
