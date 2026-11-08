//! `ActionsPanel`: the rows, the cells, drags, Button Mode, the footer and the context menu.

use std::cell::RefCell;
use std::rc::Rc;

use craft_automation::actions::{ActionColor, ActionList, ActionsFile};
use craft_automation::ui::actions::mock::{FakeActionsHost, action, list_of, sample_list, step};
use craft_automation::ui::actions::{
    ActionMove, ActionPlace, ActionSelection, ActionsEvent, ActionsPanel, ActionsPanelState, ActionsSettings, DIALOG_COLUMN, INCLUDE_COLUMN,
    ShortcutChoice,
};
use craft_behavior::{EditEvent, Mixed};
use craft_ui::contract::Role;
use craft_ui::testing::{Fixture, Target, pointer, trial};
use egui::{Key, Modifiers, Pos2, Ui, Vec2};

/// The app around the panel: what it holds and what the panel reported.
struct App {
    state: ActionsPanelState,
    list: ActionList,
    host: FakeActionsHost,
    events: Vec<ActionsEvent>,
    settings_changed: bool,
    rename: Option<usize>,
}

type Shared = Rc<RefCell<App>>;

/// The room of the panel: wide enough for a row's parts and inside the 800 by 600 screen of a fixture.
const PANEL: Vec2 = Vec2::new(520.0, 520.0);

/// A pointer move that starts a drag.
const DRAG_STEP: Vec2 = Vec2::new(0.0, 8.0);

fn app(list: ActionList) -> App {
    App {
        state: ActionsPanelState::default(),
        list,
        host: FakeActionsHost { command_prefix: "Run ".to_owned(), ..FakeActionsHost::default() },
        events: Vec::new(),
        settings_changed: false,
        rename: None,
    }
}

fn show(ui: &mut Ui, app: &mut App) {
    let App { state, list, host, events, settings_changed, rename } = app;
    let mut panel = ActionsPanel::new(state, list).host(&*host);
    if let Some(action) = rename.take() {
        panel = panel.start_rename(action);
    }
    let out = panel.show(ui);
    *settings_changed |= out.settings_changed;
    events.extend(out.events);
}

fn rig_in(app: App, room: Vec2) -> (Fixture, Shared) {
    let shared = Rc::new(RefCell::new(app));
    let shown = Rc::clone(&shared);
    let mut fixture = Fixture::new(move |ui| show(ui, &mut shown.borrow_mut()));
    fixture.room(room);
    fixture.settle();
    (fixture, shared)
}

fn rig(app: App) -> (Fixture, Shared) {
    rig_in(app, PANEL)
}

fn take(shared: &Shared) -> Vec<ActionsEvent> {
    std::mem::take(&mut shared.borrow_mut().events)
}

/// The node named `label`.
fn node(fixture: &Fixture, label: &str) -> Target {
    fixture.by_label(label).unwrap_or_else(|| {
        let names: Vec<_> = fixture.tree().keys().into_iter().filter_map(|key| fixture.tree().name_of(key)).collect();
        panic!("no node named {label:?}; the tree has {names:?}")
    })
}

/// The point `share` of the way down `target`, in the middle of its width.
fn at(target: &Target, share: f32) -> Pos2 {
    Pos2::new(target.rect.center().x, target.rect.top() + target.rect.height() * share)
}

/// Press at `from`, move past the drag threshold and on to `to`, release there.
fn drag_to(fixture: &mut Fixture, from: Pos2, to: Pos2) {
    fixture.idle(6);
    fixture.frame(vec![pointer::moved(from)]);
    fixture.frame(vec![pointer::press(from)]);
    fixture.frame(vec![pointer::moved(from + DRAG_STEP)]);
    fixture.frame(vec![pointer::moved(to)]);
    fixture.frame(vec![pointer::moved(to)]);
    fixture.frame(vec![pointer::release(to)]);
    fixture.idle(2);
}

/// Right-click on `target`, then let the menu open.
fn context_click(fixture: &mut Fixture, target: &Target) {
    let pos = target.center();
    fixture.frame(vec![pointer::moved(pos)]);
    fixture.frame(vec![pointer::secondary(pos, true)]);
    fixture.frame(vec![pointer::secondary(pos, false)]);
    fixture.idle(3);
}

