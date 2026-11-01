//! The Actions panel on screen (egui_kittest).
//!
//! The AC2 tests draw the part of the panel above the footer (`show_body`); the two AC3 tests
//! draw the whole panel (`ActionsPanel::show`) for the footer and the context menu.

use craft_ui::edit::EditEvent;
use craft_ui::localized::{LabelColorLabels, Labels as _, SharedLabels as _};
use craft_ui::testing::{HarnessExt as _, events};
use craft_ui::theme::{Base, Tokens, TokensOf as _, install};
use egui::accesskit::{Role, Toggled};
use egui::{Event, Id, Key, Modifiers, PointerButton, Pos2, Ui};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, Node};
use serde_json::Value;

use super::labels::ActionsLabels;
use super::panel::{ActionPlace, ActionSelection, ActionsEvent, ActionsHost, ActionsPanel, ActionsPanelState, ShortcutChoice, show_body};
use crate::actions::{Action, ActionColor, ActionList, ActionStep, ActionsFile};

/// The app: labels for steps, the recording it runs and the steps recorded so far.
#[derive(Default)]
struct Host {
    recording: Option<usize>,
    pending: Vec<ActionStep>,
    /// The shortcut of every action.
    shortcut: Option<ShortcutChoice>,
}

impl ActionsHost for Host {
    fn step_label(&self, step: &ActionStep) -> String {
        format!("Run {}", step.command)
    }

    fn shortcut(&self, _: &Action) -> Option<ShortcutChoice> {
        self.shortcut
    }

    fn recording(&self) -> Option<usize> {
        self.recording
    }

    fn pending_steps(&self, action: usize) -> &[ActionStep] {
        if self.recording == Some(action) { &self.pending } else { &[] }
    }
}

struct App {
    state: ActionsPanelState,
    list: ActionList,
    host: Host,
    events: Vec<ActionsEvent>,
    height: f32,
    settings_changed: bool,
    rename: Option<usize>,
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
    App { state: ActionsPanelState::default(), list, host: Host::default(), events: Vec::new(), height: 320.0, settings_changed: false, rename: None }
}

fn show(ui: &mut Ui, app: &mut App) {
    let labels = ActionsLabels::shared(ui);
    let style = ActionsPanel::style(&Tokens::of(ui));
    let App { state, list, host, events, height, rename, .. } = app;
    events.extend(show_body(ui, Id::NULL.with("actions"), state, list, Some(&*host), &labels, &style, *height, *rename));
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

/// The cells the toggle gestures ended on (a `Commit` or a `Set`): row, column, state.
fn toggles(events: &[ActionsEvent]) -> Vec<(ActionSelection, usize, bool)> {
    events
        .iter()
        .filter_map(|event| match event {
            ActionsEvent::Toggle(EditEvent::Commit { value, .. } | EditEvent::Set { value, .. }) => Some((value.row.clone(), value.column, value.on)),
            _ => None,
        })
        .collect()
}

/// The moves the reorder gestures ended on (a `Commit` or a `Set`): the dragged row and where it goes.
fn moves(list: &ActionList, events: &[ActionsEvent]) -> Vec<(ActionSelection, ActionPlace)> {
    events
        .iter()
        .filter_map(|event| match event {
            ActionsEvent::Reorder(EditEvent::Commit { value, .. } | EditEvent::Set { value, .. }) => {
                Some((value.items.first()?.clone(), ActionPlace::of(list, value)?))
            }
            _ => None,
        })
        .collect()
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
    assert_eq!(events_of(&mut harness), [ActionsEvent::Rename { action: 0, text: "Crisp".into() }]);
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
        toggles(&events_of(&mut harness)),
        [(ActionSelection::Step(0, 1), 0, true), (ActionSelection::Action(0), 0, true), (ActionSelection::Step(0, 2), 1, false)]
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
    let list = harness.state().list.clone();
    assert_eq!(
        moves(&list, &events_of(&mut harness)),
        [(ActionSelection::Action(1), ActionPlace::Action { index: 0, set: None })],
        "an action dropped above another takes its place"
    );

    let (from, to) = (top(item(&harness, "Vignette"), 0.5), top(item(&harness, "Effects"), 0.5));
    harness.drag(from, to, Modifiers::NONE);
    assert_eq!(
        moves(&list, &events_of(&mut harness)),
        [(ActionSelection::Action(1), ActionPlace::Action { index: 1, set: Some("Effects".into()) })],
        "dropped on a set it goes into it"
    );

    let (from, to) = (top(item(&harness, "Run blur"), 0.5), top(item(&harness, "Run levels"), 0.1));
    harness.drag(from, to, Modifiers::NONE);
    assert_eq!(
        moves(&list, &events_of(&mut harness)),
        [(ActionSelection::Step(0, 2), ActionPlace::Step { action: 0, index: 0 })],
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
    assert_eq!(toggles(&events_of(&mut harness)), [(ActionSelection::Step(0, 0), 0, false)], "Space toggles a step's include cell");
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
    let list = harness.state().list.clone();
    assert_eq!(moves(&list, &events_of(&mut harness)), [(ActionSelection::Action(1), ActionPlace::Action { index: 0, set: None })]);

    press(&mut harness, Modifiers::NONE, Key::Delete);
    assert_eq!(
        events_of(&mut harness),
        [ActionsEvent::Delete(ActionSelection::Action(0))],
        "Delete deletes the selection, which followed the moved action to index 0"
    );
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

fn show_panel(ui: &mut Ui, app: &mut App) {
    let App { state, list, host, events, settings_changed, rename, .. } = app;
    let mut panel = ActionsPanel::new("actions", state, list).host(&*host);
    if let Some(action) = *rename {
        panel = panel.start_rename(action);
    }
    let out = panel.show(ui);
    *settings_changed |= out.settings_changed;
    events.extend(out.events);
}

fn panel_harness(app: App) -> Harness<'static, App> {
    let mut harness = Harness::<App>::builder()
        .with_size([320.0, 720.0])
        .with_step_dt(0.05)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(show_panel, app);
    harness.run();
    harness
}

fn button<'h>(harness: &'h Harness<'_, App>, name: &'h str) -> Node<'h> {
    harness.get_by_role_and_label(Role::Button, name)
}

