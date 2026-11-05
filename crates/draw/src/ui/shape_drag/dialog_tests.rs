//! Tests of the Create Shape dialog. PhotoCraft's `dialogs/shape_dialog/tests.rs`
//! cases move here; PhotoCraft keeps its control-channel tests.

use std::sync::Arc;

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};
use kurbo::{Point, Rect};

use super::{CreateShapeDialog, CreateShapeOptions, CreatedShape, ShapeKind};
use crate::geometry::shape::ShapeGeometry;
use craft_ui::dialog::{DialogHost, DialogResult, GuideUnits};
use craft_ui::localized::units::LengthUnit;
use craft_ui::theme::{Base, install};

#[derive(Default)]
struct App {
    host: DialogHost,
    answers: Vec<CreatedShape>,
    cancelled: usize,
}

fn harness(options: CreateShapeOptions, store: Arc<craft_host::MemorySections>) -> Harness<'static, App> {
    let mut app = App::default();
    app.host.open(CreateShapeDialog::new(options));
    HarnessBuilder::default()
        .with_size(vec2(900.0, 700.0))
        .with_context_setup(move |ctx| {
            install(ctx, &Base::suite_default());
            craft_host::Host::builder(craft_host::AppInfo { id: "test", name: "Test", version: "1.0" }).settings(store.clone()).install(ctx);
        })
        .build_ui_state(
            |ui, app: &mut App| {
                app.host.show(ui.ctx());
                match app.host.take::<CreateShapeDialog>() {
                    Some(DialogResult::Ok(shape)) => app.answers.push(shape),
                    Some(DialogResult::Cancel) => app.cancelled += 1,
                    _ => {}
                }
            },
            app,
        )
}

fn open(options: CreateShapeOptions, store: &Arc<craft_host::MemorySections>) -> Harness<'static, App> {
    let mut h = harness(options, store.clone());
    h.run();
    h
}

fn shown(h: &Harness<'_, App>) -> CreateShapeOptions {
    let Some(dialog) = h.state().host.get::<CreateShapeDialog>() else { panic!("the dialog is open") };
    *dialog.options()
}

fn type_into(h: &mut Harness<'_, App>, field: &str, text: &str) {
    h.get_by_role_and_label(Role::SpinButton, field).focus();
    h.run();
    h.event(Event::Text(text.into()));
    h.run();
    h.key_press(Key::Tab);
    h.run();
}

