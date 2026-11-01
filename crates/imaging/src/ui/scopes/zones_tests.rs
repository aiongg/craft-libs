//! Tone zones on screen (egui_kittest).

use craft_ui::edit::EditEvent;
use craft_ui::theme::{Base, Tokens, TokensOf as _, install};
use egui::{Context, Event, Id, Key, Modifiers, PointerButton, Pos2, Rect, Sense, pos2, vec2};
use egui_kittest::Harness;

use super::labels::ScopePanelLabels;
use super::panel::ScopePanel;
use super::zones::{ScopeZoneEdit, ToneZone, ZoneChange, zones};

const PLOT: Rect = Rect::from_min_max(pos2(20.0, 20.0), pos2(420.0, 120.0));

#[derive(Default)]
struct Host {
    edits: Vec<ScopeZoneEdit>,
}

fn rig() -> Harness<'static, Host> {
    let mut harness =
        Harness::builder().with_size([460.0, 160.0]).with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default())).build_ui_state(
            |ui, host: &mut Host| {
                let response = ui.interact(PLOT, Id::unique("plot"), Sense::click_and_drag());
                let style = ScopePanel::style(&Tokens::of(ui));
                let values = ["0", "1", "2", "3", "4"].map(String::from);
                let labels = ScopePanelLabels::default();
                host.edits.extend(zones(ui, &response, PLOT, &ToneZone::CAMERA_RAW, &values, &labels, &style));
            },
            Host::default(),
        );
    harness.run();
    harness
}

fn x_at(share: f32) -> f32 {
    PLOT.left() + share * PLOT.width()
}

fn button(harness: &mut Harness<'_, Host>, pos: Pos2, pressed: bool, modifiers: Modifiers) {
    harness.event(Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers });
}

fn move_to(harness: &mut Harness<'_, Host>, pos: Pos2) {
    harness.event(Event::PointerMoved(pos));
    harness.run();
}

fn drags(host: &Host) -> Vec<&EditEvent<f32>> {
    host.edits.iter().filter_map(|e| if let ZoneChange::Drag(event) = &e.change { Some(event) } else { None }).collect()
}

// A drag is Begin, Update, Commit; Esc cancels; Shift moves a tenth as far; a double-click is
// `ZoneChange::Reset`.
#[test]
fn tone_zone_drag_is_one_gesture() {
    let y = PLOT.center().y;
    let start = pos2(x_at(0.2), y);
    let mut harness = rig();
    move_to(&mut harness, start);
    button(&mut harness, start, true, Modifiers::NONE);
    for dx in [20.0, 40.0] {
        move_to(&mut harness, start + vec2(dx, 0.0));
    }
    button(&mut harness, start + vec2(40.0, 0.0), false, Modifiers::NONE);
    harness.run();
    let host = harness.state();
    assert!(host.edits.iter().all(|e| e.zone == 1 && e.key == "shadows"), "the press locks the zone: {:?}", host.edits);
    let events = drags(host);
    assert!(matches!(events.first(), Some(EditEvent::Begin { .. })), "{events:?}");
    let Some(EditEvent::Commit { value, changed: true, .. }) = events.last() else { panic!("{events:?}") };
    assert!((value - 40.0 / PLOT.width()).abs() < 1e-4, "{value}");
    assert_eq!(events.iter().filter(|e| matches!(e, EditEvent::Begin { .. })).count(), 1);

    // Shift: a tenth as far.
    let mut harness = rig();
    move_to(&mut harness, start);
    button(&mut harness, start, true, Modifiers::SHIFT);
    harness.event(Event::ModifiersChanged(Modifiers::SHIFT));
    for dx in [20.0, 40.0] {
        move_to(&mut harness, start + vec2(dx, 0.0));
    }
    button(&mut harness, start + vec2(40.0, 0.0), false, Modifiers::SHIFT);
    harness.run();
    let Some(EditEvent::Commit { value, .. }) = drags(harness.state()).last().copied().cloned() else { panic!() };
    assert!((value - 4.0 / PLOT.width()).abs() < 1e-4, "{value}");

    // Esc cancels, and the release after it reports nothing more.
    let mut harness = rig();
    move_to(&mut harness, start);
    button(&mut harness, start, true, Modifiers::NONE);
    move_to(&mut harness, start + vec2(30.0, 0.0));
    harness.event(Event::Key { key: Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE });
    harness.run();
    button(&mut harness, start + vec2(30.0, 0.0), false, Modifiers::NONE);
    harness.run();
    let events = drags(harness.state());
    assert!(matches!(events.last(), Some(EditEvent::Cancel { .. })), "{events:?}");
    assert!(!events.iter().any(|e| matches!(e, EditEvent::Commit { .. })));

    // A double-click resets the zone under it.
    let mut harness = rig();
    let at = pos2(x_at(0.5), y);
    move_to(&mut harness, at);
    for _ in 0..2 {
        let tap = [true, false].map(|pressed| Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
        harness.input_mut().events.extend(tap);
        harness.step();
    }
    harness.run();
    assert!(harness.state().edits.iter().any(|e| e.key == "exposure" && e.change == ZoneChange::Reset), "{:?}", harness.state().edits);
}

// Arrow keys on the focused plot step the zone under the focus (`ZoneChange::Step`).
#[test]
fn tone_zone_keys_step_the_focused_zone() {
    let mut harness = rig();
    harness.ctx.memory_mut(|m| m.request_focus(Id::unique("plot")));
    harness.run();
    let key = |harness: &mut Harness<'_, Host>, key, modifiers| {
        harness.event(Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers });
        harness.run();
    };
    key(&mut harness, Key::ArrowRight, Modifiers::NONE);
    key(&mut harness, Key::ArrowDown, Modifiers::NONE);
    key(&mut harness, Key::ArrowRight, Modifiers::SHIFT);
    let steps: Vec<_> = harness.state().edits.iter().map(|e| (e.key, e.change.clone())).collect();
    assert_eq!(steps, [("exposure", ZoneChange::Step(1)), ("exposure", ZoneChange::Step(-1)), ("exposure", ZoneChange::Step(10))]);

    key(&mut harness, Key::PageUp, Modifiers::NONE);
    key(&mut harness, Key::ArrowLeft, Modifiers::NONE);
    assert_eq!(harness.state().edits.last().map(|e| (e.key, e.change.clone())), Some(("highlights", ZoneChange::Step(-1))));
}
