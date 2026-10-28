//! The Full colour picker in a headless egui context: the grid and the radios, a host's model,
//! the compare swatch, warnings, the hex focus and an edit scope.

use craft_theme::{Channel, ColorModel, DefaultOn, HsbModel, LabModel, MAX_CHANNELS, ModelValues, OklchModel, RgbModel, Rgba, Srgba};
use egui::accesskit::{Node, NodeId, Role, Toggled};
use egui::{Context, Event, Id, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, pos2, vec2};

use super::{AreaMode, ColorPicker, ColorPickerLayout, ColorWarning, PickedColor};
use crate::behavior::FieldTab;
use crate::controls::TextField;
use crate::edit::{self, EditEvent};
use crate::localized::LocaleNumbers;
use crate::theme::{Base, install};

/// CMYK in percent with one decimal, without a working space: a host's model.
struct Cmyk;

const CMYK: [Channel; 4] = [
    Channel { name: "Cyan", label: "C", min: 0.0, max: 100.0, unit: "%", decimals: 1, step: 0.1, track_stops: 2 },
    Channel { name: "Magenta", label: "M", min: 0.0, max: 100.0, unit: "%", decimals: 1, step: 0.1, track_stops: 2 },
    Channel { name: "Yellow", label: "Y", min: 0.0, max: 100.0, unit: "%", decimals: 1, step: 0.1, track_stops: 2 },
    Channel { name: "Black", label: "K", min: 0.0, max: 100.0, unit: "%", decimals: 1, step: 0.1, track_stops: 2 },
];

impl ColorModel for Cmyk {
    fn id(&self) -> &'static str {
        "cmyk"
    }
    fn name(&self) -> &'static str {
        "CMYK"
    }
    fn channels(&self) -> &[Channel] {
        &CMYK
    }
    fn from_srgb(&self, c: Srgba) -> ModelValues {
        let k = 1.0 - c.r.max(c.g).max(c.b);
        let ink = |v: f32| if k >= 1.0 { 0.0 } else { 100.0 * (1.0 - v - k) / (1.0 - k) };
        ModelValues::new("cmyk", &[ink(c.r), ink(c.g), ink(c.b), 100.0 * k])
    }
    fn to_srgb(&self, v: &[f32; MAX_CHANNELS]) -> Srgba {
        let k = 1.0 - v[3] / 100.0;
        Srgba::rgb((1.0 - v[0] / 100.0) * k, (1.0 - v[1] / 100.0) * k, (1.0 - v[2] / 100.0) * k)
    }
    fn default_on(&self) -> DefaultOn {
        DefaultOn { full: true, compact: false }
    }
}

/// A host editing one colour with a picker after a text field, in a dialog's [`FieldTab`]
/// scope; it applies every value and puts the colour from before a gesture back on a cancel.
struct Rig {
    ctx: Context,
    time: f64,
    color: Srgba,
    before: Srgba,
    layout: ColorPickerLayout,
    models: Vec<&'static dyn ColorModel>,
    /// The area mode the host holds.
    mode: AreaMode,
    compare: Option<Srgba>,
    warnings: Vec<(&'static str, Srgba)>,
    /// Keeps the colour in 8 bits, as a host with `Rgba` colours does.
    eight_bit: bool,
    scoped: bool,
    shown: bool,
    picker: Id,
    field: Id,
    events: Vec<EditEvent<PickedColor>>,
    nodes: Vec<(NodeId, Node)>,
}

impl Rig {
    fn new(color: Srgba) -> Self {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        ctx.enable_accesskit();
        let mut rig = Self {
            ctx,
            time: 0.0,
            color,
            before: color,
            layout: ColorPickerLayout::Full,
            models: vec![&HsbModel, &RgbModel, &LabModel, &Cmyk],
            mode: AreaMode::Hue,
            compare: None,
            warnings: Vec::new(),
            eight_bit: false,
            scoped: false,
            shown: true,
            picker: Id::NULL,
            field: Id::NULL,
            events: Vec::new(),
            nodes: Vec::new(),
        };
        rig.frame(vec![]);
        rig
    }

