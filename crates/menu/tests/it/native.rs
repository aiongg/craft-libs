//! The driver between a platform menu and the app: the gate, the command hook, display-only
//! shortcuts, shortcut recording, clipboard keys and closing an open menu. A fake backend stands
//! in for the platform, so the trials run everywhere.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use craft_menu::{
    Backend, Builtin, ClipboardKeys, CommandRef, Dispatch, Event, Gate, ItemKind, ItemState, LayoutOptions, Menu, MenuBar, MenuRole, NativeMenu,
    Node, apply_state, mac_layout, structure_key,
};
use craft_ui::contract::{Bind, Props as _, Show as _};
use craft_ui::testing::focus::key_release;
use craft_ui::testing::input::key_press;
use craft_ui::testing::pointer::click;
use craft_ui::testing::{Frames, trial};
use craft_ui::text::{SelectableText, TextField};
use egui::{Key, Modifiers};

use crate::sample::{chord, cmd, held, sample, typing};

fn modal() -> Gate {
    Gate::default().modal(true)
}

fn recording() -> Gate {
    Gate::default().recording_shortcut(true)
}

type Shared<T> = Rc<RefCell<Vec<T>>>;

#[derive(Default)]
struct Fake {
    synced: Rc<RefCell<Vec<MenuBar>>>,
    events: Rc<RefCell<Vec<Event>>>,
    tracking: Rc<Cell<bool>>,
    cancels: Rc<Cell<usize>>,
    /// Keeps reporting a menu open after `cancel_tracking`.
    stuck: bool,
    /// What the backend's menu event handler would call.
    dispatch: Rc<RefCell<Option<Dispatch>>>,
}

impl Backend for Fake {
    fn sync(&mut self, bar: &MenuBar) {
        self.synced.borrow_mut().push(bar.clone());
    }
    fn drain(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events.borrow_mut())
    }
    fn tracking(&self) -> bool {
        self.tracking.get()
    }
    fn cancel_tracking(&mut self) {
        self.cancels.set(self.cancels.get() + 1);
        if !self.stuck {
            self.tracking.set(false);
        }
    }
    fn attach(&mut self, dispatch: Dispatch) {
        *self.dispatch.borrow_mut() = Some(dispatch);
    }
}

/// A driver over a fake backend, with what it synced, its pending events and its dispatch.
fn driver() -> (NativeMenu, Shared<MenuBar>, Shared<Event>, Dispatch) {
    let fake = Fake::default();
    let (synced, events, dispatch) = (fake.synced.clone(), fake.events.clone(), fake.dispatch.clone());
    let menu = NativeMenu::new(Box::new(fake));
    let dispatch = dispatch.borrow().clone().expect("NativeMenu::new attaches the dispatch");
    (menu, synced, events, dispatch)
}

fn take_clicks(menu: &mut NativeMenu) -> Vec<CommandRef> {
    let ctx = egui::Context::default();
    let mut clicks = Vec::new();
    let mut out = ctx.run_ui(Default::default(), |ui| clicks = menu.take_clicks(ui.ctx()));
    out.textures_delta.clear();
    clicks
}

/// What egui sees of `events` in a frame of `frames` after the menu has been polled: the window host's input
/// filter has dropped the closing click and added the key equivalents, and the runtime has taken the keys it owns.
/// The menu is polled again after the frame, as in an app, so that a close the filter asked for reaches the backend.
fn through(menu: &mut NativeMenu, frames: &mut Frames, events: Vec<egui::Event>) -> Vec<egui::Event> {
    through_in(menu, frames, events, |_| {})
}

/// [`through`] with `show` run in the frame.
fn through_in(menu: &mut NativeMenu, frames: &mut Frames, events: Vec<egui::Event>, mut show: impl FnMut(&mut egui::Ui)) -> Vec<egui::Event> {
    frames.ctx.set_os(egui::os::OperatingSystem::Mac);
    menu.poll(&frames.ctx);
    let mut seen = Vec::new();
    frames.frame(events, |ui| {
        seen = ui.ctx().input(|input| input.events.clone());
        show(ui);
    });
    menu.poll(&frames.ctx);
    seen
}

