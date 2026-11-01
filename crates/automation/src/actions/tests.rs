//! The action list, its file and the player.

use serde_json::{Value, json};

use super::*;

fn step(command: &str) -> ActionStep {
    ActionStep::new(command, Value::Null)
}

fn action(name: &str, commands: &[&str]) -> Action {
    Action::new(name, commands.iter().map(|c| step(c)).collect())
}

fn list_of(actions: Vec<Action>) -> ActionList {
    ActionList::from_file(ActionsFile { version: ActionsFile::VERSION, actions, sets: Vec::new() })
}

/// Runs steps by recording their ids; `fail`, `pause` and `deny` name the commands that fail,
/// pause or are refused by the gate.
#[derive(Default)]
struct Script {
    ran: Vec<String>,
    fail: Option<&'static str>,
    pause: Option<&'static str>,
    deny: Option<&'static str>,
}

impl ActionRunner for Script {
    fn run_step(&mut self, _: &Action, _: usize, step: &ActionStep, _: bool, _: &mut PlaybackBudget) -> StepOutcome {
        if self.fail == Some(step.command.as_str()) {
            return StepOutcome::Failed("boom".to_owned());
        }
        if self.pause == Some(step.command.as_str()) {
            self.pause = None;
            return StepOutcome::Paused;
        }
        self.ran.push(step.command.clone());
        StepOutcome::Done(Value::Null)
    }

    fn authorize(&mut self, step: &ActionStep) -> Result<(), String> {
        if self.deny == Some(step.command.as_str()) { Err("denied".to_owned()) } else { Ok(()) }
    }
}

// `moved_index` cases (the moved item, items between, items outside); `move_step` within an action
// remaps `disabled` and `dialogs`, and between actions carries the flags along.
#[test]
fn action_list_moves_follow_the_selection() {
    assert_eq!(moved_index(1, 1, 3), 3);
    assert_eq!(moved_index(2, 1, 3), 1);
    assert_eq!(moved_index(2, 3, 1), 3);
    assert_eq!(moved_index(0, 1, 3), 0);
    assert_eq!(moved_index(4, 1, 3), 4);

    let mut list = list_of(vec![action("A", &["a", "b", "c"]), action("B", &["x"])]);
    list.actions[0].disabled.insert(0);
    list.actions[0].dialogs.insert(2);
    list.move_step(0, 0, 0, 2).unwrap();
    let commands: Vec<_> = list.actions[0].steps.iter().map(|s| s.command.as_str()).collect();
    assert_eq!(commands, ["b", "c", "a"]);
    assert_eq!(list.actions[0].disabled.iter().copied().collect::<Vec<_>>(), [2]);
    assert_eq!(list.actions[0].dialogs.iter().copied().collect::<Vec<_>>(), [1]);

    list.move_step(0, 2, 1, 0).unwrap();
    assert_eq!(list.actions[0].steps.len(), 2);
    assert!(list.actions[0].disabled.is_empty());
    assert_eq!(list.actions[1].steps[0].command, "a");
    assert!(list.actions[1].disabled.contains(&0));
    assert_eq!(list.move_step(0, 0, 1, 5), Err(ActionListError::OutOfRange));

    list.move_action(0, 1).unwrap();
    assert_eq!(list.actions[1].name, "A");
}

// A taken or empty name is refused; the same name changes nothing; `calls` sees every step with
// the old and new names.
#[test]
fn action_list_rename_keeps_names_unique_and_updates_callers() {
    let mut list = list_of(vec![action("A", &["x"]), action("B", &["play"])]);
    assert_eq!(list.rename(0, "B", |_, _, _| {}), Err(ActionListError::NameTaken("B".to_owned())));
    assert_eq!(list.rename(0, "  ", |_, _, _| {}), Err(ActionListError::EmptyName));
    let revision = list.revision();
    list.rename(0, "A", |_, _, _| panic!("unchanged name")).unwrap();
    assert_eq!(list.revision(), revision);

    let mut seen = Vec::new();
    list.rename(0, "C", |step, old, new| seen.push((step.command.clone(), old.to_owned(), new.to_owned()))).unwrap();
    assert_eq!(list.actions[0].name, "C");
    assert_eq!(seen, [("x".to_owned(), "A".to_owned(), "C".to_owned()), ("play".to_owned(), "A".to_owned(), "C".to_owned())]);
    assert_eq!(list.unique_name("C"), "C 2");
    assert_eq!(list.unique_name(""), "Action 3");
}

