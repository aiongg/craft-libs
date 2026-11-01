//! Tests of the brush Size row (task BR-BS2).
#![cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]

use egui::accesskit::{Action, ActionRequest, Node, NodeId, TreeId};
use egui::{Context, Event, Modifiers, PointerButton, Pos2, RawInput, Rect, vec2};

use super::BrushSizeField;
use craft_ui::edit::EditEvent;
use craft_ui::theme::{Base, install};

struct Rig {
    ctx: Context,
    time: f64,
    diameter: f64,
    restore: Option<f64>,
    max: f64,
    slider: Rect,
    events: Vec<EditEvent<f64>>,
    nodes: Vec<(NodeId, Node)>,
}

impl Rig {
    fn new(diameter: f64, restore: Option<f64>) -> Self {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        ctx.enable_accesskit();
        let mut rig = Self { ctx, time: 0.0, diameter, restore, max: 1000.0, slider: Rect::NOTHING, events: Vec::new(), nodes: Vec::new() };
        rig.frame(vec![]);
        rig.frame(vec![]);
        rig
    }

    fn frame(&mut self, events: Vec<Event>) {
        let input =
            RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(400.0, 300.0))), time: Some(self.time), events, ..Default::default() };
        self.time += 1.0 / 60.0;
        let ctx = self.ctx.clone();
        let output = ctx.run_ui(input, |ui| {
            let edit = BrushSizeField::new("size", "Size", self.diameter, self.max).restore(self.restore).show(ui);
            self.slider = edit.response.rect;
            // The host writes every value back, as the app does.
            if let Some(value) = edit.event.as_ref().and_then(EditEvent::value) {
                self.diameter = *value;
            }
            self.events.extend(edit.event);
        });
        if let Some(update) = output.platform_output.accesskit_update.clone() {
            self.nodes = update.nodes;
        }
        output.drop_without_applying_deltas();
    }

    fn button(pos: Pos2, pressed: bool) -> Event {
        Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
    }

    fn node_id(&self, label: &str) -> Option<NodeId> {
        self.nodes.iter().find(|(_, n)| n.label() == Some(label)).map(|(id, _)| *id)
    }
}

// A slider drag on the log track reports whole pixels as one gesture while the host writes the
// rounded value back each pass (gap 67); the field shows the same value.
#[test]
fn size_field_and_slider_agree_on_a_log_scale() {
    let mut rig = Rig::new(10.0, None);
    let press = rig.slider.center();
    rig.frame(vec![Event::PointerMoved(press), Rig::button(press, true)]);
    let to = press + vec2(rig.slider.width() / 4.0, 0.0);
    rig.frame(vec![Event::PointerMoved(to)]);
    rig.frame(vec![Rig::button(to, false)]);

    assert!(matches!(rig.events.first(), Some(EditEvent::Begin { .. })), "{:?}", rig.events);
    let Some(EditEvent::Commit { value, .. }) = rig.events.last() else { panic!("no commit: {:?}", rig.events) };
    // Halfway along a log track from 1 to 1000 is about 32; a quarter further is about 178.
    assert!((100.0..=260.0).contains(value), "{value}");
    for event in &rig.events {
        if let Some(v) = event.value() {
            assert_eq!(v.fract(), 0.0, "{event:?}");
        }
    }
    assert_eq!(rig.diameter, *value);
    // One gesture: every event after the Begin carries its id.
    let ids: Vec<_> = rig.events.iter().filter_map(EditEvent::id).collect();
    assert!(ids.windows(2).all(|w| w[0] == w[1]));
    // The field shows the slider's value.
    rig.frame(vec![]);
    let shown = rig.nodes.iter().find(|(_, n)| n.label() == Some("Size") && n.numeric_value().is_some_and(|v| v == rig.diameter));
    assert!(shown.is_some());
}

// With `restore(Some(80.0))` the button shows and reports `Set(80)`; values clamp to 1..=max.
#[test]
fn restore_sets_the_original_size() {
    let mut rig = Rig::new(10.0, None);
    assert!(rig.node_id("Restore original size").is_none());

    rig.restore = Some(80.0);
    rig.frame(vec![]);
    rig.frame(vec![]);
    let click = |rig: &mut Rig| {
        let target_node = rig.node_id("Restore original size").expect("the restore button");
        rig.events.clear();
        rig.frame(vec![Event::AccessKitActionRequest(ActionRequest { action: Action::Click, target_tree: TreeId::ROOT, target_node, data: None })]);
        rig.events.pop()
    };
    assert!(matches!(click(&mut rig), Some(EditEvent::Set { value, .. }) if value == 80.0));

    rig.restore = Some(5000.0);
    rig.diameter = 10.0;
    rig.frame(vec![]);
    assert!(matches!(click(&mut rig), Some(EditEvent::Set { value, .. }) if value == 1000.0));
}
