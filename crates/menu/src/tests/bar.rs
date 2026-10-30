//! The in-window bar as an adapter: the model becomes craft-ui menu items, the gate and target
//! reach the bar, and the commands chosen come back. The bar's own behaviour is tested in
//! `craft_ui::menu`.

use egui::accesskit::{self, NodeId, Role, Toggled};
use egui::os::OperatingSystem as Os;
use egui::{Event, Key, Modifiers, Pos2, Rect, Vec2, pos2, vec2};

use super::*;
use crate::bar::{Labeled, is_focused, is_open};

fn item(id: &str, label: &str) -> Item {
    Item::new(id, label)
}

/// A small app's menus, as a desktop layout leaves them.
fn sample() -> MenuBar {
    MenuBar {
        menus: vec![
            Menu::new(
                "File",
                vec![
                    item("file.new", "New").shortcut(Chord::parse("Cmd+N").unwrap()).into(),
                    item("file.open", "Open…").into(),
                    Node::Separator,
                    Node::submenu("Open Recent", vec![item("file.recent.a", "a.txt").into()]),
                    Node::submenu("Nothing Here", vec![item("file.gone", "Gone").enabled(false).into()]),
                    Node::Separator,
                    item("file.locked", "Locked").enabled(false).into(),
                    Node::Standard { kind: Standard::Zoom, label: "Zoom".to_owned() },
                    Item::new(Builtin::Minimize.command(), "Minimize").into(),
                    Item::new(Builtin::Quit.command(), "Quit").into(),
                ],
            )
            .role(MenuRole::File),
            Menu::new("Edit", vec![item("edit.undo", "Undo").into(), item("edit.redo", "Redo").into()]).role(MenuRole::Edit),
            Menu::new(
                "View",
                vec![
                    item("view.grid", "Grid").check(true).shortcut(Chord::parse("G").unwrap()).into(),
                    item("view.rulers", "Rulers").check(false).into(),
                    Node::Separator,
                    item("view.light", "Light").radio(true).into(),
                    item("view.dark", "Dark").radio(false).into(),
                ],
            )
            .role(MenuRole::View),
            Menu::new("Help", vec![item("help.about", "About").into()]).role(MenuRole::Help),
        ],
        documents: Vec::new(),
    }
}

/// The bar alone in a window, driven frame by frame with synthetic input.
struct Bar {
    ctx: egui::Context,
    bar: MenuBar,
    gate: Gate,
    target: Target,
    english: Option<fn(Labeled<'_>) -> Option<String>>,
    preferred: Vec<(&'static str, char)>,
    label: Option<fn(&'static str) -> String>,
    size: Vec2,
    time: f64,
    events: Vec<Event>,
    pointer: Pos2,
    tree: Vec<(NodeId, accesskit::Node)>,
    clicked: Vec<CommandRef>,
    /// `shrink_to_fit`'s limit; the bar then shares a row with a label, which starts at `after`.
    fit: Option<f32>,
    after: Option<f32>,
    /// The viewport commands the bar sent.
    sent: Vec<egui::ViewportCommand>,
    search: bool,
    titles: Rect,
}

impl Bar {
    fn new(bar: MenuBar, size: Vec2) -> Bar {
        let ctx = egui::Context::default();
        // Not the host's: the web target's key names follow it.
        ctx.set_os(Os::Nix);
        ctx.enable_accesskit();
        ctx.all_styles_mut(|s| s.scroll_animation = egui::style::ScrollAnimation::none());
        let mut b = Bar {
            ctx,
            bar,
            gate: Gate::default(),
            target: Target::Linux,
            english: None,
            preferred: Vec::new(),
            label: None,
            size,
            time: 0.0,
            events: Vec::new(),
            pointer: pos2(-10.0, -10.0),
            tree: Vec::new(),
            clicked: Vec::new(),
            fit: None,
            after: None,
            sent: Vec::new(),
            search: false,
            titles: Rect::NOTHING,
        };
        b.run(3);
        b
    }

    fn frame(&mut self) {
        let dt = 1.0 / 60.0;
        self.time += dt;
        let raw = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, self.size)),
            time: Some(self.time),
            predicted_dt: dt as f32,
            events: std::mem::take(&mut self.events),
            ..Default::default()
        };
        let (bar, gate, target, english, preferred, label, fit) =
            (&self.bar, self.gate, self.target, self.english, &self.preferred, self.label, self.fit);
        let search = self.search;
        let mut out = Vec::new();
        let mut titles = Rect::NOTHING;
        let mut after = None;
        let mut full = self.ctx.run_ui(raw, |ui| {
            let hook = |s: &'static str| label.map_or_else(|| s.to_owned(), |f| f(s));
            let mut ui_bar = MenuBarUi::new(bar).gate(gate).target(target).mnemonics(preferred).label(&hook).help_search(search);
            if let Some(english) = &english {
                ui_bar = ui_bar.english(english);
            }
            match fit {
                Some(max) => {
                    ui.horizontal(|ui| {
                        let shown = ui_bar.shrink_to_fit(max).show(ui);
                        (out, titles) = (shown.commands, shown.titles);
                        after = Some(ui.label("after").rect.left());
                    });
                }
                None => {
                    let shown = ui_bar.show(ui);
                    (out, titles) = (shown.commands, shown.titles);
                }
            }
        });
        full.textures_delta.clear();
        self.after = after;
        self.titles = titles;
        if let Some(root) = full.viewport_output.remove(&egui::ViewportId::ROOT) {
            self.sent.extend(root.commands);
        }
        if let Some(update) = full.platform_output.accesskit_update {
            self.tree = update.nodes;
        }
        self.clicked.extend(out);
    }

