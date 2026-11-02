//! Tests of the Shape tools' options bar (task SH-T6).

use craft_color::ui::Paint;
use craft_theme::Srgba;
use egui::accesskit::Role;
use egui::{Context, Event, Key};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::{ShapeAppearance, ShapeBar, ShapeBarOutput, ShapeChange, ShapeToolSettings};
use crate::ui::shape_drag::ShapeKind;
use craft_ui::edit::EditEvent;
use craft_ui::patterns::OptionsBar;
use craft_ui::theme::{Base, install};

struct Host {
    kind: ShapeKind,
    settings: ShapeToolSettings,
    look: ShapeAppearance,
    events: Vec<EditEvent<ShapeChange>>,
}

fn bar(kind: ShapeKind) -> Harness<'static, Host> {
    let look = ShapeAppearance::new(Paint::Solid(Srgba::rgb(1.0, 0.0, 0.0)), Paint::None, 0.0);
    let host = Host { kind, settings: ShapeToolSettings::default(), look, events: Vec::new() };
    let mut harness = Harness::builder()
        .with_size([900.0, 120.0])
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, host: &mut Host| {
                let mut out = ShapeBarOutput::default();
                OptionsBar::new("bar").show(ui, |slots| ShapeBar::new("bar", host.kind, &host.settings, &host.look).output(&mut out).add_to(slots));
                host.events.extend(out.events);
            },
            host,
        );
    harness.run_steps(4);
    harness
}

fn fields(harness: &Harness<'_, Host>) -> Vec<String> {
    harness.query_all_by_role(Role::SpinButton).filter_map(|n| n.accesskit_node().label()).collect()
}

fn type_into(harness: &mut Harness<'_, Host>, name: &str, text: &str) {
    let name = name.to_owned();
    harness.get_by(move |n| n.role() == Role::SpinButton && n.label().as_deref() == Some(name.as_str())).click();
    harness.run_steps(2);
    harness.event(Event::Text(text.into()));
    harness.run_steps(2);
    harness.key_press(Key::Enter);
    harness.run_steps(2);
}

// Rectangle shows Radius, Polygon Sides, Line Weight, Ellipse none of them; each reports its
// `ShapeChange`.
#[test]
fn shape_options_bar_shows_the_field_of_each_kind() {
    for (kind, field, text, change) in [
        (ShapeKind::Rectangle, "Radius", "12", ShapeChange::CornerRadius(12.0)),
        (ShapeKind::Polygon, "Sides", "7", ShapeChange::Sides(7)),
        (ShapeKind::Line, "Weight", "5", ShapeChange::Weight(5.0)),
    ] {
        let mut harness = bar(kind);
        assert_eq!(fields(&harness), ["Stroke width", field], "{kind:?}");
        type_into(&mut harness, field, text);
        let changes: Vec<_> = harness.state().events.iter().filter_map(EditEvent::value).cloned().collect();
        assert_eq!(changes.last(), Some(&change), "{kind:?}");
    }
    assert_eq!(fields(&bar(ShapeKind::Ellipse)), ["Stroke width"]);
}

// Choosing Solid on the Stroke chip at width 0 reports one change that carries the width 3.
#[test]
fn shape_bar_turning_the_stroke_on_is_one_change() {
    let mut harness = bar(ShapeKind::Rectangle);
    harness.get_by(|n| n.label().as_deref() == Some("Stroke color")).click();
    harness.run_steps(4);
    harness.get_by(|n| n.role() == Role::RadioButton && n.label().as_deref() == Some("Solid")).click();
    harness.run_steps(4);
    let changes: Vec<_> = harness.state().events.iter().filter_map(EditEvent::value).cloned().collect();
    assert!(matches!(changes.as_slice(), [ShapeChange::Stroke { paint: Paint::Solid(_), width: Some(w) }] if *w == 3.0), "{changes:?}");
}
