//! `PatternFillEditor` and the Pattern Fill dialog's form through the fixture.

use craft_behavior::EditEvent;
use craft_draw::ui::{PatternColors, PatternFillDialog, PatternFillEditor, PatternFillOptions, PatternSwatch};
use craft_theme::Srgba;
use craft_ui::contract::Bind;
use craft_ui::dialog::{DialogResult, DialogStack, StackDialog as _};
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers, vec2};
use std::cell::RefCell;
use std::rc::Rc;

/// A scale typed in the dialog, as text and as the number it gives.
const TYPED_SCALE_TEXT: &str = "250";
const TYPED_SCALE: f64 = 250.0;

fn swatches() -> Vec<PatternSwatch> {
    ["25 %", "50 %", "Bubbles"].iter().map(|name| PatternSwatch::new(name.to_lowercase(), *name)).collect()
}

/// An editor over options held by the test, and every event it reported.
struct EditorRig {
    options: PatternFillOptions,
    events: Vec<EditEvent<PatternFillOptions>>,
}

fn editor(options: PatternFillOptions) -> (Fixture, Rc<RefCell<EditorRig>>) {
    let state = Rc::new(RefCell::new(EditorRig { options, events: Vec::new() }));
    let shown = Rc::clone(&state);
    let library = swatches();
    let mut fixture = Fixture::new(move |ui| {
        let mut rig = shown.borrow_mut();
        let mut options = rig.options.clone();
        let out = PatternFillEditor::new("Pattern fill", Bind::bound(&mut options), &library).thumbnails(|cx, rect, _| cx.checker(rect)).show(ui);
        rig.options = options;
        rig.events.extend(out.event);
    });
    fixture.room(vec2(480.0, 640.0));
    fixture.settle();
    (fixture, state)
}

fn type_into(fixture: &mut Fixture, name: &str, text: &str) {
    let target = fixture.by_label(name).unwrap_or_else(|| panic!("{name} is shown"));
    fixture.click(&target);
    fixture.key(Key::A, Modifiers::COMMAND);
    fixture.type_text(text);
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.settle();
}

/// Typing 2000 in Scale gives 1000, 0 gives 1; Snap to Origin sets the phase to (0, 0).
#[trial]
fn scale_stays_in_range_and_snap_to_origin_clears_the_phase() {
    let mut options = PatternFillOptions::bitmap("bubbles");
    options.phase = [12.0, 7.0];
    let (mut fixture, state) = editor(options);

    type_into(&mut fixture, "Scale", "2000");
    assert_eq!(state.borrow().options.scale, *PatternFillOptions::SCALE.end());
    type_into(&mut fixture, "Scale", "0");
    assert_eq!(state.borrow().options.scale, *PatternFillOptions::SCALE.start());

    state.borrow_mut().events.clear();
    let snap = fixture.by_label("Snap to Origin").expect("the button");
    fixture.click(&snap);
    fixture.settle();
    assert_eq!(state.borrow().options.phase, [0.0, 0.0]);
    assert!(matches!(state.borrow().events.as_slice(), [EditEvent::Set { .. }]), "{:?}", state.borrow().events);
}

/// With `colors` the editor shows the swatch grid and Foreground and Background chips, no Scale or Angle; a tile click
/// sets the pattern and keeps the colours.
#[trial]
fn preset_mode_shows_the_colours_and_a_click_picks_a_tile() {
    let colors = PatternColors::new(Srgba::BLACK, Srgba::WHITE);
    let (mut fixture, state) = editor(PatternFillOptions::preset("25 %".to_lowercase(), colors));
    assert!(fixture.by_label("Foreground").is_some());
    assert!(fixture.by_label("Background").is_some());
    assert!(fixture.by_label("Scale").is_none());
    assert!(fixture.by_label("Angle").is_none());

    let tile = fixture.by_label("Bubbles").expect("a tile");
    fixture.click(&tile);
    fixture.settle();
    let options = state.borrow().options.clone();
    assert_eq!((options.pattern.as_str(), options.colors), ("bubbles", Some(colors)));
}

/// A host of one dialog.
struct DialogRig {
    dialog: PatternFillDialog,
    result: Option<DialogResult<PatternFillOptions>>,
    saw_changed: bool,
}

fn dialog(options: PatternFillOptions) -> (Fixture, Rc<RefCell<DialogRig>>) {
    let dialog = PatternFillDialog::new(options, swatches()).thumbnails(|cx, rect, _| cx.checker(rect));
    let state = Rc::new(RefCell::new(DialogRig { dialog, result: None, saw_changed: false }));
    let shown = Rc::clone(&state);
    let mut fixture = Fixture::new(move |ui| {
        let mut rig = shown.borrow_mut();
        let mut stack = DialogStack::default();
        stack.push(rig.dialog.id(), None);
        if let Some(result) = rig.dialog.show(ui, &stack) {
            rig.result = Some(result);
        }
        rig.saw_changed |= rig.dialog.changed();
    });
    fixture.room(vec2(640.0, 640.0));
    fixture.settle();
    (fixture, state)
}

// A Scale edit moves `current()` and sets `changed()` in that pass; Cancel answers `DialogResult::Cancel`; OK without a change answers the opening values and `unchanged()` is true.
#[trial]
fn the_dialog_answers_live_values_cancel_and_an_unchanged_ok() {
    {
        let (mut fixture, state) = dialog(PatternFillOptions::bitmap("bubbles"));
        assert!(!state.borrow().saw_changed);
        type_into(&mut fixture, "Scale", TYPED_SCALE_TEXT);
        assert_eq!(state.borrow().dialog.current().scale, TYPED_SCALE);
        assert!(state.borrow().saw_changed);
        assert!(!state.borrow().dialog.unchanged());
        assert!(state.borrow().result.is_none(), "the dialog stays open until it is answered");

        let cancel = fixture.by_label("Cancel").expect("the button");
        fixture.click(&cancel);
        fixture.settle();
        assert_eq!(state.borrow().result, Some(DialogResult::Cancel));
    }
    {
        let opening = PatternFillOptions::bitmap("bubbles");
        let (mut fixture, state) = dialog(opening.clone());
        let ok = fixture.by_label("OK").expect("the button");
        fixture.click(&ok);
        fixture.settle();
        assert_eq!(state.borrow().result, Some(DialogResult::Ok(opening)));
        assert!(state.borrow().dialog.unchanged());
        assert!(!state.borrow().saw_changed);
    }
}