fn click_button(h: &mut Harness<'_, App>, label: &str) {
    h.get_by_role_and_label(Role::Button, label).click();
    h.run();
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> ShapeGeometry {
    ShapeGeometry::Rect { rect: Rect::new(x0, y0, x1, y1), radii: [0.0; 4] }
}

fn document() -> GuideUnits {
    GuideUnits { ruler: LengthUnit::Pixels, width_px: 1000.0, height_px: 800.0, pixels_per_inch: 72.0, points_per_inch: 72.0 }
}

// OK draws a 100 × 100 rect with its top left at the rounded click; with From Center it is centred
// on the click.
#[test]
fn create_shape_ok_draws_at_the_click_and_from_center_centres_it() {
    let store = Arc::default();
    let options = CreateShapeOptions::initial(ShapeKind::Rectangle, Point::new(10.4, 20.6));
    let mut h = open(options, &store);
    click_button(&mut h, "OK");
    let Some(plain) = h.state().answers.last() else { panic!("OK answers") };
    assert_eq!((plain.kind, &plain.geometry, plain.from_center), (ShapeKind::Rectangle, &rect(10.0, 21.0, 110.0, 121.0), false));

    let mut h = open(options.recalled(), &Arc::default());
    h.get_by_role_and_label(Role::CheckBox, "From Center").click();
    h.run();
    h.get_by_role_and_label(Role::SpinButton, "Width").focus();
    h.run();
    h.key_press(Key::Enter);
    h.run();
    let Some(centred) = h.state().answers.last() else { panic!("Enter is OK") };
    assert_eq!((&centred.geometry, centred.from_center), (&rect(-40.0, -29.0, 60.0, 71.0), true));
}

// The four radius fields are named Top Left, Top Right, Bottom Right, Bottom Left for AccessKit;
// linked, typing in one sets all four.
#[test]
fn create_shape_radius_fields_are_named_for_their_corners_and_link() {
    let mut h = open(CreateShapeOptions::initial(ShapeKind::Rectangle, Point::ZERO).recalled(), &Arc::default());
    for corner in ["Top Left", "Top Right", "Bottom Right", "Bottom Left"] {
        assert_eq!(h.get_all_by_role_and_label(Role::SpinButton, corner).count(), 1, "{corner}");
    }
    type_into(&mut h, "Bottom Right", "12");
    assert_eq!(shown(&h).radii, [12.0; 4]);

    h.get_by_label("Link radii").click();
    h.run();
    type_into(&mut h, "Top Right", "3");
    assert_eq!(shown(&h).radii, [12.0, 3.0, 12.0, 12.0], "unlinked, a field moves alone");
    click_button(&mut h, "OK");
    let Some(made) = h.state().answers.last() else { panic!("OK answers") };
    assert_eq!(made.geometry, ShapeGeometry::Rect { rect: Rect::new(0.0, 0.0, 100.0, 100.0), radii: [12.0, 3.0, 12.0, 12.0] });
}

fn secondary_click(h: &mut Harness<'_, App>, at: Pos2) {
    h.event(Event::PointerMoved(at));
    h.run();
    for pressed in [true, false] {
        h.event(Event::PointerButton { pos: at, button: PointerButton::Secondary, pressed, modifiers: Modifiers::NONE });
        h.run();
    }
    h.run_steps(3);
}

// A secondary click on Width picks inches for Width only; Height stays in the ruler unit and the
// next opening shows the ruler unit again.
#[test]
fn create_shape_width_unit_is_per_field_and_not_remembered() {
    let store = Arc::default();
    let options = CreateShapeOptions::initial(ShapeKind::Ellipse, Point::ZERO).units(document());
    let mut h = open(options, &store);
    let at = h.get_by_role_and_label(Role::SpinButton, "Width").rect().center();
    secondary_click(&mut h, at);
    let to = h.get_by_label("Inches").rect().center();
    h.event(Event::PointerMoved(to));
    h.run();
    for pressed in [true, false] {
        h.event(Event::PointerButton { pos: to, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
        h.run();
    }
    h.run_steps(2);
    let text = |h: &Harness<'_, App>, field: &str| h.get_by_role_and_label(Role::SpinButton, field).value().unwrap_or_default();
    assert!(text(&h, "Width").contains("in"), "Width shows inches: {}", text(&h, "Width"));
    assert!(text(&h, "Height").contains("px"), "Height stays in pixels: {}", text(&h, "Height"));
    type_into(&mut h, "Width", "2");
    assert_eq!(shown(&h).size, (144.0, 100.0), "2 in at 72 ppi");
    click_button(&mut h, "OK");

    let next = open(CreateShapeOptions::initial(ShapeKind::Ellipse, Point::ZERO).units(document()), &store);
    assert!(next.get_by_role_and_label(Role::SpinButton, "Width").value().unwrap_or_default().contains("px"));
}

// OK stores width, height, From Center and the star ratio under `shape.create.<kind>` and the next
// dialog of that kind opens with them; Cancel stores nothing; 0 or 300001 px shows the error and
// disables OK.
#[test]
fn create_shape_remembers_size_and_checks_bad_values() {
    let store = Arc::default();
    let star = |store: &_| open(CreateShapeOptions::initial(ShapeKind::Star, Point::ZERO), store);

    let mut h = star(&store);
    type_into(&mut h, "Width", "250");
    type_into(&mut h, "Star Ratio", "40");
    h.get_by_role_and_label(Role::CheckBox, "From Center").click();
    h.run();
    click_button(&mut h, "OK");
    assert_eq!(h.state().answers.len(), 1);

    let again = star(&store);
    let options = shown(&again);
    assert_eq!((options.size, options.from_center), ((250.0, 100.0), true));
    assert!((options.star_ratio - 0.4).abs() < 1e-9, "{}", options.star_ratio);
    let other = open(CreateShapeOptions::initial(ShapeKind::Ellipse, Point::ZERO), &store);
    assert_eq!(shown(&other).size, (100.0, 100.0), "each kind has its own memory");

    let mut cancelled = open(CreateShapeOptions::initial(ShapeKind::Ellipse, Point::ZERO), &store);
    type_into(&mut cancelled, "Width", "300");
    click_button(&mut cancelled, "Cancel");
    assert_eq!(cancelled.state().cancelled, 1);
    assert_eq!(shown(&open(CreateShapeOptions::initial(ShapeKind::Ellipse, Point::ZERO), &store)).size, (100.0, 100.0), "Cancel stores nothing");

    let mut bad = open(CreateShapeOptions::initial(ShapeKind::Rectangle, Point::ZERO), &store);
    for (text, field) in [("0", "Width"), ("300001", "Height")] {
        type_into(&mut bad, field, text);
        assert!(bad.get_by_role_and_label(Role::Button, "OK").accesskit_node().is_disabled(), "{text} px disables OK");
        let message = format!("{field} must be between 1 and 300,000 pixels");
        assert!(bad.query_all_by_label(&message).count() > 0, "the error names the field: {message}");
        type_into(&mut bad, field, "100");
        assert!(!bad.get_by_role_and_label(Role::Button, "OK").accesskit_node().is_disabled());
    }
}

// The radius fields sit in PhotoCraft's 2×2 grid (Top Left, Top Right / Bottom Left, Bottom Right)
// and keep their places from one pass to the next.
#[test]
fn create_shape_radius_fields_form_a_grid_that_holds_across_passes() {
    let mut h = open(CreateShapeOptions::initial(ShapeKind::Rectangle, Point::ZERO).recalled(), &Arc::default());
    let places =
        |h: &Harness<'_, App>| ["Top Left", "Top Right", "Bottom Left", "Bottom Right"].map(|c| h.get_by_role_and_label(Role::SpinButton, c).rect());
    let first = places(&h);
    h.run();
    h.run();
    assert_eq!(places(&h), first, "the fields do not move between passes");
    let [tl, tr, bl, br] = first;
    assert_eq!(tl.min.y, tr.min.y, "top row");
    assert_eq!(bl.min.y, br.min.y, "bottom row");
    assert!(bl.min.y > tl.min.y, "the bottom row is under the top row");
    assert_eq!((tl.min.x, tr.min.x), (bl.min.x, br.min.x), "columns line up");
    assert!(tr.min.x > tl.max.x, "the right column is right of the left one");
}

// Create Polygon shows Star Ratio at 100 %, answers it and remembers it; a Triangle has none.
#[test]
fn create_polygon_has_a_star_ratio() {
    let store = Arc::default();
    let mut h = open(CreateShapeOptions::initial(ShapeKind::Polygon, Point::ZERO), &store);
    assert_eq!(h.get_all_by_role_and_label(Role::SpinButton, "Star Ratio").count(), 1);
    assert!((shown(&h).star_ratio - 1.0).abs() < 1e-9, "100 % by default");
    type_into(&mut h, "Star Ratio", "40");
    click_button(&mut h, "OK");
    let Some(made) = h.state().answers.last() else { panic!("OK answers") };
    let ShapeGeometry::Polygon { star_ratio, .. } = made.geometry else { panic!("a polygon: {:?}", made.geometry) };
    assert!((star_ratio - 0.4).abs() < 1e-9, "{star_ratio}");

    let again = open(CreateShapeOptions::initial(ShapeKind::Polygon, Point::ZERO), &store);
    assert!((shown(&again).star_ratio - 0.4).abs() < 1e-9, "recalled");
    let triangle = open(CreateShapeOptions::initial(ShapeKind::Triangle, Point::ZERO), &store);
    assert_eq!(triangle.query_all_by_role_and_label(Role::SpinButton, "Star Ratio").count(), 0);
}

// A click more than 300,000 px from the origin shows the position error and disables OK.
#[test]
fn create_shape_checks_the_position_range() {
    let far = open(CreateShapeOptions::initial(ShapeKind::Ellipse, Point::new(300_001.0, 0.0)).recalled(), &Arc::default());
    assert!(far.get_by_role_and_label(Role::Button, "OK").accesskit_node().is_disabled());
    assert!(far.query_all_by_label("The shape's position is out of range").count() > 0);
    let edge = open(CreateShapeOptions::initial(ShapeKind::Ellipse, Point::new(-300_000.0, 300_000.0)).recalled(), &Arc::default());
    assert!(!edge.get_by_role_and_label(Role::Button, "OK").accesskit_node().is_disabled(), "the limit itself is allowed");
}

// From Center leaves the top left on a half pixel for an odd side, as PhotoCraft does.
#[test]
fn create_shape_from_center_keeps_half_pixels() {
    let mut h = open(CreateShapeOptions::initial(ShapeKind::Rectangle, Point::new(10.0, 10.0)).recalled(), &Arc::default());
    type_into(&mut h, "Width", "101");
    h.get_by_role_and_label(Role::CheckBox, "From Center").click();
    h.run();
    click_button(&mut h, "OK");
    let Some(made) = h.state().answers.last() else { panic!("OK answers") };
    assert_eq!(made.geometry, rect(-40.5, -40.0, 60.5, 60.0));
}
