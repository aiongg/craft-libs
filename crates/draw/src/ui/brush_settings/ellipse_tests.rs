//! Tests of the tip angle and roundness widget.

use egui::vec2;

use super::{TipAngle, TipEllipseGrab, ellipse_drag, ellipse_grab, ellipse_handles};

const RADIUS: f32 = 34.0;
const REACH: f32 = 7.0;

fn tip(degrees: f64, roundness: f64) -> TipAngle {
    TipAngle { degrees, roundness }
}

// Ports PhotoCraft's: a press within 7 pt of a roundness dot grabs Roundness, elsewhere Angle.
#[test]
fn ellipse_grab_prefers_the_dots_within_7_pt() {
    let angle = tip(0.0, 0.5);
    // At 0° the major axis points right and the dots sit above and below the centre, 17 pt out.
    let [up, down] = ellipse_handles(angle, RADIUS);
    assert!((up - vec2(0.0, -17.0)).length() < 1e-4 && (down - vec2(0.0, 17.0)).length() < 1e-4);
    assert_eq!(ellipse_grab(up + vec2(6.9, 0.0), angle, RADIUS, REACH), TipEllipseGrab::Roundness);
    assert_eq!(ellipse_grab(down + vec2(0.0, 7.0), angle, RADIUS, REACH), TipEllipseGrab::Roundness);
    assert_eq!(ellipse_grab(up + vec2(7.5, 0.0), angle, RADIUS, REACH), TipEllipseGrab::Angle);
    assert_eq!(ellipse_grab(vec2(30.0, 0.0), angle, RADIUS, REACH), TipEllipseGrab::Angle);
}

// Ports PhotoCraft's `ellipse_widget_drags_angle_and_roundness`: an Angle drag points the angle at
// the pointer in whole degrees; a Roundness drag sets |along minor| / radius, clamped to 0.01..=1.
#[test]
fn ellipse_drag_sets_angle_and_roundness() {
    let start = tip(0.0, 0.5);
    // Up on screen is 90° counter-clockwise; the roundness stays.
    assert_eq!(ellipse_drag(TipEllipseGrab::Angle, vec2(0.0, -20.0), start, RADIUS), tip(90.0, 0.5));
    assert_eq!(ellipse_drag(TipEllipseGrab::Angle, vec2(-10.0, 10.0), start, RADIUS), tip(-135.0, 0.5));
    // Within 1 pt of the centre the angle stays.
    assert_eq!(ellipse_drag(TipEllipseGrab::Angle, vec2(0.5, 0.5), tip(33.0, 0.5), RADIUS), tip(33.0, 0.5));
    // The minor axis of 0° is vertical: 17 pt along it is half of the radius.
    assert_eq!(ellipse_drag(TipEllipseGrab::Roundness, vec2(40.0, -17.0), start, RADIUS), tip(0.0, 0.5));
    assert_eq!(ellipse_drag(TipEllipseGrab::Roundness, vec2(0.0, 90.0), start, RADIUS), tip(0.0, 1.0));
    assert_eq!(ellipse_drag(TipEllipseGrab::Roundness, vec2(30.0, 0.0), start, RADIUS), tip(0.0, 0.01));
}

#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
mod widget {
    use egui::accesskit::{Action, ActionData, ActionRequest, TreeId};
    use egui::{Context, Event, Id, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, vec2};

    use super::tip;
    use crate::ui::brush_settings::{TipAngle, TipEllipse, ellipse_handles};
    use craft_ui::edit::{CancelReason, EditEvent};
    use craft_ui::theme::{Base, current, install};

    struct Rig {
        ctx: Context,
        time: f64,
        modifiers: Modifiers,
        value: TipAngle,
        rect: Rect,
        id: Id,
        events: Vec<EditEvent<TipAngle>>,
    }

    impl Rig {
        fn new(value: TipAngle) -> Self {
            let ctx = Context::default();
            install(&ctx, &Base::suite_default());
            let mut rig = Self { ctx, time: 0.0, modifiers: Modifiers::NONE, value, rect: Rect::NOTHING, id: Id::NULL, events: Vec::new() };
            rig.frame(vec![]);
            rig.frame(vec![]);
            rig
        }

