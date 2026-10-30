//! The colour disc in a headless egui context: grabbing a marker, the drag, keys, the edit
//! lifecycle and AccessKit.

use craft_theme::{ColorModel, HsbModel, ModelValues, Srgba};
use egui::accesskit::{Node, NodeId, Role};
use egui::{Context, Event, Id, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, vec2};

use super::{ColorDisc, ColorDiscStyle, DiscMove};
use crate::edit::{EditEvent, GestureId};
use crate::theme::{Base, Tokens, TokensOf, install};

fn color(h: f32, s: f32, b: f32) -> Srgba {
    HsbModel.to_srgb(&ModelValues::new("hsb", &[h, s, b]).values)
}

fn hsb(c: Srgba) -> [f32; 3] {
    let v = HsbModel.from_srgb(c).values;
    [v[0], v[1], v[2]]
}

/// A host with three markers that moves the one an event names, keeps its brightness and selects
/// the marker a press grabbed.
struct Rig {
    ctx: Context,
    time: f64,
    colors: [Srgba; 3],
    selected: Option<usize>,
    pressed: Option<usize>,
    event: Option<EditEvent<DiscMove>>,
    center: Pos2,
    radius: f32,
    style: Option<ColorDiscStyle>,
    ids: Vec<Id>,
    nodes: Vec<(NodeId, Node)>,
}

impl Rig {
    fn new() -> Self {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        ctx.enable_accesskit();
        let mut rig = Self {
            ctx,
            time: 0.0,
            colors: [color(0.0, 80.0, 90.0), color(120.0, 60.0, 70.0), color(250.0, 40.0, 50.0)],
            selected: Some(0),
            pressed: None,
            event: None,
            center: Pos2::ZERO,
            radius: 0.0,
            style: None,
            ids: Vec::new(),
            nodes: Vec::new(),
        };
        rig.frame(vec![]);
        rig
    }

    fn frame(&mut self, events: Vec<Event>) {
        let input =
            RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))), time: Some(self.time), events, ..Default::default() };
        self.time += 1.0 / 60.0;
        let ctx = self.ctx.clone();
        let output = ctx.run_ui(input, |ui| {
            let shown = ColorDisc::new("disc", "Harmony", &self.colors).selected(self.selected).side(150.0).show(ui);
            let style = ColorDisc::style(&Tokens::of(ui));
            (self.center, self.radius) = ColorDisc::geometry(&style, shown.area);
            self.style = Some(style);
            self.event = shown.edit.event;
            self.pressed = shown.pressed;
            self.ids = shown.markers;
        });
        if let Some(v) = self.event.as_ref().and_then(|e| e.value()) {
            let b = hsb(self.colors[v.marker])[2];
            self.colors[v.marker] = color(v.hue, v.saturation * 100.0, b);
        }
        if let Some(p) = self.pressed {
            self.selected = Some(p);
        }
        self.nodes = output.platform_output.accesskit_update.clone().map(|u| u.nodes).unwrap_or_default();
        output.drop_without_applying_deltas();
    }

    fn at(&self, hue: f32, saturation: f32) -> Pos2 {
        ColorDisc::marker_pos(self.center, self.radius, hue, saturation)
    }

    fn press(&mut self, pos: Pos2) {
        self.frame(vec![Event::PointerMoved(pos), button(pos, true)]);
    }

    fn drag(&mut self, pos: Pos2) {
        self.frame(vec![Event::PointerMoved(pos)]);
    }

    fn release(&mut self, pos: Pos2) {
        self.frame(vec![button(pos, false)]);
    }

    fn click(&mut self, pos: Pos2) {
        self.frame(vec![Event::PointerMoved(pos), button(pos, true), button(pos, false)]);
    }

    fn key(&mut self, key: Key, modifiers: Modifiers) {
        self.frame(vec![Event::ModifiersChanged(modifiers), Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }]);
    }

    fn marker_id(&self, i: usize) -> Id {
        self.ids[i]
    }

    fn node_named(&self, name: &str) -> &Node {
        self.nodes.iter().map(|(_, n)| n).find(|n| n.label() == Some(name)).unwrap_or_else(|| panic!("no AccessKit node named {name}"))
    }
}

fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

fn begin(event: Option<EditEvent<DiscMove>>) -> GestureId {
    match event {
        Some(EditEvent::Begin { id }) => id,
        other => panic!("expected Begin, got {other:?}"),
    }
}

fn moved(event: &Option<EditEvent<DiscMove>>) -> DiscMove {
    event.as_ref().and_then(|e| e.value()).copied().unwrap_or_else(|| panic!("expected an event with a value, got {event:?}"))
}

