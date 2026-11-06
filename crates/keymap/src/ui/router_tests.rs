//! Key routing.

use craft_ui::behavior::{ClaimedKey, KeyClaim};
use std::cell::Cell;

use craft_ui::menu::{Menu, MenuEntry, MenuItem};
use craft_ui::popover::{Popover, PopoverAnchor, PopoverClose};
use egui::{Context, Event, Id, Key as EguiKey, Modifiers, Popup, RawInput};

use super::{KeyGate, KeyRouter};
use crate::egui::Dispatcher;
use crate::{Active, Bindable, Binding, Keymap, Platform, Scope, Scopes};

const CTRL: Modifiers = Modifiers { ctrl: true, command: true, ..Modifiers::NONE };

fn keymap() -> Keymap {
    let app = |id: &str, key: &str| Bindable::new(id, id, Scope::app()).with_default(Binding::parse(key).unwrap());
    let catalog = vec![
        app("nudge.right", "Right"),
        app("edit.clear", "Delete"),
        app("file.save", "Cmd+S"),
        app("file.new", "Cmd+N"),
        app("edit.copy", "Cmd+C"),
        app("tool.bold", "Ctrl+B"),
        app("view.zoomIn", "Cmd+="),
        app("tool.brush", "B"),
        app("tool.marquee", "M"),
        app("view.refresh", "F7"),
        app("window.nextDocument", "Ctrl+Tab"),
    ];
    Keymap::new(catalog, Scopes::new(), Platform::Linux)
}

fn press(key: EguiKey, modifiers: Modifiers) -> Event {
    Event::Key { key, physical_key: Some(key), pressed: true, repeat: false, modifiers }
}

/// Runs a pass and drops its texture uploads, which `Context` insists be applied.
fn frame(ctx: &Context, input: RawInput, run: impl FnMut(&mut egui::Ui)) {
    ctx.run_ui(input, run).textures_delta.clear();
}

fn raw(events: Vec<Event>) -> RawInput {
    RawInput { events, ..RawInput::default() }
}

/// Routes one frame of `events` and returns the ids that ran and the key presses left in egui's
/// input. `before` runs first in the frame and `ui` draws the frame's widgets after the routing.
fn route(
    ctx: &Context,
    events: Vec<Event>,
    gate: &KeyGate,
    before: impl FnMut(&Context),
    ui: impl FnMut(&mut egui::Ui),
) -> (Vec<String>, Vec<EguiKey>) {
    route_with(ctx, &keymap(), events, || *gate, |_| {}, before, ui)
}

/// [`route`] with the keymap, a gate asked after every command, and `after` run on each command.
fn route_with<'g>(
    ctx: &Context,
    km: &Keymap,
    events: Vec<Event>,
    gate: impl FnMut() -> KeyGate<'g>,
    mut after: impl FnMut(&str),
    mut before: impl FnMut(&Context),
    mut ui: impl FnMut(&mut egui::Ui),
) -> (Vec<String>, Vec<EguiKey>) {
    let mut router = KeyRouter::new(Dispatcher::new());
    let mut out = (Vec::new(), Vec::new());
    let mut gate = Some(gate);
    frame(ctx, raw(events), |root| {
        before(root.ctx());
        router.route(root.ctx(), km, &Active::NONE, gate.take().expect("one pass"), |hit| {
            after(&hit.command.id);
            out.0.push(hit.command.id);
        });
        out.1 = root
            .ctx()
            .input(|i| i.events.iter().filter_map(|e| if let Event::Key { key, pressed: true, .. } = e { Some(*key) } else { None }).collect());
        ui(root);
    });
    out
}

#[test]
fn a_focused_slider_keeps_arrows_and_leaves_command_chords() {
    let ctx = Context::default();
    let mut value = 0.5;
    let mut id = Id::NULL;
    frame(&ctx, raw(vec![]), |ui| {
        let slider = ui.add(egui::Slider::new(&mut value, 0.0..=1.0));
        slider.request_focus();
        id = slider.id;
    });
    let events = vec![press(EguiKey::ArrowRight, Modifiers::NONE), press(EguiKey::S, CTRL), press(EguiKey::Delete, Modifiers::NONE)];
    let claim = |ctx: &Context| KeyClaim::claim(ctx, id, &[ClaimedKey::exact(Modifiers::NONE, EguiKey::Delete)]);
    let draw = |ui: &mut egui::Ui| {
        ui.ctx().request_repaint();
    };
    let (ran, left) = route(&ctx, events, &KeyGate::default(), claim, draw);
    assert_eq!(ran, ["file.save"]);
    assert_eq!(left, [EguiKey::ArrowRight, EguiKey::Delete]);
}