// PhotoCraft's `actions.json` shape (steps as `[id, params]`, colour names, no `sets`) loads and
// saves unchanged; empty sets survive in `sets`.
#[test]
fn action_list_file_round_trip_keeps_pc_json() {
    let text = r#"{"version":1,"actions":[{"name":"A","steps":[["image.levels",{"gamma":1.2}]],"disabled":[0],"set":"S","color":"red"},{"name":"B","steps":[]}]}"#;
    let file = ActionsFile::from_json(text).unwrap();
    assert_eq!(file.actions[0].color, ActionColor::Red);
    assert_eq!(file.actions[0].steps[0], ActionStep::new("image.levels", json!({"gamma": 1.2})));
    let mut list = ActionList::from_file(file);
    let saved: Value = serde_json::from_str(&list.to_file().to_json().unwrap()).unwrap();
    assert_eq!(saved, serde_json::from_str::<Value>(text).unwrap());

    list.new_set("Empty").unwrap();
    assert_eq!(list.sets(), ["S", "Empty"]);
    let back = ActionsFile::from_json(&list.to_file().to_json().unwrap()).unwrap();
    assert_eq!(back.sets, ["Empty"]);
    assert_eq!(ActionList::from_file(back).sets(), ["S", "Empty"]);
}

// `toggle_all` turns every step on unless all already are, then every step off.
#[test]
fn action_list_toggle_all_rule() {
    let mut list = list_of(vec![action("A", &["a", "b", "c"])]);
    list.toggle_step(0, 1).unwrap();
    assert_eq!(list.actions[0].include(), ActionInclude::Partial);
    assert_eq!(list.toggle_all(0), Ok(true));
    assert_eq!(list.actions[0].include(), ActionInclude::All);
    assert_eq!(list.toggle_all(0), Ok(false));
    assert_eq!(list.actions[0].include(), ActionInclude::None);
    assert_eq!(list.toggle_all(0), Ok(true));
}

// Playback stops at the first failure, skips disabled steps uncharged, and reports `ran`.
#[test]
fn action_player_stops_at_first_error_and_skips_disabled() {
    let mut list = list_of(vec![action("A", &["a", "b", "c", "d"])]);
    list.toggle_step(0, 1).unwrap();
    let mut runner = Script { fail: Some("c"), ..Script::default() };
    let mut budget = PlaybackBudget::new();
    let report = ActionPlayer::play(&list, 0, 0, &mut runner, &mut budget);
    assert_eq!(report.ran, 1);
    assert_eq!(report.failed.as_ref().map(|f| (f.step, f.message.as_str())), Some((2, "boom")));
    assert_eq!(runner.ran, ["a"]);
    assert_eq!(budget.steps(), 2);

    let report = ActionPlayer::play(&list, 5, 0, &mut runner, &mut budget);
    assert_eq!(report.failed.map(|f| f.message), Some("no such action".to_owned()));
}

