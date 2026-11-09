//! The Mac, Windows, Linux and web layouts, platform filtering, and the keymap adapter.

use craft_keymap::{Binding, CommandDef, Keymap, Platform, Policy, Scope, Scopes};
use craft_menu::{
    Builtin, Document, Item, ItemKind, ItemRole, LayoutOptions, Menu, MenuBar, MenuRole, Node, Platforms, Standard, Target, apply_keymap, from_items,
    layout, mac_layout, structure_key, tidy, window_key,
};
use craft_ui::testing::trial;

use crate::sample::{chord, cmd, ids, row, sample, sep, titled};

#[trial]
fn the_app_menu_is_laid_out_the_mac_way() {
    let l = mac_layout(&sample(false, true), &LayoutOptions::new("Sample"));
    assert!(l.clashes.is_empty(), "{:?}", l.clashes);
    let app = &l.bar.menus[0];
    assert_eq!((app.title.as_str(), app.role), ("Sample", MenuRole::App));
    #[rustfmt::skip]
    let expected = [
        "help.about", "---", "[Settings]", "[Language]", "[Appearance]", "---",
        "<Services>", "---", Builtin::Hide.id(), Builtin::HideOthers.id(), "<ShowAll>", "---", "file.exit",
    ];
    assert_eq!(ids(&app.children), expected);
    let quit = l.bar.find(&cmd("file.exit")).unwrap();
    assert_eq!((quit.label.as_str(), quit.shortcut), ("Quit Sample", Some(chord("Cmd+Q"))), "the app's own Quit");
    assert_eq!(l.bar.find(&cmd("help.about")).unwrap().label, "About Sample");
    assert_eq!(l.bar.find(&Builtin::Hide.command()).unwrap().shortcut, Some(chord("Cmd+H")));
    assert_eq!(l.bar.find(&Builtin::HideOthers.command()).unwrap().shortcut, Some(chord("Cmd+Alt+H")));
    assert_eq!(l.bar.find(&cmd("prefs.general")).unwrap().shortcut, Some(chord("Cmd+,")), "⌘, opens the first settings page");
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
#[trial]
fn an_untagged_quit_is_craft_quits_and_labels_are_translated() {
    let mut bar = sample(false, false);
    bar.menus.retain(|m| m.role != MenuRole::Window);
    let german = |s: &'static str| match s {
        "Quit {app}" => "{app} beenden".to_string(),
        "Window" => "Fenster".into(),
        "Services" => "Dienste".into(),
        other => other.into(),
    };
    let l = mac_layout(&bar, &LayoutOptions::new("Sample").label(&german));
    let quit = l.bar.find(&Builtin::Quit.command()).unwrap();
    assert_eq!((quit.label.as_str(), quit.shortcut, quit.role), ("Sample beenden", Some(chord("Cmd+Q")), Some(ItemRole::Quit)));
    assert!(ids(&titled(&l.bar, "File").children).contains(&"file.exit".to_string()), "File › Exit stays");
    let app = l.bar.menu(MenuRole::App).unwrap();
    assert!(app.children.contains(&Node::standard(Standard::Services, "Dienste")));
    let titles: Vec<&str> = l.bar.menus.iter().map(|m| m.title.as_str()).collect();
    assert_eq!(titles, ["Sample", "File", "Edit", "View", "Fenster", "Help"], "a Window menu before Help");
    assert_eq!(ids(&l.bar.menu(MenuRole::Window).unwrap().children), ["<Minimize>", "<Zoom>", "---", "<BringAllToFront>", "---", "doc.0"]);
}

/// App commands keep ⌘H, ⌘M, ⌘W and ⌘,: the system items give way, and each clash is reported.
#[trial]
fn clashes_are_resolved_in_the_apps_favour() {
    let l = mac_layout(&sample(true, true), &LayoutOptions::new("Sample"));
    let clash = |sc: &str| l.clashes.iter().find(|c| c.shortcut == chord(sc)).map(|c| c.command.id.as_str());
    assert_eq!(clash("Cmd+H"), Some("view.extras"));
    assert_eq!(clash("Cmd+M"), Some("view.curves"));
    assert_eq!(clash("Cmd+W"), Some("file.close"));
    assert_eq!(clash("Cmd+,"), Some("view.hideLayers"));
    assert_eq!(l.clashes.len(), 4);
    assert_eq!(l.bar.find(&Builtin::Hide.command()).unwrap().shortcut, Some(chord("Ctrl+Cmd+H")));
    assert_eq!(l.bar.find(&Builtin::Minimize.command()).unwrap().shortcut, Some(chord("Ctrl+Cmd+M")));
    assert_eq!(l.bar.find(&cmd("prefs.general")).unwrap().shortcut, None);
    assert_eq!(ids(&l.bar.menu(MenuRole::Window).unwrap().children)[..2], [Builtin::Minimize.id(), "<Zoom>"]);
    assert!(!ids(&titled(&l.bar, "File").children).contains(&"<CloseWindow>".to_string()), "⌘W stays File › Close");

    // With ⌃⌘H taken too, Hide goes without a shortcut.
    let mut bar = sample(true, true);
    bar.menus[0].children.push(Item::new("file.other", "Other").shortcut(chord("Ctrl+Cmd+H")).into());
    let l = mac_layout(&bar, &LayoutOptions::new("Sample"));
    assert_eq!(l.bar.find(&Builtin::Hide.command()).unwrap().shortcut, None);
    assert_eq!(l.clashes.iter().find(|c| c.shortcut == chord("Cmd+H")).map(|c| c.resolution), Some("Hide has no shortcut"));
}