fn double_click(fixture: &mut Fixture, pos: Pos2) {
    fixture.frame(vec![pointer::moved(pos)]);
    for _ in 0..2 {
        fixture.frame(vec![pointer::press(pos)]);
        fixture.frame(vec![pointer::release(pos)]);
    }
    fixture.idle(2);
}

/// The cells the toggle gestures ended on (a `Commit` or a `Set`): row, column, state.
fn toggles(events: &[ActionsEvent]) -> Vec<(ActionSelection, &'static str, bool)> {
    events
        .iter()
        .filter_map(|event| match event {
            ActionsEvent::Toggle(EditEvent::Commit { after, .. } | EditEvent::Set { after, .. }) => {
                let column = if after.column == INCLUDE_COLUMN {
                    "include"
                } else if after.column == DIALOG_COLUMN {
                    "dialog"
                } else {
                    "other"
                };
                Some((after.key.clone(), column, after.on))
            }
            _ => None,
        })
        .collect()
}

/// The moves the panel reported: the dragged row and where it goes.
fn moves(events: &[ActionsEvent]) -> Vec<(ActionSelection, ActionPlace)> {
    events
        .iter()
        .filter_map(|event| match event {
            ActionsEvent::Reorder(ActionMove { row, place, .. }) => Some((row.clone(), place.clone())),
            _ => None,
        })
        .collect()
}

fn checked(fixture: &Fixture, label: &str) -> Option<Mixed<bool>> {
    fixture.tree().toggled_of(node(fixture, label).node)
}

/// The include cell reports a step and an action; a partial action shows the mixed mark and "2/3 steps".
#[trial]
fn actions_panel_include_column_events() {
    let mut app = app(sample_list());
    app.state.expanded.insert("Sharpen".into());
    app.list = {
        let mut file = app.list.to_file();
        file.actions[0].disabled.insert(1);
        file.actions[0].dialogs.insert(2);
        ActionList::from_file(file)
    };
    let (mut fixture, shared) = rig(app);
    assert_eq!(checked(&fixture, "Include in playback: Sharpen"), Some(Mixed::Mixed));
    assert_eq!(checked(&fixture, "Include in playback: Run levels"), Some(Mixed::Same(true)));
    assert_eq!(checked(&fixture, "Include in playback: Run curves"), Some(Mixed::Same(false)));
    assert_eq!(checked(&fixture, "Include in playback: Vignette"), Some(Mixed::Same(true)));
    let painted = |fixture: &Fixture, text: &str| fixture.texts().iter().any(|painted| painted.text == text);
    assert!(painted(&fixture, "2/3 steps"), "the partial action says how many steps are on");
    assert!(painted(&fixture, "2 steps"), "a whole action only counts them");

    for label in ["Include in playback: Run curves", "Include in playback: Sharpen", "Show dialog during playback: Run blur"] {
        let cell = node(&fixture, label);
        fixture.click(&cell);
        fixture.idle(2);
    }
    assert_eq!(
        toggles(&take(&shared)),
        [
            (ActionSelection::Step(0, 1), "include", true),
            (ActionSelection::Action(0), "include", true),
            (ActionSelection::Step(0, 2), "dialog", false)
        ]
    );
}

/// Dragging an action, an action into a set and a step reports the place.
#[trial]
fn actions_panel_drag_reports_move() {
    let mut app = app(sample_list());
    app.state.expanded.insert("Sharpen".into());
    let (mut fixture, shared) = rig(app);

    let (from, to) = (node(&fixture, "Vignette").center(), at(&node(&fixture, "Sharpen"), 0.1));
    drag_to(&mut fixture, from, to);
    assert_eq!(
        moves(&take(&shared)),
        [(ActionSelection::Action(1), ActionPlace::Action { index: 0, set: None })],
        "an action dropped above another takes its place"
    );

    let (from, to) = (node(&fixture, "Vignette").center(), node(&fixture, "Effects").center());
    drag_to(&mut fixture, from, to);
    assert_eq!(
        moves(&take(&shared)),
        [(ActionSelection::Action(1), ActionPlace::Action { index: 1, set: Some("Effects".into()) })],
        "dropped on a set it goes into it"
    );

    let (from, to) = (node(&fixture, "Run blur").center(), at(&node(&fixture, "Run levels"), 0.1));
    drag_to(&mut fixture, from, to);
    assert_eq!(moves(&take(&shared)), [(ActionSelection::Step(0, 2), ActionPlace::Step { action: 0, index: 0 })], "a step dropped above another");
}

