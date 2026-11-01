//! The Actions footer (`PanelFooter`): Stop, Record, Play, New Set, New Action, Delete and the
//! Button Mode toggle.
#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::Ui;

use super::labels::ActionsLabels;
use super::panel::{ActionsEvent, ActionsHost, ActionsState};
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
    todo!()
}