#[test]
fn a_press_grabs_the_nearest_marker_within_reach_and_elsewhere_grabs_nothing() {
    let mut rig = Rig::new();
    let grab = rig.style.map_or(0.0, |s| s.grab);
    // Between marker 1 (120 degrees, 60 %) and marker 2: the nearer one wins.
    let near = rig.at(120.0, 0.6) + vec2(grab - 2.0, 0.0);
    rig.click(near);
    assert_eq!(rig.pressed, Some(1));
    rig.click(rig.at(250.0, 0.4) + vec2(0.0, grab + 3.0));
    assert_eq!(rig.pressed, None, "a press beyond the reach grabs nothing");
    assert_eq!(rig.event, None, "and starts no gesture");
    rig.press(rig.at(30.0, 0.9));
    assert_eq!((rig.pressed, rig.event.clone()), (None, None));
    rig.drag(rig.at(40.0, 0.9));
    assert_eq!(rig.event, None, "a drag that began elsewhere moves nothing");
    rig.release(rig.at(40.0, 0.9));
}

#[test]
fn a_drag_moves_hue_and_saturation_of_the_grabbed_marker_in_one_gesture_and_keeps_brightness() {
    let mut rig = Rig::new();
    let before = hsb(rig.colors[1]);
    rig.press(rig.at(120.0, 0.6));
    let id = begin(rig.event.clone());
    assert_eq!(rig.pressed, Some(1));
    rig.drag(rig.at(200.0, 0.9));
    let m = moved(&rig.event);
    assert_eq!((m.marker, m.hue, m.saturation), (1, 200.0, 0.9));
    assert!(matches!(rig.event, Some(EditEvent::Update { id: i, .. }) if i == id));
    // Past the rim the saturation stops at 1.
    rig.drag(rig.center + vec2(rig.radius * 3.0, 0.0));
    assert_eq!(moved(&rig.event).saturation, 1.0);
    rig.release(rig.center + vec2(rig.radius * 3.0, 0.0));
    assert!(matches!(rig.event, Some(EditEvent::Commit { changed: true, .. })), "{:?}", rig.event);
    let after = hsb(rig.colors[1]);
    assert!((after[2] - before[2]).abs() < 0.01, "brightness stays: {before:?} -> {after:?}");
    assert_eq!(rig.selected, Some(1), "the host selected the grabbed marker");
}

#[test]
fn esc_cancels_a_drag() {
    let mut rig = Rig::new();
    rig.press(rig.at(0.0, 0.8));
    rig.drag(rig.at(90.0, 0.5));
    rig.key(Key::Escape, Modifiers::NONE);
    assert!(matches!(rig.event, Some(EditEvent::Cancel { .. })), "{:?}", rig.event);
}

#[test]
fn keys_move_the_selected_marker_as_sets_and_only_it_is_a_tab_stop() {
    let mut rig = Rig::new();
    rig.click(rig.at(120.0, 0.6));
    assert_eq!(rig.selected, Some(1));
    assert_eq!(rig.ctx.memory(|m| m.focused()), Some(rig.marker_id(1)), "a press focuses the marker");
    rig.key(Key::ArrowRight, Modifiers::NONE);
    let m = moved(&rig.event);
    assert!(matches!(rig.event, Some(EditEvent::Set { .. })));
    assert_eq!((m.marker, m.hue), (1, 121.0));
    rig.key(Key::ArrowDown, Modifiers::SHIFT);
    assert_eq!(moved(&rig.event).saturation, 0.5);
    rig.key(Key::ArrowLeft, Modifiers::SHIFT);
    assert_eq!(moved(&rig.event).hue, 111.0);
    rig.frame(vec![Event::Key { key: Key::Tab, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE }]);
    assert_ne!(rig.ctx.memory(|m| m.focused()), Some(rig.marker_id(2)), "Tab leaves the disc: the other markers are no stops");
    assert_ne!(rig.ctx.memory(|m| m.focused()), Some(rig.marker_id(0)));
}

#[test]
fn accesskit_names_the_group_and_a_slider_per_marker() {
    let rig = Rig::new();
    assert!(rig.nodes.iter().any(|(_, n)| n.role() == Role::Group && n.label() == Some("Harmony")));
    for (name, value) in [
        ("Color 1", "Hue 0 degrees, Saturation 80 %"),
        ("Color 2", "Hue 120 degrees, Saturation 60 %"),
        ("Color 3", "Hue 250 degrees, Saturation 40 %"),
    ] {
        let node = rig.node_named(name);
        assert_eq!(node.role(), Role::Slider);
        assert_eq!(node.value(), Some(value));
    }
}
