//! Menu tests: the aim triangle, the keys across rows and levels, mnemonics, clicks that keep the
//! menu open, Esc, and AccessKit. A raw egui context driven frame by frame, so time and the
//! pointer path are exact.

use craft_theme::ControlSize;
use egui::accesskit::{self, HasPopup, NodeId, Role, Toggled};
use egui::{Event, Id, Key, Modifiers, Popup, Pos2, Rect, pos2, vec2};

use craft_theme::Srgba;

use super::nav::{Nav, heading};
use super::row::RowPainter;
use super::*;
use crate::theme::{Base, TokensOf as _, install};
use crate::theme::{ToEgui as _, Tokens};

fn item(label: &str, command: &'static str) -> MenuEntry<&'static str> {
    MenuItem::new(label, command).into()
}

/// A button "Open" at the top left with `entries` in its menu, driven frame by frame.
struct Driver {
    ctx: egui::Context,
    entries: Vec<MenuEntry<&'static str>>,
    dt: f64,
    time: f64,
    events: Vec<Event>,
    pointer: Pos2,
    tree: Vec<(NodeId, accesskit::Node)>,
    chosen: Vec<&'static str>,
    steps: Vec<MenuStep>,
    popup: Id,
    open: bool,
    /// Passed to [`Menu::opened_by_keyboard`].
    keyboard: bool,
    /// What the last frame painted.
    shapes: Vec<egui::epaint::ClippedShape>,
}

impl Driver {
    fn new(entries: Vec<MenuEntry<&'static str>>) -> Driver {
        let ctx = egui::Context::default();
        install(&ctx, &Base::suite_default());
        ctx.set_os(egui::os::OperatingSystem::Nix);
        ctx.enable_accesskit();
        ctx.all_styles_mut(|s| s.scroll_animation = egui::style::ScrollAnimation::none());
        let mut d = Driver {
            ctx,
            entries,
            dt: 1.0 / 60.0,
            time: 0.0,
            events: Vec::new(),
            pointer: pos2(-10.0, -10.0),
            tree: Vec::new(),
            chosen: Vec::new(),
            steps: Vec::new(),
            popup: Id::NULL,
            open: false,
            keyboard: false,
            shapes: Vec::new(),
        };
        d.run(3);
        d
    }

    fn frame(&mut self) {
        self.time += self.dt;
        let raw = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))),
            time: Some(self.time),
            predicted_dt: self.dt as f32,
            events: std::mem::take(&mut self.events),
            ..Default::default()
        };
        let (entries, keyboard) = (&self.entries, self.keyboard);
        let mut shown = None;
        let mut popup = Id::NULL;
        let mut full = self.ctx.run_ui(raw, |ui| {
            let button = ui.add(crate::controls::Button::new("Open"));
            // Only a pointer click toggles it: Enter and Space on the focused button are the menu's.
            let toggle = button.clicked_by(egui::PointerButton::Primary).then_some(egui::SetOpenCommand::Toggle);
            let p = Popup::menu(&button).open_memory(toggle);
            popup = p.get_id();
            shown = Some(Menu::new(entries).opened_by_keyboard(keyboard).show(ui, p));
        });
        full.textures_delta.clear();
        if let Some(update) = full.platform_output.accesskit_update {
            self.tree = update.nodes;
        }
        self.popup = popup;
        self.shapes = std::mem::take(&mut full.shapes);
        if let Some(shown) = shown {
            self.chosen.extend(shown.chosen);
            self.steps.extend(shown.step);
            self.open = shown.open;
        }
    }

    fn run(&mut self, frames: usize) {
        for _ in 0..frames {
            self.frame();
        }
    }

    fn node(&self, label: &str) -> Option<&accesskit::Node> {
        self.tree.iter().map(|(_, n)| n).find(|n| n.label() == Some(label))
    }

    fn get(&self, label: &str) -> accesskit::Node {
        self.node(label).unwrap_or_else(|| panic!("no {label:?}")).clone()
    }

    fn shown(&self, label: &str) -> bool {
        self.node(label).is_some()
    }

    fn rect(&self, label: &str) -> Rect {
        let n = self.node(label).unwrap_or_else(|| panic!("no {label:?}"));
        let b = n.bounds().unwrap_or_else(|| panic!("{label:?} has no bounds"));
        Rect::from_min_max(pos2(b.x0 as f32, b.y0 as f32), pos2(b.x1 as f32, b.y1 as f32))
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
        let r = self.rect(label);
        self.move_to(pos2(r.left() + (r.width() / 2.0).min(20.0), r.center().y));
        self.button(true);
        self.button(false);
        self.run(3);
    }

    fn key(&mut self, key: Key) {
        for pressed in [true, false] {
            self.events.push(Event::Key { key, physical_key: None, pressed, repeat: false, modifiers: Modifiers::NONE });
        }
        self.run(3);
    }

    fn nav(&self) -> Nav {
        Nav::load(&self.ctx, self.popup, false)
    }

    /// The label of the highlighted row in the level the keys move in.
    fn highlighted(&self) -> Option<String> {
        let nav = self.nav();
        let row = nav.highlighted(nav.level).and_then(|i| nav.rows.get(nav.level)?.get(i))?;
        self.tree.iter().find(|(id, _)| *id == row.widget.accesskit_id()).and_then(|(_, n)| n.label().map(str::to_owned))
    }
}

/// The `active_descendant` of the menu node that has one.
pub(super) fn active_in_menu(tree: &[(NodeId, accesskit::Node)]) -> Option<NodeId> {
    tree.iter().filter(|(_, n)| n.role() == Role::Menu).find_map(|(_, n)| n.active_descendant())
}

fn two_submenus() -> Driver {
    let sub = |name: &'static str, n: usize| {
        let rows = (0..n).map(|i| MenuItem::new(format!("{name} {i}"), name).into()).collect();
        MenuEntry::from(MenuItem::submenu(name, rows))
    };
    let mut d = Driver::new(vec![sub("Mode", 8), sub("Adjust", 8)]);
    d.dt = 1.0 / 120.0;
    d
}

/// Opens the menu and rests on Mode until its submenu is open; returns where the pointer is, the
/// Adjust row below it, and a row low in Mode's submenu.
fn rest_on_mode(d: &mut Driver) -> (Pos2, Rect, Rect) {
    d.click("Open");
    let mode = d.rect("Mode");
    let from = pos2(mode.right() - 40.0, mode.center().y);
    for k in 0..=12 {
        d.move_to(from - vec2(30.0 * (1.0 - k as f32 / 12.0), 0.0));
    }
    d.run(30);
    assert_eq!(open_submenu(d), "Mode", "resting on Mode opens it");
    (from, d.rect("Adjust"), d.rect("Mode 6"))
}

fn open_submenu(d: &Driver) -> &'static str {
    match (d.shown("Mode 0"), d.shown("Adjust 0")) {
        (true, false) => "Mode",
        (false, true) => "Adjust",
        (false, false) => "none",
        (true, true) => "both",
    }
}

