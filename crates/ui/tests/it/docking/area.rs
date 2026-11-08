//! The dock area: an invalid layout is an error value and draws nothing, hidden panels are not shown, and a panel
//! dropped on empty space floats with its title row under the pointer. A panel command runs the same actions from
//! the group menu and the palette, and the area names the panel that holds key focus apart from the one picked.

use craft_behavior::Axis;
use craft_behavior::dock::{Column, DockEdge, Group, Layout, Node, Permissions, SplitSize, Workspace};
use craft_theme::{Contrast, ControlSize, Mode};
use craft_ui::contract::{Bind, BodyCx};
use craft_ui::docking::{DockConfig, DockPanels, DockSizes, DragUnit, FloatSpace, HidePanels, PanelSpec, float_drop};
use craft_ui::patterns::dock::{DockArea, DockAreaOutput};
use craft_ui::style::Variant;
use craft_ui::testing::memory::global;
use craft_ui::testing::style::{tokens, with_cx};
use craft_ui::testing::{Fixture, Frames, trial};
use egui::{Pos2, Rect, Ui, Vec2};
use std::cell::RefCell;
use std::rc::Rc;

const SIDE: usize = 1;
const OTHER: usize = 2;
const CANVAS: usize = 3;
const COLUMN_WIDTH: f32 = 200.0;
const AREA: Vec2 = Vec2::new(800.0, 600.0);
const POINTER: Pos2 = Pos2::new(400.0, 300.0);

/// Records the panels whose bodies the area showed.
struct Recorder {
    shown: Rc<RefCell<Vec<usize>>>,
}

impl DockPanels<usize> for Recorder {
    fn spec(&self, panel: &usize) -> PanelSpec {
        PanelSpec::new(format!("Panel {panel}"))
    }

    fn permissions(&self, panel: &usize) -> Permissions {
        if *panel == CANVAS { Permissions::PROTECTED } else { Permissions::ALL }
    }

    fn body(&mut self, panel: &usize, _: &mut Ui, _: &mut BodyCx<'_>) {
        self.shown.borrow_mut().push(*panel);
    }
}

fn docked() -> Workspace<usize> {
    let column = Column::new(vec![Group::new(vec![SIDE], 0), Group::new(vec![OTHER], 0)], COLUMN_WIDTH);
    let root = Node::split(Axis::X, SplitSize::FixedFirst(COLUMN_WIDTH), Node::dock(DockEdge::Start, vec![column]), Node::tabs(vec![CANVAS], 0));
    Workspace::new(Layout::new(Some(root), Vec::new()))
}

fn shown_panels(workspace: Workspace<usize>, hide_all: bool) -> (Vec<usize>, Option<DockAreaOutput<usize>>) {
    let shown = Rc::new(RefCell::new(Vec::new()));
    let output: Rc<RefCell<Option<DockAreaOutput<usize>>>> = Rc::default();
    let (seen, kept) = (Rc::clone(&shown), Rc::clone(&output));
    let mut workspace = workspace;
    let mut fixture = Fixture::new(move |ui| {
        seen.borrow_mut().clear();
        let mut panels = Recorder { shown: Rc::clone(&seen) };
        let out = DockArea::new(global("dock area", "trial"), Bind::bound(&mut workspace), &mut panels, DockConfig::default())
            .hide(if hide_all { HidePanels::All } else { HidePanels::None })
            .show(ui);
        *kept.borrow_mut() = Some(out);
    });
    fixture.settle();
    let panels = shown.borrow().clone();
    let out = output.borrow_mut().take();
    (panels, out)
}

#[trial]
fn a_layout_that_fails_validation_is_an_error_value_and_draws_no_panel() {
    let broken = Workspace::new(Layout::new(Some(Node::tabs(vec![SIDE, SIDE], 0)), Vec::new()));
    let (panels, output) = shown_panels(broken, false);
    let output = output.expect("the area ran");
    assert!(output.error.is_some(), "the invalid geometry is reported");
    assert!(panels.is_empty(), "no panel is drawn: {panels:?}");
}

#[trial]
fn hidden_panels_are_not_drawn_and_a_protected_canvas_stays() {
    let (panels, output) = shown_panels(docked(), true);
    assert!(output.is_some_and(|output| output.error.is_none()));
    assert!(panels.contains(&CANVAS), "the protected canvas stays: {panels:?}");
    assert!(!panels.contains(&SIDE) && !panels.contains(&OTHER), "hidden panels are left out: {panels:?}");
}

