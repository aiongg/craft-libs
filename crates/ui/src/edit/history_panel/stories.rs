//! Stories of the History panel and the undo list (task ST3): stubs, not registered.
//! Registering them (task RG2) adds `stories()` to the edit stories and `HistoryPanel`,
//! `UndoList` to `story::DECLARED`; this file declares no `COMPONENTS` before then (the story
//! test counts those lines).
//!
//! Every story uses fakes: a `StepList` and a `SnapshotList` built in the story,
//! `FakeThumbSource<SnapshotId>` for the snapshot thumbnails; no document and no platform. Every
//! string goes through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `composites/history-panel/states` | 12 states with tool icons, the current state in the middle, later states dimmed, footer |
//! | `composites/history-panel/snapshots` | three snapshots above the states, one selected, the source on a snapshot, a rename in progress |
//! | `composites/history-panel/source` | the source on a state, on the default (oldest), and none (state gone) |
//! | `composites/history-panel/non-linear` | non-linear history with later states deletable by a drop on the trash |
//! | `composites/history-panel/branches` | EffectCraft-style tree rows with depth and off-line rows dimmed (a `StepRows` fake) |
//! | `composites/history-panel/newest-first` | LightCraft's order, no footer features |
//! | `composites/history-panel/short-dock` | 6 snapshots and 20 states in a 160 pt dock: the rows scroll above the footer |
//! | `composites/history-panel/menu` | the panel menu in each enabled combination |
//! | `composites/undo-list/popup` | WordCraft's list with 30 steps, three marked, footer "Undo 3 Actions", and the Cancel row |
//! | `composites/history-forms/new-snapshot` | the New Snapshot form with Name and From |
//! | `composites/history-forms/options` | the History Options form with one extra row |

#![expect(dead_code, clippy::todo, reason = "skeleton: removed when the bodies land")]

use crate::story::Story;

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    // new: see the module's table.
    todo!()
}