/// Moves the pointer along `path` (0…1) in `frames` frames; `pause` holds it still for that many
/// frames at that fraction of the way.
fn glide(d: &mut Driver, frames: usize, pause: Option<(f32, usize)>, mut path: impl FnMut(f32) -> Pos2) {
    let mut paused = false;
    for k in 0..=frames {
        let t = k as f32 / frames as f32;
        d.move_to(path(t));
        if let Some((_, n)) = pause.filter(|(at, _)| !paused && t >= *at) {
            paused = true;
            d.run(n);
        }
    }
}

/// Heading from Mode for a row low in its submenu, the pointer crosses the Adjust row. On a
/// diagonal or with a short pause on the way, Mode's submenu stays open, Adjust's doesn't open,
/// and the crossed row isn't highlighted. Resting on Adjust for the aim's rest time, or moving
/// away from the submenu onto it, opens Adjust's.
#[test]
fn a_submenu_stays_open_while_the_pointer_heads_for_it_and_lets_go_at_rest() {
    for (name, frames, pause) in [("straight", 36, None), ("hesitate", 36, Some((0.3, 18)))] {
        let mut d = two_submenus();
        let (from, adjust, target) = rest_on_mode(&mut d);
        let to = pos2(target.left() + 40.0, target.center().y);
        let mut crossed = false;
        glide(&mut d, frames, pause, |t| {
            let p = from + (to - from) * t;
            crossed |= adjust.contains(p);
            p
        });
        assert!(crossed, "{name}: the path crosses the Adjust row");
        assert_eq!(open_submenu(&d), "Mode", "{name}");
        assert_eq!(d.nav().highlighted(0), Some(0), "{name}: Mode stays the highlighted row");
    }

    let mut d = two_submenus();
    let (from, adjust, _) = rest_on_mode(&mut d);
    let stop = pos2(from.x + 15.0, adjust.center().y);
    glide(&mut d, 12, None, |t| from + (stop - from) * t);
    assert_eq!(open_submenu(&d), "Mode", "just arrived on Adjust, still heading for Mode's submenu");
    // Under the aim's rest time (0.3 s at 120 frames a second) it holds; past it, Adjust takes over.
    d.run(30);
    assert_eq!(open_submenu(&d), "Mode", "0.25 s at rest");
    d.run(18);
    assert_eq!(open_submenu(&d), "Adjust", "resting on Adjust opens it");
    assert_eq!(d.nav().highlighted(0), Some(1));

    let mut d = two_submenus();
    let (from, adjust, _) = rest_on_mode(&mut d);
    let away = pos2(from.x - 60.0, adjust.center().y);
    glide(&mut d, 10, None, |t| from + (away - from) * t);
    d.run(30);
    assert_eq!(open_submenu(&d), "Adjust", "moving away from Mode's submenu opens Adjust's");
}

#[test]
fn heading_is_the_triangle_to_the_submenus_near_edge() {
    let sub = Rect::from_min_max(pos2(200.0, 100.0), pos2(400.0, 400.0));
    let from = pos2(150.0, 110.0);
    assert!(heading(from, from, sub));
    assert!(heading(from, pos2(170.0, 160.0), sub), "down and right, toward the submenu");
    assert!(heading(from, pos2(190.0, 105.0), sub), "straight across");
    assert!(!heading(from, pos2(150.0, 160.0), sub), "straight down");
    assert!(!heading(from, pos2(120.0, 140.0), sub), "away to the left");
    assert!(!heading(from, pos2(170.0, 60.0), sub), "up past the submenu's top");
    let left = Rect::from_min_max(pos2(0.0, 100.0), pos2(100.0, 400.0));
    assert!(heading(pos2(150.0, 110.0), pos2(130.0, 160.0), left), "a submenu that opened to the left");
    assert!(!heading(pos2(150.0, 110.0), pos2(170.0, 160.0), left));
    assert!(!heading(from, from, Rect::NOTHING));
}

#[test]
fn up_and_down_skip_disabled_rows_and_wrap() {
    let mut d = Driver::new(vec![
        item("Cut", "cut"),
        MenuItem::new("Copy", "copy").enabled(false).into(),
        MenuEntry::Separator,
        item("Paste", "paste"),
        MenuItem::new("Delete", "delete").enabled(false).into(),
    ]);
    d.click("Open");
    assert_eq!(d.highlighted(), None, "opened with the pointer: nothing highlighted");
    let mut seen = Vec::new();
    for key in [Key::ArrowDown, Key::ArrowDown, Key::ArrowDown, Key::ArrowUp, Key::ArrowUp] {
        d.key(key);
        seen.push(d.highlighted().unwrap_or_default());
    }
    assert_eq!(seen, ["Cut", "Paste", "Cut", "Paste", "Cut"]);
    let mut d2 = Driver::new(vec![item("Cut", "cut"), item("Paste", "paste")]);
    d2.click("Open");
    d2.key(Key::ArrowUp);
    assert_eq!(d2.highlighted().as_deref(), Some("Paste"), "↑ from nothing is the last row");
    d.key(Key::ArrowUp);
    d.key(Key::Enter);
    assert_eq!(d.chosen, ["paste"]);
    assert!(!d.open, "running a row closes the menu");
}

#[test]
fn a_menu_opened_from_the_keyboard_highlights_its_first_enabled_row() {
    let mut d = Driver::new(vec![MenuItem::new("Undo", "undo").enabled(false).into(), item("Redo", "redo")]);
    d.keyboard = true;
    Popup::open_id(&d.ctx, d.popup);
    d.run(3);
    assert_eq!(d.highlighted().as_deref(), Some("Redo"));
    d.key(Key::ArrowDown);
    assert_eq!(d.highlighted().as_deref(), Some("Redo"), "the only enabled row; the first pass's highlight isn't repeated");
}

#[test]
fn right_opens_a_submenu_left_closes_it_and_the_top_level_reports_steps() {
    let mut d = Driver::new(vec![
        item("New", "new"),
        MenuItem::submenu("Recent", vec![item("a.txt", "a"), MenuItem::new("gone.txt", "gone").enabled(false).into(), item("b.txt", "b")]).into(),
    ]);
    d.click("Open");
    d.key(Key::ArrowDown);
    d.key(Key::ArrowRight);
    assert_eq!(d.steps, [MenuStep::Next], "→ on a row without a submenu asks for the next bar menu");
    assert!(d.open, "the menu stays open for its owner");
    d.key(Key::ArrowDown);
    d.key(Key::ArrowRight);
    assert!(d.shown("a.txt"), "→ opens the submenu");
    assert_eq!(d.highlighted().as_deref(), Some("a.txt"), "on its first enabled row");
    assert_eq!(d.node("Recent").and_then(|n| n.is_expanded()), Some(true));
    d.key(Key::ArrowDown);
    assert_eq!(d.highlighted().as_deref(), Some("b.txt"));
    d.key(Key::ArrowLeft);
    assert!(!d.shown("a.txt"), "← closes the submenu");
    assert_eq!(d.highlighted().as_deref(), Some("Recent"), "back on its row");
    assert_eq!(d.node("Recent").and_then(|n| n.is_expanded()), Some(false));
    d.key(Key::ArrowLeft);
    assert_eq!(d.steps, [MenuStep::Next, MenuStep::Previous], "← at the top level asks for the previous bar menu");
    d.key(Key::Enter);
    assert!(d.shown("a.txt"), "Enter opens the submenu too");
    d.key(Key::ArrowUp);
    d.key(Key::Space);
    assert_eq!(d.chosen, ["b"], "↑ wraps to the last enabled row, Space runs it");
}

