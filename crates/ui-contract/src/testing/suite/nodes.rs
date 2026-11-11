//! What the container and collection cases read from the AccessKit tree of a fixture.

use crate::runtime::access::Role;
use crate::runtime::egui::access::NodeKey;
use crate::testing::fixture::{Fixture, Target};

/// The node of the latest shown component of a story, when the tree has it.
pub(super) fn root(fixture: &Fixture) -> Option<Target> {
    fixture.root_id().and_then(|id| fixture.node_of(id))
}

/// Whether the node AccessKit reports as focused lies in the subtree of `root`.
pub(super) fn focus_within(fixture: &Fixture, root: NodeKey) -> bool {
    let tree = fixture.tree();
    tree.subtree(root).contains(&tree.focused_key())
}

/// The roles of the rows of a collection: list items, list box options, tree items and table rows.
const ROW_ROLES: [Role; 4] = [Role::LIST_ITEM, Role::LIST_BOX_OPTION, Role::TREE_ITEM, Role::ROW];

/// Whether `role` is a row of a collection, which the item step gives `LEAD`, `DRAGGING` and `DROP_TARGET`.
pub(super) fn is_row_role(role: Role) -> bool {
    ROW_ROLES.contains(&role)
}

/// Whether `role` is a menu entry, which shows `LEAD` on the entry the menu rests on.
pub(super) fn is_menu_item_role(role: Role) -> bool {
    [Role::MENU_ITEM, Role::MENU_ITEM_CHECK_BOX, Role::MENU_ITEM_RADIO].contains(&role)
}

/// The rows of the collection below `root`, top to bottom. A node with no place in a set, such as a header row, is
/// not a row.
pub(super) fn rows(fixture: &Fixture, root: NodeKey) -> Vec<Target> {
    let tree = fixture.tree();
    let inside = tree.subtree(root);
    let mut rows: Vec<Target> = ROW_ROLES
        .into_iter()
        .flat_map(|role| fixture.by_role(role))
        .filter(|target| inside.contains(&target.node) && position_in_set(fixture, target.node).is_some())
        .collect();
    rows.sort_by(|a, b| a.rect.min.y.total_cmp(&b.rect.min.y));
    rows
}

/// The row of `rows` that holds the AccessKit focus, with its place among them.
pub(super) fn focused_row(fixture: &Fixture, rows: &[Target]) -> Option<(usize, NodeKey)> {
    let tree = fixture.tree();
    let focused = tree.focused_key();
    std::iter::once(focused).chain(tree.ancestors(focused)).find_map(|key| rows.iter().position(|row| row.node == key).map(|index| (index, key)))
}

/// The zero-based position and the set size a row reports.
pub(super) fn position_in_set(fixture: &Fixture, row: NodeKey) -> Option<(usize, usize)> {
    let node = fixture.tree().data_of(row)?;
    Some((node.position_in_set()?, node.size_of_set()?))
}

/// Whether the node reports itself selected.
pub(super) fn is_selected(fixture: &Fixture, node: NodeKey) -> bool {
    fixture.tree().data_of(node).and_then(|data| data.is_selected()).unwrap_or(false)
}