#[test]
fn dialogs_run_only_the_in_dialog_commands() {
    let ctx = Context::default();
    let gate = KeyGate { dialog: true, in_dialogs: &["view.zoomIn"], ..KeyGate::default() };
    let events = vec![press(EguiKey::Equals, CTRL), press(EguiKey::B, Modifiers::NONE)];
    let (ran, left) = route(&ctx, events, &gate, |_| {}, |_| {});
    assert_eq!(ran, ["view.zoomIn"]);
    assert_eq!(left, [EguiKey::B]);
}

#[test]
fn an_open_menu_owns_every_key() {
    let ctx = Context::default();
    let entries = [MenuEntry::from(MenuItem::new("Item", 1))];
    let mut first = true;
    let mut open_menu = |ui: &mut egui::Ui| {
        let button = ui.button("Open");
        let popup = Popup::menu(&button).open_memory(first.then_some(egui::SetOpenCommand::Toggle));
        first = false;
        let shown = Menu::new(&entries).show(ui, popup);
        assert!(shown.open);
    };
    frame(&ctx, raw(vec![]), &mut open_menu);
    let events = vec![press(EguiKey::M, Modifiers::NONE), press(EguiKey::F7, Modifiers::NONE), press(EguiKey::S, CTRL)];
    let (ran, left) = route(&ctx, events, &KeyGate::default(), |_| {}, &mut open_menu);
    assert!(ran.is_empty());
    assert_eq!(left, [EguiKey::M, EguiKey::F7, EguiKey::S]);
}

/// Draws a popover that stays open, closing by `close`'s rules.
fn popover(ui: &mut egui::Ui, open: &mut bool, close: PopoverClose) {
    let chip = ui.button("Open");
    Popover::new("p", PopoverAnchor::Point(chip.rect.left_bottom())).close(close).label("P").show(ui, open, |ui, _| ui.label("In the popover"));
}

#[test]
fn a_popover_owns_the_keys_but_a_non_modal_one_lets_command_chords_through() {
    let events = || vec![press(EguiKey::B, Modifiers::NONE), press(EguiKey::S, CTRL)];
    for (close, ran, left) in [
        (PopoverClose::default(), vec![], vec![EguiKey::B, EguiKey::S]),
        (PopoverClose::default().outside(false), vec!["file.save"], vec![EguiKey::B]),
    ] {
        let ctx = Context::default();
        let mut open = true;
        frame(&ctx, raw(vec![]), |ui| popover(ui, &mut open, close));
        let (got_ran, got_left) = route(&ctx, events(), &KeyGate::default(), |_| {}, |ui| popover(ui, &mut open, close));
        assert_eq!((got_ran, got_left), (ran.iter().map(|s| s.to_string()).collect::<Vec<_>>(), left));
    }
}

#[test]
fn a_dialog_runs_nothing_while_a_text_field_has_focus() {
    let ctx = Context::default();
    let gate = KeyGate { dialog: true, in_dialogs: &["view.zoomIn"], ..KeyGate::default() };
    let mut text = String::new();
    frame(&ctx, raw(vec![]), |ui| ui.text_edit_singleline(&mut text).request_focus());
    let (ran, left) = route(
        &ctx,
        vec![press(EguiKey::Equals, CTRL)],
        &gate,
        |_| {},
        |ui| {
            ui.text_edit_singleline(&mut text);
        },
    );
    assert!(ran.is_empty());
    assert_eq!(left, [EguiKey::Equals]);
}

#[test]
fn routing_stops_at_the_command_that_opens_a_dialog() {
    let ctx = Context::default();
    let dialog = Cell::new(false);
    let gate = || KeyGate { dialog: dialog.get(), in_dialogs: &["view.zoomIn"], ..KeyGate::default() };
    let events = vec![press(EguiKey::N, CTRL), press(EguiKey::B, Modifiers::NONE), press(EguiKey::Equals, CTRL)];
    let (ran, left) = route_with(&ctx, &keymap(), events, gate, |id| dialog.set(id == "file.new"), |_| {}, |_| {});
    assert_eq!(ran, ["file.new"]);
    assert_eq!(left, [EguiKey::B, EguiKey::Equals]);
}