#[test]
fn a_mnemonic_runs_its_row_or_opens_its_submenu_and_is_off_on_a_mac() {
    let entries = || {
        vec![
            item("Copy", "copy"),
            item("Paste", "paste"),
            MenuItem::submenu("Arrange", vec![item("Bring Forward", "forward"), item("Send Backward", "backward")]).into(),
            MenuItem::new("Print", "print").mnemonic('R').into(),
        ]
    };
    let mut d = Driver::new(entries());
    d.click("Open");
    assert_eq!(d.node("Print").and_then(|n| n.access_key()), Some("R"), "the preferred letter");
    assert_eq!(d.node("Paste").and_then(|n| n.access_key()), Some("P"));
    d.key(Key::A);
    assert!(d.shown("Bring Forward"), "A opens Arrange");
    d.key(Key::S);
    assert_eq!(d.chosen, ["backward"], "S runs Send Backward in the submenu the keys moved into");
    assert!(!d.open);

    let mut d = Driver::new(entries());
    d.ctx.set_os(egui::os::OperatingSystem::Mac);
    d.click("Open");
    d.key(Key::P);
    assert!(d.chosen.is_empty() && d.open, "no mnemonics on a Mac");
    assert_eq!(d.node("Paste").and_then(|n| n.access_key()), None);
    set_mnemonics(&d.ctx, Some(true));
    d.key(Key::P);
    assert_eq!(d.chosen, ["paste"], "turned on");
}

#[test]
fn a_click_on_a_disabled_row_or_a_separator_keeps_the_menu_open() {
    let mut d = Driver::new(vec![
        item("Cut", "cut"),
        MenuEntry::Separator,
        MenuItem::new("Copy", "copy").enabled(false).into(),
        MenuItem::submenu("Empty", vec![MenuItem::new("Nothing", "nothing").enabled(false).into()]).into(),
        item("Paste", "paste"),
    ]);
    d.click("Open");
    d.click("Copy");
    assert!(d.open && d.chosen.is_empty(), "disabled row");
    let (cut, copy) = (d.rect("Cut"), d.rect("Copy"));
    let gap = pos2(cut.left() + 20.0, (cut.bottom() + copy.top()) / 2.0);
    d.move_to(gap);
    d.button(true);
    d.button(false);
    d.run(3);
    assert!(d.open && d.chosen.is_empty(), "separator");
    d.click("Empty");
    assert!(d.open && !d.shown("Nothing"), "a submenu with nothing to run is disabled");
    d.click("Paste");
    assert_eq!(d.chosen, ["paste"]);
    assert!(!d.open);
    d.click("Open");
    d.move_to(pos2(700.0, 500.0));
    d.button(true);
    d.button(false);
    d.run(3);
    assert!(!d.open, "a click outside closes it");
}

#[test]
fn a_press_outside_closes_the_menu_before_the_release_and_a_press_inside_a_submenu_does_not() {
    let mut d = two_submenus();
    d.click("Open");
    assert!(d.open);
    d.move_to(pos2(700.0, 500.0));
    d.button(true);
    assert!(!d.open, "closed on the press");
    d.button(false);
    assert!(!d.open && d.chosen.is_empty());

    let (_, _, deep) = rest_on_mode(&mut d);
    d.move_to(deep.center());
    d.button(true);
    assert!(d.open && d.shown("Mode 0"), "a press inside a submenu keeps every level open");
    d.button(false);
}

#[test]
fn escape_closes_every_level() {
    let mut d = Driver::new(vec![MenuItem::submenu("Zoom", vec![MenuItem::submenu("Preset", vec![item("100%", "actual")]).into()]).into()]);
    d.click("Open");
    d.key(Key::ArrowDown);
    d.key(Key::ArrowRight);
    d.key(Key::ArrowRight);
    assert!(d.shown("100%"), "two levels open");
    d.key(Key::Escape);
    assert!(!d.open && !d.shown("Preset") && !d.shown("100%"));
    assert!(!crate::popup_list::any_open(&d.ctx), "no list recorded open");
}

#[test]
fn accesskit_sees_menu_roles_checked_states_submenus_and_shortcuts() {
    use egui::KeyboardShortcut;
    let save = crate::controls::Kbd::new(&KeyboardShortcut::new(Modifiers::COMMAND | Modifiers::SHIFT, Key::S));
    let mut d = Driver::new(vec![
        MenuItem::new("Save As…", "save").shortcut(save).into(),
        MenuItem::new("Grid", "grid").check(true).into(),
        MenuItem::new("Rulers", "rulers").check(false).into(),
        MenuItem::new("Light", "light").radio(true).into(),
        MenuItem::new("Dark", "dark").radio(false).enabled(false).into(),
        MenuItem::submenu("Zoom", vec![item("100%", "actual")]).into(),
    ]);
    d.click("Open");
    assert!(crate::popup_list::any_open(&d.ctx), "the open menu is recorded");
    assert!(d.tree.iter().any(|(_, n)| n.role() == Role::Menu), "the popup is a menu");
    let save = d.get("Save As…");
    assert_eq!((save.role(), save.keyboard_shortcut()), (Role::MenuItem, Some("Ctrl+Shift+S")));
    assert_eq!((d.get("Grid").role(), d.get("Grid").toggled()), (Role::MenuItemCheckBox, Some(Toggled::True)));
    assert_eq!(d.get("Rulers").toggled(), Some(Toggled::False));
    assert_eq!((d.get("Light").role(), d.get("Light").toggled()), (Role::MenuItemRadio, Some(Toggled::True)));
    assert!(d.get("Dark").is_disabled());
    let zoom = d.get("Zoom");
    assert_eq!((zoom.role(), zoom.has_popup(), zoom.is_expanded()), (Role::MenuItem, Some(HasPopup::Menu), Some(false)));
    d.click("Zoom");
    assert_eq!(d.get("Zoom").is_expanded(), Some(true));
    assert_eq!(d.tree.iter().filter(|(_, n)| n.role() == Role::Menu).count(), 2, "the submenu is a menu too");

    d.ctx.set_os(egui::os::OperatingSystem::Mac);
    d.run(2);
    assert_eq!(d.get("Save As…").keyboard_shortcut(), Some("Shift+Cmd+S"), "the names in Apple's order");
}

