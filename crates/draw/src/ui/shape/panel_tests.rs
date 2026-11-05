//! Tests of the shape panel (task SH-T6).

use craft_color::ui::Paint;
use craft_theme::Srgba;
use egui::accesskit::Role;
use egui::{Context, Event, Key, Modifiers, PointerButton, Ui};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, Node};

use super::{LiveShape, ShapeAppearance, ShapeChange, ShapePanel};
use craft_ui::edit::EditEvent;
use craft_ui::theme::{Base, install};

/// A host that applies every change the panel reports to its shape and appearance.
struct Host {
    shape: LiveShape,
    look: ShapeAppearance,
    open: bool,
    button: bool,
    events: Vec<EditEvent<ShapeChange>>,
}

impl Host {
    fn new(shape: LiveShape, look: ShapeAppearance) -> Self {
        Host { shape, look, open: false, button: false, events: Vec::new() }
    }

    fn show(&mut self, ui: &mut Ui) {
        let panel = ShapePanel::new("panel", &self.shape, &self.look);
        let panel = if self.button { panel.stroke_open(&mut self.open) } else { panel };
        let out = panel.show(ui);
        for event in out.events {
            if let Some(change) = event.value() {
                self.apply(change.clone());
            }
            self.events.push(event);
        }
    }

    fn apply(&mut self, change: ShapeChange) {
        match (change, &mut self.shape) {
            (ShapeChange::Size { width: w, height: h }, LiveShape::Rect { width, height, .. } | LiveShape::Ellipse { width, height }) => {
                (*width, *height) = (w, h);
            }
            (ShapeChange::Radii(r), LiveShape::Rect { radii, .. }) => *radii = r,
            (ShapeChange::Fill(paint), _) => self.look.fill = paint,
            (ShapeChange::Stroke { paint, width }, _) => {
                self.look.stroke = paint;
                self.look.stroke_width = width.unwrap_or(self.look.stroke_width);
            }
            (ShapeChange::StrokeWidth(width), _) => self.look.stroke_width = width,
            (ShapeChange::StrokeStyle { style, width }, _) => {
                self.look.stroke_style = style;
                self.look.stroke_width = width.unwrap_or(self.look.stroke_width);
            }
            _ => {}
        }
    }
}

fn harness(host: Host) -> Harness<'static, Host> {
    let mut harness = Harness::builder()
        .with_size([420.0, 900.0])
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default()))
        .build_ui_state(|ui, host: &mut Host| host.show(ui), host);
    harness.run_steps(4);
    harness
}

fn node<'a>(harness: &'a Harness<'_, Host>, role: Role, name: &str) -> Node<'a> {
    let name = name.to_owned();
    harness.get_by(move |n| n.role() == role && n.label().as_deref() == Some(name.as_str()))
}

fn type_into(harness: &mut Harness<'_, Host>, name: &str, text: &str) {
    node(harness, Role::SpinButton, name).click();
    harness.run_steps(2);
    harness.event(Event::Text(text.into()));
    harness.run_steps(2);
    harness.key_press(Key::Enter);
    harness.run_steps(2);
}

fn rect() -> LiveShape {
    LiveShape::Rect { width: 100.0, height: 60.0, radii: [1.0, 2.0, 3.0, 4.0] }
}

fn look(width: f64) -> ShapeAppearance {
    ShapeAppearance::new(Paint::Solid(Srgba::rgb(1.0, 0.0, 0.0)), Paint::None, width)
}

