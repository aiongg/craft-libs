//! The public hex field: digits per channel, the fixed `#`, and the character limit.

use craft_theme::Rgba;
use egui::accesskit::Role;
use egui::{Context, Event, Id, Key, Modifiers, Pos2, RawInput, Rect, vec2};

use super::HexField;
use craft_ui::theme::{Base, install};

struct Rig {
    ctx: Context,
    time: f64,
    color: Rgba,
    digits: usize,
    alpha: bool,
    id: Id,
    text: String,
}

impl Rig {
    fn new(color: Rgba, digits: usize) -> Self {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        ctx.enable_accesskit();
        let mut rig = Self { ctx, time: 0.0, color, digits, alpha: false, id: Id::NULL, text: String::new() };
        rig.frame(vec![]);
        rig
    }

    fn frame(&mut self, events: Vec<Event>) {
        let input =
            RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(400.0, 300.0))), time: Some(self.time), events, ..Default::default() };
        self.time += 1.0 / 60.0;
        let ctx = self.ctx.clone();
        let output = ctx.run_ui(input, |ui| {
            let edit = HexField::new("hex", "Hex colour", self.color).digits(self.digits).alpha(self.alpha).show(ui);
            self.id = edit.response.id;
            if let Some(c) = edit.event.as_ref().and_then(|e| e.value()) {
                self.color = *c;
            }
        });
        let nodes = output.platform_output.accesskit_update.clone().map(|u| u.nodes).unwrap_or_default();
        self.text = nodes
            .iter()
            .find(|(_, n)| n.role() == Role::TextInput && n.label() == Some("Hex colour"))
            .and_then(|(_, n)| n.value().map(str::to_owned))
            .unwrap_or_default();
        output.drop_without_applying_deltas();
    }

    fn focus_and_type(&mut self, text: &str) {
        self.ctx.memory_mut(|m| m.request_focus(self.id));
        self.frame(vec![]);
        let select_all = Event::Key { key: Key::A, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::COMMAND };
        self.frame(vec![select_all]);
        self.frame(vec![Event::Text(text.to_owned())]);
        self.frame(vec![]);
    }
}

#[test]
fn a_hex_field_with_one_digit_per_channel_doubles_each_digit() {
    let mut rig = Rig::new(Rgba::rgb(0x33, 0x66, 0x99), 1);
    assert_eq!(rig.text, "369", "a web colour shows as three digits");
    rig.focus_and_type("fc0");
    assert_eq!(rig.color, Rgba::rgb(0xff, 0xcc, 0x00), "each digit is doubled on read");
    rig.focus_and_type("ffcc00");
    assert_eq!(rig.color, Rgba::rgb(0xff, 0xcc, 0x00), "six digits don't fit in a three-digit field");

    let mut two = Rig::new(Rgba::rgb(0x33, 0x66, 0x99), 2);
    assert_eq!(two.text, "336699");
    two.focus_and_type("fc0");
    assert_eq!(two.color, Rgba::rgb(0x33, 0x66, 0x99), "three digits are a draft in a six-digit field");
    two.focus_and_type("a1b2c3");
    assert_eq!(two.color, Rgba::rgb(0xa1, 0xb2, 0xc3));
}

#[test]
fn a_hex_field_with_alpha_writes_the_digits_it_reads() {
    for (digits, color, text, typed) in
        [(2, Rgba([0x33, 0x66, 0x99, 0xcc]), "336699CC", "11223344"), (1, Rgba([0x33, 0x66, 0x99, 0xcc]), "369C", "fc08")]
    {
        let mut rig = Rig::new(color, digits);
        rig.alpha = true;
        rig.frame(vec![]);
        assert_eq!(rig.text, text, "the alpha shows");
        rig.focus_and_type(typed);
        let read = rig.color;
        assert_ne!(read, color, "{typed} is read");
        rig.focus_and_type(&rig.text.clone());
        assert_eq!(rig.color, read, "what the field shows reads back as the same colour");
    }
    let mut rig = Rig::new(Rgba([0x33, 0x66, 0x99, 0xcc]), 2);
    rig.alpha = true;
    rig.focus_and_type("112233");
    assert_eq!(rig.color, Rgba([0x11, 0x22, 0x33, 0xcc]), "six digits keep the alpha");
}
