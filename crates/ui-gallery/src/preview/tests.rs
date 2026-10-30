use craft_theme::{ModeChoice, SystemAppearance};
use craft_ui::inspect::AccessTree;
use craft_ui::story::Args;
use craft_ui::theme::install;
use egui::accesskit::{Action, ActionRequest, NodeId, Role, TreeId};
use egui::{Context, Event, Key, Modifiers, RawInput, Rect, vec2};

use super::model::Tool;
use super::panels::Panel;
use super::{AppPreview, ShellView, stories};
use crate::Appearance;

/// A context with the gallery's theme installed, and a preview in the window the gallery gives
/// its panels.
struct Fixture {
    ctx: Context,
    appearance: Appearance,
    preview: AppPreview,
    view: ShellView,
    key: Option<craft_ui::shell::WindowKey>,
    time: f64,
}

impl Fixture {
    fn new(theme: &str, mode: ModeChoice) -> Self {
        let ctx = Context::default();
        ctx.enable_accesskit();
        let appearance = Appearance { theme: theme.to_owned(), mode, ..Appearance::default() };
        appearance.configure(&ctx, None, false, SystemAppearance::default());
        Self { ctx, appearance, preview: AppPreview::default(), view: ShellView::default(), key: None, time: 0.0 }
    }

    fn frame(&mut self, events: Vec<Event>) -> Option<AccessTree> {
        let mut raw = RawInput {
            screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(1280.0, 800.0))),
            events,
            time: Some(self.time),
            ..RawInput::default()
        };
        AppPreview::raw_input_hook(&self.ctx, &mut raw);
        let base = self.appearance.base(SystemAppearance::default());
        let output = self.ctx.run_ui(raw, |ui| {
            install(ui.ctx(), &base);
            self.key = Some(AppPreview::window_key(ui));
            self.preview.show_with(ui, self.view);
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
        self.act(Action::Click, name, role);
    }

    fn act(&mut self, action: Action, name: &str, role: Role) {
        let tree = self.settled();
        let nodes = named(&tree, name, role);
        assert_eq!(nodes.len(), 1, "one {role:?} named {name:?}");
        let request = ActionRequest { action, target_tree: TreeId::ROOT, target_node: nodes[0], data: None };
        self.frame(vec![Event::AccessKitActionRequest(request)]);
        self.frame(Vec::new());
    }

    /// A real primary click on the centre of the node, pressed and released in one frame.
    fn pointer_click(&mut self, name: &str, role: Role) {
        let tree = self.settled();
        let nodes = named(&tree, name, role);
        assert_eq!(nodes.len(), 1, "one {role:?} named {name:?}");
        let bounds = tree.node(nodes[0]).and_then(|n| n.bounds()).expect("bounds");
        let ppp = self.ctx.pixels_per_point();
        let pos = egui::pos2(((bounds.x0 + bounds.x1) / 2.0) as f32 / ppp, ((bounds.y0 + bounds.y1) / 2.0) as f32 / ppp);
        self.pointer_click_at(pos);
    }

    fn pointer_click_at(&mut self, pos: egui::Pos2) {
        self.time += 1.0;
        let button = |pressed| Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
        self.frame(vec![Event::PointerMoved(pos)]);
        self.frame(vec![button(true), button(false)]);
        self.frame(Vec::new());
        self.frame(Vec::new());
    }

    fn press(&mut self, key: Key) {
        self.press_with(key, Modifiers::NONE);
    }

    fn press_with(&mut self, key: Key, modifiers: Modifiers) {
        self.settled();
        let event = |pressed| Event::Key { key, physical_key: None, pressed, repeat: false, modifiers };
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

#[test]
fn a_modified_key_leaves_the_tool_alone() {
    let mut fixture = Fixture::new("craft.artcraft", ModeChoice::Dark);
    fixture.preview.mock.select_tool(Tool::Brush);
    fixture.press_with(Key::V, Modifiers::ALT);
    fixture.press_with(Key::V, Modifiers::COMMAND);
    assert_eq!(fixture.preview.mock.tools.current, Tool::Brush);
}

#[test]
fn a_letter_with_a_menu_open_does_not_switch_the_tool() {
    let mut fixture = Fixture::new("craft.artcraft", ModeChoice::Dark);
    fixture.preview.mock.select_tool(Tool::Brush);
    let tree = fixture.settled();
    let titles: Vec<_> = tree.walk(tree.root()).into_iter().map(|(id, _)| id).filter(|id| tree.name(*id).as_deref() == Some("Layer")).collect();
    let role = tree.node(titles[0]).expect("the title").role();
    fixture.click("Layer", role);
    fixture.settled();
    assert!(craft_ui::menu::any_open(&fixture.ctx) || craft_ui::menu::bar::is_open(&fixture.ctx), "the Layer menu is open");
    fixture.press(Key::V);
    assert_eq!(fixture.preview.mock.tools.current, Tool::Brush);
}

#[test]
fn a_letter_after_picking_a_select_value_switches_the_tool() {
    let mut fixture = Fixture::new("craft.artcraft", ModeChoice::Dark);
    fixture.click("Auto-Select target", Role::ComboBox);
    fixture.click("Group", Role::ListBoxOption);
    assert_eq!(fixture.preview.mock.tools.move_.target, super::model::MoveTarget::Group);
    fixture.press(Key::B);
    assert_eq!(fixture.preview.mock.tools.current, Tool::Brush);
}

#[test]
fn a_letter_in_a_focused_text_field_does_not_switch_the_tool() {
    let mut fixture = Fixture::new("craft.artcraft", ModeChoice::Dark);
    fixture.act(Action::Focus, "Zoom", Role::SpinButton);
    assert!(fixture.ctx.text_edit_focused(), "the zoom field is being edited");
    fixture.press(Key::B);
    assert_eq!(fixture.preview.mock.tools.current, Tool::Move);
}

#[test]
fn history_starts_as_an_icon_and_the_other_panels_as_tabs_with_no_panel_rail() {
    let mut fixture = Fixture::new("craft.artcraft", ModeChoice::Dark);
    let tree = fixture.settled();
    assert!(!has_name(&tree, "Panels"), "no panel rail toolbar");
    assert!(fixture.preview.dock.is_visible(Panel::History));
    assert!(fixture.preview.dock.is_iconic(Panel::History));
    for panel in [Panel::Swatches, Panel::Properties, Panel::Adjustments, Panel::Layers, Panel::Channels] {
        assert!(!fixture.preview.dock.is_iconic(panel), "{panel:?} is in the expanded column");
        assert!(has_name(&tree, panel.label()), "{panel:?} has a tab");
    }
    assert_eq!(named(&tree, "History", Role::Button).len(), 1, "History's icon");
    assert!(named(&tree, "Open", Role::ListBoxOption).is_empty(), "History's rows are not drawn until the flyout opens");
}

#[test]
fn a_click_on_the_history_icon_opens_its_flyout_and_a_click_on_the_canvas_closes_it() {
    let mut fixture = Fixture::new("craft.artcraft", ModeChoice::Dark);
    let rows = |fixture: &mut Fixture| !named(&fixture.settled(), "Open", Role::ListBoxOption).is_empty();
    assert!(!rows(&mut fixture));
    fixture.pointer_click("History", Role::Button);
    assert!(rows(&mut fixture), "the flyout shows History's rows");
    fixture.pointer_click_at(egui::pos2(500.0, 400.0));
    assert!(!rows(&mut fixture), "a press outside closes the flyout");
}

#[test]
fn choosing_history_in_the_window_menu_with_real_clicks_closes_its_open_flyout() {
    let mut fixture = Fixture::new("craft.artcraft", ModeChoice::Dark);
    let rows = |fixture: &mut Fixture| !named(&fixture.settled(), "Open", Role::ListBoxOption).is_empty();
    fixture.pointer_click("History", Role::Button);
    assert!(rows(&mut fixture), "the flyout is open");
    // The press on the menu title is not a click outside the flyout.
    fixture.pointer_click("Window", Role::MenuItem);
    assert!(rows(&mut fixture), "opening the menu leaves the flyout open");
    fixture.pointer_click("History", Role::MenuItemCheckBox);
    assert!(!rows(&mut fixture), "choosing History closes the open flyout");
    assert!(fixture.preview.dock.is_visible(Panel::History), "the check stays on");
}

#[test]
fn the_window_menu_opens_the_flyout_of_a_collapsed_panel_and_keeps_it_shown() {
    let mut fixture = Fixture::new("craft.artcraft", ModeChoice::Dark);
    fixture.click("Window", Role::MenuItem);
    fixture.click("History", Role::MenuItemCheckBox);
    assert!(fixture.preview.dock.is_visible(Panel::History));
    assert!(!named(&fixture.settled(), "Open", Role::ListBoxOption).is_empty(), "the flyout shows History's rows");
}

#[test]
fn the_expanded_column_is_as_wide_as_the_dock_width_beside_the_iconic_column() {
    let mut fixture = Fixture::new("craft.artcraft", ModeChoice::Dark);
    let tree = fixture.settled();
    let bounds =
        |name: &str| -> Vec<_> { named(&tree, name, Role::Button).into_iter().filter_map(|id| tree.node(id).and_then(|n| n.bounds())).collect() };
    let end = bounds("Collapse column to icons").pop().expect("the expanded column's collapse button");
    // The iconic column has a move handle too; the expanded column's is the last one.
    let start = bounds("Move panel column").into_iter().max_by(|a, b| a.x0.total_cmp(&b.x0)).expect("a move handle");
    let ppp = f64::from(fixture.ctx.pixels_per_point());
    assert_eq!((end.x1 - start.x0) / ppp, f64::from(super::dock::WIDTH));
}

#[test]
fn choosing_a_shown_iconic_panel_in_the_window_menu_opens_and_closes_its_flyout_and_keeps_it_shown() {
    let mut fixture = Fixture::new("craft.artcraft", ModeChoice::Dark);
    let rows = |fixture: &mut Fixture| !named(&fixture.settled(), "Open", Role::ListBoxOption).is_empty();
    for expected in [true, false, true] {
        fixture.preview.dock.toggle(Panel::History);
        assert_eq!(rows(&mut fixture), expected, "the flyout is open: {expected}");
        assert!(fixture.preview.dock.is_visible(Panel::History), "the check stays on");
    }
}

#[test]
fn choosing_a_hidden_iconic_panel_shows_it_with_its_flyout_open() {
    let mut fixture = Fixture::new("craft.artcraft", ModeChoice::Dark);
    fixture.settled();
    fixture.preview.dock.hide(Panel::History);
    assert!(!fixture.preview.dock.is_visible(Panel::History));
    assert!(named(&fixture.settled(), "Open", Role::ListBoxOption).is_empty());
    fixture.preview.dock.toggle(Panel::History);
    assert!(fixture.preview.dock.is_visible(Panel::History));
    assert!(!named(&fixture.settled(), "Open", Role::ListBoxOption).is_empty(), "the flyout shows History's rows");
}

#[test]
fn choosing_an_expanded_panel_in_the_window_menu_hides_and_shows_it() {
    let mut fixture = Fixture::new("craft.artcraft", ModeChoice::Dark);
    fixture.settled();
    fixture.preview.dock.toggle(Panel::Layers);
    assert!(!fixture.preview.dock.is_visible(Panel::Layers));
    fixture.preview.dock.toggle(Panel::Layers);
    assert!(fixture.preview.dock.is_visible(Panel::Layers));
}

#[test]
fn tab_toggles_the_reveal_in_a_body_only_mode_while_no_control_has_focus() {
    let mut fixture = Fixture::new("craft.artcraft", ModeChoice::Dark);
    fixture.view.mode = craft_ui::shell::ScreenMode::FullScreen;
    let revealed = |f: &Fixture| craft_ui::shell::AppShell::reveal_active(&f.ctx, f.key.as_ref().expect("a frame ran"));
    fixture.settled();
    fixture.pointer_click("Document", Role::Canvas);
    fixture.press(Key::Tab);
    assert!(revealed(&fixture), "Tab on the canvas shows the chrome");
    assert!(has_name(&fixture.settled(), "Tools"), "the tool rail shows in the reveal");
    fixture.press(Key::Tab);
    assert!(!revealed(&fixture), "a second Tab hides it");
    assert!(!has_name(&fixture.settled(), "Tools"));
}

#[test]
fn tab_is_not_taken_after_leaving_a_body_only_preview() {
    let mut fixture = Fixture::new("craft.artcraft", ModeChoice::Dark);
    fixture.view.mode = craft_ui::shell::ScreenMode::FullScreen;
    fixture.settled();
    let tab = || RawInput { events: vec![Event::Key { key: Key::Tab, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE }], ..RawInput::default() };
    let mut raw = tab();
    AppPreview::raw_input_hook(&fixture.ctx, &mut raw);
    assert!(raw.events.is_empty(), "Tab is taken while a body-only preview is shown");
    fixture.view.mode = craft_ui::shell::ScreenMode::Standard;
    fixture.settled();
    let mut raw = tab();
    AppPreview::raw_input_hook(&fixture.ctx, &mut raw);
    assert_eq!(raw.events.len(), 1, "Tab is left to egui once no body-only preview is shown");
}

#[test]
fn tab_moves_the_focus_off_the_canvas_in_standard_mode() {
    let mut fixture = Fixture::new("craft.artcraft", ModeChoice::Dark);
    fixture.settled();
    let canvas = super::canvas::canvas_id(fixture.key.as_ref().expect("a frame ran"));
    fixture.ctx.memory_mut(|m| m.request_focus(canvas));
    fixture.settled();
    assert_eq!(fixture.ctx.memory(|m| m.focused()), Some(canvas));
    fixture.press(Key::Tab);
    let focused = fixture.ctx.memory(|m| m.focused());
    assert!(focused.is_some() && focused != Some(canvas), "Tab moves the focus to another widget");
}

#[test]
fn the_shell_stories_lay_out_inside_their_rect() {
    for story in stories().into_iter().filter(|s| s.id.starts_with("shells/canvas/")) {
        let ctx = Context::default();
        let appearance = Appearance { theme: "craft.artcraft".to_owned(), mode: ModeChoice::Dark, ..Appearance::default() };
        appearance.configure(&ctx, None, false, SystemAppearance::default());
        let mut args: Args = story.defaults();
        let mut bottom = 0.0;
        for _ in 0..4 {
            let raw = RawInput { screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(1280.0, 900.0))), ..RawInput::default() };
            let base = appearance.base(SystemAppearance::default());
            let output = ctx.run_ui(raw, |ui| {
                install(ui.ctx(), &base);
                (story.show)(ui, &mut args);
                bottom = ui.min_rect().bottom();
            });
            output.drop_without_applying_deltas();
        }
        assert!(bottom <= 601.0, "{} stays within its 600 pt rect (bottom {bottom})", story.id);
    }
}
