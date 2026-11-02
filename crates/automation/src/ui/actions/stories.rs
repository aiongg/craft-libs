//! Stories of the Actions panel.
//!
//! Every story uses fakes: an `ActionList` built in the story, a [`FakeActionsHost`] (step labels
//! from the command id, a recording flag, pending steps), and story words (the gallery's
//! pseudo-locale test). The panel only reports events; the story applies each one to its own list,
//! state and host, so the panel shows the result. The story keeps its model in egui memory.
//!
//! | Story id | Shows |
//! |---|---|
//! | `patterns/actions-panel/sets` | two sets, a loose action, an expanded action with steps, one step off |
//! | `patterns/actions-panel/recording` | a recording row with pending steps and the dot |
//! | `patterns/actions-panel/button-mode` | the six colours, gray and none |
//! | `patterns/actions-panel/empty` | the empty state |

use std::collections::BTreeMap;

use craft_ui::edit::EditEvent;
use craft_ui::localized::{LabelColorLabels, Labels as _, SharedLabels as _, fmt};
use craft_ui::story::{Level, NoArgs, Story, lock, shared};
use craft_ui::typed_story;
use egui::{Ui, vec2};
use serde_json::Value;

use super::{ActionPlace, ActionSelection, ActionsEvent, ActionsHost, ActionsPanel, ActionsPanelState, ShortcutChoice};
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
/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["ActionsPanel"];

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!("patterns/actions-panel/sets", Level::Patterns, "ActionsPanel", "Sets", NoArgs {}, show_sets),
        typed_story!("patterns/actions-panel/recording", Level::Patterns, "ActionsPanel", "Recording", NoArgs {}, show_recording),
        typed_story!("patterns/actions-panel/button-mode", Level::Patterns, "ActionsPanel", "Button Mode", NoArgs {}, show_button_mode),
        typed_story!("patterns/actions-panel/empty", Level::Patterns, "ActionsPanel", "Empty", NoArgs {}, show_empty),
    ]
}

/// The panel's height in the stories: tall enough for the context menu (about 400 points). Its
/// width is the tile's.
const PANEL_HEIGHT: f32 = 460.0;

/// The app's side for stories and tests: step labels by command id (a command without one shows
/// as `command_prefix` and the id), the recording and its pending steps, shortcuts by action name.
#[derive(Clone, Default)]
pub(crate) struct FakeActionsHost {
    pub(crate) step_labels: BTreeMap<String, String>,
    pub(crate) command_prefix: String,
    pub(crate) recording: Option<usize>,
    pub(crate) pending: Vec<ActionStep>,
    pub(crate) shortcuts: BTreeMap<String, ShortcutChoice>,
}

impl ActionsHost for FakeActionsHost {
    fn step_label(&self, step: &ActionStep) -> String {
        self.step_labels.get(&step.command).cloned().unwrap_or_else(|| format!("{}{}", self.command_prefix, step.command))
    }