#[trial]
fn a_panel_dropped_on_empty_space_floats_with_its_title_row_under_the_pointer() {
    let frames = Frames::new();
    let sizes = with_cx(&frames.ctx, &tokens(Mode::Dark, Contrast::Standard), Variant::SECONDARY, ControlSize::Md, DockSizes::style);
    let area = Rect::from_min_size(Pos2::ZERO, AREA);
    let space = FloatSpace::<usize>::new(area, &sizes);
    let dropped = float_drop(&DragUnit::Panel(SIDE), &space, POINTER);
    let craft_behavior::dock::Action::Float { rect, .. } = dropped.action else { panic!("a panel floats alone") };
    assert!(rect.contains(POINTER), "the pointer is on the window: {rect:?}");
    assert!(POINTER.y - rect.top() <= space.title_row, "the pointer is inside the title row: {rect:?}");
}

// Panel commands and focus (l4e-dcmd).

use craft_behavior::dock::Action;
use craft_ui::buttons::Button;
use craft_ui::contract::{Role, Show as _};
use craft_ui::docking::{DockCommands, DockScene, actions};
use craft_ui::runtime::commands::{FireSource, fire};
use craft_ui::testing::access::name;
use craft_ui::testing::focus::AccessActionKind;

const MENU_FRAMES: usize = 10;

/// Panels whose side panel holds a button.
struct Controls;

impl DockPanels<usize> for Controls {
    fn spec(&self, panel: &usize) -> PanelSpec {
        PanelSpec::new(format!("Panel {panel}"))
    }

    fn permissions(&self, panel: &usize) -> Permissions {
        if *panel == CANVAS { Permissions::PROTECTED } else { Permissions::ALL }
    }

    fn body(&mut self, panel: &usize, ui: &mut Ui, _: &mut BodyCx<'_>) {
        if *panel == SIDE {
            let _ = Button::new("Inside").show(ui);
        }
    }
}

type Outputs = Rc<RefCell<Vec<DockAreaOutput<usize>>>>;

/// A fixture of the docked layout that keeps every output and applies none of its actions.
fn commanded() -> (Fixture, Outputs) {
    let outputs: Outputs = Rc::default();
    let kept = Rc::clone(&outputs);
    let mut workspace = docked();
    let fixture = Fixture::new(move |ui| {
        let out = DockArea::new(global("dock area", "commands"), Bind::bound(&mut workspace), &mut Controls, DockConfig::default()).show(ui);
        kept.borrow_mut().push(out);
    });
    (fixture, outputs)
}

fn drained(outputs: &Outputs) -> Vec<Action<usize>> {
    outputs.borrow_mut().drain(..).flat_map(|output| output.actions).collect()
}

#[trial]
fn float_runs_the_same_actions_from_the_group_menu_and_the_palette() {
    let (mut fixture, outputs) = commanded();
    fixture.settle();
    let space = outputs.borrow().last().and_then(|output| output.float_space.clone()).expect("the area reports its float space");
    let permissions = |panel: &usize| Controls.permissions(panel);
    let layout = docked().layout;
    let expected = actions(&DockScene::new(&layout, &space, false, &permissions), DockCommands::FloatPanel, &SIDE).expect("the side panel floats");

    let inside = fixture.by_label("Inside").expect("the side panel shows its button");
    fixture.access(AccessActionKind::Focus, &inside);
    fixture.idle(2);
    drained(&outputs);
    assert!(fire(fixture.ctx(), DockCommands::FloatPanel, FireSource::Palette), "the group that holds focus takes Float");
    fixture.idle(2);
    assert_eq!(drained(&outputs), expected, "the palette runs the command's actions");

    let menu = fixture
        .by_role(Role::BUTTON)
        .into_iter()
        .filter(|button| name(fixture.tree(), button.node) == Some("Panel Menu") && button.rect.max.x <= COLUMN_WIDTH)
        .min_by(|a, b| a.rect.top().total_cmp(&b.rect.top()))
        .expect("the side group has a menu button");
    fixture.click(&menu);
    fixture.idle(MENU_FRAMES);
    let float = fixture.by_label("Float").expect("the group menu lists Float");
    drained(&outputs);
    fixture.click(&float);
    fixture.idle(MENU_FRAMES);
    assert_eq!(drained(&outputs), expected, "the menu row runs the same actions");
}

#[trial]
fn focus_in_a_panels_content_names_its_panel_while_nothing_is_picked() {
    let (mut fixture, outputs) = commanded();
    fixture.settle();
    let inside = fixture.by_label("Inside").expect("the side panel shows its button");
    fixture.access(AccessActionKind::Focus, &inside);
    fixture.idle(2);
    let last = outputs.borrow_mut().pop().expect("the area ran");
    assert_eq!(last.focused, Some(SIDE), "the group holding focus names its panel");
    assert_eq!(last.picked, None, "focus alone picks nothing");
}
