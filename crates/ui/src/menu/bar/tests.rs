//! Menu bar tests: titles and press-drag-release, hover switching, ← / → across menus, Alt, F10
//! and Alt+letter, mnemonics on titles, overflow, the Help search and AccessKit. A raw egui
//! context driven frame by frame.

use egui::accesskit::{self, NodeId, Role};
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Rect, Vec2, pos2, vec2};

use super::keys::is_command_key;
use super::*;
use crate::direction::Direction;
use crate::theme::{Base, install};

fn item(label: &str, command: &'static str) -> MenuEntry<&'static str> {
    MenuItem::new(label, command).into()
}

/// A small app's menus.
fn sample() -> Vec<MenuItem<&'static str>> {
    vec![
        MenuItem::submenu(
            "File",
            vec![
                item("New", "file.new"),
                item("Open…", "file.open"),
                MenuEntry::Separator,
                MenuItem::submenu("Open Recent", vec![item("a.txt", "file.recent.a"), item("b.txt", "file.recent.b")]).into(),
                MenuItem::new("Locked", "file.locked").enabled(false).into(),
            ],
        ),
        MenuItem::submenu("Edit", vec![item("Undo", "edit.undo"), item("Redo", "edit.redo")]),
        MenuItem::submenu(
            "View",
            vec![MenuItem::new("Grid", "view.grid").check(true).into(), MenuItem::new("Rulers", "view.rulers").check(false).into()],
        ),
        MenuItem::submenu("Help", vec![item("About", "help.about")]),
    ]
}

/// The bar alone in a window, driven frame by frame with synthetic input.
struct Bar {
    ctx: egui::Context,
    menus: Vec<MenuItem<&'static str>>,
    size: Vec2,
    fit: Option<f32>,
    search: Option<usize>,
    keyboard: bool,
    time: f64,
    events: Vec<Event>,
    pointer: Pos2,
    tree: Vec<(NodeId, accesskit::Node)>,
    /// The AccessKit tree's focus in the last update.
    accesskit_focus: Option<NodeId>,
    chosen: Vec<&'static str>,
    /// Where a label after a shrunk bar starts.
    after: Option<f32>,
    /// An Esc was left in the input after the bar, in some pass.
    escape_after: bool,
    /// [`crate::menu::any_open`] as the last pass began, before the bar was drawn.
    open_before: bool,
}

impl Bar {
    fn new(menus: Vec<MenuItem<&'static str>>, size: Vec2) -> Bar {
        let ctx = egui::Context::default();
        install(&ctx, &Base::suite_default());
        ctx.set_os(egui::os::OperatingSystem::Nix);
        ctx.enable_accesskit();
        ctx.all_styles_mut(|s| s.scroll_animation = egui::style::ScrollAnimation::none());
        let mut b = Bar {
            ctx,
            menus,
            size,
            fit: None,
            search: None,
            keyboard: true,
            time: 0.0,
            events: Vec::new(),
            pointer: pos2(-10.0, -10.0),
            tree: Vec::new(),
            accesskit_focus: None,
            chosen: Vec::new(),
            after: None,
            escape_after: false,
            open_before: false,
        };
        b.run(3);
        b
    }

    fn wide() -> Bar {
        Bar::new(sample(), vec2(800.0, 600.0))
    }