fn menu_row(harness: &mut Harness<'_, App>, label: &str) {
    harness.get_by_label(label).click();
    harness.run();
}

// AC3: Play with nothing selected reports `Play` for action 0.
#[test]
fn actions_panel_play_with_no_selection_plays_the_first() {
    let mut harness = panel_harness(app(list()));
    button(&harness, "Play selection").click();
    harness.run();
    assert_eq!(events_of(&mut harness), [ActionsEvent::Play { action: 0, from: 0 }]);

    harness.get_by_role_and_label(Role::Button, "Expand Vignette").click();
    harness.run();
    item(&harness, "Run blur").click();
    harness.run();
    button(&harness, "Play selection").click();
    harness.run();
    assert_eq!(events_of(&mut harness), [ActionsEvent::Play { action: 1, from: 1 }], "a selected step plays its action from that step");

    button(&harness, "Delete").click();
    harness.run();
    assert_eq!(events_of(&mut harness), [ActionsEvent::Delete(ActionSelection::Step(1, 1))]);
    button(&harness, "Create new set").click();
    harness.run();
    assert_eq!(events_of(&mut harness), [ActionsEvent::NewSet]);
    button(&harness, "Button Mode").click();
    harness.run();
    assert!(harness.state().state.settings.button_mode, "Button Mode toggles in the state");
    assert!(events_of(&mut harness).is_empty());
    assert!(harness.state().settings_changed, "and the output says the settings changed");
    assert_eq!(button(&harness, "Button Mode").accesskit_node().toggled(), Some(Toggled::True), "Button Mode is a toggle button");
}

// FX6-17: a set's disclosure changes the persisted settings and the output says so.
#[test]
fn actions_panel_set_disclosure_reports_settings_changed() {
    let mut harness = panel_harness(app(list()));
    assert!(!harness.state().settings_changed);
    harness.get_by_role_and_label(Role::Button, "Collapse Effects").click();
    harness.run();
    assert_eq!(harness.state().state.settings.collapsed_sets, ["Effects"]);
    assert!(harness.state().settings_changed);
}

// FX6-17: Record stays live while recording: toggled, in the alert tone, and its click starts nothing.
#[test]
fn actions_panel_record_button_shows_the_recording_and_ignores_the_click() {
    let idle = panel_harness(app(list()));
    assert_eq!(button(&idle, "Begin recording").accesskit_node().toggled(), Some(Toggled::False));
    let mut recording = app(list());
    recording.host.recording = Some(0);
    let mut harness = panel_harness(recording);
    let record = button(&harness, "Begin recording");
    assert_eq!(record.accesskit_node().toggled(), Some(Toggled::True), "Record shows the recording");
    assert!(!record.accesskit_node().is_disabled(), "and stays live");
    record.click();
    harness.run();
    assert!(events_of(&mut harness).is_empty(), "a click while recording starts nothing");
}

