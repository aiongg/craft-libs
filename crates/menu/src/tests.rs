#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use egui::{Key, Modifiers};

use super::*;

fn row(path: &[&str], id: &str, label: &str, shortcut: Option<&str>) -> Row {
    Row {
        path: path.iter().map(|s| s.to_string()).collect(),
        id: id.into(),
        label: label.into(),
        shortcut: shortcut.map(Into::into),
        enabled: true,
        checked: None,
    }
}

fn sep(path: &[&str]) -> Row {
    row(path, "", "---", None)
}

/// A small app's menus, tagged. With `clashing`, app commands take ⌘H, ⌘M, ⌘W and ⌘,.
fn sample(clashing: bool, tag_quit: bool) -> MenuBar {
    let k = |own: &'static str| clashing.then_some(own);
    let rows = vec![
        row(&["File"], "file.new", "New", Some("Cmd+N")),
        sep(&["File"]),
        row(&["File"], "file.close", "Close", k("Cmd+W")),
        sep(&["File"]),
        row(&["File"], "file.exit", "Exit", None),
        row(&["Edit"], "edit.undo", "Undo", Some("Cmd+Z")),
        row(&["Edit"], "edit.redo", "Redo", Some("Cmd+Shift+Z")),
        sep(&["Edit"]),
        row(&["Edit", "Preferences"], "prefs.general", "General", None),
        row(&["Edit", "Preferences"], "prefs.keys", "Keys", None),
        row(&["Edit", "Language"], "lang.en", "English", None),
        row(&["View"], "view.extras", "Extras", k("Cmd+H")),
        row(&["View"], "view.hideLayers", "Hide Layers", k("Cmd+,")),
        row(&["View"], "view.curves", "Curves", k("Cmd+M")),
        row(&["View"], "view.zoomIn", "Zoom In", Some("Cmd++")),
        row(&["Window", "Theme"], "theme.dark", "Dark", None),
        row(&["Window"], "theme.next", "Next Theme", None),
        row(&["Window"], "panel.layers", "Layers", Some("F7")),
        row(&["Window"], "doc.0", "Untitled-1", None),
        row(&["Help"], "help.about", "About", None),
        row(&["Help"], "help.docs", "Docs", None),
    ];
    let mut bar = from_items(&["File", "Edit", "View", "Window", "Help"], &rows);
    bar.tag_item("help.about", ItemRole::About);
    if tag_quit {
        bar.tag_item("file.exit", ItemRole::Quit);
    }
    bar.tag_item("doc.0", ItemRole::Document);
    bar.tag_item("theme.next", ItemRole::Appearance);
    bar.tag_submenu("Edit", "Preferences", ItemRole::Settings);
    bar.tag_submenu("Edit", "Language", ItemRole::Language);
    bar.tag_submenu("Window", "Theme", ItemRole::Appearance);
    bar.tag_menu("File", MenuRole::File);
    bar.tag_menu("Window", MenuRole::Window);
    bar.tag_menu("Help", MenuRole::Help);
    bar
}

fn ids(nodes: &[Node]) -> Vec<String> {
    nodes
        .iter()
        .map(|n| match n {
            Node::Item(it) => it.id.clone(),
            Node::Separator => "---".into(),
            Node::Submenu { label, .. } => format!("[{label}]"),
            Node::Standard { kind, .. } => format!("<{kind:?}>"),
        })
        .collect()
}

fn titled<'a>(bar: &'a MenuBar, title: &str) -> &'a Menu {
    bar.menus.iter().find(|m| m.title == title).unwrap()
}

