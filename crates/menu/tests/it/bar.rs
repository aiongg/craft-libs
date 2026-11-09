//! The in-window bar as an adapter: the menu model becomes command rows, the target and the app's hooks reach the
//! bar, and a choice comes back as a fired command. The bar's own behaviour is craft-ui's.

use craft_behavior::Mixed;
use craft_host::target::{Os, WindowChrome};
use craft_menu::{
    Builtin, Chord, Document, Item, LayoutOptions, Menu, MenuBar, MenuBarUi, MenuRole, Node, Standard, Target,
    bar::{is_focused, is_open},
    layout,
};
use craft_ui::contract::{Bind, Show as _};
use craft_ui::runtime::commands::{CommandState, FireSource};
use craft_ui::runtime::frame::take_output;
use craft_ui::testing::{Fixture, Frames, trial};
use craft_ui::text::TextField;
use egui::{Key, Modifiers, Ui};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn item(id: &str, label: &str) -> Item {
    Item::new(id, label)
}

/// A small app's menus, as a desktop layout leaves them.
fn sample() -> MenuBar {
    MenuBar::new(vec![
        Menu::new(
            "File",
            vec![
                item("file.new", "New").shortcut(Chord::parse("Cmd+N").unwrap()).into(),
                item("file.open", "Open…").into(),
                Node::Separator,
                Node::submenu("Open Recent", vec![item("file.recent.a", "a.txt").into()]),
                Node::submenu("Nothing Here", vec![item("file.gone", "Gone").enabled(false).into()]),
                Node::Separator,
                item("file.locked", "Locked").into(),
                Node::standard(Standard::Zoom, "Zoom"),
                Item::new(Builtin::Minimize.command(), "Minimize").into(),
                Item::new(Builtin::Quit.command(), "Quit").into(),
            ],
        )
        .role(MenuRole::File),
        Menu::new("Edit", vec![item("edit.undo", "Undo").into(), item("edit.redo", "Redo").into()]).role(MenuRole::Edit),
        Menu::new(
            "View",
            vec![
                item("view.grid", "Grid").check(true).into(),
                item("view.rulers", "Rulers").check(false).into(),
                Node::Separator,
                item("view.light", "Light").radio(true).into(),
                item("view.dark", "Dark").radio(false).into(),
            ],
        )
        .role(MenuRole::View),
        Menu::new("Help", vec![item("help.about", "About").into()]).role(MenuRole::Help),
    ])
}

/// How the bar is set up in a rig.
#[derive(Clone, Copy)]
struct Setup {
    target: Target,
    help_search: bool,
}

const LINUX: Setup = Setup { target: Target::Linux, help_search: false };

fn rig(bar: MenuBar, setup: Setup) -> Fixture {
    let mut fixture = Fixture::new(move |ui| {
        MenuBarUi::new(&bar).target(setup.target).help_search(setup.help_search).show(ui);
    });
    fixture.settle();
    fixture
}

fn click(fixture: &mut Fixture, label: &str) {
    let target = fixture.by_label(label).unwrap_or_else(|| panic!("no {label:?}"));
    fixture.click(&target);
    fixture.settle();
}

fn key(fixture: &mut Fixture, key: Key, modifiers: Modifiers) {
    fixture.key(key, modifiers);
    fixture.settle();
}

/// The commands fired since the last call, with their sources.
fn fired(fixture: &Fixture) -> Vec<(String, FireSource)> {
    take_output(fixture.ctx()).fired.into_iter().map(|fired| (fired.command.id.clone(), fired.source)).collect()
}

fn toggled(fixture: &Fixture, label: &str) -> Option<Mixed<bool>> {
    fixture.by_label(label).and_then(|target| fixture.tree().toggled_of(target.node))
}

#[trial]
fn clicking_an_item_fires_its_command_and_closes_the_menu() {
    let mut fixture = rig(sample(), LINUX);
    assert!(fixture.by_label("New").is_none());
    click(&mut fixture, "File");
    assert!(fixture.by_label("New").is_some() && is_open(fixture.ctx()), "a click on a title opens its menu");
    click(&mut fixture, "New");
    assert_eq!(fired(&fixture), [("file.new".to_owned(), FireSource::Menu)]);
    assert!(fixture.by_label("New").is_none() && !is_open(fixture.ctx()), "choosing an item closes the menu");
}

#[trial]
fn the_models_check_radio_and_submenu_rows_reach_the_bar() {
    let mut fixture = rig(sample(), LINUX);
    click(&mut fixture, "View");
    assert_eq!(toggled(&fixture, "Grid"), Some(Mixed::Same(true)));
    assert_eq!(toggled(&fixture, "Rulers"), Some(Mixed::Same(false)));
    assert_eq!(toggled(&fixture, "Light"), Some(Mixed::Same(true)));
    assert_eq!(toggled(&fixture, "Dark"), Some(Mixed::Same(false)));
    key(&mut fixture, Key::Escape, Modifiers::NONE);
    click(&mut fixture, "File");
    assert!(fixture.by_label("Zoom").is_none(), "AppKit's own items are left out");
    click(&mut fixture, "Open Recent");
    click(&mut fixture, "a.txt");
    assert_eq!(fired(&fixture), [("file.recent.a".to_owned(), FireSource::Menu)], "an item of a submenu fires its command");
}

