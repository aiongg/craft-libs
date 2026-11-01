//! The Actions panel (F13.06): sets, actions and steps as a tree, the include and dialog cells,
//! rename, drag to move, Button Mode, the footer and the menus. It runs nothing; it reports
//! events. Builds on every target.
//!
//! ```ignore
//! let out = ActionsPanel::new("actions", &mut ui_state.actions, &app.actions).host(&app_host).show(ui);
//! for event in out.events {
//!     match event {
//!         ActionsEvent::Play { action, from } => app.play(action, from),
//!         ActionsEvent::BeginRecording { into } => app.record(into),
//!         ActionsEvent::ToggleStep { action, step } => { app.actions.toggle_step(action, step)?; }
//!         _ => app.run_action_event(event),
//!     }
//! }
//! ```
//!
//! | Part | Names |
//! |---|---|
//! | The panel | [`ActionsPanel`], [`ActionsPanelOutput`], [`ActionsEvent`], [`ActionPlace`], [`ActionSelection`], [`ShortcutChoice`] |
//! | The app's side | [`ActionsHost`], [`ActionsMenuAction`] (from `run_command`) |
//! | State | [`ActionsPanelState`], [`ActionsSettings`] (persisted) |
//! | Look and words | [`ActionsPanelStyle`], [`ActionsLabels`], [`COUNTS`] |

mod footer;
mod labels;
mod menu;
mod panel;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod panel_tests;
mod rows;
#[cfg(test)]
mod settings_tests;
pub(crate) mod stories;
mod style;

pub use labels::ActionsLabels;
pub use menu::ActionsMenuAction;
pub use panel::{
    ActionPlace, ActionSelection, ActionsEvent, ActionsHost, ActionsPanel, ActionsPanelOutput, ActionsPanelState, ActionsSettings, ShortcutChoice,
};
pub use style::ActionsPanelStyle;

/// The Actions panel's count words ("{n} step", "{n} action"), for the words registry.
pub const COUNTS: &[craft_ui::localized::words::Counted] = &[labels::STEPS, labels::ACTIONS];
