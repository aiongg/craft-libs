//! Tests of the Ruler options bar (task TO-R3).

use egui::accesskit::{Action, ActionRequest, Node, NodeId, Role};
use egui::{Context, Event, Pos2, RawInput, Rect, vec2};

use super::{RulerBar, RulerBarOutput, RulerLine};
use craft_ui::patterns::OptionsBar;
use craft_ui::theme::{Base, install};
use kurbo::Point;

struct Rig {
    ctx: Context,
    line: Option<RulerLine>,
    use_scale: bool,
    factor: f64,
    output: RulerBarOutput,
    nodes: Vec<(NodeId, Node)>,
}

impl Rig {
    fn new(line: Option<RulerLine>, use_scale: bool, factor: f64) -> Rig {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        ctx.enable_accesskit();
        let mut rig = Rig { ctx, line, use_scale, factor, output: RulerBarOutput::default(), nodes: Vec::new() };
        rig.frame(vec![]);
        rig
    }

    fn frame(&mut self, events: Vec<Event>) {
        let input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1400.0, 100.0))), events, ..Default::default() };
        let ctx = self.ctx.clone();
        let (line, factor) = (self.line, self.factor);
        let use_scale = &mut self.use_scale;
        self.output = RulerBarOutput::default();
        let output = &mut self.output;
        let result = ctx.run_ui(input, |ui| {
            OptionsBar::new("ruler-bar").show(ui, |slots| RulerBar::new(line, use_scale).scale(factor).output(output).add_to(slots));
        });
        if let Some(update) = result.platform_output.accesskit_update.clone() {
            self.nodes = update.nodes;
        }
        result.drop_without_applying_deltas();
    }

    fn labelled(&self, role: Role, label: &str) -> Option<&Node> {
        self.nodes.iter().map(|(_, n)| n).find(|n| n.role() == role && n.label() == Some(label))
    }

    fn has_text(&self, text: &str) -> bool {
        self.nodes.iter().any(|(_, n)| n.label() == Some(text) || n.value() == Some(text))
    }

    fn click(&mut self, label: &str) {
        let Some((id, _)) = self.nodes.iter().find(|(_, n)| n.role() == Role::Button && n.label() == Some(label)) else {
            panic!("no button named {label}");
        };
        let request = ActionRequest { action: Action::Click, target_node: *id, target_tree: egui::accesskit::TreeId::ROOT, data: None };
        self.frame(vec![Event::AccessKitActionRequest(request)]);
    }
}

fn line() -> RulerLine {
    RulerLine::new(Point::new(1.0, 2.0), Point::new(4.0, 6.0))
}

// With Use Measurement Scale on and factor 2, the X, Y, W, H, L1 and L2 readouts double and A does not; values show two decimals.
#[test]
fn scale_multiplies_lengths_not_the_angle() {
    let mut with_protractor = line();
    with_protractor.protractor = Some(Point::new(1.0, 7.0));
    let reading = with_protractor.reading();
    let expected = |v: f64| format!("{v:.2}");

    let rig = Rig::new(Some(with_protractor), true, 2.0);
    for name in ["X", "Y", "W", "H", "A", "L1", "L2"] {
        assert!(rig.labelled(Role::Group, name).is_some(), "no readout {name}");
    }
    for v in [reading.x * 2.0, reading.y * 2.0, reading.w * 2.0, reading.h * 2.0, reading.l1 * 2.0, reading.l2.unwrap_or(0.0) * 2.0, reading.angle] {
        assert!(rig.has_text(&expected(v)), "no value {}", expected(v));
    }
    assert!(!rig.has_text(&expected(reading.angle * 2.0)));

    let off = Rig::new(Some(with_protractor), false, 2.0);
    assert!(off.has_text(&expected(reading.w)));
    assert!(!off.has_text(&expected(reading.w * 2.0)));
}

// Without a line the readouts are blank and Straighten Layer and Clear are disabled; with one, clicking them sets the output.
#[test]
fn buttons_are_disabled_without_a_line() {
    let mut none = Rig::new(None, false, 1.0);
    for name in ["Straighten Layer", "Clear"] {
        let Some(node) = none.labelled(Role::Button, name) else { panic!("no button {name}") };
        assert!(node.is_disabled());
    }
    assert!(!none.has_text("0.00"));
    none.click("Clear");
    assert_eq!(none.output, RulerBarOutput::default());

    let mut some = Rig::new(Some(line()), false, 1.0);
    some.click("Straighten Layer");
    assert_eq!(some.output, RulerBarOutput { straighten_layer: true, clear: false });
    some.click("Clear");
    assert_eq!(some.output, RulerBarOutput { straighten_layer: false, clear: true });
}
