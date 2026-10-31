//! The hex pair field and the colour readout in a headless egui context.

use craft_theme::Rgba;
use egui::accesskit::{Node, NodeId};
use egui::{Context, Event, Id, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, pos2, vec2};

use super::{ColorReadout, HexPairField};
use craft_ui::edit::EditEvent;
use craft_ui::localized::LocaleNumbers;
use craft_ui::theme::{Base, install};

#[derive(Clone, Copy)]
enum Subject {
    Pair,
    Readout,
}

struct Rig {
    ctx: Context,
    numbers: LocaleNumbers,
    time: f64,
    subject: Subject,
    pair: u8,
    color: Rgba,
    pair_id: Id,
    pair_events: Vec<EditEvent<u8>>,
    color_events: Vec<EditEvent<Rgba>>,
    nodes: Vec<(NodeId, Node)>,
    /// The width the readout is shown in.
    width: f32,
}

impl Rig {
    fn new(subject: Subject) -> Self {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        ctx.enable_accesskit();
        let mut rig = Self {
            ctx,
            numbers: LocaleNumbers::from_tag("en"),
            time: 0.0,
            subject,
            pair: 0x33,
            color: Rgba::rgb(0x33, 0x66, 0x99),
            pair_id: Id::NULL,
            pair_events: Vec::new(),
            color_events: Vec::new(),
            nodes: Vec::new(),
            width: 600.0,
        };
        rig.settle();
        rig
    }

    fn frame(&mut self, events: Vec<Event>) {
        let input =
            RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(600.0, 200.0))), time: Some(self.time), events, ..Default::default() };
        self.time += 1.0 / 60.0;
        let ctx = self.ctx.clone();
        let (mut pair_event, mut color_event) = (None, None);
        let output = ctx.run_ui(input, |ui| match self.subject {
            Subject::Pair => {
                self.pair_id = ui.make_persistent_id("pair");
                pair_event = HexPairField::new("pair", "Red", self.pair).show(ui).event;
            }
            Subject::Readout => {
                ui.set_max_width(self.width);
                color_event = ColorReadout::new("readout", "Hex colour", self.color, &self.numbers).show(ui).event;
            }
        });
        if let Some(event) = pair_event {
            if let Some(v) = event.value() {
                self.pair = *v;
            }
            self.pair_events.push(event);
        }
        if let Some(event) = color_event {
            if let Some(v) = event.value() {
                self.color = *v;
            }
            self.color_events.push(event);
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
        self.frame(vec![Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }]);
        self.frame(vec![]);
    }

    fn text(&mut self, text: &str) {
        self.frame(vec![Event::Text(text.to_owned())]);
        self.settle();
    }

    fn rect(&self, name: &str) -> Rect {
        let node = self.nodes.iter().map(|(_, n)| n).find(|n| n.label() == Some(name)).unwrap_or_else(|| panic!("no {name}"));
        let b = node.bounds().expect("bounds");
        Rect::from_min_max(pos2(b.x0 as f32, b.y0 as f32), pos2(b.x1 as f32, b.y1 as f32))
    }

    fn click(&mut self, pos: Pos2) {
        let button = |pressed| Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
        self.frame(vec![Event::PointerMoved(pos), button(true)]);
        self.frame(vec![button(false)]);
        self.settle();
    }
}

#[test]
fn a_hex_pair_applies_two_digits_and_marks_anything_else_invalid() {
    let mut rig = Rig::new(Subject::Pair);
    assert!(rig.rect("Red").width() > 0.0);
    let field = rig.rect("Red");
    rig.click(field.center());
    rig.key(Key::A, Modifiers::COMMAND);
    rig.text("1");
    assert!(rig.pair_events.iter().all(|e| e.value().is_none()), "one digit is not a value yet: {:?}", rig.pair_events);
    rig.text("f");
    assert_eq!(rig.pair, 0x1f);
    rig.key(Key::A, Modifiers::COMMAND);
    rig.text("+f");
    assert_eq!(rig.pair, 0x1f, "a sign is not a digit");
    let codec = super::hex::PairCodec { reason: String::new() };
    assert!(craft_ui::behavior::Codec::parse(&codec, "1G", &0).is_err());
}

#[test]
fn a_typed_channel_reports_the_whole_colour() {
    let mut rig = Rig::new(Subject::Readout);
    let red = rig.rect("Red");
    rig.click(red.center());
    rig.key(Key::A, Modifiers::COMMAND);
    rig.text("200");
    rig.key(Key::Tab, Modifiers::NONE);
    assert_eq!(rig.color, Rgba::rgb(200, 0x66, 0x99));

    let hex = rig.rect("Hex colour");
    rig.click(hex.center());
    rig.key(Key::A, Modifiers::COMMAND);
    rig.text("102030");
    assert_eq!(rig.color, Rgba::rgb(0x10, 0x20, 0x30));
}

/// PhotoCraft's Color panel is 300 points wide by default (290 in the pro themes) inside an
/// 8 point body margin: 274 points for the readout.
const PANEL_ROW_WIDTH: f32 = 274.0;

fn decoration(rig: &Rig, text: &str) -> Rect {
    let node = rig.nodes.iter().map(|(_, n)| n).find(|n| n.value() == Some(text) || n.label() == Some(text)).unwrap_or_else(|| panic!("no {text}"));
    let b = node.bounds().expect("bounds");
    Rect::from_min_max(pos2(b.x0 as f32, b.y0 as f32), pos2(b.x1 as f32, b.y1 as f32))
}

#[test]
fn the_readout_is_one_row_with_photocrafts_field_widths_in_the_default_panel() {
    let mut rig = Rig::new(Subject::Readout);
    rig.width = PANEL_ROW_WIDTH;
    rig.settle();
    let hex = rig.rect("Hex colour");
    assert_eq!(hex.width(), 64.0);
    let channels = ["Red", "Green", "Blue"].map(|name| rig.rect(name));
    for channel in channels {
        assert_eq!(channel.width(), 38.0);
        assert!((channel.center().y - hex.center().y).abs() < 1.0, "{channel:?} is on another row than {hex:?}");
    }
    assert!(channels.windows(2).all(|w| w[0].right() < w[1].left()));
    let hash = decoration(&rig, "#");
    assert!(hash.right() <= hex.left(), "the # is outside the field: {hash:?} against {hex:?}");
    assert!(channels[2].right() - hash.left() <= PANEL_ROW_WIDTH, "the row fits the panel");
}
