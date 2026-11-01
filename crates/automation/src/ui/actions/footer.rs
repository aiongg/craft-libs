//! The Actions footer (`PanelFooter`): Stop, Record, Play, New Set, New Action, Delete and the
//! Button Mode toggle.

use craft_ui::composites::{PanelFooter, Tone};
use craft_ui::icons::Icon;
use egui::{Response, Ui};

use super::labels::ActionsLabels;
use super::panel::{ActionSelection, ActionsEvent, ActionsHost, ActionsPanelState, delete_event};
use crate::actions::ActionList;

/// The footer under the list: a [`PanelFooter`](craft_ui::composites::PanelFooter) of icon
/// buttons (lead calls L22, L32), in this order, each a Tab stop with its name and tooltip:
///
/// | Button | Icon | Reports | Enabled |
/// |---|---|---|---|
/// | Stop playing/recording | `Square` | `StopRecording` | while recording or playing |
/// | Begin recording | `Circle`, a pressed button in `Tone::Bad` while recording | `BeginRecording { into: selected action }` | not recording; while recording it stays live and ignores the click |
/// | Play selection | `Play` | `Play { action, from }`: the selected action from 0, a selected step's action from that step, else the first action (index 0) | some action, not recording |
/// | Create new set | `NewFolder` | `NewSet` | not recording |
/// | Create new action | `Plus` | `BeginRecording { into: None }` | not recording |
/// | Delete | `Trash` | `Delete(selection)` | a selection, and while recording only a step |
/// | Button Mode | `ToggleRight`, pressed button | toggles [`ActionsSettings::button_mode`](super::ActionsSettings::button_mode) in `state` | not recording |
///
/// PhotoCraft's "Button Mode" small text button becomes a pressed icon button in the one footer
/// look (lead call L21: the panel menu's check row has the same state).
pub(super) fn footer(
    ui: &mut Ui,
    state: &mut ActionsPanelState,
    list: &ActionList,
    host: Option<&dyn ActionsHost>,
    labels: &ActionsLabels,
) -> Vec<ActionsEvent> {
    // pc: crates/ui-egui/src/panels/actions.rs:1104-1139 (bottom row), :63 `playback_selection`;
    // vectorcraft: crates/ui-egui/src/panels/actions.rs:174 (Create New Set).
    let recording = host.and_then(|host| host.recording());
    let playing = host.is_some_and(|host| host.playing());
    let hosted = host.is_some();
    let idle = recording.is_none() && !playing;
    let target = play_target(state.selected.as_ref(), list);
    let delete = state.selected.as_ref().filter(|_| !playing).and_then(|selected| delete_event(selected, recording.is_some()));
    let record_into = selected_action(state.selected.as_ref(), list);
    let mut events = Vec::new();
    // The footer fills from the reading end, so the buttons are added last to first: they read
    // Stop, Record, Play, New Set, New Action, Delete, Button Mode.
    PanelFooter::show(ui, |ui| {
        let mode = ui.add_enabled_ui(idle, |ui| {
            PanelFooter::pressed_button(ui, Icon::ToggleRight, &labels.button_mode, state.settings.button_mode, Tone::Neutral)
        });
        if mode.inner.clicked() {
            state.settings.button_mode = !state.settings.button_mode;
        }
        if plain(ui, Icon::Trash, &labels.delete, delete.is_some()).clicked() {
            events.extend(delete);
        }
        if plain(ui, Icon::Plus, &labels.new_action, hosted && idle).clicked() {
            events.push(ActionsEvent::BeginRecording { into: None });
        }
        if plain(ui, Icon::NewFolder, &labels.new_set, idle).clicked() {
            events.push(ActionsEvent::NewSet);
        }
        if plain(ui, Icon::Play, &labels.play, hosted && idle && target.is_some()).clicked()
            && let Some((action, from)) = target
        {
            events.push(ActionsEvent::Play { action, from });
        }
        // Record stays live while recording: it shows the alert state and ignores the click.
        let tone = if recording.is_some() { Tone::Bad } else { Tone::Neutral };
        let record = ui
            .add_enabled_ui(hosted && !playing, |ui| PanelFooter::pressed_button(ui, Icon::Circle, &labels.record, recording.is_some(), tone))
            .inner;
        if record.clicked() && idle {
            events.push(ActionsEvent::BeginRecording { into: record_into });
        }
        if plain(ui, Icon::Square, &labels.stop, hosted && !idle).clicked() {
            events.push(ActionsEvent::StopRecording);
        }
    });
    events
}

/// A footer button that is enabled or not.
fn plain(ui: &mut Ui, icon: Icon, label: &str, enabled: bool) -> Response {
    ui.add_enabled_ui(enabled, |ui| PanelFooter::button(ui, icon, label)).inner
}

/// What Play plays: the selected action from its first step, a selected step's action from that
/// step, else the first action. `None` without actions.
pub(super) fn play_target(selected: Option<&ActionSelection>, list: &ActionList) -> Option<(usize, usize)> {
    let len = list.actions().len();
    match selected {
        Some(&ActionSelection::Action(action)) if action < len => Some((action, 0)),
        Some(&ActionSelection::Step(action, step)) if action < len => Some((action, step)),
        _ => (len > 0).then_some((0, 0)),
    }
}

/// The action the selection is, or a selected step belongs to.
pub(super) fn selected_action(selected: Option<&ActionSelection>, list: &ActionList) -> Option<usize> {
    match selected {
        Some(&ActionSelection::Action(action) | &ActionSelection::Step(action, _)) if action < list.actions().len() => Some(action),
        _ => None,
    }
}