/// In Button Mode a click reports one `Play`, a double click no second one; the tree comes back while recording.
#[trial]
fn actions_panel_button_mode_plays_on_click_once() {
    let mut app = app(sample_list());
    app.state.settings.button_mode = true;
    let (mut fixture, shared) = rig(app);
    assert!(fixture.by_role(Role::TREE_ITEM).is_empty(), "the buttons replace the tree");
    let button = node(&fixture, "Vignette");
    fixture.click(&button);
    fixture.idle(2);
    assert_eq!(take(&shared), [ActionsEvent::Play { action: 1, from: 0 }]);
    let glow = node(&fixture, "Glow").center();
    double_click(&mut fixture, glow);
    assert_eq!(take(&shared), [ActionsEvent::Play { action: 2, from: 0 }], "a double click plays once");

    shared.borrow_mut().host.recording = Some(0);
    fixture.idle(3);
    assert!(!fixture.by_role(Role::TREE_ITEM).is_empty(), "Button Mode is off while recording");
}

/// The recording action opens, shows its pending steps and scrolls its latest into view.
#[trial]
fn actions_panel_recording_row_expands_and_follows() {
    let mut app = app(sample_list());
    app.host.recording = Some(0);
    app.host.pending = vec![step("pending-1"), step("pending-2")];
    let (mut fixture, shared) = rig_in(app, Vec2::new(520.0, 120.0));
    assert!(fixture.by_label("Sharpen, recording").is_some(), "the recording row says so");
    let footer = node(&fixture, "Delete").rect;
    let latest = node(&fixture, "Run pending-2").rect;
    assert!(latest.bottom() <= footer.top() + 0.5, "the latest step is in view: {latest:?} above {footer:?}");
    let below = fixture.by_label("Vignette").map(|target| target.rect);
    assert!(below.is_none_or(|rect| rect.top() >= footer.top() - 0.5), "the rows below are scrolled away: {below:?}");

    shared.borrow_mut().host.pending.push(step("pending-3"));
    fixture.idle(3);
    let latest = node(&fixture, "Run pending-3").rect;
    assert!(latest.bottom() <= footer.top() + 0.5, "a new step scrolls into view: {latest:?} above {footer:?}");
}

/// An empty list shows the empty text; an empty set shows its header.
#[trial]
fn actions_panel_empty_state_and_empty_set() {
    let (mut fixture, shared) = rig(app(ActionList::new()));
    let text = "Record a sequence of edits, then play it on any document.";
    assert!(fixture.by_label(text).is_some());
    assert!(fixture.by_role(Role::TREE_ITEM).is_empty());

    shared.borrow_mut().list = ActionList::from_file(ActionsFile { version: ActionsFile::VERSION, actions: Vec::new(), sets: vec!["Empty".into()] });
    fixture.idle(3);
    assert!(fixture.by_label(text).is_none());
    assert!(fixture.by_label("Empty").is_some(), "an empty set keeps its header");
}

