//! The Compact colour picker in a headless egui context: the hex field, kept model values, the
//! picker's gestures, an edit scope, AccessKit and the dialog Tab order.

use craft_theme::{Channel, ColorModel, DefaultOn, LabModel, MAX_CHANNELS, ModelValues, Rgba, Srgba};
use egui::accesskit::{HasPopup, Node, NodeId, Role, Toggled};
use egui::{Context, Event, Id, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, pos2, vec2};

use super::hex::hex_id;
use super::{ColorPicker, PickedColor};
use crate::behavior::FieldTab;
use crate::controls::TextField;
use crate::edit::{self, EditEvent};
use crate::localized::LocaleNumbers;
use crate::theme::{Base, install};

/// A host editing one colour with a Compact picker (alpha and eyedropper on) between two text
/// fields, in a dialog's [`FieldTab`] scope; it applies every value and puts the colour from
/// before a gesture back on a cancel.
struct Rig {
    ctx: Context,
    time: f64,
    color: Srgba,
    before: Srgba,
    /// Keeps the colour in 8 bits, as a host with `Rgba` colours does.
    eight_bit: bool,
    scoped: bool,
    /// Snaps every colour to a web colour (each channel a multiple of 0x33).
    web: bool,
    models: Option<Vec<&'static dyn ColorModel>>,
    picker: Id,
    fields: (Id, Id),
    events: Vec<EditEvent<PickedColor>>,
    nodes: Vec<(NodeId, Node)>,
}

impl Rig {
    fn new(color: Srgba) -> Self {
        Self::with(color, None)
    }

    fn with(color: Srgba, models: Option<Vec<&'static dyn ColorModel>>) -> Self {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        ctx.enable_accesskit();
        let mut rig = Self {
            ctx,
            time: 0.0,
            color,
            before: color,
            eight_bit: false,
            scoped: false,
            web: false,
            models,
            picker: Id::NULL,
            fields: (Id::NULL, Id::NULL),
            events: Vec::new(),
            nodes: Vec::new(),
        };
        rig.frame(vec![]);
        rig
    }

