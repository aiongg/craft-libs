//! The signature pad's stored and drawn forms.

use craft_review::signature::{SavedSignature, SignatureDraft, SignatureTab};
use craft_review::ui::signature::DrawPad;
use craft_ui::contract::{Bind, Role, Show as _};
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers, vec2};
use std::cell::RefCell;
use std::rc::Rc;

/// Apply returns what the current tab holds: typed text, the strokes or the picked image.
#[trial]
fn apply_returns_the_current_tab() {
    let mut draft = SignatureDraft::new(false, "Ana Reyes");
    draft.tab = SignatureTab::Type;
    assert!(matches!(draft.saved(), Some(SavedSignature::Typed(text)) if text == "Ana Reyes"));
    draft.tab = SignatureTab::Draw;
    draft.strokes = vec![vec![[0.1, 0.2], [0.3, 0.4]]];
    assert!(matches!(draft.saved(), Some(SavedSignature::Drawn(strokes)) if strokes.len() == 1));
}

/// Apply waits for content: an empty draft is not ready.
#[trial]
fn apply_waits_for_content() {
    let mut draft = SignatureDraft::new(false, "");
    assert!(!draft.ready());
    draft.tab = SignatureTab::Draw;
    assert!(!draft.ready());
    draft.strokes = vec![vec![[0.1, 0.2]]];
    assert!(!draft.ready(), "a bare click is not a stroke");
    draft.strokes = vec![vec![[0.1, 0.2], [0.3, 0.4]]];
    assert!(draft.ready());
}

/// Command+Z in the draw pad removes the last stroke and leaves the ones before it.
#[trial]
fn undo_removes_the_last_stroke() {
    let strokes = Rc::new(RefCell::new(Vec::new()));
    let shown = strokes.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut strokes = shown.borrow_mut();
        DrawPad::new("Draw", Bind::bound(&mut strokes)).show(ui);
    });
    let pad = fixture.by_role(Role::IMAGE).into_iter().next().expect("the pad");
    fixture.drag(&pad, vec2(40.0, 10.0));
    fixture.drag(&pad, vec2(-30.0, 20.0));
    assert_eq!(strokes.borrow().len(), 2, "two strokes drawn");
    fixture.key(Key::Z, Modifiers::COMMAND);
    assert_eq!(strokes.borrow().len(), 1, "Command+Z removes the last stroke");
}