#[test]
fn the_app_menu_is_laid_out_the_mac_way() {
    let l = mac_layout(&sample(false, true), &LayoutOptions::new("Sample"));
    assert!(l.clashes.is_empty(), "{:?}", l.clashes);
    let app = &l.bar.menus[0];
    assert_eq!((app.title.as_str(), app.role), ("Sample", MenuRole::App));
    #[rustfmt::skip]
    let expected = [
        "help.about", "---", "[Settings]", "[Language]", "[Appearance]", "---",
        "<Services>", "---", HIDE, "<HideOthers>", "<ShowAll>", "---", "file.exit",
    ];
    assert_eq!(ids(&app.children), expected);
    let quit = l.bar.find("file.exit").unwrap();
    assert_eq!((quit.label.as_str(), quit.shortcut.as_deref()), ("Quit Sample", Some("Cmd+Q")), "the app's own Quit");
    assert_eq!(l.bar.find("help.about").unwrap().label, "About Sample");
    assert_eq!(l.bar.find(HIDE).unwrap().shortcut.as_deref(), Some("Cmd+H"));
    assert_eq!(l.bar.find("prefs.general").unwrap().shortcut.as_deref(), Some("Cmd+,"), "⌘, opens the first settings page");
    let Some(Node::Submenu { children, .. }) = app.children.iter().find(|n| matches!(n, Node::Submenu { label, .. } if label == "Appearance")) else {
        panic!("an Appearance submenu");
    };
    assert_eq!(ids(children), ["theme.dark", "---", "theme.next"]);

    // The moved items leave their menus, and emptied separators go with them.
    let titles: Vec<&str> = l.bar.menus.iter().map(|m| m.title.as_str()).collect();
    assert_eq!(titles, ["Sample", "File", "Edit", "View", "Window", "Help"]);
    assert_eq!(ids(&titled(&l.bar, "File").children), ["file.new", "---", "file.close", "---", "<CloseWindow>"]);
    assert_eq!(ids(&titled(&l.bar, "Edit").children), ["edit.undo", "edit.redo"]);
    assert_eq!(ids(&titled(&l.bar, "Help").children), ["help.docs"]);
    assert_eq!(
        ids(&l.bar.menu(MenuRole::Window).unwrap().children),
        ["<Minimize>", "<Zoom>", "---", "panel.layers", "---", "<BringAllToFront>", "---", "doc.0"],
        "the open documents follow Bring All to Front"
    );
}

/// Without a tagged Quit the layout adds craft-quit's; the app's translations name what the
/// layout writes, AppKit's items included.
#[test]
fn an_untagged_quit_is_craft_quits_and_labels_are_translated() {
    let mut bar = sample(false, false);
    bar.menus.retain(|m| m.role != MenuRole::Window);
    let german = |s: &'static str| match s {
        "Quit {app}" => "{app} beenden".to_string(),
        "Window" => "Fenster".into(),
        "Services" => "Dienste".into(),
        other => other.into(),
    };
    let l = mac_layout(&bar, &LayoutOptions { app_name: "Sample", label: &german });
    let quit = l.bar.find(craft_quit::QUIT_MENU_ID).unwrap();
    assert_eq!((quit.label.as_str(), quit.shortcut.as_deref(), quit.role), ("Sample beenden", Some("Cmd+Q"), Some(ItemRole::Quit)));
    assert!(ids(&titled(&l.bar, "File").children).contains(&"file.exit".to_string()), "File › Exit stays");
    let app = l.bar.menu(MenuRole::App).unwrap();
    assert!(app.children.contains(&Node::Standard { kind: Standard::Services, label: "Dienste".into() }));
    let titles: Vec<&str> = l.bar.menus.iter().map(|m| m.title.as_str()).collect();
    assert_eq!(titles, ["Sample", "File", "Edit", "View", "Fenster", "Help"], "a Window menu before Help");
    assert_eq!(ids(&l.bar.menu(MenuRole::Window).unwrap().children), ["<Minimize>", "<Zoom>", "---", "<BringAllToFront>"]);
}

/// App commands keep ⌘H, ⌘M, ⌘W and ⌘,: the system items give way, and each clash is reported.
#[test]
fn clashes_are_resolved_in_the_apps_favour() {
    let l = mac_layout(&sample(true, true), &LayoutOptions::new("Sample"));
    let clash = |sc: &str| l.clashes.iter().find(|c| c.shortcut == sc).map(|c| c.command.as_str());
    assert_eq!(clash("Cmd+H"), Some("view.extras"));
    assert_eq!(clash("Cmd+M"), Some("view.curves"));
    assert_eq!(clash("Cmd+W"), Some("file.close"));
    assert_eq!(clash("Cmd+,"), Some("view.hideLayers"));
    assert_eq!(l.clashes.len(), 4);
    assert_eq!(l.bar.find(HIDE).unwrap().shortcut.as_deref(), Some("Ctrl+Cmd+H"));
    assert_eq!(l.bar.find(MINIMIZE).unwrap().shortcut.as_deref(), Some("Ctrl+Cmd+M"));
    assert_eq!(l.bar.find("prefs.general").unwrap().shortcut, None);
    assert_eq!(ids(&l.bar.menu(MenuRole::Window).unwrap().children)[..2], [MINIMIZE, "<Zoom>"]);
    assert!(!ids(&titled(&l.bar, "File").children).contains(&"<CloseWindow>".to_string()), "⌘W stays File › Close");

    // With ⌃⌘H taken too, Hide goes without a shortcut.
    let mut bar = sample(true, true);
    bar.menus[0].children.push(Item::new("file.other", "Other").shortcut("Ctrl+Cmd+H").into());
    let l = mac_layout(&bar, &LayoutOptions::new("Sample"));
    assert_eq!(l.bar.find(HIDE).unwrap().shortcut, None);
    assert_eq!(l.clashes.iter().find(|c| c.shortcut == "Cmd+H").map(|c| c.resolution), Some("Hide has no shortcut"));
}