    fn frame(&mut self, events: Vec<Event>) {
        let input =
            RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(400.0, 900.0))), time: Some(self.time), events, ..Default::default() };
        self.time += 1.0 / 60.0;
        let numbers = LocaleNumbers::from_tag("en");
        let ctx = self.ctx.clone();
        let mut event = None;
        let output = ctx.run_ui(input, |ui| {
            FieldTab::new("dialog").show(ui, |ui| {
                self.fields.0 = TextField::new("before", "Before", "a").show(ui).response.id;
                let mut picker = ColorPicker::new("picker", "Foreground colour", self.color, &numbers).alpha(true).eyedropper("Pick screen color");
                if let Some(models) = &self.models {
                    picker = picker.models(models.iter().copied());
                }
                if self.web {
                    picker = picker.constrain(|c| Srgba::new((c.r * 5.0).round() / 5.0, (c.g * 5.0).round() / 5.0, (c.b * 5.0).round() / 5.0, c.a));
                }
                let picked = match self.scoped {
                    true => edit::scope(ui, |ui| picker.show(ui)).inner,
                    false => picker.show(ui),
                };
                self.picker = picked.edit.response.id;
                event = picked.edit.event;
                self.fields.1 = TextField::new("after", "After", "b").show(ui).response.id;
            });
        });
        if let Some(event) = event {
            match &event {
                EditEvent::Begin { .. } => self.before = self.color,
                EditEvent::Cancel { .. } => self.color = self.before,
                _ => {}
            }
            if let Some(p) = event.value() {
                self.color = if self.eight_bit { p.color.to_rgba().into() } else { p.color };
            }
            self.events.push(event);
        }
        self.nodes = output.platform_output.accesskit_update.clone().map(|u| u.nodes).unwrap_or_default();
        output.drop_without_applying_deltas();
    }

    fn settle(&mut self) {
        for _ in 0..3 {
            self.frame(vec![]);
        }
    }

    fn key(&mut self, key: Key, modifiers: Modifiers) {
        self.frame(vec![Event::ModifiersChanged(modifiers), Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }]);
        self.frame(vec![Event::ModifiersChanged(Modifiers::NONE)]);
    }

    fn focus(&mut self, id: Id) {
        self.ctx.memory_mut(|m| m.request_focus(id));
        self.frame(vec![]);
    }

    /// Focuses the hex field with its text selected.
    fn focus_hex(&mut self) {
        self.focus(hex_id(self.picker));
        self.key(Key::A, Modifiers::COMMAND);
    }

    /// Tabs from the field before the picker to its `n`th field, which selects its text.
    fn tab_to(&mut self, n: usize) {
        self.focus(self.fields.0);
        for _ in 0..=n {
            self.key(Key::Tab, Modifiers::NONE);
        }
    }

    fn type_text(&mut self, text: &str) {
        self.frame(vec![Event::Text(text.to_owned())]);
        self.settle();
    }

    fn paste(&mut self, text: &str) {
        self.frame(vec![Event::Paste(text.to_owned())]);
        self.settle();
    }

    fn blur(&mut self) {
        self.focus(self.fields.1);
        self.settle();
    }

    fn drag(&mut self, from: Pos2, to: Pos2) {
        self.frame(vec![Event::PointerMoved(from), button(from, true)]);
        self.frame(vec![Event::PointerMoved(from + vec2(4.0, 0.0))]);
        self.frame(vec![Event::PointerMoved(to)]);
        self.frame(vec![button(to, false)]);
        self.settle();
    }

    fn focused_name(&self) -> Option<String> {
        let id = self.ctx.memory(|m| m.focused())?.accesskit_id();
        self.nodes.iter().find(|(n, _)| *n == id).map(|(_, node)| self.name(node))
    }

    /// A node's accessible name: its label, or the text of the nodes labelling it.
    fn name(&self, node: &Node) -> String {
        if let Some(label) = node.label() {
            return label.to_owned();
        }
        let by = node.labelled_by();
        let texts = self.nodes.iter().filter(|(id, _)| by.contains(id)).filter_map(|(_, n)| n.label().or(n.value()));
        texts.collect::<Vec<_>>().join(" ")
    }

    /// The nodes of `role` named `name`.
    fn nodes(&self, role: Role, name: &str) -> Vec<&Node> {
        self.nodes.iter().map(|(_, n)| n).filter(|n| n.role() == role && self.name(n) == name).collect()
    }

    fn node(&self, role: Role, name: &str) -> &Node {
        self.nodes(role, name).into_iter().next().unwrap_or_else(|| panic!("no {role:?} node named {name:?}"))
    }

    fn hex_text(&self) -> String {
        self.node(Role::TextInput, "Hex colour").value().unwrap_or_default().to_owned()
    }

    fn area(&self) -> Rect {
        let b = self.node(Role::Slider, "Saturation and brightness").bounds().expect("area bounds");
        Rect::from_min_max(pos2(b.x0 as f32, b.y0 as f32), pos2(b.x1 as f32, b.y1 as f32))
    }

    fn last_model(&self) -> Option<ModelValues> {
        self.events.iter().rev().find_map(|e| e.value()).and_then(|p| p.model)
    }
}

fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

fn hex(color: Srgba) -> String {
    color.to_rgba().to_hex()
}

fn kind(event: &EditEvent<PickedColor>) -> &'static str {
    match event {
        EditEvent::Begin { .. } => "Begin",
        EditEvent::Update { .. } => "Update",
        EditEvent::Commit { .. } => "Commit",
        EditEvent::Cancel { .. } => "Cancel",
        EditEvent::Set { .. } => "Set",
    }
}

