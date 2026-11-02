//! Escape on a focused control.

use egui::{Context, Event, Id, Key, Modifiers, RawInput, Rect, Sense, pos2, vec2};

use super::escaped;

fn key(key: Key, modifiers: Modifiers) -> Event {
    Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }
}

/// One pass with a focusable widget `id`; `focus` asks for the focus first. Returns `escaped` and
/// `lost_focus` as the widget sees them.
fn frame(ctx: &Context, id: Id, focus: bool, events: Vec<Event>) -> (bool, bool) {
    let mut seen = (false, false);
    ctx.run_ui(RawInput { events, ..Default::default() }, |ui| {
        let r = ui.interact(Rect::from_min_size(pos2(0.0, 0.0), vec2(50.0, 50.0)), id, Sense::focusable_noninteractive());
        if focus {
            r.request_focus();
        }
        seen = (escaped(&r), r.lost_focus());
    })
    .drop_without_applying_deltas();
    seen
}

#[test]
fn escape_on_a_control_that_had_focus_is_seen_and_other_ways_of_losing_focus_are_not() {
    let id = Id::unique("escape-target");
    let ctx = Context::default();
    frame(&ctx, id, true, vec![]);
    assert_eq!(frame(&ctx, id, false, vec![]), (false, false), "focused, no key");
    assert_eq!(frame(&ctx, id, false, vec![key(Key::Escape, Modifiers::NONE)]), (true, true));
    // After the Escape the control no longer has the focus: a second Escape is not "on" it.
    assert!(!frame(&ctx, id, false, vec![key(Key::Escape, Modifiers::NONE)]).0);

    frame(&ctx, id, true, vec![]);
    assert_eq!(frame(&ctx, id, false, vec![key(Key::Escape, Modifiers::SHIFT)]), (false, false), "a modified Escape keeps the focus");
    frame(&ctx, id, true, vec![]);
    frame(&ctx, id, false, vec![]);
    ctx.memory_mut(|m| m.surrender_focus(id));
    assert!(!frame(&ctx, id, false, vec![]).0, "focus lost without Escape");
}

/// Whether an `IconButton` holds the focus after a pointer click, from raw egui input and through
/// kittest's `click`.
#[test]
fn a_pointer_click_focuses_an_icon_button_in_raw_input_and_in_kittest() {
    use crate::icons::Icon;
    use egui::{PointerButton, Ui};
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable as _;

    let id = Id::unique("icon-focus");
    let show = |ui: &mut Ui| {
        ui.add(crate::controls::IconButton::new(Icon::Plus, "Add").with_id(id));
    };
    let ctx = Context::default();
    crate::theme::install(&ctx, &crate::theme::Base::suite_default());
    let run = |events: Vec<Event>| {
        ctx.run_ui(RawInput { events, screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(300.0, 200.0))), ..Default::default() }, show)
            .drop_without_applying_deltas();
    };
    run(vec![]);
    run(vec![]);
    let at = pos2(10.0, 10.0);
    let button = |pressed| Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
    run(vec![Event::PointerMoved(at)]);
    run(vec![button(true)]);
    run(vec![button(false)]);
    run(vec![]);
    let raw = ctx.memory(|m| m.focused()) == Some(id);

    let mut harness = Harness::builder()
        .with_size(vec2(300.0, 200.0))
        .with_context_setup(|ctx| crate::theme::install(ctx, &crate::theme::Base::suite_default()))
        .build_ui(show);
    harness.run();
    harness.get_by_label("Add").click();
    harness.run();
    let kittest = harness.ctx.memory(|m| m.focused()) == Some(id);
    assert_eq!((raw, kittest), (false, false), "an IconButton takes no focus from a pointer click");
}