/// Selectable text that holds key focus with all of it selected: a clipboard key equivalent copies it.
fn selected_text(frames: &mut Frames) -> impl FnMut(&mut egui::Ui) + use<> {
    let rect = Rc::new(Cell::new(egui::Rect::NOTHING));
    let shown = rect.clone();
    let mut show = move |ui: &mut egui::Ui| shown.set(SelectableText::new("Note", "read me").id_salt("note").show(ui).response.rect);
    frames.idle(3, &mut show);
    click(frames, rect.get().center(), &mut show);
    frames.frame(vec![key_press(Key::A, Modifiers::COMMAND)], &mut show);
    frames.frame(vec![], &mut show);
    show
}

/// ⌘ on a Mac keyboard, with ⇧ when `shift`: the modifiers egui reports for a key equivalent.
fn mac_cmd(shift: bool) -> Modifiers {
    Modifiers { shift, mac_cmd: true, command: true, ..Modifiers::NONE }
}

fn key(key: Key, pressed: bool, modifiers: Modifiers) -> egui::Event {
    if pressed { key_press(key, modifiers) } else { key_release(key, modifiers) }
}

fn press() -> egui::Event {
    egui::Event::PointerButton { pos: egui::pos2(300.0, 200.0), button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() }
}

/// The gate is checked when the app takes the clicks, not only when the menu was open: a click
/// that arrived before a modal dialog opened doesn't run behind it. Built-ins still do.
#[trial]
fn a_click_waiting_when_a_dialog_opens_is_dropped() {
    let (mut menu, _, events, _) = driver();
    events.borrow_mut().extend([Event::Click(cmd("file.save")), Event::Click(Builtin::HideOthers.command())]);
    menu.poll(&egui::Context::default());
    menu.set_gate(modal());
    assert_eq!(take_clicks(&mut menu), [Builtin::HideOthers.command()]);

    events.borrow_mut().push(Event::Click(cmd("file.save")));
    menu.poll(&egui::Context::default());
    menu.set_gate(Gate::default());
    assert_eq!(take_clicks(&mut menu), [cmd("file.save")]);
}

/// While a dialog is open the app's items are greyed and the built-ins aren't. Opening and
/// closing it re-syncs the last menus in place, without laying them out again. While a text field
/// has the keys nothing is greyed.
#[trial]
fn a_modal_gate_greys_the_apps_items() {
    let (mut menu, synced, _, _) = driver();
    let opts = LayoutOptions::new("Sample");
    let builds = Cell::new(0);
    let build = || {
        builds.set(builds.get() + 1);
        mac_layout(&sample(false, true), &opts)
    };
    menu.sync_with(1, build);
    menu.set_gate(modal());
    menu.sync_with(1, build);
    menu.sync_with(1, build);
    menu.set_gate(Gate::default());
    menu.sync_with(1, build);
    menu.set_gate(typing(true));
    menu.sync_with(1, build);
    assert_eq!((builds.get(), synced.borrow().len()), (1, 4), "one layout; synced open, greyed, open, typing");

    let synced = synced.borrow();
    let enabled = |bar: &MenuBar, c: &CommandRef| bar.find(c).unwrap().enabled;
    let greyed = &synced[1];
    assert!(!enabled(greyed, &cmd("file.new")) && !enabled(greyed, &cmd("help.about")));
    // AppKit sends a disabled item's key equivalent nowhere: ⌘Z must reach the dialog's fields.
    assert!(enabled(greyed, &cmd("edit.undo")), "the text-editing keys stay");
    assert!(enabled(greyed, &Builtin::Hide.command()) && enabled(greyed, &Builtin::HideOthers.command()));
    assert!(enabled(&synced[2], &cmd("file.new")));
    assert_eq!(structure_key(greyed), structure_key(&synced[0]), "greying is an update in place");
    assert_eq!(synced[2], synced[0], "an open gate greys nothing");
    assert_eq!(synced[3], synced[0], "text editing greys nothing");
}

