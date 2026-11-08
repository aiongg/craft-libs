//! The dock area: an invalid layout is an error value and draws nothing, hidden panels are not shown, a tab released
//! over empty space or with the command key held floats with its title row under the pointer, and a read-only area
//! takes no drag. A panel command runs the same actions from the group menu and the palette, and the area names the
//! panel that holds key focus apart from the one picked.

use craft_behavior::Axis;
use craft_behavior::dock::{Action, Column, DockEdge, Group, Layout, Node, Permissions, SplitSize, Workspace};
use craft_theme::{Contrast, ControlSize, Mode};
use craft_ui::contract::{Bind, BodyCx, Role};
use craft_ui::docking::{DockConfig, DockEvent, DockPanels, DockSizes, HidePanels, PanelSpec};
use craft_ui::patterns::dock::{DockArea, DockAreaOutput};
use craft_ui::style::Variant;
use craft_ui::testing::layout::in_region;
use craft_ui::testing::pointer::{moved, press, release};
use craft_ui::testing::style::{tokens, with_cx};
use craft_ui::testing::{Fixture, Target, trial};
use egui::{Modifiers, Pos2, Rect, Ui, Vec2};
use std::cell::RefCell;
use std::rc::Rc;

const SIDE: usize = 1;
const OTHER: usize = 2;
const CANVAS: usize = 3;
const COLUMN_WIDTH: f32 = 200.0;
const AREA: Vec2 = Vec2::new(800.0, 600.0);

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
        let out = DockArea::new("Dock area", Bind::bound(&mut workspace), &mut panels, DockConfig::default())
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

// dflags: drops through the area.

/// A point over the canvas, which takes no drop, clear of the outer edge bands.
const EMPTY: Pos2 = Pos2::new(500.0, 150.0);

/// A fixture that shows the docked workspace in an area of `AREA`, read-only or bound, and keeps every action the area
/// reports.
fn drag_bed(read_only: bool) -> (Fixture, Rc<RefCell<Vec<Action<usize>>>>) {
    let actions = Rc::new(RefCell::new(Vec::new()));
    let kept = Rc::clone(&actions);
    let mut workspace = docked();
    let layout = workspace.layout.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut panels = Recorder { shown: Rc::default() };
        let area = if read_only {
            DockArea::read_only("Dock area", &layout, &mut panels, DockConfig::default())
        } else {
            DockArea::new("Dock area", Bind::bound(&mut workspace), &mut panels, DockConfig::default())
        };
        kept.borrow_mut().extend(area.show(ui).actions);
    });
    fixture.room(AREA).settle();
    (fixture, actions)
}

/// The tabs of the dock column, top to bottom: `SIDE`'s, then `OTHER`'s.
fn column_tabs(fixture: &Fixture) -> Vec<Target> {
    let mut tabs: Vec<Target> = fixture.by_role(Role::TAB).into_iter().filter(|tab| tab.rect.max.x <= COLUMN_WIDTH).collect();
    tabs.sort_by(|a, b| a.rect.min.y.total_cmp(&b.rect.min.y));
    tabs
}

/// Presses `from`, moves to `to` in steps and releases there. With `command` the platform command key is held from
/// the last move on, so it is down when the drag is released.
fn drag_tab(fixture: &mut Fixture, from: Pos2, to: Pos2, command: bool) {
    fixture.frame(vec![moved(from)]);
    fixture.frame(vec![press(from)]);
    for step in 1..=4 {
        fixture.frame(vec![moved(from + (to - from) * (step as f32 / 4.0))]);
    }
    if command {
        fixture.hold(Modifiers::COMMAND);
    }
    fixture.frame(vec![moved(to)]);
    fixture.frame(vec![release(to)]);
    fixture.hold(Modifiers::NONE);
}

fn floated(actions: &[Action<usize>]) -> Option<Rect> {
    actions.iter().find_map(|action| match action {
        Action::Float { panel, rect, .. } if *panel == SIDE => Some(*rect),
        _ => None,
    })
}