#[test]
fn rows_are_as_wide_as_the_widest_and_at_least_the_menus_width() {
    let mut d = Driver::new(vec![item("A", "a"), item("A much longer row than the least width of a menu holds", "b")]);
    d.click("Open");
    let (a, b) = (d.rect("A"), d.rect("A much longer row than the least width of a menu holds"));
    assert_eq!(a.width(), b.width(), "rows share one width");
    assert!(a.width() > 220.0);
    let mut d = Driver::new(vec![item("A", "a")]);
    d.click("Open");
    let mut style = None;
    d.ctx
        .clone()
        .run_ui(egui::RawInput::default(), |ui| style = Some(Menu::style(&crate::theme::Tokens::of(ui), ControlSize::Md)))
        .drop_without_applying_deltas();
    let style = style.unwrap_or_else(|| panic!("no style"));
    assert_eq!(d.rect("A").size(), vec2(style.min_width, style.rows.height), "menu.min_width, and the list box row height");
}

#[test]
fn the_story_opens_a_menu_whose_rows_are_all_named() {
    let ctx = egui::Context::default();
    install(&ctx, &Base::suite_default());
    ctx.enable_accesskit();
    let story = stories().into_iter().next().unwrap_or_else(|| panic!("no story"));
    let mut args = story.defaults();
    let mut frame = |events: Vec<Event>| {
        let raw = egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))), events, ..Default::default() };
        let mut out = ctx.run_ui(raw, |ui| (story.show)(ui, &mut args));
        out.textures_delta.clear();
        out.platform_output.accesskit_update.map(|u| u.nodes).unwrap_or_default()
    };
    frame(Vec::new());
    let tree = frame(Vec::new());
    let edit = tree.iter().find(|(_, n)| n.label() == Some("Edit")).and_then(|(_, n)| n.bounds()).unwrap_or_else(|| panic!("no Edit button"));
    let at = pos2(((edit.x0 + edit.x1) / 2.0) as f32, ((edit.y0 + edit.y1) / 2.0) as f32);
    let button = |pressed| Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
    frame(vec![Event::PointerMoved(at), button(true), button(false)]);
    frame(Vec::new());
    let tree = frame(Vec::new());
    let rows: Vec<_> =
        tree.iter().map(|(_, n)| n).filter(|n| matches!(n.role(), Role::MenuItem | Role::MenuItemCheckBox | Role::MenuItemRadio)).collect();
    assert_eq!(rows.len(), 9, "the Edit menu's rows");
    assert!(rows.iter().all(|n| n.label().is_some_and(|l| !l.is_empty())), "every row is named");
}

/// Two press menus "One" and "Two" and a context-menu target "Target", each with its own rows.
struct Owners {
    ctx: egui::Context,
    time: f64,
    events: Vec<Event>,
    pointer: Pos2,
    tree: Vec<(NodeId, accesskit::Node)>,
    chosen: Vec<&'static str>,
    open: [bool; 3],
    entries: [Vec<MenuEntry<&'static str>>; 3],
    /// Give this owner (0 One, 1 Two, 2 Target) the focus in the next pass.
    focus: Option<usize>,
    /// The owners' widget ids.
    ids: [Id; 3],
    /// Key and text events left in the input after the menus, every pass.
    after: Vec<Event>,
    /// The pass each owner's menu was last shown in, and the last pass.
    shown_in: [u64; 3],
    pass: u64,
    /// [`any_open`] as the last pass began, before the menus were drawn.
    open_before: bool,
    /// Space before the owners.
    indent: f32,
}

impl Owners {
    fn new() -> Owners {
        let ctx = egui::Context::default();
        install(&ctx, &Base::suite_default());
        ctx.set_os(egui::os::OperatingSystem::Nix);
        ctx.enable_accesskit();
        let mut o = Owners {
            ctx,
            time: 0.0,
            events: Vec::new(),
            pointer: pos2(-10.0, -10.0),
            tree: Vec::new(),
            chosen: Vec::new(),
            open: [false; 3],
            entries: [
                vec![item("Alpha", "alpha"), MenuItem::new("Off", "off").enabled(false).into(), item("Beta", "beta")],
                vec![item("Gamma", "gamma"), item("Delta", "delta")],
                vec![item("Cut", "cut"), item("Copy", "copy")],
            ],
            focus: None,
            ids: [Id::NULL; 3],
            after: Vec::new(),
            shown_in: [0; 3],
            pass: 0,
            open_before: false,
            indent: 0.0,
        };
        o.run(3);
        o
    }

    fn frame(&mut self) {
        self.time += 1.0 / 60.0;
        let raw = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))),
            time: Some(self.time),
            predicted_dt: 1.0 / 60.0,
            events: std::mem::take(&mut self.events),
            ..Default::default()
        };
        let mut chosen = Vec::new();
        let mut open = [false; 3];
        let mut ids = [Id::NULL; 3];
        let mut after = Vec::new();
        let mut shown_in = self.shown_in;
        let mut pass = 0;
        let mut open_before = false;
        let focus = self.focus.take();
        let entries = &self.entries;
        let indent = self.indent;
        let mut full = self.ctx.run_ui(raw, |ui| {
            open_before = any_open(ui.ctx());
            ui.horizontal(|ui| {
                ui.add_space(indent);
                let one = ui.add(crate::controls::Button::new("One"));
                let two = ui.add(crate::controls::Button::new("Two"));
                let target = ui.add(crate::controls::Button::new("Target"));
                ids = [one.id, two.id, target.id];
                if let Some(k) = focus {
                    [&one, &two, &target][k].request_focus();
                }
                let shown = [
                    PressMenu::new(&entries[0]).show(ui, &one),
                    PressMenu::new(&entries[1]).show(ui, &two),
                    ContextMenu::new(&entries[2]).show(ui, &target),
                ];
                for (k, s) in shown.into_iter().enumerate() {
                    chosen.extend(s.chosen);
                    open[k] = s.open;
                    if s.popup.is_some() {
                        shown_in[k] = ui.ctx().cumulative_pass_nr();
                    }
                }
                pass = ui.ctx().cumulative_pass_nr();
                after = ui.input(|i| i.events.iter().filter(|e| matches!(e, Event::Key { .. } | Event::Text(_))).cloned().collect());
            });
        });
        full.textures_delta.clear();
        if let Some(update) = full.platform_output.accesskit_update {
            self.tree = update.nodes;
        }
        self.chosen.extend(chosen);
        self.open = open;
        self.ids = ids;
        self.after.extend(after);
        self.shown_in = shown_in;
        self.pass = pass;
        self.open_before = open_before;
    }

    fn run(&mut self, frames: usize) {
        for _ in 0..frames {
            self.frame();
        }
    }

    fn rect(&self, label: &str) -> Rect {
        let n = self.tree.iter().map(|(_, n)| n).find(|n| n.label() == Some(label)).unwrap_or_else(|| panic!("no {label:?}"));
        let b = n.bounds().expect("bounds");
        Rect::from_min_max(pos2(b.x0 as f32, b.y0 as f32), pos2(b.x1 as f32, b.y1 as f32))
    }

    fn move_to(&mut self, p: Pos2) {
        self.pointer = p;
        self.events.push(Event::PointerMoved(p));
        self.frame();
    }

    fn at(&self, label: &str) -> Pos2 {
        let r = self.rect(label);
        pos2(r.left() + (r.width() / 2.0).min(20.0), r.center().y)
    }

    fn button(&mut self, button: egui::PointerButton, pressed: bool) {
        self.events.push(Event::PointerButton { pos: self.pointer, button, pressed, modifiers: Modifiers::NONE });
        self.frame();
    }

    fn press(&mut self, label: &str) {
        let p = self.at(label);
        self.move_to(p);
        self.button(egui::PointerButton::Primary, true);
    }

    fn release(&mut self) {
        self.button(egui::PointerButton::Primary, false);
        self.run(3);
    }

    fn key(&mut self, key: Key, modifiers: Modifiers) {
        for pressed in [true, false] {
            self.events.push(Event::Key { key, physical_key: None, pressed, repeat: false, modifiers });
        }
        self.run(3);
    }

    fn shown(&self, label: &str) -> bool {
        self.tree.iter().any(|(_, n)| n.label() == Some(label))
    }

    /// A letter typed: its key and the text it types.
    fn letter(&mut self, c: char, modifiers: Modifiers) {
        let key = Key::from_name(&c.to_ascii_uppercase().to_string()).unwrap_or_else(|| panic!("no key {c}"));
        self.events.push(Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers });
        self.events.push(Event::Text(c.to_string()));
        self.events.push(Event::Key { key, physical_key: None, pressed: false, repeat: false, modifiers });
        self.run(3);
    }

    fn node_id(&self, label: &str) -> NodeId {
        self.tree.iter().find(|(_, n)| n.label() == Some(label)).map(|(id, _)| *id).unwrap_or_else(|| panic!("no {label:?}"))
    }

    /// The `active_descendant` of the open menu's node, and of the owner `k`'s widget.
    fn active(&self, k: usize) -> (Option<NodeId>, Option<NodeId>) {
        let menu = active_in_menu(&self.tree);
        let owner = self.ids.get(k).and_then(|id| self.tree.iter().find(|(n, _)| *n == id.accesskit_id())).map(|(_, n)| n);
        (menu, owner.and_then(accesskit::Node::active_descendant))
    }
}