/// The hook sees app commands clicked while the gate allows them, as they arrive; key presses and
/// built-ins don't reach it. `take_clicks` still returns the commands.
#[trial]
fn on_command_sees_allowed_clicks_only() {
    let (mut menu, _, events, dispatch) = driver();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let log = seen.clone();
    menu.on_command(Box::new(move |c| {
        log.lock().unwrap().push(c.clone());
        false
    }));

    let save = chord("Cmd+S");
    let mouse = |c: &CommandRef| dispatch.chosen(c, Some(&save), false, None);
    events.borrow_mut().push(mouse(&cmd("file.save")));
    assert_eq!(dispatch.chosen(&cmd("file.save"), Some(&save), false, held([true, false, false, false])), Event::Key(save));
    mouse(&Builtin::Quit.command());
    menu.set_gate(modal());
    mouse(&cmd("file.export"));
    menu.set_gate(Gate::default());
    assert_eq!(*seen.lock().unwrap(), [cmd("file.save")]);

    menu.poll(&egui::Context::default());
    assert_eq!(take_clicks(&mut menu), [cmd("file.save")], "a click the hook didn't run");

    menu.on_command(Box::new(|c| c.id == "file.save"));
    events.borrow_mut().extend([mouse(&cmd("file.save")), mouse(&cmd("file.export"))]);
    assert_eq!(events.borrow()[0], Event::Handled(cmd("file.save")));
    menu.poll(&egui::Context::default());
    assert_eq!(take_clicks(&mut menu), [cmd("file.export")], "the hook ran file.save");
}

/// While a text field has the keys, clicking Edit › Copy or Undo acts on the field: the item's
/// shortcut is tapped (press and release, since a click has no key-up), not the app's command run.
#[trial]
fn a_click_on_a_text_editing_item_while_typing_taps_its_shortcut() {
    let mut frames = Frames::new();
    let (mut menu, _, events, dispatch) = driver();
    menu.set_gate(typing(true));
    let (copy, undo, save) = (chord("Cmd+C"), chord("Cmd+Z"), chord("Cmd+S"));
    assert_eq!(dispatch.chosen(&cmd("edit.copy"), Some(&copy), false, None), Event::Tap(copy));
    assert_eq!(dispatch.chosen(&cmd("edit.undo"), Some(&undo), false, None), Event::Tap(undo));
    assert_eq!(dispatch.chosen(&cmd("file.save"), Some(&save), false, None), Event::Click(cmd("file.save")));
    menu.set_gate(typing(false));
    assert_eq!(dispatch.chosen(&cmd("edit.undo"), Some(&undo), false, None), Event::Click(cmd("edit.undo")));

    // Both taps in one poll reach one frame, with the text shown.
    menu.set_gate(typing(true));
    let mut show = selected_text(&mut frames);
    events.borrow_mut().extend([Event::Tap(undo), Event::Tap(copy)]);
    let seen = through_in(&mut menu, &mut frames, vec![], &mut show);
    assert!(
        seen.iter().all(|event| !matches!(event, egui::Event::Key { key: Key::Z, pressed: false, .. })),
        "the focused field takes the ⌘Z press as Undo, so its key-up goes too: {seen:?}"
    );
    assert_eq!(frames.copied(), ["read me"], "the tapped Copy acts on the focused field");
    assert!(take_clicks(&mut menu).is_empty());
}

/// `sync` lays nothing out itself: it compares each frame's layout with the last, so a change
/// without input still reaches the menus, and an unchanged one costs no sync.
#[trial]
fn sync_compares_each_frames_layout() {
    let (mut menu, synced, _, _) = driver();
    let opts = LayoutOptions::new("Sample");
    let mut bar = sample(false, true);
    menu.sync(mac_layout(&bar, &opts));
    menu.sync(mac_layout(&bar, &opts));
    assert_eq!(synced.borrow().len(), 1, "unchanged: no sync");
    apply_state(&mut bar, |c| ItemState::default().enabled(c.id != "file.new"));
    menu.sync(mac_layout(&bar, &opts));
    assert_eq!(synced.borrow().len(), 2);
    assert!(!synced.borrow()[1].find(&cmd("file.new")).unwrap().enabled);
}