/// Play with nothing selected plays the first action; a selected step plays its action from that step.
#[trial]
fn actions_panel_play_with_no_selection_plays_the_first() {
    let (mut fixture, shared) = rig(app(sample_list()));
    let play = node(&fixture, "Play selection");
    fixture.click(&play);
    fixture.idle(2);
    assert_eq!(take(&shared), [ActionsEvent::Play { action: 0, from: 0 }]);

    shared.borrow_mut().state.expanded.insert("Vignette".into());
    fixture.idle(3);
    let step_row = node(&fixture, "Run blur");
    fixture.click(&step_row);
    fixture.idle(2);
    take(&shared);
    fixture.click(&play);
    fixture.idle(2);
    assert_eq!(take(&shared), [ActionsEvent::Play { action: 1, from: 1 }], "a selected step plays its action from that step");

    let delete = node(&fixture, "Delete");
    fixture.click(&delete);
    fixture.idle(2);
    assert_eq!(take(&shared), [ActionsEvent::Delete(ActionSelection::Step(1, 1))]);
    let new_set = node(&fixture, "Create new set");
    fixture.click(&new_set);
    fixture.idle(2);
    assert_eq!(take(&shared), [ActionsEvent::NewSet]);
    let mode = node(&fixture, "Button Mode");
    fixture.click(&mode);
    fixture.idle(2);
    assert!(shared.borrow().state.settings.button_mode, "Button Mode toggles in the state");
    assert!(take(&shared).is_empty());
    assert!(shared.borrow().settings_changed, "and the output says the settings changed");
}

/// A set's disclosure changes the persisted settings and the output says so.
#[trial]
fn actions_panel_set_disclosure_reports_settings_changed() {
    let (mut fixture, shared) = rig(app(sample_list()));
    assert!(!shared.borrow().settings_changed);
    let set = node(&fixture, "Effects");
    fixture.click(&set);
    fixture.key(Key::ArrowLeft, Modifiers::NONE);
    fixture.idle(2);
    assert_eq!(shared.borrow().state.settings.collapsed_sets, ["Effects"]);
    assert!(shared.borrow().settings_changed);
}

/// Record stays live while recording: pressed, and its click starts nothing.
#[trial]
fn actions_panel_record_button_shows_the_recording_and_ignores_the_click() {
    let (idle, _) = rig(app(sample_list()));
    assert_ne!(checked(&idle, "Begin recording"), Some(Mixed::Same(true)), "Record is not pressed while idle");
    let mut recording = app(sample_list());
    recording.host.recording = Some(0);
    let (mut fixture, shared) = rig(recording);
    let record = node(&fixture, "Begin recording");
    assert_eq!(checked(&fixture, "Begin recording"), Some(Mixed::Same(true)), "Record shows the recording");
    assert!(!fixture.tree().is_disabled_key(record.node), "and stays live");
    fixture.click(&record);
    fixture.idle(2);
    assert!(take(&shared).is_empty(), "a click while recording starts nothing");
}

/// The context menu reports the shortcut with Shift and Command ticked as they are made, the colour, and Save Set.
#[trial]
fn actions_panel_context_menu_shortcut_and_color() {
    let mut app = app(sample_list());
    app.host.shortcuts.insert("Sharpen".to_owned(), ShortcutChoice { key: 3, shift: false, command: false });
    let (mut fixture, shared) = rig(app);
    let assign = |action: usize, key: u8, shift: bool, command: bool| ActionsEvent::AssignShortcut {
        action,
        choice: Some(ShortcutChoice { key, shift, command }),
    };
    let click = |fixture: &mut Fixture, label: &str| {
        let row = node(fixture, label);
        fixture.click(&row);
        fixture.idle(2);
    };

    // Check rows keep the menu open, so both can be ticked; each tick is reported at once.
    let row = node(&fixture, "Sharpen");
    context_click(&mut fixture, &row);
    click(&mut fixture, "Shift");
    assert_eq!(take(&shared), [assign(0, 3, true, false)]);
    click(&mut fixture, if cfg!(target_os = "macos") { "Command" } else { "Ctrl" });
    assert_eq!(take(&shared), [assign(0, 3, true, true)]);
    fixture.key(Key::Escape, Modifiers::NONE);
    fixture.idle(3);
    assert!(take(&shared).is_empty(), "closing the menu reports nothing more");

    // A key row reports at once, with the ticks the menu opened with.
    let row = node(&fixture, "Vignette");
    context_click(&mut fixture, &row);
    click(&mut fixture, "F7");
    assert_eq!(take(&shared), [assign(1, 7, false, false)]);
    let row = node(&fixture, "Vignette");
    context_click(&mut fixture, &row);
    click(&mut fixture, "None");
    assert_eq!(take(&shared), [ActionsEvent::AssignShortcut { action: 1, choice: None }]);

    let row = node(&fixture, "Vignette");
    context_click(&mut fixture, &row);
    click(&mut fixture, "Color");
    click(&mut fixture, "Red");
    assert_eq!(take(&shared), [ActionsEvent::SetColor { action: 1, color: ActionColor::Red }]);

    let row = node(&fixture, "Effects");
    context_click(&mut fixture, &row);
    click(&mut fixture, "Save Set…");
    assert_eq!(take(&shared), [ActionsEvent::Save { set: Some("Effects".to_owned()) }]);
}