fn title_row(fixture: &Fixture) -> f32 {
    with_cx(fixture.ctx(), &tokens(Mode::Dark, Contrast::Standard), Variant::SECONDARY, ControlSize::Md, DockSizes::style).bar
}

#[trial]
fn a_tab_released_over_empty_space_floats_with_its_title_row_under_the_pointer() {
    let (mut fixture, actions) = drag_bed(false);
    let side = column_tabs(&fixture)[0];
    drag_tab(&mut fixture, side.center(), EMPTY, false);
    let rect = floated(&actions.borrow()).expect("a release over a canvas that takes no drop floats the panel");
    assert!(rect.contains(EMPTY), "the pointer is on the window: {rect:?}");
    assert!(EMPTY.y - rect.top() <= title_row(&fixture), "the pointer is inside the title row: {rect:?}");
}

#[trial]
fn command_held_at_the_release_floats_a_tab_over_a_dock_spot() {
    let (mut fixture, actions) = drag_bed(false);
    let tabs = column_tabs(&fixture);
    let (side, other) = (tabs[0], tabs[1]);
    drag_tab(&mut fixture, side.center(), other.center(), true);
    let actions = actions.borrow();
    let rect = floated(&actions).expect("the command key floats the drop");
    assert!(rect.contains(other.center()), "the window floats under the pointer: {rect:?}");
    assert!(!actions.iter().any(|action| matches!(action, Action::Move { .. })), "the tab does not join the group: {actions:?}");
}

#[trial]
fn a_drag_in_a_read_only_area_moves_and_floats_nothing() {
    let (mut fixture, actions) = drag_bed(true);
    let tabs = column_tabs(&fixture);
    let (side, other) = (tabs[0], tabs[1]);
    drag_tab(&mut fixture, side.center(), EMPTY, false);
    drag_tab(&mut fixture, side.center(), other.center(), false);
    let actions = actions.borrow();
    assert!(actions.iter().all(|action| matches!(action, Action::Activate { .. })), "only tab picks are reported: {actions:?}");
}

// Panel commands and focus (l4e-dcmd).

use craft_ui::buttons::Button;
use craft_ui::contract::Show as _;
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

