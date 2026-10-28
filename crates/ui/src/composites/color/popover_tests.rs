//! The colour popover and the swatch palette in a headless egui context: opening, focus, Esc,
//! a click outside, AccessKit and the palette's keys.

use craft_theme::Srgba;
use egui::accesskit::{Node, NodeId, Role, Toggled};
use egui::{Context, Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, pos2, vec2};

use super::hex::hex_id;
use super::{ColorPopover, PickedColor, SwatchPalette};
use crate::direction::{self, Direction};
use crate::edit::{self, EditEvent};
use crate::localized::LocaleNumbers;
use crate::theme::{Base, install};

const SWATCHES: [(&str, [u8; 3]); 6] = [
    ("Red", [0xe5, 0x48, 0x4d]),
    ("Orange", [0xf7, 0x6b, 0x15]),
    ("Green", [0x30, 0xa4, 0x6c]),
    ("Teal", [0x12, 0xa5, 0x94]),
    ("Blue", [0x3e, 0x63, 0xdd]),
    ("Violet", [0x8e, 0x4e, 0xc6]),
];

fn srgba(rgb: [u8; 3]) -> Srgba {
    craft_theme::Rgba::rgb(rgb[0], rgb[1], rgb[2]).into()
}

/// A host with a colour popover over a text field, and a palette under them; it applies every
/// value.
struct Rig {
    ctx: Context,
    time: f64,
    color: Srgba,
    popover_events: Vec<EditEvent<PickedColor>>,
    palette_events: Vec<EditEvent<Srgba>>,
    scoped: bool,
    open: bool,
    picker: egui::Id,
    nodes: Vec<(NodeId, Node)>,
}

impl Rig {
    fn new(color: Srgba) -> Self {
        Self::with(color, Direction::Ltr)
    }

    fn with(color: Srgba, dir: Direction) -> Self {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        direction::apply(&ctx, dir);
        ctx.enable_accesskit();
        let mut rig = Self {
            ctx,
            time: 0.0,
            color,
            popover_events: Vec::new(),
            palette_events: Vec::new(),
            scoped: false,
            open: false,
            picker: egui::Id::NULL,
            nodes: Vec::new(),
        };
        rig.settle();
        rig
    }

