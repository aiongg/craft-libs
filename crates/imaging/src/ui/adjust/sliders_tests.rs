//! Tests of the slider tables (task AD-3).

use egui::accesskit::Role;
use egui::{Color32, Context, Event, Modifiers, PointerButton, RawInput, pos2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use super::{AdjustContext, AdjustField, AdjustLabels, AdjustSliders, AdjustTrack};
use craft_theme::Srgba;
use craft_ui::edit::EditEvent;
use craft_ui::localized::{LocaleNumbers, Unit, Units, Words};
use craft_ui::theme::{Base, install};

/// A host of Exposure's table that applies every value and records the events.
struct Host {
    values: Vec<f64>,
    events: Vec<EditEvent<Vec<f64>>>,
    context: AdjustContext,
}

fn rig(values: Vec<f64>, context: AdjustContext) -> Harness<'static, Host> {
    let state = Host { values, events: Vec::new(), context };
    let mut h = Harness::builder()
        .with_size([320.0, 300.0])
        .with_step_dt(0.02)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, host: &mut Host| {
                let fields = AdjustField::exposure(&AdjustLabels::default());
                let numbers = LocaleNumbers::from_tag("en");
                let out = AdjustSliders::new("exposure", &fields, &host.values, &numbers).context(host.context).show(ui);
                let Some(event) = out.event else { return };
                if let Some(values) = event.value() {
                    host.values.clone_from(values);
                }
                host.events.push(event);
            },
            state,
        );
    h.run_steps(3);
    h
}

fn exposure_slider(h: &Harness<'_, Host>) -> egui::Rect {
    h.get_all_by_role(Role::Slider).next().expect("Exposure's slider is the first").rect()
}

fn wheel_event() -> Event {
    Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: vec2(0.0, 1.0),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Wheel,
        modifiers: Modifiers::NONE,
    }
}

