//! The canvas tool pill.

use egui::{Context, Event, Key, Modifiers, PointerButton, Pos2, Ui, vec2};
use egui_kittest::kittest::Queryable as _;
use egui_kittest::{Harness, HarnessBuilder};

use super::ToolPill;
use crate::icons::Icon;
use crate::patterns::{RailTool, ToolSlot};
use crate::theme::{Base, install};

type Tool = &'static str;

struct App {
    slots: Vec<ToolSlot<Tool>>,
    current: Tool,
    selected: Vec<Tool>,
    time: f64,
}

fn app() -> App {
    let slots = vec![
        ToolSlot::new(0, vec![RailTool::new("move", Icon::Move, "Move")]),
        ToolSlot::new(1, vec![RailTool::new("brush", Icon::Brush, "Brush"), RailTool::new("pencil", Icon::Pencil, "Pencil")]),
    ];
    App { slots, current: "move", selected: Vec::new(), time: 0.0 }
}

fn harness() -> Harness<'static, App> {
    let builder = HarnessBuilder::default().with_size(vec2(600.0, 400.0)).with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default()));
    let mut h = builder.build_ui_state(
        |ui: &mut Ui, app: &mut App| {
            let canvas = ui.max_rect();
            if let Some(tool) = ToolPill::new("pill").show(ui, canvas, &app.slots, &app.current).select {
                app.current = tool;
                app.selected.push(tool);
            }
        },
        app(),
    );
    h.input_mut().focused = true;
    h.run_steps(3);
    h
}

fn step(h: &mut Harness<'_, App>, events: Vec<Event>) {
    h.state_mut().time += 0.016;
    let time = h.state().time;
    h.input_mut().time = Some(time);
    h.input_mut().events.extend(events);
    h.run_steps(1);
}

fn click(h: &mut Harness<'_, App>, at: Pos2) {
    let event = |pressed| Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
    step(h, vec![Event::PointerMoved(at), event(true)]);
    step(h, vec![event(false)]);
    step(h, vec![]);
}

#[allow(clippy::expect_used)]
fn centre(h: &Harness<'_, App>, label: &str) -> Pos2 {
    h.query_by_label(label).expect("a button").rect().center()
}

#[test]
fn pill_selects_tools_and_opens_group_flyouts() {
    let mut h = harness();
    let brush = centre(&h, "Brush");
    click(&mut h, brush);
    assert_eq!(h.state().selected, ["brush"]);

    // The pill sits at the canvas's bottom centre.
    assert!(brush.y > 300.0 && (brush.x - 300.0).abs() < 100.0, "{brush:?}");

    // Alt+Down on the focused group button opens its flyout; a row of it selects that tool.
    h.get_by_label("Brush").focus();
    h.run_steps(2);
    let alt_down = |pressed| Event::Key { key: Key::ArrowDown, physical_key: None, pressed, repeat: false, modifiers: Modifiers::ALT };
    step(&mut h, vec![alt_down(true)]);
    step(&mut h, vec![alt_down(false)]);
    step(&mut h, vec![]);
    let pencil = centre(&h, "Pencil");
    click(&mut h, pencil);
    assert_eq!(h.state().selected, ["brush", "pencil"]);
}