    fn frame(&mut self, events: Vec<Event>) {
        let input =
            RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(500.0, 900.0))), time: Some(self.time), events, ..Default::default() };
        self.time += 1.0 / 60.0;
        let numbers = LocaleNumbers::from_tag("en");
        let ctx = self.ctx.clone();
        let (mut popover, mut palette) = (None, None);
        let output = ctx.run_ui(input, |ui| {
            let widget = ColorPopover::new("fill", "Fill colour", self.color, &numbers);
            let picked = ui
                .allocate_ui_with_layout(vec2(ui.available_width(), 24.0), direction::row(ui), |ui| {
                    if self.scoped { edit::scope(ui, |ui| widget.show(ui)).inner } else { widget.show(ui) }
                })
                .inner;
            self.open = picked.open;
            self.picker = picked.picker_id.unwrap_or(egui::Id::NULL);
            popover = picked.edit.event;
            ui.text_edit_singleline(&mut String::from("elsewhere"));
            let swatches = SWATCHES.map(|(name, rgb)| (name, srgba(rgb)));
            palette = SwatchPalette::new("palette", "Swatches", self.color, swatches).show(ui).event;
        });
        for event in [popover.as_ref().and_then(|e| e.value().copied())].into_iter().flatten() {
            self.color = event.color;
        }
        if let Some(e) = &palette
            && let Some(c) = e.value()
        {
            self.color = *c;
        }
        self.popover_events.extend(popover);
        self.palette_events.extend(palette);
        self.nodes = output.platform_output.accesskit_update.clone().map(|u| u.nodes).unwrap_or_default();
        output.drop_without_applying_deltas();
    }

    fn settle(&mut self) {
        for _ in 0..4 {
            self.frame(vec![]);
        }
    }

    fn key(&mut self, key: Key, modifiers: Modifiers) {
        self.frame(vec![Event::ModifiersChanged(modifiers), Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }]);
        self.frame(vec![Event::ModifiersChanged(Modifiers::NONE)]);
        self.settle();
    }

    fn click(&mut self, at: Pos2) {
        self.frame(vec![Event::PointerMoved(at), button(at, true)]);
        self.frame(vec![button(at, false)]);
        self.settle();
    }

    fn drag(&mut self, from: Pos2, to: Pos2) {
        self.frame(vec![Event::PointerMoved(from), button(from, true)]);
        self.frame(vec![Event::PointerMoved(from + vec2(4.0, 0.0))]);
        self.frame(vec![Event::PointerMoved(to)]);
        self.frame(vec![button(to, false)]);
        self.settle();
    }

    fn bounds(&self, role: Role, name: &str) -> Rect {
        let b = self.node(role, name).bounds().expect("bounds");
        Rect::from_min_max(pos2(b.x0 as f32, b.y0 as f32), pos2(b.x1 as f32, b.y1 as f32))
    }

    fn click_node(&mut self, role: Role, name: &str) {
        let at = self.center(role, name);
        self.click(at);
    }

    /// Tabs until the control named `name` has the focus.
    fn tab_to(&mut self, name: &str) {
        for _ in 0..12 {
            if self.focused_name().as_deref() == Some(name) {
                return;
            }
            self.key(Key::Tab, Modifiers::NONE);
        }
        panic!("Tab never reached {name}");
    }

    fn center(&self, role: Role, name: &str) -> Pos2 {
        let b = self.node(role, name).bounds().expect("bounds");
        pos2(((b.x0 + b.x1) / 2.0) as f32, ((b.y0 + b.y1) / 2.0) as f32)
    }

    fn focused_name(&self) -> Option<String> {
        let id = self.ctx.memory(|m| m.focused())?.accesskit_id();
        self.nodes.iter().find(|(n, _)| *n == id).map(|(_, node)| node.label().unwrap_or_default().to_owned())
    }

    fn nodes(&self, role: Role, name: &str) -> Vec<&Node> {
        self.nodes.iter().map(|(_, n)| n).filter(|n| n.role() == role && n.label() == Some(name)).collect()
    }

    fn node(&self, role: Role, name: &str) -> &Node {
        self.nodes(role, name).into_iter().next().unwrap_or_else(|| panic!("no {role:?} node named {name:?}"))
    }
}

fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

#[test]
fn the_popover_opens_with_the_hex_field_focused_closes_on_esc_and_keeps_the_colour_on_a_click_outside() {
    let mut rig = Rig::new(Srgba::rgb(0.2, 0.4, 0.6));
    assert!(!rig.open && rig.nodes(Role::Group, "Fill colour").is_empty(), "closed: no picker");
    rig.click_node(Role::Button, "Fill colour");
    assert!(rig.open);
    rig.nodes(Role::Group, "Fill colour").first().expect("the picker is in the popup");
    assert_eq!(rig.ctx.memory(|m| m.focused()), Some(hex_id(rig.picker)), "the hex field has the focus");
    assert!(rig.node(Role::Button, "Fill colour").is_expanded() == Some(true));

    rig.key(Key::Escape, Modifiers::NONE);
    assert!(!rig.open, "Esc closes the popup");
    let swatch = rig.node(Role::Button, "Fill colour").clone();
    assert_eq!(
        rig.ctx.memory(|m| m.focused()).map(|id| id.accesskit_id()),
        rig.nodes.iter().find(|(_, n)| *n == swatch).map(|(id, _)| *id),
        "the focus is back on the swatch"
    );

    // Typing a colour applies it; a click outside closes the popup and the colour stays.
    rig.key(Key::Enter, Modifiers::NONE);
    assert!(rig.open, "Enter on the swatch opens it");
    rig.frame(vec![Event::Text("FF0000".into())]);
    rig.settle();
    assert_eq!(rig.color.to_rgba().to_hex(), "#ff0000");
    let outside = pos2(480.0, 880.0);
    rig.click(outside);
    assert!(!rig.open, "a click outside closes the popup");
    assert_eq!(rig.color.to_rgba().to_hex(), "#ff0000", "the colour stays");
}

