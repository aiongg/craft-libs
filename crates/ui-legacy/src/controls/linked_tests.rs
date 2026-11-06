//! `LinkedFields` link rules.
#![allow(clippy::expect_used, reason = "tests")]

use egui::accesskit::Role;
use egui::{Context, Event, Modifiers, PointerButton, Pos2, RawInput, Rect, pos2, vec2};

use super::NumberField;
use super::linked::{as_reported, reported_event};
use super::{LinkMode, LinkedFields, LinkedLayout, LinkedSlot, follow_equal, follow_ratio};
use crate::edit::{CancelReason, EditEvent, GestureId};
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
    /// `None`: the fields in a grid with X and Y cells (`show_cells`); else the layout.
    layout: Option<LinkedLayout>,
    captions: [&'static str; 2],
    names_only: bool,
    /// Each AccessKit node's role, name and bounds in the last frame.
    nodes: Vec<(Role, String, Rect)>,
}

impl Rig {
    fn new(values: [f64; 2], linked: bool, mode: LinkMode) -> Self {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        ctx.enable_accesskit();
        let mut rig = Self {
            ctx,
            time: 0.0,
            values,
            linked,
            mode,
            numbers: LocaleNumbers::from_tag("en"),
            events: Vec::new(),
            layout: Some(LinkedLayout::Row),
            captions: ["W:", "H:"],
            names_only: false,
            nodes: Vec::new(),
        };
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
            let fields = LinkedFields::new("size", self.captions, self.values, &mut self.linked, self.mode, &numbers);
            let fields = if self.names_only { fields.captions_as_names() } else { fields };
            let out = match self.layout {
                Some(layout) => fields.layout(layout).show(ui),
                None => {
                    let cell = |ui: &mut egui::Ui, name: &str| {
                        let _ = NumberField::new(name, name, 0.0, &numbers).show(ui);
                    };
                    egui::Grid::new("clone")
                        .show(ui, |ui| {
                            fields.show_cells(ui, |ui, slot, add| match slot {
                                LinkedSlot::Field(0) => {
                                    cell(ui, "X:");
                                    add(ui);
                                }
                                LinkedSlot::Link => {
                                    add(ui);
                                    ui.end_row();
                                    cell(ui, "Y:");
                                }
                                LinkedSlot::Field(_) => add(ui),
                            })
                        })
                        .inner
                }
            };
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
                        n.label().or_else(|| n.value()).unwrap_or_default().to_owned(),
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

fn center_of(rig: &Rig, role: Role, name: &str) -> Pos2 {
    rig.center(role, name)
}

// In a harness: `Columns` puts W, the link and H side by side on one line; `show_cells` puts X, W and the link on one grid row and Y, H under X and W.
#[test]
fn the_layouts_place_the_fields_as_the_apps_do() {
    let mut rig = Rig::new([10.0, 20.0], true, LinkMode::Equal);
    rig.layout = Some(LinkedLayout::Columns);
    rig.frame(vec![]);
    rig.frame(vec![]);
    let (w, h) = (center_of(&rig, Role::SpinButton, "W:"), center_of(&rig, Role::SpinButton, "H:"));
    let link = center_of(&rig, Role::Button, "Make all settings the same");
    assert!(w.x < link.x && link.x < h.x, "{w:?} {link:?} {h:?}");
    assert!((w.y - h.y).abs() < 1.0 && (link.y - w.y).abs() < 1.0, "one line: {w:?} {link:?} {h:?}");

    rig.layout = None;
    rig.frame(vec![]);
    rig.frame(vec![]);
    let at = |rig: &Rig, name: &str| center_of(rig, Role::SpinButton, name);
    let (x, w, y, h) = (at(&rig, "X:"), at(&rig, "W:"), at(&rig, "Y:"), at(&rig, "H:"));
    let link = center_of(&rig, Role::Button, "Make all settings the same");
    assert!(x.x < w.x && w.x < link.x && (x.y - w.y).abs() < 1.0 && (link.y - w.y).abs() < 1.0, "first row: {x:?} {w:?} {link:?}");
    assert!(y.y > x.y && (y.y - h.y).abs() < 1.0 && (y.x - x.x).abs() < 1.0 && (h.x - w.x).abs() < 1.0, "second row: {y:?} {h:?}");
}

// In a harness: a column with the corner names as captions lays out the same in every pass (the captions sit
// before the boxes, so the text keeps its room) and the boxes line up.
#[test]
fn a_column_with_long_captions_keeps_its_layout() {
    let mut rig = Rig::new([10.0, 20.0], false, LinkMode::Equal);
    rig.captions = ["Top Left", "Bottom Right"];
    rig.layout = Some(LinkedLayout::Column);
    let boxes = |rig: &Rig| {
        let at = |name| rig.nodes.iter().find(|(r, n, _)| *r == Role::SpinButton && n == name).map(|node| node.2).expect("the field is in the tree");
        [at("Top Left"), at("Bottom Right")]
    };
    rig.frame(vec![]);
    let first = boxes(&rig);
    for _ in 0..4 {
        rig.frame(vec![]);
        assert_eq!(boxes(&rig), first, "the layout is stable across passes");
    }
    assert_eq!(first[0].left(), first[1].left(), "the boxes line up");
    assert_eq!(first[0].width(), first[1].width());
}

// In a harness: `captions_as_names` shows no caption text, and the field keeps its name.
#[test]
fn captions_as_names_leaves_the_captions_out() {
    let mut rig = Rig::new([10.0, 20.0], false, LinkMode::Equal);
    rig.captions = ["Top Left", "Bottom Right"];
    rig.layout = Some(LinkedLayout::Column);
    rig.frame(vec![]);
    let outside = rig.nodes.iter().filter(|(r, n, _)| *r == Role::Label && n.contains("Left")).count();
    assert_eq!(outside, 1, "the caption is a label of its own in a column");
    rig.names_only = true;
    rig.frame(vec![]);
    rig.frame(vec![]);
    assert!(rig.nodes.iter().all(|(r, n, _)| *r != Role::Label || !n.contains("Left")), "{:?}", rig.nodes);
    assert!(rig.nodes.iter().any(|(r, n, _)| *r == Role::SpinButton && n == "Top Left"), "the name stays");
}

// In a harness: digits typed into W of 400×300 keep the starting ratio (W 40 at H 30), and Esc ends the typing with one Cancel.
#[test]
fn typing_keeps_the_ratio_and_escape_cancels() {
    let mut rig = Rig::new([400.0, 300.0], true, LinkMode::Ratio);
    focus_first_field(&rig.ctx, true);
    rig.frame(vec![]);
    rig.frame(vec![]);
    for digit in ["4", "0"] {
        rig.frame(vec![Event::Text(digit.to_owned())]);
    }
    assert_eq!(rig.values, [40.0, 30.0]);
    rig.frame(vec![Event::Key { key: egui::Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE }]);
    let cancels = rig.events.iter().filter(|e| matches!(e, EditEvent::Cancel { .. })).count();
    assert_eq!(cancels, 1, "{:?}", rig.events);
    assert!(matches!(rig.events.last(), Some(EditEvent::Cancel { .. })), "{:?}", rig.events);
}

// `as_reported` hands the host a `Begin` before an `Update` or `Commit` of a gesture whose own `Begin` another field's `Commit` displaced.
#[test]
fn a_gesture_the_host_never_saw_begin_is_reported_whole() {
    let (a, b) = (GestureId::next(), GestureId::next());
    let update = EditEvent::update(b, [1.0], Modifiers::NONE);
    let (event, reported) = as_reported(update.clone(), Some(a));
    assert!(matches!(event, Some(EditEvent::Begin { id, value: Some(_) }) if id == b), "{event:?}");
    assert_eq!(reported, Some(b));
    let (event, reported) = as_reported(update.clone(), Some(b));
    assert_eq!((event, reported), (Some(update), Some(b)), "a begun gesture goes on");
    let (event, reported) = as_reported(EditEvent::commit(b, [2.0], true, Modifiers::NONE), Some(a));
    assert!(matches!(event, Some(EditEvent::Set { value: [2.0], .. })), "{event:?}");
    assert_eq!(reported, None);
    let (event, _) = as_reported(EditEvent::<[f64; 1]>::cancel(b, CancelReason::Escape), Some(a));
    assert!(event.is_none());

    let begin = (1, EditEvent::begin(b));
    let commit = (0, EditEvent::commit(a, 5.0, true, Modifiers::NONE));
    assert!(matches!(reported_event(&[begin.clone(), commit.clone()]), Some((0, EditEvent::Commit { .. }))), "an ending wins over a beginning");
    assert!(matches!(reported_event(&[begin]), Some((1, EditEvent::Begin { .. }))));
}
