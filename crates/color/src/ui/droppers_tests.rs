//! Behaviour tests of `droppers`: one armed dropper, the pass rule, and the button's toggle.

use craft_ui::edit::CancelReason;
use craft_ui::theme::{Base, install};
use egui::accesskit::{Node, Role, Toggled};
use egui::{Context, Event, Id, Modifiers, PointerButton, Pos2, RawInput, Rect, vec2};

use super::{DropperButton, DropperKind, DropperState, DropperTool};
use craft_theme::Srgba;
use craft_ui::canvas::kurbo::Point;
use craft_ui::canvas::{CanvasView, ToolHost, ToolInput};
use egui::Sense;
use egui_kittest::Harness;

fn frame(ctx: &Context, events: Vec<Event>, add: impl FnOnce(&mut egui::Ui)) -> Vec<Node> {
    let input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(400.0, 300.0))), events, ..Default::default() };
    let mut add = Some(add);
    let output = ctx.run_ui(input, |ui| add.take().map_or((), |add| add(ui)));
    let nodes = output.platform_output.accesskit_update.clone().map(|u| u.nodes).unwrap_or_default().into_iter().map(|(_, n)| n).collect();
    output.drop_without_applying_deltas();
    nodes
}

/// Runs `passes` empty frames.
fn age(ctx: &Context, passes: usize) {
    for _ in 0..passes {
        frame(ctx, Vec::new(), |_| {});
    }
}

/// Arming one dropper disarms the other; a delivered sample is taken once by its owner.
#[test]
fn one_dropper_is_armed_and_takes_its_sample_once() {
    let ctx = Context::default();
    let (a, b) = (Id::unique("a"), Id::unique("b"));
    DropperState::arm(&ctx, a);
    DropperState::arm(&ctx, b);
    assert_eq!(DropperState::armed(&ctx), Some(b));
    DropperState::disarm(&ctx, a);
    assert_eq!(DropperState::armed(&ctx), Some(b), "only the armed one is disarmed by its owner");

    let red = Srgba::rgb(1.0, 0.0, 0.0);
    DropperState::deliver(&ctx, red);
    assert_eq!(DropperState::take(&ctx, a), None);
    assert_eq!(DropperState::take(&ctx, b), Some(red));
    assert_eq!(DropperState::take(&ctx, b), None);
    assert_eq!(DropperState::armed(&ctx), None, "taking disarms until the owner arms again");
    DropperState::deliver(&ctx, red);
    assert_eq!(DropperState::take(&ctx, b), None, "nothing is armed to receive");
}

/// A dropper stays armed until its owner disarms or takes; a new arm replaces it.
#[test]
fn a_dropper_stays_armed_until_its_owner_ends_it() {
    let ctx = Context::default();
    let (a, b) = (Id::unique("a"), Id::unique("b"));
    DropperState::arm(&ctx, a);
    age(&ctx, 3);
    assert_eq!(DropperState::armed(&ctx), Some(a));
    DropperState::disarm(&ctx, b);
    assert_eq!(DropperState::armed(&ctx), Some(a), "only the owner disarms");
    DropperState::disarm(&ctx, a);
    assert_eq!(DropperState::armed(&ctx), None);
}

/// The button reads as a pressed toggle named by its label while armed.
#[test]
fn the_button_is_a_pressed_toggle_while_armed() {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    ctx.enable_accesskit();
    let show = |ui: &mut egui::Ui| DropperButton::new("black", "Set black point").kind(DropperKind::Black).show(ui);
    let find = |nodes: &[Node]| nodes.iter().find(|n| n.role() == Role::Button && n.label() == Some("Set black point")).cloned().expect("button");

    let nodes = frame(&ctx, Vec::new(), |ui| drop(show(ui)));
    let button = find(&nodes);
    assert_ne!(button.toggled(), Some(Toggled::True));
    let bounds = button.bounds().expect("bounds");
    let at = Pos2::new(((bounds.x0 + bounds.x1) / 2.0) as f32, ((bounds.y0 + bounds.y1) / 2.0) as f32);

    let press = |pressed| Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
    frame(&ctx, vec![Event::PointerMoved(at), press(true)], |ui| drop(show(ui)));
    let mut armed = false;
    let nodes = frame(&ctx, vec![press(false)], |ui| armed = show(ui).armed);
    assert!(armed && DropperState::armed(&ctx).is_some());
    assert_eq!(find(&nodes).toggled(), Some(Toggled::True));

    // A sample the canvas delivers reaches the button once, and it stays armed for the next.
    let red = Srgba::rgb(1.0, 0.0, 0.0);
    DropperState::deliver(&ctx, red);
    let mut got = None;
    frame(&ctx, Vec::new(), |ui| {
        let shown = show(ui);
        got = shown.sampled;
        armed = shown.armed;
    });
    assert_eq!(got, Some(red));
    assert!(armed);
}