    fn frame(&mut self) {
        self.time += 1.0 / 60.0;
        let raw = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, self.size)),
            time: Some(self.time),
            predicted_dt: 1.0 / 60.0,
            events: std::mem::take(&mut self.events),
            ..Default::default()
        };
        let (menus, fit, search, keyboard) = (&self.menus, self.fit, self.search, self.keyboard);
        let mut chosen = None;
        let mut after = None;
        let mut escape = false;
        let mut open_before = false;
        let mut full = self.ctx.run_ui(raw, |ui| {
            open_before = crate::menu::any_open(ui.ctx());
            let mut bar = MenuBar::new(menus).keyboard(keyboard);
            if let Some(i) = search {
                bar = bar.help_search(i);
            }
            match fit {
                Some(max) => {
                    ui.horizontal(|ui| {
                        chosen = bar.shrink_to_fit(max).show(ui).chosen;
                        after = Some(ui.label("after").rect.left());
                    });
                }
                None => chosen = bar.show(ui).chosen,
            }
            escape = ui.input(|i| i.key_pressed(Key::Escape));
        });
        full.textures_delta.clear();
        if let Some(update) = full.platform_output.accesskit_update {
            self.tree = update.nodes;
            self.accesskit_focus = Some(update.focus);
        }
        self.after = after;
        self.escape_after |= escape;
        self.open_before = open_before;
        self.chosen.extend(chosen);
    }

    fn run(&mut self, frames: usize) {
        for _ in 0..frames {
            self.frame();
        }
    }

    fn take(&mut self) -> Vec<&'static str> {
        std::mem::take(&mut self.chosen)
    }

    fn node(&self, label: &str) -> Option<&accesskit::Node> {
        self.tree.iter().map(|(_, n)| n).find(|n| n.label() == Some(label))
    }

    fn shown(&self, label: &str) -> bool {
        self.node(label).is_some()
    }

    /// A title of the bar (not a row of the same name).
    fn title(&self, label: &str) -> Option<&accesskit::Node> {
        let (_, bar) = self.tree.iter().find(|(_, n)| n.role() == Role::MenuBar)?;
        self.tree.iter().filter(|(id, _)| bar.children().contains(id)).map(|(_, n)| n).find(|n| n.label() == Some(label))
    }

    fn titles(&self) -> Vec<String> {
        let Some((_, bar)) = self.tree.iter().find(|(_, n)| n.role() == Role::MenuBar) else { return Vec::new() };
        bar.children().iter().filter_map(|id| self.tree.iter().find(|(i, _)| i == id)).filter_map(|(_, n)| n.label().map(str::to_owned)).collect()
    }

    fn rect(&self, label: &str) -> Rect {
        let n = self.node(label).unwrap_or_else(|| panic!("no {label:?}"));
        let b = n.bounds().unwrap_or_else(|| panic!("{label:?} has no bounds"));
        Rect::from_min_max(pos2(b.x0 as f32, b.y0 as f32), pos2(b.x1 as f32, b.y1 as f32))
    }

    fn point(&self, label: &str) -> Pos2 {
        let r = self.rect(label);
        pos2(r.left() + (r.width() / 2.0).min(20.0), r.center().y)
    }

    fn move_to(&mut self, p: Pos2) {
        self.pointer = p;
        self.events.push(Event::PointerMoved(p));
        self.frame();
    }

    fn hover(&mut self, label: &str) {
        let p = self.point(label);
        self.move_to(p);
        self.run(3);
    }

    fn button(&mut self, pressed: bool) {
        self.events.push(Event::PointerButton { pos: self.pointer, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
        self.frame();
    }

    fn click_at(&mut self, p: Pos2) {
        self.move_to(p);
        self.button(true);
        self.button(false);
        self.run(3);
    }

    fn click(&mut self, label: &str) {
        let p = self.point(label);
        self.click_at(p);
    }

    fn key_with(&mut self, key: Key, modifiers: Modifiers) {
        for pressed in [true, false] {
            self.events.push(Event::Key { key, physical_key: None, pressed, repeat: false, modifiers });
        }
        self.run(3);
    }

    fn key(&mut self, key: Key) {
        self.key_with(key, Modifiers::NONE);
    }

    fn alt(&mut self, pressed: bool) {
        let modifiers = if pressed { Modifiers::ALT } else { Modifiers::NONE };
        self.events.push(Event::Key { key: Key::AltLeft, physical_key: None, pressed, repeat: false, modifiers });
        self.run(2);
    }

    fn alt_tap(&mut self) {
        self.alt(true);
        self.alt(false);
        self.run(1);
    }

    /// A letter typed: its key and the text it types.
    fn letter(&mut self, c: char) {
        let key = Key::from_name(&c.to_ascii_uppercase().to_string()).unwrap();
        self.events.push(Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE });
        self.events.push(Event::Text(c.to_string()));
        self.events.push(Event::Key { key, physical_key: None, pressed: false, repeat: false, modifiers: Modifiers::NONE });
        self.run(3);
    }

    fn typed(&mut self, text: &str) {
        for c in text.chars() {
            self.letter(c);
        }
    }

    fn focus(&self) -> Option<usize> {
        let state: BarState = self.ctx.data(|d| d.get_temp(Id::unique("craft-ui-menu-bar"))).unwrap_or_default();
        state.keys.focus
    }

    fn open(&self) -> bool {
        is_open(&self.ctx)
    }
}

