//! Behaviour tests of `eyedropper`.

use egui::accesskit::{Action, ActionRequest, Role};
use egui::{Context, Event, Key, Modifiers, Pos2, RawInput, Rect, vec2};

use super::{EyedropperLabels, EyedropperOptions, EyedropperOptionsBar, EyedropperPick, EyedropperTool, SampleSize, SampleSource};
use craft_theme::Srgba;
use craft_ui::canvas::kurbo::Point;
use craft_ui::canvas::{CanvasTool, CanvasView, ToolHost, ToolInput};
use craft_ui::patterns::OptionsBar;
use craft_ui::theme::{Base, install};
use egui::Id;
use egui::Sense;
use egui_kittest::Harness;

/// The sample size menu lists Photoshop's seven sizes in order with their labels.
#[test]
fn sample_sizes_follow_photoshops_list() {
    let labels = EyedropperLabels::default();
    let shown: Vec<_> = SampleSize::ALL.iter().map(|s| (s.pixels(), s.label(&labels))).collect();
    let want = [
        (1, "Point Sample"),
        (3, "3 by 3 Average"),
        (5, "5 by 5 Average"),
        (11, "11 by 11 Average"),
        (31, "31 by 31 Average"),
        (51, "51 by 51 Average"),
        (101, "101 by 101 Average"),
    ];
    assert_eq!(shown, want.map(|(n, text)| (n, text.to_owned())));
    let sources: Vec<_> = SampleSource::ALL.iter().map(|s| (s.id(), s.label(&labels))).collect();
    assert_eq!(
        sources,
        [
            ("current", "Current Layer"),
            ("currentAndBelow", "Current & Below"),
            ("all", "All Layers"),
            ("allNoAdjustments", "All Layers No Adjustments"),
            ("currentAndBelowNoAdjustments", "Current & Below No Adjustments"),
        ]
        .map(|(id, text)| (id, text.to_owned()))
    );
}

struct Rig {
    ctx: Context,
    options: EyedropperOptions,
    time: f64,
    nodes: Vec<(egui::accesskit::NodeId, egui::accesskit::Node)>,
}

impl Rig {
    fn new() -> Rig {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        ctx.enable_accesskit();
        let mut rig = Rig { ctx, options: EyedropperOptions::default(), time: 0.0, nodes: Vec::new() };
        rig.frame(vec![]);
        rig
    }

    fn frame(&mut self, events: Vec<Event>) {
        let input =
            RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(900.0, 100.0))), time: Some(self.time), events, ..Default::default() };
        self.time += 1.0 / 60.0;
        let ctx = self.ctx.clone();
        let options = &mut self.options;
        let output = ctx.run_ui(input, |ui| {
            OptionsBar::new("eyedropper-bar").show(ui, |slots| EyedropperOptionsBar::new(options).add_to(slots));
        });
        if let Some(update) = output.platform_output.accesskit_update.clone() {
            self.nodes = update.nodes;
        }
        output.drop_without_applying_deltas();
    }

    fn node(&self, role: Role, label: &str) -> egui::accesskit::NodeId {
        self.nodes.iter().find(|(_, n)| n.role() == role && n.label() == Some(label)).unwrap_or_else(|| panic!("no {role:?} named {label}")).0
    }

    fn act(&mut self, target: egui::accesskit::NodeId, action: Action) {
        let request = ActionRequest { action, target_node: target, target_tree: egui::accesskit::TreeId::ROOT, data: None };
        self.frame(vec![Event::AccessKitActionRequest(request)]);
    }

    fn press(&mut self, key: Key) {
        for pressed in [true, false] {
            self.frame(vec![Event::Key { key, physical_key: None, pressed, repeat: false, modifiers: Modifiers::NONE }]);
        }
    }
}

/// The options bar edits size, source and the ring box.
#[test]
fn the_options_bar_edits_the_options() {
    let mut rig = Rig::new();
    assert_eq!(rig.options, EyedropperOptions { size: SampleSize::Point, source: SampleSource::All, show_ring: true });

    let ring = rig.node(Role::CheckBox, "Show Sampling Ring");
    rig.act(ring, Action::Click);
    assert!(!rig.options.show_ring);

    let size = rig.node(Role::ComboBox, "Size");
    rig.act(size, Action::Focus);
    rig.frame(vec![]);
    rig.press(Key::ArrowDown);
    assert_eq!(rig.options.size, SampleSize::Avg3);

    let source = rig.node(Role::ComboBox, "Source");
    rig.act(source, Action::Focus);
    rig.frame(vec![]);
    rig.press(Key::ArrowUp);
    assert_eq!(rig.options.source, SampleSource::CurrentAndBelow);
}

