//! The dialog stack, canvas pass-through and the blocking overlay in a harness.

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Rect, Vec2, pos2, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::pass::{self, Buttons};
use super::*;
use crate::theme::{Base, install};

const WINDOW: Vec2 = vec2(640.0, 480.0);

struct App {
    stack: DialogStack,
    a: DialogState,
    b: DialogState,
    a_open: bool,
    b_open: bool,
    /// B is shown from A's body.
    nested: bool,
    overlay: bool,
    raw_modal: bool,
    below_clicks: usize,
    log: Vec<(&'static str, Outcome)>,
    free: Option<Pos2>,
    any_modal: bool,
    presses: Vec<Pos2>,
    buttons: Buttons,
    pans: Vec<Vec2>,
    a_rect: Rect,
}

impl App {
    fn new() -> Self {
        Self {
            stack: DialogStack::default(),
            a: DialogState::new(),
            b: DialogState::new(),
            a_open: true,
            b_open: false,
            nested: true,
            overlay: false,
            raw_modal: false,
            below_clicks: 0,
            log: Vec::new(),
            free: None,
            any_modal: false,
            presses: Vec::new(),
            buttons: Buttons::default(),
            pans: Vec::new(),
            a_rect: Rect::NOTHING,
        }
    }
}

fn a_id() -> DialogId {
    dialog_id("a")
}

fn b_id() -> DialogId {
    dialog_id("b")
}

fn harness(app: App) -> Harness<'static, App> {
    HarnessBuilder::default().with_size(WINDOW).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, app: &mut App| {
            let ctx = &ui.ctx().clone();
            let canvas = ctx.content_rect();
            app.free = pass::free_pointer_over(ctx, canvas);
            app.any_modal = any_modal(ctx);
            app.presses.extend(pass::free_press(ctx, canvas));
            app.buttons = pass::free_buttons(ctx, canvas);
            app.pans.extend(pass::pan_delta(ctx, canvas, true).filter(|d| *d != Vec2::ZERO));
            if ui.button("Below").clicked() {
                app.below_clicks += 1;
            }
            if app.raw_modal {
                egui::Modal::new(egui::Id::unique("raw")).show(ctx, |ui| {
                    ui.label("raw");
                });
            }
            if app.overlay {
                BlockingOverlay::new("busy").show(ctx);
            }
            let App { stack, a, b, a_open, b_open, nested, log, a_rect, .. } = app;
            if *a_open {
                stack.push(a_id(), None);
            }
            if *b_open {
                stack.push(b_id(), nested.then(a_id));
            }
            let mut b_outcome = None;
            let mut a_closed = false;
            if *a_open {
                let shown = Dialog::new("a", "Dialog A").stack(stack).footer(Footer::standard().ok("Save A")).show(ctx, a, &mut (), |_, ()| {
                    if *b_open && *nested {
                        b_outcome = show_b(ctx, b, stack);
                    }
                });
                *a_rect = shown.rect;
                if let Some(outcome) = shown.outcome {
                    log.push(("a", outcome));
                    a_closed = outcome.closes();
                }
            }
            if *b_open && !*nested {
                b_outcome = show_b(ctx, b, stack);
            }
            let b_closed = b_outcome.is_some_and(Outcome::closes);
            log.extend(b_outcome.map(|outcome| ("b", outcome)));
            if b_closed {
                *b_open = false;
                stack.remove(b_id());
            }
            if a_closed {
                *a_open = false;
                stack.remove_tree(a_id());
            }
        },
        app,
    )
}

fn show_b(ctx: &egui::Context, b: &mut DialogState, stack: &DialogStack) -> Option<Outcome> {
    Dialog::new("b", "Dialog B")
        .stack(stack)
        .footer(Footer::standard().ok("Save B"))
        .show(ctx, b, &mut (), |ui, ()| {
            ui.label("child body");
        })
        .outcome
}

fn event(harness: &mut Harness<'_, App>, event: Event) {
    harness.event(event);
    harness.run();
}

fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

#[test]
fn only_the_top_dialog_answers_enter_and_esc() {
    let mut app = App::new();
    app.b_open = true;
    let mut harness = harness(app);
    harness.run();
    assert!(harness.get_by_label("Save A").accesskit_node().is_disabled(), "the parent is disabled while it has a child");
    assert!(!harness.get_by_label("Save B").accesskit_node().is_disabled());
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().log, [("b", Outcome::Ok)]);
    assert!(harness.state().stack.is_top(a_id()) && !harness.state().stack.has_child(a_id()));
    harness.run();
    assert!(!harness.get_by_label("Save A").accesskit_node().is_disabled(), "the parent is live again");
    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!(harness.state().log, [("b", Outcome::Ok), ("a", Outcome::Cancel)]);
    assert!(harness.state().stack.is_empty());
}

