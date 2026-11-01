//! The preset library: groups, nested folders and presets with unique names, and the browser rows made from them (`PresetKey`, `GroupPath`, `PresetLibrary`, `PresetChange`, `PresetError`).

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use serde::{Deserialize, Serialize};

use super::PresetRow;

/// A group or folder by its names from the top group down: `["Brushes", "Dry Media"]`. The empty
/// path is the library's root, which holds the ungrouped presets and the top groups. Names are
/// unique among siblings, so a path names one group.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GroupPath(Vec<String>);

impl GroupPath {
    /// The path of `names`, from the top group down.
    pub fn new<S: Into<String>>(names: impl IntoIterator<Item = S>) -> Self {
        GroupPath(names.into_iter().map(Into::into).collect())
    }

    /// The root: the ungrouped presets and the top groups.
    pub fn root() -> Self {
        GroupPath(Vec::new())
    }

    /// The names, from the top group down.
    pub fn names(&self) -> &[String] {
        &self.0
    }

    /// The group's own name; `""` for the root.
    pub fn name(&self) -> &str {
        self.0.last().map_or("", String::as_str)
    }

    /// How deep the group is: 0 for the root, 1 for a top group, 2 for a folder in it.
    pub fn depth(&self) -> usize {
        self.0.len()
    }

    /// The group that holds this one; `None` for the root.
    pub fn parent(&self) -> Option<GroupPath> {
        let (_, parent) = self.0.split_last()?;
        Some(GroupPath(parent.to_vec()))
    }

    /// The path of the child group `name` of this one.
    pub fn child(&self, name: impl Into<String>) -> GroupPath {
        let mut names = self.0.clone();
        names.push(name.into());
        GroupPath(names)
    }
}

/// The key of a browser row: a group (or folder) or a preset, so a group's key and a preset's
/// key never collide (PhotoCraft's `"group:{name}"` strings, the private key of
/// `SwatchesPanel`). `I` is the preset's id: [`PresetId`] in a [`PresetLibrary`]; PhotoCraft's
/// engine library uses the preset's name (`PresetKey<String>`).
///
/// ```
/// use craft_ui::presets::model::PresetKey;
/// let group: PresetKey<String> = PresetKey::group(["A"]);
/// assert_ne!(group, PresetKey::item("A".to_owned()));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PresetKey<I> {
    /// A group or folder.
    Group(GroupPath),
    /// A preset.
    Item(I),
}

impl<I> PresetKey<I> {
    /// The key of the group `names` (from the top group down).
    pub fn group<S: Into<String>>(names: impl IntoIterator<Item = S>) -> Self {
        PresetKey::Group(GroupPath::new(names))
    }

    /// The key of preset `id`.
    pub fn item(id: I) -> Self {
        PresetKey::Item(id)
    }
}

/// One preset of a [`PresetLibrary`], unique by id; ids are never reused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PresetId(u64);

impl PresetId {
    /// The id `value`.
    pub const fn new(value: u64) -> Self {
        PresetId(value)
    }

    /// The number inside.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// A preset: its id, its name (unique across the library, ignoring case) and the app's value.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PresetItem<T> {
    pub id: PresetId,
    pub name: String,
    pub value: T,
}

/// A group or folder: its name (unique among its siblings, ignoring case), its presets and its
/// folders, each in display order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(bound(serialize = "T: Serialize", deserialize = "T: Deserialize<'de>"))]
pub struct PresetGroup<T> {
    pub name: String,
    pub items: Vec<PresetItem<T>>,
    pub groups: Vec<PresetGroup<T>>,
}

/// What a library change did, for the app to save the library or follow its selection.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PresetChange {
    /// A preset was added.
    Added { id: PresetId, group: GroupPath },
    /// A group was added.
    GroupAdded(GroupPath),
    /// A preset was renamed.
    Renamed { id: PresetId, name: String },
    /// A group was renamed; `to` is its new path (its folders' paths change with it).
    GroupRenamed { from: GroupPath, to: GroupPath },
    /// A preset was deleted.
    Deleted(PresetId),
    /// A group was deleted with everything inside it; `items` are the presets that went with it.
    GroupDeleted { path: GroupPath, items: Vec<PresetId> },
    /// A preset moved to another group (Move to, or a drag into a group).
    Moved { id: PresetId, to: GroupPath },
    /// A group moved to another parent; `to` is its new path.
    GroupMoved { from: GroupPath, to: GroupPath },
    /// Presets or groups changed order inside one group.
    Reordered(GroupPath),
}

