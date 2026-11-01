//! `SplitButton` in a harness.
#![allow(clippy::expect_used, reason = "tests")]

use egui::accesskit::Role;
use egui::{Context, Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, pos2, vec2};

use super::SplitButton;
use crate::menu::{MenuEntry, MenuItem};
use crate::theme::{Base, install};

struct Rig {
    ctx: Context,
    time: f64,
    entries: Vec<MenuEntry<&'static str>>,
    runs: usize,
    chosen: Vec<&'static str>,
    open: bool,
    active: bool,
    /// Each AccessKit node's role, name and bounds in the last frame.
    nodes: Vec<(Role, String, Rect)>,
}

impl Rig {
    fn new() -> Self {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        ctx.enable_accesskit();
        let entries = vec![MenuItem::new("Standard", "standard").into(), MenuItem::new("Detailed", "detailed").into()];
        let mut rig = Self { ctx, time: 0.0, entries, runs: 0, chosen: Vec::new(), open: false, active: false, nodes: Vec::new() };
        rig.frame(vec![]);
        rig.frame(vec![]);
        rig
    }

    fn frame(&mut self, events: Vec<Event>) {
        let input =
            RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))), time: Some(self.time), events, ..Default::default() };
        self.time += 1.0 / 60.0;
        let ctx = self.ctx.clone();
        let entries = self.entries.clone();
        let output = ctx.run_ui(input, |ui| {
            let out = SplitButton::new("trace", "Image Trace", &entries).arrow_label("Tracing Presets").show(ui);
            self.runs += usize::from(out.main.clicked());
            self.chosen.extend(out.menu.chosen);
            self.open = out.menu.open;
            self.active = out.menu.active.is_some();
        });
        if let Some(update) = output.platform_output.accesskit_update.clone() {
            self.nodes = update
                .nodes
                .iter()
                .filter_map(|(_, n)| {
                    let b = n.bounds()?;
                    Some((
                        n.role(),
                        n.label().unwrap_or_default().to_owned(),
                        Rect::from_min_max(pos2(b.x0 as f32, b.y0 as f32), pos2(b.x1 as f32, b.y1 as f32)),
                    ))
                })
                .collect();
        }
        output.drop_without_applying_deltas();
    }

    fn center(&self, role: Role, name: &str) -> Pos2 {
        self.nodes.iter().find(|(r, n, _)| *r == role && n == name).map(|node| node.2.center()).expect("the node is in the tree")
    }

    fn button(&mut self, at: Pos2, pressed: bool) {
        self.frame(vec![Event::PointerMoved(at)]);
        self.frame(vec![Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }]);
    }

    fn key(&mut self, key: Key, modifiers: Modifiers) {
        self.frame(vec![Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }]);
        self.frame(vec![Event::Key { key, physical_key: None, pressed: false, repeat: false, modifiers }]);
        self.frame(vec![]);
    }
}

// A click on the main part reports `main.clicked()` without opening the menu; a press on the arrow opens it and a release on a row chooses it.
#[test]
fn the_main_part_runs_and_the_arrow_opens_the_menu() {
    let mut rig = Rig::new();
    let main = rig.center(Role::Button, "Image Trace");
    rig.button(main, true);
    rig.button(main, false);
    assert_eq!((rig.runs, rig.open), (1, false), "the main part runs the action and opens nothing");

    let arrow = rig.center(Role::Button, "Tracing Presets");
    rig.button(arrow, true);
    rig.frame(vec![]);
    assert!(rig.open, "a press on the arrow opens the menu");
    let row = rig.center(Role::MenuItem, "Detailed");
    rig.button(row, false);
    assert_eq!(rig.chosen, ["detailed"]);
    assert!(!rig.open);
    assert_eq!(rig.runs, 1);

    // The press that closes an open menu does not run the action.
    rig.button(arrow, true);
    rig.button(arrow, false);
    assert!(rig.open);
    rig.button(main, true);
    rig.button(main, false);
    assert_eq!((rig.open, rig.runs), (false, 1), "the closing press does nothing else");

    // A press on the arrow closes the menu it opened, and the main part's action does not run.
    rig.button(arrow, true);
    rig.button(arrow, false);
    assert!(rig.open);
    rig.button(arrow, true);
    rig.button(arrow, false);
    assert_eq!((rig.open, rig.runs), (false, 1), "a press on the arrow closes the menu");

    // A menu opened from the keyboard is closed by a press on the main part, which runs nothing.
    let mut rig = Rig::new();
    rig.key(Key::Tab, Modifiers::NONE);
    rig.key(Key::ArrowDown, Modifiers::ALT);
    assert!(rig.open);
    let main = rig.center(Role::Button, "Image Trace");
    rig.button(main, true);
    rig.button(main, false);
    assert_eq!((rig.open, rig.runs), (false, 0), "the closing press does nothing else");
}

// The border is one rectangle around the whole button: no part has an edge of its own.
#[test]
fn one_border_surrounds_both_parts() {
    let rig = Rig::new();
    let ctx = rig.ctx.clone();
    let entries = rig.entries.clone();
    let mut width = 0.0;
    let input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))), time: Some(rig.time), ..Default::default() };
    let output = ctx.run_ui(input, |ui| {
        width = SplitButton::new("trace", "Image Trace", &entries).arrow_label("Tracing Presets").show(ui).main.rect.width();
    });
    let outlines: Vec<f32> = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Rect(r) if r.stroke.width > 0.0 => Some(r.rect.width()),
            _ => None,
        })
        .collect();
    output.drop_without_applying_deltas();
    assert!(!outlines.is_empty(), "the button has a border");
    assert!(outlines.iter().all(|w| *w > width + 1.0), "every outline spans the arrow part too: {outlines:?} beside the main part's {width}");
}

// With the focus on the button, Alt+↓ opens the menu with its first row active; Enter runs the action when it is closed.
#[test]
fn alt_down_opens_the_menu_from_the_keyboard() {
    let mut rig = Rig::new();
    rig.key(Key::Tab, Modifiers::NONE);
    rig.key(Key::ArrowDown, Modifiers::ALT);
    assert!(rig.open && rig.active, "open with its first row active");
    rig.key(Key::Escape, Modifiers::NONE);
    assert!(!rig.open);
    rig.key(Key::Enter, Modifiers::NONE);
    assert_eq!((rig.runs, rig.open), (1, false), "Enter runs the action, the focus stayed on the button");
}