// A double-click on Exposure's slider reports one committed change with every value and Exposure at its default 0.
#[test]
fn adjust_sliders_double_click_returns_to_default() {
    let mut h = rig(vec![5.0, 0.1, 2.0], AdjustContext::dialog(true));
    // Off the centre: the middle of Exposure's track is 0 already.
    let track = exposure_slider(&h);
    let at = pos2(track.left() + track.width() * 0.8, track.center().y);
    h.event(Event::PointerMoved(at));
    h.run_steps(2);
    for pressed in [true, false, true, false] {
        h.event(Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
        h.run_steps(2);
    }
    assert_eq!(h.state().values, [0.0, 0.1, 2.0], "Exposure is back at 0, the other rows are as they were");
    // The slider reports the reset as its own gesture, one committed change, not a `Set`.
    let Some(EditEvent::Commit { value, changed: true, .. }) = h.state().events.last() else { panic!("a committed reset: {:?}", h.state().events) };
    assert_eq!(value, &[0.0, 0.1, 2.0], "the whole table, Exposure at its default");
}

// The wheel over an unfocused slider steps it with `AdjustContext::dialog` and does nothing with `properties`.
#[test]
fn adjust_sliders_wheel_steps_only_in_a_dialog_context() {
    for (context, stepped) in [(AdjustContext::dialog(true), true), (AdjustContext::properties(true), false)] {
        let mut h = rig(vec![1.0, 0.0, 1.0], context);
        h.event(Event::PointerMoved(exposure_slider(&h).center()));
        h.run_steps(2);
        h.event(wheel_event());
        h.run_steps(3);
        let exposure = h.state().values[0];
        if stepped {
            assert!((exposure - 1.01).abs() < 1e-9, "one notch is Exposure's 0.01: {exposure}");
        } else {
            assert_eq!((exposure, h.state().events.len()), (1.0, 0), "the Properties panel's wheel scrolls the panel");
        }
    }
}

// `AdjustField::round`: Exposure keeps two places, Offset four, a 0..10 range 0.01, wider ranges whole numbers (PhotoCraft's `row_value` test).
#[test]
fn adjust_sliders_round_to_the_fields_decimals() {
    let [exposure, offset, gamma] =
        <[AdjustField; 3]>::try_from(AdjustField::exposure(&AdjustLabels::default())).unwrap_or_else(|_| panic!("three rows"));
    assert_eq!(exposure.round(1.23456), 1.23);
    assert_eq!(offset.round(0.123456), 0.1235);
    assert_eq!(gamma.round(2.3456), 2.35, "a range 10 wide or less rounds to 0.01");
    let units = Units::new(std::sync::Arc::new(LocaleNumbers::from_tag("en")), std::sync::Arc::new(Words::english()));
    let vibrance = &AdjustField::vibrance(&AdjustLabels::default(), &units)[0];
    assert_eq!(vibrance.round(33.6), 34.0, "wider ranges keep whole numbers");
    let many = AdjustField::new("x", "X", 0.0..=1.0, 0.0).decimals(9);
    assert_eq!(many.round(0.123_456_789), 0.123_457, "at most six places");
}

// A field with `AdjustTrack::Colors` draws its colour track with dynamic tracks on and the plain track with them off.
#[test]
fn dynamic_tracks_off_draws_plain_tracks() {
    let red = Srgba::rgb(0.9, 0.1, 0.1);
    let colour_meshes = |context: AdjustContext| {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        let field = AdjustField::new("cyanRed", "Cyan · Red", -100.0..=100.0, 0.0)
            .unit(Unit::new("{n} %"))
            .track(AdjustTrack::Colors(vec![Srgba::rgb(0.0, 0.7, 0.8), red]));
        let output = ctx.run_ui(RawInput::default(), |ui| {
            let numbers = LocaleNumbers::from_tag("en");
            let _ = AdjustSliders::new("tracks", std::slice::from_ref(&field), &[0.0], &numbers).context(context).show(ui);
        });
        let want = Color32::from(red);
        let found = output
            .shapes
            .iter()
            .filter(|clipped| matches!(&clipped.shape, egui::Shape::Mesh(mesh) if mesh.vertices.iter().any(|v| v.color == want)))
            .count();
        output.drop_without_applying_deltas();
        found
    };
    assert_eq!(colour_meshes(AdjustContext::dialog(true)), 1, "dynamic tracks on: one gradient");
    assert_eq!(colour_meshes(AdjustContext::dialog(false)), 0, "off: the plain track");
}

// A press on Exposure's slider while Offset's field types: in that pass the slider begins and the field (drawn after it) commits on the focus loss. Both reach the host, the commit first, and the slider's updates follow its own Begin (con "ending event wins", adj M7).
#[test]
fn two_controls_in_one_pass_keep_both_begins() {
    let mut h = rig(vec![0.0, 0.0, 1.0], AdjustContext::dialog(true));
    let offset = h.get_by_role_and_label(Role::SpinButton, "Offset").rect().center();
    h.event(Event::PointerMoved(offset));
    for pressed in [true, false] {
        h.event(Event::PointerButton { pos: offset, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
        h.run_steps(2);
    }
    h.event(Event::Text("0.25".into()));
    h.run_steps(2);
    h.state_mut().events.clear();
    let track = exposure_slider(&h);
    let (from, to) = (pos2(track.center().x, track.center().y), pos2(track.left() + track.width() * 0.75, track.center().y));
    h.event(Event::PointerMoved(from));
    h.event(Event::PointerButton { pos: from, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.run_steps(2);
    for step in 1..=4 {
        h.event(Event::PointerMoved(from + (to - from) * (step as f32 / 4.0)));
        h.run_steps(2);
    }
    h.event(Event::PointerButton { pos: to, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(3);

    let events = &h.state().events;
    let kinds: Vec<&str> = events
        .iter()
        .map(|e| match e {
            EditEvent::Begin { .. } => "Begin",
            EditEvent::Update { .. } => "Update",
            EditEvent::Commit { .. } => "Commit",
            _ => "Other",
        })
        .collect();
    assert!(matches!(kinds.first(), Some(&"Commit")), "the field's run commits first: {kinds:?}");
    let mut begun = None;
    for event in &events[1..] {
        match event {
            EditEvent::Begin { id, .. } => begun = Some(*id),
            EditEvent::Update { id, .. } | EditEvent::Commit { id, .. } => assert_eq!(begun, Some(*id), "every update follows its Begin: {kinds:?}"),
            _ => {}
        }
    }
    assert!(begun.is_some() && kinds.last() == Some(&"Commit"), "the drag begins and commits: {kinds:?}");
    let values = &h.state().values;
    assert_eq!(values[1], 0.25, "the typed Offset is kept");
    assert!(values[0] > 5.0, "the drag moved Exposure: {values:?}");
}
