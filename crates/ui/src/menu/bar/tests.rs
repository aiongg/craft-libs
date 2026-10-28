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
    chosen: Vec<&'static str>,
    /// Where a label after a shrunk bar starts.
    after: Option<f32>,
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
            chosen: Vec::new(),
            after: None,
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
        let mut full = self.ctx.run_ui(raw, |ui| {
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
        });
        full.textures_delta.clear();
        if let Some(update) = full.platform_output.accesskit_update {
            self.tree = update.nodes;
        }
        self.after = after;
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

#[test]
fn without_mnemonics_alt_and_f10_leave_the_bar_alone() {
    let mut b = Bar::wide();
    b.ctx.set_os(egui::os::OperatingSystem::Mac);
    b.alt_tap();
    b.key(Key::F10);
    assert!(b.focus().is_none() && !is_focused(&b.ctx), "Alt and F10 leave the bar alone on a Mac");
    b.key_with(Key::F, Modifiers::ALT);
    assert!(!b.open(), "Option+F types");
    assert_eq!(b.title("File").and_then(|n| n.access_key()), None);
    b.ctx.set_os(egui::os::OperatingSystem::Nix);
    b.keyboard = false;
    b.alt_tap();
    assert!(b.focus().is_none(), "nor while a dialog has the keys");
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