    fn run(&mut self, frames: usize) {
        for _ in 0..frames {
            self.frame();
        }
    }

    /// The commands chosen since the last call.
    fn take(&mut self) -> Vec<CommandRef> {
        std::mem::take(&mut self.clicked)
    }

    fn node(&self, label: &str) -> Option<&accesskit::Node> {
        self.tree.iter().map(|(_, n)| n).find(|n| n.label() == Some(label))
    }

    fn shown(&self, label: &str) -> bool {
        self.node(label).is_some()
    }

    /// A title of the bar (not a submenu row of the same name).
    fn title(&self, label: &str) -> Option<&accesskit::Node> {
        let (_, bar) = self.tree.iter().find(|(_, n)| n.role() == Role::MenuBar)?;
        self.tree.iter().filter(|(id, _)| bar.children().contains(id)).map(|(_, n)| n).find(|n| n.label() == Some(label))
    }

    fn point(&self, label: &str) -> Pos2 {
        let n = self.node(label).unwrap_or_else(|| panic!("no {label:?}"));
        let b = n.bounds().unwrap_or_else(|| panic!("{label:?} has no bounds"));
        let r = Rect::from_min_max(pos2(b.x0 as f32, b.y0 as f32), pos2(b.x1 as f32, b.y1 as f32));
        pos2(r.left() + (r.width() / 2.0).min(20.0), r.center().y)
    }

    fn move_to(&mut self, p: Pos2) {
        self.pointer = p;
        self.events.push(Event::PointerMoved(p));
        self.frame();
    }

    fn button(&mut self, pressed: bool) {
        self.events.push(Event::PointerButton { pos: self.pointer, button: egui::PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
        self.frame();
    }

    fn click(&mut self, label: &str) {
        let p = self.point(label);
        self.move_to(p);
        self.button(true);
        self.button(false);
        self.run(3);
    }

    fn key_with(&mut self, key: Key, modifiers: Modifiers) {
        for pressed in [true, false] {
            self.events.push(Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers: if pressed { modifiers } else { Modifiers::NONE },
            });
        }
        self.run(3);
    }

    fn key(&mut self, key: Key) {
        self.key_with(key, Modifiers::NONE);
    }

    /// A tap on Alt: down, then up with nothing else pressed between.
    fn alt_tap(&mut self) {
        for (pressed, modifiers) in [(true, Modifiers::ALT), (false, Modifiers::NONE)] {
            self.events.push(Event::Key { key: Key::AltLeft, physical_key: None, pressed, repeat: false, modifiers });
            self.run(2);
        }
        self.run(1);
    }

    /// A letter typed: its key and the text it types.
    fn letter(&mut self, c: char) {
        let key = Key::from_name(&c.to_ascii_uppercase().to_string()).unwrap();
        self.events.push(Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE });
        self.events.push(Event::Text(c.to_ascii_lowercase().to_string()));
        self.events.push(Event::Key { key, physical_key: None, pressed: false, repeat: false, modifiers: Modifiers::NONE });
        self.run(3);
    }
}