#[test]
fn a_text_field_keeps_the_clipboard_chords_and_a_mac_control_key() {
    let ctx = Context::default();
    let mut text = String::new();
    frame(&ctx, raw(vec![]), |ui| ui.text_edit_singleline(&mut text).request_focus());
    let events = vec![press(EguiKey::C, CTRL), press(EguiKey::S, CTRL)];
    let draw = |ui: &mut egui::Ui| {
        ui.text_edit_singleline(&mut text);
    };
    let (ran, left) = route(&ctx, events.clone(), &KeyGate::default(), |_| {}, draw);
    assert_eq!(ran, ["file.save"]);
    assert_eq!(left, [EguiKey::C]);

    // On a Mac Control+B is Cocoa's "back one character": a text field keeps it.
    let mac = Keymap::new(
        vec![Bindable::new("tool.bold", "tool.bold", Scope::app()).with_default(Binding::parse("Ctrl+B").unwrap())],
        Scopes::new(),
        Platform::Mac,
    );
    let control = Modifiers { ctrl: true, ..Modifiers::NONE };
    let (ran, left) = route_with(
        &ctx,
        &mac,
        vec![press(EguiKey::B, control)],
        KeyGate::default,
        |_| {},
        |_| {},
        |ui| {
            ui.text_edit_singleline(&mut text);
        },
    );
    assert!(ran.is_empty());
    assert_eq!(left, [EguiKey::B]);
}

#[test]
fn ctrl_tab_runs_whatever_has_focus() {
    let ctx = Context::default();
    let mut text = String::new();
    frame(&ctx, raw(vec![]), |ui| ui.text_edit_singleline(&mut text).request_focus());
    let (ran, left) = route(
        &ctx,
        vec![press(EguiKey::Tab, CTRL), press(EguiKey::B, Modifiers::NONE)],
        &KeyGate::default(),
        |_| {},
        |ui| {
            ui.text_edit_singleline(&mut text);
        },
    );
    assert_eq!(ran, ["window.nextDocument"]);
    assert_eq!(left, [EguiKey::B]);
}

#[test]
fn the_menu_bar_holding_the_keyboard_keeps_letters_and_a_command_chord_leaves_it() {
    use craft_ui::menu::{MenuBar, MenuItem as Title};
    let titles = [Title::submenu("File", vec![MenuEntry::from(MenuItem::new("New", 1))])];
    let bar = |ui: &mut egui::Ui| {
        let _ = MenuBar::new(&titles).show(ui);
    };
    let alt = |pressed: bool| Event::Key {
        key: EguiKey::AltLeft,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: if pressed { Modifiers::ALT } else { Modifiers::NONE },
    };
    for (events, ran, left) in [
        (vec![press(EguiKey::B, Modifiers::NONE)], vec![], vec![EguiKey::B]),
        (vec![press(EguiKey::B, Modifiers::NONE), press(EguiKey::S, CTRL)], vec!["tool.brush", "file.save"], vec![]),
    ] {
        let ctx = Context::default();
        for events in [vec![alt(true)], vec![alt(false)], vec![]] {
            frame(&ctx, raw(events), bar);
        }
        assert!(craft_ui::menu::bar::is_focused(&ctx));
        let (got_ran, got_left) = route(&ctx, events, &KeyGate::default(), |_| {}, bar);
        assert_eq!((got_ran, got_left), (ran.iter().map(|s| s.to_string()).collect::<Vec<_>>(), left));
    }
}

#[test]
fn alt_on_an_opted_in_chord_hits_the_variant_in_press_order() {
    let command = |id: &str, key: &str| Bindable::new(id, id, Scope::app()).with_default(Binding::parse(key).unwrap());
    let catalog = vec![
        command("adjust.levels", "Cmd+L").with_alt_variant(),
        command("adjust.hue", "Cmd+U"),
        command("adjust.curves", "Cmd+M").with_alt_variant(),
        command("adjust.curvesOwn", "Cmd+Alt+M"),
        command("adjust.balance", "Cmd+B").with_alt_variant(),
        command("file.save", "Cmd+S"),
    ];
    let km = Keymap::new(catalog, Scopes::new(), Platform::Linux);
    let alt = Modifiers { alt: true, ..CTRL };
    let ctx = Context::default();
    let events = vec![press(EguiKey::L, alt), press(EguiKey::S, CTRL), press(EguiKey::U, alt), press(EguiKey::M, alt), press(EguiKey::B, CTRL)];
    let mut ran = Vec::new();
    let mut router = KeyRouter::new(Dispatcher::new());
    frame(&ctx, raw(events), |root| {
        router.route(root.ctx(), &km, &Active::NONE, KeyGate::default, |hit| ran.push((hit.command.id, hit.alt_variant)));
    });
    // Hue did not opt in, and Cmd+Alt+M is bound to its own command. The last press uses a key not
    // pressed before: egui reports a second press of a key still down as a repeat.
    assert_eq!(
        ran,
        [
            ("adjust.levels".to_string(), true),
            ("file.save".to_string(), false),
            ("adjust.curvesOwn".to_string(), false),
            ("adjust.balance".to_string(), false)
        ]
    );
}