/// A canvas with a tool host over a 400 x 300 view at zoom 1 whose centre is document (200, 150),
/// so document points equal screen points.
struct Canvas<T> {
    tool: T,
    id: craft_ui::canvas::CanvasId,
}

fn harness<T: CanvasTool + 'static>(tool: T) -> Harness<'static, Canvas<T>> {
    let mut h = Harness::builder().with_size(vec2(400.0, 300.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, rig: &mut Canvas<T>| {
            let view = CanvasView::new(Rect::from_min_size(Pos2::ZERO, vec2(400.0, 300.0)), Point::new(200.0, 150.0), 1.0);
            let canvas_id = craft_ui::canvas::CanvasId::new(ui, "canvas");
            let canvas = canvas_id.interact(ui, view.rect, Sense::click_and_drag());
            rig.id = ToolHost::new(canvas_id).show(ui, &view, &canvas, &mut rig.tool).id;
        },
        Canvas { tool, id: craft_ui::canvas::CanvasId::from_id(Id::NULL) },
    );
    h.step();
    h
}

fn input<T>(h: &mut Harness<'_, Canvas<T>>, input: ToolInput) {
    ToolHost::inject(&h.ctx, h.state().id, input);
    h.step();
}

fn point(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn down<T>(h: &mut Harness<'_, Canvas<T>>, at: Point, modifiers: Modifiers) {
    input(h, ToolInput::Down { button: egui::PointerButton::Primary, doc: at, pressure: 1.0, tilt: [0.0; 2], rotation: 0.0, modifiers });
}

fn move_to<T>(h: &mut Harness<'_, Canvas<T>>, at: Point, modifiers: Modifiers) {
    input(h, ToolInput::Move { doc: at, pressure: 1.0, tilt: [0.0; 2], rotation: 0.0, modifiers });
}

fn up<T>(h: &mut Harness<'_, Canvas<T>>, at: Point, modifiers: Modifiers) {
    input(h, ToolInput::Up { doc: at, modifiers });
}

/// The sample is a grey that follows the document x.
fn grey(x: f64) -> Srgba {
    Srgba::rgb(x as f32 / 400.0, 0.0, 0.0)
}

/// Alt-press picks for the background; a drag keeps sampling and the release keeps the last sample.
#[test]
fn alt_picks_the_background_and_a_drag_resamples() {
    let tool = EyedropperTool::new(EyedropperOptions::default(), |doc, _, _| (doc[0] < 300.0).then(|| grey(doc[0])));
    let mut h = harness(tool);
    let alt = Modifiers::ALT;

    down(&mut h, point(100.0, 50.0), alt);
    assert!(h.state().tool.ring_shown(), "the ring shows while the button is held");
    move_to(&mut h, point(200.0, 50.0), alt);
    assert_eq!(h.state_mut().tool.take(), Some(EyedropperPick { color: grey(200.0), background: true }), "the drag resampled");
    // Over transparency or off the image: no ring, the last pick stands.
    move_to(&mut h, point(350.0, 50.0), alt);
    assert!(!h.state().tool.ring_shown());
    assert_eq!(h.state_mut().tool.take(), None, "nothing new was picked");
    move_to(&mut h, point(250.0, 50.0), alt);
    up(&mut h, point(250.0, 50.0), alt);
    assert!(!h.state().tool.ring_shown(), "the ring goes with the button");
    assert_eq!(h.state_mut().tool.take(), Some(EyedropperPick { color: grey(250.0), background: true }));
    assert_eq!(h.state_mut().tool.take(), None, "a pick is taken once");

    // A plain click is the foreground.
    down(&mut h, point(40.0, 50.0), Modifiers::NONE);
    up(&mut h, point(40.0, 50.0), Modifiers::NONE);
    assert_eq!(h.state_mut().tool.take(), Some(EyedropperPick { color: grey(40.0), background: false }));
}
