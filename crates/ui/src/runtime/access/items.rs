//! AccessKit policy of collections (17.3 step 16): the header cells as `ColumnHeader` sub-nodes.

use super::emit::{SubDecl, report_sub};
use super::{Name, Role, SubNode};
use craft_behavior::PartRef;
use egui::{Context, Id, Rect};

/// A header cell as step 16 reports it.
pub(crate) struct HeaderCell<'a> {
    pub part: PartRef,
    pub name: &'a str,
    pub rect: Rect,
    /// Activating the cell sorts.
    pub clickable: bool,
    /// The cell holds the collection's keyboard focus.
    pub active: bool,
}

/// The `ColumnHeader` sub-nodes of the collection `owner`. Every cell is a focus stop of the header's roving group.
pub(crate) fn report_header(ctx: &Context, owner: Id, enabled: bool, cells: &[HeaderCell<'_>]) {
    for cell in cells {
        let mut node = SubNode::new(cell.part, Role::COLUMN_HEADER);
        node.name = Name::Text(cell.name.to_owned());
        report_sub(
            ctx,
            &SubDecl {
                owner,
                component: "Collection",
                node: &node,
                rect: cell.rect,
                enabled,
                focusable: enabled,
                clickable: enabled && cell.clickable,
                active: cell.active,
                face: None,
                label: cell.name,
                parent: owner,
            },
        );
    }
}
