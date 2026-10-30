use craft_theme::{ModeChoice, SystemAppearance};
use craft_ui::inspect::AccessTree;
use craft_ui::theme::install;
use egui::accesskit::{Action, ActionRequest, NodeId, Role, TreeId};
use egui::{Context, Event, Key, Modifiers, RawInput, Rect, vec2};

use super::Panel;
use crate::Appearance;
use crate::preview::AppPreview;

/// A context with the gallery's theme installed, and a preview in the window the gallery gives
/// its panels.
struct Fixture {
    ctx: Context,
    appearance: Appearance,
    preview: AppPreview,
}

impl Fixture {
    fn new() -> Self {
        let ctx = Context::default();
        ctx.enable_accesskit();
        let appearance = Appearance { theme: "craft.artcraft".to_owned(), mode: ModeChoice::Dark, ..Appearance::default() };
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

    fn act(&mut self, action: Action, name: &str, role: Role) {
        let tree = self.settled();
        let nodes = named(&tree, name, role);
        assert_eq!(nodes.len(), 1, "one {role:?} named {name:?}");
        let request = ActionRequest { action, target_tree: TreeId::ROOT, target_node: nodes[0], data: None };
        self.frame(vec![Event::AccessKitActionRequest(request)]);
        self.frame(Vec::new());
    }

    fn click(&mut self, name: &str, role: Role) {
        self.act(Action::Click, name, role);
    }

    fn key(&mut self, key: Key) {
        let event = |pressed| Event::Key { key, physical_key: None, pressed, repeat: false, modifiers: Modifiers::NONE };
        self.frame(vec![event(true), event(false)]);
        self.frame(Vec::new());
    }

    /// Selects the layer, gives the list the keyboard and presses F2.
    fn rename_key(&mut self, layer: &str) {
        self.click(layer, Role::ListBoxOption);
        self.act(Action::Focus, "Layers", Role::ListBox);
        self.key(Key::F2);
    }

    fn names(&mut self, role: Role) -> Vec<String> {
        let tree = self.settled();
        tree.walk(tree.root())
            .into_iter()
            .filter(|(id, _)| tree.node(*id).is_some_and(|n| n.role() == role))
            .filter_map(|(id, _)| tree.name(id))
            .collect()
    }

    fn layer_names(&self) -> Vec<&str> {
        self.preview.mock.doc.layers.iter().map(|l| l.name.as_str()).collect()
    }
}

fn named(tree: &AccessTree, name: &str, role: Role) -> Vec<NodeId> {
    tree.walk(tree.root())
        .into_iter()
        .map(|(id, _)| id)
        .filter(|id| tree.name(*id).as_deref() == Some(name) && tree.node(*id).is_some_and(|n| n.role() == role))
        .collect()
}

#[test]
fn the_eye_column_hides_and_shows_a_layer() {
    let mut fixture = Fixture::new();
    assert!(!fixture.preview.mock.doc.layers[1].visible, "Sky starts hidden");
    fixture.click("Visibility Sky", Role::Button);
    assert!(fixture.preview.mock.doc.layers[1].visible);
    fixture.click("Visibility Sky", Role::Button);
    assert!(!fixture.preview.mock.doc.layers[1].visible);
}

#[test]
fn f2_renames_the_selected_layer_and_a_locked_layer_refuses() {
    let mut fixture = Fixture::new();
    fixture.rename_key("Shapes");
    fixture.frame(vec![Event::Text("Hero".to_owned())]);
    fixture.key(Key::Enter);
    assert_eq!(fixture.layer_names(), ["Hero", "Sky", "Background"]);
    fixture.rename_key("Background");
    fixture.frame(vec![Event::Text("Floor".to_owned())]);
    fixture.key(Key::Enter);
    assert_eq!(fixture.layer_names(), ["Hero", "Sky", "Background"], "a locked layer keeps its name");
}

#[test]
fn the_footer_adds_a_layer_above_the_selection_and_selects_it() {
    let mut fixture = Fixture::new();
    fixture.click("New layer", Role::Button);
    let doc = &fixture.preview.mock.doc;
    assert_eq!(doc.layers.len(), 4);
    assert_eq!(doc.selected, [doc.layers[0].id], "the new layer is selected");
    assert_eq!(doc.layers[0].name, "Layer 4");
    let rows = fixture.names(Role::ListBoxOption);
    assert_eq!(rows, ["Layer 4", "Shapes", "Sky", "Background"]);
}

#[test]
fn a_swatch_sets_the_foreground() {
    let mut fixture = Fixture::new();
    fixture.click("Red", Role::RadioButton);
    assert_eq!(fixture.preview.mock.colors.fg, [237, 28, 36]);
    assert_eq!(fixture.preview.mock.colors.bg, [255, 255, 255]);
}

#[test]
fn the_window_menu_hides_layers_and_the_panel_rail_brings_it_back() {
    let mut fixture = Fixture::new();
    assert!(fixture.names(Role::ListBoxOption).contains(&"Shapes".to_owned()));
    fixture.click("Window", Role::MenuItem);
    fixture.click("Layers", Role::MenuItemCheckBox);
    assert!(!fixture.preview.dock.is_visible(Panel::Layers));
    assert!(!fixture.names(Role::ListBoxOption).contains(&"Shapes".to_owned()), "the Layers list is gone");
    fixture.click("Layers", Role::Button);
    assert!(fixture.preview.dock.is_visible(Panel::Layers));
    assert!(fixture.preview.dock.is_front(Panel::Layers));
    assert!(fixture.names(Role::ListBoxOption).contains(&"Shapes".to_owned()));
}

#[test]
fn a_panel_rail_click_fronts_channels() {
    let mut fixture = Fixture::new();
    assert!(fixture.preview.dock.is_front(Panel::Layers));
    assert!(!fixture.preview.dock.is_front(Panel::Channels));
    fixture.click("Channels", Role::Button);
    assert!(fixture.preview.dock.is_front(Panel::Channels));
    assert!(!fixture.preview.dock.is_front(Panel::Layers));
    assert!(fixture.names(Role::ListBoxOption).contains(&"Green".to_owned()));
}

#[test]
fn history_records_one_row_per_committed_layer_property_change() {
    let mut fixture = Fixture::new();
    let before = fixture.preview.mock.history.len();
    fixture.click("Lock all", Role::Button);
    assert!(fixture.preview.mock.doc.layers[0].locked);
    let history = &fixture.preview.mock.history;
    assert_eq!(history.len(), before + 1);
    assert_eq!(history.last().map(String::as_str), Some("Lock Layers"));
}
