//! The Full colour picker in a headless egui context: the grid and the radios, a host's model,
//! the compare swatch, warnings, the hex focus and an edit scope.

use craft_theme::{Channel, ColorModel, DefaultOn, HsbModel, LabModel, MAX_CHANNELS, ModelValues, OklchModel, RgbModel, Rgba, Srgba};
use egui::accesskit::{Node, NodeId, Role, Toggled};
use egui::{Context, Event, Id, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, pos2, vec2};

use super::{AreaMode, ColorPicker, ColorPickerLayout, ColorWarning, PickedColor};
use craft_ui::behavior::FieldTab;
use craft_ui::controls::TextField;
use craft_ui::dialog::{ButtonRole, Clicked, FooterButton};
use craft_ui::edit::{self, EditEvent, Ending};
use craft_ui::localized::LocaleNumbers;
use craft_ui::theme::{Base, install};

/// CMYK in percent with one decimal, without a working space: a host's model.
struct Cmyk;

const CMYK: [Channel; 4] = [
    Channel { name: "Cyan", qualified: "CMYK cyan", label: "C", min: 0.0, max: 100.0, unit: "%", decimals: 1, step: 0.1, track_stops: 2 },
    Channel { name: "Magenta", qualified: "CMYK magenta", label: "M", min: 0.0, max: 100.0, unit: "%", decimals: 1, step: 0.1, track_stops: 2 },
    Channel { name: "Yellow", qualified: "CMYK yellow", label: "Y", min: 0.0, max: 100.0, unit: "%", decimals: 1, step: 0.1, track_stops: 2 },
    Channel { name: "Black", qualified: "CMYK black", label: "K", min: 0.0, max: 100.0, unit: "%", decimals: 1, step: 0.1, track_stops: 2 },
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
    /// The host's Only Web Colors setting, when the picker offers it.
    web: Option<bool>,
    warnings: Vec<(&'static str, Srgba)>,
    /// Keeps the colour in 8 bits, as a host with `Rgba` colours does.
    eight_bit: bool,
    scoped: bool,
    shown: bool,
    focus_on_open: bool,
    side: Vec<FooterButton>,
    side_clicked: Vec<Clicked>,
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
            web: None,
            warnings: Vec::new(),
            eight_bit: false,
            scoped: false,
            shown: true,
            focus_on_open: true,
            side: Vec::new(),
            side_clicked: Vec::new(),
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
                    .focus_on_open(self.focus_on_open)
                    .eyedropper("Pick screen color");
                if let Some(current) = self.compare {
                    picker = picker.compare(current);
                }
                let mut web = self.web;
                if let Some(web) = web.as_mut() {
                    picker = picker.web_colors(web);
                }
                for (tip, nearest) in &self.warnings {
                    picker = picker.warning(ColorWarning::new(*tip, *nearest));
                }
                if !self.side.is_empty() {
                    picker = picker.side_actions(self.side.clone());
                }
                let picked = match self.scoped {
                    true => edit::scope(ui, |ui| picker.show(ui)).inner,
                    false => picker.show(ui),
                };
                self.picker = picked.edit.response.id;
                self.side_clicked.extend(picked.side_clicked);
                event = picked.edit.event;
                self.web = web;
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
            _ => "other",
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
        let unit = craft_ui::localized::Unit::new("{n}°");
        widest = craft_ui::controls::NumberField::fit_width(ui, craft_theme::ControlSize::Md, &numbers, 0.0..=360.0, 0, &unit);
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

/// HSL on HSB's values: a host's model whose channel names repeat built-in ones.
struct Hsl;

const HSL: [Channel; 3] = [
    Channel { name: "Hue", qualified: "HSL hue", label: "H", min: 0.0, max: 360.0, unit: "°", decimals: 0, step: 1.0, track_stops: 13 },
    Channel { name: "Saturation", qualified: "HSL saturation", label: "S", min: 0.0, max: 100.0, unit: "%", decimals: 0, step: 1.0, track_stops: 2 },
    Channel { name: "Lightness", qualified: "HSL lightness", label: "L", min: 0.0, max: 100.0, unit: "%", decimals: 0, step: 1.0, track_stops: 2 },
];

impl ColorModel for Hsl {
    fn id(&self) -> &'static str {
        "hsl"
    }
    fn name(&self) -> &'static str {
        "HSL"
    }
    fn channels(&self) -> &[Channel] {
        &HSL
    }
    fn from_srgb(&self, c: Srgba) -> ModelValues {
        ModelValues::new("hsl", &HsbModel.from_srgb(c).values[..3])
    }
    fn to_srgb(&self, v: &[f32; MAX_CHANNELS]) -> Srgba {
        HsbModel.to_srgb(v)
    }
    fn default_on(&self) -> DefaultOn {
        DefaultOn { full: true, compact: true }
    }
}