#[test]
fn a_press_opens_a_title_a_second_press_closes_it_and_a_row_runs() {
    let mut b = Bar::wide();
    assert!(!b.shown("New"));
    b.click("File");
    assert!(b.shown("New") && b.open(), "a click on a title opens its menu");
    b.click("File");
    assert!(!b.shown("New") && !b.open(), "a second click on the title closes it");
    b.click("File");
    b.click("New");
    assert_eq!(b.take(), ["file.new"]);
    assert!(!b.shown("New") && !b.open(), "running a row closes the menu");
    b.click("File");
    b.click("Locked");
    assert!(b.take().is_empty() && b.shown("New"), "a click on a disabled row leaves the menu open");
    b.click_at(pos2(700.0, 500.0));
    assert!(!b.open(), "a click outside closes it");
}

#[test]
fn press_drag_release_runs_the_row_and_a_release_on_the_title_keeps_the_menu_open() {
    let mut b = Bar::wide();
    let file = b.rect("File").center();
    b.move_to(file);
    b.button(true);
    b.run(3);
    assert!(b.shown("Open…"), "the press opens the menu");
    b.button(false);
    b.run(3);
    assert!(b.shown("Open…") && b.take().is_empty(), "the release on the title keeps it open");
    b.click("File");
    assert!(!b.open());

    b.move_to(file);
    b.button(true);
    b.run(2);
    // Down out of the bar first, then across: crossing the Edit title would open Edit.
    let open = b.point("Open…");
    let below = pos2(file.x, open.y);
    for k in 1..=6 {
        b.move_to(file + (below - file) * (k as f32 / 6.0));
    }
    b.move_to(open);
    b.button(false);
    b.run(2);
    assert_eq!(b.take(), ["file.open"], "the release on a row runs it");
    assert!(!b.open());

    // Released over nothing, the menu stays open.
    b.move_to(file);
    b.button(true);
    b.run(2);
    b.move_to(pos2(file.x, 500.0));
    b.button(false);
    b.run(2);
    assert!(b.shown("Open…") && b.take().is_empty(), "a release over nothing keeps it open");
}

#[test]
fn hovering_another_title_switches_menus_only_while_one_is_open_and_the_pointer_moves() {
    let mut b = Bar::wide();
    b.hover("Edit");
    assert!(!b.shown("Undo"), "hover alone opens nothing");
    b.click("File");
    b.hover("Edit");
    assert!(b.shown("Undo") && !b.shown("New"), "hover switches to Edit");
    b.hover("View");
    assert!(b.shown("Grid") && !b.shown("Undo"));
    // The pointer rests on View: → opens Help, and the resting pointer doesn't switch back.
    b.key(Key::ArrowRight);
    b.run(3);
    assert!(b.shown("About") && !b.shown("Grid"), "a pointer at rest doesn't undo →");
}

#[test]
fn right_and_left_open_the_neighbouring_menus_and_wrap() {
    let mut b = Bar::wide();
    b.click("File");
    b.key(Key::ArrowRight);
    assert!(b.shown("Undo") && !b.shown("New"), "→ on a plain row opens the next menu");
    b.key(Key::Enter);
    assert_eq!(b.take(), ["edit.undo"], "a menu opened with → lights its first row");
    b.click("File");
    b.key(Key::ArrowLeft);
    assert!(b.shown("About"), "← from the first menu wraps to the last");
    b.key(Key::ArrowRight);
    assert!(b.shown("New"), "→ from the last wraps to the first");
    b.key(Key::ArrowDown);
    b.key(Key::ArrowDown);
    b.key(Key::ArrowRight);
    assert!(b.shown("a.txt") && b.shown("New"), "→ on a submenu row opens the submenu, not the next menu");
    b.key(Key::ArrowLeft);
    b.key(Key::ArrowLeft);
    assert!(b.shown("About"), "← closes the submenu, then opens the previous menu");
    b.key(Key::Escape);
    assert!(!b.open());

    // Right to left the titles run from the right, and ← opens the next menu.
    crate::direction::apply(&b.ctx, Direction::Rtl);
    b.run(2);
    assert!(b.rect("Edit").right() <= b.rect("File").left() && b.rect("File").right() > 790.0, "File is rightmost, Edit on its left");
    b.click("File");
    b.key(Key::ArrowLeft);
    assert!(b.shown("Undo"), "← opens Edit, on File's left");
}