#[trial]
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
    assert_eq!(tidy(nested), [], "a submenu left empty goes");
}

/// Labels, enabled and checked are updated in place; a shortcut or a check item that used to be
/// plain needs a rebuild. A document opening or closing rebuilds only the Window menu.
#[trial]
fn keys_separate_rebuilds_from_updates_in_place() {
    let opts = LayoutOptions::new("Sample");
    let base = mac_layout(&sample(false, true), &opts).bar;
    let key = |bar: &MenuBar| (structure_key(bar), window_key(bar));
    let edit = |f: &dyn Fn(&mut Item)| {
        let mut bar = sample(false, true);
        for m in &mut bar.menus {
            for n in &mut m.children {
                if let Node::Item(it) = n
                    && it.command.id == "edit.undo"
                {
                    f(it);
                }
            }
        }
        mac_layout(&bar, &opts).bar
    };
    assert_eq!(key(&edit(&|it| it.label = "Undo Typing".into())), key(&base));
    assert_eq!(key(&edit(&|it| it.enabled = false)), key(&base));
    assert_ne!(structure_key(&edit(&|it| it.shortcut = Some(chord("Ctrl+Z")))), structure_key(&base));
    assert_ne!(structure_key(&edit(&|it| it.kind = ItemKind::Check(false))), structure_key(&base));

    let mut two = sample(false, true);
    two.documents.push(Document::new("doc.1", "Untitled-2"));
    let two = mac_layout(&two, &opts).bar;
    assert_eq!(structure_key(&two), structure_key(&base));
    assert_ne!(window_key(&two), window_key(&base));
    assert_eq!(ids(&two.menu(MenuRole::Window).unwrap().children).last().map(String::as_str), Some("doc.1"));
}

/// Hide Others is the crate's own item: an app command on ⌥⌘H, in a menu or bound elsewhere,
/// keeps the key, and shortcuts bound outside the menus clash like menu ones.
#[trial]
fn hide_others_and_extra_shortcuts_give_way_to_the_app() {
    let extra = [(chord("Cmd+Alt+H"), cmd("canvas.hideGuides")), (chord("Shift+Cmd+Q"), cmd("canvas.other"))];
    let l = mac_layout(&sample(false, true), &LayoutOptions::new("Sample").extra_shortcuts(&extra));
    assert_eq!(l.bar.find(&Builtin::HideOthers.command()).unwrap().shortcut, None);
    let [clash] = &l.clashes[..] else { panic!("one clash: {:?}", l.clashes) };
    assert_eq!(
        (clash.shortcut, clash.system, &clash.command, clash.resolution),
        (chord("Cmd+Alt+H"), "Hide Others", &cmd("canvas.hideGuides"), "Hide Others has no shortcut")
    );
    let extra = [(chord("Cmd+H"), cmd("canvas.hand"))];
    let l = mac_layout(&sample(false, true), &LayoutOptions::new("Sample").extra_shortcuts(&extra));
    assert_eq!(l.bar.find(&Builtin::Hide.command()).unwrap().shortcut, Some(chord("Ctrl+Cmd+H")));
}