#[test]
fn the_top_dialog_is_raised_above_an_older_one() {
    let mut app = App::new();
    app.a_open = false;
    app.b_open = true;
    app.nested = false;
    app.stack.push(a_id(), None);
    let mut harness = harness(app);
    harness.run();
    harness.state_mut().a_open = true;
    harness.run();
    harness.run();
    let ctx = harness.ctx.clone();
    let (a, b) = (pass::layer_of(&ctx, a_id()).expect("A shown"), pass::layer_of(&ctx, b_id()).expect("B shown"));
    let order: Vec<_> = ctx.memory(|m| m.layer_ids().collect());
    let at = |layer| order.iter().position(|l| *l == layer).expect("in the order");
    assert!(harness.state().stack.is_top(b_id()) && at(b) > at(a), "B is the top and is drawn above A");
}

#[test]
fn the_canvas_is_free_outside_the_dialogs_only() {
    let mut harness = harness(App::new());
    harness.run();
    harness.run();
    let rect = harness.state().a_rect;
    harness.event(Event::PointerMoved(pos2(5.0, 5.0)));
    harness.run();
    assert!(harness.state().any_modal);
    assert_eq!(harness.state().free, Some(pos2(5.0, 5.0)));
    harness.event(Event::PointerMoved(rect.center()));
    harness.run();
    assert_eq!(harness.state().free, None, "over the dialog");

    harness.event(Event::PointerMoved(pos2(5.0, 5.0)));
    harness.run();
    harness.key_press(Key::Escape);
    harness.run();
    harness.run();
    harness.run();
    assert!(!harness.state().any_modal);
    harness.event(Event::PointerMoved(rect.center()));
    harness.run();
    assert_eq!(harness.state().free, Some(rect.center()), "the dialog is gone");
}

#[test]
fn a_press_counts_on_the_free_canvas_and_never_when_it_began_on_a_dialog() {
    let mut harness = harness(App::new());
    harness.run();
    harness.run();
    let rect = harness.state().a_rect;
    let title = pos2(rect.left() + 40.0, rect.top() + 14.0);

    harness.event(Event::PointerMoved(title));
    harness.run();
    harness.event(button(title, true));
    harness.run();
    harness.event(Event::PointerMoved(pos2(5.0, 400.0)));
    harness.run();
    harness.event(Event::PointerMoved(pos2(8.0, 405.0)));
    harness.run();
    assert!(harness.state().presses.is_empty(), "a title drag is not a canvas press: {:?}", harness.state().presses);
    assert!(harness.state().pans.is_empty());
    event(&mut harness, button(pos2(8.0, 405.0), false));

    let free = pos2(5.0, 5.0);
    harness.event(Event::PointerMoved(free));
    harness.run();
    harness.event(button(free, true));
    harness.event(button(free, false));
    harness.run();
    assert_eq!(harness.state().presses, [free], "a quick click counts in its own pass");
}

#[test]
fn a_hand_drag_pans_from_the_free_canvas_only() {
    let mut harness = harness(App::new());
    harness.run();
    harness.run();
    let free = pos2(5.0, 5.0);
    harness.event(Event::PointerMoved(free));
    harness.run();
    harness.event(button(free, true));
    harness.run();
    harness.event(Event::PointerMoved(free + vec2(12.0, 3.0)));
    harness.run();
    assert_eq!(harness.state().pans, [vec2(12.0, 3.0)]);
}

#[test]
fn modal_dialogs_report_themselves_to_accesskit() {
    let mut harness = harness(App::new());
    harness.run();
    let dialog = harness.get_by(|n| n.role() == Role::Dialog);
    assert!(dialog.accesskit_node().is_modal());
}

#[test]
fn a_blocking_overlay_takes_the_window_input() {
    let mut app = App::new();
    app.a_open = false;
    let mut harness = harness(app);
    harness.run();
    harness.get_by_label("Below").click();
    harness.run();
    assert_eq!(harness.state().below_clicks, 1);
    assert!(!harness.state().any_modal);

    harness.state_mut().overlay = true;
    harness.run();
    harness.run();
    assert!(harness.state().any_modal);
    harness.get_by_label("Below").click();
    harness.run();
    assert_eq!(harness.state().below_clicks, 1, "the click did not reach the window");
    let free = pos2(5.0, 5.0);
    harness.event(Event::PointerMoved(free));
    harness.run();
    assert_eq!(harness.state().free, None, "no pass-through under an overlay");
}

#[test]
fn any_modal_sees_an_egui_modal_that_is_not_a_dialog() {
    let mut app = App::new();
    app.a_open = false;
    let mut harness = harness(app);
    harness.run();
    assert!(!harness.state().any_modal);
    harness.state_mut().raw_modal = true;
    harness.run();
    harness.run();
    assert!(harness.state().any_modal, "a full-window dialog is a plain egui modal");
    harness.state_mut().raw_modal = false;
    harness.run();
    harness.run();
    assert!(!harness.state().any_modal);
}
