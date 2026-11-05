//! Tests of the dynamic control row (task BR-BS5).
#![cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]

use egui::accesskit::{Action, ActionData, ActionRequest, Node, NodeId, Role, Toggled, TreeId};
use egui::{Context, Event, Key, Modifiers, Pos2, RawInput, Rect, vec2};

use super::{DynamicControl, DynamicControlRow, DynamicControlRowResponse};
use craft_ui::edit::EditEvent;
use craft_ui::theme::{Base, install};

#[derive(Clone, Copy, Debug, PartialEq)]
enum Source {
    Off,
    Fade,
    Direction,
}

struct Rig {
    ctx: Context,
    time: f64,
    current: Source,
    open: bool,
    shaped: bool,
    nodes: Vec<(NodeId, Node)>,
    fade: Vec<EditEvent<f64>>,
    picked: Vec<Source>,
    toggled: u32,
}

impl Rig {
    fn new(current: Source) -> Self {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        ctx.enable_accesskit();
        let mut rig =
            Self { ctx, time: 0.0, current, open: false, shaped: false, nodes: Vec::new(), fade: Vec::new(), picked: Vec::new(), toggled: 0 };
        rig.frame(vec![]);
        rig.frame(vec![]);
        rig
    }

    fn frame(&mut self, events: Vec<Event>) {
        let input =
            RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(500.0, 200.0))), time: Some(self.time), events, ..Default::default() };
        self.time += 1.0 / 60.0;
        let ctx = self.ctx.clone();
        let controls = [
            DynamicControl::new(Source::Off, "Off"),
            DynamicControl::new(Source::Fade, "Fade").fade().curve(),
            DynamicControl::new(Source::Direction, "Direction"),
        ];
        let output = ctx.run_ui(input, |ui| {
            let DynamicControlRowResponse { control, fade_steps, curve_toggled, .. } =
                DynamicControlRow::new("row", &controls, &self.current).curve(self.open, self.shaped).show(ui);
            if let Some(control) = control {
                self.current = control;
                self.picked.push(control);
            }
            self.fade.extend(fade_steps);
            self.toggled += u32::from(curve_toggled);
        });
        if let Some(update) = output.platform_output.accesskit_update.clone() {
            self.nodes = update.nodes;
        }
        output.drop_without_applying_deltas();
    }

    fn node(&self, label: &str) -> Option<&Node> {
        self.nodes.iter().find(|(_, n)| n.label() == Some(label)).map(|(_, n)| n)
    }

    fn request(&mut self, label: &str, action: Action, data: Option<ActionData>) {
        let Some(target_node) = self.nodes.iter().find(|(_, n)| n.label() == Some(label)).map(|(id, _)| *id) else {
            panic!("no node {label}");
        };
        self.frame(vec![Event::AccessKitActionRequest(ActionRequest { action, target_tree: TreeId::ROOT, target_node, data })]);
    }
}

// The Fade steps field shows only while the current entry is `fade`; a typed 30 reports Set(30).
#[test]
fn dynamic_row_shows_fade_steps_only_for_fade() {
    let mut rig = Rig::new(Source::Off);
    assert!(rig.node("Fade steps").is_none());
    rig.current = Source::Direction;
    rig.frame(vec![]);
    assert!(rig.node("Fade steps").is_none());

    rig.current = Source::Fade;
    rig.frame(vec![]);
    rig.frame(vec![]);
    assert!(rig.node("Fade steps").is_some());
    rig.request("Fade steps", Action::SetValue, Some(ActionData::NumericValue(30.0)));
    let Some(EditEvent::Set { value, .. }) = rig.fade.last() else { panic!("no Set: {:?}", rig.fade) };
    assert_eq!(*value, 30.0);
}

// The curve toggle shows only for `curve` entries and is `toggled` while the curve is open or
// shaped (AccessKit toggled); a click reports `curve_toggled`.
#[test]
fn curve_toggle_lights_when_the_curve_is_not_linear() {
    let mut rig = Rig::new(Source::Direction);
    assert!(rig.node("Response curve").is_none());

    rig.current = Source::Fade;
    rig.frame(vec![]);
    rig.frame(vec![]);
    let selected = |rig: &Rig| rig.node("Response curve").map(|n| n.toggled());
    assert_eq!(selected(&rig), Some(Some(Toggled::False)));
    rig.shaped = true;
    rig.frame(vec![]);
    assert_eq!(selected(&rig), Some(Some(Toggled::True)));
    rig.shaped = false;
    rig.open = true;
    rig.frame(vec![]);
    assert_eq!(selected(&rig), Some(Some(Toggled::True)));

    assert_eq!(rig.toggled, 0);
    rig.request("Response curve", Action::Click, None);
    assert_eq!(rig.toggled, 1);
}

// Picking another entry in the select reports `control`.
#[test]
fn picking_a_control_reports_it() {
    let mut rig = Rig::new(Source::Off);
    rig.request("Control:", Action::Focus, None);
    rig.frame(vec![]);
    rig.frame(vec![Event::Key { key: Key::ArrowDown, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE }]);
    assert_eq!(rig.picked, vec![Source::Fade]);
    assert_eq!(rig.node("Control:").map(Node::role), Some(Role::ComboBox));
}