#[test]
fn the_palette_is_a_radio_group_that_selects_by_click_and_moves_focus_by_arrows() {
    let mut rig = Rig::new(srgba(SWATCHES[2].1));
    let group = rig.node(Role::RadioGroup, "Swatches");
    assert_eq!(group.size_of_set(), Some(SWATCHES.len()));
    let radios: Vec<_> = rig.nodes.iter().map(|(_, n)| n).filter(|n| n.role() == Role::RadioButton).collect();
    assert_eq!(radios.len(), SWATCHES.len());
    for (name, rgb) in SWATCHES {
        let node = rig.node(Role::RadioButton, name);
        assert_eq!(node.value(), Some(format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]).as_str()), "{name}: the hex colour is the value");
        assert_eq!(node.toggled() == Some(Toggled::True), name == "Green", "{name}: only the colour in use is checked");
    }

    rig.click_node(Role::RadioButton, "Blue");
    assert_eq!(rig.palette_events.len(), 1);
    assert!(matches!(rig.palette_events[0], EditEvent::Set { .. }), "a click is one Set");
    assert_eq!(rig.node(Role::RadioButton, "Blue").toggled(), Some(Toggled::True));

    // Every swatch is a Tab stop; arrows move the focus without selecting.
    rig.tab_to("Blue");
    rig.key(Key::ArrowRight, Modifiers::NONE);
    assert_eq!(rig.focused_name().as_deref(), Some("Violet"));
    rig.key(Key::ArrowLeft, Modifiers::NONE);
    rig.key(Key::ArrowLeft, Modifiers::NONE);
    assert_eq!(rig.focused_name().as_deref(), Some("Teal"));
    assert_eq!(rig.palette_events.len(), 1, "no selection by moving");
    rig.key(Key::Space, Modifiers::NONE);
    assert_eq!(rig.palette_events.len(), 2, "Space selects the focused swatch");
    assert_eq!(rig.color.to_rgba().to_hex(), "#12a594");
}

#[test]
fn a_popover_in_an_edit_scope_sends_only_updates() {
    let mut rig = Rig::new(Srgba::rgb(0.2, 0.4, 0.6));
    rig.scoped = true;
    rig.settle();
    rig.click_node(Role::Button, "Fill colour");
    assert!(rig.open);
    let area = rig.bounds(Role::Slider, "Saturation and brightness");
    rig.drag(area.center(), area.center() + vec2(30.0, 20.0));
    assert!(!rig.popover_events.is_empty());
    assert!(rig.popover_events.iter().all(|e| matches!(e, EditEvent::Update { .. })), "{:?}", rig.popover_events);
    assert_ne!(rig.color.to_rgba().to_hex(), "#336699", "the drag changed the colour");
}

#[test]
fn the_popover_and_the_palette_mirror_right_to_left() {
    let ltr = Rig::new(Srgba::rgb(0.2, 0.4, 0.6));
    let mut rig = Rig::with(Srgba::rgb(0.2, 0.4, 0.6), Direction::Rtl);
    assert!(rig.bounds(Role::RadioButton, "Red").center().x > rig.bounds(Role::RadioButton, "Orange").center().x, "the palette starts at the right");
    assert!(ltr.bounds(Role::RadioButton, "Red").center().x < ltr.bounds(Role::RadioButton, "Orange").center().x);

    // The popup's start edge is under the swatch's start edge: the right edges line up.
    rig.click_node(Role::Button, "Fill colour");
    let (swatch, area) = (rig.bounds(Role::Button, "Fill colour"), rig.bounds(Role::Slider, "Saturation and brightness"));
    assert!(swatch.min.x > 300.0, "the swatch is at the right: {swatch:?}");
    assert!(area.max.x <= swatch.max.x + 16.0 && area.min.x < swatch.min.x - 100.0, "{swatch:?} {area:?}");
    rig.key(Key::Escape, Modifiers::NONE);
    assert!(!rig.open);

    // Left moves on to the next swatch, which sits to the left.
    rig.tab_to("Red");
    rig.key(Key::ArrowLeft, Modifiers::NONE);
    assert_eq!(rig.focused_name().as_deref(), Some("Orange"));
    rig.key(Key::ArrowRight, Modifiers::NONE);
    assert_eq!(rig.focused_name().as_deref(), Some("Red"));
    assert!(rig.palette_events.is_empty(), "the arrows select nothing");
}
