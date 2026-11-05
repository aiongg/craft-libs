//! Tests of the point picker row and its canvas tool.

use egui::accesskit::Role;
use egui::{Id, Key, Pos2, Rect, Sense, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::{TonePick, TonePickTool, TonePicker, TonePickerRow};
use craft_theme::Srgba;
use craft_ui::canvas::kurbo::Point;
use craft_ui::canvas::{CanvasId, CanvasView, ToolHost, ToolInput};
use craft_ui::theme::{Base, install};

/// A host that keeps the armed picker and records what the row reported.
struct Row {
    armed: Option<TonePicker>,
    gray_enabled: bool,
    unavailable: bool,
    changes: Vec<Option<TonePicker>>,
}

fn row(gray_enabled: bool, unavailable: bool) -> Harness<'static, Row> {
    let state = Row { armed: None, gray_enabled, unavailable, changes: Vec::new() };
    let mut h = Harness::builder().with_size([400.0, 80.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, host: &mut Row| {
            let out = TonePickerRow::new("pickers", host.armed).enabled(TonePicker::Gray, host.gray_enabled).unavailable(host.unavailable).show(ui);
            if let Some(change) = out.change {
                host.armed = change;
                host.changes.push(change);
            }
        },
        state,
    );
    h.run_steps(2);
    h
}

fn button<'a>(h: &'a Harness<'_, Row>, name: &'a str) -> egui_kittest::Node<'a> {
    h.get_by_role_and_label(Role::Button, name)
}

// A click arms Black (`Some(Some(Black))`), a second click disarms (`Some(None)`), Esc on the focused armed button disarms and keeps the press.
#[test]
fn tone_picker_row_reports_arm_and_disarm() {
    let mut h = row(true, false);
    button(&h, "Set Black Point").click();
    h.run_steps(2);
    assert_eq!(h.state().changes, [Some(TonePicker::Black)]);
    assert_eq!(button(&h, "Set Black Point").accesskit_node().toggled(), Some(egui::accesskit::Toggled::True), "the armed button shows toggled");

    button(&h, "Set Black Point").click();
    h.run_steps(2);
    assert_eq!(h.state().changes, [Some(TonePicker::Black), None]);

    button(&h, "Set White Point").click();
    h.run_steps(2);
    assert_eq!(h.state().armed, Some(TonePicker::White));
    button(&h, "Set White Point").focus();
    h.run_steps(2);
    h.key_press(Key::Escape);
    h.run_steps(2);
    assert_eq!(h.state().changes.last(), Some(&None), "Esc on the focused armed button disarms");
}

// A picker disabled by the host (Gray in a grey document) takes no click; `unavailable` disables all three and shows the note.
#[test]
fn tone_picker_row_disables_a_picker_the_host_disables() {
    let mut h = row(false, false);
    assert!(button(&h, "Set Neutral Gray Point").accesskit_node().is_disabled());
    button(&h, "Set Neutral Gray Point").click();
    h.run_steps(2);
    assert!(h.state().changes.is_empty());
    assert!(!button(&h, "Set Black Point").accesskit_node().is_disabled());
    assert!(h.query_by_label("Unavailable in this color mode").is_none());

    let h = row(true, true);
    for name in ["Set Black Point", "Set Neutral Gray Point", "Set White Point"] {
        assert!(button(&h, name).accesskit_node().is_disabled(), "{name}");
    }
    assert!(h.query_by_label("Unavailable in this color mode").is_some());
}

/// A tool host over a 400 x 300 view at zoom 1, so document points equal screen points.
struct Canvas<'a> {
    tool: TonePickTool<'a>,
    id: CanvasId,
}

// A press on the tool host's canvas samples at the document point and `take` yields a `TonePick` for the armed picker once; off the image nothing.
#[test]
fn tone_pick_tool_reports_the_sample_for_the_armed_picker() {
    let tool = TonePickTool::new(TonePicker::White, |at| (at[0] < 300.0).then(|| Srgba::rgb(at[0] as f32 / 400.0, 0.0, 0.0)));
    assert_eq!(tool.picker(), TonePicker::White);
    let mut h = Harness::builder().with_size(vec2(400.0, 300.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, rig: &mut Canvas<'static>| {
            let view = CanvasView::new(Rect::from_min_size(Pos2::ZERO, vec2(400.0, 300.0)), Point::new(200.0, 150.0), 1.0);
            let canvas_id = CanvasId::new(ui, "canvas");
            let canvas = canvas_id.interact(ui, view.rect, Sense::click_and_drag());
            rig.id = ToolHost::new(canvas_id).show(ui, &view, &canvas, &mut rig.tool).id;
        },
        Canvas { tool, id: CanvasId::from_id(Id::NULL) },
    );
    h.step();
    let input = |h: &mut Harness<'_, Canvas<'static>>, input: ToolInput| {
        ToolHost::inject(&h.ctx, h.state().id, input);
        h.step();
    };
    let at = Point::new(100.0, 50.0);
    input(
        &mut h,
        ToolInput::Down {
            button: egui::PointerButton::Primary,
            doc: at,
            pressure: 1.0,
            tilt: [0.0; 2],
            rotation: 0.0,
            modifiers: egui::Modifiers::NONE,
        },
    );
    input(&mut h, ToolInput::Up { doc: at, modifiers: egui::Modifiers::NONE });
    let Some(pick) = h.state_mut().tool.take() else { panic!("a press on the image reports a pick") };
    assert_eq!((pick.picker, pick.at, pick.color), (TonePicker::White, [100.0, 50.0], Srgba::rgb(0.25, 0.0, 0.0)));
    assert_eq!(h.state_mut().tool.take(), None::<TonePick>, "a pick is taken once");

    let off = Point::new(350.0, 50.0);
    input(
        &mut h,
        ToolInput::Down {
            button: egui::PointerButton::Primary,
            doc: off,
            pressure: 1.0,
            tilt: [0.0; 2],
            rotation: 0.0,
            modifiers: egui::Modifiers::NONE,
        },
    );
    input(&mut h, ToolInput::Up { doc: off, modifiers: egui::Modifiers::NONE });
    assert_eq!(h.state_mut().tool.take(), None, "off the image nothing is reported");
}