fn wide() -> Bar {
    Bar::new(sample(), vec2(800.0, 600.0))
}

#[test]
fn clicking_an_item_returns_its_command_and_closes_the_menu() {
    let mut b = wide();
    assert!(!b.shown("New"));
    b.click("File");
    assert!(b.shown("New") && is_open(&b.ctx), "a click on a title opens its menu");
    b.click("New");
    assert_eq!(b.take(), vec![CommandRef::new("file.new")]);
    assert!(!b.shown("New") && !is_open(&b.ctx), "choosing an item closes the menu");
}

#[test]
fn the_models_check_radio_and_submenu_rows_reach_the_bar() {
    let mut b = wide();
    b.click("View");
    let state = |label: &str| b.node(label).map(|n| (n.role(), n.toggled()));
    assert_eq!(state("Grid"), Some((Role::MenuItemCheckBox, Some(Toggled::True))));
    assert_eq!(state("Rulers"), Some((Role::MenuItemCheckBox, Some(Toggled::False))));
    assert_eq!(state("Light"), Some((Role::MenuItemRadio, Some(Toggled::True))));
    assert_eq!(state("Dark"), Some((Role::MenuItemRadio, Some(Toggled::False))));
    b.key(Key::Escape);
    b.click("File");
    assert!(!b.shown("Zoom"), "AppKit's own items are left out");
    b.click("Open Recent");
    b.click("a.txt");
    assert_eq!(b.take(), vec![CommandRef::new("file.recent.a")], "an item of a submenu returns its command");
}

#[test]
fn disabled_and_gate_forbidden_items_return_nothing() {
    let mut b = wide();
    b.click("File");
    b.click("Locked");
    assert!(b.take().is_empty(), "a disabled item");
    assert!(b.node("Locked").is_some_and(|n| n.is_disabled()));
    assert!(b.node("Nothing Here").is_some_and(|n| n.is_disabled()), "a submenu of disabled items is disabled");

    b.key(Key::Escape);
    b.gate = Gate { modal: true, ..Gate::default() };
    b.click("File");
    assert!(b.node("New").is_some_and(|n| n.is_disabled()), "a modal dialog disables the app's items");
    b.click("New");
    assert!(b.take().is_empty(), "and they don't run");
    b.click("Quit");
    assert!(b.take().is_empty(), "the bar runs the built-in Quit itself");
    assert!(b.sent.contains(&egui::ViewportCommand::Close), "and it still runs: the window is asked to close");

    b.gate = Gate { text_editing: true, ..Gate::default() };
    b.click("File");
    b.click("New");
    assert_eq!(b.take(), vec![CommandRef::new("file.new")], "a text field having the keys doesn't stop a click");
}

#[test]
fn a_gate_forbidden_item_doesnt_run_from_the_keys() {
    let mut b = wide();
    b.gate = Gate { recording_shortcut: true, ..Gate::default() };
    b.click("Edit");
    b.key(Key::ArrowDown);
    b.key(Key::Enter);
    assert!(b.take().is_empty());
}

#[test]
fn minimize_runs_in_the_bar_and_is_not_returned() {
    let mut b = wide();
    b.click("File");
    b.click("Minimize");
    assert!(b.take().is_empty());
    assert!(b.sent.contains(&egui::ViewportCommand::Minimized(true)));
}

#[test]
fn a_dialog_or_a_recorder_keeps_the_keys_but_a_text_field_does_not_stop_alt() {
    let mut b = wide();
    b.alt_tap();
    assert!(is_focused(&b.ctx), "Alt takes the keyboard onto the bar");
    b.key(Key::Escape);

    for gate in [Gate { modal: true, ..Gate::default() }, Gate { recording_shortcut: true, ..Gate::default() }] {
        b.gate = gate;
        b.alt_tap();
        assert!(!is_focused(&b.ctx), "{gate:?}: a dialog keeps Enter and Esc");
        b.key_with(Key::F, Modifiers::ALT);
        assert!(!is_open(&b.ctx), "{gate:?}: nor does Alt+F open a menu");
    }

    b.gate = Gate { text_editing: true, ..Gate::default() };
    b.alt_tap();
    assert!(is_focused(&b.ctx), "a tap on Alt still focuses the bar from a text field");
    b.letter('e');
    assert!(!is_focused(&b.ctx) && !is_open(&b.ctx), "a letter goes on to the field and leaves the bar");
}