#[test]
fn channels_whose_names_clash_are_named_with_their_model_in_both_layouts() {
    let mut rig = Rig::new(Srgba::rgb(0.2, 0.4, 0.6));
    rig.models.push(&OklchModel);
    rig.settle();
    for name in ["HSB hue", "OKLCH hue", "Lab lightness", "OKLCH lightness"] {
        rig.node(Role::SpinButton, name);
        rig.node(Role::RadioButton, name);
    }
    for plain in ["Saturation", "Red", "Chroma", "Green-red"] {
        rig.node(Role::SpinButton, plain);
    }
    assert!(rig.nodes(Role::SpinButton, "Hue").is_empty() && rig.nodes(Role::SpinButton, "Lightness").is_empty());
    rig.node(Role::Slider, "HSB hue");

    // A host model that repeats built-in names.
    rig.models = vec![&HsbModel, &RgbModel, &LabModel, &Hsl];
    rig.settle();
    for name in ["HSB hue", "HSL hue", "HSB saturation", "HSL saturation", "Lab lightness", "HSL lightness"] {
        rig.node(Role::SpinButton, name);
    }
    rig.node(Role::SpinButton, "Brightness");

    // Compact: the rows' model and the area's.
    rig.layout = ColorPickerLayout::Compact;
    rig.models = vec![&HsbModel, &RgbModel, &OklchModel];
    rig.settle();
    rig.node(Role::SpinButton, "Hue");
    assert_eq!(rig.nodes(Role::Slider, "Hue").len(), 2, "the slider and the strip, no clash");
    rig.mode = AreaMode::OklchH;
    rig.settle();
    rig.node(Role::SpinButton, "HSB hue");
    assert_eq!(rig.nodes(Role::Slider, "HSB hue").len(), 1, "the row's slider");
    assert_eq!(rig.nodes(Role::Slider, "OKLCH hue").len(), 1, "the strip");
}

#[test]
fn right_to_left_mirrors_the_full_layout_and_keeps_the_keys_of_area_and_strip() {
    let mut rig = Rig::new(HsbModel.to_srgb(&[200.0, 50.0, 50.0, 0.0]));
    craft_ui::direction::apply(&rig.ctx, craft_ui::direction::Direction::Rtl);
    rig.settle();
    let area = rig.rect(Role::Slider, "Saturation and brightness");
    let strip = rig.rect(Role::Slider, "Hue");
    let (hue, lightness) = (rig.rect(Role::SpinButton, "Hue"), rig.rect(Role::SpinButton, "Lightness"));
    let radio = rig.rect(Role::RadioButton, "Hue");
    assert!(strip.right() < area.left(), "the strip at the reading end of the area");
    assert!(hue.right() < strip.left(), "the grid after the area and strip");
    assert!(lightness.right() < hue.left(), "H S B R G B first, at the right");
    assert!(radio.left() > hue.right(), "each radio and label before its field");

    // The axes and the keys don't mirror: Right still raises saturation, Up brightness and hue.
    rig.click(area.center());
    let (s, b) = (rig.value(Role::SpinButton, "Saturation"), rig.value(Role::SpinButton, "Brightness"));
    rig.events.clear();
    rig.key(Key::ArrowRight);
    rig.key(Key::ArrowUp);
    rig.settle();
    assert_eq!(kinds(&rig.events), ["Set", "Set"]);
    assert_eq!((rig.value(Role::SpinButton, "Saturation"), rig.value(Role::SpinButton, "Brightness")), (s + 1.0, b + 1.0));
    rig.click(strip.center());
    let h = rig.value(Role::SpinButton, "Hue");
    rig.key(Key::ArrowUp);
    rig.settle();
    assert_eq!(rig.value(Role::SpinButton, "Hue"), h + 1.0);
}

