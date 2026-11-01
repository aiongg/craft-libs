//! Named snapshots that outlive history states, the history brush source, and the New Snapshot
//! and History Options forms (`SnapshotList`, `HistorySourceChoice`, `new_snapshot_form`,
//! `history_options_form`).

mod forms;
mod list;
mod source;
#[cfg(test)]
mod tests;
