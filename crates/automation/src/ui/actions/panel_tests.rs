//! The Actions panel on screen (egui_kittest).
//!
//! The AC2 tests draw the part of the panel above the footer (`show_body`); the two AC3 tests
//! need the footer and the context menu.
#![expect(clippy::todo, reason = "skeleton: the AC3 tests are removed when their bodies land")]

use craft_ui::localized::SharedLabels as _;
use craft_ui::testing::{HarnessExt as _, events};
use craft_ui::theme::{Base, Tokens, TokensOf as _, install};
use egui::accesskit::{Role, Toggled};
use egui::{Event, Id, Key, Modifiers, PointerButton, Pos2, Ui};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, Node};
use serde_json::Value;

use super::labels::ActionsLabels;
use super::panel::{ActionPlace, ActionSelection, ActionsEvent, ActionsHost, ActionsPanel, ActionsState, show_body};
use crate::actions::{Action, ActionList, ActionStep, ActionsFile};

/// The app: labels for steps, the recording it runs and the steps recorded so far.
#[derive(Default)]
struct Host {
    recording: Option<usize>,
    pending: Vec<ActionStep>,
}

impl ActionsHost for Host {
    fn step_label(&self, step: &ActionStep) -> String {
        format!("Run {}", step.command)
    }

    fn recording(&self) -> Option<usize> {
        self.recording
    }

    fn pending_steps(&self, action: usize) -> &[ActionStep] {
        if self.recording == Some(action) { &self.pending } else { &[] }
    }
}

struct App {
    state: ActionsState,
    list: ActionList,
    host: Host,
    events: Vec<ActionsEvent>,
    height: f32,
}

fn step(command: &str) -> ActionStep {
    ActionStep::new(command, Value::Null)
}

fn action(name: &str, set: Option<&str>, commands: &[&str]) -> Action {
    let mut action = Action::new(name, commands.iter().map(|command| step(command)).collect());
    action.set = set.map(str::to_owned);
    action
}

/// Sharpen (levels, curves, blur) and Vignette (mask, blur) loose; Glow (glow) in the set Effects.
fn list() -> ActionList {
    let actions = vec![
        action("Sharpen", None, &["levels", "curves", "blur"]),
        action("Vignette", None, &["mask", "blur"]),
        action("Glow", Some("Effects"), &["glow"]),
    ];
    ActionList::from_file(ActionsFile { version: ActionsFile::VERSION, actions, sets: Vec::new() })
}

fn app(list: ActionList) -> App {
    App { state: ActionsState::default(), list, host: Host::default(), events: Vec::new(), height: 320.0 }
}

fn show(ui: &mut Ui, app: &mut App) {
    let labels = ActionsLabels::shared(ui);
    let style = ActionsPanel::style(&Tokens::of(ui));
    let App { state, list, host, events, height } = app;
    events.extend(show_body(ui, Id::NULL.with("actions"), state, list, Some(&*host), &labels, &style, *height));
}

fn harness(app: App) -> Harness<'static, App> {
    let mut harness = Harness::<App>::builder()
        .with_size([320.0, 360.0])
        .with_step_dt(0.05)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(show, app);
    harness.run();
    harness
}

fn events_of(harness: &mut Harness<'_, App>) -> Vec<ActionsEvent> {
    std::mem::take(&mut harness.state_mut().events)
}

fn item<'h>(harness: &'h Harness<'_, App>, name: &'h str) -> Node<'h> {
    harness.get_by_role_and_label(Role::TreeItem, name)
}

fn check<'h>(harness: &'h Harness<'_, App>, name: &'h str) -> Node<'h> {
    harness.get_by_role_and_label(Role::CheckBox, name)
}

fn double_click(harness: &mut Harness<'_, App>, pos: Pos2) {
    for frame in events::click(pos, PointerButton::Primary, 2, Modifiers::NONE) {
        for event in frame {
            harness.event(event);
        }
        harness.run_steps(1);
    }
    harness.run();
}

fn focus(harness: &mut Harness<'_, App>) {
    harness.key_press(Key::Tab);
    harness.run();
}

fn press(harness: &mut Harness<'_, App>, modifiers: Modifiers, key: Key) {
    harness.key_press_modifiers(modifiers, key);
    harness.run();
}

