//! The swatch pair in a headless egui context: activating, opening, the actions, keys, AccessKit
//! and right to left.

use craft_theme::Srgba;
use egui::accesskit::{Node, NodeId, Role};
use egui::{Context, Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, pos2, vec2};

use super::{PairWell, SwatchPair, Well};
use crate::direction::{self, Direction};
use crate::localized::LocaleNumbers;
use crate::theme::{Base, install};

struct Rig {
    ctx: Context,
    time: f64,
    active: Well,
    colors: [Srgba; 2],
    open: Option<Well>,
    swaps: usize,
    defaults: usize,
    nodes: Vec<(NodeId, Node)>,
}

impl Rig {
    fn new(dir: Direction) -> Self {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        direction::apply(&ctx, dir);
        ctx.enable_accesskit();
        let mut rig = Self {
            ctx,
            time: 0.0,
            active: Well::Primary,
            colors: [Srgba::rgb(0.0, 0.0, 0.0), Srgba::rgb(1.0, 1.0, 1.0)],
            open: None,
            swaps: 0,
            defaults: 0,
            nodes: Vec::new(),
        };
        rig.settle();
        rig
    }

    fn frame(&mut self, events: Vec<Event>) {
        let input =
            RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(400.0, 900.0))), time: Some(self.time), events, ..Default::default() };
        self.time += 1.0 / 60.0;
        let numbers = LocaleNumbers::from_tag("en");
        let ctx = self.ctx.clone();
        let mut shown = None;
        let output = ctx.run_ui(input, |ui| {
            let wells = [PairWell::new("Foreground colour", self.colors[0]), PairWell::new("Background colour", self.colors[1])];
            let pair = SwatchPair::new("pair", "Colours", wells, &numbers)
                .active(self.active)
                .swap_button("Swap colours")
                .default_button("Default colours")
                .show(ui);
            ui.text_edit_singleline(&mut String::from("elsewhere"));
            shown = Some(pair);
        });
        let pair = shown.expect("shown");
        self.active = pair.active;
        self.open = pair.open;
        self.swaps += usize::from(pair.swap);
        self.defaults += usize::from(pair.defaults);
        self.nodes = output.platform_output.accesskit_update.clone().map(|u| u.nodes).unwrap_or_default();
        output.drop_without_applying_deltas();
    }

    fn settle(&mut self) {
        for _ in 0..4 {
            self.frame(vec![]);
        }
    }

    fn key(&mut self, key: Key) {
        let modifiers = Modifiers::NONE;
        self.frame(vec![Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }]);
        self.frame(vec![]);
        self.settle();
    }

    fn click(&mut self, at: Pos2) {
        self.frame(vec![Event::PointerMoved(at), button(at, true)]);
        self.frame(vec![button(at, false)]);
        self.settle();
    }

    fn double_click(&mut self, at: Pos2) {
        // Apart from an earlier click, which would make the first press a double one.
        self.time += 1.0;
        self.frame(vec![Event::PointerMoved(at), button(at, true)]);
        self.frame(vec![button(at, false)]);
        self.frame(vec![button(at, true)]);
        self.frame(vec![button(at, false)]);
        self.settle();
    }

    fn bounds(&self, name: &str) -> Rect {
        let b = self.node(Role::Button, name).bounds().expect("bounds");
        Rect::from_min_max(pos2(b.x0 as f32, b.y0 as f32), pos2(b.x1 as f32, b.y1 as f32))
    }

    fn focused_name(&self) -> Option<String> {
        let id = self.ctx.memory(|m| m.focused())?.accesskit_id();
        self.nodes.iter().find(|(n, _)| *n == id).map(|(_, node)| node.label().unwrap_or_default().to_owned())
    }

    fn tab_to(&mut self, name: &str) {
        for _ in 0..6 {
            if self.focused_name().as_deref() == Some(name) {
                return;
            }
            self.key(Key::Tab);
        }
        panic!("Tab never reached {name}");
    }

    fn node(&self, role: Role, name: &str) -> &Node {
        self.nodes
            .iter()
            .map(|(_, n)| n)
            .find(|n| n.role() == role && n.label() == Some(name))
            .unwrap_or_else(|| panic!("no {role:?} named {name:?}"))
    }
}

fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

/// A point of the secondary well outside the primary one.
fn secondary_only(rig: &Rig) -> Pos2 {
    let (primary, secondary) = (rig.bounds("Foreground colour"), rig.bounds("Background colour"));
    let corner = if direction::is_rtl(&rig.ctx) { secondary.left_bottom() + vec2(3.0, -3.0) } else { secondary.right_bottom() - vec2(3.0, 3.0) };
    assert!(!primary.contains(corner));
    corner
}

#[test]
fn a_click_makes_a_well_active_and_a_double_click_opens_the_active_wells_popup() {
    let mut rig = Rig::new(Direction::Ltr);
    assert_eq!(rig.active, Well::Primary);
    let at = secondary_only(&rig);
    rig.click(at);
    assert_eq!(rig.active, Well::Secondary, "a click makes the well active");
    assert_eq!(rig.open, None, "a click opens nothing");

    rig.double_click(at);
    assert_eq!(rig.active, Well::Secondary);
    assert_eq!(rig.open, Some(Well::Secondary), "a double-click opens the well's popup");
    assert!(rig.node(Role::Button, "Background colour").is_expanded() == Some(true));
    assert!(rig.node(Role::Button, "Foreground colour").is_expanded() != Some(true));
    rig.nodes.iter().find(|(_, n)| n.role() == Role::Group && n.label() == Some("Background colour")).expect("the picker names the well");

    rig.key(Key::Escape);
    assert_eq!(rig.open, None);
}

#[test]
fn enter_opens_the_focused_wells_popup_and_the_arrows_switch_the_active_well() {
    let mut rig = Rig::new(Direction::Ltr);
    rig.tab_to("Foreground colour");
    rig.key(Key::ArrowRight);
    assert_eq!(rig.active, Well::Secondary);
    assert_eq!(rig.focused_name().as_deref(), Some("Background colour"), "the focus follows");
    rig.key(Key::ArrowUp);
    assert_eq!(rig.active, Well::Primary);
    rig.key(Key::ArrowDown);
    assert_eq!(rig.active, Well::Secondary);
    rig.key(Key::Enter);
    assert_eq!(rig.open, Some(Well::Secondary), "Enter opens the focused well's popup");
}

#[test]
fn the_wells_are_one_tab_stop() {
    let mut rig = Rig::new(Direction::Ltr);
    rig.tab_to("Foreground colour");
    rig.key(Key::Tab);
    assert_eq!(rig.focused_name().as_deref(), Some("Default colours"), "the next Tab leaves the wells");
    rig.key(Key::Tab);
    assert_eq!(rig.focused_name().as_deref(), Some("Swap colours"));
}

#[test]
fn the_buttons_report_swap_and_default() {
    let mut rig = Rig::new(Direction::Ltr);
    let swap = rig.bounds("Swap colours").center();
    rig.click(swap);
    assert_eq!((rig.swaps, rig.defaults), (1, 0));
    let default = rig.bounds("Default colours").center();
    rig.click(default);
    assert_eq!((rig.swaps, rig.defaults), (1, 1));
}

#[test]
fn accesskit_names_the_wells_and_marks_the_active_one() {
    let mut rig = Rig::new(Direction::Ltr);
    rig.nodes.iter().find(|(_, n)| n.role() == Role::Group && n.label() == Some("Colours")).expect("the group");
    let fg = rig.node(Role::Button, "Foreground colour");
    assert_eq!(fg.value(), Some("#000000"));
    assert_eq!(fg.is_selected(), Some(true));
    let bg = rig.node(Role::Button, "Background colour");
    assert_eq!(bg.value(), Some("#ffffff"));
    assert_eq!(bg.is_selected(), Some(false));
    rig.node(Role::Button, "Swap colours");
    rig.node(Role::Button, "Default colours");
    let at = secondary_only(&rig);
    rig.click(at);
    assert_eq!(rig.node(Role::Button, "Background colour").is_selected(), Some(true));
    assert_eq!(rig.node(Role::Button, "Foreground colour").is_selected(), Some(false));
}