// AC3: the context menu reports `AssignShortcut` with key, Shift and Command, and `SetColor`.
#[test]
fn actions_panel_context_menu_shortcut_and_color() {
    let labels = ActionsLabels::english();
    let mut app = app(list());
    app.host.shortcut = Some(ShortcutChoice { key: 3, shift: false, command: false });
    let mut harness = panel_harness(app);

    // Check rows keep the menu open, so both can be ticked; each tick is reported at once.
    item(&harness, "Sharpen").click_secondary();
    harness.run();
    menu_row(&mut harness, &labels.shift);
    assert_eq!(
        events_of(&mut harness),
        [ActionsEvent::AssignShortcut { action: 0, choice: Some(ShortcutChoice { key: 3, shift: true, command: false }) }]
    );
    menu_row(&mut harness, if cfg!(target_os = "macos") { &labels.command } else { &labels.ctrl });
    assert_eq!(
        events_of(&mut harness),
        [ActionsEvent::AssignShortcut { action: 0, choice: Some(ShortcutChoice { key: 3, shift: true, command: true }) }]
    );
    press(&mut harness, Modifiers::NONE, Key::Escape);
    assert!(events_of(&mut harness).is_empty(), "closing the menu reports nothing more");

    // A key row reports at once, with the ticks the menu opened with.
    item(&harness, "Vignette").click_secondary();
    harness.run();
    menu_row(&mut harness, "F7");
    assert_eq!(
        events_of(&mut harness),
        [ActionsEvent::AssignShortcut { action: 1, choice: Some(ShortcutChoice { key: 7, shift: false, command: false }) }]
    );
    item(&harness, "Vignette").click_secondary();
    harness.run();
    menu_row(&mut harness, &labels.none);
    assert_eq!(events_of(&mut harness), [ActionsEvent::AssignShortcut { action: 1, choice: None }]);

    item(&harness, "Vignette").click_secondary();
    harness.run();
    menu_row(&mut harness, &labels.color);
    menu_row(&mut harness, &LabelColorLabels::english().red);
    assert_eq!(events_of(&mut harness), [ActionsEvent::SetColor { action: 1, color: ActionColor::Red }]);

    item(&harness, "Effects").click_secondary();
    harness.run();
    menu_row(&mut harness, &labels.save_set);
    assert_eq!(events_of(&mut harness), [ActionsEvent::Save { set: Some("Effects".to_owned()) }]);
}

// FX6-17: reopening the menu on the same row (Shift+F10) starts from the host's shortcut again:
// its key is marked and Shift is enabled.
#[test]
fn actions_panel_context_menu_reopens_from_the_hosts_shortcut() {
    let labels = ActionsLabels::english();
    let mut app = app(list());
    app.host.shortcut = Some(ShortcutChoice { key: 3, shift: false, command: false });
    let mut harness = panel_harness(app);
    item(&harness, "Sharpen").click();
    harness.run();
    for _ in 0..2 {
        press(&mut harness, Modifiers::SHIFT, Key::F10);
        assert_eq!(harness.get_by_role_and_label(Role::MenuItemRadio, "F3").accesskit_node().toggled(), Some(Toggled::True), "F3 is marked");
        assert!(!harness.get_by_label(&labels.shift).accesskit_node().is_disabled(), "Shift is enabled with a key");
        press(&mut harness, Modifiers::NONE, Key::Escape);
        assert!(events_of(&mut harness).is_empty());
        item(&harness, "Sharpen").click();
        harness.run();
    }
}

/// Loose "Loose", then the sets "Alpha" (one action) and "Beta" (one action).
fn two_sets() -> ActionList {
    let actions = vec![action("Loose", None, &["a"]), action("First", Some("Alpha"), &["b"]), action("Second", Some("Beta"), &["c"])];
    ActionList::from_file(ActionsFile { version: ActionsFile::VERSION, actions, sets: Vec::new() })
}

fn at(node: Node<'_>, share: f32) -> Pos2 {
    let rect = node.rect();
    Pos2::new(rect.center().x, rect.top() + rect.height() * share)
}

// nav M1 (LC57): a drop before a later set goes to the end of the set before it; before the first
// set it is loose.
#[test]
fn actions_panel_drop_before_a_later_set_lands_at_the_end_of_the_previous_set() {
    let mut harness = harness(app(two_sets()));
    harness.drag(at(item(&harness, "Loose"), 0.5), at(item(&harness, "Beta"), 0.1), Modifiers::NONE);
    let list = harness.state().list.clone();
    assert_eq!(
        moves(&list, &events_of(&mut harness)),
        [(ActionSelection::Action(0), ActionPlace::Action { index: 1, set: Some("Alpha".into()) })],
        "the drop line sits under Alpha's last action"
    );

    harness.drag(at(item(&harness, "Second"), 0.5), at(item(&harness, "Alpha"), 0.1), Modifiers::NONE);
    assert_eq!(
        moves(&list, &events_of(&mut harness)),
        [(ActionSelection::Action(2), ActionPlace::Action { index: 1, set: None })],
        "before the first set the action is loose, after the last loose one"
    );
}