// Scrubbing W reports `Begin`, `Update`s and one `Commit` of `ShapeChange::Size`.
#[test]
fn shape_panel_scrub_is_one_undo_gesture() {
    let mut harness = harness(Host::new(rect(), look(0.0)));
    let at = node(&harness, Role::SpinButton, "Width").rect().center();
    let button = |pressed| Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
    harness.event(Event::PointerMoved(at));
    harness.event(button(true));
    harness.run_steps(2);
    for dx in [10.0, 20.0, 30.0] {
        harness.event(Event::PointerMoved(at + egui::vec2(dx, 0.0)));
        harness.run_steps(2);
    }
    harness.event(Event::PointerButton {
        pos: at + egui::vec2(30.0, 0.0),
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.run_steps(2);

    let events = &harness.state().events;
    let begins = events.iter().filter(|e| matches!(e, EditEvent::Begin { .. })).count();
    let commits: Vec<_> = events.iter().filter(|e| matches!(e, EditEvent::Commit { .. })).collect();
    assert_eq!((begins, commits.len()), (1, 1), "{events:?}");
    assert!(
        matches!(commits[0], EditEvent::Commit { value: ShapeChange::Size { width, height }, changed: true, .. } if *width > 100.0 && *height == 60.0),
        "{commits:?}"
    );
    assert!(events.iter().all(|e| e.value().is_none_or(|v| matches!(v, ShapeChange::Size { .. }))), "{events:?}");
}

// ↓ on a stroke width of 0 reports nothing; turning the stroke chip from No Color to Solid at
// width 0 reports one `Stroke` change carrying width 3.
#[test]
fn shape_panel_stroke_width_zero_does_not_add_a_stroke_and_on_gives_three_px() {
    let mut harness = harness(Host::new(rect(), look(0.0)));
    node(&harness, Role::SpinButton, "Stroke width").click();
    harness.run_steps(2);
    harness.key_press(Key::ArrowDown);
    harness.run_steps(2);
    assert!(harness.state().events.is_empty(), "{:?}", harness.state().events);

    node(&harness, Role::ColorWell, "Stroke color").click();
    harness.run_steps(4);
    node(&harness, Role::RadioButton, "Solid").click();
    harness.run_steps(4);
    let changes: Vec<_> = harness.state().events.iter().filter_map(EditEvent::value).cloned().collect();
    assert!(matches!(changes.as_slice(), [ShapeChange::Stroke { paint: Paint::Solid(_), width: Some(w) }] if *w == 3.0), "{changes:?}");
}

// A locked appearance disables every field and chip (AccessKit `disabled`).
#[test]
fn shape_panel_disables_for_locked_layer() {
    let harness = harness(Host { button: true, ..Host::new(rect(), look(2.0).locked(true)) });
    let fields: Vec<_> = harness.query_all_by_role(Role::SpinButton).chain(harness.query_all_by_role(Role::ColorWell)).collect();
    assert!(fields.len() >= 10, "{}", fields.len());
    assert!(fields.iter().all(|n| n.accesskit_node().is_disabled()));
    assert!(node(&harness, Role::Button, "Stroke Options").accesskit_node().is_disabled());
}

// A rect shows "Corner radius" then Top Left, Top Right, Bottom Left, Bottom Right; editing Bottom
// Left reports `Radii` with index 3 changed.
#[test]
fn shape_panel_rect_shows_uniform_radius_and_four_corners() {
    let mut harness = harness(Host::new(rect(), look(0.0)));
    let names: Vec<_> = harness.query_all_by_role(Role::SpinButton).filter_map(|n| n.accesskit_node().label()).collect();
    assert_eq!(names, ["Stroke width", "Width", "Height", "Corner radius", "Top Left", "Top Right", "Bottom Left", "Bottom Right"]);

    type_into(&mut harness, "Bottom Left", "9");
    let changes: Vec<_> = harness.state().events.iter().filter_map(EditEvent::value).cloned().collect();
    assert_eq!(changes.last(), Some(&ShapeChange::Radii([1.0, 2.0, 3.0, 9.0])), "{changes:?}");

    type_into(&mut harness, "Corner radius", "5");
    let changes: Vec<_> = harness.state().events.iter().filter_map(EditEvent::value).cloned().collect();
    assert_eq!(changes.last(), Some(&ShapeChange::Radii([5.0; 4])));
}

// With `stroke_open` the Stroke style button shows `ShapeAppearance::stroke_style`, is disabled
// at width 0, and a pick in its popover (Dashed on) reports `ShapeChange::StrokeStyle`.
#[test]
fn shape_panel_stroke_style_button_reports_the_style() {
    let value = |h: &Harness<'_, Host>| node(h, Role::Button, "Stroke Options").accesskit_node().value();

    let mut on = harness(Host { button: true, ..Host::new(rect(), look(2.0)) });
    assert_eq!(value(&on).as_deref(), Some("Solid"));
    node(&on, Role::Button, "Stroke Options").click();
    on.run_steps(8);
    node(&on, Role::RadioButton, "Dashed").click();
    on.run_steps(8);
    let changes: Vec<_> = on.state().events.iter().filter_map(EditEvent::value).cloned().collect();
    assert!(matches!(changes.as_slice(), [ShapeChange::StrokeStyle { style, width: None }] if !style.dashes.is_empty()), "{changes:?}");

    let mut off = harness(Host { button: true, ..Host::new(rect(), look(0.0)) });
    assert_eq!(value(&off).as_deref(), Some("No Color"));
    node(&off, Role::Button, "Stroke Options").click();
    off.run_steps(8);
    node(&off, Role::RadioButton, "Dashed").click();
    off.run_steps(8);
    let changes: Vec<_> = off.state().events.iter().filter_map(EditEvent::value).cloned().collect();
    assert!(matches!(changes.as_slice(), [ShapeChange::StrokeStyle { width: Some(w), .. }] if *w == 3.0), "{changes:?}");
}
