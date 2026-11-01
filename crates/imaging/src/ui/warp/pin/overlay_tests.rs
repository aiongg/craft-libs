//! Tests of the pin overlay and the mesh painter (task TR-T12).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Shift-click adds a pin to the selection and takes it out; dragging a selected pin moves every
// selected pin as one Pins gesture.
#[test]
#[ignore = "skeleton"]
fn shift_click_toggles_the_selection_and_a_drag_moves_every_selected_pin() {
    todo!()
}

// ⌥-drag within `transform.pin_turn` of a selected pin, not on it, reports the pin with its angle
// set to the drag's angle.
#[test]
#[ignore = "skeleton"]
fn alt_drag_near_a_selected_pin_turns_it_and_fixes_its_rotation() {
    todo!()
}

// Delete reports the pins without the selected ones; ⌘A reports a selection of every pin.
#[test]
#[ignore = "skeleton"]
fn delete_removes_the_selected_pins_and_select_all_selects_every_pin() {
    todo!()
}

// Insert places a pin (at the pointer's rest position, else the mesh centre), Page keys pick,
// arrows move the picked pin, Delete removes it.
#[test]
#[ignore = "skeleton"]
fn the_keyboard_places_picks_moves_and_removes_pins() {
    todo!()
}

// Pins draw in depth order; the selected pin shows its core and, when fixed, its turn ring.
#[test]
#[ignore = "skeleton"]
fn pins_draw_in_depth_order_with_the_selected_core_and_turn_ring() {
    todo!()
}

// `paint_mesh` draws each edge once in the mesh style; Show Mesh off draws no mesh line.
#[test]
#[ignore = "skeleton"]
fn the_mesh_painter_draws_each_edge_once() {
    todo!()
}

// AccessKit: a group with one graphics object per pin named "Pin 2 of 5" with value "x, y".
#[test]
#[ignore = "skeleton"]
fn every_pin_is_named_and_valued_for_assistive_technology() {
    todo!()
}