/// A click on the compare swatch's current half ends a typing run in H in the same frame: the
/// typed value is committed, then the current colour is set.
#[test]
fn a_restore_click_that_ends_a_typing_run_restores_the_current_colour() {
    let mut rig = Rig::new(Srgba::rgb(0.2, 0.4, 0.6));
    rig.compare = Some(Srgba::rgb(1.0, 0.0, 0.0));
    rig.settle();
    rig.tab_to("Hue");
    rig.type_text("12");
    rig.events.clear();
    let current = rig.rect(Role::Button, "Restore current colour");
    rig.click(current.center());
    let kinds = kinds(&rig.events);
    assert_eq!(kinds.iter().filter(|k| **k == "Commit").count(), 1, "the typed hue is committed: {kinds:?}");
    assert_eq!(kinds.last(), Some(&"Set"), "then the current colour is set: {kinds:?}");
    assert_eq!(hex(rig.color), "#ff0000");
}

/// A screen reader's click on the compare swatch's current half takes no focus: the typing run in
/// H still ends (the field leaves editing, the typed value is committed), then the current
/// colour is set as its own step.
#[test]
fn a_screen_readers_compare_click_during_a_typing_run_ends_the_run_and_restores() {
    let mut rig = Rig::new(Srgba::rgb(0.2, 0.4, 0.6));
    rig.compare = Some(Srgba::rgb(1.0, 0.0, 0.0));
    rig.settle();
    rig.tab_to("Hue");
    rig.type_text("12");
    rig.events.clear();
    let target = rig
        .nodes
        .iter()
        .find(|(_, n)| n.role() == Role::Button && rig.name(n) == "Restore current colour")
        .map(|(id, _)| *id)
        .expect("restore button");
    rig.frame(vec![Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
        action: egui::accesskit::Action::Click,
        target_tree: egui::accesskit::TreeId::ROOT,
        target_node: target,
        data: None,
    })]);
    rig.settle();
    assert_eq!(kinds(&rig.events), ["Commit", "Set"], "the typed hue is committed, then the current colour is set");
    assert!(matches!(rig.events[0], EditEvent::Commit { changed: true, .. }));
    assert_eq!(rig.focused_name(), None, "the field left editing");
    assert_eq!(hex(rig.color), "#ff0000");
}

/// A tap on the area (press and release in one frame) that ends a partial hex run sets the
/// tapped colour.
#[test]
fn an_area_tap_that_ends_a_partial_hex_run_sets_the_tapped_colour() {
    let start = Srgba::rgb(0.2, 0.4, 0.6);
    let mut rig = Rig::new(start);
    rig.tab_to("Hex colour");
    rig.type_text("AB");
    let area = rig.rect(Role::Slider, "Saturation and brightness");
    let at = area.left_top() + vec2(area.width() * 0.9, area.height() * 0.1);
    let button = |pressed| Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
    rig.frame(vec![Event::PointerMoved(at), button(true), button(false)]);
    rig.settle();
    assert_eq!(kinds(&rig.events), ["Begin", "Commit", "Set"], "the run ends, then the tap sets");
    let tapped = HsbModel.from_srgb(rig.color).values.map(f32::round);
    assert_eq!(tapped[1..3], [90.0, 90.0], "the tap's saturation and brightness: {}", hex(rig.color));
}