/// A press and a drag on the canvas deliver the sampled colour to the armed dropper; off the image
/// nothing is delivered; Cancel (Esc during the gesture) disarms.
#[test]
fn the_canvas_tool_samples_for_the_armed_dropper() {
    let owner = Id::unique("owner");
    let tool = std::cell::RefCell::new(DropperTool::new(|doc| (doc[0] < 300.0).then(|| Srgba::rgb(doc[0] as f32 / 300.0, 0.0, 0.0))));
    let id = std::rc::Rc::new(std::cell::Cell::new(craft_ui::canvas::CanvasId::from_id(Id::NULL)));
    let shown = id.clone();
    let mut h = Harness::builder()
        .with_size(vec2(400.0, 300.0))
        .with_context_setup(|ctx| craft_ui::theme::install(ctx, &craft_ui::theme::Base::suite_default()))
        .build_ui(|ui| {
            let view = CanvasView::new(Rect::from_min_size(Pos2::ZERO, vec2(400.0, 300.0)), Point::new(200.0, 150.0), 1.0);
            let canvas_id = craft_ui::canvas::CanvasId::new(ui, "canvas");
            let canvas = canvas_id.interact(ui, view.rect, Sense::click_and_drag());
            shown.set(ToolHost::new(canvas_id).show(ui, &view, &canvas, &mut *tool.borrow_mut()).id);
        });
    h.step();
    let send = |h: &mut Harness<'_, ()>, input| {
        ToolHost::inject(&h.ctx, id.get(), input);
        h.step();
    };
    let (m, tilt) = (Modifiers::NONE, [0.0; 2]);
    DropperState::arm(&h.ctx, owner);
    send(&mut h, ToolInput::Down { doc: Point::new(150.0, 10.0), pressure: 1.0, tilt, rotation: 0.0, modifiers: m });
    assert_eq!(DropperState::take(&h.ctx, owner), Some(Srgba::rgb(0.5, 0.0, 0.0)));
    DropperState::arm(&h.ctx, owner);
    send(&mut h, ToolInput::Move { doc: Point::new(350.0, 10.0), pressure: 1.0, tilt, rotation: 0.0, modifiers: m });
    assert_eq!(DropperState::take(&h.ctx, owner), None, "off the image nothing is delivered");
    send(&mut h, ToolInput::Move { doc: Point::new(30.0, 10.0), pressure: 1.0, tilt, rotation: 0.0, modifiers: m });
    assert_eq!(DropperState::take(&h.ctx, owner), Some(Srgba::rgb(0.1, 0.0, 0.0)), "a drag resamples");
    DropperState::arm(&h.ctx, owner);
    ToolHost::cancel(&h.ctx.clone(), id.get(), &mut *tool.borrow_mut(), CancelReason::Removed);
    h.step();
    assert_eq!(DropperState::armed(&h.ctx), None);
}

/// With a modal dialog open the canvas is inert, so the free-press path samples for the armed
/// dropper: a press on the free canvas delivers, a press on the dialog does not.
#[test]
fn an_armed_dropper_samples_the_free_canvas_under_a_modal_dialog() {
    use craft_ui::dialog::{Dialog, DialogState};
    let owner = Id::unique("owner");
    let mut tool = DropperTool::new(|doc| Some(Srgba::rgb((doc[0] / 400.0) as f32, 0.0, 0.0)));
    let rect = std::rc::Rc::new(std::cell::Cell::new(Rect::NOTHING));
    let dialog_rect = rect.clone();
    let mut state = DialogState::new();
    let mut h = Harness::builder()
        .with_size(vec2(400.0, 300.0))
        .with_context_setup(|ctx| craft_ui::theme::install(ctx, &craft_ui::theme::Base::suite_default()))
        .build_ui(move |ui| {
            let ctx = ui.ctx().clone();
            let canvas = ctx.content_rect();
            let view = CanvasView::new(canvas, Point::new(200.0, 150.0), 1.0);
            dialog_rect.set(Dialog::new("modal", "Modal").show(&ctx, &mut state, &mut (), |_, ()| {}).rect);
            tool.show_free(&ctx, &view, canvas);
        });
    h.step();
    h.step();
    assert!(craft_ui::dialog::any_modal(&h.ctx), "the dialog is modal");
    let dialog = rect.get();
    let free = Pos2::new(5.0, 5.0);
    assert!(!dialog.contains(free));
    DropperState::arm(&h.ctx, owner);
    h.event(Event::PointerMoved(dialog.center()));
    h.event(Event::PointerButton { pos: dialog.center(), button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.step();
    assert_eq!(DropperState::take(&h.ctx, owner), None, "a press on the dialog is the dialog's");
    h.event(Event::PointerButton { pos: dialog.center(), button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.step();
    DropperState::arm(&h.ctx, owner);
    h.event(Event::PointerMoved(free));
    h.event(Event::PointerButton { pos: free, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.step();
    let red = DropperState::take(&h.ctx, owner).expect("the free press sampled");
    assert!((red.r - 5.0 / 400.0).abs() < 1e-3);
}