#[test]
fn alt_and_f10_focus_the_bar_and_alt_with_a_letter_opens_a_menu() {
    let mut b = Bar::wide();
    b.alt_tap();
    assert_eq!(b.focus(), Some(0), "Alt puts the keyboard on File");
    assert!(is_focused(&b.ctx) && !b.open(), "no menu opens yet");
    b.key(Key::ArrowRight);
    assert_eq!(b.focus(), Some(1), "→ moves to Edit");
    b.key(Key::ArrowLeft);
    b.key(Key::ArrowLeft);
    assert_eq!(b.focus(), Some(3), "← wraps round to Help");
    b.key(Key::ArrowDown);
    assert!(b.shown("About") && b.focus().is_none(), "↓ opens the lit menu");
    b.key(Key::Escape);
    assert!(!b.open() && !is_focused(&b.ctx), "Esc leaves the menu and the bar");

    b.key(Key::F10);
    assert_eq!(b.focus(), Some(0), "F10 focuses the bar too");
    b.letter('v');
    assert!(b.shown("Grid"), "a title's letter on the focused bar opens it");
    b.key(Key::Escape);
    b.key(Key::F10);
    b.alt_tap();
    assert_eq!(b.focus(), None, "a tap leaves the bar");

    b.key_with(Key::E, Modifiers::ALT);
    assert!(b.shown("Undo"), "Alt+E opens Edit");
    b.key_with(Key::F, Modifiers::ALT);
    assert!(b.shown("Open…") && !b.shown("Undo"), "Alt+F switches to File while a menu is open");
    b.letter('r');
    assert!(b.shown("a.txt"), "a letter in the open menu opens Open Recent");
    b.letter('b');
    assert_eq!(b.take(), ["file.recent.b"]);
    b.alt(true);
    b.alt(false);
    assert!(!b.open(), "Alt alone closes nothing that is closed");
    b.click("File");
    b.alt_tap();
    assert!(!b.open(), "an Alt tap closes an open menu, as on Windows");
}

#[test]
fn titles_take_preferred_letters_and_letterless_titles_show_their_english_key() {
    let mut menus = sample();
    menus[2] = MenuItem::submenu("View", vec![item("Grid", "view.grid")]).mnemonic('W');
    menus[3] = MenuItem::submenu("帮助", vec![item("About", "help.about")]).english("Help");
    let mut b = Bar::new(menus, vec2(800.0, 600.0));
    assert!(b.title("帮助(H)").is_some(), "{:?}", b.titles());
    assert_eq!(b.title("View").and_then(|n| n.access_key()), Some("W"));
    b.key_with(Key::H, Modifiers::ALT);
    assert!(b.shown("About"), "Alt+H opens 帮助(H)");
    b.key_with(Key::W, Modifiers::ALT);
    assert!(b.shown("Grid"), "View took its W");
}

/// Without mnemonics (a Mac, a browser on a Mac) Option types, so Alt leaves the bar alone, but
/// F10 still puts the keyboard on it: the in-window bar is the only one there.
#[test]
fn without_mnemonics_alt_leaves_the_bar_alone_and_f10_still_reaches_it() {
    let mut b = Bar::wide();
    b.ctx.set_os(egui::os::OperatingSystem::Mac);
    b.alt_tap();
    assert!(b.focus().is_none() && !is_focused(&b.ctx), "an Alt tap leaves the bar alone on a Mac");
    b.key_with(Key::F, Modifiers::ALT);
    assert!(!b.open(), "Option+F types");
    assert_eq!(b.title("File").and_then(|n| n.access_key()), None);
    b.key(Key::F10);
    assert_eq!(b.focus(), Some(0), "F10 puts the keyboard on File");
    b.key(Key::ArrowRight);
    b.key(Key::ArrowDown);
    assert!(b.shown("Undo") && b.focus().is_none(), "→ and ↓ open Edit");
    b.key(Key::Escape);
    b.key(Key::F10);
    b.key(Key::F10);
    assert!(b.focus().is_none(), "F10 again leaves the bar");

    b.ctx.set_os(egui::os::OperatingSystem::Nix);
    b.keyboard = false;
    b.alt_tap();
    b.key(Key::F10);
    assert!(b.focus().is_none(), "nor while a dialog has the keys");
}

