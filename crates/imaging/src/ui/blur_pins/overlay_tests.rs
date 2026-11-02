//! Tests of the pins' overlay and panel (task WS-BP2).

use egui::accesskit::Role;
use egui::{Context, Event, Id, Key, Modifiers, PointerButton, Pos2, Rect, Sense, TextEdit, WidgetInfo, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use craft_ui::canvas::CanvasView;
use craft_ui::canvas::kurbo::Point;
use craft_ui::edit::EditEvent;
use craft_ui::theme::{Base, install};

use super::{BlurKind, BlurPin, BlurPins, BlurPinsOverlay, BlurPinsPanel};

const BOUNDS: [f64; 4] = [0.0, 0.0, 400.0, 300.0];

struct Host {
    pins: BlurPins,
    rect: Rect,
    events: Vec<EditEvent<BlurPins>>,
    claimed: bool,
    text: String,
    text_id: Id,
    focus_text: bool,
}

fn pins_of(kind: BlurKind, amounts: &[f64]) -> BlurPins {
    let pin = |i: usize, amount: f64| BlurPin { x: 0.25 + 0.5 * i as f64 / amounts.len() as f64, y: 0.5, amount, iris: None, tilt: None };
    let mut pins = BlurPins { kind, bounds: BOUNDS, pins: amounts.iter().enumerate().map(|(i, &a)| pin(i, a)).collect(), selected: Some(0) };
    pins.clean();
    pins
}

fn overlay(pins: BlurPins) -> Harness<'static, Host> {
    let host = Host {
        pins,
        rect: Rect::NOTHING,
        events: Vec::new(),
        claimed: false,
        text: String::new(),
        text_id: Id::unique("blur-pins-test-text"),
        focus_text: false,
    };
    let mut h = Harness::<Host>::builder().with_size([480.0, 400.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, host: &mut Host| {
            let field = ui.add(TextEdit::singleline(&mut host.text).id(host.text_id));
            field.widget_info(|| WidgetInfo::labeled(Role::TextInput, true, "Note"));
            if std::mem::take(&mut host.focus_text) {
                ui.memory_mut(|m| m.request_focus(host.text_id));
            }
            let (rect, canvas) = ui.allocate_exact_size(vec2(400.0, 300.0), Sense::click_and_drag() - Sense::FOCUSABLE);
            host.rect = rect;
            let view = CanvasView::new(rect, Point::new(200.0, 150.0), 1.0);
            let out = BlurPinsOverlay::new("pins", &mut host.pins).show(ui, &canvas, &view);
            host.claimed = out.claimed;
            host.events.extend(out.edit);
        },
        host,
    );
    h.step();
    h
}

fn at(h: &Harness<'_, Host>, x: f64, y: f64) -> Pos2 {
    h.state().rect.min + vec2(x as f32, y as f32)
}

fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

fn press(h: &mut Harness<'_, Host>, pos: Pos2) {
    h.event(Event::PointerMoved(pos));
    h.event(button(pos, true));
    h.run();
}

fn move_to(h: &mut Harness<'_, Host>, pos: Pos2) {
    h.event(Event::PointerMoved(pos));
    h.run();
}

fn release(h: &mut Harness<'_, Host>, pos: Pos2) {
    h.event(Event::PointerMoved(pos));
    h.event(button(pos, false));
    h.run();
}

fn key(h: &mut Harness<'_, Host>, key: Key, modifiers: Modifiers) {
    for pressed in [true, false] {
        h.event(Event::Key { key, physical_key: None, pressed, repeat: false, modifiers });
    }
    h.run();
}

fn kinds(h: &Harness<'_, Host>) -> Vec<&'static str> {
    h.state()
        .events
        .iter()
        .map(|e| match e {
            EditEvent::Begin { .. } => "begin",
            EditEvent::Update { .. } => "update",
            EditEvent::Commit { .. } => "commit",
            EditEvent::Cancel { .. } => "cancel",
            EditEvent::Set { .. } => "set",
            _ => "other",
        })
        .collect()
}

// A click adds a pin (Set), a drag reports Begin, Update, Commit, Delete removes the selected pin
// unless a text field has focus.
#[test]
fn blur_pins_overlay_reports_edit_events_and_delete_removes() {
    let mut h = overlay(pins_of(BlurKind::Field, &[15.0]));

    let spot = at(&h, 50.0, 50.0);
    press(&mut h, spot);
    assert!(h.state().claimed, "the overlay took the press");
    release(&mut h, spot);
    assert_eq!(kinds(&h), ["set"], "a click on nothing adds a pin");
    assert_eq!(h.state().pins.pins.len(), 2);
    assert_eq!(h.state().pins.selected, Some(1));

    h.state_mut().events.clear();
    let target = at(&h, 120.0, 80.0);
    press(&mut h, spot);
    move_to(&mut h, target);
    release(&mut h, target);
    let events = kinds(&h);
    assert_eq!(events.first(), Some(&"begin"), "{events:?}");
    assert_eq!(events.last(), Some(&"commit"), "{events:?}");
    let pin = h.state().pins.pins[1];
    assert!((pin.x - 120.0 / 400.0).abs() < 1e-6 && (pin.y - 80.0 / 300.0).abs() < 1e-6, "{pin:?}");

    h.state_mut().focus_text = true;
    h.run();
    h.state_mut().events.clear();
    key(&mut h, Key::Delete, Modifiers::NONE);
    assert_eq!(h.state().pins.pins.len(), 2, "a text field has focus: Delete is its");
    assert!(h.state().events.is_empty());

    let off = at(&h, 300.0, 250.0);
    press(&mut h, off);
    release(&mut h, off);
    assert_eq!(h.state().pins.pins.len(), 3);
    h.state_mut().events.clear();
    key(&mut h, Key::Delete, Modifiers::NONE);
    assert_eq!(kinds(&h), ["set"]);
    assert_eq!(h.state().pins.pins.len(), 2);
    assert_eq!(h.state().pins.selected, Some(1), "the pin before the removed one is selected");
}

// Page picks a pin as a keyboard part, arrows move it, and the part is named "Pin 2, Blur 15 px".
#[test]
fn blur_pins_are_keyboard_parts() {
    let mut h = overlay(pins_of(BlurKind::Field, &[30.0, 15.0]));
    assert!(h.query_by_label("Pin 2, Blur 15 px").is_some());
    assert!(h.query_by_label("Pin 1, Blur 30 px").is_some());

    key(&mut h, Key::Tab, Modifiers::NONE);
    key(&mut h, Key::Tab, Modifiers::NONE);
    key(&mut h, Key::PageDown, Modifiers::NONE);
    assert_eq!(h.state().pins.selected, Some(1), "Page Down picks and selects the next pin");

    let x = h.state().pins.pins[1].x;
    key(&mut h, Key::ArrowRight, Modifiers::NONE);
    assert!((h.state().pins.pins[1].x - x - 1.0 / 400.0).abs() < 1e-9, "one document pixel");
    key(&mut h, Key::ArrowRight, Modifiers::SHIFT);
    assert!((h.state().pins.pins[1].x - x - 11.0 / 400.0).abs() < 1e-9, "Shift moves ten");
}

// A Spin Blur pin's part is named with its angle and the degree unit.
#[test]
fn spin_pins_are_named_with_their_angle() {
    let h = overlay(pins_of(BlurKind::Spin, &[90.0]));
    assert!(h.query_by_label("Pin 1, Blur Angle 90°").is_some());
}

struct PanelHost {
    pins: BlurPins,
    edits: usize,
}

fn panel(pins: BlurPins) -> Harness<'static, PanelHost> {
    let host = PanelHost { pins, edits: 0 };
    let mut h = Harness::<PanelHost>::builder()
        .with_size([320.0, 400.0])
        .with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, host: &mut PanelHost| {
                host.edits += usize::from(BlurPinsPanel::new("rows", &mut host.pins).show(ui).edit.is_some());
            },
            host,
        );
    h.run();
    h
}