/// The picker stops being drawn during a typing run in S (the host hides the panel): its gesture
/// ends as the field's run ends when the focus leaves it. A number goes out as typed, so the host
/// has it and keeps it; text after it that reads as no number ends the run at its start.
#[test]
fn an_interrupted_typing_run_ends_as_its_field_would() {
    let start = Srgba::rgb(0.2, 0.4, 0.6);
    let begun =
        |rig: &Rig| rig.events.iter().find_map(|e| if let EditEvent::Begin { id, .. } = e { Some(*id) } else { None }).expect("the run began");

    let mut rig = Rig::new(start);
    rig.tab_to("Saturation");
    rig.type_text("40");
    assert_eq!(kinds(&rig.events), ["Begin", "Update"], "a number goes out as typed");
    let id = begun(&rig);
    rig.shown = false;
    rig.frame(vec![]);
    assert!(!edit::is_live(&rig.ctx, id));
    assert_eq!(edit::ending(&rig.ctx, id), Ending::Commit { changed: true });
    assert_eq!(edit::unreported::<PickedColor>(&rig.ctx, id), None, "the host has the typed value");
    let s = HsbModel.from_srgb(rig.color).values[1];
    assert!((s - 40.0).abs() < 0.5, "S = {s}");

    for interrupted in [false, true] {
        let mut rig = Rig::new(start);
        rig.tab_to("Saturation");
        rig.type_text("40");
        rig.type_text("-");
        let id = begun(&rig);
        match interrupted {
            true => {
                rig.shown = false;
                rig.frame(vec![]);
                assert_eq!(edit::ending(&rig.ctx, id), Ending::Commit { changed: false }, "the host reverts the run");
            }
            false => {
                rig.key(Key::Tab);
                assert!(matches!(rig.events.last(), Some(EditEvent::Commit { changed: false, .. })), "{:?}", kinds(&rig.events));
                assert_eq!(hex(rig.color), hex(start));
            }
        }
    }
}

/// Pasted text goes to the hex field whole: eight digits with the alpha off are not a colour,
/// and a colour copied with spaces round it applies.
#[test]
fn pasted_hex_text_is_parsed_whole() {
    let start = Srgba::rgb(0.2, 0.4, 0.6);
    let mut rig = Rig::new(start);
    rig.tab_to("Hex colour");
    rig.frame(vec![Event::Paste("#AABBCCDD".to_owned())]);
    rig.settle();
    assert_eq!(hex(rig.color), hex(start), "eight digits without alpha are not cut to six");

    rig.tab_to("Hex colour");
    rig.frame(vec![Event::Paste("  #aabbcc ".to_owned())]);
    rig.settle();
    assert_eq!(hex(rig.color), "#aabbcc");
}

/// A host that shows the Full picker in a panel turns the opening focus off: showing the panel
/// again leaves the focus where it was.
#[test]
fn a_picker_shown_again_without_focus_on_open_leaves_the_focus() {
    let mut rig = Rig::new(Srgba::rgb(0.2, 0.4, 0.6));
    rig.focus_on_open = false;
    rig.shown = false;
    rig.settle();
    rig.ctx.memory_mut(|m| m.request_focus(rig.field));
    rig.frame(vec![]);
    rig.shown = true;
    rig.settle();
    assert_eq!(rig.focused_name().as_deref(), Some("Before"), "the focus stays in the field before the picker");

    rig.focus_on_open = true;
    rig.shown = false;
    rig.settle();
    rig.shown = true;
    rig.settle();
    assert_eq!(rig.focused_name().as_deref(), Some("Hex colour"), "a dialog's picker takes it when it opens");
}

#[test]
fn two_warning_chips_in_one_ui_keep_their_own_clicks() {
    use crate::ui::ColorWarningChip;
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let mut time = 0.0;
    // Each chip's rect, id and click in the last frame.
    let mut frame = |events: Vec<Event>| {
        let input =
            RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(300.0, 200.0))), time: Some(time), events, ..Default::default() };
        time += 1.0 / 60.0;
        let mut seen = Vec::new();
        let output = ctx.run_ui(input, |ui| {
            ui.horizontal(|ui| {
                for salt in ["first", "second"] {
                    let warning = ColorWarning::new("Not a web colour", Srgba::rgb(0.2, 0.4, 0.6));
                    let response = ColorWarningChip::new(salt, warning).show(ui);
                    seen.push((response.rect, response.id, response.clicked()));
                }
            });
        });
        output.drop_without_applying_deltas();
        seen
    };
    let seen = frame(vec![]);
    assert_ne!(seen[0].1, seen[1].1, "chips with different salts have different ids");
    let at = seen[1].0.center();
    let button = |pressed| Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
    frame(vec![Event::PointerMoved(at)]);
    frame(vec![Event::PointerMoved(at), button(true)]);
    let seen = frame(vec![Event::PointerMoved(at), button(false)]);
    let clicked = [seen[0].2, seen[1].2];
    assert_eq!(clicked, [false, true], "a click on the second chip is the second's");
}

