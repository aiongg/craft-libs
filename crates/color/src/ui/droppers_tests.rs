//! Behaviour tests of `droppers`: one armed dropper, the pass rule, and the button's toggle.

use craft_ui::theme::{Base, install};
use egui::accesskit::{Node, Role, Toggled};
use egui::{Context, Event, Id, Modifiers, PointerButton, Pos2, RawInput, Rect, vec2};

use super::{DropperButton, DropperKind, DropperState};
use craft_theme::Srgba;

fn frame(ctx: &Context, events: Vec<Event>, add: impl FnOnce(&mut egui::Ui)) -> Vec<Node> {
    let input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(400.0, 300.0))), events, ..Default::default() };
    let mut add = Some(add);
    let output = ctx.run_ui(input, |ui| add.take().map_or((), |add| add(ui)));
    let nodes = output.platform_output.accesskit_update.clone().map(|u| u.nodes).unwrap_or_default().into_iter().map(|(_, n)| n).collect();
    output.drop_without_applying_deltas();
    nodes
}

/// A dropper not drawn for two passes is no longer armed.
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

/// A dropper not drawn for a pass is no longer armed.
#[test]
fn an_undrawn_dropper_is_disarmed() {
    let ctx = Context::default();
    let owner = Id::unique("owner");
    DropperState::arm(&ctx, owner);
    age(&ctx, 1);
    assert_eq!(DropperState::armed(&ctx), Some(owner), "it was drawn in the pass before");
    age(&ctx, 1);
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