/// An app menu in the app's own tree merges into the layout's: no second app menu.
#[trial]
fn an_apps_own_app_menu_is_merged() {
    let mut bar = sample(false, true);
    let own = vec![
        Item::new("app.discord", "Join Our Discord").into(),
        Node::submenu("Theme", vec![Item::new("theme.light", "Light").into()]).role(ItemRole::Appearance),
    ];
    bar.menus.insert(0, Menu::new("Sample", own).role(MenuRole::App));
    bar.menus.iter_mut().find(|m| m.title == "Window").unwrap().children.retain(|n| !matches!(n, Node::Submenu { .. }));
    let l = mac_layout(&bar, &LayoutOptions::new("Sample"));
    assert_eq!(l.bar.menus.iter().filter(|m| m.role == MenuRole::App).count(), 1);
    let app = l.bar.menu(MenuRole::App).unwrap();
    assert_eq!(ids(&app.children)[..7], ["help.about", "---", "[Settings]", "[Language]", "[Appearance]", "---", "app.discord"]);
}

fn menu(bar: &MenuBar, role: MenuRole) -> &Menu {
    bar.menu(role).unwrap()
}

fn has(bar: &MenuBar, id: &str) -> bool {
    bar.find(&cmd(id)).is_some()
}

#[trial]
fn windows_puts_exit_last_in_file_and_documents_in_window() {
    let l = layout(&sample(false, true), Target::Windows, &LayoutOptions::new("Sample"));
    assert!(l.clashes.is_empty());
    assert_eq!(ids(&menu(&l.bar, MenuRole::File).children), ["file.new", "---", "file.close", "---", "file.exit"]);
    assert_eq!(l.bar.find(&cmd("file.exit")).unwrap().label, "Exit");
    assert_eq!(ids(&menu(&l.bar, MenuRole::Edit).children), ["edit.undo", "edit.redo", "---", "[Language]", "---", "[Preferences]"]);
    assert_eq!(ids(&menu(&l.bar, MenuRole::Help).children), ["help.docs", "---", "help.about"]);
    assert_eq!(l.bar.find(&cmd("help.about")).unwrap().label, "About Sample");
    assert_eq!(ids(&menu(&l.bar, MenuRole::Window).children), ["[Theme]", "theme.next", "panel.layers", "---", "doc.0"]);
    assert_eq!(l.bar.find(&cmd("doc.0")).unwrap().kind, ItemKind::Check(true), "the document in front is checked");
    assert!(!has(&l.bar, Builtin::Hide.id()) && !has(&l.bar, Builtin::Minimize.id()));
    assert_eq!(l.bar.menus.len(), 5);

    let mac = mac_layout(&sample(false, true), &LayoutOptions::new("Sample")).bar;
    assert_eq!(mac.find(&cmd("doc.0")).unwrap().kind, ItemKind::Check(true));
}

/// Without the menus the moved items need, the layout adds them, with translated titles.
#[trial]
fn linux_adds_quit_and_the_menus_it_needs() {
    let view = vec![
        Item::new("help.about", "About").role(ItemRole::About).into(),
        Item::new("app.settings", "Settings").role(ItemRole::Settings).into(),
        Item::new("view.zoom", "Zoom").into(),
    ];
    let mut bar = MenuBar::new(vec![Menu::new("View", view).role(MenuRole::View)]).documents(vec![Document::new("doc.0", "Untitled")]);
    bar.documents.push(Document::new("doc.1", "Background").hidden(true));
    let label = |s: &'static str| format!("«{s}»");
    let l = layout(&bar, Target::Linux, &LayoutOptions::new("Sample").label(&label));
    let titles: Vec<&str> = l.bar.menus.iter().map(|m| m.title.as_str()).collect();
    assert_eq!(titles, ["«File»", "«Edit»", "View", "«Window»", "«Help»"]);
    let quit = &menu(&l.bar, MenuRole::File).children;
    assert_eq!(ids(quit), [Builtin::Quit.id()]);
    assert_eq!(l.bar.find(&Builtin::Quit.command()).unwrap().label, "«Quit»");
    assert_eq!(l.bar.find(&cmd("app.settings")).unwrap().label, "«Preferences…»");
    assert_eq!(l.bar.find(&cmd("help.about")).unwrap().label, "«About Sample»");
    assert_eq!(ids(&menu(&l.bar, MenuRole::Window).children), ["doc.0"], "a hidden document isn't listed");
}

#[trial]
fn the_web_has_no_quit_and_gives_the_browser_its_shortcuts() {
    let mut bar = sample(false, true);
    let file = bar.menus.iter_mut().find(|m| m.role == MenuRole::File).unwrap();
    file.children.push(Item::new("file.reopen", "Reopen").shortcut(chord("Cmd+Shift+T")).into());
    file.children.push(Item::new("file.tab", "Tab").shortcut(chord("Cmd+Alt+T")).into());
    let l = layout(&bar, Target::Web, &LayoutOptions::new("Sample"));
    assert!(!has(&l.bar, "file.exit") && !has(&l.bar, Builtin::Quit.id()));
    let clashes: Vec<(String, &str)> = l.clashes.iter().map(|c| (c.command.key(), c.system)).collect();
    assert_eq!(clashes, [("file.new".into(), "New Window"), ("file.reopen".into(), "Reopen Closed Tab")]);
    assert_eq!(l.bar.find(&cmd("file.new")).unwrap().shortcut, None);
    assert_eq!(l.bar.find(&cmd("file.tab")).unwrap().shortcut, Some(chord("Cmd+Alt+T")));
    assert_eq!(l.bar.find(&cmd("edit.undo")).unwrap().shortcut, Some(chord("Cmd+Z")));
}

