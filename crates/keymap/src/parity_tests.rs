//! PhotoCraft parity in the keymap.

use egui::{Event, InputState, Key as EguiKey, Modifiers};

use crate::chord::{Chord, Key, Mods, Platform};
use crate::egui::Dispatcher;
use crate::keymap::{Bindable, Binding, Keymap, Kind, Lookup};
use crate::layout::Layout;
use crate::scope::Active;
use crate::{Scope, Scopes};

fn tool(id: &str, group: &str, key: &str) -> Bindable {
    Bindable::new(id, id, Scope::app()).with_kind(Kind::Tool { group: Some(group.to_owned()) }).with_default(Binding::parse(key).unwrap())
}

fn command(id: &str, keys: &[&str]) -> Bindable {
    keys.iter().fold(Bindable::new(id, id, Scope::app()), |b, k| b.with_default(Binding::parse(k).unwrap()))
}

fn press(key: EguiKey, modifiers: Modifiers) -> Event {
    Event::Key { key, physical_key: Some(key), pressed: true, repeat: false, modifiers }
}

#[test]
fn tools_share_keys_when_allowed() {
    let catalog = || vec![tool("tool.move", "a", "V"), tool("tool.artboard", "b", "V")];
    let km = Keymap::new(catalog(), Scopes::new(), Platform::Windows);
    assert!(!km.shares_tool_keys());
    assert_eq!(km.conflicts().len(), 1);
    let v = Chord::plain(Key::V);
    assert!(matches!(km.lookup(&v, false, &Active::NONE), Lookup::Run(_)));

    let km = Keymap::new(catalog(), Scopes::new(), Platform::Windows).with_shared_tool_keys(true);
    assert!(km.shares_tool_keys());
    assert!(km.conflicts().is_empty());
    let Lookup::Cycle(cycle) = km.lookup(&v, false, &Active::NONE) else { panic!("V should cycle") };
    assert_eq!(cycle.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(), ["tool.move", "tool.artboard"]);
}

#[test]
fn delete_and_backspace_match_each_other() {
    let km = |key: &str| Keymap::new(vec![command("edit.clear", &[key])], Scopes::new(), Platform::Windows);
    let run = |d: Dispatcher, bound: &str, key: EguiKey| {
        let mut input = InputState::default();
        input.events = vec![press(key, Modifiers::NONE)];
        d.clone().dispatch(&mut input, &km(bound), &Active::NONE).len()
    };
    assert_eq!(run(Dispatcher::new(), "Backspace", EguiKey::Delete), 1);
    assert_eq!(run(Dispatcher::new(), "Delete", EguiKey::Backspace), 1);
    assert_eq!(run(Dispatcher::new().with_delete_equivalence(false), "Backspace", EguiKey::Delete), 0);
    assert_eq!(run(Dispatcher::new().with_delete_equivalence(false), "Delete", EguiKey::Delete), 1);
}

#[test]
fn clipboard_chords_pair_with_their_events() {
    use crate::clipboard_presses;
    let cmd = Modifiers::COMMAND;
    let mut input = InputState::default();
    input.modifiers = cmd;
    input.events = vec![press(EguiKey::V, cmd), Event::Paste("hi".into()), Event::Copy];
    let got = clipboard_presses(&input, &Layout::new(), Platform::Windows);
    assert_eq!(got.len(), 2);
    assert_eq!((got[0].press, got[0].event, got[0].pasted.as_deref()), (Some(0), Some(1), Some("hi")));
    assert_eq!((got[1].press, got[1].event), (None, Some(2)));
    assert_eq!(got[1].chord, Chord::new(Mods::CMD, Key::C));
}