#[test]
fn rows_become_a_tidy_tree() {
    let rows = [
        sep(&["A"]),
        row(&["A"], "a", "a", None),
        sep(&["A"]),
        sep(&["A"]),
        sep(&["A", "Sub"]),
        row(&["A", "Sub"], "b", "b", None),
        sep(&["A", "Sub"]),
    ];
    let bar = from_items(&["A", "Empty"], &rows);
    assert_eq!(bar.menus.len(), 2);
    assert_eq!(bar.menus[0].children, [Item::new("a", "a").into(), Node::Separator, Node::submenu("Sub", vec![Item::new("b", "b").into()])]);
    assert!(bar.menus[1].children.is_empty());
    let nested = vec![Node::Separator, Node::submenu("S", vec![Node::Separator, Node::Separator]), Node::Separator, Node::Separator];
    assert_eq!(tidy(nested), [Node::submenu("S", vec![])]);
}

/// Labels, enabled and checked are updated in place; a shortcut or a check item that used to be
/// plain needs a rebuild. A document opening or closing rebuilds only the Window menu.
#[test]
fn keys_separate_rebuilds_from_updates_in_place() {
    let opts = LayoutOptions::new("Sample");
    let base = mac_layout(&sample(false, true), &opts).bar;
    let key = |bar: &MenuBar| (structure_key(bar), window_key(bar));
    let edit = |f: &dyn Fn(&mut Item)| {
        let mut bar = sample(false, true);
        for m in &mut bar.menus {
            for n in &mut m.children {
                if let Node::Item(it) = n
                    && it.id == "edit.undo"
                {
                    f(it);
                }
            }
        }
        mac_layout(&bar, &opts).bar
    };
    assert_eq!(key(&edit(&|it| it.label = "Undo Typing".into())), key(&base));
    assert_eq!(key(&edit(&|it| it.enabled = false)), key(&base));
    assert_ne!(structure_key(&edit(&|it| it.shortcut = Some("Ctrl+Z".into()))), structure_key(&base));
    assert_ne!(structure_key(&edit(&|it| it.checked = Some(false))), structure_key(&base));

    let mut two = sample(false, true);
    let window = two.menus.iter_mut().find(|m| m.role == MenuRole::Window).unwrap();
    window.children.push(Item::new("doc.1", "Untitled-2").checked(true).role(ItemRole::Document).into());
    let two = mac_layout(&two, &opts).bar;
    assert_eq!(structure_key(&two), structure_key(&base));
    assert_ne!(window_key(&two), window_key(&base));
    assert_eq!(ids(&two.menu(MenuRole::Window).unwrap().children).last().map(String::as_str), Some("doc.1"));
}