/// Why a library change was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PresetError {
    /// The name is empty or all blank.
    EmptyName,
    /// Another preset (or a sibling group) has the name, ignoring case.
    NameTaken,
    /// The group path names no group.
    NoGroup,
    /// No preset has the id.
    NoPreset,
    /// The group would sit deeper than the library's nesting.
    TooDeep,
}

impl std::fmt::Display for PresetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            PresetError::EmptyName => "the name is empty",
            PresetError::NameTaken => "the name is taken",
            PresetError::NoGroup => "no group has that path",
            PresetError::NoPreset => "no preset has that id",
            PresetError::TooDeep => "the group would be nested too deep",
        })
    }
}

impl std::error::Error for PresetError {}

/// A library of presets the app owns and keeps in a settings section (it is serde): ungrouped
/// presets and groups at the root, folders inside groups up to [`nesting`](Self::nesting) levels
/// deep, presets with names unique across the library ignoring case (as `Profiles` and New
/// Document). An optional helper: the browser's contract is [`PresetRow`] and
/// [`PresetEvent`](super::super::PresetEvent), and an app with its own engine library
/// (PhotoCraft's journaled `<kind>.presets.*` commands) uses only [`PresetKey`].
///
/// ```ignore
/// let rows = library.rows();
/// let out = PresetBrowser::new("gradients", "Gradients", &mut state, &rows)
///     .reorder(PresetReorder::items_and_groups().nest(library.nesting()))
///     .show(ui, paint);
/// for change in library.apply_all(&out.events) { save_library(change); }
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(bound(serialize = "T: Serialize", deserialize = "T: Deserialize<'de>"))]
pub struct PresetLibrary<T> {
    /// The root: its `name` is empty, its items are the ungrouped presets.
    pub(super) root: PresetGroup<T>,
    /// How deep folders go below a top group: 0 for flat groups (PhotoCraft's Gradients,
    /// Patterns, Styles, Shapes, Swatches), 32 for PhotoCraft's Brushes.
    pub(super) nesting: usize,
    /// The number of the next id; never goes down.
    pub(super) next_id: u64,
}

impl<T> Default for PresetLibrary<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> PresetLibrary<T> {
    /// An empty library with flat groups (nesting 0).
    pub fn new() -> Self {
        Self { root: PresetGroup { name: String::new(), items: Vec::new(), groups: Vec::new() }, nesting: 0, next_id: 1 }
    }

    /// Sets how deep folders may go below a top group: the number `PresetReorder::nest` takes
    /// (PhotoCraft's Brushes `MAX_FOLDER_DEPTH` 32).
    #[must_use]
    pub fn with_nesting(mut self, nesting: usize) -> Self {
        self.nesting = nesting;
        self
    }

    /// How deep folders may go below a top group.
    pub fn nesting(&self) -> usize {
        self.nesting
    }

    /// Adds a preset named `name` at the end of `group` (the root for ungrouped presets) and
    /// returns its id. The name is trimmed; a taken name is numbered "Red 2", "Red 3", …
    /// (PhotoCraft's `unique_name`, here ignoring case). An empty or blank name is
    /// [`PresetError::EmptyName`]; a path that names no group is [`PresetError::NoGroup`].
    pub fn add(&mut self, group: &GroupPath, name: &str, value: T) -> Result<PresetId, PresetError> {
        // pc: crates/engine/src/presets.rs:293 (`unique_name`, "base 2", "base 3")
        todo!()
    }

    /// Adds a group under `parent` (the root for a top group) named `base`, numbered "Group 2",
    /// "Group 3", … when a sibling has the name, and returns its path. Refuses a group deeper
    /// than the nesting allows ([`PresetError::TooDeep`]: a top group is always allowed; a folder
    /// at depth `d` below its top group needs `d <= nesting`).
    pub fn new_group(&mut self, parent: &GroupPath, base: &str) -> Result<GroupPath, PresetError> {
        // pc: crates/engine/src/presets.rs:293 (`unique_name`)
        todo!()
    }