// The panel shows Blur (Spin: Blur Angle) for the selected pin, "No pin selected" without one, and
// the kind's hints.
#[test]
fn blur_pins_panel_shows_the_selected_pins_amount_and_hints() {
    let h = panel(pins_of(BlurKind::Field, &[15.0]));
    assert!(h.query_all_by_label("Blur").count() > 0);
    assert!(h.query_by_label("No pin selected").is_none());
    assert!(h.query_by_label_contains("Click the image to add a pin").is_some());
    assert!(h.query_by_label_contains("Drag the handles on the outline").is_none());

    let h = panel(pins_of(BlurKind::Spin, &[90.0]));
    assert!(h.query_all_by_label("Blur Angle").count() > 0);
    assert!(h.query_by_label_contains("the ring sets the blur angle").is_some());

    let h = panel(pins_of(BlurKind::TiltShift, &[15.0]));
    assert!(h.query_by_label_contains("Drag the solid lines").is_some());
    assert!(h.query_by_label_contains("Click the image").is_none());

    let mut none = pins_of(BlurKind::Iris, &[15.0]);
    none.selected = None;
    let h = panel(none);
    assert!(h.query_by_label("No pin selected").is_some());
    assert!(h.query_all_by_label("Blur").count() == 0);
    assert!(h.query_by_label_contains("square to round its corners").is_some());
}

// A drag on the selected pin's ring turns the amount frame by frame: a quarter turn clockwise on
// screen adds a quarter of the range, and the turns add up past a half turn.
#[test]
fn blur_pins_ring_drag_turns_the_amount_over_several_frames() {
    let mut h = overlay(pins_of(BlurKind::Field, &[15.0]));
    let centre = at(&h, 100.0, 150.0);
    let ring = h.state().pins.pins[0];
    assert!((ring.x - 0.25).abs() < 1e-9);
    press(&mut h, centre + vec2(24.0, 0.0));
    let steps = [vec2(0.0, 24.0), vec2(-24.0, 0.0), vec2(0.0, -24.0)];
    for step in steps {
        move_to(&mut h, centre + step);
    }
    release(&mut h, centre + steps[2]);
    // Three quarter turns: 15 + 0.75 * 500.
    assert!((h.state().pins.pins[0].amount - 390.0).abs() < 1.0, "{}", h.state().pins.pins[0].amount);
}
