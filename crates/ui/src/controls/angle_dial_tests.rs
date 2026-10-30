//! The angle dial in a headless egui context: where the pointer points the needle, the edit
//! gesture of a drag, the keys, the orientation, Tab in a dialog, AccessKit and right to left.

use egui::accesskit::{Action, ActionData, ActionRequest, Node, NodeId, Role, TreeId};
use egui::{Context, Event, Id, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, Vec2, pos2, vec2};

use super::{AngleDial, AngleOrientation};
use crate::direction::{self, Direction};
use crate::edit::{CancelReason, EditEvent};
use crate::localized::LocaleNumbers;
use crate::theme::{Base, install};

/// A host with one dial and its field, which applies every value it is sent.
struct Rig {
    ctx: Context,
    time: f64,
    modifiers: Modifiers,
    angle: f64,
    range: (f64, f64),
    wrap: Option<bool>,
    orientation: AngleOrientation,
    enabled: bool,
    event: Option<EditEvent<f64>>,
    rect: Rect,
    dial: Id,
    nodes: Vec<(NodeId, Node)>,
}

impl Rig {
    fn new() -> Self {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        ctx.enable_accesskit();
        let mut rig = Self {
            ctx,
            time: 0.0,
            modifiers: Modifiers::NONE,
            angle: 120.0,
            range: (-180.0, 180.0),
            wrap: None,
            orientation: AngleOrientation::CounterClockwise,
            enabled: true,
            event: None,
            rect: Rect::NOTHING,
            dial: Id::NULL,
            nodes: Vec::new(),
        };
        rig.frame(vec![]);
        rig.frame(vec![]);
        rig
    }

    fn frame(&mut self, events: Vec<Event>) {
        let events = std::iter::once(Event::ModifiersChanged(self.modifiers)).chain(events).collect();
        let input =
            RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(600.0, 400.0))), time: Some(self.time), events, ..Default::default() };
        self.time += 1.0 / 60.0;
        let numbers = LocaleNumbers::from_tag("en");
        let ctx = self.ctx.clone();
        let output = ctx.run_ui(input, |ui| {
            let dial = AngleDial::new("dial", "Angle", self.angle, &numbers).range(self.range.0..=self.range.1);
            let dial = match self.wrap {
                Some(wrap) => dial.wrap(wrap),
                None => dial,
            };
            let edit = dial.orientation(self.orientation).enabled(self.enabled).show(ui);
            ui.text_edit_singleline(&mut String::from("elsewhere"));
            self.rect = edit.response.rect;
            self.dial = edit.response.id;
            if let Some(value) = edit.event.as_ref().and_then(EditEvent::value) {
                self.angle = *value;
            }
            self.event = edit.event;
        });
        if let Some(update) = output.platform_output.accesskit_update.clone() {
            self.nodes = update.nodes;
        }
        output.drop_without_applying_deltas();
    }

    fn at(&self, offset: Vec2) -> Pos2 {
        self.rect.center() + offset
    }

    fn press(&mut self, pos: Pos2) {
        self.frame(vec![Event::PointerMoved(pos), button(pos, true)]);
    }

    fn drag(&mut self, pos: Pos2) {
        self.frame(vec![Event::PointerMoved(pos)]);
    }

    fn release(&mut self, pos: Pos2) {
        self.frame(vec![button(pos, false)]);
    }

    /// A press and release in one frame; the value it sets, if any.
    fn tap(&mut self, offset: Vec2) -> Option<f64> {
        let pos = self.at(offset);
        self.frame(vec![Event::PointerMoved(pos), button(pos, true), button(pos, false)]);
        self.event.as_ref().and_then(EditEvent::value).copied()
    }

    /// One key press with `modifiers` on the focused dial; the value it sets, if any.
    fn key(&mut self, key: Key, modifiers: Modifiers) -> Option<f64> {
        let dial = self.dial;
        self.ctx.memory_mut(|m| m.request_focus(dial));
        self.frame(vec![]);
        self.modifiers = modifiers;
        self.frame(vec![Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }]);
        self.modifiers = Modifiers::NONE;
        let value = self.event.as_ref().and_then(EditEvent::value).copied();
        self.frame(vec![]);
        value
    }

    fn node(&self, role: Role) -> &Node {
        self.nodes.iter().map(|(_, n)| n).find(|n| n.role() == role && n.label() == Some("Angle")).expect("node")
    }

    fn action(&mut self, action: Action, data: Option<ActionData>) {
        let target_node = self.dial.accesskit_id();
        self.frame(vec![Event::AccessKitActionRequest(ActionRequest { action, target_tree: TreeId::ROOT, target_node, data })]);
    }
}

fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

fn pressed(key: Key, modifiers: Modifiers) -> Event {
    Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }
}

#[test]
fn a_press_points_the_needle_and_a_range_decides_which_angles_it_can_take() {
    let mut rig = Rig::new();
    // Counter-clockwise from 3 o'clock, y up: the light's direction under the pointer.
    assert_eq!(rig.tap(vec2(20.0, 0.0)), Some(0.0));
    assert_eq!(rig.tap(vec2(0.0, -20.0)), Some(90.0));
    assert_eq!(rig.tap(vec2(-20.0, 0.0)), Some(180.0));
    assert_eq!(rig.tap(vec2(20.0, 20.0)), Some(-45.0));
    // Whole degrees.
    assert_eq!(rig.tap(vec2(20.0, -10.0)), Some(27.0));
    // Within the dead zone at the centre the pointer changes nothing.
    assert_eq!(rig.tap(vec2(1.0, 1.0)), None);
    // Shift snaps to 45 degrees.
    rig.modifiers = Modifiers::SHIFT;
    assert_eq!(rig.tap(vec2(20.0, -10.0)), Some(45.0));
    rig.modifiers = Modifiers::NONE;

    // A range wider than a turn takes the angle nearest the current value.
    rig.range = (-360.0, 360.0);
    rig.angle = 200.0;
    assert_eq!(rig.tap(vec2(-20.0, -1.0)), Some(177.0));
    rig.angle = -200.0;
    assert_eq!(rig.tap(vec2(-20.0, 1.0)), Some(-177.0));
    rig.angle = 250.0;
    assert_eq!(rig.tap(vec2(-20.0, 1.0)), Some(183.0));

    // A narrow range holds the angle at its nearer end.
    rig.range = (0.0, 90.0);
    rig.angle = 45.0;
    assert_eq!(rig.tap(vec2(-20.0, -1.0)), Some(90.0));
    assert_eq!(rig.tap(vec2(0.0, 20.0)), Some(0.0));
}

#[test]
fn a_clockwise_dial_counts_from_12_oclock() {
    let mut rig = Rig::new();
    rig.orientation = AngleOrientation::Clockwise;
    assert_eq!(rig.tap(vec2(0.0, -20.0)), Some(0.0));
    assert_eq!(rig.tap(vec2(20.0, 0.0)), Some(90.0));
    assert_eq!(rig.tap(vec2(0.0, 20.0)), Some(180.0));
    assert_eq!(rig.tap(vec2(-20.0, 0.0)), Some(-90.0));
}

#[test]
fn a_drag_is_one_gesture_and_esc_cancels_it() {
    let mut rig = Rig::new();
    rig.press(rig.at(vec2(20.0, 0.0)));
    let Some(EditEvent::Begin { id }) = rig.event.clone() else { panic!("expected Begin, got {:?}", rig.event) };
    rig.frame(vec![]);
    assert_eq!(rig.event.as_ref().and_then(EditEvent::value), Some(&0.0), "the press points the needle");
    rig.drag(rig.at(vec2(0.0, -20.0)));
    assert!(matches!(rig.event, Some(EditEvent::Update { id: u, value, .. }) if u == id && value == 90.0), "{:?}", rig.event);
    rig.release(rig.at(vec2(0.0, -20.0)));
    assert!(matches!(rig.event, Some(EditEvent::Commit { id: c, value, changed: true, .. }) if c == id && value == 90.0), "{:?}", rig.event);

    rig.press(rig.at(vec2(-20.0, 0.0)));
    let Some(EditEvent::Begin { id }) = rig.event.clone() else { panic!("expected Begin, got {:?}", rig.event) };
    rig.drag(rig.at(vec2(0.0, 20.0)));
    rig.frame(vec![pressed(Key::Escape, Modifiers::NONE)]);
    assert_eq!(rig.event, Some(EditEvent::Cancel { id, reason: CancelReason::Escape }));
}