#[test]
fn press_drag_release_runs_the_row_under_the_release_and_keeps_the_menu_open_on_the_button() {
    let mut o = Owners::new();
    o.press("One");
    o.run(2);
    assert!(o.open[0] && o.shown("Beta"));
    let p = o.at("Beta");
    o.move_to(p);
    o.release();
    assert_eq!(o.chosen, ["beta"]);
    assert!(!o.open[0], "the row ran and the menu closed");

    // Released back on the button: the menu stays open, and so after a disabled row.
    o.press("One");
    o.release();
    assert!(o.open[0] && o.chosen.len() == 1, "a click opens it");
    o.press("One");
    o.release();
    assert!(!o.open[0], "a press on the open button closes it");
    o.press("One");
    o.run(2);
    let (alpha, beta) = (o.rect("Alpha"), o.rect("Beta"));
    o.move_to(pos2(alpha.left() + 20.0, (alpha.center().y + beta.center().y) / 2.0));
    o.release();
    assert!(o.open[0] && o.chosen.len() == 1, "released on a disabled row");
    o.press("One");
    o.release();
    o.press("One");
    o.move_to(pos2(700.0, 500.0));
    o.release();
    assert!(o.open[0], "released on nothing keeps the menu open");
    assert_eq!(o.chosen, ["beta"]);
}

#[test]
fn secondary_click_opens_both_kinds_of_menu() {
    let mut o = Owners::new();
    for (label, row, k) in [("One", "Alpha", 0), ("Target", "Cut", 2)] {
        let p = o.at(label);
        o.move_to(p);
        o.button(egui::PointerButton::Secondary, true);
        o.button(egui::PointerButton::Secondary, false);
        o.run(3);
        assert!(o.open[k] && o.shown(row), "{label}");
        if k == 2 {
            // At the pointer.
            assert!(o.rect("Cut").contains(o.rect("Cut").left_top() + vec2(1.0, 1.0)));
            assert!((o.rect("Cut").left() - p.x).abs() < 40.0, "{:?} {:?}", o.rect("Cut"), p);
        }
        o.key(Key::Escape, Modifiers::NONE);
        assert!(!o.open[k]);
    }
}

#[test]
fn the_menu_key_and_shift_f10_open_a_context_menu_at_the_focused_widget() {
    for (key, modifiers) in [(Key::Menu, Modifiers::NONE), (Key::F10, Modifiers::SHIFT)] {
        let mut o = Owners::new();
        o.focus = Some(2);
        o.run(2);
        o.key(key, modifiers);
        assert!(o.open[2], "{key:?}");
        let (target, cut) = (o.rect("Target"), o.rect("Cut"));
        assert!((cut.left() - target.left()).abs() < 10.0 && cut.top() >= target.bottom() - 1.0, "{cut:?} under {target:?}");
        o.key(Key::ArrowDown, Modifiers::NONE);
        o.key(Key::Enter, Modifiers::NONE);
        assert_eq!(o.chosen, ["copy"], "first row lit on opening, then one step down");
        assert!(!o.open[2]);
    }
}

/// Right to left, a context menu runs leftwards from its point, though there is room on the right.
#[test]
fn right_to_left_a_context_menu_ends_at_its_point() {
    let mut o = Owners::new();
    crate::direction::apply(&o.ctx, crate::direction::Direction::Rtl);
    o.indent = 400.0;
    o.focus = Some(2);
    o.run(2);
    let target = o.rect("Target");
    assert!(target.right() < 700.0, "room on the right: {target:?}");
    o.key(Key::F10, Modifiers::SHIFT);
    let cut = o.rect("Cut");
    assert!((cut.right() - target.right()).abs() < 10.0 && cut.top() >= target.bottom() - 1.0, "{cut:?} under {target:?}");
    o.key(Key::Escape, Modifiers::NONE);
    let p = target.center();
    o.move_to(p);
    o.button(egui::PointerButton::Secondary, true);
    o.button(egui::PointerButton::Secondary, false);
    o.run(3);
    assert!(o.open[2]);
    let cut = o.rect("Cut");
    assert!((cut.right() - p.x).abs() < 10.0, "{cut:?} ends at {p:?}");
}

#[test]
fn two_press_menus_in_one_frame_keep_their_own_gestures() {
    let mut o = Owners::new();
    o.press("One");
    // Over the other button's row positions: Two's rows aren't shown, and One's gesture is One's.
    o.release();
    assert!(o.open[0] && !o.open[1]);
    o.press("Two");
    o.run(2);
    assert!(o.open[1] && o.shown("Delta"));
    let p = o.at("Delta");
    o.move_to(p);
    o.release();
    assert_eq!(o.chosen, ["delta"], "Two's gesture runs Two's row");
    assert!(!o.open[1]);
}

