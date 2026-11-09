//! `ShapePanel` and `ShapeBar` through the fixture: a fake layer takes every change the cells report.

use craft_behavior::EditEvent;
use craft_color::ui::Paint;
use craft_draw::ui::{LiveShape, ShapeAppearance, ShapeBar, ShapeChange, ShapeKind, ShapePanel, ShapeToolSettings};
use craft_theme::Srgba;
use craft_ui::bars::{Bar, BarKind};
use craft_ui::contract::{Bind, Role, ShowContainer as _};
use craft_ui::testing::{Fixture, Target, pointer, trial};
use egui::{Key, Modifiers, Vec2, vec2};
use std::cell::RefCell;
use std::rc::Rc;

/// The resolution of the test document.
const PIXELS_PER_INCH: f64 = 72.0;
const PANEL_ROOM: Vec2 = vec2(420.0, 900.0);
const BAR_ROOM: Vec2 = vec2(900.0, 120.0);

/// The fake layer and what the cells reported.
struct Layer {
    shape: LiveShape,
    look: ShapeAppearance,
    settings: ShapeToolSettings,
    open: bool,
    events: Vec<EditEvent<ShapeChange>>,
}

fn rect() -> LiveShape {
    LiveShape::rect(100.0, 60.0, [1.0, 2.0, 3.0, 4.0])
}

fn look(width: f64) -> ShapeAppearance {
    ShapeAppearance::new(Paint::Solid(Srgba::rgb(1.0, 0.0, 0.0)), Paint::None, width)
}

impl Layer {
    fn new(look: ShapeAppearance) -> Rc<RefCell<Layer>> {
        Rc::new(RefCell::new(Layer { shape: rect(), look, settings: ShapeToolSettings::default(), open: false, events: Vec::new() }))
    }

    fn apply(&mut self, change: &ShapeChange) {
        match (change.clone(), &mut self.shape) {
            (ShapeChange::Size { width: w, height: h, .. }, LiveShape::Rect { width, height, .. }) => (*width, *height) = (w, h),
            (ShapeChange::Radii(next), LiveShape::Rect { radii, .. }) => *radii = next,
            (ShapeChange::Stroke { paint, width, .. }, _) => {
                self.look.stroke = paint;
                self.look.stroke_width = width.unwrap_or(self.look.stroke_width);
            }
            (ShapeChange::StrokeWidth(width), _) => self.look.stroke_width = width,
            (ShapeChange::CornerRadius(radius), _) => self.settings.corner_radius = radius,
            (ShapeChange::Sides(sides), _) => self.settings.sides = sides,
            (ShapeChange::Weight(weight), _) => self.settings.line_weight = weight,
            _ => {}
        }
    }

    fn take(&mut self, events: Vec<EditEvent<ShapeChange>>) {
        for event in events {
            if let Some(change) = event.value() {
                self.apply(&change.clone());
            }
            self.events.push(event);
        }
    }

    fn changes(&self) -> Vec<ShapeChange> {
        self.events.iter().filter_map(EditEvent::value).cloned().collect()
    }
}

fn panel(layer: &Rc<RefCell<Layer>>) -> Fixture {
    let shown = Rc::clone(layer);
    let mut fixture = Fixture::new(move |ui| {
        let mut layer = shown.borrow_mut();
        let (shape, look) = (layer.shape.clone(), layer.look.clone());
        let mut open = layer.open;
        let out = ShapePanel::new("Shape properties", &shape, &look, PIXELS_PER_INCH).stroke_open(Bind::bound(&mut open)).show(ui);
        layer.open = open;
        layer.take(out.events);
    });
    fixture.room(PANEL_ROOM);
    fixture.settle();
    fixture
}