#[test]
fn hex_field_shows_uppercase_keeps_partial_typing_and_reverts_invalid_text_on_blur() {
    let mut rig = Rig::new(Rgba::rgb(0x1a, 0x2b, 0x3c).into());
    assert_eq!(rig.hex_text(), "1A2B3C", "the colour shows in uppercase");

    rig.focus_hex();
    rig.type_text("ff0");
    assert_eq!(rig.hex_text(), "ff0", "partial input stays while the field has the focus");
    assert_eq!(hex(rig.color), "#1a2b3c");
    rig.blur();
    assert_eq!(rig.hex_text(), "1A2B3C", "it goes when the focus leaves");
    assert_eq!(hex(rig.color), "#1a2b3c");
    let kinds: Vec<_> = rig.events.drain(..).map(|e| (kind(&e), matches!(e, EditEvent::Commit { changed: true, .. }))).collect();
    assert_eq!(kinds, [("Begin", false), ("Commit", false)], "the typing run changes nothing");

    for invalid in ["zz0000", "+00ff00", "-0ff00", "ＦＦ００ＦＦ"] {
        rig.focus_hex();
        rig.type_text(invalid);
        assert_eq!(hex(rig.color), "#1a2b3c", "{invalid:?} is not a colour");
        assert!(rig.node(Role::TextInput, "Hex colour").invalid().is_some(), "{invalid:?} is marked invalid");
        rig.blur();
        assert_eq!(rig.hex_text(), "1A2B3C", "{invalid:?} reverts on blur");
    }

    rig.focus_hex();
    rig.type_text("abcdef");
    assert_eq!(hex(rig.color), "#abcdef", "valid input applies as typed");
    rig.blur();
    assert_eq!(rig.hex_text(), "ABCDEF");

    rig.focus_hex();
    rig.paste("#ff00ff");
    assert_eq!(hex(rig.color), "#ff00ff", "a paste with \"#\" applies");
    rig.focus_hex();
    rig.paste("00ff00");
    assert_eq!(hex(rig.color), "#00ff00", "a paste without \"#\" applies");
    rig.focus_hex();
    rig.paste("336699CC");
    assert_eq!(hex(rig.color), "#336699cc", "eight digits set the alpha too");
    let mut values = rig.events.iter().filter_map(|e| e.value()).peekable();
    assert!(values.peek().is_some() && values.all(|p| p.model.is_none()), "a hex colour carries no model values");
}

/// CMY in percent with one decimal: a host's model whose values a host keeps (as CMYK).
struct Cmy;

const CMY: [Channel; 3] = [
    Channel { name: "Cyan", label: "C", min: 0.0, max: 100.0, unit: "%", decimals: 1, step: 0.1, track_stops: 2 },
    Channel { name: "Magenta", label: "M", min: 0.0, max: 100.0, unit: "%", decimals: 1, step: 0.1, track_stops: 2 },
    Channel { name: "Yellow", label: "Y", min: 0.0, max: 100.0, unit: "%", decimals: 1, step: 0.1, track_stops: 2 },
];

impl ColorModel for Cmy {
    fn id(&self) -> &'static str {
        "cmy"
    }
    fn name(&self) -> &'static str {
        "CMY"
    }
    fn channels(&self) -> &[Channel] {
        &CMY
    }
    fn from_srgb(&self, c: Srgba) -> ModelValues {
        ModelValues::new("cmy", &[100.0 * (1.0 - c.r), 100.0 * (1.0 - c.g), 100.0 * (1.0 - c.b)])
    }
    fn to_srgb(&self, v: &[f32; MAX_CHANNELS]) -> Srgba {
        Srgba::rgb(1.0 - v[0] / 100.0, 1.0 - v[1] / 100.0, 1.0 - v[2] / 100.0)
    }
    fn default_on(&self) -> DefaultOn {
        DefaultOn { full: true, compact: true }
    }
}

