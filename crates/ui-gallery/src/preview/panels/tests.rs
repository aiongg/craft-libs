use craft_theme::{ModeChoice, SystemAppearance};
use craft_ui::inspect::AccessTree;
use craft_ui::theme::install;
use egui::accesskit::{Action, ActionRequest, NodeId, Role, TreeId};
use egui::{Context, Event, Key, Modifiers, RawInput, Rect, vec2};

use super::Panel;
use crate::Appearance;
use crate::preview::AppPreview;
use crate::preview::model::LayerKind;

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
fn the_window_menu_hides_layers_and_shows_them_again() {
    let mut fixture = Fixture::new();
    assert!(fixture.names(Role::ListBoxOption).contains(&"Shapes".to_owned()));
    fixture.click("Window", Role::MenuItem);
    fixture.click("Layers", Role::MenuItemCheckBox);
    assert!(!fixture.preview.dock.is_visible(Panel::Layers));
    assert!(!fixture.names(Role::ListBoxOption).contains(&"Shapes".to_owned()), "the Layers list is gone");
    fixture.click("Window", Role::MenuItem);
    fixture.click("Layers", Role::MenuItemCheckBox);
    assert!(fixture.preview.dock.is_visible(Panel::Layers));
    assert!(fixture.preview.dock.is_front(Panel::Layers));
    assert!(fixture.names(Role::ListBoxOption).contains(&"Shapes".to_owned()));
}