    fn frame(&mut self, events: Vec<Event>) {
        let input =
            RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(900.0, 900.0))), time: Some(self.time), events, ..Default::default() };
        self.time += 1.0 / 60.0;
        let numbers = LocaleNumbers::from_tag("en");
        let ctx = self.ctx.clone();
        let mut event = None;
        let output = ctx.run_ui(input, |ui| {
            FieldTab::new("dialog").show(ui, |ui| {
                self.field = TextField::new("before", "Before", "a").show(ui).response.id;
                if !self.shown {
                    return;
                }
                let mut picker = ColorPicker::new("picker", "Fill colour", self.color, &numbers)
                    .layout(self.layout)
                    .models(self.models.iter().copied())
                    .mode(&mut self.mode)
                    .eyedropper("Pick screen color");
                if let Some(current) = self.compare {
                    picker = picker.compare(current);
                }
                for (tip, nearest) in &self.warnings {
                    picker = picker.warning(ColorWarning::new(*tip, *nearest));
                }
                let picked = match self.scoped {
                    true => edit::scope(ui, |ui| picker.show(ui)).inner,
                    false => picker.show(ui),
                };
                self.picker = picked.edit.response.id;
                event = picked.edit.event;
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

    fn key(&mut self, key: Key) {
        self.frame(vec![Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE }]);
        self.frame(vec![]);
    }

    fn type_text(&mut self, text: &str) {
        self.frame(vec![Event::Text(text.to_owned())]);
        self.settle();
    }

    /// Tabs from the field before the picker to the field named `name`, which selects its text.
    fn tab_to(&mut self, name: &str) {
        self.ctx.memory_mut(|m| m.request_focus(self.field));
        self.frame(vec![]);
        for _ in 0..20 {
            self.key(Key::Tab);
            if self.focused_name().as_deref() == Some(name) {
                return;
            }
        }
        panic!("Tab never reached {name:?}");
    }

    fn click(&mut self, pos: Pos2) {
        let button = |pressed| Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
        self.frame(vec![Event::PointerMoved(pos), button(true)]);
        self.frame(vec![button(false)]);
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

    fn nodes(&self, role: Role, name: &str) -> Vec<&Node> {
        self.nodes.iter().map(|(_, n)| n).filter(|n| n.role() == role && self.name(n) == name).collect()
    }

    fn node(&self, role: Role, name: &str) -> &Node {
        self.nodes(role, name).into_iter().next().unwrap_or_else(|| panic!("no {role:?} node named {name:?}"))
    }

    fn rect(&self, role: Role, name: &str) -> Rect {
        let b = self.node(role, name).bounds().expect("bounds");
        Rect::from_min_max(pos2(b.x0 as f32, b.y0 as f32), pos2(b.x1 as f32, b.y1 as f32))
    }

    fn value(&self, role: Role, name: &str) -> f64 {
        self.node(role, name).numeric_value().unwrap_or_else(|| panic!("{name} has no value"))
    }
}

fn hex(color: Srgba) -> String {
    color.to_rgba().to_hex()
}

fn kinds(events: &[EditEvent<PickedColor>]) -> Vec<&'static str> {
    events
        .iter()
        .map(|e| match e {
            EditEvent::Begin { .. } => "Begin",
            EditEvent::Update { .. } => "Update",
            EditEvent::Commit { .. } => "Commit",
            EditEvent::Cancel { .. } => "Cancel",
            EditEvent::Set { .. } => "Set",
        })
        .collect()
}

#[test]
fn full_layout_puts_the_channels_in_two_columns_beside_the_area_with_radios_choosing_the_area() {
    let mut rig = Rig::new(Srgba::rgb(0.2, 0.4, 0.6));
    let area = rig.rect(Role::Slider, "Saturation and brightness");
    assert_eq!(area.width(), area.height(), "the area is square");

    let left = ["Hue", "Saturation", "Brightness", "Red", "Green", "Blue"];
    let right = ["Lightness", "Green-red", "Blue-yellow", "Cyan", "Magenta", "Yellow", "Black"];
    let fields: Vec<Rect> = left.iter().chain(&right).map(|n| rig.rect(Role::SpinButton, n)).collect();
    let hex_field = rig.rect(Role::TextInput, "Hex colour");
    let (l, r) = fields.split_at(left.len());
    assert!(l.iter().all(|f| f.left() == l[0].left()) && r.iter().all(|f| f.left() == r[0].left()), "two aligned columns");
    assert!(area.right() < l[0].left() && l[0].right() < r[0].left(), "the area, then H S B R G B, then L a b and the host's model");
    assert!(l.windows(2).all(|w| w[0].bottom() < w[1].top()) && l[5].bottom() < hex_field.top(), "PhotoCraft's order, the hex field last");
    assert_eq!(hex_field.left(), l[0].left(), "the hex field ends the first column");
    assert!(fields.iter().all(|f| f.width() == l[0].width()), "one channel width");
    assert!(rig.nodes(Role::SpinButton, "Chroma").is_empty(), "OKLCH is off unless the host turns it on");

    // Radios: PhotoCraft's nine; the host's model has none.
    assert_eq!(rig.nodes(Role::RadioGroup, "Field shows").len(), 1);
    let mut radios: Vec<_> = rig.nodes.iter().map(|(_, n)| n).filter(|n| n.role() == Role::RadioButton).collect();
    radios.sort_by_key(|n| n.position_in_set());
    let radios: Vec<_> = radios.into_iter().map(|n| rig.name(n)).collect();
    assert_eq!(radios, ["Hue", "Saturation", "Brightness", "Red", "Green", "Blue", "Lightness", "Green-red", "Blue-yellow"]);
    assert_eq!(rig.node(Role::RadioButton, "Hue").toggled(), Some(Toggled::True));
    rig.node(Role::Button, "Pick screen color");

    // A radio changes what the area shows and the host's mode, and edits nothing.
    let red = rig.rect(Role::RadioButton, "Red");
    rig.click(red.center());
    assert_eq!(rig.mode, AreaMode::Red);
    rig.node(Role::Slider, "Blue and green");
    assert_eq!(rig.node(Role::RadioButton, "Red").toggled(), Some(Toggled::True));
    assert!(rig.events.is_empty(), "the mode isn't a colour edit: {:?}", rig.events);

    // The host turns OKLCH on: a column of its own, with three radios.
    rig.models.push(&OklchModel);
    rig.settle();
    let chroma = rig.rect(Role::SpinButton, "Chroma");
    assert!(chroma.left() > r[0].right(), "a third column");
    assert_eq!(chroma.width(), l[0].width());
    assert_eq!(rig.nodes.iter().filter(|(_, n)| n.role() == Role::RadioButton).count(), 12);
}

#[test]
fn compact_and_full_fields_are_as_wide_as_the_widest_channel_value() {
    let mut rig = Rig::new(Srgba::rgb(0.2, 0.4, 0.6));
    let full = rig.rect(Role::SpinButton, "Hue").width();
    rig.layout = ColorPickerLayout::Compact;
    rig.settle();
    assert_eq!(rig.rect(Role::SpinButton, "Hue").width(), full, "one width rule in both layouts");
    let numbers = LocaleNumbers::from_tag("en");
    let ctx = rig.ctx.clone();
    let mut widest = 0.0;
    let _ = ctx.run_ui(RawInput::default(), |ui| {
        let unit = crate::localized::Unit::new("{n}°");
        widest = crate::controls::NumberField::fit_width(ui, craft_theme::ControlSize::Md, &numbers, 0.0..=360.0, 0, &unit);
    });
    assert!(full >= widest, "\"360°\" fits: {full} < {widest}");
}

#[test]
fn typed_host_model_values_survive_in_the_grid_while_the_colour_is_the_same() {
    let mut rig = Rig::new(Rgba::rgb(229, 128, 51).into());
    rig.eight_bit = true;
    let magenta = rig.value(Role::SpinButton, "Magenta");
    assert!((magenta - 44.10).abs() < 0.01, "converted from the colour: {magenta}");
    rig.tab_to("Magenta");
    rig.type_text("44.2");
    rig.key(Key::Enter);
    rig.settle();
    assert_eq!(hex(rig.color), "#e58033", "the same 8-bit colour");
    let model = rig.events.iter().rev().find_map(|e| e.value()).and_then(|p| p.model).expect("the host model's values go out");
    assert_eq!((model.model, model.values[1]), ("cmyk", 44.2));
    assert_eq!(rig.value(Role::SpinButton, "Magenta"), 44.2, "kept while the colour is the same");
    assert_eq!(rig.value(Role::SpinButton, "Green"), 128.0, "the other models show the colour");

    rig.color = Srgba::rgb(0.2, 0.4, 0.6);
    rig.settle();
    let converted = f64::from(Cmyk.from_srgb(rig.color).values[1]);
    assert!((rig.value(Role::SpinButton, "Magenta") - converted).abs() < 1e-4, "recomputed after the host's own change");
}

#[test]
fn compare_and_warnings_set_their_colours_and_are_named_buttons() {
    let mut rig = Rig::new(Srgba::rgb(0.2, 0.4, 0.6));
    rig.compare = Some(Srgba::rgb(1.0, 0.0, 0.0));
    rig.warnings = vec![("Not a web colour", Srgba::rgb(0.2, 0.4, 0.6)), ("Out of gamut for printing", Srgba::rgb(0.0, 0.6, 0.4))];
    rig.settle();
    assert_eq!(rig.node(Role::Image, "New colour").value(), Some("#336699"));
    assert_eq!(rig.node(Role::Button, "Restore current colour").value(), Some("#ff0000"));
    let new = rig.rect(Role::Image, "New colour");
    let current = rig.rect(Role::Button, "Restore current colour");
    assert!(new.bottom() <= current.top() && new.left() == current.left(), "new over current");
    rig.node(Role::Button, "Not a web colour");

    rig.click(current.center());
    assert_eq!(kinds(&rig.events), ["Set"]);
    assert_eq!(hex(rig.color), "#ff0000", "the current colour is back");

    rig.events.clear();
    let warning = rig.rect(Role::Button, "Out of gamut for printing");
    rig.click(warning.center());
    assert_eq!(kinds(&rig.events), ["Set"]);
    assert_eq!(hex(rig.color), hex(Srgba::rgb(0.0, 0.6, 0.4)), "the warning's colour");
    assert!(rig.events.iter().all(|e| e.value().is_some_and(|p| p.model.is_none())), "a colour set as such carries no values");
}

#[test]
fn the_full_picker_opens_with_the_hex_text_selected_and_focus_hex_selects_it_on_request() {
    let mut rig = Rig::new(Rgba::rgb(0x1a, 0x2b, 0x3c).into());
    rig.settle();
    assert_eq!(rig.focused_name().as_deref(), Some("Hex colour"), "the hex field has the focus on open");
    rig.type_text("123456");
    assert_eq!(hex(rig.color), "#123456", "typing replaces the selected text");

    // Shown again after a frame without it: focused again.
    rig.ctx.memory_mut(|m| m.request_focus(rig.field));
    rig.settle();
    rig.shown = false;
    rig.frame(vec![]);
    rig.shown = true;
    rig.settle();
    assert_eq!(rig.focused_name().as_deref(), Some("Hex colour"));

    // The Compact picker only on request.
    let mut rig = Rig::new(Rgba::rgb(0x1a, 0x2b, 0x3c).into());
    rig.layout = ColorPickerLayout::Compact;
    rig.ctx.memory_mut(|m| m.request_focus(rig.field));
    rig.settle();
    assert_eq!(rig.focused_name().as_deref(), Some("Before"));
    ColorPicker::focus_hex(&rig.ctx, rig.picker);
    rig.settle();
    assert_eq!(rig.focused_name().as_deref(), Some("Hex colour"));
    rig.type_text("abcdef");
    assert_eq!(hex(rig.color), "#abcdef");
}

#[test]
fn inside_an_edit_scope_the_full_picker_sends_only_updates_and_its_radios_none() {
    let mut rig = Rig::new(Srgba::rgb(0.2, 0.4, 0.6));
    rig.scoped = true;
    rig.compare = Some(Srgba::rgb(1.0, 0.0, 0.0));
    rig.settle();
    let lab = rig.rect(Role::RadioButton, "Lightness");
    rig.click(lab.center());
    assert_eq!(rig.mode, AreaMode::LabL);
    assert!(rig.events.is_empty(), "{:?}", rig.events);

    rig.tab_to("Red");
    rig.type_text("200");
    rig.key(Key::Enter);
    let area = rig.rect(Role::Slider, "Green-red and blue-yellow");
    rig.click(area.center());
    let current = rig.rect(Role::Button, "Restore current colour");
    rig.click(current.center());
    assert_eq!(hex(rig.color), "#ff0000");
    assert!(rig.events.len() >= 3, "{:?}", rig.events);
    assert!(kinds(&rig.events).iter().all(|k| *k == "Update"), "only updates inside a scope: {:?}", kinds(&rig.events));
}
