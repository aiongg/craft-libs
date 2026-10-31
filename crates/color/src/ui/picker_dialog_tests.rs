//! Behaviour tests of the Color Picker dialog on a `DialogHost`, driven through AccessKit nodes
//! and pointer events.

use craft_cms::convert::CmykConverter;
use craft_settings::Remembered;
use craft_theme::{ModelValues, Srgba};
use craft_ui::dialog::{DialogHost, DialogResult, ReportDialog};
use craft_ui::theme::{Base, install};
use egui::accesskit::{Node, NodeId, Role, Toggled};
use egui::{Context, Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, vec2};

use super::{ColorPickerDialog, ColorPickerMemory, ColorPickerResult, DropperState};
use crate::{AreaMode, format_hex};

type Answer = DialogResult<ColorPickerResult>;

struct Rig {
    ctx: Context,
    host: DialogHost,
    time: f64,
    modifiers: Modifiers,
    reported: Modifiers,
    nodes: Vec<(NodeId, Node)>,
    focus: Option<NodeId>,
    answer: Option<Answer>,
}

impl Rig {
    fn new() -> Self {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        craft_host::Host::builder(craft_host::AppInfo { id: "test", name: "Test", version: "1.0" }).install(&ctx);
        ctx.enable_accesskit();
        Rig {
            ctx,
            host: DialogHost::new(),
            time: 0.0,
            modifiers: Modifiers::NONE,
            reported: Modifiers::NONE,
            nodes: Vec::new(),
            focus: None,
            answer: None,
        }
    }

    fn open(dialog: ColorPickerDialog) -> Self {
        let mut rig = Rig::new();
        rig.host.open(dialog.eyedropper(false));
        rig.frames(6);
        rig
    }

    fn frame(&mut self, mut events: Vec<Event>) {
        if self.modifiers != self.reported {
            events.insert(0, Event::ModifiersChanged(self.modifiers));
            self.reported = self.modifiers;
        }
        let input =
            RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1400.0, 900.0))), time: Some(self.time), events, ..Default::default() };
        self.time += 1.0 / 60.0;
        let host = &mut self.host;
        let output = self.ctx.run_ui(input, |ui| host.show(ui.ctx()));
        let update = output.platform_output.accesskit_update.clone();
        self.nodes = update.as_ref().map(|u| u.nodes.clone()).unwrap_or_default();
        self.focus = update.map(|u| u.focus);
        output.drop_without_applying_deltas();
        if let Some(answer) = self.host.take::<ColorPickerDialog>() {
            self.answer = Some(answer);
        }
    }

    fn frames(&mut self, count: usize) {
        for _ in 0..count {
            self.frame(Vec::new());
        }
    }

    fn node(&self, role: Role, name: &str) -> (NodeId, &Node) {
        self.nodes.iter().find(|(_, n)| n.role() == role && n.label() == Some(name)).map(|(id, n)| (*id, n)).unwrap_or_else(|| panic!("no {name}"))
    }

    fn centre(&self, role: Role, name: &str) -> Pos2 {
        let b = self.node(role, name).1.bounds().expect("bounds");
        Pos2::new(((b.x0 + b.x1) / 2.0) as f32, ((b.y0 + b.y1) / 2.0) as f32)
    }

    fn click(&mut self, role: Role, name: &str) {
        let at = self.centre(role, name);
        let modifiers = self.modifiers;
        let button = |pressed| Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers };
        self.frame(vec![Event::PointerMoved(at)]);
        self.frame(vec![button(true)]);
        self.frame(vec![button(false)]);
        self.frames(2);
    }

    fn dialog(&self) -> &ColorPickerDialog {
        self.host.get::<ColorPickerDialog>().expect("the dialog is open")
    }

    fn memory(&self) -> Option<ColorPickerMemory> {
        let host = craft_host::Host::try_of_ctx(&self.ctx).expect("host");
        Remembered::<ColorPickerMemory>::new(ColorPickerMemory::SCOPE).recall(&host)
    }

    fn remember(&self, memory: ColorPickerMemory) {
        let host = craft_host::Host::try_of_ctx(&self.ctx).expect("host");
        Remembered::<ColorPickerMemory>::new(ColorPickerMemory::SCOPE).store(&host, &memory);
    }
}