#[test]
fn a_secondary_click_while_a_context_menu_is_open_reopens_it_at_the_new_pointer() {
    let mut o = Owners::new();
    let target = o.rect("Target");
    let secondary = |o: &mut Owners, p: Pos2| {
        o.move_to(p);
        o.button(egui::PointerButton::Secondary, true);
        o.button(egui::PointerButton::Secondary, false);
        o.run(3);
    };
    // B is left of the menu opened at A, so the click lands on the widget.
    let (a, b) = (target.left_center() + vec2(40.0, 0.0), target.left_center() + vec2(4.0, 0.0));
    secondary(&mut o, a);
    assert!(o.open[2]);
    let first = o.rect("Cut").left();
    o.key(Key::ArrowDown, Modifiers::NONE);
    assert!(o.active(2).0.is_some(), "a row is lit");
    o.move_to(b);
    for pressed in [true, false] {
        o.button(egui::PointerButton::Secondary, pressed);
        assert_eq!(o.shown_in[2], o.pass, "the menu shows on every pass while it moves (pressed: {pressed})");
    }
    o.run(3);
    assert!(o.open[2], "still open");
    let second = o.rect("Cut").left();
    assert!((first - second - 36.0).abs() < 1.0, "moved from {first} to {second}");
    assert_eq!(o.tree.iter().filter(|(_, n)| n.label() == Some("Cut")).count(), 1, "one menu");
    assert_eq!(o.active(2).0, None, "it starts over with nothing lit");
}

fn key_events(key: Key, modifiers: Modifiers) -> Vec<Event> {
    [true, false].map(|pressed| Event::Key { key, physical_key: None, pressed, repeat: false, modifiers }).into()
}

/// Opening View's submenu and lighting a row there, then opening Mode's with the pointer: Mode's
/// submenu shows nothing lit (the old highlight belonged to View's).
#[test]
fn a_row_lit_in_one_submenu_is_not_lit_in_the_next_one_the_pointer_opens() {
    let mut d = two_submenus();
    let (_, adjust, _) = rest_on_mode(&mut d);
    let row = d.rect("Mode 2");
    d.move_to(pos2(row.left() + 20.0, row.center().y));
    d.run(3);
    assert_eq!(d.nav().highlighted(1), Some(2), "Mode 2 is lit");
    d.move_to(pos2(adjust.left() + 20.0, adjust.center().y));
    d.run(60);
    assert_eq!(open_submenu(&d), "Adjust");
    assert_eq!(d.nav().highlighted(1), None, "nothing in Adjust's submenu is lit");
}

/// A press menu inside a popover (an egui popup of its own), with a reader of Esc after both: the
/// Esc that closes the menu closes nothing else, and the menu's button gets the focus back.
#[test]
fn escape_closes_a_menu_and_leaves_the_popup_it_is_in_open() {
    let ctx = egui::Context::default();
    install(&ctx, &Base::suite_default());
    ctx.set_os(egui::os::OperatingSystem::Nix);
    let entries = vec![item("Cut", "cut"), item("Copy", "copy")];
    let mut outer = true;
    let mut time = 0.0;
    // (menu open, popover open, Esc read after them, button focused)
    let mut frame = |events: Vec<Event>, focus: bool| {
        time += 1.0 / 60.0;
        let raw =
            egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))), time: Some(time), events, ..Default::default() };
        let mut out = (false, false, false, false);
        let mut full = ctx.run_ui(raw, |ui| {
            let host = ui.add(crate::controls::Button::new("Host"));
            egui::Popup::from_response(&host).open_bool(&mut outer).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
                let switch = ui.add(crate::controls::Button::new("Switch"));
                if focus {
                    switch.request_focus();
                }
                out.0 = PressMenu::new(&entries).show(ui, &switch).open;
                out.3 = switch.has_focus();
            });
            out.2 = ui.input(|i| i.key_pressed(Key::Escape));
        });
        full.textures_delta.clear();
        out.1 = outer;
        out
    };
    for _ in 0..3 {
        frame(Vec::new(), true);
    }
    frame(key_events(Key::Enter, Modifiers::NONE), false);
    assert!(frame(Vec::new(), false).0, "Enter on the switch opens its menu");
    let (menu, popover, read, _) = frame(key_events(Key::Escape, Modifiers::NONE), false);
    assert!(!menu && popover && !read, "Esc closes the menu alone and is not read after it");
    let (_, popover, _, focused) = frame(Vec::new(), false);
    assert!(popover && focused, "the switch has the focus back");
    let (_, popover, read, _) = frame(key_events(Key::Escape, Modifiers::NONE), false);
    assert!(!popover && read, "the next Esc is the popover's");
}

