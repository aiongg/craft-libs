//! Injected input: steps land on separate passes, a step is withdrawn when focus moved before its pass, and a
//! cancelled queue leaves nothing pending.

use craft_behavior::PointerButton;
use craft_keymap::{Key, Mods};
use craft_ui::buttons::SplitButton;
use craft_ui::contract::{Entry, Show as _};
use craft_ui::runtime::egui::injected::{hook_without_pass, move_focus_without_pass};
use craft_ui::runtime::windows::WindowHost;
use craft_ui::testing::focus::named_id;
use craft_ui::testing::{Fixture, trial};
use egui::{Context, Event, ViewportId};
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
    WindowHost::of(&ctx).inject_click(ViewportId::ROOT, at, PointerButton::Primary, 1);
    assert!(!WindowHost::of(&ctx).injected_idle(), "the click is queued");
    let before = frames.borrow().len();
    for _ in 0..3 {
        fixture.frame(Vec::new());
    }
    assert_eq!(frames.borrow()[before..], [vec!["move"], vec!["press"], vec!["release"]]);
    assert_eq!(clicks.get(), 1);
    assert!(WindowHost::of(&ctx).injected_idle(), "nothing is queued or in flight after the last step's pass");
}

/// A step handed to the input whose pass did not run is withdrawn when focus moved, with the rest of its queue;
/// the withdrawal is reported once.
#[trial]
fn a_step_in_flight_is_withdrawn_when_focus_moves_before_its_pass() {
    let ctx = Context::default();
    let viewport = ViewportId::ROOT;
    WindowHost::of(&ctx).inject_key(viewport, Key::A, Mods::NONE);
    WindowHost::of(&ctx).inject_text(viewport, "x");
    let first = hook_without_pass(&ctx, viewport);
    assert!(matches!(first[..], [Event::Key { pressed: true, .. }]), "the first step is injected: {first:?}");
    assert!(!WindowHost::of(&ctx).take_injected_withdrawn());

    move_focus_without_pass(&ctx, viewport, named_id("elsewhere"));
    assert!(hook_without_pass(&ctx, viewport).is_empty(), "the withdrawn step injects nothing");
    assert!(WindowHost::of(&ctx).take_injected_withdrawn(), "the withdrawal is reported");
    assert!(!WindowHost::of(&ctx).take_injected_withdrawn(), "and once");
    assert!(hook_without_pass(&ctx, viewport).is_empty(), "the rest of the queue went with it");
    assert!(WindowHost::of(&ctx).injected_idle());
}

/// The application withdrawing a viewport's queue is reported like a withdrawal; cancelling empties every queue and
/// reports whether anything was pending.
#[trial]
fn cancel_and_withdraw_empty_the_queue() {
    let ctx = Context::default();
    let viewport = ViewportId::ROOT;
    WindowHost::of(&ctx).inject_text(viewport, "x");
    WindowHost::of(&ctx).withdraw_injected(viewport);
    assert!(WindowHost::of(&ctx).take_injected_withdrawn());
    assert!(WindowHost::of(&ctx).injected_idle());

    WindowHost::of(&ctx).inject_text(viewport, "y");
    assert!(!WindowHost::of(&ctx).injected_idle());
    assert!(WindowHost::of(&ctx).cancel_injected(), "something was pending");
    assert!(WindowHost::of(&ctx).injected_idle());
    assert!(!WindowHost::of(&ctx).cancel_injected(), "nothing is left to cancel");
    assert!(!WindowHost::of(&ctx).take_injected_withdrawn(), "a cancel is not a withdrawal");
}

/// A chord with cmd reaches egui as the integration reports it: `mac_cmd` with `command` on macOS, `ctrl` with
/// `command` elsewhere.
#[trial]
fn injected_cmd_is_mac_cmd_on_macos_and_ctrl_elsewhere() {
    let modifiers = |os| {
        let ctx = Context::default();
        ctx.set_os(os);
        WindowHost::of(&ctx).inject_key(ViewportId::ROOT, Key::A, Mods::CMD);
        hook_without_pass(&ctx, ViewportId::ROOT)
            .into_iter()
            .find_map(|event| if let Event::ModifiersChanged(held) = event { Some(held) } else { None })
            .expect("the key's modifiers are announced")
    };
    let mac = modifiers(egui::os::OperatingSystem::Mac);
    assert!(mac.mac_cmd && mac.command && !mac.ctrl, "{mac:?}");
    let other = modifiers(egui::os::OperatingSystem::Windows);
    assert!(other.ctrl && other.command && !other.mac_cmd, "{other:?}");
}