#[test]
fn typed_lab_and_host_model_values_stay_while_they_give_the_same_colour() {
    // Lab: an a* far outside sRGB is clamped in the colour and kept in the field.
    let mut rig = Rig::with(Srgba::rgb(0.5, 0.5, 0.5), Some(vec![&LabModel]));
    rig.eight_bit = true;
    rig.tab_to(1);
    assert_eq!(rig.focused_name().as_deref(), Some("Green-red"));
    rig.type_text("120");
    rig.key(Key::Enter, Modifiers::NONE);
    rig.settle();
    let typed = rig.last_model().expect("the Lab values go out with the colour");
    assert_eq!((typed.model, typed.values[1]), ("lab", 120.0));
    assert_eq!(rig.node(Role::SpinButton, "Green-red").numeric_value(), Some(120.0), "the field keeps the typed value");
    assert!(LabModel.from_srgb(rig.color).values[1] < 110.0, "the colour is the clamped one");

    // The host changes the colour: the values follow it.
    rig.color = Srgba::rgb(0.2, 0.4, 0.6);
    rig.settle();
    let a = LabModel.from_srgb(rig.color).values[1];
    assert_eq!(rig.node(Role::SpinButton, "Green-red").numeric_value(), Some(f64::from(a)));

    // A host model: a typed cyan that gives the same 8-bit colour stays as typed.
    let mut rig = Rig::with(Rgba::rgb(229, 128, 51).into(), Some(vec![&Cmy]));
    rig.eight_bit = true;
    let shown = rig.node(Role::SpinButton, "Cyan").numeric_value().expect("cyan");
    assert!((shown - 10.196).abs() < 0.01, "converted from the colour: {shown}");
    rig.tab_to(0);
    assert_eq!(rig.focused_name().as_deref(), Some("Cyan"));
    rig.type_text("10.1");
    rig.key(Key::Enter, Modifiers::NONE);
    rig.settle();
    assert_eq!(hex(rig.color), "#e58033", "the same 8-bit colour");
    let typed = rig.last_model().expect("the host model's values go out");
    assert_eq!((typed.model, typed.values[0]), ("cmy", 10.1));
    assert_eq!(rig.node(Role::SpinButton, "Cyan").numeric_value(), Some(10.1), "kept while the colour is the same");
}

#[test]
fn an_area_drag_is_one_gesture_and_inside_an_edit_scope_only_updates_go_out() {
    let mut rig = Rig::new(Srgba::rgb(0.2, 0.4, 0.6));
    let area = rig.area();
    rig.drag(area.center(), area.left_top() + vec2(area.width() * 0.75, area.height() * 0.25));
    let kinds: Vec<_> = rig.events.iter().map(kind).collect();
    assert_eq!(kinds.first(), Some(&"Begin"), "{kinds:?}");
    assert_eq!(kinds.last(), Some(&"Commit"), "{kinds:?}");
    assert!(kinds[1..kinds.len() - 1].iter().all(|k| *k == "Update"), "one Begin … Commit: {kinds:?}");
    let ids: Vec<_> = rig.events.iter().filter_map(EditEvent::id).collect();
    assert!(ids.windows(2).all(|w| w[0] == w[1]), "one gesture");
    let model = rig.last_model().expect("HSB values");
    assert_eq!((model.model, model.values[1], model.values[2]), ("hsb", 75.0, 75.0));

    // Esc during a drag cancels it, and the host puts the colour back.
    let before = rig.color;
    rig.events.clear();
    rig.frame(vec![Event::PointerMoved(area.center()), button(area.center(), true)]);
    rig.frame(vec![Event::PointerMoved(area.left_top() + vec2(5.0, 5.0))]);
    rig.key(Key::Escape, Modifiers::NONE);
    rig.frame(vec![button(area.left_top(), false)]);
    rig.settle();
    assert_eq!(rig.events.last().map(kind), Some("Cancel"), "{:?}", rig.events);
    assert_eq!(rig.color, before);

    let mut rig = Rig::new(Srgba::rgb(0.2, 0.4, 0.6));
    rig.scoped = true;
    rig.settle();
    let area = rig.area();
    rig.drag(area.center(), area.left_top());
    rig.focus_hex();
    rig.type_text("123456");
    rig.blur();
    assert_eq!(hex(rig.color), "#123456");
    rig.tab_to(0);
    rig.key(Key::ArrowUp, Modifiers::NONE);
    rig.settle();
    assert!(rig.events.len() >= 3, "{:?}", rig.events);
    assert!(rig.events.iter().all(|e| kind(e) == "Update"), "only updates inside a scope: {:?}", rig.events);
}