#[test]
fn the_lit_row_is_the_active_descendant_of_its_menu_and_of_the_focused_owner() {
    let mut o = Owners::new();
    o.focus = Some(2);
    o.run(2);
    o.key(Key::F10, Modifiers::SHIFT);
    let (cut, copy) = (o.node_id("Cut"), o.node_id("Copy"));
    assert_eq!(o.active(2), (Some(cut), Some(cut)), "context menu");
    o.key(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(o.active(2), (Some(copy), Some(copy)));
    o.key(Key::Escape, Modifiers::NONE);

    o.focus = Some(0);
    o.run(2);
    o.key(Key::Enter, Modifiers::NONE);
    let (alpha, beta) = (o.node_id("Alpha"), o.node_id("Beta"));
    assert_eq!(o.active(0), (Some(alpha), Some(alpha)), "press menu");
    o.key(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(o.active(0), (Some(beta), Some(beta)));
}

#[test]
fn enter_or_space_on_a_press_menu_button_opens_it_on_its_first_row_without_running_it() {
    for key in [Key::Enter, Key::Space] {
        let mut o = Owners::new();
        o.focus = Some(0);
        o.run(2);
        o.key(key, Modifiers::NONE);
        assert!(o.open[0] && o.chosen.is_empty(), "{key:?} opens the menu and runs nothing");
        assert_eq!(o.active(0).0, Some(o.node_id("Alpha")), "{key:?}: the first enabled row is lit");
        o.key(key, Modifiers::NONE);
        assert_eq!(o.chosen, ["alpha"], "{key:?} then runs it");
    }
}

#[test]
fn an_accesskit_click_opens_a_press_menu_from_the_keyboard_and_closes_it() {
    use egui::accesskit::{Action, ActionRequest, TreeId};
    let mut o = Owners::new();
    let click = |o: &mut Owners| {
        let target_node = o.ids[0].accesskit_id();
        o.events.push(Event::AccessKitActionRequest(ActionRequest { action: Action::Click, target_tree: TreeId::ROOT, target_node, data: None }));
        o.run(3);
    };
    let expanded = |o: &Owners| o.tree.iter().find(|(id, _)| *id == o.ids[0].accesskit_id()).and_then(|(_, n)| n.is_expanded());
    click(&mut o);
    assert!(o.open[0], "an AccessKit click opens it");
    assert_eq!(expanded(&o), Some(true));
    assert_eq!(o.active(0).0, Some(o.node_id("Alpha")), "as from the keyboard: the first enabled row is lit");
    click(&mut o);
    assert!(!o.open[0], "a second one closes it");
    assert_eq!(expanded(&o), Some(false));
}

/// Typing in an open menu reaches nothing under it: a letter without a row, a letter on a Mac
/// (no mnemonics), with or without a row. Command shortcuts go on.
#[test]
fn typing_goes_nowhere_while_a_menu_is_open() {
    let mut o = Owners::new();
    o.focus = Some(2);
    o.run(2);
    o.key(Key::Menu, Modifiers::NONE);
    assert!(o.open[2]);
    o.after.clear();
    o.letter('b', Modifiers::NONE);
    assert!(o.after.is_empty() && o.open[2] && o.chosen.is_empty(), "B, which no row has, is taken: {:?}", o.after);
    o.letter('b', Modifiers::COMMAND);
    assert!(o.after.iter().any(|e| matches!(e, Event::Key { key: Key::B, modifiers, .. } if modifiers.command)), "Ctrl+B goes on");
    o.key(Key::Escape, Modifiers::NONE);
    o.after.clear();
    o.letter('b', Modifiers::NONE);
    assert!(o.after.contains(&Event::Text("b".into())), "with the menu closed, B types");

    o.ctx.set_os(egui::os::OperatingSystem::Mac);
    o.key(Key::Menu, Modifiers::NONE);
    assert!(o.open[2]);
    o.after.clear();
    o.letter('c', Modifiers::NONE);
    assert!(o.after.is_empty() && o.open[2] && o.chosen.is_empty(), "on a Mac a row's letter runs nothing and goes nowhere");
}

/// A context menu and a press menu taller than the window take translated scroll-strip names.
#[test]
fn context_and_press_menus_name_their_scroll_strips() {
    let ctx = egui::Context::default();
    install(&ctx, &Base::suite_default());
    ctx.enable_accesskit();
    let rows: Vec<MenuEntry<&'static str>> = (0..60).map(|_| item("Row", "row")).collect();
    let frame = |events: Vec<Event>| {
        let raw = egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(600.0, 300.0))), events, ..Default::default() };
        let mut full = ctx.run_ui(raw, |ui| {
            ui.horizontal(|ui| {
                let press = ui.add(crate::controls::Button::new("Press"));
                let target = ui.add(crate::controls::Button::new("Target"));
                let _ = PressMenu::new(&rows).scroll_labels("Press up", "Press down").show(ui, &press);
                let _ = ContextMenu::new(&rows).scroll_labels("Context up", "Context down").show(ui, &target);
            });
        });
        full.textures_delta.clear();
        full.platform_output.accesskit_update.map(|u| u.nodes).unwrap_or_default()
    };
    let tree = frame(Vec::new());
    let at = |label: &str| {
        let b = tree.iter().find(|(_, n)| n.label() == Some(label)).and_then(|(_, n)| n.bounds()).unwrap_or_else(|| panic!("no {label}"));
        pos2(((b.x0 + b.x1) / 2.0) as f32, ((b.y0 + b.y1) / 2.0) as f32)
    };
    for (owner, up, down) in [("Press", "Press up", "Press down"), ("Target", "Context up", "Context down")] {
        let p = at(owner);
        let secondary = |pressed| Event::PointerButton { pos: p, button: egui::PointerButton::Secondary, pressed, modifiers: Modifiers::NONE };
        frame(vec![Event::PointerMoved(p), secondary(true), secondary(false)]);
        let mut tree = Vec::new();
        for _ in 0..4 {
            tree = frame(Vec::new());
        }
        let named = |name: &str| tree.iter().any(|(_, n)| n.label() == Some(name));
        assert!(named(up) && named(down), "{owner}");
        frame(key_events(Key::Escape, Modifiers::NONE));
        // The closed menu's layer still covers the window for one pass's hit tests.
        frame(Vec::new());
    }
}

/// An app reads `any_open` before drawing its menus, to skip its single-key shortcuts: it is true
/// while a press menu or a context menu is open, and false once Esc has closed it.
#[test]
fn any_open_is_true_while_a_press_or_context_menu_is_open() {
    let mut o = Owners::new();
    assert!(!o.open_before && !any_open(&o.ctx));
    o.press("One");
    o.release();
    assert!(o.open[0] && o.open_before, "press menu");
    o.key(Key::Escape, Modifiers::NONE);
    assert!(!o.open_before, "closed");

    o.focus = Some(2);
    o.run(2);
    o.key(Key::F10, Modifiers::SHIFT);
    assert!(o.open[2] && o.open_before, "context menu");
    o.key(Key::Escape, Modifiers::NONE);
    assert!(!o.open_before, "closed");
}

/// The fills of the rectangles the last frame painted.
fn fills(d: &Driver) -> Vec<egui::Color32> {
    fn collect(shape: &egui::Shape, out: &mut Vec<egui::Color32>) {
        match shape {
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| collect(s, out)),
            egui::Shape::Rect(r) => out.push(r.fill),
            _ => {}
        }
    }
    let mut out = Vec::new();
    d.shapes.iter().for_each(|c| collect(&c.shape, &mut out));
    out
}

fn tokens_in(theme: &craft_theme::ThemeDef, mode: craft_theme::Mode, contrast: craft_theme::Contrast) -> crate::theme::UiTokens {
    use craft_theme::{Appearance, Size, TextSize};
    let appearance = Appearance { mode, contrast, size: Size::Compact, text_size: TextSize::Default, reduced_motion: false };
    crate::theme::UiTokens::new(craft_theme::resolve(theme, appearance, None).tokens, appearance, mode, crate::theme::FontFamilies::default())
}