// AC2: a click selects a row; a click on the disclosure toggles it; Alt-click opens or closes all.
#[test]
fn actions_panel_click_select_and_disclosure() {
    let mut harness = harness(app(list()));
    item(&harness, "Vignette").click();
    harness.run();
    assert_eq!(harness.state().state.selected, Some(ActionSelection::Action(1)));
    item(&harness, "Effects").click();
    harness.run();
    assert_eq!(harness.state().state.selected, Some(ActionSelection::Set("Effects".into())));

    harness.get_by_role_and_label(Role::Button, "Expand Sharpen").click();
    harness.run();
    assert!(harness.state().state.expanded.contains("Sharpen"));
    assert_eq!(item(&harness, "Run curves").accesskit_node().level(), Some(1), "the steps sit one level below a loose action");
    harness.get_by_role_and_label(Role::Button, "Collapse Sharpen").click();
    harness.run();
    assert!(harness.state().state.expanded.is_empty());
    assert!(harness.query_by_role_and_label(Role::TreeItem, "Run curves").is_none());

    harness.get_by_role_and_label(Role::Button, "Collapse Effects").click();
    harness.run();
    assert_eq!(harness.state().state.settings.collapsed_sets, ["Effects"]);
    assert!(harness.query_by_role_and_label(Role::TreeItem, "Glow").is_none(), "a collapsed set hides its actions");

    harness.get_by_role_and_label(Role::Button, "Expand Vignette").click_modifiers(Modifiers::ALT);
    harness.run();
    let opened: Vec<_> = harness.state().state.expanded.iter().cloned().collect();
    assert_eq!(opened, ["Glow", "Sharpen", "Vignette"], "Alt-click opens every action");
    assert!(harness.state().state.settings.collapsed_sets.is_empty(), "and every set");
    harness.get_by_role_and_label(Role::Button, "Collapse Vignette").click_modifiers(Modifiers::ALT);
    harness.run();
    assert!(harness.state().state.expanded.is_empty());
    assert_eq!(harness.state().state.settings.collapsed_sets, ["Effects"]);
}

// AC2: a double-click on the name and F2 start a rename; Enter reports `Rename`, Esc nothing.
#[test]
fn actions_panel_rename_by_double_click_and_f2() {
    let mut harness = harness(app(list()));
    let row = item(&harness, "Sharpen").rect();
    let name = Pos2::new(row.left() + 90.0, row.center().y);
    double_click(&mut harness, name);
    assert!(harness.query_by_role(Role::TextInput).is_some(), "a double-click on the name opens the field");
    assert!(events_of(&mut harness).is_empty(), "and does not play the action");
    harness.event(Event::Text("Crisp".into()));
    harness.run();
    press(&mut harness, Modifiers::NONE, Key::Enter);
    assert_eq!(events_of(&mut harness), [ActionsEvent::Rename { action: 0, name: "Crisp".into() }]);
    assert!(harness.query_by_role(Role::TextInput).is_none());

    item(&harness, "Vignette").click();
    harness.run();
    press(&mut harness, Modifiers::NONE, Key::F2);
    assert_eq!(harness.get_by_role(Role::TextInput).value().as_deref(), Some("Vignette"), "F2 renames the selected action");
    harness.event(Event::Text("Dark".into()));
    harness.run();
    press(&mut harness, Modifiers::NONE, Key::Escape);
    assert!(events_of(&mut harness).is_empty(), "Esc reports nothing");
    assert!(harness.query_by_role(Role::TextInput).is_none());
}

// AC2: the include cell reports `ToggleStep` on a step and `ToggleAll` on an action; a partial
// action shows the mixed mark and "2/3 steps".
#[test]
fn actions_panel_include_column_events() {
    let mut app = app(list());
    app.state.expanded.insert("Sharpen".into());
    app.list = {
        let mut file = app.list.to_file();
        file.actions[0].disabled.insert(1);
        file.actions[0].dialogs.insert(2);
        ActionList::from_file(file)
    };
    let mut harness = harness(app);
    assert_eq!(check(&harness, "Include in playback: Sharpen").accesskit_node().toggled(), Some(Toggled::Mixed));
    assert_eq!(check(&harness, "Include in playback: Run levels").accesskit_node().toggled(), Some(Toggled::True));
    assert_eq!(check(&harness, "Include in playback: Run curves").accesskit_node().toggled(), Some(Toggled::False));
    assert_eq!(check(&harness, "Include in playback: Vignette").accesskit_node().toggled(), Some(Toggled::True));
    assert!(harness.query_by_label("2/3 steps").is_some(), "the partial action says how many steps are on");
    assert!(harness.query_by_label("2 steps").is_some(), "a whole action only counts them");

    check(&harness, "Include in playback: Run curves").click();
    harness.run();
    check(&harness, "Include in playback: Sharpen").click();
    harness.run();
    check(&harness, "Show dialog during playback: Run blur").click();
    harness.run();
    assert_eq!(
        events_of(&mut harness),
        [ActionsEvent::ToggleStep { action: 0, step: 1 }, ActionsEvent::ToggleAll { action: 0 }, ActionsEvent::ToggleDialog { action: 0, step: 2 }]
    );
    assert_eq!(check(&harness, "Show dialog during playback: Run blur").accesskit_node().toggled(), Some(Toggled::True));
    assert_eq!(check(&harness, "Show dialog during playback: Run levels").accesskit_node().toggled(), Some(Toggled::False));
}

