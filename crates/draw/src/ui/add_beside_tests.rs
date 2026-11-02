//! Tests of the "+" buttons (task SH-T8), and of the Artboard tool's "+" press, which needs them.

use egui::accesskit::Role;
use egui::{Key, Modifiers, Pos2, Sense, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;
use kurbo::{Point, Rect};

use super::add_beside::{AddBesideHandles, AddBesideResponse, AddSide};
use crate::ui::artboard::{ArtboardBackground, ArtboardEvent, ArtboardItem, ArtboardRules, ArtboardTool};
use craft_ui::canvas::{CanvasId, CanvasView, ToolHost, ToolInput};
use craft_ui::theme::{Base, current, install};

fn view() -> CanvasView {
    CanvasView::new(egui::Rect::from_min_size(Pos2::ZERO, vec2(400.0, 300.0)), Point::new(200.0, 150.0), 1.0)
}

fn style() -> super::add_beside::AddBesideStyle {
    let ctx = egui::Context::default();
    install(&ctx, &Base::suite_default());
    AddBesideHandles::style(&current(&ctx))
}

// An object 15 points wide on screen shows no buttons; a button under a `clear_of` rect is hidden
// and the others show; `hidden(true)` hides all.
#[test]
fn add_beside_hides_when_small_or_under_clear_rectangles() {
    let (view, style) = (view(), style());
    let big = Rect::new(150.0, 100.0, 250.0, 200.0);
    let sides = |handles: &AddBesideHandles<'_>| handles.buttons(&view, &style).into_iter().map(|b| b.0).collect::<Vec<_>>();
    assert_eq!(sides(&AddBesideHandles::new("big", big)), AddSide::ALL);
    assert!(sides(&AddBesideHandles::new("small", Rect::new(190.0, 140.0, 205.0, 160.0))).is_empty(), "15 points wide");
    assert!(sides(&AddBesideHandles::new("hidden", big).hidden(true)).is_empty());

    let right = egui::Rect::from_center_size(Pos2::new(250.0 + style.offset, 150.0), vec2(4.0, 4.0));
    let clear = [right];
    assert_eq!(sides(&AddBesideHandles::new("clear", big).clear_of(&clear)), [AddSide::Top, AddSide::Bottom, AddSide::Left]);

    let near_edge = Rect::new(-40.0, 100.0, 60.0, 200.0);
    assert!(!sides(&AddBesideHandles::new("edge", near_edge)).contains(&AddSide::Left), "a button off the canvas is hidden");
}

struct Keys {
    view: CanvasView,
    out: AddBesideResponse,
}

// Tab focuses the group, → picks Right, Enter reports `clicked: Some(Right)`; a click with ⌥
// reports `copy`.
#[test]
fn add_beside_keys_and_alt_click() {
    let state = Keys { view: view(), out: AddBesideResponse::default() };
    let mut h = Harness::builder().with_size(vec2(400.0, 300.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, state: &mut Keys| {
            let out = AddBesideHandles::new("ab", Rect::new(150.0, 100.0, 250.0, 200.0)).show(ui, &state.view);
            if out.clicked.is_some() {
                state.out = out;
            }
        },
        state,
    );
    h.run_steps(2);
    h.key_press(Key::Tab);
    h.run_steps(3);
    h.key_press(Key::ArrowRight);
    h.run_steps(3);
    h.key_press(Key::Enter);
    h.run_steps(3);
    assert_eq!((h.state().out.clicked, h.state().out.copy), (Some(AddSide::Right), false));

    h.state_mut().out = AddBesideResponse::default();
    h.get_by_role_and_label(Role::Button, "Add below").click_modifiers(Modifiers::ALT);
    h.run_steps(3);
    assert_eq!((h.state().out.clicked, h.state().out.copy), (Some(AddSide::Bottom), true));
}

// On a view turned a quarter, ↑ picks the button that sits highest on screen, not the document's
// top side.
#[test]
fn add_beside_arrows_follow_screen_direction_on_a_turned_view() {
    let mut turned = view();
    turned.rotation = 90.0;
    let object = Rect::new(150.0, 100.0, 250.0, 200.0);
    let highest = AddBesideHandles::new("ab", object)
        .buttons(&turned, &style())
        .into_iter()
        .min_by(|a, b| a.1.y.total_cmp(&b.1.y))
        .map(|b| b.0)
        .expect("four buttons");
    assert_ne!(highest, AddSide::Top, "the quarter turn moved the top side");
    let state = Keys { view: turned, out: AddBesideResponse::default() };
    let mut h = Harness::builder().with_size(vec2(400.0, 300.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, state: &mut Keys| {
            let out = AddBesideHandles::new("ab", Rect::new(150.0, 100.0, 250.0, 200.0)).show(ui, &state.view);
            if out.clicked.is_some() {
                state.out = out;
            }
        },
        state,
    );
    h.run_steps(2);
    h.key_press(Key::Tab);
    h.run_steps(3);
    h.key_press(Key::ArrowUp);
    h.run_steps(3);
    h.key_press(Key::Enter);
    h.run_steps(3);
    assert_eq!(h.state().out.clicked, Some(highest));
}

struct Tool {
    tool: ArtboardTool,
    items: Vec<ArtboardItem>,
    canvas: Option<CanvasId>,
    events: Vec<ArtboardEvent>,
}

// The Artboard tool with Photoshop's rules (the "+" buttons on, which paint them every pass): a
// press on the right "+" of the selected board reports `AddBeside { side: Right, copy: false }`,
// with ⌥ `copy: true`, and starts no drag (PhotoCraft `plus_buttons_add_boards_and_shift_keeps_proportions`).
#[test]
fn artboard_plus_button_reports_add_beside_and_alt_copies() {
    let board = ArtboardItem { name: "a".into(), rect: Rect::new(150.0, 100.0, 250.0, 200.0), locked: false, background: ArtboardBackground::White };
    let state = Tool { tool: ArtboardTool::new(), items: vec![board], canvas: None, events: Vec::new() };
    let mut h = Harness::builder()
        .with_size(vec2(400.0, 300.0))
        .with_step_dt(0.05)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, s: &mut Tool| {
                let view = view();
                let id = CanvasId::new(ui, "artboard");
                s.canvas = Some(id);
                let canvas = id.interact(ui, view.rect, Sense::click_and_drag() | Sense::FOCUSABLE);
                s.tool.items(&s.items).selected(&[0]).rules(ArtboardRules::PHOTOSHOP);
                let _ = ToolHost::new(id).show(ui, &view, &canvas, &mut s.tool);
                s.events.extend(s.tool.take_events());
            },
            state,
        );
    h.step();
    let right = Point::new(250.0 + f64::from(style().offset), 150.0);
    for alt in [false, true] {
        let modifiers = if alt { Modifiers::ALT } else { Modifiers::NONE };
        let Some(id) = h.state().canvas else { panic!("a pass ran") };
        ToolHost::inject(&h.ctx, id, ToolInput::Down { doc: right, pressure: 1.0, tilt: [0.0; 2], rotation: 0.0, modifiers });
        h.step();
        ToolHost::inject(&h.ctx, id, ToolInput::Up { doc: right, modifiers });
        h.step();
        let events = std::mem::take(&mut h.state_mut().events);
        assert_eq!(events, [ArtboardEvent::AddBeside { index: 0, side: AddSide::Right, copy: alt }]);
    }
}
