//! Tests of the connector bar (task SH-T11).

use egui::accesskit::Role;
use egui::{Context, Painter, Rect};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use super::{ConnectorBar, ConnectorBarEvent, ConnectorLine};
use crate::geometry::connector::Routing;
use crate::ui::stroke::{StrokeChoice, StrokeChoiceRow};
use craft_ui::controls::MixedValue;
use craft_ui::theme::{Base, install};

struct Host {
    routing: Routing,
    line: ConnectorLine,
    choices: Vec<StrokeChoice>,
    events: Vec<ConnectorBarEvent>,
}

fn bar() -> Harness<'static, Host> {
    let choices = vec![StrokeChoice::new("none", "None"), StrokeChoice::new("arrow", "Arrow"), StrokeChoice::new("dot", "Dot")];
    let host = Host { routing: Routing::Straight, line: ConnectorLine::default(), choices, events: Vec::new() };
    let mut harness = Harness::builder()
        .with_size([600.0, 160.0])
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, host: &mut Host| {
                ui.horizontal(|ui| {
                    let paint = |_: &Painter, _: Rect, _: &StrokeChoice| {};
                    let start = StrokeChoiceRow::new("start", "Start", &host.choices).selected(MixedValue::Same("none")).paint(paint);
                    let end = StrokeChoiceRow::new("end", "End", &host.choices).selected(MixedValue::Same("arrow")).paint(paint);
                    let out = ConnectorBar::new("bar", host.routing, host.line).caps(start, end).show(ui);
                    for event in out.events {
                        match &event {
                            ConnectorBarEvent::Routing(routing) => host.routing = *routing,
                            ConnectorBarEvent::Line(line) => host.line = *line,
                            _ => {}
                        }
                        host.events.push(event);
                    }
                });
            },
            host,
        );
    harness.run_steps(4);
    harness
}

fn pick(harness: &mut Harness<'_, Host>, menu: &str, option: &str) {
    let menu = menu.to_owned();
    harness.get_by(move |n| n.role() == Role::ComboBox && n.label().as_deref() == Some(menu.as_str())).click();
    harness.run_steps(4);
    let option = option.to_owned();
    harness.get_by(move |n| n.role() == Role::ListBoxOption && n.label().as_deref() == Some(option.as_str())).click();
    harness.run_steps(4);
}

// Clicking Elbow reports `Routing(Elbow)`; picking an end cap reports `EndCap` with its choice id; Dash then
// weight 4 report `Line { dashed: true, weight: 4 }`.
#[test]
fn connector_bar_routing_caps_and_line_style_events() {
    let mut harness = bar();
    harness.get_by(|n| n.role() == Role::RadioButton && n.label().as_deref() == Some("Elbow")).click();
    harness.run_steps(4);
    pick(&mut harness, "End point", "Dot");
    pick(&mut harness, "Stroke style", "Dash");
    pick(&mut harness, "Stroke style", "Stroke weight 4");
    assert_eq!(
        harness.state().events,
        [
            ConnectorBarEvent::Routing(Routing::Elbow),
            ConnectorBarEvent::EndCap("dot".to_owned()),
            ConnectorBarEvent::Line(ConnectorLine { dashed: true, weight: 2.0 }),
            ConnectorBarEvent::Line(ConnectorLine { dashed: true, weight: 4.0 }),
        ]
    );
}