fn is_web(color: Srgba) -> bool {
    color.to_rgba().0[..3].iter().all(|c| c % 0x33 == 0)
}

#[test]
fn only_web_colors_snaps_every_control_and_reports_the_snapped_model_values_in_hsb_mode() {
    let mut rig = Rig::new(Srgba::rgb(0.2, 0.4, 0.6));
    rig.web = Some(true);
    rig.settle();
    rig.events.clear();
    // The area takes the nearest web colour under the pointer; the report keeps HSB values.
    let area = rig.rect(Role::Slider, AreaMode::Hue.name());
    for (fx, fy) in [(0.3, 0.2), (0.77, 0.61), (0.52, 0.93)] {
        rig.click(pos2(area.left() + fx * area.width(), area.top() + fy * area.height()));
        assert!(is_web(rig.color), "{}", hex(rig.color));
    }
    let picked = rig.events.iter().rev().find_map(|e| e.value()).expect("the clicks reported");
    assert_eq!(picked.model.map(|m| m.model), Some("hsb"), "the area's own values go out with the snapped colour");
    // The strip, the hex field and a number field snap as well.
    let strip = rig.rect(Role::Slider, "Hue");
    rig.click(strip.center() + vec2(0.0, 17.0));
    assert!(is_web(rig.color), "{}", hex(rig.color));
    rig.tab_to("Hex colour");
    rig.type_text("123456");
    assert_eq!(hex(rig.color), "#003366");
    rig.tab_to("Red");
    rig.type_text("100");
    assert!(is_web(rig.color) && rig.color.to_rgba().0[0] == 0x66, "{}", hex(rig.color));
}

#[test]
fn the_only_web_colors_box_toggles_by_pointer_and_keyboard_and_snaps_when_it_turns_on() {
    let mut rig = Rig::new(Rgba::rgb(0x12, 0x34, 0x56).into());
    assert!(rig.nodes(Role::CheckBox, "Only Web Colors").is_empty(), "the box is offered only when the host holds the setting");
    rig.web = Some(false);
    rig.settle();
    rig.events.clear();
    assert_eq!(rig.node(Role::CheckBox, "Only Web Colors").toggled(), Some(Toggled::False));

    let at = rig.rect(Role::CheckBox, "Only Web Colors").center();
    rig.click(at);
    assert_eq!(rig.web, Some(true));
    assert_eq!(hex(rig.color), "#003366", "turning it on snaps the colour at once");
    assert!(matches!(rig.events.as_slice(), [EditEvent::Set { .. }]), "{:?}", rig.events);
    rig.click(at);
    assert_eq!(rig.web, Some(false));
    assert_eq!(hex(rig.color), "#003366", "turning it off keeps the colour");

    // Ctrl+Tab (Tab only visits fields in a dialog) reaches the box and Space toggles it.
    rig.ctx.memory_mut(|m| m.request_focus(rig.field));
    rig.frame(vec![]);
    let ctrl = Modifiers { ctrl: true, ..Modifiers::NONE };
    for _ in 0..40 {
        if rig.focused_name().as_deref() == Some("Only Web Colors") {
            break;
        }
        rig.frame(vec![Event::Key { key: Key::Tab, physical_key: None, pressed: true, repeat: false, modifiers: ctrl }]);
        rig.frame(vec![]);
    }
    assert_eq!(rig.focused_name().as_deref(), Some("Only Web Colors"));
    rig.key(Key::Space);
    assert_eq!(rig.web, Some(true));
}