/// PDFCraft leaves full screen on an Esc nothing else took: the Esc that closes a bar's menu is
/// the menu's.
#[test]
fn the_escape_that_closes_a_menu_is_not_read_after_the_bar() {
    let mut b = Bar::wide();
    b.click("File");
    b.escape_after = false;
    b.key(Key::Escape);
    assert!(!b.open() && !b.escape_after, "the menu closed and took the key");
    b.key(Key::Escape);
    assert!(b.escape_after, "with no menu open, Esc goes on");
}

#[test]
fn the_lit_row_of_a_bar_menu_is_its_menus_active_descendant() {
    let mut b = Bar::wide();
    b.key(Key::F10);
    b.key(Key::ArrowDown);
    let id = |b: &Bar, label: &str| b.tree.iter().find(|(_, n)| n.label() == Some(label)).map(|(id, _)| *id);
    assert_eq!(crate::menu::tests::active_in_menu(&b.tree), id(&b, "New"));
    b.key(Key::ArrowDown);
    assert_eq!(crate::menu::tests::active_in_menu(&b.tree), id(&b, "Open…"));
}

/// Two bars in one window: the second, with no menu open, doesn't hide the first one's open menu
/// from [`is_open`].
#[test]
fn is_open_sees_the_open_menu_of_either_bar() {
    let ctx = egui::Context::default();
    install(&ctx, &Base::suite_default());
    ctx.enable_accesskit();
    let (one, two) = (sample(), sample());
    let mut tree = Vec::new();
    let mut frame = |events: Vec<Event>| {
        let raw = egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))), events, ..Default::default() };
        let mut full = ctx.run_ui(raw, |ui| {
            let _ = MenuBar::new(&one).id(Id::unique("one")).show(ui);
            let _ = MenuBar::new(&two).id(Id::unique("two")).show(ui);
        });
        full.textures_delta.clear();
        if let Some(update) = full.platform_output.accesskit_update {
            tree = update.nodes;
        }
        tree.clone()
    };
    let tree = frame(Vec::new());
    let file = tree.iter().find(|(_, n)| n.label() == Some("File")).and_then(|(_, n)| n.bounds()).expect("File");
    let p = pos2((file.x0 + 10.0) as f32, ((file.y0 + file.y1) / 2.0) as f32);
    let button = |pressed| Event::PointerButton { pos: p, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
    frame(vec![Event::PointerMoved(p), button(true), button(false)]);
    for _ in 0..3 {
        frame(Vec::new());
    }
    assert!(is_open(&ctx), "the first bar's File menu is open");
}

#[test]
fn a_command_shortcut_takes_the_keyboard_off_the_bar() {
    let mut b = Bar::wide();
    b.alt_tap();
    assert!(is_focused(&b.ctx));
    b.events.push(Event::Key { key: Key::S, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::COMMAND });
    b.time += 1.0 / 60.0;
    let raw = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, b.size)),
        time: Some(b.time),
        events: std::mem::take(&mut b.events),
        ..Default::default()
    };
    let menus = &b.menus;
    let mut left = false;
    let mut out = b.ctx.run_ui(raw, |ui| {
        left = leave_for_shortcut(ui.ctx());
        let _ = MenuBar::new(menus).show(ui);
    });
    out.textures_delta.clear();
    assert!(left && !is_focused(&b.ctx), "Ctrl+S leaves the bar for the app's shortcut");
    b.run(2);
    assert_eq!(b.focus(), None);
    assert!(is_command_key(Key::F7, Modifiers::NONE) && !is_command_key(Key::F10, Modifiers::NONE) && !is_command_key(Key::A, Modifiers::NONE));
}

#[test]
fn titles_that_dont_fit_move_into_the_overflow_menu() {
    let file_and_edit = {
        let b = Bar::wide();
        b.rect("Edit").right() - b.rect("File").left()
    };
    let mut b = Bar::new(sample(), vec2(file_and_edit + 30.0, 400.0));
    assert_eq!(b.titles(), ["File", "Edit", "⋯"], "View and Help moved into ⋯");
    b.click("⋯");
    assert!(b.shown("View") && b.shown("Help"), "⋯ lists the menus that don't fit");
    b.hover("Help");
    b.run(3);
    b.click("About");
    assert_eq!(b.take(), ["help.about"]);

    let b = Bar::new(sample(), vec2(20.0, 400.0));
    assert_eq!(b.titles(), ["Menu"], "one Menu holds all when none fits");

    let mut b = Bar::wide();
    b.fit = Some(f32::INFINITY);
    b.run(2);
    assert!(b.after.is_some_and(|x| x < 400.0), "a shrunk bar takes only its titles' width: {:?}", b.after);
    assert_eq!(b.titles().len(), 4);
    b.fit = Some(100.0);
    b.run(2);
    assert_eq!(b.titles().last().map(String::as_str), Some("⋯"), "past its limit the titles overflow");
    assert!(b.after.is_some_and(|x| x <= 100.0 + 8.0 + 0.5), "{:?}", b.after);
}

