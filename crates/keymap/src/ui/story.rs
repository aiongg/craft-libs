//! The story of the Keyboard Shortcuts dialog.

use std::sync::{Arc, Mutex, PoisonError};

use egui::Ui;

use super::{DialogState, ShortcutsDialog};
use crate::Platform;
use crate::keymap::{Bindable, Binding, Keymap};
use crate::scope::{Scope, Scopes};
use craft_ui::story::{Level, NoArgs, Story};
use craft_ui::typed_story;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Keyboard Shortcuts"];

pub(super) fn stories() -> Vec<Story> {
    vec![typed_story!("patterns/keyboard-shortcuts/dialog", Level::Patterns, "Keyboard Shortcuts", "Dialog", NoArgs {}, show)]
}

#[allow(clippy::panic, reason = "the story's own fixed chords are valid")]
fn binding(text: &str) -> Binding {
    Binding::parse(text).unwrap_or_else(|e| panic!("{text}: {e}"))
}

/// A small app: file commands, two commands that both default to `M` (the conflict), and a command
/// of the Timeline panel scope.
fn keymap() -> Keymap {
    let path = |parts: &[&str]| parts.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
    let timeline = Scope::new("Timeline").unwrap_or_else(Scope::app);
    let catalog = vec![
        Bindable::new("file.save", "Save", Scope::app()).with_path(path(&["File", "Save"])).with_default(binding("Cmd+S")),
        Bindable::new("file.export", "Export…", Scope::app()).with_path(path(&["File", "Export…"])).with_default(binding("Cmd+E")),
        Bindable::new("tool.marquee", "Marquee Tool", Scope::app()).with_default(binding("M")),
        Bindable::new("tool.move", "Move Tool", Scope::app()).with_default(binding("M")),
        Bindable::new("timeline.loop", "Loop", timeline.clone()).with_default(binding("L")),
        Bindable::new("timeline.play", "Play", timeline).with_default(binding("Space")),
    ];
    Keymap::new(catalog, Scopes::new().with_panel("Timeline"), Platform::Linux)
}

/// The dialog inline, over a keymap and dialog state that live as long as the story's `Ui` id.
fn show(ui: &mut Ui, _: &mut NoArgs) {
    let key = ui.scope_id().with("keyboard-shortcuts-story");
    let state: Arc<Mutex<(Keymap, DialogState)>> =
        ui.data_mut(|d| d.get_temp_mut_or_insert_with(key, || Arc::new(Mutex::new((keymap(), DialogState::new())))).clone());
    let mut guard = state.lock().unwrap_or_else(PoisonError::into_inner);
    let (keymap, dialog_state) = &mut *guard;
    ui.set_min_height(360.0);
    ShortcutsDialog::new(keymap).ui(ui, dialog_state);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn story_keymap_has_one_conflict_in_two_scopes() {
        let km = keymap();
        assert_eq!(km.conflicts().len(), 1);
        let scopes: std::collections::HashSet<_> = km.catalog().iter().map(|b| b.scope.clone()).collect();
        assert_eq!(scopes.len(), 2);
    }

    #[test]
    fn story_shows_with_its_defaults() {
        let ctx = egui::Context::default();
        let story = &stories()[0];
        let mut args = story.defaults();
        ctx.run_ui(egui::RawInput::default(), |ui| (story.show)(ui, &mut args)).drop_without_applying_deltas();
    }
}