#[test]
fn right_to_left_mirrors_the_wells_and_the_arrows() {
    let mut rig = Rig::new(Direction::Rtl);
    let (primary, secondary) = (rig.bounds("Foreground colour"), rig.bounds("Background colour"));
    assert!(primary.center().x > secondary.center().x, "the primary well is at the start, on the right");
    assert!(primary.center().y < secondary.center().y);
    assert!(rig.bounds("Swap colours").center().x < secondary.center().x, "the buttons are at the end, on the left");
    rig.tab_to("Foreground colour");
    rig.key(Key::ArrowLeft);
    assert_eq!(rig.active, Well::Secondary, "Left is forward right to left");
    rig.key(Key::ArrowRight);
    assert_eq!(rig.active, Well::Primary);
    let at = secondary_only(&rig);
    rig.click(at);
    assert_eq!(rig.active, Well::Secondary);
}

mod in_a_dialog {
    use egui::accesskit::Role;
    use egui::{Key, Modifiers};
    use egui_kittest::Harness;
    use egui_kittest::kittest::{NodeT as _, Queryable as _};

    use crate::story::Args;
    use crate::theme::{Base, install};

    fn focused(h: &Harness<'_, Args>) -> String {
        h.get_all_by(|_| true).find(|n| n.accesskit_node().is_focused()).and_then(|n| n.accesskit_node().label()).unwrap_or_default()
    }

    #[test]
    fn tab_passes_the_pair_in_a_field_scope_and_ctrl_tab_reaches_it() {
        let story = super::super::pair_stories::stories().into_iter().find(|s| s.id == "composites/color/pair-dialog").expect("the story");
        let mut h = Harness::<Args>::builder()
            .with_size([480.0, 320.0])
            .with_step_dt(1.0 / 60.0)
            .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
            .build_ui_state(story.show, story.defaults());
        h.run_steps(3);

        h.key_press(Key::Tab);
        h.run_steps(3);
        assert_eq!(focused(&h), "Palette name", "Tab reaches the first field");
        h.key_press(Key::Tab);
        h.run_steps(3);
        assert_eq!(focused(&h), "Notes", "Tab goes to the next field and passes the pair");

        h.get_by_label("Palette name").focus();
        h.run_steps(3);
        h.key_press_modifiers(Modifiers::CTRL, Key::Tab);
        h.run_steps(3);
        let well = h.get_by_role_and_label(Role::Button, "Fill colour");
        assert!(well.accesskit_node().is_focused(), "Ctrl+Tab reaches the pair, but focus is on {:?}", focused(&h));
    }
}

impl Rig {
    /// Sends a screen reader's `action` to the node named `name`.
    fn action(&mut self, role: Role, name: &str, action: egui::accesskit::Action) {
        let target_node = self.nodes.iter().find(|(_, n)| n.role() == role && n.label() == Some(name)).map(|(id, _)| *id).expect("node");
        let request = egui::accesskit::ActionRequest { action, target_tree: egui::accesskit::TreeId::ROOT, target_node, data: None };
        self.frame(vec![Event::AccessKitActionRequest(request)]);
        self.settle();
    }
}

/// The model menu in a well's popup is part of the popup: choosing in it by pointer or by keys
/// leaves the popup open.
#[test]
fn choosing_in_a_wells_model_menu_keeps_its_popup_open() {
    let mut rig = Rig::new(Direction::Ltr);
    rig.tab_to("Foreground colour");
    rig.key(Key::Enter);
    assert_eq!(rig.open, Some(Well::Primary));

    rig.click(rig.bounds("Colour model: HSB").center());
    assert_eq!(rig.open, Some(Well::Primary), "opening the menu keeps the popup open");
    let row = rig.node(Role::MenuItemRadio, "RGB").bounds().expect("bounds");
    rig.click(pos2(((row.x0 + row.x1) / 2.0) as f32, ((row.y0 + row.y1) / 2.0) as f32));
    assert_eq!(rig.open, Some(Well::Primary), "a click on a row is not outside the popup");
    rig.node(Role::Button, "Colour model: RGB");

    rig.action(Role::Button, "Colour model: RGB", egui::accesskit::Action::Focus);
    rig.key(Key::Enter);
    assert_eq!(rig.node(Role::Button, "Colour model: RGB").is_expanded(), Some(true), "Enter opens the menu");
    assert_eq!(rig.open, Some(Well::Primary));
    rig.key(Key::Enter);
    assert_eq!(rig.open, Some(Well::Primary), "a row chosen by Enter leaves the popup open");
    rig.node(Role::Button, "Colour model: HSB");
}