        fn frame(&mut self, events: Vec<Event>) {
            let events = std::iter::once(Event::ModifiersChanged(self.modifiers)).chain(events).collect();
            let input = RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(400.0, 300.0))),
                time: Some(self.time),
                events,
                ..Default::default()
            };
            self.time += 1.0 / 60.0;
            let ctx = self.ctx.clone();
            let output = ctx.run_ui(input, |ui| {
                let edit = TipEllipse::new("ellipse", self.value).show(ui);
                self.rect = edit.response.rect;
                self.id = edit.response.id;
                if let Some(value) = edit.event.as_ref().and_then(EditEvent::value) {
                    self.value = *value;
                }
                self.events.extend(edit.event);
            });
            output.drop_without_applying_deltas();
        }

        fn button(pos: Pos2, pressed: bool) -> Event {
            Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
        }

        fn key(&mut self, key: Key, modifiers: Modifiers) -> Option<EditEvent<TipAngle>> {
            let id = self.id;
            self.ctx.memory_mut(|m| m.request_focus(id));
            self.frame(vec![]);
            self.events.clear();
            self.modifiers = modifiers;
            self.frame(vec![Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }]);
            self.modifiers = Modifiers::NONE;
            let event = self.events.pop();
            self.frame(vec![]);
            event
        }
    }

    // A drag reports Begin, Update and Commit; Esc during the drag reports Cancel.
    #[test]
    fn ellipse_drag_is_one_gesture() {
        let mut rig = Rig::new(tip(0.0, 1.0));
        let centre = rig.rect.center();
        // The arrow points right: a press 25 pt out is off the dots (at 34 pt up and down).
        let press = centre + vec2(25.0, 0.0);
        rig.frame(vec![Event::PointerMoved(press), Rig::button(press, true)]);
        let up = centre + vec2(0.0, -30.0);
        rig.frame(vec![Event::PointerMoved(up)]);
        let down = centre + vec2(0.0, 30.0);
        rig.frame(vec![Event::PointerMoved(down)]);
        rig.frame(vec![Rig::button(down, false)]);
        let kinds: Vec<&str> = rig
            .events
            .iter()
            .map(|e| match e {
                EditEvent::Begin { .. } => "begin",
                EditEvent::Update { .. } => "update",
                EditEvent::Commit { .. } => "commit",
                _ => "other",
            })
            .collect();
        assert_eq!(kinds.first(), Some(&"begin"));
        assert_eq!(kinds.last(), Some(&"commit"));
        assert!(kinds.iter().filter(|k| **k == "update").count() >= 1 && !kinds.contains(&"other"));
        assert_eq!(rig.value, tip(-90.0, 1.0));

        // Esc during a second drag cancels it.
        rig.events.clear();
        rig.frame(vec![Event::PointerMoved(press), Rig::button(press, true)]);
        rig.frame(vec![Event::PointerMoved(up)]);
        rig.frame(vec![Event::Key { key: Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE }]);
        assert!(rig.events.iter().any(|e| matches!(e, EditEvent::Cancel { reason: CancelReason::Escape, .. })), "{:?}", rig.events);
    }

    // Arrows turn 1° (Shift 15°) and wrap at ±180; Alt+arrows change the roundness by 1 %; each a Set.
    #[test]
    fn ellipse_keys_turn_and_squash() {
        let mut rig = Rig::new(tip(179.0, 0.5));
        let set = |event: Option<EditEvent<TipAngle>>| match event {
            Some(EditEvent::Set { value, .. }) => value,
            other => panic!("expected a Set, got {other:?}"),
        };
        assert_eq!(set(rig.key(Key::ArrowUp, Modifiers::NONE)), tip(180.0, 0.5));
        assert_eq!(set(rig.key(Key::ArrowRight, Modifiers::NONE)), tip(-179.0, 0.5));
        assert_eq!(set(rig.key(Key::ArrowDown, Modifiers::SHIFT)), tip(166.0, 0.5));
        assert_eq!(set(rig.key(Key::ArrowLeft, Modifiers::NONE)), tip(165.0, 0.5));
        assert_eq!(set(rig.key(Key::ArrowUp, Modifiers::ALT)), tip(165.0, 0.51));
        assert_eq!(set(rig.key(Key::ArrowDown, Modifiers::ALT)), tip(165.0, 0.5));
    }

    // The grab is the part under the press, not the part under the pointer when the drag began:
    // a press on a roundness dot that moves 8 pt sideways before the drag starts still squashes.
    #[test]
    fn ellipse_grab_follows_the_press_origin() {
        let mut rig = Rig::new(tip(0.0, 0.5));
        let centre = rig.rect.center();
        let [handle, _] = ellipse_handles(tip(0.0, 0.5), TipEllipse::style(&current(&rig.ctx)).radius);
        let dot = centre + handle;
        rig.frame(vec![Event::PointerMoved(dot), Rig::button(dot, true)]);
        let moved = dot + vec2(8.0, 0.0);
        rig.frame(vec![Event::PointerMoved(moved)]);
        rig.frame(vec![Rig::button(moved, false)]);
        assert_eq!(rig.value, tip(0.0, 0.5));
    }

    // A screen reader's SetValue sets the angle in whole degrees and keeps the roundness.
    #[test]
    fn ellipse_set_value_sets_the_angle() {
        let mut rig = Rig::new(tip(10.0, 0.5));
        let target_node = rig.id.accesskit_id();
        rig.events.clear();
        rig.frame(vec![Event::AccessKitActionRequest(ActionRequest {
            action: Action::SetValue,
            target_tree: TreeId::ROOT,
            target_node,
            data: Some(ActionData::NumericValue(-47.4)),
        })]);
        assert_eq!(rig.value, tip(-47.0, 0.5));
        assert!(matches!(rig.events.last(), Some(EditEvent::Set { .. })), "{:?}", rig.events);
    }
}