#[test]
fn accesskit_sees_a_menu_bar_of_titles_with_popups() {
    let mut b = Bar::wide();
    let file = b.title("File").expect("File is a title in the bar");
    assert_eq!(file.role(), Role::MenuItem);
    assert_eq!(file.has_popup(), Some(accesskit::HasPopup::Menu));
    assert_eq!(file.is_expanded(), Some(false));
    assert_eq!(file.access_key(), Some("F"));
    b.click("View");
    assert_eq!(b.title("View").and_then(|n| n.is_expanded()), Some(true));
    assert_eq!(b.title("File").and_then(|n| n.is_expanded()), Some(false));
    assert!(b.tree.iter().any(|(_, n)| n.role() == Role::Menu), "the open menu");
    assert_eq!(b.node("Grid").map(|n| n.role()), Some(Role::MenuItemCheckBox));
}

#[test]
fn the_help_search_lists_matches_takes_typing_and_runs_a_match() {
    let mut b = Bar::wide();
    b.search = Some(3);
    b.click("Help");
    assert!(b.shown("About") && b.ctx.text_edit_focused(), "Help opens with its search field focused");
    b.typed("re");
    assert!(b.shown("Help") && b.shown("File › Open Recent › a.txt") && b.shown("Edit › Redo"), "typing lists matches");
    assert!(b.take().is_empty(), "letters type into the field, not mnemonics");
    b.typed("do");
    assert!(b.shown("Edit › Redo") && !b.shown("File › Open Recent › a.txt"));
    b.key(Key::Enter);
    assert_eq!(b.take(), ["edit.redo"], "Enter in the field runs the first match");
    assert!(!b.open());
    b.click("Help");
    assert!(!b.shown("Edit › Redo"), "a fresh opening starts empty");
    assert!(b.ctx.text_edit_focused(), "focused again");
    b.typed("zzz");
    assert!(b.tree.iter().any(|(_, n)| n.value() == Some("No matching commands")));
}

#[test]
fn the_help_search_lists_a_command_in_several_menus_once() {
    let mut menus = sample();
    menus[2] = MenuItem::submenu("View", vec![MenuItem::submenu("History", vec![item("Undo", "edit.undo")]).into()]);
    let mut b = Bar::new(menus, vec2(800.0, 600.0));
    b.search = Some(3);
    b.click("Help");
    b.typed("undo");
    assert!(b.shown("Edit › Undo"), "listed at its first place in menu order");
    assert!(!b.shown("View › History › Undo"), "and only once");
}

#[test]
fn search_matches_take_no_mnemonics_and_leave_the_help_rows_their_letters() {
    let mut menus = sample();
    menus.insert(3, MenuItem::submenu("Arrange", vec![item("Report Layout", "arrange.report")]));
    menus[4] = MenuItem::submenu("Help", vec![item("About", "help.about"), item("Report a Bug", "help.bug")]);
    let mut b = Bar::new(menus, vec2(800.0, 600.0));
    b.search = Some(4);
    let key = |b: &Bar, label: &str| b.node(label).and_then(|n| n.access_key()).map(str::to_owned);
    b.click("Help");
    let letters = (key(&b, "About"), key(&b, "Report a Bug"));
    assert_eq!(letters, (Some("A".to_owned()), Some("R".to_owned())));
    b.typed("rep");
    assert!(b.shown("Arrange › Report Layout") && b.shown("Help › Report a Bug"));
    let check = |b: &Bar, what: &str| {
        assert_eq!((key(b, "About"), key(b, "Report a Bug")), letters, "Help's rows keep their letters ({what})");
        assert_eq!(key(b, "Arrange › Report Layout"), None, "a match has no mnemonic ({what})");
        assert_eq!(key(b, "Help › Report a Bug"), None, "a match has no mnemonic ({what})");
    };
    check(&b, "field focused");
    b.ctx.memory_mut(|m| {
        if let Some(id) = m.focused() {
            m.surrender_focus(id);
        }
    });
    b.run(3);
    assert!(!b.ctx.text_edit_focused());
    check(&b, "field not focused");
    b.letter('a');
    assert_eq!(b.take(), ["help.about"], "A runs About, not the Arrange match");
}

