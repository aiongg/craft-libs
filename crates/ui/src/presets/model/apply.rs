//! `PresetLibrary::apply`: the browser's Rename, Delete, MoveTo and Reorder events carried out on the library.

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use super::library::{PresetChange, PresetError, PresetId, PresetKey, PresetLibrary};
use crate::behavior::Move;
use crate::presets::PresetEvent;

impl<T> PresetLibrary<T> {
    /// Carries out one browser event and returns what changed (empty when nothing did):
    ///
    /// - `Rename` of a preset or group ([`rename`](Self::rename), [`rename_group`](Self::rename_group));
    ///   a refused name changes nothing.
    /// - `Delete` of a preset or group.
    /// - `MoveTo { key, group }` with a preset key and a group key.
    /// - `Reorder` when it is `Commit` with `changed` or `Set` ([`reorder`](Self::reorder)); a
    ///   `Reorder` with `copy` is ignored, and so are `Begin`, `Update` and `Cancel`.
    /// - Every other event (`Select`, `Activate`, `Open`, `DropAt`, `Extra`) returns nothing.
    ///
    /// The browser's `accepts` and `nest` already refused what the library refuses; the library
    /// checks again and ignores a refused move.
    pub fn apply(&mut self, event: &PresetEvent<PresetKey<PresetId>>) -> Vec<PresetChange> {
        // pc-port: crates/ui-egui/src/panels/preset_panels.rs:557-594 (`generic`: Rename, Delete, Move mapped to engine commands)
        todo!()
    }

    /// [`apply`](Self::apply) for each event in order, the changes concatenated.
    pub fn apply_all(&mut self, events: &[PresetEvent<PresetKey<PresetId>>]) -> Vec<PresetChange> {
        // new: a loop over `apply`.
        todo!()
    }

    /// Moves `mv.items` before, after or into `mv.target` (`DropPosition`), keeping their display
    /// order: presets into a group or next to a preset of another group change group
    /// (`Moved`), groups next to or into another group change parent (`GroupMoved`), and the
    /// order inside each group touched changes (`Reordered`). A group moved into itself or one of
    /// its folders, or deeper than the nesting, is refused ([`PresetError::TooDeep`]), and so is
    /// a group whose name a new sibling already has ([`PresetError::NameTaken`]); nothing changes
    /// then.
    pub fn reorder(&mut self, mv: &Move<PresetKey<PresetId>>) -> Result<Vec<PresetChange>, PresetError> {
        // new: the story `Library::apply_move` (`presets/stories.rs:181-230`) on a nested model.
        todo!()
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

    // Each browser event (Rename, Delete, MoveTo, Reorder Commit) changes the library and returns its `PresetChange`; Begin and copy moves return nothing.
    #[test]
    #[ignore = "skeleton"]
    fn library_applies_rename_delete_move_and_reorder_events() {
        todo!()
    }

    // A group dropped into a folder past `with_nesting` is refused and the library is unchanged.
    #[test]
    #[ignore = "skeleton"]
    fn library_refuses_nesting_past_its_depth() {
        todo!()
    }
}