#[trial]
fn a_check_row_follows_the_commands_state() {
    let mut fixture = rig(sample(), LINUX);
    fixture.command_state("view.rulers", CommandState::new().checked(true));
    click(&mut fixture, "View");
    assert_eq!(toggled(&fixture, "Rulers"), Some(Mixed::Same(true)), "the command's state wins over the model's");
    assert_eq!(toggled(&fixture, "Grid"), Some(Mixed::Same(true)), "the model's state stands for a command with none");
}

#[trial]
fn disabled_commands_and_dead_submenus_fire_nothing() {
    let mut fixture = rig(sample(), LINUX);
    fixture.command_state("file.locked", CommandState::new().enabled(false));
    click(&mut fixture, "File");
    click(&mut fixture, "Locked");
    assert!(fired(&fixture).is_empty(), "a disabled command");
    click(&mut fixture, "Nothing Here");
    assert!(fixture.by_label("Gone").is_none() && fired(&fixture).is_empty(), "a submenu of disabled items does not open");
}

/// A frame that sends a close command, from a bar whose File menu is clicked open and whose row `label` is clicked.
fn closes_after_clicking(label: &str) -> bool {
    let bar = sample();
    let mut show = |ui: &mut Ui| {
        MenuBarUi::new(&bar).target(Target::Linux).show(ui);
    };
    let mut frames = Frames::new();
    frames.idle(4, &mut show);
    let mut closed = false;
    for name in ["File", label] {
        let tree = frames.tree();
        let key = tree.keys().into_iter().find(|key| tree.name_of(*key) == Some(name)).unwrap_or_else(|| panic!("no {name:?}"));
        let at = tree.bounds_of(key).unwrap_or_else(|| panic!("{name:?} has no bounds")).center();
        for event in [craft_ui::testing::pointer::moved(at), craft_ui::testing::pointer::press(at), craft_ui::testing::pointer::release(at)] {
            closed |= frames.frame_without_input(vec![event], &mut show);
        }
        frames.idle(3, &mut show);
    }
    closed
}

#[trial]
fn a_quit_row_asks_the_root_window_to_close_and_fires_nothing() {
    assert!(closes_after_clicking("Quit"), "the runtime's close path is asked");
    assert!(!closes_after_clicking("New"), "other rows leave the window alone");
    {
        let mut fixture = rig(sample(), LINUX);
        click(&mut fixture, "File");
        click(&mut fixture, "Minimize");
        assert!(fired(&fixture).is_empty());
    }
}

#[trial]
fn the_app_can_prefer_letters_and_name_english_labels_for_letterless_ones() {
    let mut bar = sample();
    bar.menus[3].title = "帮助".to_owned();
    bar.menus[0].children[0] = item("file.new", "新建…").into();
    let preferred = [("View", 'W'), ("file.open", 'P')];
    let english = |labeled: craft_menu::Labeled<'_>| match labeled {
        craft_menu::Labeled::Title("帮助") => Some("Help".to_owned()),
        craft_menu::Labeled::Command(c) if c.id == "file.new" => Some("New".to_owned()),
        _ => None,
    };
    let mut fixture = Fixture::new(move |ui| {
        MenuBarUi::new(&bar).target(Target::Linux).mnemonics(&preferred).english(&english).show(ui);
    });
    fixture.settle();
    assert!(fixture.by_label("帮助(H)").is_some(), "a letterless title shows its English key");
    key(&mut fixture, Key::W, Modifiers::ALT);
    assert!(fixture.by_label("Grid").is_some(), "View took the app's W");
    key(&mut fixture, Key::Escape, Modifiers::NONE);
    key(&mut fixture, Key::F, Modifiers::ALT);
    assert!(fixture.by_label("新建(N)…").is_some(), "the key goes before the ellipsis");
    let open = fixture.by_label("Open…").expect("the Open row");
    assert_eq!(fixture.tree().access_key_of(open.node), Some('P'), "a command's preferred letter");
}

/// The web shows the names of the system the browser runs on, and a Mac has no mnemonics: Option types there.
#[trial]
fn the_web_shows_the_browsers_system_names_and_the_mac_its_glyphs() {
    for (target, os, mnemonic) in [(Target::Web, Os::Linux, true), (Target::Web, Os::Mac, false), (Target::Windows, Os::Linux, true)] {
        let mut fixture = rig(sample(), Setup { target, ..LINUX });
        fixture.os(os);
        fixture.chrome(WindowChrome::Integrated);
        fixture.settle();
        click(&mut fixture, "File");
        let new = fixture.by_label("New").expect("the New row");
        assert_eq!(fixture.tree().access_key_of(new.node).is_some(), mnemonic, "{target:?} on {os:?}");
    }
}