/// An item that was a check item stays one when the app's state stops saying checked
/// (`checked: None`): unchecked, with the same structure, so the menus aren't rebuilt.
#[trial]
fn a_check_item_keeps_its_kind() {
    let (mut menu, synced, _, _) = driver();
    let opts = LayoutOptions::new("Sample");
    let with = |checked: Option<bool>| {
        let mut bar = sample(false, true);
        apply_state(&mut bar, |c| ItemState::default().checked(checked.filter(|_| c.id == "file.new")));
        mac_layout(&bar, &opts)
    };
    menu.sync(with(Some(true)));
    menu.sync(with(None));
    let synced = synced.borrow();
    assert_eq!(synced[1].find(&cmd("file.new")).unwrap().kind, ItemKind::Check(false));
    assert_eq!(structure_key(&synced[1]), structure_key(&synced[0]));
}

/// Only shortcuts with ⌘ or ⌃ are key equivalents. A tool key (`B`, `⇧B`, `⌥B`, `F5`) is
/// display-only: choosing its item with ↩ in the open menu is a click. Every shortcut the Mac
/// layout writes is a key equivalent; the app's F7 is not.
#[trial]
fn only_command_or_control_shortcuts_are_key_equivalents() {
    let (_, _, _, dispatch) = driver();
    let chosen = |s: &str| dispatch.chosen(&cmd("tool"), Some(&chord(s)), false, held([true, true, false, false]));
    for s in ["Cmd+B", "Ctrl+B", "Ctrl+F5", "Alt+Cmd+F12", "Shift+Ctrl+Tab"] {
        assert!(matches!(chosen(s), Event::Key(_)), "{s}");
    }
    for s in ["B", "Shift+B", "Alt+B", "Shift+Alt+B", "F5", "Shift+F5", "Escape", "Alt+F12", "Shift+X"] {
        assert_eq!(chosen(s), Event::Click(cmd("tool")), "{s}");
    }

    let bar = mac_layout(&sample(true, true), &LayoutOptions::new("Sample")).bar;
    // Minimize and Quit run in the crate, so they are clicks whatever their shortcut.
    let runs_in_crate = |it: &craft_menu::Item| matches!(Builtin::of(&it.command), Some(Builtin::Minimize | Builtin::Quit));
    for it in bar.items().into_iter().filter(|it| it.layout_shortcut && !runs_in_crate(it)) {
        let Some(sc) = &it.shortcut else { continue };
        assert!(matches!(dispatch.chosen(&it.command, Some(sc), false, held([true, true, false, false])), Event::Key(_)), "{sc} on {}", it.command);
    }
    let layers = bar.find(&cmd("panel.layers")).unwrap().shortcut.unwrap();
    assert!(matches!(dispatch.chosen(&cmd("panel.layers"), Some(&layers), false, held([true, true, false, false])), Event::Click(_)));
}

/// While a shortcut is recorded every key equivalent reaches egui as the key that was pressed,
/// also those that are clicks otherwise (⌘Q on the app's Quit, Minimize, while typing) and the
/// clipboard keys.
#[trial]
fn recording_a_shortcut_passes_key_equivalents_through() {
    let mut frames = Frames::new();
    let (mut menu, _, events, dispatch) = driver();
    menu.set_gate(recording());
    let cmd_held = held([true, false, false, false]);
    let quit = chord("Cmd+Q");
    assert_eq!(dispatch.chosen(&cmd("file.exit"), Some(&quit), true, cmd_held), Event::Key(quit));
    let min = chord("Ctrl+Cmd+M");
    assert_eq!(dispatch.chosen(&Builtin::Minimize.command(), Some(&min), false, held([true, true, false, false])), Event::Key(min));
    let hide = chord("Cmd+H");
    assert_eq!(dispatch.chosen(&Builtin::Hide.command(), Some(&hide), true, cmd_held), Event::Key(hide));
    assert_eq!(dispatch.chosen(&cmd("file.exit"), Some(&quit), true, None), Event::Click(cmd("file.exit")), "a mouse click");
    menu.set_gate(Gate::default().text_editing(true).recording_shortcut(true));
    assert_eq!(dispatch.chosen(&cmd("file.save"), Some(&chord("Cmd+S")), false, cmd_held), Event::Key(chord("Cmd+S")));

    menu.set_gate(recording());
    events.borrow_mut().extend([Event::Key(chord("Cmd+C")), Event::Key(chord("Cmd+V"))]);
    let seen = through(&mut menu, &mut frames, vec![]);
    assert_eq!(seen, [key(Key::C, true, mac_cmd(false)), key(Key::V, true, mac_cmd(false))], "no Copy or Paste events");
}

