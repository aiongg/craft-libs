//! The point picker in a harness.

use egui::accesskit::{Action, ActionData, ActionRequest, Node, NodeId, Role, TreeId};
use egui::{Context, Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, Vec2, vec2};

use super::PointPicker;
use crate::edit::{CancelReason, EditEvent};
use crate::theme::{Base, install};

struct Rig {
    ctx: Context,
    time: f64,
    point: [f64; 2],
    event: Option<EditEvent<[f64; 2]>>,
    rect: Rect,
    focus: egui::Id,
    nodes: Vec<(NodeId, Node)>,
}

impl Rig {
    fn new() -> Self {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        ctx.enable_accesskit();
        let mut rig = Self { ctx, time: 0.0, point: [50.0, 50.0], event: None, rect: Rect::NOTHING, focus: egui::Id::NULL, nodes: Vec::new() };
        rig.frame(vec![]);
        rig.frame(vec![]);
        rig
    }

    fn frame(&mut self, events: Vec<Event>) {
        let input =
            RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(600.0, 400.0))), time: Some(self.time), events, ..Default::default() };
        self.time += 1.0 / 60.0;
        let ctx = self.ctx.clone();
        let output = ctx.run_ui(input, |ui| {
            let edit = PointPicker::new("picker", "Blur Center", self.point, None).show(ui);
            self.rect = edit.response.rect;
            self.focus = edit.response.id;
            if let Some(value) = edit.event.as_ref().and_then(EditEvent::value) {
                self.point = *value;
            }
            self.event = edit.event;
        });
        if let Some(update) = output.platform_output.accesskit_update.clone() {
            self.nodes = update.nodes;
        }
        output.drop_without_applying_deltas();
    }

    /// The point `fraction` across the thumbnail.
    fn at(&self, fraction: Vec2) -> Pos2 {
        self.rect.min + self.rect.size() * fraction
    }

    fn value(&self) -> Option<[f64; 2]> {
        self.event.as_ref().and_then(EditEvent::value).copied()
    }
}

fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

fn pressed(key: Key, modifiers: Modifiers) -> Event {
    Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }
}

// A press on the thumbnail moves the point there and reports one Set; a drag reports Begin,
// Updates and a Commit, and Esc during it a Cancel.
#[test]
fn press_and_drag_edit_the_point_as_one_gesture() {
    let mut rig = Rig::new();
    let pos = rig.at(vec2(0.25, 0.75));
    rig.frame(vec![Event::PointerMoved(pos), button(pos, true), button(pos, false)]);
    assert!(matches!(rig.event, Some(EditEvent::Set { value, .. }) if value == [25.0, 75.0]), "{:?}", rig.event);

    let start = rig.at(vec2(0.5, 0.5));
    rig.frame(vec![Event::PointerMoved(start), button(start, true)]);
    let Some(EditEvent::Begin { id, .. }) = rig.event.clone() else { panic!("expected Begin, got {:?}", rig.event) };
    let to = rig.at(vec2(1.5, 0.1));
    rig.frame(vec![Event::PointerMoved(to)]);
    assert!(matches!(rig.event, Some(EditEvent::Update { id: u, value, .. }) if u == id && value == [100.0, 10.0]), "{:?}", rig.event);
    rig.frame(vec![button(to, false)]);
    assert!(matches!(rig.event, Some(EditEvent::Commit { id: c, value, changed: true, .. }) if c == id && value == [100.0, 10.0]), "{:?}", rig.event);

    rig.frame(vec![Event::PointerMoved(start), button(start, true)]);
    let Some(EditEvent::Begin { id, .. }) = rig.event.clone() else { panic!("expected Begin, got {:?}", rig.event) };
    rig.frame(vec![Event::PointerMoved(rig.at(vec2(0.2, 0.2)))]);
    rig.frame(vec![pressed(Key::Escape, Modifiers::NONE)]);
    assert_eq!(rig.event, Some(EditEvent::Cancel { id, reason: CancelReason::Escape }));
}

// Typing in the X field moves the cross; arrows on the focused thumbnail move the point by one
// unit, Shift by ten.
#[test]
fn fields_and_arrow_keys_move_the_point() {
    let mut rig = Rig::new();
    let focus = rig.focus;
    rig.ctx.memory_mut(|m| m.request_focus(focus));
    rig.frame(vec![]);
    rig.frame(vec![pressed(Key::ArrowRight, Modifiers::NONE)]);
    assert_eq!(rig.value(), Some([51.0, 50.0]));
    rig.frame(vec![pressed(Key::ArrowDown, Modifiers::SHIFT)]);
    assert_eq!(rig.value(), Some([51.0, 60.0]));

    let x = rig.nodes.iter().find(|(_, n)| n.role() == Role::SpinButton && n.label() == Some("Blur Center X")).map(|(id, _)| *id).expect("X field");
    let data = Some(ActionData::NumericValue(20.0));
    rig.frame(vec![Event::AccessKitActionRequest(ActionRequest { action: Action::SetValue, target_tree: TreeId::ROOT, target_node: x, data })]);
    assert_eq!(rig.value(), Some([20.0, 60.0]));
}