/// PhotoCraft's label colours (`menu_tint`).
fn tint_hues() -> [(&'static str, Srgba); 7] {
    [
        ("red", [190, 60, 60]),
        ("orange", [200, 120, 40]),
        ("yellow", [190, 170, 40]),
        ("green", [60, 150, 70]),
        ("blue", [50, 110, 200]),
        ("violet", [130, 80, 190]),
        ("gray", [120, 120, 120]),
    ]
    .map(|(name, [r, g, b])| (name, Srgba::from(egui::Color32::from_rgb(r, g, b))))
}

/// A tinted row's text over its fill must reach: the label, shortcut (text) 4.5:1, 7:1 in high
/// contrast; the check and radio marks, the submenu arrow and the icon (graphics) 3:1, 4.5:1.
/// Returns each pair with its ratio over its minimum.
fn tint_margins(painter: &RowPainter<'_>, tint: Srgba, high: bool) -> Vec<(&'static str, f64)> {
    let (text_min, mark_min) = if high { (7.0, 4.5) } else { (4.5, 3.0) };
    let colors = painter.tint_colors(tint);
    let fill = colors.fill;
    let on_fill = |fg: egui::Color32| craft_theme::contrast_ratio(craft_theme::Srgba::from(fg).into(), fill);
    let (label, shortcut) = (colors.text.to_egui(), colors.shortcut.to_egui());
    // The painter draws marks, arrows and icons in the label colour.
    vec![
        ("label", on_fill(label) / text_min),
        ("shortcut", on_fill(shortcut) / text_min),
        ("check and radio mark", on_fill(label) / mark_min),
        ("submenu arrow", on_fill(label) / mark_min),
        ("icon", on_fill(label) / mark_min),
    ]
}

#[test]
fn a_tinted_row_keeps_its_text_and_marks_readable_in_every_builtin_theme() {
    use craft_theme::{Contrast, Mode};
    for theme in craft_theme::builtins() {
        for mode in [Mode::Light, Mode::Dark] {
            for contrast in [Contrast::Standard, Contrast::High] {
                let tokens = tokens_in(theme, mode, contrast);
                let style = Menu::style(&tokens, ControlSize::Md);
                let painter = RowPainter { style: &style, tokens: &tokens, size: ControlSize::Md };
                for (name, hue) in tint_hues() {
                    assert_eq!(tokens.component.menu.tint_of(hue.into()), Some(painter.tint_colors(hue)), "{name}: the seven hues use their tokens");
                    for (what, margin) in tint_margins(&painter, hue, contrast == Contrast::High) {
                        assert!(
                            margin >= 1.0,
                            "{} {mode:?} {contrast:?}: the {what} on the {name} tint is {margin:.3} of its minimum",
                            theme.id.as_str()
                        );
                    }
                }
                // A colour that is not one of the seven is derived the same way.
                let other = Srgba::rgb(0.1, 0.8, 0.8);
                for (what, margin) in tint_margins(&painter, other, contrast == Contrast::High) {
                    assert!(
                        margin >= 1.0,
                        "{} {mode:?} {contrast:?}: the {what} on an arbitrary tint is {margin:.3} of its minimum",
                        theme.id.as_str()
                    );
                }
            }
        }
    }
}

#[test]
fn the_active_row_ignores_the_tint() {
    let tint = Srgba::rgb(0.86, 0.24, 0.24);
    let entries =
        vec![MenuItem::new("Red", "red").tint(tint).into(), MenuItem::new("Off", "off").tint(tint).enabled(false).into(), item("Plain", "plain")];
    let mut d = Driver::new(entries);
    d.click("Open");
    let ctx = egui::Context::default();
    install(&ctx, &Base::suite_default());
    let mut style = None;
    ctx.run_ui(egui::RawInput::default(), |ui| style = Some(Menu::style(&Tokens::of(ui), ControlSize::Md))).drop_without_applying_deltas();
    let style = style.unwrap_or_else(|| panic!("no style"));
    let tinted = {
        let mut tokens = None;
        ctx.run_ui(egui::RawInput::default(), |ui| tokens = Some(Tokens::of(ui))).drop_without_applying_deltas();
        let tokens = tokens.unwrap_or_else(|| panic!("no tokens"));
        RowPainter { style: &style, tokens: &tokens, size: ControlSize::Md }.tint_colors(tint).fill.to_egui()
    };
    let count = |d: &Driver, c| fills(d).iter().filter(|&&f| f == c).count();
    d.move_to(pos2(-10.0, -10.0));
    d.run(2);
    assert_eq!(count(&d, tinted), 1, "the enabled tinted row is tinted; the disabled one is not");
    d.move_to(d.rect("Red").center());
    d.run(2);
    assert_eq!(count(&d, tinted), 0, "the active row draws no tint");
    assert_eq!(count(&d, style.active_fill), 1, "it draws the active fill");
}

#[test]
fn the_active_row_is_the_accent_and_a_submenu_row_is_hover_grey_while_the_pointer_is_in_its_submenu() {
    let mut d = two_submenus();
    rest_on_mode(&mut d);
    let ctx = egui::Context::default();
    install(&ctx, &Base::suite_default());
    let (mut style, mut accent) = (None, None);
    ctx.run_ui(egui::RawInput::default(), |ui| {
        let tokens = Tokens::of(ui);
        style = Some(Menu::style(&tokens, ControlSize::Md));
        accent = Some((tokens.color.accent.solid.to_egui(), tokens.color.state.hover.to_egui()));
    })
    .drop_without_applying_deltas();
    let (style, (accent, hover)) = (style.unwrap_or_else(|| panic!("no style")), accent.unwrap_or_else(|| panic!("no tokens")));
    assert_eq!((style.active_fill, style.open_fill), (accent, hover), "accent.solid and state.hover");
    let count = |d: &Driver, c| fills(d).iter().filter(|&&f| f == c).count();
    assert_eq!((count(&d, style.active_fill), count(&d, style.open_fill)), (1, 0), "the pointer on Mode: Mode is the accent");
    let inside = d.rect("Mode 6").center();
    d.move_to(inside);
    d.run(3);
    assert_eq!(
        (count(&d, style.active_fill), count(&d, style.open_fill)),
        (1, 1),
        "the pointer in the submenu: its row is the accent and Mode stays grey"
    );
}

#[test]
fn a_row_tooltip_shows_after_the_delay() {
    let mut d = Driver::new(vec![MenuItem::new("Save", "save").tooltip("Writes the file").into(), item("Quit", "quit")]);
    d.click("Open");
    let tip = |d: &Driver| d.tree.iter().filter(|(_, n)| n.role() == Role::Label && n.value() == Some("Writes the file")).count();
    assert_eq!(tip(&d), 0);
    d.move_to(d.rect("Save").center());
    d.run(5);
    assert_eq!(tip(&d), 0, "not before the delay");
    d.run(40);
    assert_eq!(tip(&d), 1, "the tooltip shows");
    assert_eq!(d.get("Save").description(), Some("Writes the file"));
    assert_eq!(d.get("Quit").description(), None);
}

/// The corners of the filled triangle the last frame painted inside `row`, if there is one.
fn triangle_in(d: &Driver, row: Rect) -> Option<Vec<Pos2>> {
    fn collect(shape: &egui::Shape, row: Rect, out: &mut Option<Vec<Pos2>>) {
        match shape {
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| collect(s, row, out)),
            egui::Shape::Path(p) if p.points.len() == 3 && p.fill != egui::Color32::TRANSPARENT && p.points.iter().all(|q| row.contains(*q)) => {
                *out = Some(p.points.clone());
            }
            _ => {}
        }
    }
    let mut out = None;
    d.shapes.iter().for_each(|c| collect(&c.shape, row, &mut out));
    out
}

/// A submenu row ends in a filled triangle (PhotoCraft's, from egui) that points the way the
/// submenu opens: right, and left in a right-to-left layout.
#[test]
fn a_submenu_row_ends_in_a_filled_triangle_pointing_towards_its_submenu() {
    for rtl in [false, true] {
        let mut d = Driver::new(vec![MenuItem::submenu("Mode", vec![item("Fast", "fast")]).into()]);
        if rtl {
            crate::direction::apply(&d.ctx, crate::direction::Direction::Rtl);
        }
        d.click("Open");
        let row = d.rect("Mode");
        let corners = triangle_in(&d, row).unwrap_or_else(|| panic!("a filled triangle in {row:?}"));
        let tip = corners.iter().max_by(|a, b| (if rtl { b.x } else { a.x }).total_cmp(&if rtl { a.x } else { b.x })).copied().unwrap_or_default();
        let (base_x, side) = (corners.iter().map(|c| c.x).filter(|x| (x - tip.x).abs() > 0.5).sum::<f32>() / 2.0, if rtl { -1.0 } else { 1.0 });
        assert!(side * (tip.x - base_x) > 1.0, "rtl {rtl}: tip {tip:?} is beyond the base at x {base_x}");
        let edge = if rtl { row.left() } else { row.right() };
        assert!((edge - tip.x).abs() < row.height(), "rtl {rtl}: the arrow is at the row's end");
    }
}
