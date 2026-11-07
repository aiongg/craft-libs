//! Text cases: paste, IME composition and the editor's lines in the AccessKit tree.

use super::drive::{Plan, settled};
use super::{ANY, Case, Subject};
use crate::testing::fixture::Fixture;
use crate::testing::story::EditKind;
use craft_behavior::Input;
use egui::{Event, ImeEvent, Key, Modifiers};

pub(super) const CASES: &[Case] = &[
    Case::new("paste-once", ANY, paste_lands_once),
    Case::new("ime", ANY, ime_composition_is_guarded),
    Case::new("editor-lines", ANY, editor_lines_reach_accesskit),
];

fn is_text(subject: &Subject) -> bool {
    matches!(subject.probe.input, Input::Text | Input::Editor)
}

/// The text the focused node shows: its value, or its text runs for an editor.
fn shown_text(fixture: &Fixture) -> String {
    let Some(id) = fixture.focused() else { return String::new() };
    let tree = fixture.tree();
    let runs = tree.run_texts(id).join("\n");
    match tree.value(id) {
        Some(value) if runs.is_empty() => value.to_owned(),
        _ => runs,
    }
}

/// The stories whose drive types into the component, focused and editable.
fn typing_fixtures(subject: &Subject) -> Vec<(String, Fixture)> {
    let mut found = Vec::new();
    for story in &subject.stories {
        let (mut fixture, plan) = settled(story, subject.probe);
        if !matches!(plan, Plan::Type(_)) {
            continue;
        }
        fixture.key(Key::Tab, Modifiers::NONE);
        let editable = fixture.focused().is_some()
            && fixture
                .tree()
                .keys()
                .into_iter()
                .any(|key| fixture.tree().data_of(key).is_some_and(|node| !node.is_read_only() && !node.is_disabled()));
        if editable {
            found.push((story.name.to_owned(), fixture));
        }
    }
    found
}

/// A paste event lands once in the focused text part.
fn paste_lands_once(subject: &Subject) -> Result<(), String> {
    if !is_text(subject) {
        return Ok(());
    }
    for (name, mut fixture) in typing_fixtures(subject) {
        fixture.frame(vec![Event::Paste("qzq".to_owned())]);
        fixture.idle(2);
        let count = shown_text(&fixture).matches("qzq").count();
        if count != 1 {
            return Err(format!("{name}: a paste landed {count} times"));
        }
    }
    Ok(())
}

/// Preedit then commit gives the committed text once; Enter and Escape while composing neither submit nor cancel;
/// the caret rect is reported.
fn ime_composition_is_guarded(subject: &Subject) -> Result<(), String> {
    if !is_text(subject) {
        return Ok(());
    }
    for (name, mut fixture) in typing_fixtures(subject) {
        fixture.frame(vec![Event::Ime(ImeEvent::Preedit { text: "kzk".to_owned(), active_range_chars: None })]);
        match fixture.ime() {
            Some(ime) if ime.caret.min.is_finite() && ime.caret.max.is_finite() => {}
            _ => return Err(format!("{name}: no caret rect was reported while composing")),
        }
        fixture.take_outputs();
        fixture.key(Key::Enter, Modifiers::NONE);
        fixture.key(Key::Escape, Modifiers::NONE);
        let kinds: Vec<EditKind> = fixture.take_outputs().into_iter().flat_map(|output| output.edits).collect();
        if kinds.iter().any(|kind| matches!(kind, EditKind::Commit | EditKind::Set | EditKind::Cancel)) {
            return Err(format!("{name}: Enter or Escape while composing gave {kinds:?}"));
        }
        fixture.frame(vec![Event::Ime(ImeEvent::Commit("kzk".to_owned()))]);
        fixture.idle(2);
        let count = shown_text(&fixture).matches("kzk").count();
        if count != 1 {
            return Err(format!("{name}: a composition landed {count} times"));
        }
    }
    Ok(())
}

/// An editor with text reports its lines as AccessKit text runs.
fn editor_lines_reach_accesskit(subject: &Subject) -> Result<(), String> {
    if !matches!(subject.probe.input, Input::Editor) {
        return Ok(());
    }
    for story in &subject.stories {
        let (fixture, _) = settled(story, subject.probe);
        if !fixture.texts().iter().any(|run| !run.text.trim().is_empty()) {
            continue;
        }
        let tree = fixture.tree();
        if !tree.keys().into_iter().any(|key| tree.data_of(key).is_some_and(|node| node.role() == accesskit::Role::TextRun)) {
            return Err(format!("{}: the editor paints text but reports no text run", story.name));
        }
    }
    Ok(())
}