/// Items and submenus show only on their platforms; a submenu left empty goes, one holding only
/// the app's disabled placeholder stays.
#[trial]
fn layouts_keep_only_what_the_target_shows() {
    let file = vec![
        Item::new("file.new", "New").into(),
        Item::new("file.tabs", "Show Tab Bar").platforms(Platforms::MAC).into(),
        Node::submenu("Export", vec![Item::new("file.export_native", "Native").platforms(Platforms::MAC).into()]),
        Node::submenu("Open Recent", vec![Item::new("file.no_recent", "No Recent Files").enabled(false).into()]),
        Node::submenu("Plugins", vec![Item::new("plugin.a", "A").into()]).platforms(Platforms::ALL.without(Platforms::WEB)),
    ];
    let bar = MenuBar::new(vec![Menu::new("File", file).role(MenuRole::File)]);
    let file_on = |target| {
        let l = layout(&bar, target, &LayoutOptions::new("Sample"));
        let file = menu(&l.bar, MenuRole::File).children.iter().take_while(|n| **n != Node::Separator).cloned().collect::<Vec<_>>();
        ids(&file)
    };
    assert_eq!(file_on(Target::Mac), ["file.new", "file.tabs", "[Export]", "[Open Recent]", "[Plugins]"]);
    assert_eq!(file_on(Target::Linux), ["file.new", "[Open Recent]", "[Plugins]"]);
    assert_eq!(file_on(Target::Web), ["file.new", "[Open Recent]"]);
}

fn keymap() -> Keymap {
    let app = Scope::app();
    let layers = Scope::new("Layers").unwrap();
    let catalog = vec![
        CommandDef::new("file.save", "Save", app.clone()).with_default(Binding::new(chord("Cmd+S"))),
        CommandDef::new("view.rulers", "Rulers", app.clone()).toggle().with_default(Binding::new(chord("Cmd+R"))),
        CommandDef::new("view.grid", "Grid", app.clone()).toggle(),
        CommandDef::new("layer.new", "New Layer", layers.clone())
            .with_default(Binding::in_scope(chord("Cmd+Shift+N"), app.clone()))
            .with_default(Binding::new(chord("Cmd+L"))),
        CommandDef::new("view.mode", "Mode", app).with_default(Binding::new(chord("M"))),
    ];
    Keymap::new(catalog, Scopes::new().with_panel("Layers"), Platform::Linux)
}

#[trial]
fn the_keymap_gives_shortcuts_and_check_items() {
    let mut keys = keymap();
    keys.set_bindings(&cmd("file.save"), vec![Binding::new(chord("Cmd+Alt+S"))], Policy::Steal).unwrap();
    let view = vec![
        Item::new("file.save", "Save").shortcut(chord("Cmd+S")).into(),
        Item::new("view.rulers", "Rulers").into(),
        Item::new("view.grid", "Grid").check(true).into(),
        Item::new("layer.new", "New Layer").into(),
        Item::new("view.mode", "Mode").radio(true).into(),
        Item::new("view.other", "Other").shortcut(chord("F9")).into(),
    ];
    let mut bar = MenuBar::new(vec![Menu::new("View", view)]);
    apply_keymap(&mut bar, &keys);
    let item = |id: &str| bar.find(&cmd(id)).unwrap().clone();
    assert_eq!(item("file.save").shortcut, Some(chord("Cmd+Alt+S")), "the user's override");
    assert_eq!((item("view.rulers").shortcut, item("view.rulers").kind), (Some(chord("Cmd+R")), ItemKind::Check(false)));
    assert_eq!((item("view.grid").shortcut, item("view.grid").kind), (None, ItemKind::Check(true)), "a toggle keeps its state");
    assert_eq!(item("layer.new").shortcut, Some(chord("Cmd+L")), "the home scope's binding first");
    assert_eq!(item("view.mode").kind, ItemKind::Radio(true));
    assert_eq!(item("view.other").shortcut, Some(chord("F9")), "a command the keymap doesn't know keeps its shortcut");
}