#[test]
fn chords_parse_and_become_egui_key_presses() {
    let c = Chord::parse("Shift+Cmd+Z").unwrap();
    assert_eq!(c.portable(), "Cmd+Shift+Z");
    assert_eq!(Chord::parse("Ctrl+Cmd+M").unwrap().portable(), "Ctrl+Cmd+M");
    assert_eq!(Chord::parse("Cmd++").unwrap().key, "+");
    assert_eq!(Chord::parse("Cmd+A+B"), None);
    assert_eq!(Chord::parse("Cmd"), None);
    assert!(Chord::parse("Cmd+B").unwrap().native_ok());
    assert!(Chord::parse("Ctrl+B").unwrap().native_ok());
    assert!(Chord::parse("F5").unwrap().native_ok());
    assert!(!Chord::parse("B").unwrap().native_ok());
    assert!(!Chord::parse("Alt+F").unwrap().native_ok());
    assert!(!Chord::parse("Shift+X").unwrap().native_ok());
    assert!(!Chord::parse("F25").unwrap().is_function_key());

    let Some(egui::Event::Key { key, modifiers, pressed: true, .. }) = c.key_event(true) else { panic!("a key press") };
    assert_eq!(key, Key::Z);
    assert!(modifiers.command && modifiers.mac_cmd && modifiers.shift && !modifiers.ctrl && !modifiers.alt);
    let ctrl = Chord::parse("Ctrl+Cmd+M").unwrap().key_event(true);
    assert!(matches!(ctrl, Some(egui::Event::Key { modifiers, .. }) if modifiers.ctrl && modifiers.command));
    assert_eq!(Chord::parse("Cmd+NoSuchKey").unwrap().key_event(true), None);

    // Every shortcut the layout writes parses back to itself and has a key press.
    for it in mac_layout(&sample(true, true), &LayoutOptions::new("Sample")).bar.items() {
        let Some(sc) = &it.shortcut else { continue };
        let c = Chord::parse(sc).unwrap();
        assert_eq!(&c.portable(), sc);
        assert!(c.key_event(true).is_some(), "{sc} on {}", it.id);
    }
}

struct Fake {
    synced: Rc<RefCell<Vec<MenuBar>>>,
    events: Rc<RefCell<Vec<Event>>>,
}

impl Backend for Fake {
    fn sync(&mut self, bar: &MenuBar) {
        self.synced.borrow_mut().push(bar.clone());
    }
    fn drain(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events.borrow_mut())
    }
}

/// AppKit takes a key equivalent's key-down but winit still delivers the key-up: the menu adds
/// only the press, before a key-up already queued, so the shortcut runs once.
#[test]
fn key_equivalents_become_one_key_press() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut menu = NativeMenu::new(Box::new(Fake { synced: Default::default(), events: events.clone() }));
    let paste = Chord::parse("Cmd+V").unwrap();
    let release = egui::Event::Key { key: Key::V, physical_key: Some(Key::V), pressed: false, repeat: false, modifiers: Modifiers::COMMAND };
    let press = paste.key_event(true).unwrap();

    events.borrow_mut().push(Event::Key(paste.clone()));
    let mut raw = egui::RawInput { events: vec![egui::Event::ModifiersChanged(Modifiers::COMMAND), release.clone()], ..Default::default() };
    menu.raw_input(&mut raw);
    assert_eq!(raw.events, [egui::Event::ModifiersChanged(Modifiers::COMMAND), press.clone(), release]);

    events.borrow_mut().push(Event::Key(paste));
    let mut raw = egui::RawInput::default();
    menu.raw_input(&mut raw);
    assert_eq!(raw.events, [press], "no made-up release: the real one comes in a later frame");
}

/// Minimize and craft-quit's Quit run in the crate; other clicks go to the app. The menus are
/// laid out again only when the state changed or an event arrived.
#[test]
fn clicks_and_syncs() {
    let synced = Rc::new(RefCell::new(Vec::new()));
    let events =
        Rc::new(RefCell::new(vec![Event::Click(MINIMIZE.into()), Event::Click("edit.undo".into()), Event::Click(craft_quit::QUIT_MENU_ID.into())]));
    let mut menu = NativeMenu::new(Box::new(Fake { synced: synced.clone(), events: events.clone() }));
    let mut raw = egui::RawInput::default();
    menu.raw_input(&mut raw);
    assert!(raw.events.is_empty());

    let ctx = egui::Context::default();
    let mut clicks = Vec::new();
    let mut out = ctx.run_ui(raw, |ui| clicks = menu.take_clicks(ui.ctx()));
    out.textures_delta.clear(); // not painted
    assert_eq!(clicks, ["edit.undo"]);
    let commands = out.viewport_output.remove(&egui::ViewportId::ROOT).map(|o| o.commands).unwrap_or_default();
    assert!(commands.contains(&egui::ViewportCommand::Minimized(true)));
    assert!(commands.contains(&egui::ViewportCommand::Close), "Quit asks the window to close");

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
    menu.raw_input(&mut egui::RawInput::default());
    menu.sync_with(2, build);
    assert_eq!((builds.get(), synced.borrow().len()), (3, 3), "an event re-syncs");
}