// nav M2: the panel follows its selection through a move, for an action and for a step.
#[test]
fn actions_panel_selection_follows_a_move() {
    let mut app = app(list());
    app.state.expanded.insert("Sharpen".into());
    let mut harness = harness(app);
    item(&harness, "Vignette").click();
    harness.run();
    assert_eq!(harness.state().state.selected, Some(ActionSelection::Action(1)));
    harness.drag(at(item(&harness, "Vignette"), 0.5), at(item(&harness, "Sharpen"), 0.1), Modifiers::NONE);
    assert_eq!(harness.state().state.selected, Some(ActionSelection::Action(0)), "the selected action moved to the front");

    harness.state_mut().state.selected = Some(ActionSelection::Step(0, 1));
    harness.run();
    harness.drag(at(item(&harness, "Run blur"), 0.5), at(item(&harness, "Run levels"), 0.1), Modifiers::NONE);
    assert_eq!(harness.state().state.selected, Some(ActionSelection::Step(0, 2)), "a step the moved one passed shifts by one");
}

// A sweep over the include column is one gesture: Begin, then one Commit, so the app has one undo
// step; the events are the list's own.
#[test]
fn actions_panel_toggle_gesture_is_forwarded_whole() {
    let mut app = app(list());
    app.state.expanded.insert("Sharpen".into());
    let mut harness = harness(app);
    check(&harness, "Include in playback: Run levels").click();
    harness.run();
    let events = events_of(&mut harness);
    let begins = events.iter().filter(|event| matches!(event, ActionsEvent::Toggle(EditEvent::Begin { .. }))).count();
    let commits = events.iter().filter(|event| matches!(event, ActionsEvent::Toggle(EditEvent::Commit { .. }))).count();
    assert_eq!((begins, commits), (1, 1), "{events:?}");
    assert_eq!(toggles(&events), [(ActionSelection::Step(0, 0), 0, false)]);
}

// nav L6: an action with no steps shows no include box.
#[test]
fn actions_panel_an_action_without_steps_has_no_include_cell() {
    let actions = vec![action("Empty", None, &[]), action("Full", None, &["a"])];
    let list = ActionList::from_file(ActionsFile { version: ActionsFile::VERSION, actions, sets: Vec::new() });
    let harness = harness(app(list));
    assert!(harness.query_by_role_and_label(Role::CheckBox, "Include in playback: Empty").is_none());
    assert!(harness.query_by_role_and_label(Role::CheckBox, "Include in playback: Full").is_some());
}

// con §9: `start_rename` opens the field; the rename ends with the typed `text`.
#[test]
fn actions_panel_start_rename_opens_the_field() {
    let mut app = app(list());
    app.rename = Some(1);
    let mut harness = panel_harness(app);
    assert_eq!(harness.get_by_role(Role::TextInput).value().as_deref(), Some("Vignette"));
    harness.state_mut().rename = None;
    harness.event(Event::Text("X".into()));
    harness.run();
    press(&mut harness, Modifiers::NONE, Key::Enter);
    assert!(
        events_of(&mut harness)
            .iter()
            .any(|event| matches!(event, ActionsEvent::Rename { action: 1, text } if text.starts_with('V') || text.ends_with('X')))
    );
}

// A row of this list dropped on the footer's Delete reports `Delete` for the grabbed row, with
// nothing selected.
#[test]
fn actions_panel_a_row_dropped_on_delete_reports_delete() {
    let mut harness = panel_harness(app(list()));
    harness.drag(at(item(&harness, "Vignette"), 0.5), button(&harness, "Delete").rect().center(), Modifiers::NONE);
    assert!(events_of(&mut harness).contains(&ActionsEvent::Delete(ActionSelection::Action(1))));
}

// While recording, a dropped action is refused, as the button is.
#[test]
fn actions_panel_delete_refuses_a_dropped_action_while_recording() {
    let mut app = app(list());
    app.host.recording = Some(0);
    let mut harness = panel_harness(app);
    harness.drag(at(item(&harness, "Vignette"), 0.5), button(&harness, "Delete").rect().center(), Modifiers::NONE);
    assert!(!events_of(&mut harness).iter().any(|event| matches!(event, ActionsEvent::Delete(_))));
}
