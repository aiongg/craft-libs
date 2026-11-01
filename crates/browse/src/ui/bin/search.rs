//! `name_matches` and the flat list of a search's matches.

use super::{BinEntry, BinModel};

/// Whether `name` matches the search `filter`: every whitespace-separated word of `filter` is in
/// `name`, in any order, both compared in Unicode lower case (`str::to_lowercase`; FilmCraft's
/// ASCII lower case missed other scripts). An empty or blank filter matches everything.
pub fn name_matches(name: &str, filter: &str) -> bool {
    // filmcraft: crates/ui-egui/src/panels/project_views.rs:85 (`name_matches`).
    let name = name.to_lowercase();
    filter.to_lowercase().split_whitespace().all(|word| name.contains(word))
}

/// The items under `parent` (`None`: the root), sub-bins included, whose names match `filter`, in
/// the model's order. Bins are searched through but are not results. `depth` stops a model that
/// contains itself.
pub(super) fn collect_matches<K: Clone, M: BinModel<K> + ?Sized>(model: &M, parent: Option<&K>, filter: &str, depth: usize, out: &mut Vec<K>) {
    const MAX_DEPTH: usize = 64;
    if depth > MAX_DEPTH {
        return;
    }
    let mut children = Vec::new();
    model.children(parent, &mut children);
    let mut item = super::BinItem::default();
    for child in children {
        match child {
            BinEntry::Bin(key) => collect_matches(model, Some(&key), filter, depth + 1, out),
            BinEntry::Item(key) => {
                item.name.clear();
                model.item(&key, &mut item);
                if name_matches(&item.name, filter) {
                    out.push(key);
                }
            }
        }
    }
}
