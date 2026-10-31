//! Context menus from menu-model rows.

use craft_keymap::{Bindable, Binding, Chord, CommandRef, Keymap, Platform, Scope, Scopes};
use craft_ui::menu::{MenuEntry, MenuItem};
use egui::{Context, Event, Key, Modifiers, PointerButton, RawInput, Rect, pos2, vec2};

use crate::{CommandContextMenu, Gate, Item, ItemState, Node, Target};

fn rows() -> Vec<Node> {
    vec![
        Node::from(Item::new("edit.copy", "Copy")),
        Node::from(Item::new("edit.paste", "Paste")),
        Node::Separator,
        Node::submenu("Layer", vec![Node::from(Item::new("layer.duplicate", "Duplicate Layer"))]),
    ]
}

#[test]
// A Copy row gets Cmd+C from the keymap and is disabled when the state says so; separators and submenus carry over.
fn context_rows_take_keymap_shortcuts_and_state() {
    let copy = Binding::parse("Cmd+C").unwrap();
    let keymap = Keymap::new(vec![Bindable::new("edit.copy", "Copy", Scope::app()).with_default(copy)], Scopes::new(), Platform::Mac);
    let state = |c: &CommandRef| ItemState { enabled: c.id != "edit.copy", checked: None };
    let nodes = rows();
    let entries = CommandContextMenu::new(&nodes).keymap(&keymap).state(&state).target(Target::Mac).entries();
    assert_eq!(entries.len(), 4);
    let shortcut = craft_keymap::ui::kbd(&Chord::parse("Cmd+C").unwrap(), Platform::Mac);
    let expected = MenuItem::new("Copy", CommandRef::new("edit.copy")).enabled(false).shortcut(shortcut);
    assert_eq!(entries[0], MenuEntry::from(expected));
    assert!(entries[1].item().is_some_and(MenuItem::is_enabled));
    assert!(matches!(entries[2], MenuEntry::Separator));
    assert_eq!(entries[3].item().and_then(|i| i.children()).map(<[_]>::len), Some(1));
}

/// Runs one frame of a context menu over a button in a fixed place.
struct Frame {
    ctx: Context,
    nodes: Vec<Node>,
    gate: Gate,
}

impl Frame {
    fn run(&self, events: Vec<Event>) -> Option<CommandRef> {
        let mut chosen = None;
        let input = RawInput { screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(400.0, 300.0))), events, ..RawInput::default() };
        let mut output = self.ctx.run_ui(input, |ui| {
            let target = ui.allocate_response(vec2(100.0, 40.0), egui::Sense::click());
            chosen = chosen.take().or(CommandContextMenu::new(&self.nodes).gate(self.gate).show(ui, &target));
        });
        output.textures_delta.clear();
        chosen
    }
}

fn key(key: Key) -> Vec<Event> {
    [true, false].map(|pressed| Event::Key { key, physical_key: None, pressed, repeat: false, modifiers: Modifiers::NONE }).into()
}

#[test]
// A secondary click opens the menu and a click on a row returns its CommandRef; a gate-forbidden row is disabled and returns nothing.
fn context_menu_reports_the_chosen_command() {
    let ctx = Context::default();
    ctx.set_fonts(egui::FontDefinitions::default());
    // A dialog is open: only the built-in commands run, so both rows are forbidden.
    let frame = Frame { ctx, nodes: rows(), gate: Gate { modal: true, ..Gate::default() } };
    let at = pos2(20.0, 10.0);
    let secondary = |pressed| Event::PointerButton { pos: at, button: PointerButton::Secondary, pressed, modifiers: Modifiers::NONE };
    frame.run(vec![]);
    frame.run(vec![Event::PointerMoved(at), secondary(true), secondary(false)]);
    for _ in 0..3 {
        frame.run(vec![]);
    }
    let mut chosen = frame.run(key(Key::ArrowDown));
    chosen = chosen.or(frame.run(key(Key::Enter)));
    assert_eq!(chosen, None, "forbidden rows can't be chosen");

    let open = Frame { gate: Gate::default(), ..frame };
    open.run(vec![Event::PointerMoved(at), secondary(true), secondary(false)]);
    for _ in 0..3 {
        open.run(vec![]);
    }
    let mut chosen = open.run(key(Key::ArrowDown));
    for _ in 0..2 {
        chosen = chosen.or(open.run(vec![]));
    }
    chosen = chosen.or(open.run(key(Key::Enter)));
    assert_eq!(chosen, Some(CommandRef::new("edit.copy")));
}
