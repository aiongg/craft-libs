//! `Disclosure`: toggles its body by pointer and key and reports the expanded state.

use craft_ui::contract::{Bind, Props as _, Show as _};
use craft_ui::controls::disclosure::Disclosure;
use craft_ui::testing::focus::{egui_focused, key_release};
use craft_ui::testing::input::key_press;
use craft_ui::testing::pointer::click;
use craft_ui::testing::{Frames, trial};
use egui::{Id, Key, Modifiers, Rect, Ui};
use std::cell::Cell;

/// A disclosure and the body it opens; the body is shown only while the value is open.
fn frame(ui: &mut Ui, open: &mut bool, at: &Cell<Option<(Rect, Id)>>, body_shown: &Cell<bool>) {
    let response = Disclosure::new(Bind::bound(&mut *open)).label("Details").id_salt("disclosure").show(ui);
    at.set(Some((response.response.rect, response.response.id)));
    body_shown.set(*open);
    if *open {
        ui.label("body");
    }
}

#[trial]
fn a_disclosure_toggles_its_body_by_pointer_and_key() {
    let mut frames = Frames::new();
    let (at, body) = (Cell::new(None), Cell::new(false));
    let mut open = false;
    frames.idle(2, |ui| frame(ui, &mut open, &at, &body));
    let (rect, id) = at.get().expect("the disclosure showed");
    click(&mut frames, rect.center(), &mut |ui| frame(ui, &mut open, &at, &body));
    assert!(open && body.get(), "a click opens the body");
    for (key, expected) in [(Key::Space, false), (Key::Enter, true)] {
        // Tab until the disclosure holds focus, then press the key.
        while egui_focused(&frames.ctx) != Some(id) {
            frames.frame(vec![key_press(Key::Tab, Modifiers::NONE)], |ui| frame(ui, &mut open, &at, &body));
            frames.frame(vec![key_release(Key::Tab, Modifiers::NONE)], |ui| frame(ui, &mut open, &at, &body));
        }
        frames.frame(vec![key_press(key, Modifiers::NONE)], |ui| frame(ui, &mut open, &at, &body));
        frames.frame(vec![key_release(key, Modifiers::NONE)], |ui| frame(ui, &mut open, &at, &body));
        assert_eq!((open, body.get()), (expected, expected), "{key:?} toggles the body");
    }
}

#[trial]
fn a_disclosure_reports_its_expanded_state_to_assistive_technology() {
    let mut frames = Frames::new();
    let (at, body) = (Cell::new(None), Cell::new(false));
    let mut open = false;
    frames.idle(2, |ui| frame(ui, &mut open, &at, &body));
    let (_, id) = at.get().expect("the disclosure showed");
    assert_eq!(frames.tree().expanded(id), Some(false));
    open = true;
    frames.idle(2, |ui| frame(ui, &mut open, &at, &body));
    assert_eq!(frames.tree().expanded(id), Some(true));
}