#[test]
fn side_actions_stack_in_a_column_at_the_trailing_side_and_report_their_click() {
    let mut rig = Rig::new(Srgba::rgb(0.2, 0.4, 0.6));
    rig.side = vec![
        FooterButton::new(ButtonRole::Default, "OK"),
        FooterButton::new(ButtonRole::Cancel, "Cancel"),
        FooterButton::new(ButtonRole::Alternate, "Add to Swatches"),
    ];
    rig.settle();
    let (ok, cancel, add) = (rig.rect(Role::Button, "OK"), rig.rect(Role::Button, "Cancel"), rig.rect(Role::Button, "Add to Swatches"));
    assert!(ok.min.x == cancel.min.x && cancel.min.x == add.min.x, "one column");
    assert!(ok.max.y <= cancel.min.y && cancel.max.y <= add.min.y, "in the listed order, top to bottom");
    assert!(ok.width() == add.width(), "as wide as the longest label");
    let hex = rig.rect(Role::TextInput, "Hex colour");
    let area = rig.rect(Role::Slider, "Saturation and brightness");
    assert!(ok.min.x >= area.max.x, "beside the area, at the trailing side");
    assert!(ok.max.x >= hex.max.x && add.max.y < hex.min.y, "over the channel grid, in the picker's width");

    rig.click(add.center());
    rig.click(ok.center());
    let kinds: Vec<_> = rig.side_clicked.iter().map(|c| (c.index, c.role)).collect();
    assert_eq!(kinds, [(2, ButtonRole::Alternate), (0, ButtonRole::Default)]);
    assert!(rig.events.is_empty(), "a side action edits nothing");

    rig.side.clear();
    rig.settle();
    assert!(rig.nodes(Role::Button, "OK").is_empty(), "no actions, no column");
}

#[test]
fn the_full_pickers_eyedropper_button_has_a_tooltip_from_a_registered_word() {
    let mut rig = Rig::new(Srgba::rgb(0.2, 0.4, 0.6));
    rig.settle();
    let button = rig.node(Role::Button, "Pick screen color");
    assert_eq!(button.description(), Some("Sample color from the screen"));
    assert!(ColorPicker::TEXTS.contains(&"Sample color from the screen"));
}

#[test]
fn a_hex_focus_request_made_in_a_sizing_pass_reaches_the_field_in_the_pass_that_is_shown() {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let numbers = LocaleNumbers::from_tag("en");
    let mut time = 0.0;
    let mut run = |sizing: bool, request: Option<Id>| -> Id {
        let input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(900.0, 900.0))), time: Some(time), ..Default::default() };
        time += 1.0 / 60.0;
        let mut id = Id::NULL;
        let output = ctx.run_ui(input, |ui| {
            let builder = egui::UiBuilder::new().id_salt("host");
            let builder = if sizing { builder.sizing_pass().invisible() } else { builder };
            ui.scope_builder(builder, |ui| {
                if let Some(picker) = request {
                    ColorPicker::focus_hex(ui.ctx(), picker);
                }
                let picked =
                    ColorPicker::new("picker", "Fill colour", Srgba::rgb(0.2, 0.4, 0.6), &numbers).layout(ColorPickerLayout::Compact).show(ui);
                id = picked.edit.response.id;
            });
        });
        output.drop_without_applying_deltas();
        id
    };
    let picker = run(false, None);
    let hex = super::hex::hex_id(picker);
    assert_eq!(run(true, Some(picker)), picker, "the sizing pass has the picker's id");
    run(false, None);
    run(false, None);
    assert_eq!(ctx.memory(|m| m.focused()), Some(hex), "the shown pass took the request");
}

