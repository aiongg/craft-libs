//! Behaviour tests of `eyedropper`.

#![expect(clippy::todo, reason = "S4C-F13: the tool test below is a stub until the canvas tool lands")]

use egui::accesskit::{Action, ActionRequest, Role};
use egui::{Context, Event, Key, Modifiers, Pos2, RawInput, Rect, vec2};

use super::{EyedropperLabels, EyedropperOptions, EyedropperOptionsBar, SampleSize, SampleSource};
use craft_ui::patterns::OptionsBar;
use craft_ui::theme::{Base, install};

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

/// Alt-press picks for the background; a drag keeps sampling and the release keeps the last sample.
#[test]
#[ignore = "S4C-F13: needs EyedropperTool"]
fn alt_picks_the_background_and_a_drag_resamples() {
    todo!()
}
