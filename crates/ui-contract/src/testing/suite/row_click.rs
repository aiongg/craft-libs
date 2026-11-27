//! The point at which the suite presses a collection row to give it the lead or pick it: a point of the row's own
//! area (17.5). A row that holds child components (a switch, a select), or that the collection draws controls over (a
//! grid's filter buttons in its header cells, its pointer-only column edges), is pressed where none of them is, so the
//! press reaches the row.

use super::nodes::subtree;
use crate::contract::rt::harness::Target;
use crate::testing::story_run::StoryHarness;
use accesskit::Action;
use craft_ui_rt::{Pos2, Rect};

/// The point of `row` to press: its centre when no focusable or clickable node of its collection (the row's parent:
/// the row's own children and the controls the collection shows beside or over its rows) and no pointer-only part of
/// the collection, its children or its ancestors (a press part with no node, such as a grid's column edge) covers it,
/// else the middle of the widest stretch of the row's centre line that none covers, and the centre when they cover the
/// whole line. Nodes outside the collection, such as the field a popup list opens over, lie under it and are not
/// counted.
pub(super) fn point(fixture: &StoryHarness, row: &Target) -> Pos2 {
    let tree = &fixture.tree();
    let centre = row.bounds.center();
    let collection = tree.parent_of_key(row.id).unwrap_or(row.id);
    let inside = subtree(tree, collection);
    let nodes = inside
        .iter()
        .copied()
        .filter(|node| *node != collection && *node != row.id)
        .filter(|node| tree.declares_key(*node, Action::Focus) || tree.declares_key(*node, Action::Click))
        .filter_map(|node| tree.bounds_of(node));
    let owners: Vec<accesskit::NodeId> =
        inside.iter().copied().chain(std::iter::successors(tree.parent_of_key(collection), |node| tree.parent_of_key(*node))).collect();
    let pointer_only = fixture
        .press_parts()
        .into_iter()
        .filter(|(owner, part, _)| fixture.node_of(*owner).is_some_and(|node| owners.contains(&node.id)) && fixture.part(*owner, *part).is_none())
        .map(|(_, _, bounds)| bounds);
    let mut covered: Vec<Rect> = nodes.chain(pointer_only).filter(|bounds| bounds.min.y <= centre.y && centre.y <= bounds.max.y).collect();
    if !covered.iter().any(|bounds| bounds.contains(centre)) {
        return centre;
    }
    covered.sort_by(|a, b| a.min.x.total_cmp(&b.min.x));
    let mut widest: Option<(f32, f32)> = None;
    let mut free = row.bounds.min.x;
    for (from, to) in covered.iter().map(|bounds| (bounds.min.x, bounds.max.x)).chain([(row.bounds.max.x, row.bounds.max.x)]) {
        if from > free && widest.is_none_or(|(start, end)| from - free > end - start) {
            widest = Some((free, from.min(row.bounds.max.x)));
        }
        free = free.max(to);
    }
    widest.map_or(centre, |(start, end)| Pos2::new((start + end) / 2.0, centre.y))
}

/// Presses `row` at its own area.
pub(super) fn click(fixture: &mut StoryHarness, row: &Target) {
    let at = point(fixture, row);
    fixture.click_at(at);
}