fn bar(kind: ShapeKind, layer: &Rc<RefCell<Layer>>) -> Fixture {
    let shown = Rc::clone(layer);
    let mut fixture = Fixture::new(move |ui| {
        let mut layer = shown.borrow_mut();
        let (settings, look) = (layer.settings, layer.look.clone());
        let out = Bar::new("Shape options", BarKind::Options).show(ui, |ui, _| ShapeBar::new(kind, &settings, &look, PIXELS_PER_INCH).show(ui)).inner;
        layer.take(out.into_iter().flat_map(|out| out.events).collect());
    });
    fixture.room(BAR_ROOM);
    fixture.settle();
    fixture
}

fn field(fixture: &Fixture, name: &str) -> Target {
    fixture
        .by_role(Role::SPIN_BUTTON)
        .into_iter()
        .find(|target| craft_ui::testing::access::name(fixture.tree(), target.node) == Some(name))
        .unwrap_or_else(|| panic!("{name} is shown"))
}

/// The paint chip named `name` by its field's caption.
fn chip(fixture: &Fixture, name: &str) -> Target {
    fixture
        .by_role(Role::COLOR_WELL)
        .into_iter()
        .find(|target| craft_ui::testing::access::name(fixture.tree(), target.node) == Some(name))
        .unwrap_or_else(|| panic!("the {name} chip is shown"))
}

fn field_names(fixture: &Fixture) -> Vec<String> {
    fixture
        .by_role(Role::SPIN_BUTTON)
        .iter()
        .filter_map(|target| craft_ui::testing::access::name(fixture.tree(), target.node).map(str::to_owned))
        .collect()
}

fn type_into(fixture: &mut Fixture, name: &str, text: &str) {
    let target = field(fixture, name);
    fixture.click(&target);
    fixture.key(Key::A, Modifiers::COMMAND);
    fixture.type_text(text);
    fixture.key(Key::Enter, Modifiers::NONE);
    fixture.settle();
}

/// Scrubbing W reports `Begin`, `Update`s and one `Commit` of `ShapeChange::Size`: one undo step.
#[trial]
fn scrubbing_a_size_is_one_gesture() {
    let layer = Layer::new(look(0.0));
    let mut fixture = panel(&layer);
    // The caption left of the field scrubs it.
    let caption = fixture.texts().iter().find(|text| text.text == "W").expect("the Width caption is painted").rect.center();
    fixture.frame(vec![pointer::moved(caption)]);
    fixture.frame(vec![pointer::press(caption)]);
    fixture.frame(vec![pointer::moved(caption + vec2(15.0, 0.0))]);
    fixture.frame(vec![pointer::moved(caption + vec2(30.0, 0.0))]);
    fixture.frame(vec![pointer::release(caption + vec2(30.0, 0.0))]);
    fixture.settle();
    let layer = layer.borrow();
    let begins = layer.events.iter().filter(|event| matches!(event, EditEvent::Begin { .. })).count();
    let commits: Vec<_> = layer.events.iter().filter(|event| matches!(event, EditEvent::Commit { .. })).collect();
    assert_eq!((begins, commits.len()), (1, 1), "{:?}", layer.events);
    assert!(layer.changes().iter().all(|change| matches!(change, ShapeChange::Size { .. })), "{:?}", layer.events);
}

/// An arrow step down on a stroke width of 0 reports nothing; choosing Solid on the stroke chip at width 0 reports
/// one `Stroke` change carrying width 3.
#[trial]
fn stroke_width_zero_adds_no_stroke_and_turning_it_on_gives_three_px() {
    let layer = Layer::new(look(0.0));
    let mut fixture = panel(&layer);
    let width = field(&fixture, "Stroke width");
    fixture.click(&width);
    fixture.key(Key::ArrowDown, Modifiers::NONE);
    fixture.settle();
    assert!(layer.borrow().events.is_empty(), "{:?}", layer.borrow().events);

    let chip = chip(&fixture, "Stroke");
    fixture.click(&chip);
    fixture.settle();
    let solid = fixture.by_label("Solid").expect("the Solid tab");
    fixture.click(&solid);
    fixture.settle();
    let changes = layer.borrow().changes();
    assert!(
        matches!(changes.as_slice(), [ShapeChange::Stroke { paint: Paint::Solid(_), width: Some(w), .. }] if *w == ShapeChange::ENABLED_STROKE_WIDTH),
        "{changes:?}"
    );
}

