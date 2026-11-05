//! Tests of the connector bar (task SH-T11).

use egui::accesskit::Role;
use egui::{Context, Painter, Rect};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use super::{ConnectorBar, ConnectorBarOutput, ConnectorLine};
use crate::geometry::connector::Routing;
use crate::ui::stroke::{StrokeChoice, StrokeChoiceRow};
use craft_ui::controls::MixedValue;
use craft_ui::patterns::OptionsBar;
use craft_ui::theme::{Base, install};

struct Host {
    routing: Routing,
    line: ConnectorLine,
    choices: Vec<StrokeChoice>,
    events: Vec<ConnectorBarOutput>,
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
                let mut out = ConnectorBarOutput::default();
                OptionsBar::new("bar-options").show(ui, |slots| {
                    let paint = |_: &Painter, _: Rect, _: &StrokeChoice| {};
                    let start = StrokeChoiceRow::new("start", "Start", &host.choices).selected(MixedValue::Same("none")).paint(paint);
                    let end = StrokeChoiceRow::new("end", "End", &host.choices).selected(MixedValue::Same("arrow")).paint(paint);
                    ConnectorBar::new("bar", host.routing, host.line).caps(start, end).output(&mut out).add_to(slots);
                });
                if let Some(routing) = out.routing {
                    host.routing = routing;
                }
                if let Some(line) = out.line {
                    host.line = line;
                }
                if out != ConnectorBarOutput::default() {
                    host.events.push(out);
                }
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

// Clicking Elbow sets `routing` to Elbow; picking an end cap sets `end_cap` to its choice id; Dash then
// weight 4 set `line` to `{ dashed: true, weight: 4 }`.
#[test]
fn connector_bar_routing_caps_and_line_style_events() {
    let mut harness = bar();
    harness.get_by(|n| n.role() == Role::RadioButton && n.label().as_deref() == Some("Elbow")).click();
    harness.run_steps(4);
    pick(&mut harness, "End point", "Dot");
    pick(&mut harness, "Stroke style", "Dash");
    pick(&mut harness, "Stroke style", "Stroke weight 4");
    let events = &harness.state().events;
    assert_eq!(events.len(), 4);
    assert_eq!(events[0].routing, Some(Routing::Elbow));
    assert_eq!(events[1].end_cap.as_deref(), Some("dot"));
    assert_eq!(events[2].line, Some(ConnectorLine { dashed: true, weight: 2.0 }));
    assert_eq!(events[3].line, Some(ConnectorLine { dashed: true, weight: 4.0 }));
}
