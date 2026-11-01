//! Stories of the Actions panel (task ST7), not registered yet. Registering them (task RG2) adds
//! `stories()` to the facade's automation stories, [`ActionsStoryLabels`] to the words registry,
//! `COMPONENTS` (`"Actions panel"`) and regenerates the registry dump.
//!
//! Every story uses fakes: an `ActionList` built in the story, a fake `ActionsHost` (step labels
//! from the command id, a recording flag, pending steps), and story words (the gallery's
//! pseudo-locale test). The panel only reports events; the story applies each one to its own list,
//! state and host, so the panel shows the result. The story keeps its model in egui memory.
//!
//! | Story id | Shows |
//! |---|---|
//! | `patterns/automation/actions/sets` | two sets, a loose action, an expanded action with steps, one step off |
//! | `patterns/automation/actions/recording` | a recording row with pending steps and the dot |
//! | `patterns/automation/actions/button-mode` | the six colours, gray and none |
//! | `patterns/automation/actions/empty` | the empty state |

#![expect(dead_code, reason = "registered by task RG2")]

use std::collections::BTreeMap;

use craft_ui::localized::{Labels as _, SharedLabels as _};
use craft_ui::story::{Level, NoArgs, Story};
use craft_ui::typed_story;
use egui::{Id, Ui, vec2};
use serde_json::Value;

use super::{ActionPlace, ActionSelection, ActionsEvent, ActionsHost, ActionsLabels, ActionsPanel, ActionsState, ShortcutChoice};
use crate::actions::{Action, ActionColor, ActionList, ActionStep};

craft_ui::words! {
    /// The action, set and step names the Actions stories show (the host's words and the user's
    /// sample names, not the panel's).
    #[non_exhaustive]
    pub struct ActionsStoryLabels {
        sharpen: "actionsStory.sharpen" = "Sharpen";
        vignette: "actionsStory.vignette" = "Vignette";
        soft_glow: "actionsStory.softGlow" = "Soft Glow";
        retouch: "actionsStory.retouch" = "Skin Retouch";
        effects: "actionsStory.effects" = "Effects";
        portrait: "actionsStory.portrait" = "Portrait";
        /// The base of a new action's name; the list adds a number when it is taken.
        new_action: "actionsStory.newAction" = "New Action";
        /// A new set's name: `{n}` is a number.
        new_set: "actionsStory.newSet" = "New Set {n}";
        levels: "actionsStory.levels" = "Levels";
        curves: "actionsStory.curves" = "Curves";
        gaussian_blur: "actionsStory.gaussianBlur" = "Gaussian Blur";
        unsharp_mask: "actionsStory.unsharpMask" = "Unsharp Mask";
        add_mask: "actionsStory.addMask" = "Add Layer Mask";
        glow: "actionsStory.glow" = "Soft Glow Filter";
        hue: "actionsStory.hue" = "Hue/Saturation";
        brightness: "actionsStory.brightness" = "Brightness/Contrast";
    }
}

/// The four stories above.
pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!("patterns/automation/actions/sets", Level::Patterns, "Actions panel", "Sets", NoArgs {}, show_sets),
        typed_story!("patterns/automation/actions/recording", Level::Patterns, "Actions panel", "Recording", NoArgs {}, show_recording),
        typed_story!("patterns/automation/actions/button-mode", Level::Patterns, "Actions panel", "Button Mode", NoArgs {}, show_button_mode),
        typed_story!("patterns/automation/actions/empty", Level::Patterns, "Actions panel", "Empty", NoArgs {}, show_empty),
    ]
}

/// The panel's size in the stories: tall enough for the context menu (about 400 points).
const PANEL: egui::Vec2 = vec2(300.0, 460.0);

/// The app's side: step words from command ids, the recording and its pending steps, shortcut
/// texts by action name.
#[derive(Clone)]
struct StoryHost {
    words: ActionsStoryLabels,
    recording: Option<usize>,
    pending: Vec<ActionStep>,
    shortcuts: BTreeMap<String, String>,
}

impl ActionsHost for StoryHost {
    fn step_label(&self, step: &ActionStep) -> String {
        let words = &self.words;
        match step.command.as_str() {
            "levels" => words.levels.clone(),
            "curves" => words.curves.clone(),
            "blur" => words.gaussian_blur.clone(),
            "unsharp" => words.unsharp_mask.clone(),
            "mask" => words.add_mask.clone(),
            "glow" => words.glow.clone(),
            "hue" => words.hue.clone(),
            "brightness" => words.brightness.clone(),
            other => other.to_owned(),
        }
    }

    fn shortcut(&self, action: &Action) -> Option<String> {
        self.shortcuts.get(&action.name).cloned()
    }