#[test]
fn keys_step_wrap_inside_a_turn_and_step_on_in_a_wider_range() {
    let mut rig = Rig::new();
    let none = Modifiers::NONE;
    assert_eq!(rig.key(Key::ArrowRight, none), Some(121.0));
    assert_eq!(rig.key(Key::ArrowUp, none), Some(122.0));
    assert_eq!(rig.key(Key::ArrowLeft, none), Some(121.0));
    assert_eq!(rig.key(Key::ArrowDown, Modifiers::SHIFT), Some(111.0));
    assert_eq!(rig.key(Key::PageUp, none), Some(156.0));
    assert_eq!(rig.key(Key::PageDown, none), Some(111.0));

    // One turn wraps: 180 and -180 are the same direction.
    rig.angle = 180.0;
    assert_eq!(rig.key(Key::ArrowRight, none), Some(-179.0));
    rig.angle = -180.0;
    assert_eq!(rig.key(Key::ArrowLeft, none), Some(179.0));
    rig.angle = 170.0;
    assert_eq!(rig.key(Key::PageUp, none), Some(-145.0));

    // A wider range steps on past a half turn, and stops at its ends.
    rig.range = (-360.0, 360.0);
    rig.angle = 180.0;
    assert_eq!(rig.key(Key::ArrowRight, none), Some(181.0));
    rig.angle = 359.0;
    assert_eq!(rig.key(Key::PageUp, none), Some(360.0));

    // A narrow range stops too, and Home and End are not the dial's.
    rig.range = (0.0, 90.0);
    rig.angle = 89.0;
    assert_eq!(rig.key(Key::ArrowRight, Modifiers::SHIFT), Some(90.0));
    assert_eq!(rig.key(Key::End, none), None);
    assert_eq!(rig.key(Key::Home, none), None);
}

#[test]
fn a_wrapping_dial_continues_from_the_other_end_of_a_narrow_range() {
    let mut rig = Rig::new();
    rig.range = (-90.0, 90.0);
    rig.wrap = Some(true);
    rig.angle = 80.0;
    // A drag from 80 degrees on to 100 reads -80: the opposite direction is the same line.
    rig.press(rig.at(vec2(20.0 * 80f32.to_radians().cos(), -20.0 * 80f32.to_radians().sin())));
    rig.drag(rig.at(vec2(20.0 * 100f32.to_radians().cos(), -20.0 * 100f32.to_radians().sin())));
    assert_eq!(rig.event.as_ref().and_then(EditEvent::value), Some(&-80.0));
    rig.release(rig.at(vec2(0.0, 20.0)));

    rig.angle = 90.0;
    assert_eq!(rig.key(Key::ArrowRight, Modifiers::NONE), Some(-89.0));

    // Without wrap the same range holds the angle at its end.
    rig.wrap = None;
    rig.angle = 90.0;
    assert_eq!(rig.key(Key::ArrowRight, Modifiers::NONE), None);
    // A full turn can be told not to wrap.
    rig.range = (-180.0, 180.0);
    rig.wrap = Some(false);
    rig.angle = 180.0;
    assert_eq!(rig.key(Key::ArrowRight, Modifiers::NONE), None);
}