/// A fixture of the docked layout over a given workspace, so the area applies none of its actions, that keeps every
/// output.
fn commanded() -> (Fixture, Outputs) {
    let outputs: Outputs = Rc::default();
    let kept = Rc::clone(&outputs);
    let fixture = Fixture::new(move |ui| {
        let out = DockArea::new("Dock area", Bind::given(docked()), &mut Controls, DockConfig::default()).show(ui);
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

// The area applies its actions to the bound workspace (l4f-fdock).

/// Panels that revoke every right once a panel body has run in a pass, when `revoke` is set: the rights the area read
/// at the start of the pass allow the action, and the workspace refuses it when it applies the action.
struct Revoking {
    revoke: bool,
    ran: bool,
}

impl DockPanels<usize> for Revoking {
    fn spec(&self, panel: &usize) -> PanelSpec {
        PanelSpec::new(format!("Panel {panel}"))
    }

    fn permissions(&self, panel: &usize) -> Permissions {
        if (self.revoke && self.ran) || *panel == CANVAS { Permissions::PROTECTED } else { Permissions::ALL }
    }

    fn body(&mut self, _: &usize, _: &mut Ui, _: &mut BodyCx<'_>) {
        self.ran = true;
    }
}

/// A fixture over a bound workspace the test can read, with every output the area made.
fn applying(revoke: bool) -> (Fixture, Rc<RefCell<Workspace<usize>>>, Outputs) {
    let held = Rc::new(RefCell::new(docked()));
    let outputs: Outputs = Rc::default();
    let (bound, kept) = (Rc::clone(&held), Rc::clone(&outputs));
    let fixture = Fixture::new(move |ui| {
        let mut workspace = bound.borrow().clone();
        let mut panels = Revoking { revoke, ran: false };
        let out = DockArea::new("Dock area", Bind::bound(&mut workspace), &mut panels, DockConfig::default()).show(ui);
        *bound.borrow_mut() = workspace;
        kept.borrow_mut().push(out);
    });
    (fixture, held, outputs)
}

/// Closes the side panel from its group menu.
fn close_side_from_menu(fixture: &mut Fixture) {
    fixture.room(AREA).settle();
    let menu = fixture
        .by_role(Role::BUTTON)
        .into_iter()
        .filter(|button| name(fixture.tree(), button.node) == Some("Panel Menu") && button.rect.max.x <= COLUMN_WIDTH)
        .min_by(|a, b| a.rect.top().total_cmp(&b.rect.top()))
        .expect("the side group has a menu button");
    fixture.click(&menu);
    fixture.idle(MENU_FRAMES);
    let close = fixture.by_label("Close").expect("the group menu lists Close");
    fixture.click(&close);
    fixture.idle(MENU_FRAMES);
}

#[trial]
fn closing_a_panel_from_its_group_menu_closes_it_in_the_bound_workspace_and_is_reported() {
    let (mut fixture, held, outputs) = applying(false);
    close_side_from_menu(&mut fixture);
    assert!(!held.borrow().layout.panels().contains(&&SIDE), "the panel left the layout");
    assert!(drained(&outputs).contains(&Action::close(SIDE)), "the output reports the close");
}

#[trial]
fn an_action_the_workspace_refuses_changes_nothing_and_is_not_reported() {
    let (mut fixture, held, outputs) = applying(true);
    close_side_from_menu(&mut fixture);
    assert_eq!(*held.borrow(), docked(), "the workspace is as it was");
    assert!(drained(&outputs).is_empty(), "nothing is reported as applied");
}

/// Panels named after their area, so a click can tell the areas apart.
struct Named(&'static str);

impl DockPanels<usize> for Named {
    fn spec(&self, panel: &usize) -> PanelSpec {
        PanelSpec::new(format!("{} {panel}", self.0))
    }

    fn body(&mut self, _: &usize, _: &mut Ui, _: &mut BodyCx<'_>) {}
}

#[trial]
fn areas_with_different_labels_keep_separate_flyouts() {
    let mut iconic = Column::new(vec![Group::new(vec![SIDE], 0)], COLUMN_WIDTH);
    iconic.iconic = true;
    let root = Node::split(Axis::X, SplitSize::FixedFirst(COLUMN_WIDTH), Node::dock(DockEdge::Start, vec![iconic]), Node::tabs(vec![CANVAS], 0));
    let layout = Layout::new(Some(root), Vec::new());
    let opened: Rc<RefCell<Vec<(&'static str, bool)>>> = Rc::default();
    let seen = Rc::clone(&opened);
    let mut fixture = Fixture::new(move |ui| {
        let half = Vec2::new(AREA.x / 2.0, AREA.y);
        for (at, label) in ["Left", "Right"].into_iter().enumerate() {
            in_region(ui, Rect::from_min_size(Pos2::new(half.x * at as f32, 0.0), half), |ui| {
                let mut panels = Named(label);
                let out = DockArea::read_only(label, &layout, &mut panels, DockConfig::default()).show(ui);
                let flyout = out.events.iter().any(|event| matches!(event, DockEvent::FlyoutChanged { panel: Some(_), .. }));
                seen.borrow_mut().push((label, flyout));
            });
        }
    });
    fixture.room(AREA).settle();
    opened.borrow_mut().clear();
    let icon = fixture.by_label("Left 1").expect("the left area shows its panel's icon");
    fixture.click(&icon);
    fixture.idle(2);
    let opened = opened.borrow();
    assert!(opened.contains(&("Left", true)), "the flyout opened in the area that was clicked: {opened:?}");
    assert!(!opened.contains(&("Right", true)), "the other area's flyout stayed closed: {opened:?}");
}