/// The inactive well isn't a Tab stop, but a screen reader can focus it, and that makes it
/// active.
#[test]
fn a_screen_readers_focus_on_the_inactive_well_makes_it_active() {
    let mut rig = Rig::new(Direction::Ltr);
    assert_eq!(rig.active, Well::Primary);
    assert!(rig.node(Role::Button, "Background colour").supports_action(egui::accesskit::Action::Focus), "the inactive well offers Focus");
    rig.action(Role::Button, "Background colour", egui::accesskit::Action::Focus);
    assert_eq!(rig.active, Well::Secondary);
    assert_eq!(rig.focused_name().as_deref(), Some("Background colour"));
}

/// PhotoCraft's chips: the default button over the start of the wells and the swap button over the
/// end, both in a row above them, mirrored right to left.
#[test]
fn the_buttons_are_a_row_over_the_wells_default_at_the_start_and_swap_at_the_end() {
    for dir in [Direction::Ltr, Direction::Rtl] {
        let rig = Rig::new(dir);
        let (fg, bg) = (rig.bounds("Foreground colour"), rig.bounds("Background colour"));
        let (default, swap) = (rig.bounds("Default colours"), rig.bounds("Swap colours"));
        let wells = fg.union(bg);
        assert_eq!((default.size(), swap.size()), (vec2(13.0, 13.0), vec2(13.0, 13.0)));
        assert_eq!(default.top(), swap.top());
        assert_eq!(wells.top() - default.top(), 16.0, "the wells start 13 + 3 points under the row");
        assert!(default.bottom() <= wells.top());
        if dir == Direction::Ltr {
            assert_eq!((default.left(), swap.right()), (wells.left(), wells.right()));
        } else {
            assert_eq!((default.right(), swap.left()), (wells.right(), wells.left()));
        }
    }
}

/// The front well is a light 1 point line inside a dark one, with the backdrop's gap round them.
#[test]
fn the_front_well_has_a_light_inner_line_a_dark_outer_line_and_a_backdrop_gap() {
    use egui::epaint::RectShape;
    use egui::{Color32, Shape, StrokeKind};
    fn rects(shape: &Shape, out: &mut Vec<RectShape>) {
        match shape {
            Shape::Vec(shapes) => shapes.iter().for_each(|s| rects(s, out)),
            Shape::Rect(r) => out.push(r.clone()),
            _ => {}
        }
    }
    let toolbar = Color32::from_rgb(0x12, 0x34, 0x56);
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let numbers = LocaleNumbers::from_tag("en");
    let mut tokens = None;
    let output = ctx.run_ui(RawInput::default(), |ui| {
        tokens = Some(<crate::theme::Tokens as crate::theme::TokensOf>::of(ui));
        let wells = [PairWell::new("Foreground colour", Srgba::rgb(1.0, 0.0, 0.0)), PairWell::new("Background colour", Srgba::WHITE)];
        let _ = SwatchPair::new("pair", "Colours", wells, &numbers).backdrop(toolbar).show(ui);
    });
    let mut all = Vec::new();
    for clipped in &output.shapes {
        rects(&clipped.shape, &mut all);
    }
    output.drop_without_applying_deltas();
    let tokens = tokens.expect("tokens");
    let fill = all.iter().find(|r| r.fill == Color32::RED).expect("the front well");
    let on = |kind| all.iter().find(|r| r.rect == fill.rect && r.stroke.width == 1.0 && r.stroke_kind == kind).map(|r| r.stroke.color);
    assert_eq!(on(StrokeKind::Inside), Some(crate::theme::ToEgui::to_egui(tokens.color.text.normal).gamma_multiply(0.9)));
    assert_eq!(on(StrokeKind::Outside), Some(crate::theme::ToEgui::to_egui(tokens.component.field.border.rest)));
    let gap = all.iter().find(|r| r.fill == toolbar).expect("the gap");
    assert_eq!(gap.rect, fill.rect.expand(tokens.component.color_picker.pair_gap));
}