    /// Renames preset `id`: trimmed; [`PresetError::EmptyName`] for a blank name,
    /// [`PresetError::NameTaken`] when another preset has it ignoring case (a change of case of
    /// its own name is allowed).
    pub fn rename(&mut self, id: PresetId, name: &str) -> Result<PresetChange, PresetError> {
        // settings: crates/settings/src/profiles.rs:87-104 (names compare without case)
        todo!()
    }

    /// Renames group `path` among its siblings, with the same rules as [`rename`](Self::rename).
    pub fn rename_group(&mut self, path: &GroupPath, name: &str) -> Result<PresetChange, PresetError> {
        // new: group names are unique among siblings.
        todo!()
    }

    /// Deletes preset `id`.
    pub fn delete(&mut self, id: PresetId) -> Result<PresetChange, PresetError> {
        // new: the browser's Delete.
        todo!()
    }

    /// Deletes group `path` and everything inside it. The root cannot be deleted
    /// ([`PresetError::NoGroup`]).
    pub fn delete_group(&mut self, path: &GroupPath) -> Result<PresetChange, PresetError> {
        // new: the browser's Delete Group.
        todo!()
    }

    /// Moves preset `id` to the end of group `to` (Move to).
    pub fn move_to(&mut self, id: PresetId, to: &GroupPath) -> Result<PresetChange, PresetError> {
        // new: the browser's Move to.
        todo!()
    }

    /// Preset `id`.
    pub fn get(&self, id: PresetId) -> Option<&PresetItem<T>> {
        // new: a walk of the groups.
        todo!()
    }

    /// Preset `id`, to change its value.
    pub fn get_mut(&mut self, id: PresetId) -> Option<&mut PresetItem<T>> {
        // new: a walk of the groups.
        todo!()
    }

    /// The group at `path` (the root for the empty path).
    pub fn group(&self, path: &GroupPath) -> Option<&PresetGroup<T>> {
        // new: a walk by name.
        todo!()
    }

    /// The group that holds preset `id`.
    pub fn group_of(&self, id: PresetId) -> Option<GroupPath> {
        // new: a walk of the groups.
        todo!()
    }

    /// Why `name` cannot be a new preset's name, if it cannot: blank, or taken ignoring case.
    /// For a named row's `check_name`.
    pub fn name_error(&self, name: &str) -> Option<PresetError> {
        // new: the rules of `add` without the numbering.
        todo!()
    }

    /// The number of presets.
    pub fn total(&self) -> usize {
        // new: a count over the groups.
        todo!()
    }

    /// Every preset, in display order (root presets, then each group's presets and folders depth
    /// first).
    pub fn items(&self) -> Vec<&PresetItem<T>> {
        // new: the order of `rows`.
        todo!()
    }

    /// The browser's display rows: the root's presets at depth 0, then each group as a header
    /// row (depth = its depth - 1, `count` = the presets inside it and its folders) followed by
    /// its presets (depth = the group's depth) and its folders, depth first.
    pub fn rows(&self) -> Vec<PresetRow<PresetKey<PresetId>>> {
        // pc-port: crates/ui-egui/src/panels/preset_panels.rs:331-379 (`GroupView`, `ItemView`, `"group:"` keys this replaces)
        todo!()
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

    // `rows()` lists root presets, groups, folders and presets with the right depths, counts and order.
    #[test]
    #[ignore = "skeleton"]
    fn library_rows_list_groups_folders_and_items_with_counts() {
        todo!()
    }

    // Adding "Red" twice gives "Red 2"; renaming another preset to "red" is `NameTaken`; a blank name is `EmptyName`.
    #[test]
    #[ignore = "skeleton"]
    fn library_names_are_unique_ignoring_case_and_numbered_on_add() {
        todo!()
    }

    // A nested library (three levels, values) survives a JSON round trip unchanged, ids and next id included.
    #[test]
    #[ignore = "skeleton"]
    fn library_round_trips_through_serde() {
        todo!()
    }

    // `PresetKey::group(["A"])` is not `PresetKey::item("A")`, and both work as hash keys side by side.
    #[test]
    #[ignore = "skeleton"]
    fn preset_key_distinguishes_a_group_from_an_item_of_the_same_name() {
        todo!()
    }
}