/// Reopening the menu on the same row starts from the host's shortcut again: its key is marked and Shift
/// is enabled.
#[trial]
fn actions_panel_context_menu_reopens_from_the_hosts_shortcut() {
    let mut app = app(sample_list());
    app.host.shortcuts.insert("Sharpen".to_owned(), ShortcutChoice { key: 3, shift: false, command: false });
    let (mut fixture, shared) = rig(app);
    for _ in 0..2 {
        let row = node(&fixture, "Sharpen");
        context_click(&mut fixture, &row);
        assert_eq!(checked(&fixture, "F3"), Some(Mixed::Same(true)), "F3 is marked");
        assert!(!fixture.tree().is_disabled_key(node(&fixture, "Shift").node), "Shift is enabled with a key");
        fixture.key(Key::Escape, Modifiers::NONE);
        fixture.idle(3);
        assert!(take(&shared).is_empty());
    }
}

/// Loose "Loose", then the sets "Alpha" (one action) and "Beta" (one action).
fn two_sets() -> ActionList {
    list_of(vec![action("Loose", None, &["a"]), action("First", Some("Alpha"), &["b"]), action("Second", Some("Beta"), &["c"])])
}

/// A drop before a later set goes to the end of the set before it; before the first set it is loose.
#[trial]
fn actions_panel_drop_before_a_later_set_lands_at_the_end_of_the_previous_set() {
    let (mut fixture, shared) = rig(app(two_sets()));
    let (from, to) = (node(&fixture, "Loose").center(), at(&node(&fixture, "Beta"), 0.1));
    drag_to(&mut fixture, from, to);
    assert_eq!(
        moves(&take(&shared)),
        [(ActionSelection::Action(0), ActionPlace::Action { index: 1, set: Some("Alpha".into()) })],
        "the drop line sits under Alpha's last action"
    );

    let (from, to) = (node(&fixture, "Second").center(), at(&node(&fixture, "Alpha"), 0.1));
    drag_to(&mut fixture, from, to);
    assert_eq!(
        moves(&take(&shared)),
        [(ActionSelection::Action(2), ActionPlace::Action { index: 1, set: None })],
        "before the first set the action is loose, after the last loose one"
    );
}

/// The panel follows its selection through a move, for an action and for a step.
#[trial]
fn actions_panel_selection_follows_a_move() {
    let mut app = app(sample_list());
    app.state.expanded.insert("Sharpen".into());
    let (mut fixture, shared) = rig(app);
    let row = node(&fixture, "Vignette");
    fixture.click(&row);
    fixture.idle(2);
    assert_eq!(shared.borrow().state.selected, Some(ActionSelection::Action(1)));
    let (from, to) = (node(&fixture, "Vignette").center(), at(&node(&fixture, "Sharpen"), 0.1));
    drag_to(&mut fixture, from, to);
    assert_eq!(shared.borrow().state.selected, Some(ActionSelection::Action(0)), "the selected action moved to the front");

    shared.borrow_mut().state.selected = Some(ActionSelection::Step(0, 1));
    fixture.idle(3);
    let (from, to) = (node(&fixture, "Run blur").center(), at(&node(&fixture, "Run levels"), 0.1));
    drag_to(&mut fixture, from, to);
    assert_eq!(shared.borrow().state.selected, Some(ActionSelection::Step(0, 0)), "a pressed step is selected and follows its move");
}

