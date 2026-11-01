//! `LinkedFields` link rules.
#![allow(clippy::expect_used, reason = "tests")]

use egui::accesskit::Role;
use egui::{Context, Event, Modifiers, PointerButton, Pos2, RawInput, Rect, pos2, vec2};

use super::{LinkMode, LinkedFields, follow_equal, follow_ratio};
use crate::edit::EditEvent;
use crate::localized::{LocaleNumbers, focus_first_field};
use crate::theme::{Base, install};

const UNBOUNDED: std::ops::RangeInclusive<f64> = f64::NEG_INFINITY..=f64::INFINITY;

// `follow_ratio`: 100×50 with W 200 gives 200×100; the factor is lowered so no value leaves the range; an old zero scales nothing.
#[test]
fn a_ratio_link_scales_the_others_within_the_range() {
    let free = [UNBOUNDED, UNBOUNDED];
    assert_eq!(follow_ratio([100.0, 50.0], 0, 200.0, &free), [200.0, 100.0]);
    assert_eq!(follow_ratio([100.0, 50.0], 1, 25.0, &free), [50.0, 25.0]);
    let capped = [0.0..=150.0, 0.0..=80.0];
    assert_eq!(follow_ratio([100.0, 50.0], 0, 200.0, &capped), [150.0, 75.0], "W reaches its limit and H keeps the ratio");
    assert_eq!(follow_ratio([100.0, 50.0], 1, 100.0, &capped), [150.0, 75.0], "H would pass W's limit");
    assert_eq!(follow_ratio([100.0, 0.0], 1, 10.0, &free), [100.0, 10.0], "an old zero scales nothing");
    assert_eq!(follow_ratio([500.0, 400.0], 0, 100.0, &free), [100.0, 80.0], "no rounding noise");
}

// `follow_equal` sets every value to the new one.
#[test]
fn an_equal_link_copies_the_value() {
    assert_eq!(follow_equal([1.0, 2.0, 3.0, 4.0], 2, 9.0), [9.0; 4]);
}

/// A host with W and H fields and the events it was sent.
struct Rig {
    ctx: Context,
    time: f64,
    values: [f64; 2],
    linked: bool,
    mode: LinkMode,
    numbers: LocaleNumbers,
    events: Vec<EditEvent<[f64; 2]>>,
    /// Each AccessKit node's role, name and bounds in the last frame.
    nodes: Vec<(Role, String, Rect)>,
}

impl Rig {
    fn new(values: [f64; 2], linked: bool, mode: LinkMode) -> Self {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        ctx.enable_accesskit();
        let mut rig = Self { ctx, time: 0.0, values, linked, mode, numbers: LocaleNumbers::from_tag("en"), events: Vec::new(), nodes: Vec::new() };
        rig.frame(vec![]);
        rig.frame(vec![]);
        rig
    }

    fn frame(&mut self, events: Vec<Event>) {
        let input =
            RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))), time: Some(self.time), events, ..Default::default() };
        self.time += 1.0 / 60.0;
        let ctx = self.ctx.clone();
        let numbers = self.numbers.clone();
        let output = ctx.run_ui(input, |ui| {
            let out = LinkedFields::new("size", ["W:", "H:"], self.values, &mut self.linked, self.mode, &numbers).show(ui);
            if let Some(event) = out.event {
                self.events.push(event.clone());
                if let Some(value) = event.value() {
                    self.values = *value;
                }
            }
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

    fn press(&mut self, at: Pos2) {
        self.frame(vec![Event::PointerMoved(at)]);
        self.frame(vec![Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE }]);
    }

    fn release(&mut self, at: Pos2) {
        self.frame(vec![Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE }]);
        self.frame(vec![]);
    }

    fn click(&mut self, at: Pos2) {
        self.press(at);
        self.release(at);
    }
}

// In a harness: turning the link on in `Equal` reports one `Set` with every value equal to the first.
#[test]
fn linking_equal_fields_copies_the_first() {
    let mut rig = Rig::new([10.0, 20.0], false, LinkMode::Equal);
    let link = rig.center(Role::Button, "Make all settings the same");
    rig.click(link);
    assert!(rig.linked);
    assert_eq!(rig.events.len(), 1);
    assert!(matches!(&rig.events[0], EditEvent::Set { value, .. } if *value == [10.0, 10.0]), "{:?}", rig.events);
}

// In a harness: a scrub of W with the ratio link reports Begin, Updates and one Commit, each with both values;
// typing "100" digit by digit into W of 500×400 ends at 100×80 (the starting ratio, not one drifted per digit).
#[test]
fn a_linked_scrub_moves_both_fields_in_one_gesture() {
    let ratio = |event: &EditEvent<[f64; 2]>| event.value().map(|[w, h]| (w * 0.8 - h).abs() < 1e-6);

    let mut rig = Rig::new([500.0, 400.0], true, LinkMode::Ratio);
    let w = rig.center(Role::SpinButton, "W:");
    rig.press(w);
    for step in 1..=3 {
        rig.frame(vec![Event::PointerMoved(w + vec2(10.0 * step as f32, 0.0))]);
    }
    rig.release(w + vec2(30.0, 0.0));
    assert!(matches!(rig.events.first(), Some(EditEvent::Begin { .. })), "{:?}", rig.events);
    assert!(matches!(rig.events.last(), Some(EditEvent::Commit { .. })), "{:?}", rig.events);
    assert!(rig.events.iter().any(|e| matches!(e, EditEvent::Update { .. })));
    assert!(rig.events.iter().all(|e| ratio(e) != Some(false)), "{:?}", rig.events);
    assert!(rig.values[0] > 500.0 && (rig.values[0] * 0.8 - rig.values[1]).abs() < 1e-6, "{:?}", rig.values);

    let mut rig = Rig::new([500.0, 400.0], true, LinkMode::Ratio);
    focus_first_field(&rig.ctx, true);
    rig.frame(vec![]);
    rig.frame(vec![]);
    for digit in ["1", "0", "0"] {
        rig.frame(vec![Event::Text(digit.to_owned())]);
    }
    assert_eq!(rig.values, [100.0, 80.0]);
    rig.frame(vec![Event::Key { key: egui::Key::Enter, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE }]);
    assert!(matches!(rig.events.last(), Some(EditEvent::Commit { value: [100.0, 80.0], .. })), "{:?}", rig.events);
}