// AC2: dragging an action, a step, and an action into a set reports `Move` with the place.
#[test]
fn actions_panel_drag_reports_move() {
    let mut app = app(list());
    app.state.expanded.insert("Sharpen".into());
    let mut harness = harness(app);
    let top = |node: Node<'_>, share: f32| {
        let rect = node.rect();
        Pos2::new(rect.center().x, rect.top() + rect.height() * share)
    };

    let (from, to) = (top(item(&harness, "Vignette"), 0.5), top(item(&harness, "Sharpen"), 0.1));
    harness.drag(from, to, Modifiers::NONE);
    assert_eq!(
        events_of(&mut harness),
        [ActionsEvent::Move { source: ActionSelection::Action(1), to: ActionPlace::Action { index: 0, set: None } }],
        "an action dropped above another takes its place"
    );

    let (from, to) = (top(item(&harness, "Vignette"), 0.5), top(item(&harness, "Effects"), 0.5));
    harness.drag(from, to, Modifiers::NONE);
    assert_eq!(
        events_of(&mut harness),
        [ActionsEvent::Move { source: ActionSelection::Action(1), to: ActionPlace::Action { index: 1, set: Some("Effects".into()) } }],
        "dropped on a set it goes into it"
    );

    let (from, to) = (top(item(&harness, "Run blur"), 0.5), top(item(&harness, "Run levels"), 0.1));
    harness.drag(from, to, Modifiers::NONE);
    assert_eq!(
        events_of(&mut harness),
        [ActionsEvent::Move { source: ActionSelection::Step(0, 2), to: ActionPlace::Step { action: 0, index: 0 } }],
        "a step dropped above another"
    );
}

// AC2: in Button Mode a click reports one `Play`, a double-click no second one.
#[test]
fn actions_panel_button_mode_plays_on_click_once() {
    let mut app = app(list());
    app.state.settings.button_mode = true;
    let mut harness = harness(app);
    assert!(harness.query_by_role(Role::Tree).is_none(), "the buttons replace the tree");
    harness.get_by_role_and_label(Role::Button, "Vignette").click();
    harness.run();
    assert_eq!(events_of(&mut harness), [ActionsEvent::Play { action: 1, from: 0 }]);
    let center = harness.get_by_role_and_label(Role::Button, "Glow").rect().center();
    double_click(&mut harness, center);
    assert_eq!(events_of(&mut harness), [ActionsEvent::Play { action: 2, from: 0 }], "a double-click plays once");

    harness.state_mut().host.recording = Some(0);
    harness.run();
    assert!(harness.query_by_role(Role::Tree).is_some(), "Button Mode is off while recording");
}

// AC2: the recording action expands, shows its pending steps and scrolls its latest into view.
#[test]
fn actions_panel_recording_row_expands_and_follows() {
    let mut app = app(list());
    app.height = 100.0;
    app.host.recording = Some(0);
    app.host.pending = vec![step("pending-1"), step("pending-2")];
    let mut harness = harness(app);
    assert!(harness.query_by_role_and_label(Role::TreeItem, "Sharpen, recording").is_some(), "the recording row says so");
    let tree = harness.get_by_role(Role::Tree).rect();
    let latest = item(&harness, "Run pending-2").rect();
    assert!(latest.top() >= tree.top() && latest.bottom() <= tree.bottom() + 0.5, "the latest step is in view: {latest:?} in {tree:?}");
    let below = harness.query_by_role_and_label(Role::TreeItem, "Vignette").map(|node| node.rect());
    assert!(below.is_none_or(|rect| rect.top() >= tree.bottom() - 0.5), "the rows below are scrolled away: {below:?}");

    harness.state_mut().host.pending.push(step("pending-3"));
    harness.run();
    let latest = item(&harness, "Run pending-3").rect();
    assert!(latest.bottom() <= tree.bottom() + 0.5, "a new step scrolls into view: {latest:?} in {tree:?}");
}

// AC2: an empty list shows the empty text; an empty set shows its header.
#[test]
fn actions_panel_empty_state_and_empty_set() {
    let mut harness = harness(app(ActionList::new()));
    let text = ActionsLabels::default().empty;
    assert!(harness.query_by_label(&text).is_some());
    assert!(harness.query_by_role(Role::Tree).is_none());

    let file = ActionsFile { version: ActionsFile::VERSION, actions: Vec::new(), sets: vec!["Empty".into()] };
    harness.state_mut().list = ActionList::from_file(file);
    harness.run();
    assert!(harness.query_by_label(&text).is_none());
    assert!(harness.query_by_role_and_label(Role::TreeItem, "Empty").is_some(), "an empty set keeps its header");
}

