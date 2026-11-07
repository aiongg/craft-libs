//! Text AccessKit and the field's visible states: value, `SetValue`, selection, INVALID, selectable text.

use super::kit::Rig;
use craft_ui::contract::{Bind, Props as _, Show as _};
use craft_ui::testing::access::{Declared, NodeKey};
use craft_ui::testing::input::key_press;
use craft_ui::testing::pointer::click;
use craft_ui::testing::text::{caret, run_end, run_node, set_value_request, text_selection_request};
use craft_ui::testing::{Frames, trial};
use craft_ui::text::{NumberField, SelectableText};
use egui::{Event, Key, Modifiers};
use std::cell::{Cell, RefCell};

/// The value a refused commit leaves.
const KEPT: f64 = 1.0;

/// A position maps to the run that holds its byte and the character index in it.
#[trial]
fn a_position_maps_to_the_run_that_holds_its_byte_and_the_character_index_in_it() {
    let owner = craft_ui::testing::focus::named_id("owner");
    let runs = [(0, 0..3, "abc"), (0, 3..7, "d\u{e9}f"), (1, 0..2, "xy")];
    let at = |line, byte| run_end(owner, &runs, craft_behavior::TextPos::new(line, byte));
    assert_eq!(at(0, 1), Some((run_node(owner, 0, 0), 1)));
    assert_eq!(at(0, 6), Some((run_node(owner, 0, 1), 2)), "a byte after a two-byte character");
    assert_eq!(at(0, 7), Some((run_node(owner, 0, 1), 3)), "the end of the line is the end of its last run");
    assert_eq!(at(1, 2), Some((run_node(owner, 1, 0), 2)));
    assert_eq!(at(2, 0), None);
}

/// The node carries the text as its value with a run node under it, declares `SetValue` and `SetTextSelection`,
/// serves a selection by run node and character, and a `SetValue` becomes the field's value.
#[trial]
fn a_text_field_reports_value_and_selection_and_serves_set_value_and_set_text_selection() {
    let mut rig = Rig::new("hello");
    rig.focus();
    rig.idle(1);
    let id = rig.seen.id;
    let tree = rig.frames.tree();
    assert_eq!(tree.value(id), Some("hello"));
    assert_eq!(tree.run_texts(id), ["hello"]);
    assert!(tree.declares(id, Declared::SetValue) && tree.declares(id, Declared::SetTextSelection));

    let run = run_node(id, 0, 0);
    rig.frame(vec![text_selection_request(id, (run, 1), (run, 3))]);
    rig.idle(1);
    assert_eq!(caret(&rig.frames.ctx, rig.widget()), Some(3), "the focus end of the request is the caret");
    let selection = rig.frames.tree().text_selection(id);
    assert_eq!(selection, Some(((NodeKey::of(run), 1), (NodeKey::of(run), 3))), "and the node reports it back");

    rig.frame(vec![set_value_request(id, "world")]);
    rig.idle(2);
    assert_eq!(rig.text, "world", "SetValue replaces the value");
}

/// A rejected draft marks the field invalid until the run ends; Escape restores the value and clears the mark.
#[trial]
fn a_rejected_draft_shows_invalid_until_it_is_restored() {
    let mut frames = Frames::new();
    let value = RefCell::new(Some(1.0f64));
    let seen = Cell::new(egui::Id::NULL);
    let rect = Cell::new(egui::Rect::NOTHING);
    let mut show = |ui: &mut egui::Ui| {
        let out = NumberField::new("Size", Bind::bound(&mut *value.borrow_mut())).id_salt("size").show(ui);
        seen.set(out.response.id);
        rect.set(out.response.rect);
    };
    frames.idle(3, &mut show);
    click(&mut frames, rect.get().center(), &mut show);
    frames.frame(vec![key_press(Key::A, Modifiers::COMMAND)], &mut show);
    frames.frame(vec![Event::Text("x1".into())], &mut show);
    frames.frame(vec![key_press(Key::Enter, Modifiers::NONE)], &mut show);
    frames.frame(vec![], &mut show);
    assert!(frames.tree().invalid(seen.get()), "a refused commit keeps the run open and the field invalid");
    assert_eq!(*value.borrow(), Some(KEPT), "and leaves the value");
    frames.frame(vec![key_press(Key::Escape, Modifiers::NONE)], &mut show);
    frames.frame(vec![], &mut show);
    assert!(!frames.tree().invalid(seen.get()), "Escape restores the value and clears the mark");
}

/// Selectable text takes focus, selects all and copies its text, and edits nothing.
#[trial]
fn selectable_text_selects_and_copies() {
    let mut frames = Frames::new();
    let rect = Cell::new(egui::Rect::NOTHING);
    let mut show = |ui: &mut egui::Ui| {
        let out = SelectableText::new("Note", "read me").id_salt("note").show(ui);
        rect.set(out.response.rect);
    };
    frames.idle(3, &mut show);
    click(&mut frames, rect.get().center(), &mut show);
    frames.frame(vec![key_press(Key::A, Modifiers::COMMAND)], &mut show);
    frames.frame(vec![], &mut show);
    frames.frame(vec![key_press(Key::C, Modifiers::COMMAND)], &mut show);
    assert_eq!(frames.copied(), ["read me"], "Copy puts the selected text on the clipboard");
    frames.frame(vec![Event::Text("!".into())], &mut show);
    frames.frame(vec![key_press(Key::X, Modifiers::COMMAND)], &mut show);
    assert!(frames.copied().is_empty(), "Cut is dropped: the text is read-only");
}
