//! Number fields that read arithmetic.

use egui::{Context, Event, Key, Modifiers, Pos2, RawInput, Rect, Ui, vec2};

use super::NumberField;
use crate::edit::EditEvent;
use crate::localized::{LocaleNumbers, focus_first_field};
use crate::theme::{Base, install};

/// A host with one expression-reading number field that applies every value it is sent.
struct Probe {
    ctx: Context,
    time: f64,
    value: f64,
    numbers: LocaleNumbers,
    on: bool,
}

impl Probe {
    fn new(value: f64, tag: &str) -> Self {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        let mut probe = Self { ctx, time: 0.0, value, numbers: LocaleNumbers::from_tag(tag), on: true };
        probe.frame(vec![]);
        probe.frame(vec![]);
        probe
    }

    fn frame(&mut self, events: Vec<Event>) {
        let input =
            RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))), time: Some(self.time), events, ..Default::default() };
        self.time += 1.0 / 60.0;
        let ctx = self.ctx.clone();
        let output = ctx.run_ui(input, |ui| self.field(ui));
        output.drop_without_applying_deltas();
    }

    fn field(&mut self, ui: &mut Ui) {
        let numbers = self.numbers.clone();
        let edit = NumberField::new("probe", "Width", self.value, &numbers).decimals(0..=2).expressions(self.on).show(ui);
        if let Some(EditEvent::Commit { value, .. } | EditEvent::Update { value, .. }) = edit.event {
            self.value = value;
        }
    }

    /// Types `text` into the focused field and presses Enter.
    fn enter(&mut self, text: &str) {
        focus_first_field(&self.ctx, true);
        self.frame(vec![]);
        self.frame(vec![]);
        self.frame(vec![Event::Text(text.to_owned())]);
        let key = |pressed| Event::Key { key: Key::Enter, physical_key: None, pressed, repeat: false, modifiers: Modifiers::NONE };
        self.frame(vec![key(true), key(false)]);
        self.frame(vec![]);
    }
}

// With expressions(true), typing "(1920-40)/2" and Enter commits 940.
#[test]
fn typed_arithmetic_commits_its_result() {
    let mut probe = Probe::new(5.0, "en");
    probe.enter("(1920-40)/2");
    assert_eq!(probe.value, 940.0);

    let mut off = Probe::new(5.0, "en");
    off.on = false;
    off.enter("(1920-40)/2");
    assert_eq!(off.value, 5.0, "without expressions the text is refused");
}

// With the value 150, typing "*2" commits 300.
#[test]
fn a_relative_entry_scales_the_fields_value() {
    let mut probe = Probe::new(150.0, "en");
    probe.enter("*2");
    assert_eq!(probe.value, 300.0);
    probe.enter("/4");
    assert_eq!(probe.value, 75.0);
    probe.enter("-10");
    assert_eq!(probe.value, -10.0, "a leading minus is the sign");
}

// In a comma-decimal region "1,5*2" commits 3.
#[test]
fn the_regions_decimal_mark_is_read_in_arithmetic() {
    let mut probe = Probe::new(5.0, "de");
    probe.enter("1,5*2");
    assert_eq!(probe.value, 3.0);
}