#[test]
fn the_app_can_prefer_letters_and_name_english_labels_for_letterless_ones() {
    let mut b = wide();
    b.preferred = vec![("View", 'W'), ("file.open", 'P')];
    b.english = Some(|l: Labeled<'_>| match l {
        Labeled::Title("帮助") => Some("Help".to_owned()),
        Labeled::Command(c) if c.id == "file.new" => Some("New".to_owned()),
        _ => None,
    });
    b.bar.menus[3].title = "帮助".to_owned();
    b.bar.menus[0].children[0] = item("file.new", "新建…").into();
    b.run(2);
    assert!(b.title("帮助(H)").is_some(), "a letterless title shows its English key");
    b.key_with(Key::W, Modifiers::ALT);
    assert!(b.shown("Grid"), "View took the app's W");
    b.key_with(Key::F, Modifiers::ALT);
    assert!(b.shown("新建(N)…"), "the key goes before the ellipsis");
    assert_eq!(b.node("Open…").and_then(|n| n.access_key()), Some("P"), "a command's preferred letter");
    b.letter('n');
    assert_eq!(b.take(), vec![CommandRef::new("file.new")]);
}

#[test]
fn the_label_hook_names_the_bars_own_labels() {
    let mut b = Bar::new(sample(), vec2(20.0, 400.0));
    b.label = Some(|english| if english == "Menu" { "Menü".to_owned() } else { english.to_owned() });
    b.run(2);
    assert!(b.title("Menü").is_some(), "one Menu holds all when none fits");
}

#[test]
fn titles_that_dont_fit_move_into_the_overflow_menu() {
    let narrow = {
        let b = wide();
        let (file, edit) = (b.point("File"), b.point("Edit"));
        edit.x - file.x + 60.0
    };
    let mut b = Bar::new(sample(), vec2(narrow, 400.0));
    assert!(b.title("Help").is_none() && b.title("…").is_some(), "Help moved into …");
    b.click("…");
    assert!(b.shown("View") && b.shown("Help"), "… lists the menus that don't fit");
}

#[test]
fn accesskit_sees_the_shortcut_in_the_platforms_names() {
    let mut b = wide();
    b.click("View");
    assert_eq!(b.node("Grid").and_then(|n| n.keyboard_shortcut()), Some("G"), "a display-only shortcut too");
    b.key(Key::Escape);
    b.click("File");
    assert_eq!(b.node("New").map(|n| (n.role(), n.keyboard_shortcut())), Some((Role::MenuItem, Some("Ctrl+N"))));
}

/// In a browser the names are those of the system it runs on (AccessKit gets the spoken form,
/// `Cmd+N`; the row shows ⌘N), and on a Mac there are no mnemonics: Option types there.
#[test]
fn the_web_shows_the_browsers_system_names_and_the_mac_its_glyphs() {
    for (target, os, text, mnemonic) in [
        (Target::Web, Os::Nix, "Ctrl+N", true),
        (Target::Web, Os::Mac, "Cmd+N", false),
        (Target::Windows, Os::Nix, "Ctrl+N", true),
        (Target::Mac, Os::Nix, "Cmd+N", false),
    ] {
        let mut b = wide();
        b.ctx.set_os(os);
        b.target = target;
        b.click("File");
        let new = b.node("New");
        assert_eq!(new.and_then(|n| n.keyboard_shortcut()), Some(text), "{target:?} on {os:?}");
        assert_eq!(new.and_then(|n| n.access_key()).is_some(), mnemonic, "{target:?} on {os:?}");
    }
}

/// In a browser on a Mac the in-window bar is the only one: F10 reaches it, while Alt (Option)
/// types.
#[test]
fn in_a_browser_on_a_mac_f10_reaches_the_bar_and_alt_does_not() {
    let mut b = wide();
    b.ctx.set_os(Os::Mac);
    b.target = Target::Web;
    b.alt_tap();
    assert!(!is_focused(&b.ctx), "Alt leaves the bar alone");
    b.key(Key::F10);
    assert!(is_focused(&b.ctx), "F10 puts the keyboard on the bar");
    b.key(Key::ArrowDown);
    assert!(is_open(&b.ctx) && b.shown("New"), "↓ opens File");
}