fn hex(color: Srgba) -> String {
    format_hex([color.r, color.g, color.b]).to_uppercase()
}

const BLUE: Srgba = Srgba { r: 0.2, g: 0.4, b: 0.6, a: 1.0 };

/// The hex field has the focus with its text selected when the dialog opens.
#[test]
fn opening_focuses_the_hex_field() {
    let rig = Rig::open(ColorPickerDialog::foreground(BLUE));
    let (hex_field, _) = rig.node(Role::TextInput, "Hex colour");
    assert_eq!(rig.focus, Some(hex_field));
}

/// Only Web Colors and the field's component are recalled on open and stored on OK, not on Cancel.
#[test]
fn mode_and_web_colors_are_remembered_on_ok_only() {
    let recalled = ColorPickerMemory { mode: AreaMode::Saturation, web_colors: true };
    let mut rig = Rig::new();
    rig.remember(recalled);
    rig.host.open(ColorPickerDialog::foreground(BLUE).eyedropper(false));
    rig.frames(6);
    assert_eq!(rig.node(Role::CheckBox, "Only Web Colors").1.toggled(), Some(Toggled::True));
    assert_eq!(rig.node(Role::RadioButton, "Saturation").1.toggled(), Some(Toggled::True));

    // Changes, then Cancel: the stored memory stays.
    rig.click(Role::CheckBox, "Only Web Colors");
    rig.click(Role::RadioButton, "Red");
    rig.click(Role::Button, "Cancel");
    assert!(matches!(rig.answer, Some(DialogResult::Cancel)));
    assert_eq!(rig.memory(), Some(recalled));

    // The same changes with OK are stored.
    rig.answer = None;
    rig.host.open(ColorPickerDialog::foreground(BLUE).eyedropper(false));
    rig.frames(6);
    rig.click(Role::CheckBox, "Only Web Colors");
    rig.click(Role::RadioButton, "Red");
    rig.click(Role::Button, "OK");
    assert!(matches!(rig.answer, Some(DialogResult::Ok(_))));
    assert_eq!(rig.memory(), Some(ColorPickerMemory { mode: AreaMode::Red, web_colors: false }));
}

/// With Only Web Colors on, the opening colour is snapped and OK without an edit returns it unchanged as the compare current.
#[test]
fn web_only_snaps_the_opening_colour_and_current_follows() {
    let mut rig = Rig::new();
    rig.remember(ColorPickerMemory { mode: AreaMode::Hue, web_colors: true });
    rig.host.open(ColorPickerDialog::foreground(Srgba::rgb(0.18, 0.42, 0.63)).eyedropper(false));
    rig.frames(6);
    assert_eq!(hex(rig.dialog().current().color), "#336699");
    let (_, current) = rig.node(Role::Button, "Restore current colour");
    assert_eq!(current.value(), Some("#336699"));
    rig.click(Role::Button, "OK");
    let Some(DialogResult::Ok(answer)) = rig.answer else { panic!("OK answers") };
    assert_eq!(hex(answer.color.color), "#336699");
}

/// Alt turns Cancel into Reset, which restores the opening colour and keeps the dialog open.
#[test]
fn alt_cancel_restores_the_opening_colour() {
    let mut rig = Rig::open(ColorPickerDialog::foreground(BLUE));
    let red = Srgba::rgb(1.0, 0.0, 0.0);
    DropperState::deliver(&rig.ctx, red);
    rig.frames(4);
    assert_eq!(hex(rig.dialog().current().color), "#FF0000");

    rig.modifiers = Modifiers::ALT;
    rig.frames(2);
    rig.click(Role::Button, "Reset");
    assert!(rig.answer.is_none() && rig.host.is_open::<ColorPickerDialog>());
    assert_eq!(hex(rig.dialog().current().color), hex(BLUE));
}