#[test]
fn accesskit_names_the_parts_and_dialog_tab_visits_only_the_fields() {
    let mut rig = Rig::new(Srgba::rgb(0.2, 0.4, 0.6));
    assert_eq!(rig.nodes(Role::Group, "Foreground colour").len(), 1, "the picker is a named group");
    rig.node(Role::Slider, "Saturation and brightness");
    for name in ["Hue", "Saturation", "Brightness", "Opacity"] {
        rig.node(Role::SpinButton, name);
        let sliders = if name == "Hue" { 2 } else { 1 };
        assert_eq!(rig.nodes(Role::Slider, name).len(), sliders, "{name}: the slider (and the strip) share the field's name");
    }
    rig.node(Role::TextInput, "Hex colour");
    rig.node(Role::Button, "Pick screen color");
    let switch = rig.node(Role::Button, "HSB");
    assert_eq!(switch.description(), Some("Colour model"), "the model switch");
    assert_eq!(switch.has_popup(), Some(HasPopup::Menu));

    rig.focus(rig.fields.0);
    let mut order = Vec::new();
    for _ in 0..6 {
        rig.key(Key::Tab, Modifiers::NONE);
        order.push(rig.focused_name().unwrap_or_default());
    }
    assert_eq!(order, ["Hue", "Saturation", "Brightness", "Hex colour", "Opacity", "After"], "Tab skips the area, strip, sliders and menu");
}

#[test]
fn constrained_drags_stay_one_gesture_and_samples_arrive_as_sets() {
    let mut rig = Rig::new(Srgba::rgb(0.2, 0.4, 0.6));
    rig.web = true;
    rig.settle();
    let area = rig.area();
    rig.drag(area.center(), area.left_top() + vec2(area.width() * 0.37, area.height() * 0.41));
    let kinds: Vec<_> = rig.events.iter().map(kind).collect();
    assert_eq!((kinds.first(), kinds.last()), (Some(&"Begin"), Some(&"Commit")), "the snapped colours don't end the drag: {kinds:?}");
    assert!(rig.color.to_rgba().0[..3].iter().all(|c| c % 0x33 == 0), "every colour is snapped: {}", hex(rig.color));
    assert_eq!(rig.last_model(), None, "values the colour no longer has don't go out");

    rig.events.clear();
    rig.color = rig.color.with_alpha(0.5);
    rig.settle();
    ColorPicker::sampled(&rig.ctx, rig.picker, Srgba::rgb(1.0, 0.6, 0.0));
    rig.settle();
    assert_eq!(rig.events.iter().map(kind).collect::<Vec<_>>(), ["Set"]);
    assert_eq!(hex(rig.color), "#ff990080", "the sample, snapped, with the picker's alpha");
}

/// The model switch is a press menu: a click on the button opens the models as radio items, and
/// a click on one switches the rows to it.
#[test]
fn the_model_switch_menu_switches_the_rows() {
    let mut rig = Rig::new(Srgba::rgb(0.2, 0.4, 0.6));
    let center = |node: &Node| {
        let b = node.bounds().expect("bounds");
        pos2(((b.x0 + b.x1) / 2.0) as f32, ((b.y0 + b.y1) / 2.0) as f32)
    };
    let at = center(rig.node(Role::Button, "HSB"));
    rig.frame(vec![Event::PointerMoved(at), button(at, true)]);
    rig.frame(vec![button(at, false)]);
    rig.settle();
    assert_eq!(rig.node(Role::Button, "HSB").is_expanded(), Some(true));
    assert_eq!(rig.node(Role::MenuItemRadio, "HSB").toggled(), Some(Toggled::True), "the current model is checked");
    let at = center(rig.node(Role::MenuItemRadio, "RGB"));
    rig.frame(vec![Event::PointerMoved(at), button(at, true)]);
    rig.frame(vec![button(at, false)]);
    rig.settle();
    rig.node(Role::Button, "RGB");
    rig.node(Role::SpinButton, "Red");
    assert!(rig.nodes(Role::SpinButton, "Hue").is_empty());
}