#[test]
fn a_mac_target_has_no_mnemonics_and_leaves_alt_and_f10_alone() {
    let mut b = wide();
    b.target = Target::Mac;
    b.alt_tap();
    b.key(Key::F10);
    assert!(!is_focused(&b.ctx), "Alt and F10 leave the bar alone");
    b.key_with(Key::F, Modifiers::ALT);
    assert!(!is_open(&b.ctx), "Option+F is a typing key");
    b.click("File");
    b.letter('n');
    assert!(b.take().is_empty() && b.shown("New"), "letters run nothing");
}

/// The layouts add the open documents (radio items, the one in front selected) and the app's
/// disabled placeholder for a dynamic list with nothing in it.
#[test]
fn documents_and_placeholders_from_the_layout_are_rows_of_the_bar() {
    let bar = MenuBar {
        menus: vec![
            Menu::new("File", vec![Node::submenu("Open Recent", vec![item("file.no_recent", "No Recent Files").enabled(false).into()])])
                .role(MenuRole::File),
            Menu::new("Window", Vec::new()).role(MenuRole::Window),
        ],
        documents: vec![
            Document::new("doc.0", "One"),
            Document::new("doc.1", "Two").active(true),
            Document { hidden: true, ..Document::new("doc.2", "Hidden") },
        ],
    };
    let laid_out = layout(&bar, Target::Linux, &LayoutOptions::new("Sample")).bar;
    let mut b = Bar::new(laid_out, vec2(800.0, 600.0));
    b.click("Window");
    assert_eq!(b.node("One").map(|n| (n.role(), n.toggled())), Some((Role::MenuItemRadio, Some(Toggled::False))));
    assert_eq!(b.node("Two").map(|n| (n.role(), n.toggled())), Some((Role::MenuItemRadio, Some(Toggled::True))));
    assert!(!b.shown("Hidden"), "a hidden document isn't listed");
    b.click("One");
    assert_eq!(b.take(), vec![CommandRef::new("doc.0")], "choosing a document runs its command");
    b.click("File");
    assert!(b.node("Open Recent").is_some_and(|n| n.is_disabled()), "a list holding only the disabled placeholder is disabled");
    b.click("Open Recent");
    assert!(b.take().is_empty() && !b.shown("No Recent Files"));
}

/// `shrink_to_fit` leaves the rest of the row to other controls.
#[test]
fn a_bar_shrunk_to_fit_shares_its_row() {
    let mut b = wide();
    b.fit = Some(f32::INFINITY);
    b.run(2);
    let after = b.after.expect("the label after the bar");
    assert!(after < b.size.x / 2.0, "the bar takes only its titles' width: {after}");
    assert!(b.title("Help").is_some() && b.title("…").is_none());
    b.fit = Some(100.0);
    b.run(2);
    assert!(b.title("…").is_some(), "the menus past 100 points overflow");
    // A limit an app computed from a full row can go negative.
    b.fit = Some(-20.0);
    b.run(2);
}

/// The tint and the tooltip of a model item reach the row: the tooltip is its accessible
/// description. The Help menu opens with the search field, which finds an item by its label.
#[test]
fn tooltips_reach_the_rows_and_the_help_search_finds_items() {
    let mut b = wide();
    assert!(!b.titles.is_negative() && b.titles.width() > 0.0, "the titles' rect is returned: {:?}", b.titles);
    b.bar.menus[1].children[0] =
        Node::Item(item("edit.undo", "Undo").tint(craft_theme::Srgba::new(1.0, 0.0, 0.0, 1.0)).tooltip("Take back the last change"));
    b.search = true;
    b.run(2);
    b.click("Edit");
    assert_eq!(b.node("Undo").and_then(|n| n.description()), Some("Take back the last change"));
    b.click("Edit");
    b.click("Help");
    for c in "redo".chars() {
        b.letter(c);
    }
    assert!(b.shown("Edit › Redo"), "the search lists the item by its label");
    b.key(Key::Enter);
    assert_eq!(b.take(), [CommandRef::from("edit.redo")]);
}
