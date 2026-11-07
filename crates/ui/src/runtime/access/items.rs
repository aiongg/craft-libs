//! Item access policy: where AccessKit focus goes inside a collection (17.4, 17.5, 17.7).

use crate::runtime::egui::access::sub_node_id;
use crate::runtime::focus::StopKey;
use craft_behavior::PartKey;
use egui::Id;

/// The node AccessKit focus goes to: the focused child or part of the lead row, else the lead row's cell
/// sub-node when the collection selects cells, else the lead row's node.
pub(crate) fn focus_node(row: Id, child: Option<StopKey>, cell: Option<usize>) -> Id {
    match (child, cell) {
        (Some((id, Some(part))), _) => sub_node_id(id, part),
        (Some((id, None)), _) => id,
        (None, Some(column)) => sub_node_id(row, PartKey::CELL.at(column as u64)),
        (None, None) => row,
    }
}