/// In a browser on a Mac the in-window bar is the only one: F10 reaches it, while Alt (Option) types.
#[trial]
fn in_a_browser_on_a_mac_f10_reaches_the_bar_and_alt_does_not() {
    let mut fixture = rig(sample(), Setup { target: Target::Web, ..LINUX });
    fixture.os(Os::Mac);
    fixture.chrome(WindowChrome::Integrated);
    fixture.settle();
    key(&mut fixture, Key::AltLeft, Modifiers::ALT);
    assert!(!is_focused(fixture.ctx()), "Alt leaves the bar alone");
    key(&mut fixture, Key::F10, Modifiers::NONE);
    assert!(is_focused(fixture.ctx()), "F10 puts the keyboard on the bar");
    key(&mut fixture, Key::ArrowDown, Modifiers::NONE);
    assert!(is_open(fixture.ctx()) && fixture.by_label("New").is_some(), "Down opens File");
}

#[trial]
fn a_mac_target_has_no_mnemonics_and_leaves_alt_and_f10_alone() {
    let mut fixture = rig(sample(), Setup { target: Target::Mac, ..LINUX });
    key(&mut fixture, Key::F10, Modifiers::NONE);
    assert!(!is_focused(fixture.ctx()), "F10 leaves the bar alone");
    key(&mut fixture, Key::F, Modifiers::ALT);
    assert!(!is_open(fixture.ctx()), "Option+F is a typing key");
}

/// The layouts add the open documents (check items, the one in front checked) and the app's disabled placeholder
/// for a dynamic list with nothing in it.
#[trial]
fn documents_and_placeholders_from_the_layout_are_rows_of_the_bar() {
    let bar = MenuBar::new(vec![
        Menu::new("File", vec![Node::submenu("Open Recent", vec![item("file.no_recent", "No Recent Files").enabled(false).into()])])
            .role(MenuRole::File),
        Menu::new("Window", Vec::new()).role(MenuRole::Window),
    ])
    .documents(vec![Document::new("doc.0", "One"), Document::new("doc.1", "Two").active(true), Document::new("doc.2", "Hidden").hidden(true)]);
    let laid_out = layout(&bar, Target::Linux, &LayoutOptions::new("Sample")).bar;
    let mut fixture = rig(laid_out, LINUX);
    click(&mut fixture, "Window");
    assert_eq!(toggled(&fixture, "One"), Some(Mixed::Same(false)));
    assert_eq!(toggled(&fixture, "Two"), Some(Mixed::Same(true)));
    assert!(fixture.by_label("Hidden").is_none(), "a hidden document isn't listed");
    click(&mut fixture, "One");
    assert_eq!(fired(&fixture), [("doc.0".to_owned(), FireSource::Menu)], "choosing a document fires its command");
    click(&mut fixture, "File");
    click(&mut fixture, "Open Recent");
    assert!(fixture.by_label("No Recent Files").is_none(), "a list holding only the disabled placeholder does not open");
}

/// The Help menu opens with the search field, which finds an item by its label and fires its command. Undo found
/// there acts on the text field that held focus before the search opened (text back, focus in the field, menu
/// closed) and fires nothing; a non-edit item found there fires its command from the menu.
#[trial]
fn the_help_search_finds_items_and_fires_their_commands() {
    let bar = sample();
    let text = Rc::new(RefCell::new(String::new()));
    let held = Rc::clone(&text);
    let field = Rc::new(Cell::new(None));
    let shown = Rc::clone(&field);
    let mut fixture = Fixture::new(move |ui| {
        MenuBarUi::new(&bar).target(Target::Linux).help_search(true).show(ui);
        shown.set(Some(TextField::new("Name", Bind::bound(&mut *held.borrow_mut())).show(ui).response.id));
    });
    fixture.settle();
    // A search result is a command row: it shows the command's own label.
    fixture.command_state("edit.undo", CommandState::new().title("Undo"));
    click(&mut fixture, "Name");
    fixture.type_text("abc");
    fixture.advance(2.0);
    fixture.settle();
    assert_eq!(*text.borrow(), "abc", "the typing reaches the field");
    let _ = fired(&fixture);
    click(&mut fixture, "Help");
    fixture.type_text("undo");
    fixture.settle();
    click(&mut fixture, "Undo");
    assert_eq!(*text.borrow(), "", "Undo took back the typing in the field");
    assert_eq!(fixture.focused_stop().map(|(id, _)| id), field.get(), "focus is back in the field");
    assert!(fixture.by_label("Redo").is_none() && fixture.by_label("About").is_none(), "the pick closes the Help menu");
    assert_eq!(fired(&fixture), [], "Undo reached the field, so nothing fired for edit.undo");
    click(&mut fixture, "Help");
    fixture.command_state("file.open", CommandState::new().title("Open…"));
    fixture.type_text("open");
    fixture.settle();
    click(&mut fixture, "Open…");
    assert_eq!(fired(&fixture), [("file.open".to_owned(), FireSource::Menu)]);
}
