//! The Actions footer (`PanelFooter`): Stop, Record, Play, New Set, New Action, Delete and the
//! Button Mode toggle.

use craft_ui::composites::PanelFooter;
use craft_ui::controls::{ControlSize, IconButton};
use craft_ui::icons::Icon;
use craft_ui::theme::{Tokens, TokensOf as _};
use egui::{Response, Ui, vec2};

use super::labels::ActionsLabels;
use super::panel::{ActionSelection, ActionsEvent, ActionsHost, ActionsState, delete_event};
use super::style::ActionsPanelStyle;
use crate::actions::ActionList;

/// The footer under the list: a [`PanelFooter`](craft_ui::composites::PanelFooter) of icon
/// buttons (lead calls L22, L32), in this order, each a Tab stop with its name and tooltip:
///
/// | Button | Icon | Reports | Enabled |
/// |---|---|---|---|
/// | Stop playing/recording | `Square` | `StopRecording` | while recording or playing |
/// | Begin recording | `Circle`, in `status.bad.solid` while recording | `BeginRecording { into: selected action }` | not recording |
/// | Play selection | `Play` | `Play { action, from }`: the selected action from 0, a selected step's action from that step, else the first action (index 0) | some action, not recording |
/// | Create new set | `NewFolder` | `NewSet` | not recording |
/// | Create new action | `Plus` | `BeginRecording { into: None }` | not recording |
/// | Delete | `Trash` | `Delete(selection)` | a selection, and while recording only a step |
/// | Button Mode | `ToggleRight`, pressed state | toggles [`ActionsSettings::button_mode`](super::ActionsSettings::button_mode) in `state` | not recording |
///
/// PhotoCraft's "Button Mode" small text button becomes a pressed icon button in the one footer
/// look (lead call L21: the panel menu's check row has the same state).
pub(super) fn footer(
    ui: &mut Ui,
    state: &mut ActionsState,
    list: &ActionList,
    host: Option<&dyn ActionsHost>,
    labels: &ActionsLabels,
    style: &ActionsPanelStyle,
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
        let mode = button(ui, style, Icon::ToggleRight, &labels.button_mode, idle, Some(state.settings.button_mode), false);
        if mode.clicked() {
            state.settings.button_mode = !state.settings.button_mode;
        }
        if button(ui, style, Icon::Trash, &labels.delete, delete.is_some(), None, false).clicked() {
            events.extend(delete);
        }
        if button(ui, style, Icon::Plus, &labels.new_action, hosted && idle, None, false).clicked() {
            events.push(ActionsEvent::BeginRecording { into: None });
        }
        if button(ui, style, Icon::NewFolder, &labels.new_set, idle, None, false).clicked() {
            events.push(ActionsEvent::NewSet);
        }
        if button(ui, style, Icon::Play, &labels.play, hosted && idle && target.is_some(), None, false).clicked()
            && let Some((action, from)) = target
        {
            events.push(ActionsEvent::Play { action, from });
        }
        if button(ui, style, Icon::Circle, &labels.record, hosted && idle, None, recording.is_some()).clicked() {
            events.push(ActionsEvent::BeginRecording { into: record_into });
        }
        if button(ui, style, Icon::Square, &labels.stop, hosted && !idle, None, false).clicked() {
            events.push(ActionsEvent::StopRecording);
        }
    });
    events
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

/// A footer icon button. `pressed` makes it a toggle button showing that state; `alert` draws its
/// icon in the recording colour. `PanelFooter::button` has neither, so this is its
/// `IconButton` at the footer's square size.
fn button(ui: &mut Ui, style: &ActionsPanelStyle, icon: Icon, label: &str, enabled: bool, pressed: Option<bool>, alert: bool) -> Response {
    let tokens = Tokens::of(ui);
    let side = tokens.component.panel_footer.button;
    let (_, square) = ui.allocate_space(vec2(side, side));
    let mut look = IconButton::style(&tokens, ControlSize::Md);
    if alert {
        look.text.rest = style.recording;
        look.text.hover = style.recording;
        look.text.pressed = style.recording;
    }
    let mut button = IconButton::new(icon, label).size(ControlSize::Md).with_style(look).enabled(enabled).id_salt(("actions-footer", label));
    if let Some(on) = pressed {
        button = button.selected(on);
    }
    button.show_at(ui, square)
}