    fn shortcut(&self, action: &Action) -> Option<ShortcutChoice> {
        self.shortcuts.get(&action.name).copied()
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
    state: ActionsPanelState,
    host: FakeActionsHost,
    new_action: String,
    new_set: String,
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

fn host(words: &ActionsStoryLabels) -> FakeActionsHost {
    let step_labels = [
        ("levels", &words.levels),
        ("curves", &words.curves),
        ("blur", &words.gaussian_blur),
        ("unsharp", &words.unsharp_mask),
        ("mask", &words.add_mask),
        ("glow", &words.glow),
        ("hue", &words.hue),
        ("brightness", &words.brightness),
    ]
    .into_iter()
    .map(|(command, label)| (command.to_owned(), label.clone()))
    .collect();
    FakeActionsHost { step_labels, ..FakeActionsHost::default() }
}

fn new_model(list: ActionList, state: ActionsPanelState, host: FakeActionsHost, words: &ActionsStoryLabels) -> Model {
    Model { list, state, host, new_action: words.new_action.clone(), new_set: words.new_set.clone() }
}

/// A set of sample actions: a loose one with three steps (the middle one off, expanded, with a
/// shortcut), and two sets.
fn sets_model(words: &ActionsStoryLabels) -> Model {
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
    let mut model = new_model(list, ActionsPanelState::default(), host(words), words);
    model.state.expanded.insert(words.sharpen.clone());
    model.state.selected = Some(ActionSelection::Action(0));
    model.host.shortcuts.insert(words.sharpen.clone(), ShortcutChoice { key: 2, shift: false, command: false });
    model
}

/// One action being recorded into, with two recorded steps and two pending ones.
fn recording_model(words: &ActionsStoryLabels) -> Model {
    let list = list_of(
        vec![action(&words.sharpen, None, &["levels", "unsharp"]), action(&words.retouch, Some(&words.portrait), &["hue", "curves"])],
        &[&words.portrait],
    );
    let mut host = host(words);
    host.recording = Some(1);
    host.pending = vec![step("brightness"), step("blur")];
    let mut state = ActionsPanelState::default();
    state.expanded.insert(words.retouch.clone());
    state.selected = Some(ActionSelection::Action(1));
    new_model(list, state, host, words)
}

/// One action per colour in Button Mode, named after its colour.
fn button_model(words: &ActionsStoryLabels, labels: &LabelColorLabels) -> Model {
    let names = ActionColor::ALL.map(|color| (labels.name(color.label_color()), color));
    let mut list = ActionList::new();
    for (name, color) in names {
        if let Ok(index) = list.new_action(name, None) {
            let _ = list.set_color(index, color);
        }
    }
    let mut state = ActionsPanelState::default();
    state.settings.button_mode = true;
    new_model(list, state, host(words), words)
}

/// Applies what the user did to the story's own model, as the app would with the matching
/// `ActionList` calls. Play, the dialogs and the file commands have no app behind them.
fn apply(model: &mut Model, events: Vec<ActionsEvent>) {
    let Model { list, state, host, new_action, new_set } = model;
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
                let name = list.unique_name(new_action);
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
                let name =
                    (1..).map(|n| fmt(new_set, &[("n", &n.to_string())])).find(|name| !list.sets().contains(&name.as_str())).unwrap_or_default();
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
            ActionsEvent::Rename { action, text: name } => {
                let old = list.action(action).map(|current| current.name.clone());
                if list.rename(action, &name, |_, _, _| {}).is_ok()
                    && let Some(old) = old
                {
                    let new = name.trim().to_owned();
                    if state.expanded.remove(&old) {
                        state.expanded.insert(new.clone());
                    }
                    if let Some(choice) = host.shortcuts.remove(&old) {
                        host.shortcuts.insert(new, choice);
                    }
                }
            }
            ActionsEvent::Reorder(EditEvent::Commit { value, .. } | EditEvent::Set { value, .. }) => {
                match (value.items.first(), ActionPlace::of(list, &value)) {
                    (Some(&ActionSelection::Action(from)), Some(ActionPlace::Action { index, set })) => {
                        if list.move_action(from, index).is_ok() {
                            let _ = list.assign_set(index, set);
                        }
                    }
                    (Some(&ActionSelection::Step(from, at)), Some(ActionPlace::Step { action, index })) => {
                        let _ = list.move_step(from, at, action, index);
                    }
                    _ => {}
                }
            }
            ActionsEvent::Toggle(EditEvent::Update { value, .. } | EditEvent::Commit { value, .. } | EditEvent::Set { value, .. }) => {
                let _ = match (value.row, value.column) {
                    (ActionSelection::Step(action, step), 0) => list.set_step(action, step, value.on),
                    (ActionSelection::Action(action), 0) => list.set_all(action, value.on),
                    (ActionSelection::Step(action, step), 1) => list.set_dialog(action, step, value.on),
                    _ => Ok(()),
                };
            }
            ActionsEvent::SetColor { action, color } => {
                let _ = list.set_color(action, color);
            }
            ActionsEvent::AssignShortcut { action, choice } => {
                if let Some(name) = list.action(action).map(|current| current.name.clone()) {
                    match choice {
                        Some(choice) => host.shortcuts.insert(name, choice),
                        None => host.shortcuts.remove(&name),
                    };
                }
            }
            _ => {}
        }
    }
}

/// Shows the panel over the model kept under `name`, seeded by `seed` on the first pass, and
/// applies its events.
fn show_model(ui: &mut Ui, name: &str, seed: impl FnOnce(&ActionsStoryLabels, &LabelColorLabels) -> Model) {
    let (words, colors) = (ActionsStoryLabels::of(ui), LabelColorLabels::shared(ui));
    let kept = shared(ui, name, || seed(&words, &colors));
    let mut guard = lock(&kept);
    let model = &mut *guard;
    let size = vec2(ui.available_width(), PANEL_HEIGHT);
    let events = ui
        .allocate_ui(size, |ui| {
            ui.set_min_size(size);
            ActionsPanel::new(("actions-story-panel", name), &mut model.state, &model.list).host(&model.host).show(ui).events
        })
        .inner;
    apply(model, events);
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
    show_model(ui, "empty", |words, _| new_model(ActionList::new(), ActionsPanelState::default(), host(words), words));
}

#[cfg(test)]
mod tests {
    use super::*;

    // Renaming moves the action's shortcut whether or not the action was expanded.
    #[test]
    fn renaming_a_collapsed_action_moves_its_shortcut() {
        let words = ActionsStoryLabels::default();
        let mut model = sets_model(&words);
        model.state.expanded.clear();
        let choice = ShortcutChoice { key: 2, shift: false, command: false };
        assert_eq!(model.host.shortcuts.get(&words.sharpen), Some(&choice));
        apply(&mut model, vec![ActionsEvent::Rename { action: 0, text: "Crisp".to_owned() }]);
        assert_eq!(model.host.shortcuts.get("Crisp"), Some(&choice));
        assert!(!model.host.shortcuts.contains_key(&words.sharpen));
    }
}