// One budget is charged across nested plays and stops at the limit; a call back to a running
// action is refused as `Recursive`.
#[test]
fn action_player_charges_a_shared_budget_and_detects_loops() {
    struct Nested<'a>(&'a ActionList, Vec<PlaybackRefusal>);
    impl ActionRunner for Nested<'_> {
        fn run_step(&mut self, _: &Action, _: usize, step: &ActionStep, _: bool, budget: &mut PlaybackBudget) -> StepOutcome {
            if let Some(callee) = step.command.strip_prefix("play:") {
                let index = self.0.actions().iter().position(|a| a.name == callee).unwrap();
                let report = ActionPlayer::play(self.0, index, 0, self, budget);
                self.1.extend(report.refused);
            }
            StepOutcome::Done(Value::Null)
        }
    }
    let list = list_of(vec![action("A", &["x", "play:B", "y"]), action("B", &["z", "play:A"])]);
    let mut runner = Nested(&list, Vec::new());
    let mut budget = PlaybackBudget::new();
    let report = ActionPlayer::play(&list, 0, 0, &mut runner, &mut budget);
    assert_eq!(report.ran, 3);
    assert_eq!(runner.1, [PlaybackRefusal::Recursive("A".to_owned())]);
    assert_eq!(budget.steps(), 5);

    let mut small = PlaybackBudget { limit: 2, ..PlaybackBudget::new() };
    let report = ActionPlayer::play(&list, 0, 0, &mut Script::default(), &mut small);
    assert_eq!(report.ran, 2);
    assert_eq!(report.refused, Some(PlaybackRefusal::StepLimit));
    assert_eq!(report.failed.map(|f| f.step), Some(2));

    let mut deep = PlaybackBudget::new();
    for n in 0..MAX_PLAYBACK_DEPTH {
        deep.enter(&n.to_string()).unwrap();
    }
    assert_eq!(deep.enter("more"), Err(PlaybackRefusal::TooDeep));
}

// An `authorize` denial neither runs nor pauses the step.
#[test]
fn action_player_gates_each_step() {
    let list = list_of(vec![action("A", &["a", "b", "c"])]);
    let mut runner = Script { deny: Some("b"), pause: Some("b"), ..Script::default() };
    let report = ActionPlayer::play(&list, 0, 0, &mut runner, &mut PlaybackBudget::new());
    assert_eq!(report.ran, 1);
    assert_eq!(report.failed.map(|f| (f.step, f.message)), Some((1, "denied".to_owned())));
    assert_eq!(report.paused_at, None);
    assert_eq!(runner.ran, ["a"]);
}

// `Paused` reports `paused_at`; playing again from the next step finishes the action.
#[test]
fn action_player_pauses_for_a_dialog_step() {
    let list = list_of(vec![action("A", &["a", "b", "c"])]);
    let mut runner = Script { pause: Some("b"), ..Script::default() };
    let report = ActionPlayer::play(&list, 0, 0, &mut runner, &mut PlaybackBudget::new());
    assert_eq!((report.ran, report.paused_at), (1, Some(1)));
    let report = ActionPlayer::play(&list, 0, 2, &mut runner, &mut PlaybackBudget::new());
    assert_eq!((report.ran, report.paused_at, report.failed), (1, None, None));
    assert_eq!(runner.ran, ["a", "c"]);
}

// The panel's toggle events carry the state a cell takes: the `set_*` calls are idempotent and
// bump the revision only when they change something.
#[test]
fn action_list_set_calls_are_idempotent() {
    let mut list = list_of(vec![action("A", &["a", "b"])]);
    let start = list.revision();
    list.set_step(0, 1, true).unwrap();
    list.set_all(0, true).unwrap();
    list.set_dialog(0, 1, false).unwrap();
    assert_eq!(list.revision(), start, "nothing changed");
    list.set_step(0, 1, false).unwrap();
    assert_eq!(list.actions[0].include(), ActionInclude::Partial);
    list.set_dialog(0, 0, true).unwrap();
    assert!(list.actions[0].dialogs.contains(&0));
    list.set_all(0, false).unwrap();
    assert_eq!(list.actions[0].include(), ActionInclude::None);
    assert_eq!(list.set_step(0, 5, true), Err(ActionListError::NoSuchStep { action: 0, step: 5 }));
}

// Every colour but `None` maps to its own label colour, so the shared colour words name it.
#[test]
fn action_colors_map_to_label_colors() {
    assert_eq!(ActionColor::None.label_color(), None);
    let mapped: Vec<_> = ActionColor::ALL.iter().skip(1).map(|color| color.label_color().map(|label| label.name())).collect();
    let names: Vec<_> = ActionColor::ALL.iter().skip(1).map(|color| Some(color.name())).collect();
    assert_eq!(mapped, names);
}
