//! Key routing.

use craft_ui::behavior::{ClaimedKey, KeyClaim};
use craft_ui::menu::{Menu, MenuEntry, MenuItem};
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
    mut before: impl FnMut(&Context),
    mut ui: impl FnMut(&mut egui::Ui),
) -> (Vec<String>, Vec<EguiKey>) {
    let mut router = KeyRouter::new(Dispatcher::new());
    let km = keymap();
    let mut out = (Vec::new(), Vec::new());
    frame(ctx, raw(events), |root| {
        before(root.ctx());
        let hits = router.route(root.ctx(), &km, &Active::NONE, gate);
        out.0 = hits.into_iter().map(|h| h.command.id).collect();
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
fn an_open_menu_keeps_letters_and_a_command_chord_runs() {
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
    let events = vec![press(EguiKey::M, Modifiers::NONE), press(EguiKey::F7, Modifiers::NONE)];
    let (ran, left) = route(&ctx, events, &KeyGate::default(), |_| {}, &mut open_menu);
    assert_eq!(ran, ["view.refresh"]);
    assert_eq!(left, [EguiKey::M]);
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
