//! The gradient line in the edit order: a click on the line that ends another field's typing run.

use egui::accesskit::Role;
use egui::{Event, Modifiers, PointerButton, Pos2, Sense, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;
use kurbo::Point;

use super::GradientOverlay;
use super::story::{LineHost, english_names};
use crate::canvas::{CanvasView, GradientLine, LineGradient};
use crate::composites::{ColorStop, GradientStops};
use crate::controls::NumberField;
use crate::edit::{EditEvent, EditPolicy};
use crate::localized::LocaleNumbers;
use crate::theme::{Base, install};
use craft_theme::Srgba;

/// A host of a three-stop gradient on a 400 × 300 canvas at zoom 1, with a location field of
/// the middle stop drawn before the canvas. The field reports on release only, so its Commit
/// carries a value no update did. Both draw the gradient of the frame's start, as a host that
/// applies the frame's events after its controls shows it.
struct Host {
    host: LineHost,
    view: Option<CanvasView>,
    events: Vec<EditEvent<LineGradient>>,
    field_events: Vec<EditEvent<f64>>,
}

fn harness() -> Harness<'static, Host> {
    let color = vec![
        ColorStop::new(0.0, Srgba::rgb(0.0, 0.0, 1.0)),
        ColorStop::new(0.5, Srgba::rgb(1.0, 0.0, 0.0)),
        ColorStop::new(1.0, Srgba::rgb(1.0, 1.0, 1.0)),
    ];
    let line = GradientLine { start: Point::new(100.0, 150.0), end: Point::new(300.0, 150.0) };
    let host = Host {
        host: LineHost::new(LineGradient { line, stops: GradientStops { color, opacity: Vec::new() } }),
        view: None,
        events: Vec::new(),
        field_events: Vec::new(),
    };
    Harness::<Host>::builder().with_size([480.0, 400.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, host: &mut Host| {
            let value = host.host.value.clone();
            let numbers = LocaleNumbers::from_tag("en");
            let location = f64::from(value.stops.color[1].location);
            let field = NumberField::new("location", "Location", location, &numbers).policy(EditPolicy::default().on_release_only(true)).show(ui);
            if let Some(&location) = field.event.as_ref().and_then(EditEvent::value) {
                host.host.value.stops.color[1].location = location as f32;
            }
            host.field_events.extend(field.event);
            let (rect, canvas) = ui.allocate_exact_size(vec2(400.0, 300.0), Sense::click_and_drag() - Sense::FOCUSABLE);
            let view = CanvasView::new(rect, Point::new(200.0, 150.0), 1.0);
            host.view = Some(view);
            let r = GradientOverlay::new("gradient", &value.line, &value.stops, english_names()).show(ui, &canvas, &view);
            host.host.apply(&r);
            host.events.extend(r.event);
        },
        host,
    )
}

/// A click on the line (press and release in one frame) moves the focus off a location field
/// drawn before the canvas, which ends the field's typing run in that pass. The stop the click
/// adds goes out in the next pass, made from the location the run committed, which no update
/// reported.
#[test]
fn a_line_click_ending_a_fields_typing_run_keeps_the_value_the_run_committed() {
    let mut h = harness();
    let field = h.get_by(|n| n.role() == Role::SpinButton && n.label().as_deref() == Some("Location")).rect().center();
    h.event(Event::PointerMoved(field));
    h.event(Event::PointerButton { pos: field, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.event(Event::PointerButton { pos: field, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run();
    h.event(Event::Key { key: egui::Key::A, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::COMMAND });
    h.event(Event::Text("0.4".into()));
    h.run();
    assert_eq!(h.state().host.value.stops.color[1].location, 0.5, "the field reports no update");

    let at: Pos2 = h.state().view.expect("drawn").to_screen(Point::new(100.0 + 200.0 * 0.75, 150.0));
    h.event(Event::PointerMoved(at));
    h.run();
    // One frame: `Harness::event` would give the release a frame of its own.
    h.input_mut().events.extend([true, false].map(|pressed| Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }));
    h.run();

    let state = h.state();
    assert!(matches!(state.field_events.as_slice(), [EditEvent::Begin { .. }, EditEvent::Commit { .. }]), "{:?}", state.field_events);
    assert!(matches!(state.events.as_slice(), [EditEvent::Set { .. }]), "{:?}", state.events);
    let locations: Vec<f32> = state.host.value.stops.color.iter().map(|s| s.location).collect();
    assert_eq!(locations, [0.0, 0.4, 0.75, 1.0]);
}