/// A press on the include column is one gesture: a Begin and a Commit, so the app has one undo step; the events are
/// the list's own.
#[trial]
fn actions_panel_toggle_gesture_is_forwarded_whole() {
    let mut app = app(sample_list());
    app.state.expanded.insert("Sharpen".into());
    let (mut fixture, shared) = rig(app);
    let cell = node(&fixture, "Include in playback: Run levels");
    fixture.click(&cell);
    fixture.idle(2);
    let events = take(&shared);
    let begins = events.iter().filter(|event| matches!(event, ActionsEvent::Toggle(EditEvent::Begin { .. }))).count();
    let commits = events.iter().filter(|event| matches!(event, ActionsEvent::Toggle(EditEvent::Commit { .. }))).count();
    assert_eq!((begins, commits), (1, 1), "{events:?}");
    assert_eq!(toggles(&events), [(ActionSelection::Step(0, 0), "include", false)]);
}

/// An action with no steps shows no include box.
#[trial]
fn actions_panel_an_action_without_steps_has_no_include_cell() {
    let list = list_of(vec![action("Empty", None, &[]), action("Full", None, &["a"])]);
    let (fixture, _) = rig(app(list));
    assert!(fixture.by_label("Include in playback: Empty").is_none());
    assert!(fixture.by_label("Include in playback: Full").is_some());
}

/// `start_rename` opens the field; the rename ends with the typed text.
#[trial]
fn actions_panel_start_rename_opens_the_field() {
    let mut app = app(sample_list());
    app.rename = Some(1);
    let (mut fixture, shared) = rig(app);
    fixture.idle(2);
    assert!(!fixture.by_role(Role::TEXT_INPUT).is_empty(), "the rename field is open");
    fixture.type_text("X");
    fixture.key(Key::Enter, Modifiers::NONE);
    fixture.idle(2);
    assert!(
        take(&shared).iter().any(|event| matches!(event, ActionsEvent::Rename { action: 1, text } if text.starts_with('V') || text.ends_with('X')))
    );
}

/// A row of this list dropped on the footer's Delete reports `Delete` for the grabbed row, with nothing selected.
#[trial]
fn actions_panel_a_row_dropped_on_delete_reports_delete() {
    let (mut fixture, shared) = rig(app(sample_list()));
    let (from, to) = (node(&fixture, "Vignette").center(), node(&fixture, "Delete").center());
    drag_to(&mut fixture, from, to);
    assert!(take(&shared).contains(&ActionsEvent::Delete(ActionSelection::Action(1))));
}

/// While recording, a dropped action is refused, as the button is.
#[trial]
fn actions_panel_delete_refuses_a_dropped_action_while_recording() {
    let mut app = app(sample_list());
    app.host.recording = Some(0);
    let (mut fixture, shared) = rig(app);
    let (from, to) = (node(&fixture, "Vignette").center(), node(&fixture, "Delete").center());
    drag_to(&mut fixture, from, to);
    assert!(!take(&shared).iter().any(|event| matches!(event, ActionsEvent::Delete(_))));
}

/// `ActionsSettings` round-trips through JSON in camelCase and ignores other apps' extra fields.
#[trial]
fn actions_settings_survive_a_serde_round_trip() {
    let settings = ActionsSettings { button_mode: true, collapsed_sets: vec!["Effects".into(), "Old".into()] };
    let text = serde_json::to_string(&settings).expect("serialized");
    assert_eq!(text, r#"{"buttonMode":true,"collapsedSets":["Effects","Old"]}"#);
    assert_eq!(serde_json::from_str::<ActionsSettings>(&text).expect("read back"), settings);

    let foreign = r#"{"buttonMode":true,"selected":"Sharpen","selectedStep":2,"expanded":["Sharpen"],"collapsedSets":["Effects"]}"#;
    let read: ActionsSettings = serde_json::from_str(foreign).expect("read");
    assert_eq!(read, ActionsSettings { button_mode: true, collapsed_sets: vec!["Effects".into()] });
    assert_eq!(serde_json::from_str::<ActionsSettings>("{}").expect("read"), ActionsSettings::default());
}