/// Add to Swatches keeps the dialog open and hands the colour to take_add_to_swatches once.
#[test]
fn add_to_swatches_keeps_the_dialog_open() {
    let mut rig = Rig::open(ColorPickerDialog::foreground(BLUE));
    rig.click(Role::Button, "Add to Swatches");
    assert!(rig.answer.is_none() && rig.host.is_open::<ColorPickerDialog>());
    let taken = rig.host.get_mut::<ColorPickerDialog>().and_then(ColorPickerDialog::take_add_to_swatches);
    assert_eq!(taken.map(|p| hex(p.color)), Some(hex(BLUE)));
    assert!(rig.host.get_mut::<ColorPickerDialog>().and_then(ColorPickerDialog::take_add_to_swatches).is_none());
}

/// A colour delivered by the canvas dropper while the dialog is on top becomes the new colour.
#[test]
fn a_canvas_sample_sets_the_new_colour_while_on_top() {
    let mut rig = Rig::open(ColorPickerDialog::foreground(BLUE));
    assert_eq!(DropperState::armed(&rig.ctx), Some(rig.dialog().id));
    DropperState::deliver(&rig.ctx, Srgba::rgb(1.0, 0.0, 0.0));
    rig.frames(4);
    assert_eq!(hex(rig.dialog().current().color), "#FF0000");
    assert_eq!(DropperState::armed(&rig.ctx), Some(rig.dialog().id), "the dialog stays armed after a sample");

    // Another dialog on top takes the canvas back.
    rig.host.open(ReportDialog::new("report", "Report", "Done", Vec::new()));
    rig.frames(4);
    assert_eq!(DropperState::armed(&rig.ctx), None);
}

/// Typed CMYK inks come back in the result's model values while they still give the colour.
#[test]
fn typed_cmyk_inks_come_back_in_the_result() {
    let converter = CmykConverter::coated();
    let inks = ModelValues::new("cmyk", &[10.0, 20.0, 30.0, 40.0]);
    let color = converter.to_srgb([0.1, 0.2, 0.3, 0.4]);
    let kept = ColorPickerDialog::foreground(Srgba::rgb(color[0], color[1], color[2])).gamut(converter).kept(inks);
    let mut rig = Rig::open(kept);
    assert_eq!(rig.node(Role::SpinButton, "Cyan").1.numeric_value(), Some(10.0), "the fields show the inks the colour was picked as");

    let at = rig.centre(Role::SpinButton, "Cyan");
    let press = |pressed| Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
    rig.frame(vec![Event::PointerMoved(at)]);
    rig.frame(vec![press(true)]);
    rig.frame(vec![press(false)]);
    rig.frame(vec![Event::Key { key: Key::A, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::COMMAND }]);
    rig.frame(vec![Event::Text("50".into())]);
    rig.frame(vec![Event::Key { key: Key::Tab, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE }]);
    rig.frames(3);
    rig.click(Role::Button, "OK");
    let Some(DialogResult::Ok(answer)) = rig.answer else { panic!("OK answers") };
    let values = answer.color.model.expect("the typed inks");
    assert_eq!(values.model, "cmyk");
    assert_eq!(values.values[0], 50.0);
}

/// OK answers the key the dialog was opened with.
#[test]
fn ok_answers_the_opening_key() {
    for (dialog, key) in
        [(ColorPickerDialog::background(BLUE), "background"), (ColorPickerDialog::new("vine", "Color Picker (Vine Color)", BLUE), "vine")]
    {
        let mut rig = Rig::open(dialog);
        assert_eq!(rig.dialog().key(), key);
        rig.click(Role::Button, "OK");
        let Some(DialogResult::Ok(answer)) = rig.answer else { panic!("OK answers") };
        assert_eq!((answer.key.as_str(), hex(answer.color.color)), (key, hex(BLUE)));
    }
}