    fn recording(&self) -> Option<usize> {
        self.recording
    }

    fn pending_steps(&self, action: usize) -> &[ActionStep] {
        if self.recording == Some(action) { &self.pending } else { &[] }
    }
}

/// What a story keeps between passes: the app's list and host, and the panel's state.
#[derive(Clone)]
struct Model {
    list: ActionList,
    state: ActionsState,
    host: StoryHost,
}

fn step(command: &str) -> ActionStep {
    ActionStep::new(command, Value::Null)
}

fn action(name: &str, set: Option<&str>, commands: &[&str]) -> Action {
    let mut action = Action::new(name, commands.iter().map(|command| step(command)).collect());
    action.set = set.map(str::to_owned);
    action
}

fn list_of(actions: Vec<Action>, sets: &[&str]) -> ActionList {
    let mut list = ActionList::new();
    for name in sets {
        let _ = list.new_set(name);
    }
    for action in actions {
        if let Ok(index) = list.new_action(&action.name, action.set.clone()) {
            let _ = list.append_recorded(index, action.steps.clone());
            for step in 0..action.steps.len() {
                if !action.step_enabled(step) {
                    let _ = list.toggle_step(index, step);
                }
            }
        }
    }
    list
}

fn host(words: ActionsStoryLabels) -> StoryHost {
    StoryHost { words, recording: None, pending: Vec::new(), shortcuts: BTreeMap::new() }
}

/// A set of sample actions: a loose one with three steps (the middle one off, expanded, with a
/// shortcut), and two sets.
fn sets_model(words: ActionsStoryLabels) -> Model {
    let mut sharpen = action(&words.sharpen, None, &["levels", "curves", "unsharp"]);
    sharpen.disabled.insert(1);
    let list = list_of(
        vec![
            sharpen,
            action(&words.vignette, Some(&words.effects), &["mask", "blur"]),
            action(&words.soft_glow, Some(&words.effects), &["glow"]),
            action(&words.retouch, Some(&words.portrait), &["hue", "brightness"]),
        ],
        &[&words.effects, &words.portrait],
    );
    let mut model = Model { list, state: ActionsState::default(), host: host(words.clone()) };
    model.state.expanded.insert(words.sharpen.clone());
    model.state.selected = Some(ActionSelection::Action(0));
    model.host.shortcuts.insert(words.sharpen, "F2".to_owned());
    model
}

/// One action being recorded into, with two recorded steps and two pending ones.
fn recording_model(words: ActionsStoryLabels) -> Model {
    let list = list_of(
        vec![action(&words.sharpen, None, &["levels", "unsharp"]), action(&words.retouch, Some(&words.portrait), &["hue", "curves"])],
        &[&words.portrait],
    );
    let mut host = host(words.clone());
    host.recording = Some(1);
    host.pending = vec![step("brightness"), step("blur")];
    let mut state = ActionsState::default();
    state.expanded.insert(words.retouch);
    state.selected = Some(ActionSelection::Action(1));
    Model { list, state, host }
}

/// One action per colour in Button Mode, named after its colour.
fn button_model(words: ActionsStoryLabels, labels: &ActionsLabels) -> Model {
    let names = [
        (&labels.none, ActionColor::None),
        (&labels.red, ActionColor::Red),
        (&labels.orange, ActionColor::Orange),
        (&labels.yellow, ActionColor::Yellow),
        (&labels.green, ActionColor::Green),
        (&labels.blue, ActionColor::Blue),
        (&labels.violet, ActionColor::Violet),
        (&labels.gray, ActionColor::Gray),
    ];
    let mut list = ActionList::new();
    for (name, color) in names {
        if let Ok(index) = list.new_action(name, None) {
            let _ = list.set_color(index, color);
        }
    }
    let mut state = ActionsState::default();
    state.settings.button_mode = true;
    Model { list, state, host: host(words) }
}