#[test]
fn enter_presses_the_default_side_action_and_esc_the_cancel_one_as_a_dialog_footer_does() {
    let mut rig = Rig::new(Srgba::rgb(0.2, 0.4, 0.6));
    rig.side = vec![
        FooterButton::new(ButtonRole::Default, "OK"),
        FooterButton::new(ButtonRole::Cancel, "Cancel"),
        FooterButton::new(ButtonRole::Alternate, "Add to Swatches"),
    ];
    rig.settle();
    assert_eq!(rig.focused_name().as_deref(), Some("Hex colour"));
    rig.type_text("ff0000");
    rig.key(Key::Enter);
    assert_eq!(hex(rig.color), "#ff0000", "Enter in the hex field applies it ...");
    let roles: Vec<_> = rig.side_clicked.drain(..).map(|c| c.role).collect();
    assert_eq!(roles, [ButtonRole::Default], "... and then answers the default action, as a footer does");

    rig.tab_to("Hex colour");
    rig.key(Key::Escape);
    let roles: Vec<_> = rig.side_clicked.drain(..).map(|c| c.role).collect();
    assert_eq!(roles, [ButtonRole::Cancel]);

    let tab = rig.rect(Role::Button, "Add to Swatches").center();
    rig.click(tab);
    rig.side_clicked.clear();
    rig.key(Key::Enter);
    assert!(rig.side_clicked.iter().all(|c| c.role != ButtonRole::Default), "a focused button presses itself, not the default");

    rig.side_clicked.clear();
    rig.ctx.memory_mut(|m| m.surrender_focus(m.focused().unwrap_or(Id::NULL)));
    rig.frame(vec![]);
    rig.key(Key::Enter);
    assert!(rig.side_clicked.is_empty(), "nothing has the focus");
}

/// A picker with side actions as the body of a dialog, and a second modal dialog a host opens
/// over it.
struct Stacked {
    ctx: Context,
    numbers: LocaleNumbers,
    time: f64,
    first: craft_ui::dialog::DialogState,
    second: craft_ui::dialog::DialogState,
    first_open: bool,
    second_open: bool,
    color: Srgba,
    picker: Id,
    side_clicked: Vec<Clicked>,
    second_outcomes: Vec<craft_ui::dialog::Outcome>,
}

impl Stacked {
    fn new() -> Self {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        let mut stacked = Self {
            ctx,
            numbers: LocaleNumbers::from_tag("en"),
            time: 0.0,
            first: craft_ui::dialog::DialogState::new(),
            second: craft_ui::dialog::DialogState::new(),
            first_open: true,
            second_open: false,
            color: Srgba::rgb(0.2, 0.4, 0.6),
            picker: Id::NULL,
            side_clicked: Vec::new(),
            second_outcomes: Vec::new(),
        };
        for _ in 0..4 {
            stacked.frame(vec![]);
        }
        stacked
    }

    fn frame(&mut self, events: Vec<Event>) {
        let input =
            RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(900.0, 900.0))), time: Some(self.time), events, ..Default::default() };
        self.time += 1.0 / 60.0;
        let ctx = self.ctx.clone();
        let output = ctx.run_ui(input, |ui| {
            let ctx = ui.ctx().clone();
            if self.first_open {
                let (color, numbers) = (self.color, &self.numbers);
                let (mut picker, mut clicked) = (Id::NULL, None);
                craft_ui::dialog::Dialog::new("first", "Colour Picker").initial_focus(craft_ui::dialog::InitialFocus::None).show(
                    &ctx,
                    &mut self.first,
                    &mut (),
                    |ui, ()| {
                        let shown = ColorPicker::new("picker", "Fill colour", color, numbers)
                            .layout(ColorPickerLayout::Full)
                            .side_actions([FooterButton::new(ButtonRole::Default, "OK"), FooterButton::new(ButtonRole::Cancel, "Cancel")])
                            .show(ui);
                        picker = shown.edit.response.id;
                        clicked = shown.side_clicked;
                    },
                );
                self.picker = picker;
                self.side_clicked.extend(clicked);
            }
            if self.second_open {
                let shown = craft_ui::dialog::Dialog::new("second", "Second")
                    .initial_focus(craft_ui::dialog::InitialFocus::None)
                    .footer(craft_ui::dialog::Footer::standard())
                    .show(&ctx, &mut self.second, &mut (), |ui, ()| {
                        ui.label("Over the picker");
                    });
                self.second_outcomes.extend(shown.outcome);
                self.second_open &= !shown.outcome.is_some_and(craft_ui::dialog::Outcome::closes);
            }
        });
        output.drop_without_applying_deltas();
    }

    fn key(&mut self, key: Key) {
        self.frame(vec![Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE }]);
        self.frame(vec![]);
    }
}

