use craft_theme::{ModeChoice, SystemAppearance};
use craft_ui::inspect::AccessTree;
use craft_ui::story::Args;
use craft_ui::theme::install;
use egui::accesskit::{Action, ActionRequest, NodeId, Role, TreeId};
use egui::{Context, Event, Key, Modifiers, RawInput, Rect, vec2};

use super::model::Tool;
use super::{AppPreview, stories};
use crate::Appearance;

/// A context with the gallery's theme installed, and a preview in the window the gallery gives
/// its panels.
struct Fixture {
    ctx: Context,
    appearance: Appearance,
    preview: AppPreview,
}

impl Fixture {
    fn new(theme: &str, mode: ModeChoice) -> Self {
        let ctx = Context::default();
        ctx.enable_accesskit();
        let appearance = Appearance { theme: theme.to_owned(), mode, ..Appearance::default() };
        appearance.configure(&ctx, None, false, SystemAppearance::default());
        Self { ctx, appearance, preview: AppPreview::default() }
    }

    fn frame(&mut self, events: Vec<Event>) -> Option<AccessTree> {
        let raw = RawInput { screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(1280.0, 800.0))), events, ..RawInput::default() };
        let base = self.appearance.base(SystemAppearance::default());
        let output = self.ctx.run_ui(raw, |ui| {
            install(ui.ctx(), &base);
            self.preview.show(ui);
        });
        let tree = output.platform_output.accesskit_update.as_ref().and_then(AccessTree::from_update);
        output.drop_without_applying_deltas();
        tree
    }

    fn settled(&mut self) -> AccessTree {
        for _ in 0..3 {
            self.frame(Vec::new());
        }
        self.frame(Vec::new()).expect("an AccessKit tree")
    }

    fn click(&mut self, name: &str, role: Role) {
        let tree = self.settled();
        let nodes = named(&tree, name, role);
        assert_eq!(nodes.len(), 1, "one {role:?} named {name:?}");
        let request = ActionRequest { action: Action::Click, target_tree: TreeId::ROOT, target_node: nodes[0], data: None };
        self.frame(vec![Event::AccessKitActionRequest(request)]);
        self.frame(Vec::new());
    }

    fn press(&mut self, key: Key) {
        self.settled();
        let event = |pressed| Event::Key { key, physical_key: None, pressed, repeat: false, modifiers: Modifiers::NONE };
        self.frame(vec![event(true), event(false)]);
        self.frame(Vec::new());
    }
}

fn named(tree: &AccessTree, name: &str, role: Role) -> Vec<NodeId> {
    tree.walk(tree.root())
        .into_iter()
        .map(|(id, _)| id)
        .filter(|id| tree.name(*id).as_deref() == Some(name) && tree.node(*id).is_some_and(|n| n.role() == role))
        .collect()
}

fn has_name(tree: &AccessTree, name: &str) -> bool {
    tree.walk(tree.root()).iter().any(|(id, _)| tree.name(*id).as_deref() == Some(name))
}

#[test]
fn the_story_renders_in_a_dark_and_a_light_theme() {
    let story = stories().into_iter().next().expect("the App preview story");
    for (theme, mode) in [("craft.artcraft", ModeChoice::Dark), ("craft.classic", ModeChoice::Light)] {
        let ctx = Context::default();
        ctx.enable_accesskit();
        let appearance = Appearance { theme: theme.to_owned(), mode, ..Appearance::default() };
        appearance.configure(&ctx, None, false, SystemAppearance::default());
        let mut args: Args = story.defaults();
        let mut tree = None;
        for _ in 0..4 {
            let raw = RawInput { screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(1280.0, 800.0))), ..RawInput::default() };
            let base = appearance.base(SystemAppearance::default());
            let output = ctx.run_ui(raw, |ui| {
                install(ui.ctx(), &base);
                crate::gallery::canvas(ui, &story, &mut args, &appearance, SystemAppearance::default(), None);
            });
            tree = output.platform_output.accesskit_update.as_ref().and_then(AccessTree::from_update);
            output.drop_without_applying_deltas();
        }
        let tree = tree.expect("an AccessKit tree");
        for label in ["Tools", "Tool options", "Status bar", "File"] {
            assert!(has_name(&tree, label), "{theme}: {label} is drawn");
        }
    }
}

#[test]
fn choosing_brush_on_the_rail_shows_flow_and_says_so_in_the_status_bar() {
    let mut fixture = Fixture::new("craft.artcraft", ModeChoice::Dark);
    let tree = fixture.settled();
    assert!(!has_name(&tree, "Flow"), "the Move tool has no Flow");
    fixture.click("Brush", Role::Button);
    assert_eq!(fixture.preview.mock.tools.current, Tool::Brush);
    assert_eq!(fixture.preview.mock.status, "Brush tool");
    let tree = fixture.settled();
    assert!(has_name(&tree, "Flow"), "the Brush options bar has Flow");
    assert!(has_name(&tree, "Brush tool"), "the status message is in the tree");
}

#[test]
fn a_tool_key_switches_the_tool_and_a_tool_without_a_key_binding_is_left_alone() {
    let mut fixture = Fixture::new("craft.artcraft", ModeChoice::Dark);
    fixture.press(Key::G);
    assert_eq!(fixture.preview.mock.tools.current, Tool::Gradient);
    assert_eq!(fixture.preview.mock.status, "Gradient tool");
    let tree = fixture.settled();
    assert!(has_name(&tree, "Reverse"), "the Gradient options bar has Reverse");
    fixture.press(Key::P);
    assert_eq!(fixture.preview.mock.tools.current, Tool::Gradient, "Pen does not work in the mock");
    fixture.press(Key::V);
    assert_eq!(fixture.preview.mock.tools.current, Tool::Move);
}
