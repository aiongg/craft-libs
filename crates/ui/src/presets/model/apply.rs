//! `PresetLibrary::apply`: the browser's Rename, Delete, MoveTo and Reorder events carried out on the library.

use super::library::{GroupPath, PresetChange, PresetError, PresetGroup, PresetId, PresetKey, PresetLibrary, same_name};
use crate::behavior::{DropPosition, Move};
use crate::edit::EditEvent;
use crate::presets::PresetEvent;

/// Rows of `group`'s deepest chain, counting `group` itself.
fn height<T>(group: &PresetGroup<T>) -> usize {
    1 + group.groups.iter().map(height).max().unwrap_or(0)
}

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
        let change = match event {
            PresetEvent::Rename { key: PresetKey::Item(id), text } => self.rename(*id, text),
            PresetEvent::Rename { key: PresetKey::Group(path), text } => self.rename_group(path, text),
            PresetEvent::Delete(PresetKey::Item(id)) => self.delete(*id),
            PresetEvent::Delete(PresetKey::Group(path)) => self.delete_group(path),
            PresetEvent::MoveTo { key: PresetKey::Item(id), group: PresetKey::Group(path) } => self.move_to(*id, path),
            PresetEvent::Reorder(EditEvent::Commit { value, changed: true, .. } | EditEvent::Set { value, .. }) => {
                return self.reorder(value).unwrap_or_default();
            }
            _ => return Vec::new(),
        };
        change.into_iter().collect()
    }

    /// [`apply`](Self::apply) for each event in order, the changes concatenated.
    pub fn apply_all(&mut self, events: &[PresetEvent<PresetKey<PresetId>>]) -> Vec<PresetChange> {
        events.iter().flat_map(|event| self.apply(event)).collect()
    }

    /// Moves `mv.items` before, after or into `mv.target` (`DropPosition`), keeping their display
    /// order: presets into a group or next to a preset of another group change group
    /// (`Moved`), groups next to or into another group change parent (`GroupMoved`), and the
    /// order inside each group touched changes (`Reordered`). A group moved into itself or one of
    /// its folders, or deeper than the nesting, is refused ([`PresetError::TooDeep`]), and so is
    /// a group whose name a new sibling already has ([`PresetError::NameTaken`]); nothing changes
    /// then.
    pub fn reorder(&mut self, mv: &Move<PresetKey<PresetId>>) -> Result<Vec<PresetChange>, PresetError> {
        if mv.copy {
            return Ok(Vec::new());
        }
        let nesting_limit = self.nesting + 1;

        // The destination group and the slots in it, as indices into its lists before any row is
        // taken out. Items sit before groups, so a preset next to a group goes to the end of the
        // items and a group next to a preset to the start of the groups.
        let (dest, item_slot, group_slot) = match &mv.target {
            PresetKey::Item(id) => {
                let dest = self.group_of(*id).ok_or(PresetError::NoPreset)?;
                let at = self.group(&dest).and_then(|g| g.items.iter().position(|i| i.id == *id)).ok_or(PresetError::NoPreset)?;
                match mv.position {
                    DropPosition::Before => (dest, at, 0),
                    DropPosition::After => (dest, at + 1, 0),
                    DropPosition::Into => return Err(PresetError::NoGroup),
                }
            }
            PresetKey::Group(path) => {
                self.group(path).filter(|_| path.depth() > 0).ok_or(PresetError::NoGroup)?;
                if mv.position == DropPosition::Into {
                    (path.clone(), 0, 0)
                } else {
                    let parent = path.parent().ok_or(PresetError::NoGroup)?;
                    let siblings = self.group(&parent).ok_or(PresetError::NoGroup)?;
                    let at = siblings.groups.iter().position(|g| g.name == path.name()).ok_or(PresetError::NoGroup)?;
                    let at = if mv.position == DropPosition::After { at + 1 } else { at };
                    let items = siblings.items.len();
                    (parent, items, at)
                }
            }
        };

        // The rows moved: a row inside another moved group goes with that group.
        let mut group_paths: Vec<GroupPath> = Vec::new();
        for key in &mv.items {
            if let PresetKey::Group(path) = key {
                self.group(path).filter(|_| path.depth() > 0).ok_or(PresetError::NoGroup)?;
                if !group_paths.contains(path) {
                    group_paths.push(path.clone());
                }
            }
        }
        let inside_moved = |path: &GroupPath| group_paths.iter().any(|g| path.names().starts_with(g.names()) && path != g);
        let top_paths: Vec<GroupPath> = group_paths.iter().filter(|g| !inside_moved(g)).cloned().collect();
        let mut preset_ids: Vec<PresetId> = Vec::new();
        for key in &mv.items {
            if let PresetKey::Item(id) = key {
                let from = self.group_of(*id).ok_or(PresetError::NoPreset)?;
                let goes_with_group = top_paths.iter().any(|g| from.names().starts_with(g.names()));
                if !goes_with_group && !preset_ids.contains(id) {
                    preset_ids.push(*id);
                }
            }
        }

        // Refuse before changing anything.
        let dest_group = self.group(&dest).ok_or(PresetError::NoGroup)?;
        for (n, path) in top_paths.iter().enumerate() {
            if dest.names().starts_with(path.names()) {
                return Err(PresetError::TooDeep);
            }
            let source = self.group(path).ok_or(PresetError::NoGroup)?;
            let new_deepest = dest.depth() + height(source);
            let old_deepest = path.depth() + height(source) - 1;
            if new_deepest > nesting_limit && new_deepest > old_deepest {
                return Err(PresetError::TooDeep);
            }
            let moving_out = |sibling: &PresetGroup<T>| top_paths.iter().any(|p| p.parent().as_ref() == Some(&dest) && p.name() == sibling.name);
            let clash = dest_group.groups.iter().any(|g| !moving_out(g) && same_name(&g.name, path.name()));
            let doubled = top_paths[..n].iter().any(|p| same_name(p.name(), path.name()));
            if clash || doubled {
                return Err(PresetError::NameTaken);
            }
        }

        let order = |group: &PresetGroup<T>| {
            (group.items.iter().map(|i| i.id).collect::<Vec<_>>(), group.groups.iter().map(|g| g.name.clone()).collect::<Vec<_>>())
        };
        let before = order(dest_group);
        let item_slot = item_slot - dest_group.items.iter().take(item_slot).filter(|i| preset_ids.contains(&i.id)).count();
        let group_slot = group_slot
            - dest_group
                .groups
                .iter()
                .take(group_slot)
                .filter(|g| top_paths.iter().any(|p| p.parent().as_ref() == Some(&dest) && p.name() == g.name))
                .count();

        let mut changes = Vec::new();
        let mut taken_items = Vec::new();
        for id in &preset_ids {
            let from = self.group_of(*id).ok_or(PresetError::NoPreset)?;
            let source = self.group_mut(&from).ok_or(PresetError::NoGroup)?;
            let at = source.items.iter().position(|i| i.id == *id).ok_or(PresetError::NoPreset)?;
            taken_items.push(source.items.remove(at));
            if from != dest {
                changes.push(PresetChange::Moved { id: *id, to: dest.clone() });
            }
        }
        let mut taken_groups = Vec::new();
        for path in &top_paths {
            let parent = path.parent().ok_or(PresetError::NoGroup)?;
            let source = self.group_mut(&parent).ok_or(PresetError::NoGroup)?;
            let at = source.groups.iter().position(|g| g.name == path.name()).ok_or(PresetError::NoGroup)?;
            taken_groups.push(source.groups.remove(at));
            if parent != dest {
                changes.push(PresetChange::GroupMoved { from: path.clone(), to: dest.child(path.name()) });
            }
        }
        let target = self.group_mut(&dest).ok_or(PresetError::NoGroup)?;
        let at = item_slot.min(target.items.len());
        target.items.splice(at..at, taken_items);
        let at = group_slot.min(target.groups.len());
        target.groups.splice(at..at, taken_groups);
        if order(target) != before {
            changes.push(PresetChange::Reordered(dest));
        }
        Ok(changes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edit::{CancelReason, GestureId};
    use egui::Modifiers;

    type Key = PresetKey<PresetId>;

    /// Root: Loose; A: Red, Green, folder Inner (Blue); B: Cyan; nesting 1.
    fn sample() -> (PresetLibrary<u32>, [PresetId; 5]) {
        let mut lib = PresetLibrary::new().with_nesting(1);
        let root = GroupPath::root();
        let loose = lib.add(&root, "Loose", 0).unwrap();
        let a = lib.new_group(&root, "A").unwrap();
        let red = lib.add(&a, "Red", 1).unwrap();
        let green = lib.add(&a, "Green", 2).unwrap();
        let inner = lib.new_group(&a, "Inner").unwrap();
        let blue = lib.add(&inner, "Blue", 3).unwrap();
        let b = lib.new_group(&root, "B").unwrap();
        let cyan = lib.add(&b, "Cyan", 4).unwrap();
        (lib, [loose, red, green, blue, cyan])
    }

    fn names(lib: &PresetLibrary<u32>) -> Vec<(usize, String)> {
        lib.rows().into_iter().map(|r| (r.depth, r.label)).collect()
    }

    fn mv(items: Vec<Key>, target: Key, position: DropPosition) -> Move<Key> {
        Move { items, target, position, copy: false }
    }

    fn commit(value: Move<Key>) -> PresetEvent<Key> {
        PresetEvent::Reorder(EditEvent::Commit { id: GestureId::next(), value, changed: true, modifiers: Modifiers::NONE })
    }

    // Each browser event (Rename, Delete, MoveTo, Reorder Commit) changes the library and returns its `PresetChange`; Begin and copy moves return nothing.
    #[test]
    fn library_applies_rename_delete_move_and_reorder_events() {
        let (mut lib, [loose, red, green, _blue, cyan]) = sample();
        let b = GroupPath::new(["B"]);

        let renamed = lib.apply(&PresetEvent::Rename { key: PresetKey::Item(red), text: "Crimson".into() });
        assert_eq!(renamed, [PresetChange::Renamed { id: red, name: "Crimson".into() }]);
        assert!(lib.apply(&PresetEvent::Rename { key: PresetKey::Item(green), text: "crimson".into() }).is_empty(), "a taken name changes nothing");
        let renamed = lib.apply(&PresetEvent::Rename { key: PresetKey::group(["B"]), text: "C".into() });
        assert_eq!(renamed, [PresetChange::GroupRenamed { from: b, to: GroupPath::new(["C"]) }]);

        let moved = lib.apply(&PresetEvent::MoveTo { key: PresetKey::Item(loose), group: PresetKey::group(["A", "Inner"]) });
        assert_eq!(moved, [PresetChange::Moved { id: loose, to: GroupPath::new(["A", "Inner"]) }]);

        assert_eq!(lib.apply(&PresetEvent::Delete(PresetKey::Item(cyan))), [PresetChange::Deleted(cyan)]);
        let deleted = lib.apply(&PresetEvent::Delete(PresetKey::group(["A", "Inner"])));
        assert!(matches!(&deleted[..], [PresetChange::GroupDeleted { items, .. }] if items.len() == 2));

        // Reorder: Green before Crimson, as Commit.
        let value = mv(vec![PresetKey::Item(green)], PresetKey::Item(red), DropPosition::Before);
        assert_eq!(lib.apply(&commit(value.clone())), [PresetChange::Reordered(GroupPath::new(["A"]))]);
        assert_eq!(names(&lib), [(0, "A".into()), (1, "Green".into()), (1, "Crimson".into()), (0, "C".into())]);

        // Begin, an unchanged Commit and copy moves change nothing.
        let begin = PresetEvent::Reorder(EditEvent::Begin { id: GestureId::next(), value: Some(value.clone()) });
        let cancel = PresetEvent::Reorder(EditEvent::Cancel { id: GestureId::next(), reason: CancelReason::Dismissed });
        let copy = commit(Move { copy: true, items: vec![PresetKey::Item(red)], target: PresetKey::group(["C"]), position: DropPosition::Into });
        let before = lib.clone();
        assert!(lib.apply_all(&[begin, cancel, copy]).is_empty());
        assert_eq!(lib, before);
    }

    // Presets and groups move before, after and into targets; order is kept and the changes name what moved.
    #[test]
    fn library_reorder_moves_presets_and_groups_keeping_display_order() {
        let (mut lib, [loose, red, green, blue, _cyan]) = sample();
        let a = GroupPath::new(["A"]);
        let b = GroupPath::new(["B"]);

        // Two presets of A into B: first children, in display order.
        let changes = lib.reorder(&mv(vec![PresetKey::Item(red), PresetKey::Item(green)], PresetKey::Group(b.clone()), DropPosition::Into)).unwrap();
        assert_eq!(
            changes,
            [PresetChange::Moved { id: red, to: b.clone() }, PresetChange::Moved { id: green, to: b.clone() }, PresetChange::Reordered(b.clone())]
        );
        assert_eq!(lib.group(&b).unwrap().items.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(), ["Red", "Green", "Cyan"]);

        // The target may be one of the moved rows: Red after Green, with Cyan.
        let cyan = lib.group(&b).unwrap().items[2].id;
        lib.reorder(&mv(vec![PresetKey::Item(red), PresetKey::Item(cyan)], PresetKey::Item(green), DropPosition::After)).unwrap();
        assert_eq!(lib.group(&b).unwrap().items.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(), ["Green", "Red", "Cyan"]);

        // A group next to a preset of the root becomes a top group; its presets go with it.
        let changes = lib.reorder(&mv(vec![PresetKey::group(["A", "Inner"])], PresetKey::Item(loose), DropPosition::After)).unwrap();
        assert_eq!(
            changes,
            [PresetChange::GroupMoved { from: a.child("Inner"), to: GroupPath::new(["Inner"]) }, PresetChange::Reordered(GroupPath::root())]
        );
        assert_eq!(lib.group_of(blue), Some(GroupPath::new(["Inner"])));

        // Reorder of groups at the same level: B before A.
        let changes = lib.reorder(&mv(vec![PresetKey::Group(b)], PresetKey::Group(a), DropPosition::Before)).unwrap();
        assert_eq!(changes, [PresetChange::Reordered(GroupPath::root())]);
        assert_eq!(lib.rows().iter().filter(|r| r.is_group()).map(|r| r.label.as_str()).collect::<Vec<_>>(), ["Inner", "B", "A"]);
    }

    // A group dropped into a folder past `with_nesting` is refused and the library is unchanged.
    #[test]
    fn library_refuses_nesting_past_its_depth() {
        let (mut lib, _) = sample();
        // Nesting 1: folders go one level below a top group. B (with a folder) into Inner would be depth 3.
        let b_inner = lib.new_group(&GroupPath::new(["B"]), "Sub").unwrap();
        let before_sub = lib.clone();
        let into_inner = mv(vec![PresetKey::group(["B"])], PresetKey::group(["A", "Inner"]), DropPosition::Into);
        assert_eq!(lib.reorder(&into_inner), Err(PresetError::TooDeep));
        assert_eq!(lib, before_sub);
        // A group into itself or its folder.
        let into_self = mv(vec![PresetKey::group(["A"])], PresetKey::group(["A", "Inner"]), DropPosition::Into);
        assert_eq!(lib.reorder(&into_self), Err(PresetError::TooDeep));
        assert_eq!(lib, before_sub);
        // A name a new sibling already has.
        lib.new_group(&GroupPath::new(["A"]), "Sub").unwrap();
        let snapshot = lib.clone();
        let clash = mv(vec![PresetKey::Group(b_inner)], PresetKey::Group(GroupPath::new(["A"])), DropPosition::Into);
        assert_eq!(lib.reorder(&clash), Err(PresetError::NameTaken));
        assert_eq!(lib, snapshot);
    }
}