#[test]
fn keys_change_nothing_during_a_drag() {
    let mut rig = Rig::new();
    rig.press(rig.at(vec2(20.0, 0.0)));
    rig.frame(vec![]);
    let before = rig.angle;
    rig.frame(vec![pressed(Key::ArrowUp, Modifiers::NONE)]);
    assert_eq!(rig.angle, before);
    rig.release(rig.at(vec2(20.0, 0.0)));
}

#[test]
fn the_dial_is_a_slider_with_its_value_bounds_and_actions() {
    let mut rig = Rig::new();
    let node = rig.node(Role::Slider);
    assert_eq!(node.numeric_value(), Some(120.0));
    assert_eq!((node.min_numeric_value(), node.max_numeric_value(), node.numeric_value_step()), (Some(-180.0), Some(180.0), Some(1.0)));
    assert_eq!(node.value(), Some("120°"));
    for action in [Action::Increment, Action::Decrement, Action::SetValue] {
        assert!(node.supports_action(action), "{action:?}");
    }
    let _ = rig.node(Role::SpinButton);

    rig.action(Action::Increment, None);
    assert_eq!(rig.angle, 121.0);
    rig.action(Action::Decrement, None);
    rig.action(Action::Decrement, None);
    assert_eq!(rig.angle, 119.0);
    rig.action(Action::SetValue, Some(ActionData::NumericValue(-30.0)));
    assert_eq!(rig.angle, -30.0);
    rig.frame(vec![]);
    assert_eq!(rig.node(Role::Slider).value(), Some("-30°"));
}

#[test]
fn a_disabled_dial_takes_no_input() {
    let mut rig = Rig::new();
    rig.enabled = false;
    rig.frame(vec![]);
    assert_eq!(rig.tap(vec2(20.0, 0.0)), None);
    assert_eq!(rig.angle, 120.0);
    assert!(!rig.node(Role::Slider).supports_action(Action::SetValue));
}

#[test]
fn right_to_left_keeps_the_dials_geometry_and_puts_the_field_before_it() {
    let mut rig = Rig::new();
    let field_x = |rig: &Rig| rig.node(Role::SpinButton).bounds().map(|b| b.x0 as f32).expect("bounds");
    assert!(field_x(&rig) > rig.rect.right(), "the field follows the dial");
    direction::apply(&rig.ctx, Direction::Rtl);
    rig.frame(vec![]);
    rig.frame(vec![]);
    assert!(field_x(&rig) < rig.rect.left(), "the field comes first right to left");
    assert_eq!(rig.tap(vec2(20.0, 0.0)), Some(0.0), "the angle is the document's: right is still 0");
    assert_eq!(rig.key(Key::ArrowRight, Modifiers::NONE), Some(1.0));
}

/// Tab visits the dialog's fields and passes the dial; Ctrl+Tab reaches it.
#[test]
fn tab_passes_the_dial_in_a_field_scope_and_ctrl_tab_reaches_it() {
    let story = super::angle_dial::stories().into_iter().find(|s| s.id == "controls/angle-dial/dialog").expect("the story");
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    ctx.enable_accesskit();
    let mut args = story.defaults();
    let mut time = 0.0;
    let mut nodes = Vec::new();
    let mut frame = |events: Vec<Event>, nodes: &mut Vec<(NodeId, Node)>| {
        let input =
            RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(600.0, 400.0))), time: Some(time), events, ..Default::default() };
        time += 1.0 / 60.0;
        let output = ctx.run_ui(input, |ui| (story.show)(ui, &mut args));
        if let Some(update) = output.platform_output.accesskit_update.clone() {
            *nodes = update.nodes;
        }
        output.drop_without_applying_deltas();
    };
    let focused = |ctx: &Context, nodes: &[(NodeId, Node)]| {
        let id = ctx.memory(|m| m.focused())?.accesskit_id();
        nodes.iter().find(|(n, _)| *n == id).map(|(_, node)| (node.role(), node.label().unwrap_or_default().to_owned()))
    };
    for _ in 0..3 {
        frame(vec![], &mut nodes);
    }
    for expected in ["Name", "Angle", "Note", "Name"] {
        frame(vec![pressed(Key::Tab, Modifiers::NONE)], &mut nodes);
        frame(vec![], &mut nodes);
        frame(vec![], &mut nodes);
        let (role, name) = focused(&ctx, &nodes).expect("a focus");
        assert_eq!(name, expected);
        assert_ne!(role, Role::Slider, "Tab never stops on the dial");
    }
    let mut reached = false;
    for _ in 0..4 {
        frame(vec![pressed(Key::Tab, Modifiers::CTRL)], &mut nodes);
        frame(vec![], &mut nodes);
        frame(vec![], &mut nodes);
        reached |= focused(&ctx, &nodes).is_some_and(|(role, _)| role == Role::Slider);
    }
    assert!(reached, "Ctrl+Tab reaches the dial");
}