/// Applies what the user did to the story's own model, as the app would with the matching
/// `ActionList` calls. Play, the dialogs and the file commands have no app behind them.
fn apply(model: &mut Model, events: Vec<ActionsEvent>) {
    let Model { list, state, host } = model;
    for event in events {
        match event {
            ActionsEvent::BeginRecording { into } => {
                if let Ok(recording) = list.begin_recording(into, None, 0) {
                    host.recording = Some(recording.action);
                    host.pending.clear();
                    state.expanded.extend(list.action(recording.action).map(|action| action.name.clone()));
                }
            }
            ActionsEvent::NewAction => {
                let name = list.unique_name(&host.words.new_action);
                if let Ok(index) = list.new_action(&name, None) {
                    host.recording = Some(index);
                    host.pending.clear();
                    state.expanded.insert(name);
                }
            }
            ActionsEvent::StopRecording => {
                if let Some(recording) = host.recording.take() {
                    let _ = list.append_recorded(recording, std::mem::take(&mut host.pending));
                }
            }
            ActionsEvent::NewSet => {
                let name = (1..)
                    .map(|n| host.words.new_set.replace("{n}", &n.to_string()))
                    .find(|name| !list.sets().contains(&name.as_str()))
                    .unwrap_or_default();
                let _ = list.new_set(&name);
            }
            ActionsEvent::Delete(row) => {
                match row {
                    ActionSelection::Action(index) => {
                        let _ = list.delete_action(index);
                    }
                    ActionSelection::Step(index, at) => {
                        let _ = list.delete_step(index, at);
                    }
                    ActionSelection::Set(name) => {
                        let inside: Vec<usize> = (0..list.actions().len()).filter(|&index| list.set_of(index) == Some(name.as_str())).collect();
                        for index in inside.into_iter().rev() {
                            let _ = list.delete_action(index);
                        }
                    }
                }
                state.selected = None;
            }
            ActionsEvent::Rename { action, name } => {
                let old = list.action(action).map(|current| current.name.clone());
                if list.rename(action, &name, |_, _, _| {}).is_ok()
                    && let Some(old) = old
                    && state.expanded.remove(&old)
                {
                    state.expanded.insert(name.trim().to_owned());
                    if let Some(text) = host.shortcuts.remove(&old) {
                        host.shortcuts.insert(name.trim().to_owned(), text);
                    }
                }
            }
            ActionsEvent::Move { source, to } => match (source, to) {
                (ActionSelection::Action(from), ActionPlace::Action { index, set }) => {
                    if list.move_action(from, index).is_ok() {
                        let _ = list.assign_set(index, set);
                    }
                }
                (ActionSelection::Step(from, at), ActionPlace::Step { action, index }) => {
                    let _ = list.move_step(from, at, action, index);
                }
                _ => {}
            },
            ActionsEvent::ToggleStep { action, step } => {
                let _ = list.toggle_step(action, step);
            }
            ActionsEvent::ToggleAll { action } => {
                let _ = list.toggle_all(action);
            }
            ActionsEvent::ToggleDialog { action, step } => {
                let _ = list.toggle_dialog(action, step);
            }
            ActionsEvent::SetColor { action, color } => {
                let _ = list.set_color(action, color);
            }
            ActionsEvent::AssignShortcut { action, choice } => {
                if let Some(name) = list.action(action).map(|current| current.name.clone()) {
                    match choice {
                        Some(choice) => host.shortcuts.insert(name, shortcut_text(choice)),
                        None => host.shortcuts.remove(&name),
                    };
                }
            }
            _ => {}
        }
    }
}

/// "F2", "⇧F3", "⌘F4" as the panel's rows show them.
fn shortcut_text(choice: ShortcutChoice) -> String {
    format!("{}{}F{}", if choice.shift { "⇧" } else { "" }, if choice.command { "⌘" } else { "" }, choice.key)
}

/// Shows the panel over the model kept under `name`, seeded by `seed` on the first pass, and
/// applies its events.
fn show_model(ui: &mut Ui, name: &str, seed: impl FnOnce(ActionsStoryLabels, &ActionsLabels) -> Model) {
    let key = Id::NULL.with(("actions-story", name));
    let kept: Option<Model> = ui.data(|data| data.get_temp(key));
    let mut model = kept.unwrap_or_else(|| seed(ActionsStoryLabels::of(ui), &ActionsLabels::shared(ui)));
    let events = ui
        .allocate_ui(PANEL, |ui| {
            ui.set_min_size(PANEL);
            ActionsPanel::new(("actions-story-panel", name), &mut model.state, &model.list).host(&model.host).show(ui).events
        })
        .inner;
    apply(&mut model, events);
    ui.data_mut(|data| data.insert_temp(key, model));
}

fn show_sets(ui: &mut Ui, _: &mut NoArgs) {
    show_model(ui, "sets", |words, _| sets_model(words));
}

fn show_recording(ui: &mut Ui, _: &mut NoArgs) {
    show_model(ui, "recording", |words, _| recording_model(words));
}

fn show_button_mode(ui: &mut Ui, _: &mut NoArgs) {
    show_model(ui, "button-mode", button_model);
}

fn show_empty(ui: &mut Ui, _: &mut NoArgs) {
    show_model(ui, "empty", |words, _| Model { list: ActionList::new(), state: ActionsState::default(), host: host(words) });
}
