//! Saved versions and branches of a document (N5.06).

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct VersionId(pub u64);

/// A saved version: named by the user, or an autosave.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Version {
    pub id: VersionId,
    /// Empty for an autosave.
    pub title: String,
    pub description: String,
    /// Unix seconds.
    pub created: i64,
    pub author: String,
    pub autosave: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct BranchId(pub u64);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum BranchState {
    #[default]
    Active,
    Merged,
    Archived,
}

/// A branch of the file (Figma's branching).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Branch {
    pub id: BranchId,
    pub name: String,
    pub state: BranchState,
    pub created: i64,
    pub author: String,
}

/// A row of the history list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HistoryRow {
    /// A named version, by index into the versions.
    Version(usize),
    /// A run of consecutive autosaves, collapsed by default ("3 autosave versions").
    Autosaves(Vec<usize>),
}

/// The history list of `versions` (any order): newest first, named versions as rows of their
/// own, each run of consecutive autosaves between them one group.
// figcraft: crates/ui-egui/src/panels/versions.rs:111 `panel` (rows), `:557` `autosave_group`.
pub fn history_rows(versions: &[Version]) -> Vec<HistoryRow> {
    let mut order: Vec<usize> = (0..versions.len()).collect();
    order.sort_by(|&a, &b| (versions[b].created, versions[b].id).cmp(&(versions[a].created, versions[a].id)));
    let mut rows = Vec::new();
    let mut at = 0;
    while at < order.len() {
        let run = order[at..].iter().take_while(|&&i| versions[i].autosave).count();
        // A lone autosave stays a row of its own; a group folds two or more (FigCraft's `run >= 2`).
        if run >= 2 {
            rows.push(HistoryRow::Autosaves(order[at..at + run].to_vec()));
            at += run;
        } else {
            rows.push(HistoryRow::Version(order[at]));
            at += 1;
        }
    }
    rows
}