/// The field's typing run is a gesture of the same response: Begin on the first change, one
/// Commit on Enter with the typed angle.
#[test]
fn typing_in_the_field_is_one_gesture_of_the_dial_response() {
    let mut rig = Rig::new();
    let bounds = rig.node(Role::SpinButton).bounds().expect("bounds");
    let at = pos2(((bounds.x0 + bounds.x1) / 2.0) as f32, ((bounds.y0 + bounds.y1) / 2.0) as f32);
    rig.frame(vec![Event::PointerMoved(at), button(at, true)]);
    rig.frame(vec![button(at, false)]);
    rig.frame(vec![pressed(Key::A, Modifiers::COMMAND)]);
    let mut events = Vec::new();
    for event in [Event::Text("45".into()), pressed(Key::Enter, Modifiers::NONE)] {
        rig.frame(vec![event]);
        events.extend(rig.event.clone());
        rig.frame(vec![]);
        events.extend(rig.event.clone());
    }
    let Some(EditEvent::Begin { id }) = events.first().cloned() else { panic!("expected Begin first, got {events:?}") };
    assert!(matches!(events.last(), Some(EditEvent::Commit { id: c, value, .. }) if *c == id && *value == 45.0), "{events:?}");
    assert_eq!(rig.angle, 45.0);
}

/// A mixed selection (the host's NaN) draws no needle and a blank field; a press begins no
/// gesture and a click sets the angle, as a slider's does.
#[test]
fn a_mixed_dial_takes_a_click_and_no_drag() {
    let mut rig = Rig::new();
    rig.angle = f64::NAN;
    rig.frame(vec![]);
    let mut events = Vec::new();
    let start = rig.at(vec2(20.0, 0.0));
    rig.press(start);
    events.extend(rig.event.clone());
    for offset in [vec2(0.0, -20.0), vec2(-20.0, 0.0)] {
        rig.drag(rig.at(offset));
        events.extend(rig.event.clone());
    }
    rig.release(start);
    events.extend(rig.event.clone());
    rig.frame(vec![]);
    events.extend(rig.event.clone());
    assert!(events.is_empty(), "{events:?}");
    assert!(rig.angle.is_nan());
    assert_eq!(rig.tap(vec2(0.0, -20.0)), Some(90.0));
}

#[test]
fn typing_in_a_mixed_dials_field_commits_the_number() {
    let mut rig = Rig::new();
    rig.angle = f64::NAN;
    rig.frame(vec![]);
    let bounds = rig.node(Role::SpinButton).bounds().expect("bounds");
    let at = pos2(((bounds.x0 + bounds.x1) / 2.0) as f32, ((bounds.y0 + bounds.y1) / 2.0) as f32);
    rig.frame(vec![Event::PointerMoved(at), button(at, true)]);
    rig.frame(vec![button(at, false)]);
    let mut events = Vec::new();
    for event in [Event::Text("45".into()), pressed(Key::Enter, Modifiers::NONE)] {
        rig.frame(vec![event]);
        events.extend(rig.event.clone());
        rig.frame(vec![]);
        events.extend(rig.event.clone());
    }
    let Some(EditEvent::Begin { id }) = events.first().cloned() else { panic!("expected Begin first, got {events:?}") };
    assert!(matches!(events.last(), Some(EditEvent::Commit { id: c, value, .. }) if *c == id && *value == 45.0), "{events:?}");
    assert!(!events.iter().any(|e| matches!(e, EditEvent::Cancel { .. })), "{events:?}");
    assert_eq!(rig.angle, 45.0);
}