impl Bar {
    /// The state of the first title's menu.
    fn nav(&self) -> crate::menu::nav::Nav {
        let popup = Id::unique("craft-ui-menu-bar").with(("title", 0)).with("popup");
        crate::menu::nav::Nav::load(&self.ctx, popup, false)
    }

    /// Whether the row `label` lies wholly in the visible part of menu level `level`, itself in the
    /// window.
    fn visible(&self, level: usize, label: &str) -> bool {
        let Some(view) = self.nav().views.get(level).copied() else { return false };
        let Some(b) = self.node(label).and_then(accesskit::Node::bounds) else { return false };
        let row = Rect::from_min_max(pos2(b.x0 as f32, b.y0 as f32), pos2(b.x1 as f32, b.y1 as f32));
        view.expand(0.5).contains_rect(row) && self.ctx.content_rect().expand(0.5).contains_rect(view)
    }

    fn wheel(&mut self, dy: f32) {
        self.events.push(Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: vec2(0.0, dy),
            modifiers: Modifiers::NONE,
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Unknown,
        });
        self.run(2);
    }
}

/// A menu far taller than its 300 pt window, open.
fn open_tall() -> Bar {
    let rows = (0..60).map(|i| item(&format!("Item {i}"), "tall")).collect();
    let mut b = Bar::new(vec![MenuItem::submenu("Long", rows)], vec2(600.0, 300.0));
    b.click("Long");
    b.run(3);
    assert_eq!(b.nav().rows.first().map(Vec::len), Some(60));
    b
}

#[test]
fn a_tall_menu_stays_in_the_window_below_the_bar_and_shows_scroll_strips() {
    let b = open_tall();
    let view = b.nav().views[0];
    assert!(!b.visible(0, "Item 59"), "the menu overflows");
    assert!(view.bottom() <= 300.0 && view.top() >= b.rect("Long").bottom(), "{view:?}");
    assert!(b.shown("Scroll menu down") && b.shown("Scroll menu up"));
}

#[test]
fn every_row_of_a_tall_menu_is_reachable_with_the_arrow_keys() {
    let mut b = open_tall();
    for i in 0..60 {
        b.key(Key::ArrowDown);
        // The scroll lands a pass after the row is revealed.
        b.run(2);
        assert_eq!(b.nav().highlighted(0), Some(i));
        assert!(b.visible(0, &format!("Item {i}")), "row {i} is scrolled into view");
    }
    b.key(Key::ArrowDown);
    b.run(2);
    assert_eq!(b.nav().highlighted(0), Some(0));
    assert!(b.visible(0, "Item 0"), "↓ wraps to the first row, scrolling back up");
}

#[test]
fn the_wheel_scrolls_a_tall_menu_over_its_rows_and_its_strips() {
    let mut b = open_tall();
    let view = b.nav().views[0];
    b.move_to(view.center());
    for _ in 0..60 {
        if b.visible(0, "Item 59") {
            break;
        }
        b.wheel(-60.0);
    }
    assert!(b.visible(0, "Item 59"), "the wheel reaches the last row");
    assert!(b.open(), "the menu stays open while scrolling");
    // At the end the ▼ strip is dead, so resting on it scrolls nothing; the wheel there does.
    let strip = b.rect("Scroll menu down");
    b.move_to(strip.center());
    b.run(10);
    assert!(b.visible(0, "Item 59"));
    b.wheel(120.0);
    assert!(!b.visible(0, "Item 59"), "the wheel over a strip scrolls the menu");
}

#[test]
fn resting_on_the_scroll_strips_scrolls() {
    let mut b = open_tall();
    let strip = b.rect("Scroll menu down");
    b.move_to(strip.center());
    for _ in 0..240 {
        if b.visible(0, "Item 59") {
            break;
        }
        b.frame();
    }
    assert!(b.visible(0, "Item 59"), "resting on ▼ scrolls to the end");
    let strip = b.rect("Scroll menu up");
    b.move_to(strip.center());
    for _ in 0..240 {
        if b.visible(0, "Item 0") {
            break;
        }
        b.frame();
    }
    assert!(b.visible(0, "Item 0"), "resting on ▲ scrolls back to the top");
}