#[test]
fn side_action_keys_go_to_a_modal_dialog_opened_over_the_picker_dialog() {
    let mut rig = Stacked::new();
    let hex = super::hex::hex_id(rig.picker);
    rig.ctx.memory_mut(|m| m.request_focus(hex));
    rig.frame(vec![]);
    rig.key(Key::Escape);
    assert_eq!(rig.side_clicked.drain(..).map(|c| c.role).collect::<Vec<_>>(), [ButtonRole::Cancel], "alone, the picker answers Esc");

    rig.second_open = true;
    for _ in 0..4 {
        rig.frame(vec![]);
    }
    // The focus stays in the picker's layer: only the modal above it tells the picker not to answer.
    rig.ctx.memory_mut(|m| m.request_focus(hex));
    rig.frame(vec![]);
    rig.key(Key::Enter);
    assert!(rig.side_clicked.is_empty(), "Enter is the top modal's: {:?}", rig.side_clicked.len());
    assert_eq!(std::mem::take(&mut rig.second_outcomes), [craft_ui::dialog::Outcome::Ok], "the top modal took Enter and closed");
    rig.second = craft_ui::dialog::DialogState::new();
    rig.second_open = true;
    for _ in 0..4 {
        rig.frame(vec![]);
    }
    rig.ctx.memory_mut(|m| m.request_focus(hex));
    rig.frame(vec![]);
    rig.key(Key::Escape);
    assert!(rig.side_clicked.is_empty(), "Esc is the top modal's");
    assert_eq!(rig.second_outcomes, [craft_ui::dialog::Outcome::Cancel], "the top modal took Esc");
}

#[test]
fn the_enter_that_opens_a_picker_does_not_press_its_side_action() {
    let mut rig = Rig::new(Srgba::rgb(0.2, 0.4, 0.6));
    rig.side = vec![FooterButton::new(ButtonRole::Default, "OK"), FooterButton::new(ButtonRole::Cancel, "Cancel")];
    rig.settle();
    assert_eq!(rig.focused_name().as_deref(), Some("Hex colour"), "the focus is inside when the picker goes");
    // It stays hidden for a few frames, then opens on this very Enter, with the old state of its keys.
    rig.shown = false;
    rig.settle();
    rig.shown = true;
    rig.frame(vec![Event::Key { key: Key::Enter, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE }]);
    rig.frame(vec![]);
    assert!(rig.side_clicked.is_empty(), "{:?}", rig.side_clicked.len());
    rig.settle();
    rig.ctx.memory_mut(|m| m.request_focus(super::hex::hex_id(rig.picker)));
    rig.frame(vec![]);
    rig.key(Key::Escape);
    assert_eq!(rig.side_clicked.drain(..).map(|c| c.role).collect::<Vec<_>>(), [ButtonRole::Cancel], "a later Esc is answered");
}

#[test]
fn a_pending_hex_focus_waits_for_the_picker_to_be_enabled() {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let numbers = LocaleNumbers::from_tag("en");
    let mut time = 0.0;
    let mut run = |enabled: bool, request: Option<Id>| -> Id {
        let input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(900.0, 900.0))), time: Some(time), ..Default::default() };
        time += 1.0 / 60.0;
        let mut id = Id::NULL;
        let output = ctx.run_ui(input, |ui| {
            if let Some(picker) = request {
                ColorPicker::focus_hex(ui.ctx(), picker);
            }
            let picked = ColorPicker::new("picker", "Fill colour", Srgba::rgb(0.2, 0.4, 0.6), &numbers)
                .layout(ColorPickerLayout::Compact)
                .enabled(enabled)
                .show(ui);
            id = picked.edit.response.id;
        });
        output.drop_without_applying_deltas();
        id
    };
    let picker = run(true, None);
    run(false, Some(picker));
    run(false, None);
    assert_ne!(ctx.memory(|m| m.focused()), Some(super::hex::hex_id(picker)), "disabled: no focus");
    run(true, None);
    run(true, None);
    assert_eq!(ctx.memory(|m| m.focused()), Some(super::hex::hex_id(picker)), "the request waited");
}