/// Dynamic lists (recent files, workspaces) change the tree: adding or removing one rebuilds,
/// relabelling one updates in place.
#[trial]
fn dynamic_lists_rebuild_when_they_grow_or_shrink() {
    let recents = |files: &[&str]| {
        let items = files
            .iter()
            .enumerate()
            .map(|(i, f)| craft_menu::Item::new(CommandRef::parse_key(&format!(r#"file.open_recent {{"index":{i}}}"#)).unwrap(), *f).into())
            .collect();
        let file = Menu::new("File", vec![Node::submenu("Open Recent", items)]).role(MenuRole::File);
        MenuBar::new(vec![file])
    };
    let two = structure_key(&recents(&["a.pdf", "b.pdf"]));
    assert_ne!(structure_key(&recents(&["a.pdf", "b.pdf", "c.pdf"])), two);
    assert_ne!(structure_key(&recents(&["a.pdf"])), two);
    assert_eq!(structure_key(&recents(&["b.pdf", "a.pdf"])), two, "same commands, new labels");
}

/// AppKit takes a key equivalent's key-down but winit still delivers the key-up: the menu adds
/// only the press, before a key-up already queued, so the shortcut runs once. (⌘V as a key, as
/// an app taking the raw clipboard keys gets it.) The press carries the modifiers that were held.
#[trial]
fn key_equivalents_become_one_key_press() {
    let mut frames = Frames::new();
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut menu = NativeMenu::new(Box::new(Fake { events: events.clone(), ..Default::default() })).clipboard_keys(ClipboardKeys::Raw);
    let paste = chord("Cmd+V");
    let release = key(Key::V, false, Modifiers::COMMAND);
    let pressed = key(Key::V, true, mac_cmd(false));

    events.borrow_mut().push(Event::Key(paste));
    let seen = through(&mut menu, &mut frames, vec![egui::Event::ModifiersChanged(Modifiers::COMMAND), release.clone()]);
    assert_eq!(seen, [egui::Event::ModifiersChanged(Modifiers::COMMAND), pressed.clone(), release]);

    events.borrow_mut().push(Event::Key(paste));
    let seen = through(&mut menu, &mut frames, vec![]);
    assert_eq!(seen, [pressed], "no made-up release: the real one comes in a later frame");

    // ⌘⇧Z and ⌃⌘M: ⌘ and ⌃ stay distinct.
    events.borrow_mut().extend([Event::Key(chord("Shift+Cmd+Z")), Event::Key(chord("Ctrl+Cmd+M"))]);
    let seen = through(&mut menu, &mut frames, vec![]);
    let ctrl_cmd = Modifiers { ctrl: true, ..mac_cmd(false) };
    assert_eq!(seen, [key(Key::Z, true, mac_cmd(true)), key(Key::M, true, ctrl_cmd)]);
}

/// Minimize and Quit run in the crate, through the runtime's window actions; other clicks go to the app. The menus are
/// laid out again only when the state changed or an event arrived.
#[trial]
fn clicks_and_syncs() {
    let mut frames = Frames::new();
    let synced = Rc::new(RefCell::new(Vec::new()));
    let events = Rc::new(RefCell::new(vec![
        Event::Click(Builtin::Minimize.command()),
        Event::Click("edit.undo".into()),
        Event::Click(craft_quit::QUIT_MENU_ID.into()),
    ]));
    let mut menu = NativeMenu::new(Box::new(Fake { synced: synced.clone(), events: events.clone(), ..Default::default() }));
    let seen = through(&mut menu, &mut frames, vec![]);
    assert!(seen.is_empty());

    let ctx = egui::Context::default();
    let mut clicks = Vec::new();
    let mut out = ctx.run_ui(Default::default(), |ui| clicks = menu.take_clicks(ui.ctx()));
    out.textures_delta.clear(); // not painted
    assert_eq!(clicks, [cmd("edit.undo")]);
    let commands = out.viewport_output.remove(&egui::ViewportId::ROOT).map(|o| o.commands).unwrap_or_default();
    assert!(commands.contains(&egui::ViewportCommand::Close), "Quit asks the root window to close through the runtime");

    let opts = LayoutOptions::new("Sample");
    let builds = Cell::new(0);
    let build = || {
        builds.set(builds.get() + 1);
        mac_layout(&sample(true, true), &opts)
    };
    assert_eq!(menu.sync_with(1, build).len(), 4, "the clashes, once");
    assert!(menu.sync_with(1, build).is_empty());
    assert_eq!(builds.get(), 1, "same state: nothing built");
    assert!(menu.sync_with(2, build).is_empty(), "same clashes: not reported again");
    events.borrow_mut().push(Event::Click("edit.redo".into()));
    menu.poll(&egui::Context::default());
    menu.sync_with(2, build);
    assert_eq!((builds.get(), synced.borrow().len()), (3, 3), "an event re-syncs");
}

/// A press while a menu is open closes it and is kept from egui, so it starts no canvas gesture;
/// pointer moves stay. With no menu open the press reaches egui (the menu is still asked to close,
/// in case one is on screen).
#[trial]
fn outside_press_dismisses_without_starting_a_canvas_gesture() {
    let mut frames = Frames::new();
    let tracking = Rc::new(Cell::new(true));
    let cancels = Rc::new(Cell::new(0));
    let mut menu = NativeMenu::new(Box::new(Fake { tracking: tracking.clone(), cancels: cancels.clone(), ..Default::default() }));
    let moved = egui::Event::PointerMoved(egui::pos2(300.0, 200.0));
    let seen = through(&mut menu, &mut frames, vec![moved.clone(), press()]);
    assert_eq!(seen, [moved]);
    assert_eq!((cancels.get(), tracking.get()), (1, false));

    let seen = through(&mut menu, &mut frames, vec![press()]);
    assert_eq!(seen, [press()], "ordinary canvas presses still reach egui");
    assert_eq!(cancels.get(), 2);

    through(&mut menu, &mut frames, vec![egui::Event::PointerMoved(egui::pos2(1.0, 1.0))]);
    assert_eq!(cancels.get(), 2, "a pointer move closes nothing");
}

#[trial]
fn escape_dismisses_an_open_menu_without_reaching_the_active_tool() {
    let mut frames = Frames::new();
    let tracking = Rc::new(Cell::new(true));
    let mut menu = NativeMenu::new(Box::new(Fake { tracking: tracking.clone(), ..Default::default() }));
    let escape = |pressed| key(Key::Escape, pressed, Modifiers::NONE);
    let seen = through(&mut menu, &mut frames, vec![escape(true), escape(false)]);
    assert!(seen.is_empty());
    assert!(!tracking.get());
}

#[trial]
fn moving_the_pointer_does_not_dismiss_an_open_menu() {
    let mut frames = Frames::new();
    let cancels = Rc::new(Cell::new(0));
    let mut menu = NativeMenu::new(Box::new(Fake { tracking: Rc::new(Cell::new(true)), cancels: cancels.clone(), ..Default::default() }));
    let seen = through(&mut menu, &mut frames, vec![egui::Event::PointerMoved(egui::pos2(300.0, 200.0))]);
    assert_eq!((seen.len(), cancels.get()), (1, 0));
}

/// A backend that keeps reporting a menu open costs one window click, not all of them; once it
/// reports the menu closed, the next open menu's closing click is kept from egui again.
#[trial]
fn a_stuck_open_menu_keeps_only_one_click() {
    let mut frames = Frames::new();
    let tracking = Rc::new(Cell::new(true));
    let mut menu = NativeMenu::new(Box::new(Fake { tracking: tracking.clone(), stuck: true, ..Default::default() }));
    let seen = through(&mut menu, &mut frames, vec![press()]);
    assert!(seen.is_empty(), "the click that closes the menu");
    for _ in 0..3 {
        let seen = through(&mut menu, &mut frames, vec![press()]);
        assert_eq!(seen, [press()], "later clicks reach egui");
    }
    tracking.set(false);
    menu.poll(&frames.ctx);
    tracking.set(true);
    let seen = through(&mut menu, &mut frames, vec![press()]);
    assert!(seen.is_empty());
}

/// AppKit takes ⌘C, ⌘X and ⌘V for the Edit menu, so egui-winit never makes its Copy, Cut and
/// Paste events: the driver makes them, unless the app takes the raw keys.
#[trial]
fn clipboard_key_equivalents_reach_text_fields() {
    let mut frames = Frames::new();
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut menu = NativeMenu::new(Box::new(Fake { events: events.clone(), ..Default::default() }));
    let equivalent = |s: &str| Event::Key(chord(s));
    let mut show = selected_text(&mut frames);
    events.borrow_mut().extend([equivalent("Cmd+C"), equivalent("Cmd+Shift+V")]);
    let seen = through_in(&mut menu, &mut frames, vec![], &mut show);
    assert_eq!(frames.copied(), ["read me"], "⌘C copies the focused field's selection");
    assert_eq!(seen, [key(Key::V, true, mac_cmd(true))], "⌘C is the runtime's; ⌘⇧V is an app shortcut and reaches egui once");

    events.borrow_mut().push(equivalent("Cmd+V"));
    let seen = through_in(&mut menu, &mut frames, vec![], &mut show);
    assert!(seen.is_empty(), "the paste comes from eframe, with the clipboard's text");
    let ctx = egui::Context::default();
    let mut out = ctx.run_ui(Default::default(), |ui| {
        menu.take_clicks(ui.ctx());
    });
    out.textures_delta.clear();
    let commands = out.viewport_output.remove(&egui::ViewportId::ROOT).map(|o| o.commands).unwrap_or_default();
    assert!(commands.contains(&egui::ViewportCommand::RequestPaste));

    let mut menu = NativeMenu::new(Box::new(Fake { events: events.clone(), ..Default::default() })).clipboard_keys(ClipboardKeys::Raw);
    events.borrow_mut().extend([equivalent("Cmd+C"), equivalent("Cmd+V")]);
    let mut frames = Frames::new();
    let seen = through(&mut menu, &mut frames, vec![]);
    assert_eq!(seen, [key(Key::C, true, mac_cmd(false)), key(Key::V, true, mac_cmd(false))]);
}

/// ⌘X on an editable field with a selection removes the selection and puts it on the clipboard.
#[trial]
fn cut_removes_the_selection_of_an_editable_field() {
    let mut frames = Frames::new();
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut menu = NativeMenu::new(Box::new(Fake { events: events.clone(), ..Default::default() }));
    let value = RefCell::new("cut me".to_owned());
    let rect = Cell::new(egui::Rect::NOTHING);
    let mut show = |ui: &mut egui::Ui| rect.set(TextField::new("Note", Bind::bound(&mut *value.borrow_mut())).id_salt("note").show(ui).response.rect);
    frames.idle(3, &mut show);
    click(&mut frames, rect.get().center(), &mut show);
    frames.frame(vec![key_press(Key::A, Modifiers::COMMAND)], &mut show);
    frames.frame(vec![], &mut show);
    events.borrow_mut().push(Event::Key(chord("Cmd+X")));
    through_in(&mut menu, &mut frames, vec![], &mut show);
    assert_eq!(frames.copied(), ["cut me"], "the selection is on the clipboard");
    assert_eq!(*value.borrow(), "", "and gone from the field");
}

/// The layout marks the shortcuts it gives (the app's key handling doesn't bind them), and a key
/// equivalent on such an item comes back as a click.
#[trial]
fn shortcuts_the_layout_assigns_come_back_as_clicks() {
    let (_, _, _, dispatch) = driver();
    let l = mac_layout(&sample(false, true), &LayoutOptions::new("Sample"));
    let assigned = |id: &str| l.bar.find(&cmd(id)).unwrap().layout_shortcut;
    assert!(assigned("file.exit") && assigned("prefs.general") && assigned(Builtin::Hide.id()));
    assert!(!assigned("edit.undo") && !assigned("prefs.keys"));
    let mut bar = sample(false, true);
    bar.menus[0].children.push(craft_menu::Item::new("file.quit", "Quit").shortcut(chord("Cmd+Q")).role(craft_menu::ItemRole::Quit).into());
    bar.tag_item(&cmd("file.exit"), craft_menu::ItemRole::About);
    assert!(!mac_layout(&bar, &LayoutOptions::new("Sample")).bar.find(&cmd("file.quit")).unwrap().layout_shortcut, "the app's own ⌘Q");

    let cmd_q = chord("Cmd+Q");
    let cmd_held = held([true, false, false, false]);
    assert_eq!(dispatch.chosen(&cmd("file.exit"), Some(&cmd_q), true, cmd_held), Event::Click("file.exit".into()));
    assert_eq!(dispatch.chosen(&cmd("file.quit"), Some(&cmd_q), false, cmd_held), Event::Key(cmd_q));
    // ⇧ held on a ⌘R item: the key press that was made.
    assert_eq!(
        dispatch.chosen(&cmd("view.reload"), Some(&chord("Cmd+R")), false, held([true, false, false, true])),
        Event::Key(chord("Cmd+Shift+R"))
    );
    assert_eq!(dispatch.chosen(&cmd("file.quit"), Some(&cmd_q), false, None), Event::Click("file.quit".into()), "a mouse click");
    assert_eq!(
        dispatch.chosen(&Builtin::Minimize.command(), Some(&chord("Ctrl+Cmd+M")), false, held([true, true, false, false])),
        Event::Click(Builtin::Minimize.command())
    );
    // ↩ inside the open menu: on a ⌘ item without ⌘ held, and on a bare tool key.
    let enter = held([false; 4]);
    assert_eq!(dispatch.chosen(&cmd("file.quit"), Some(&cmd_q), false, enter), Event::Click("file.quit".into()));
    assert_eq!(dispatch.chosen(&cmd("tool.brush"), Some(&chord("B")), false, enter), Event::Click("tool.brush".into()));
}

/// While a text field has the keys, a key equivalent runs its command as a click (⌘S saves while
/// typing), except the editing keys the field handles itself.
#[trial]
fn key_equivalents_run_commands_while_text_editing() {
    let (mut menu, _, _, dispatch) = driver();
    let chosen = |menu: &mut NativeMenu, shortcut: &str, mods: [bool; 4], text_editing: bool| {
        menu.set_gate(typing(text_editing));
        dispatch.chosen(&cmd("cmd"), Some(&chord(shortcut)), false, held(mods))
    };
    let cmd_only = [true, false, false, false];
    let shift_cmd = [true, false, false, true];
    for (shortcut, held) in [
        ("Cmd+S", cmd_only),
        ("Cmd+N", cmd_only),
        ("Cmd+Shift+S", shift_cmd),
        ("Ctrl+Cmd+F", [true, true, false, false]),
        ("Ctrl+F5", [false, true, false, false]),
    ] {
        assert_eq!(chosen(&mut menu, shortcut, held, true), Event::Click("cmd".into()), "{shortcut} while typing");
        assert!(matches!(chosen(&mut menu, shortcut, held, false), Event::Key(_)), "{shortcut} with no text field");
    }
    for (shortcut, held) in
        [("Cmd+C", cmd_only), ("Cmd+X", cmd_only), ("Cmd+V", cmd_only), ("Cmd+A", cmd_only), ("Cmd+Z", cmd_only), ("Cmd+Shift+Z", shift_cmd)]
    {
        assert_eq!(chosen(&mut menu, shortcut, held, true), Event::Key(chord(shortcut)), "{shortcut} stays with the field");
    }
    assert_eq!(chosen(&mut menu, "Cmd+A", shift_cmd, true), Event::Click("cmd".into()), "⇧⌘A is not select all");
}

/// A modal dialog or a recorded shortcut lets only the built-in items run.
#[trial]
fn a_modal_dialog_or_shortcut_recording_lets_only_the_built_in_items_run() {
    let save = craft_menu::Item::new(CommandRef::new("file.save"), "Save");
    let quit = craft_menu::Item::new(Builtin::Quit.command(), "Quit");
    for gate in [modal(), recording()] {
        assert!(!gate.allows(&save) && gate.allows(&quit), "{gate:?}");
    }
    assert!(typing(true).allows(&save));
    assert!(Gate::default().allows(&save));
}