/// A submenu taller than the room below its row scrolls between the bar and the window's bottom,
/// rather than egui sliding it up over the bar's titles.
#[test]
fn a_tall_submenu_never_covers_the_bar() {
    let rows = (0..60).map(|i| item(&format!("More {i}"), "more")).collect();
    let menu = MenuItem::submenu("Long", vec![item("First", "first"), MenuItem::submenu("More", rows).into()]);
    let mut b = Bar::new(vec![menu], vec2(600.0, 300.0));
    b.click("Long");
    b.key(Key::ArrowDown);
    b.key(Key::ArrowDown);
    b.key(Key::ArrowRight);
    b.run(3);
    let view = b.nav().views.get(1).copied().unwrap_or(Rect::NOTHING);
    assert!(b.shown("More 0") && !b.visible(1, "More 59"), "the submenu is open and overflows");
    assert!(view.top() >= b.rect("Long").bottom() - 0.5 && view.bottom() <= 300.0, "{view:?}");
}

/// A menu reopened scrolled down has rows scrolled out of view above it, over the bar: the press
/// on the title that reopens it doesn't open the submenu of the hidden row under the pointer, and
/// its release leaves the menu open.
#[test]
fn a_row_scrolled_out_of_view_under_the_title_opens_no_submenu() {
    let rows = (0..60).map(|i| MenuItem::submenu(format!("Sub {i}"), vec![item(&format!("Leaf {i}"), "leaf")]).into()).collect();
    let mut b = Bar::new(vec![MenuItem::submenu("Long", rows)], vec2(600.0, 300.0));
    b.click("Long");
    let view = b.nav().views[0];
    b.move_to(view.center());
    for _ in 0..60 {
        if b.visible(0, "Sub 59") {
            break;
        }
        b.wheel(-60.0);
    }
    assert!(b.visible(0, "Sub 59"));
    b.click("Long");
    assert!(!b.open(), "a press on the open title closes the menu");
    let p = b.point("Long");
    b.move_to(p);
    b.button(true);
    b.run(3);
    assert!(b.open(), "the press reopens it");
    assert!(!b.nav().views[0].contains(p), "the title is outside the menu's view");
    assert!(!b.tree.iter().any(|(_, n)| n.label().is_some_and(|l| l.starts_with("Leaf"))), "no submenu opened under the title");
    b.button(false);
    b.run(3);
    assert!(b.open(), "the release on the title leaves the menu open");
}

/// `any_open` is true while a bar's menu is open, as the app's shortcut handling reads it before
/// drawing the bar.
#[test]
fn any_open_is_true_while_a_bar_menu_is_open() {
    let mut b = Bar::wide();
    assert!(!b.open_before);
    b.click("File");
    assert!(b.open_before, "File is open");
    b.key(Key::Escape);
    assert!(!b.open_before, "closed");
    b.key(Key::F10);
    assert!(!b.open_before, "the focused bar with no menu open is is_focused's, not any_open's");
}

/// With nothing focused, AccessKit's focus follows the keyboard: the title Alt lights, then the row
/// the open menu lights. Escape gives it back to the root.
#[test]
fn with_nothing_focused_accesskit_focus_follows_the_lit_title_and_row() {
    let mut b = Bar::wide();
    let id = |b: &Bar, label: &str, role: Role| b.tree.iter().find(|(_, n)| n.label() == Some(label) && n.role() == role).map(|(id, _)| *id);
    b.alt_tap();
    assert!(is_focused(&b.ctx) && b.ctx.memory(|m| m.focused()).is_none(), "the bar has the keys, egui focus is on nothing");
    assert_eq!(b.accesskit_focus, id(&b, "File", Role::MenuItem), "the lit title");
    b.key(Key::ArrowDown);
    assert!(b.shown("New"));
    assert_eq!(b.accesskit_focus, id(&b, "New", Role::MenuItem), "the menu's lit row");
    b.key(Key::ArrowDown);
    assert_eq!(b.accesskit_focus, id(&b, "Open…", Role::MenuItem), "the next row");
    b.key(Key::Escape);
    b.key(Key::Escape);
    assert!(!b.open() && b.focus().is_none());
    let root = b.tree.iter().find(|(_, n)| n.role() == Role::Window).map(|(id, _)| *id);
    assert_eq!(b.accesskit_focus, root, "back to the root");
}