#[test]
fn a_mixed_dial_has_no_accessible_value() {
    let mut rig = Rig::new();
    rig.angle = f64::NAN;
    rig.frame(vec![]);
    let node = rig.node(Role::Slider);
    assert_eq!((node.numeric_value(), node.value()), (None, None));
    assert_eq!((node.min_numeric_value(), node.max_numeric_value()), (Some(-180.0), Some(180.0)));
    assert!(node.supports_action(Action::SetValue));
}

#[test]
fn cancelling_a_typing_run_on_a_mixed_dial_reports_no_value_the_host_never_had() {
    let mut rig = Rig::new();
    rig.angle = f64::NAN;
    rig.frame(vec![]);
    let bounds = rig.node(Role::SpinButton).bounds().expect("bounds");
    let at = pos2(((bounds.x0 + bounds.x1) / 2.0) as f32, ((bounds.y0 + bounds.y1) / 2.0) as f32);
    rig.frame(vec![Event::PointerMoved(at), button(at, true)]);
    rig.frame(vec![button(at, false)]);
    let mut events = Vec::new();
    for event in [Event::Text("4".into()), pressed(Key::Escape, Modifiers::NONE)] {
        rig.frame(vec![event]);
        events.extend(rig.event.clone());
        rig.frame(vec![]);
        events.extend(rig.event.clone());
    }
    assert!(matches!(events.first(), Some(EditEvent::Begin { .. })), "{events:?}");
    assert!(matches!(events.last(), Some(EditEvent::Cancel { reason: CancelReason::Escape, .. })), "{events:?}");
    let values: Vec<f64> = events.iter().filter_map(|e| e.value().copied()).collect();
    assert!(values.iter().all(|v| *v == 4.0), "only what was typed: {values:?}");
}

/// A screen reader's Increment on the dial while the field's typing run owns the gesture waits
/// for the run's commit, then steps from the typed angle: the typed value is the value of its
/// turn.
#[test]
fn an_increment_during_the_fields_typing_run_steps_from_the_typed_value() {
    let mut rig = Rig::new();
    let bounds = rig.node(Role::SpinButton).bounds().expect("bounds");
    let at = pos2(((bounds.x0 + bounds.x1) / 2.0) as f32, ((bounds.y0 + bounds.y1) / 2.0) as f32);
    rig.frame(vec![Event::PointerMoved(at), button(at, true)]);
    rig.frame(vec![button(at, false)]);
    rig.frame(vec![pressed(Key::A, Modifiers::COMMAND)]);
    let mut events = Vec::new();
    rig.frame(vec![Event::Text("45".into())]);
    events.extend(rig.event.clone());
    rig.action(Action::Increment, None);
    events.extend(rig.event.clone());
    for _ in 0..2 {
        rig.frame(vec![]);
        events.extend(rig.event.clone());
    }
    rig.frame(vec![pressed(Key::Enter, Modifiers::NONE)]);
    events.extend(rig.event.clone());
    for _ in 0..3 {
        rig.frame(vec![]);
        events.extend(rig.event.clone());
    }
    let commit = events.iter().position(|e| matches!(e, EditEvent::Commit { value, .. } if *value == 45.0));
    let set = events.iter().position(|e| matches!(e, EditEvent::Set { .. }));
    assert!(commit.is_some() && commit < set, "the typed commit, then the step: {events:?}");
    assert_eq!(rig.angle, 46.0, "{events:?}");
}