/// A locked layer makes every field read-only: typing into one changes nothing.
#[trial]
fn a_locked_layer_takes_no_edits() {
    let layer = Layer::new(look(2.0).locked(true));
    let mut fixture = panel(&layer);
    assert!(field_names(&fixture).len() >= 8, "{:?}", field_names(&fixture));
    type_into(&mut fixture, "Width", "250");
    assert!(layer.borrow().events.is_empty(), "{:?}", layer.borrow().events);
}

/// A rect shows "Corner radius" then Top Left, Top Right, Bottom Left, Bottom Right; editing Bottom Left changes the
/// fourth radius, editing Corner radius all four.
#[trial]
fn a_rect_shows_a_uniform_radius_and_four_corners() {
    let layer = Layer::new(look(0.0));
    let mut fixture = panel(&layer);
    assert_eq!(field_names(&fixture), ["Stroke width", "Width", "Height", "Corner radius", "Top Left", "Top Right", "Bottom Left", "Bottom Right"]);

    type_into(&mut fixture, "Bottom Left", "9");
    assert_eq!(layer.borrow().changes().last(), Some(&ShapeChange::Radii([1.0, 2.0, 3.0, 9.0])));
    type_into(&mut fixture, "Corner radius", "5");
    assert_eq!(layer.borrow().changes().last(), Some(&ShapeChange::Radii([5.0; 4])));
}

/// With `stroke_open` the Stroke style button shows, and it is off while the stroke width is 0.
#[trial]
fn the_stroke_style_button_follows_the_stroke_width() {
    let on = Layer::new(look(2.0));
    let fixture = panel(&on);
    let button = fixture.by_label("Stroke style").expect("the button is shown");
    assert!(!fixture.tree().is_disabled_key(button.node));

    let off = Layer::new(look(0.0));
    let fixture = panel(&off);
    let button = fixture.by_label("Stroke style").expect("the button is shown");
    assert!(fixture.tree().is_disabled_key(button.node));
}

/// Rectangle shows Radius, Polygon Sides, Line Weight, Ellipse none of them; each reports its `ShapeChange`.
#[trial]
fn the_bar_shows_the_field_of_each_kind() {
    for (kind, name, text, change) in [
        (ShapeKind::Rectangle, "Radius", "12", ShapeChange::CornerRadius(12.0)),
        (ShapeKind::Polygon, "Sides", "7", ShapeChange::Sides(7)),
        (ShapeKind::Line, "Weight", "5", ShapeChange::Weight(5.0)),
    ] {
        let layer = Layer::new(look(0.0));
        let mut fixture = bar(kind, &layer);
        assert_eq!(field_names(&fixture), ["Stroke width", name], "{kind:?}");
        type_into(&mut fixture, name, text);
        assert_eq!(layer.borrow().changes().last(), Some(&change), "{kind:?}");
    }
    assert_eq!(field_names(&bar(ShapeKind::Ellipse, &Layer::new(look(0.0)))), ["Stroke width"]);
}

/// Choosing Solid on the bar's Stroke chip at width 0 reports one change that carries the width 3.
#[trial]
fn the_bar_turning_the_stroke_on_is_one_change() {
    let layer = Layer::new(look(0.0));
    let mut fixture = bar(ShapeKind::Rectangle, &layer);
    let chip = chip(&fixture, "Stroke");
    fixture.click(&chip);
    fixture.settle();
    let solid = fixture.by_label("Solid").expect("the Solid tab");
    fixture.click(&solid);
    fixture.settle();
    let changes = layer.borrow().changes();
    assert!(
        matches!(changes.as_slice(), [ShapeChange::Stroke { paint: Paint::Solid(_), width: Some(w), .. }] if *w == ShapeChange::ENABLED_STROKE_WIDTH),
        "{changes:?}"
    );
}