#[test]
fn a_tab_click_fronts_channels() {
    let mut fixture = Fixture::new();
    assert!(fixture.preview.dock.is_front(Panel::Layers));
    assert!(!fixture.preview.dock.is_front(Panel::Channels));
    fixture.click("Channels", Role::Tab);
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

impl Fixture {
    fn spin_button(&mut self, name: &str) -> NodeId {
        let tree = self.settled();
        let nodes: Vec<_> = tree
            .walk(tree.root())
            .into_iter()
            .map(|(id, _)| id)
            .filter(|id| tree.name(*id).as_deref() == Some(name) && tree.node(*id).is_some_and(|n| n.role() == Role::SpinButton))
            .collect();
        assert_eq!(nodes.len(), 1, "one number field named {name:?}");
        nodes[0]
    }

    /// Focuses the number field and replaces its text, without leaving it.
    fn type_keep(&mut self, name: &str, text: &str) {
        let target = self.spin_button(name);
        let request = ActionRequest { action: Action::Focus, target_tree: TreeId::ROOT, target_node: target, data: None };
        self.frame(vec![Event::AccessKitActionRequest(request)]);
        let select_all = Event::Key { key: Key::A, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::COMMAND };
        self.frame(vec![select_all]);
        self.frame(vec![Event::Text(text.to_owned())]);
    }

    /// Types into the number field and leaves it: a committed entry.
    fn type_into(&mut self, name: &str, text: &str) {
        self.type_keep(name, text);
        self.key(Key::Tab);
    }

    fn rect(&self, id: u64) -> [f32; 4] {
        self.preview.mock.doc.layer(id).expect("layer").rect
    }

    fn history(&self) -> usize {
        self.preview.mock.history.len()
    }
}

#[test]
fn width_with_the_link_on_keeps_the_ratio_while_it_is_typed() {
    let mut fixture = Fixture::new();
    let history = fixture.history();
    fixture.type_into("Width", "100");
    assert_eq!(fixture.rect(3), [500.0, 300.0, 100.0, 80.0], "typed digit by digit, the ratio is the starting one");
    assert_eq!(fixture.history(), history + 1, "one committed edit, one row");
    assert_eq!(fixture.preview.mock.history.last().map(String::as_str), Some("Transform"));
    assert!(fixture.preview.mock.doc.modified);
}

#[test]
fn width_with_the_link_off_changes_only_the_width() {
    let mut fixture = Fixture::new();
    fixture.click("Link width and height", Role::Button);
    fixture.type_into("Width", "250");
    assert_eq!(fixture.rect(3), [500.0, 300.0, 250.0, 400.0]);
}

#[test]
fn escape_after_typing_restores_the_rect_and_adds_no_row() {
    let mut fixture = Fixture::new();
    let history = fixture.history();
    fixture.type_keep("Width", "12");
    fixture.key(Key::Escape);
    assert_eq!(fixture.rect(3), [500.0, 300.0, 500.0, 400.0]);
    assert_eq!(fixture.history(), history);
}

#[test]
fn an_x_edit_moves_the_layer() {
    let mut fixture = Fixture::new();
    fixture.type_into("X", "120");
    assert_eq!(fixture.rect(3), [120.0, 300.0, 500.0, 400.0]);
    assert_eq!(fixture.preview.mock.history.last().map(String::as_str), Some("Move Layer"));
}

#[test]
fn a_locked_layers_fields_are_disabled() {
    let mut fixture = Fixture::new();
    fixture.preview.mock.doc.selected = vec![1];
    let width = fixture.spin_button("Width");
    let tree = fixture.settled();
    assert!(tree.node(width).is_some_and(|n| n.is_disabled()));
}

#[test]
fn an_align_button_puts_the_layers_edge_on_the_page() {
    let mut fixture = Fixture::new();
    // The Marquee bar has no align buttons, so the name finds the panel's alone.
    fixture.preview.mock.tools.current = crate::preview::model::Tool::RectMarquee;
    fixture.click("Align right edges", Role::Button);
    assert_eq!(fixture.rect(3), [700.0, 300.0, 500.0, 400.0]);
    assert_eq!(fixture.preview.mock.history.last().map(String::as_str), Some("Align"));
}

#[test]
fn the_move_bars_align_button_moves_the_layer() {
    let mut fixture = Fixture::new();
    // With Adjustments in front the Properties panel's buttons are not drawn.
    fixture.click("Adjustments", Role::Tab);
    fixture.click("Align bottom edges", Role::Button);
    assert_eq!(fixture.rect(3), [500.0, 400.0, 500.0, 400.0]);
}

#[test]
fn a_brightness_edit_changes_the_adjustment_and_the_layer_below() {
    let mut fixture = Fixture::new();
    fixture.click("New adjustment layer", Role::Button);
    let id = fixture.preview.mock.doc.selected[0];
    let below = fixture.preview.mock.doc.shown_color(1);
    fixture.type_into("Brightness", "50");
    let kind = &fixture.preview.mock.doc.layer(id).unwrap().kind;
    assert_eq!(*kind, LayerKind::Adjustment { brightness: 50, contrast: 0, clip: false });
    assert_ne!(fixture.preview.mock.doc.shown_color(1), below, "the colour of the layer below follows");
    assert_eq!(fixture.preview.mock.history.last().map(String::as_str), Some("Brightness/Contrast"));
}

#[test]
fn dragging_a_slider_adds_one_history_row() {
    let mut fixture = Fixture::new();
    fixture.click("New adjustment layer", Role::Button);
    let history = fixture.history();
    let tree = fixture.settled();
    let slider = tree
        .walk(tree.root())
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| tree.name(*id).as_deref() == Some("Brightness") && tree.node(*id).is_some_and(|n| n.role() == Role::Slider))
        .expect("the Brightness slider");
    let start = tree.bounds(slider).expect("slider bounds").center();
    let button = |pressed| Event::PointerButton { pos: start, button: egui::PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
    fixture.frame(vec![Event::PointerMoved(start)]);
    fixture.frame(vec![button(true)]);
    for step in 1..=3 {
        fixture.frame(vec![Event::PointerMoved(start + vec2(25.0 * step as f32, 0.0))]);
    }
    fixture.frame(vec![Event::PointerButton {
        pos: start + vec2(75.0, 0.0),
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    }]);
    fixture.settled();
    let id = fixture.preview.mock.doc.selected[0];
    let LayerKind::Adjustment { brightness, .. } = fixture.preview.mock.doc.layer(id).unwrap().kind else { panic!("an adjustment layer") };
    assert!(brightness > 0, "the drag moved the slider: {brightness}");
    assert_eq!(fixture.history(), history + 1);
}

#[test]
fn the_clip_checkbox_toggles_clip_and_adds_a_row() {
    let mut fixture = Fixture::new();
    fixture.click("New adjustment layer", Role::Button);
    let history = fixture.history();
    fixture.click("Clip to layer below", Role::CheckBox);
    let id = fixture.preview.mock.doc.selected[0];
    assert_eq!(fixture.preview.mock.doc.layer(id).unwrap().kind, LayerKind::Adjustment { brightness: 0, contrast: 0, clip: true });
    assert_eq!(fixture.history(), history + 1);
}

#[test]
fn an_adjustments_button_adds_a_layer_above_the_selection_and_shows_properties() {
    let mut fixture = Fixture::new();
    fixture.preview.mock.doc.selected = vec![2];
    fixture.click("Adjustments", Role::Tab);
    assert!(!fixture.preview.dock.is_front(Panel::Properties));
    fixture.click("Hue/Saturation", Role::Button);
    let doc = &fixture.preview.mock.doc;
    assert_eq!(doc.layers.len(), 4);
    assert_eq!(doc.layers[1].name, "Hue/Saturation", "above Sky");
    assert_eq!(doc.selected, [doc.layers[1].id]);
    assert!(fixture.preview.dock.is_front(Panel::Properties));
}

#[test]
fn several_selected_layers_say_so() {
    let mut fixture = Fixture::new();
    fixture.preview.mock.doc.selected = vec![2, 3];
    let tree = fixture.settled();
    assert!(tree.walk(tree.root()).into_iter().any(|(id, _)| tree.name(id).as_deref() == Some("2 layers selected")));
}
