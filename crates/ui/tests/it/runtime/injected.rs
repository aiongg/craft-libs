//! Injected input: steps land on separate passes, a step is withdrawn when focus moved before its pass, and a
//! cancelled queue leaves nothing pending.

use craft_behavior::PointerButton;
use craft_keymap::{Key, Mods};
use craft_ui::buttons::SplitButton;
use craft_ui::contract::{Entry, Show as _};
use craft_ui::runtime::egui::injected::{
    cancel, hook_without_pass, idle, inject_click, inject_key, inject_text, move_focus_without_pass, take_withdrawn, withdraw,
};
use craft_ui::testing::{Fixture, trial};
use egui::{Context, Event, Id, ViewportId};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn kind(event: &Event) -> &'static str {
    match event {
        Event::PointerButton { pressed: true, .. } => "press",
        Event::PointerButton { pressed: false, .. } => "release",
        Event::PointerMoved(_) => "move",
        _ => "other",
    }
}

/// A click's move, press and release land on separate passes, one pass of events each, and the button under
/// them is clicked once; the queue is idle after the last step's pass.
#[trial]
fn a_click_lands_on_separate_passes_and_clicks_once() {
    let clicks = Rc::new(Cell::new(0));
    let frames: Rc<RefCell<Vec<Vec<&'static str>>>> = Rc::default();
    let (shown_clicks, shown_frames) = (clicks.clone(), frames.clone());
    let mut fixture = Fixture::new(move |ui| {
        shown_frames.borrow_mut().push(ui.ctx().input(|input| input.raw.events.iter().map(kind).collect()));
        let entries = [Entry::new(7u8, "Colour")];
        if craft_ui::testing::contract::clicked(&SplitButton::new("Trace", &entries).show(ui).response) {
            shown_clicks.set(shown_clicks.get() + 1);
        }
    });
    fixture.settle();
    let at = fixture.by_label("Trace").expect("the button is listed").center();
    let ctx = fixture.ctx().clone();
    inject_click(&ctx, ViewportId::ROOT, at, PointerButton::Primary, 1);
    assert!(!idle(&ctx), "the click is queued");
    let before = frames.borrow().len();
    for _ in 0..3 {
        fixture.frame(Vec::new());
    }
    assert_eq!(frames.borrow()[before..], [vec!["move"], vec!["press"], vec!["release"]]);
    assert_eq!(clicks.get(), 1);
    assert!(idle(&ctx), "nothing is queued or in flight after the last step's pass");
}

/// A step handed to the input whose pass did not run is withdrawn when focus moved, with the rest of its queue;
/// the withdrawal is reported once.
#[trial]
fn a_step_in_flight_is_withdrawn_when_focus_moves_before_its_pass() {
    let ctx = Context::default();
    let viewport = ViewportId::ROOT;
    inject_key(&ctx, viewport, Key::A, Mods::NONE);
    inject_text(&ctx, viewport, "x");
    let first = hook_without_pass(&ctx, viewport);
    assert!(matches!(first[..], [Event::Key { pressed: true, .. }]), "the first step is injected: {first:?}");
    assert!(!take_withdrawn(&ctx));

    move_focus_without_pass(&ctx, Id::unique("elsewhere"));
    assert!(hook_without_pass(&ctx, viewport).is_empty(), "the withdrawn step injects nothing");
    assert!(take_withdrawn(&ctx), "the withdrawal is reported");
    assert!(!take_withdrawn(&ctx), "and once");
    assert!(hook_without_pass(&ctx, viewport).is_empty(), "the rest of the queue went with it");
    assert!(idle(&ctx));
}

/// The application withdrawing a viewport's queue is reported like a withdrawal; cancelling empties every queue and
/// reports whether anything was pending.
#[trial]
fn cancel_and_withdraw_empty_the_queue() {
    let ctx = Context::default();
    let viewport = ViewportId::ROOT;
    inject_text(&ctx, viewport, "x");
    withdraw(&ctx, viewport);
    assert!(take_withdrawn(&ctx));
    assert!(idle(&ctx));

    inject_text(&ctx, viewport, "y");
    assert!(!idle(&ctx));
    assert!(cancel(&ctx), "something was pending");
    assert!(idle(&ctx));
    assert!(!cancel(&ctx), "nothing is left to cancel");
    assert!(!take_withdrawn(&ctx), "a cancel is not a withdrawal");
}