// AC2: arrows, Left, Right, Enter, Space, F2 and Cmd/Ctrl+] on the focused list.
#[test]
fn actions_panel_keyboard_operation() {
    let mut harness = harness(app(list()));
    focus(&mut harness);
    press(&mut harness, Modifiers::NONE, Key::ArrowDown);
    assert!(harness.state().state.selected.is_some(), "an arrow selects a row");
    harness.state_mut().state.selected = Some(ActionSelection::Action(0));
    harness.run();
    press(&mut harness, Modifiers::NONE, Key::ArrowRight);
    assert!(harness.state().state.expanded.contains("Sharpen"), "Right opens an action");
    press(&mut harness, Modifiers::NONE, Key::ArrowDown);
    assert_eq!(harness.state().state.selected, Some(ActionSelection::Step(0, 0)));
    press(&mut harness, Modifiers::NONE, Key::Space);
    assert_eq!(events_of(&mut harness), [ActionsEvent::ToggleStep { action: 0, step: 0 }], "Space toggles a step's include cell");
    press(&mut harness, Modifiers::NONE, Key::ArrowLeft);
    assert_eq!(harness.state().state.selected, Some(ActionSelection::Action(0)), "Left goes to the parent");
    press(&mut harness, Modifiers::NONE, Key::ArrowLeft);
    assert!(harness.state().state.expanded.is_empty(), "and closes it");
    press(&mut harness, Modifiers::NONE, Key::Enter);
    assert_eq!(events_of(&mut harness), [ActionsEvent::Play { action: 0, from: 0 }], "Enter plays an action");
    press(&mut harness, Modifiers::NONE, Key::F2);
    assert!(harness.query_by_role(Role::TextInput).is_some());
    press(&mut harness, Modifiers::NONE, Key::Escape);

    press(&mut harness, Modifiers::NONE, Key::ArrowDown);
    assert_eq!(harness.state().state.selected, Some(ActionSelection::Action(1)));
    let before = events_of(&mut harness);
    assert!(before.is_empty());
    press(&mut harness, Modifiers { ctrl: true, command: true, ..Modifiers::NONE }, Key::CloseBracket);
    let moves = events_of(&mut harness);
    assert_eq!(moves, [ActionsEvent::Move { source: ActionSelection::Action(1), to: ActionPlace::Action { index: 0, set: None } }]);

    press(&mut harness, Modifiers::NONE, Key::Delete);
    assert_eq!(events_of(&mut harness), [ActionsEvent::Delete(ActionSelection::Action(1))], "Delete deletes the selection");
}

// AC2: AccessKit roles and names of the tree, rows and the include and dialog cells.
#[test]
fn actions_panel_access_names() {
    let mut app = app(list());
    app.state.expanded.insert("Sharpen".into());
    app.host.recording = Some(1);
    let harness = harness(app);
    assert_eq!(harness.get_by_role(Role::Tree).accesskit_node().label().as_deref(), Some("Actions"));
    assert_eq!(item(&harness, "Sharpen").accesskit_node().level(), Some(0));
    assert_eq!(item(&harness, "Sharpen").accesskit_node().data().is_expanded(), Some(true));
    assert_eq!(item(&harness, "Glow").accesskit_node().level(), Some(1), "an action in a set");
    assert_eq!(item(&harness, "Effects").accesskit_node().data().is_expanded(), Some(true));
    assert!(harness.query_by_role_and_label(Role::TreeItem, "Vignette, recording").is_some());
    for name in ["Include in playback: Sharpen", "Include in playback: Run levels", "Show dialog during playback: Run levels"] {
        assert!(harness.query_by_role_and_label(Role::CheckBox, name).is_some(), "{name}");
    }
    assert!(harness.query_by_role_and_label(Role::CheckBox, "Show dialog during playback: Sharpen").is_none(), "only steps have a dialog cell");
    assert!(harness.query_by_role_and_label(Role::CheckBox, "Include in playback: Effects").is_none(), "a set has no cells");
    craft_ui::testing::a11y::assert_labelled(&harness);
}

// AC3: Play with nothing selected reports `Play` for action 0.
#[test]
#[ignore = "skeleton"]
fn actions_panel_play_with_no_selection_plays_the_first() {
    todo!()
}

// AC3: the context menu reports `AssignShortcut` with key, Shift and Command, and `SetColor`.
#[test]
#[ignore = "skeleton"]
fn actions_panel_context_menu_shortcut_and_color() {
    todo!()
}
