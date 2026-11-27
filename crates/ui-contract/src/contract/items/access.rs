//! AccessKit nodes and requests of item rows, section rows and toggle cells.

use super::{ITEM_NODE, ItemsHooks, Placement, SECTION, TOGGLE_CELL, key_hash, section_part};
use crate::contract::Context;
use crate::contract::collection::{ActivateOn, CollectionEvent, LeadSource, OpenChange};
use crate::contract::cx::Pointer as _;
use crate::contract::rt::items::RowsView;
use crate::contract::{Collection, Cx};
use crate::primitives::Id;
use crate::runtime::access::{self, Role};
use crate::runtime::focus;
use craft_behavior::ColumnKey;
use craft_ui_rt::{Pos2, Rect};

impl<C: Collection> ItemsHooks<'_, '_, C> {
    /// The access node of each shown toggle cell of row `(at, key)`, for the band row `item` to report under its cell:
    /// named by its cell, else by its column's header, carrying its state, and served by the collection `owner`. The
    /// collection claims each cell's part with a `Role::None` sub-node, so an AccessKit click the row hands on reaches
    /// `toggle_by_access`.
    pub(super) fn toggle_cell_nodes(
        &mut self,
        ctx: &Context,
        (owner, item): (Id, Id),
        (at, key): (usize, &C::Key),
        (cells, enabled): (&[(ColumnKey, Rect)], bool),
        env: &crate::runtime::env::EnvView<'_>,
    ) -> Vec<(ColumnKey, access::SubNode)> {
        let mut nodes = Vec::new();
        for (column, toggle) in self.header.toggle_columns() {
            if !cells.iter().any(|(held, _)| *held == column) {
                continue;
            }
            let Some(cell) = self.header.toggle_cell(ctx, item, &column, env) else { continue };
            let role = match (toggle.exclusive, cell.check_box) {
                (true, _) => Role::RADIO_BUTTON,
                (false, true) => Role::CHECK_BOX,
                (false, false) => Role::SWITCH,
            };
            let part = TOGGLE_CELL.at(key_hash(&(key, &column)));
            let mut node = access::SubNode::new(part, role);
            node.name = access::Name::Text(cell.name);
            node.flags.set(crate::style::Flags::CHECKED, self.source.toggled(at, &column));
            node.flags.set(crate::style::Flags::MIXED, cell.mixed);
            node.disabled = !(enabled && self.source.enabled(at) && self.toggle_open(at, &column));
            if let Some(chord) = toggle.key {
                node.shortcut(chord.display(ctx.os()));
            }
            node.served_by(owner, part);
            self.sub_nodes.push(access::SubNode::new(part, Role::None));
            nodes.push((column, node));
        }
        nodes
    }

    /// The collection's node for the section row `at` that the component does not lay out itself: a heading named by
    /// the row's text, expandable when it has children, that serves Click, Expand and Collapse.
    pub(super) fn report_section_node(&mut self, at: usize, rect: Rect, enabled: bool, origin: Pos2) {
        let mut node = access::SubNode::new(section_part(&self.source.key(at)), Role::HEADING);
        node.name = access::Name::Text(self.source.text(at).map(|text| text.into_owned()).unwrap_or_default());
        node.level(self.source.depth(at) + 1);
        if self.source.has_children(at) {
            node.expanded = Some(self.source.open(at));
        }
        node.disabled = !(enabled && self.source.enabled(at));
        node.bounds(rect.translate(Pos2::ZERO - origin));
        self.sub_nodes.push(node);
    }

    /// The sub-nodes the rows declared this run (the toggle cells the band rows report, as claims; section rows the
    /// component does not lay out), for the collection's access hook on the driver seam.
    pub(super) fn access_nodes(&self, access: &mut access::Access) {
        access.nodes.extend(self.sub_nodes.iter().cloned());
    }

    /// The rows the last build reached, as the rows view at the kept offset bands them, with the rows a gesture holds:
    /// the rows whose nodes and sub-nodes AccessKit requests can name.
    fn built_rows(&self, layout: &C::ItemLayout, place: Placement) -> Vec<usize> {
        let mut band = Vec::new();
        RowsView::new(place.rect, place.offset, self.scrolls).band_rows(layout, &mut band);
        self.held_rows(layout, band).1
    }

    /// AccessKit requests on an item row's node or on a section row's collection node (`report_section_node`): Focus
    /// moves the lead to the row and key focus to the collection; Click also picks an item and activates it when the
    /// collection activates on click, or activates a section; Expand and Collapse open and close rows with children.
    pub(super) fn node_by_access(
        &mut self,
        comp: &C,
        cx: &mut Cx<'_, C>,
        props: &crate::contract::CollectionProps,
        layout: &C::ItemLayout,
        place: Placement,
    ) {
        use craft_behavior::AccessAction;
        let rows: Vec<(u64, usize)> = self
            .built_rows(layout, place)
            .into_iter()
            .filter(|&at| !self.source.section_stop(at) || cx.part_if_declared(section_part(&self.source.key(at))).is_none())
            .map(|at| (key_hash(&self.source.key(at)), at))
            .collect();
        let wanted = |request: &craft_behavior::AccessRequest| {
            request.target.is_some_and(|part| {
                (part.key == ITEM_NODE || part.key == SECTION) && part.index.is_some_and(|hash| rows.iter().any(|(held, _)| *held == hash))
            })
        };
        while let Some(request) = cx.core.intents.take_access_where(wanted) {
            let Some(at) = request.target.and_then(|part| part.index).and_then(|hash| rows.iter().find(|(held, _)| *held == hash)).map(|(_, at)| *at)
            else {
                continue;
            };
            if !self.source.enabled(at) {
                continue;
            }
            let key = self.source.key(at);
            match request.action {
                AccessAction::Focus | AccessAction::Click => {
                    if self.lead.key.as_ref() != Some(&key) {
                        self.move_lead(at, LeadSource::Keys);
                    }
                    cx.core.request_focus(focus::FocusTarget::Id(cx.core.id));
                    if request.action == AccessAction::Click {
                        if self.source.section_stop(at) {
                            self.events.push(CollectionEvent::Activate(key));
                        } else {
                            self.click_pick(comp, at, false, false, craft_keymap::Mods::NONE);
                            if props.activate == ActivateOn::Click {
                                self.activation_clicks = 1;
                                self.events.push(CollectionEvent::Activate(key));
                            }
                        }
                    }
                }
                AccessAction::Expand | AccessAction::Collapse => {
                    let open = request.action == AccessAction::Expand;
                    if self.source.has_children(at) && self.source.open(at) != open {
                        self.events.push(CollectionEvent::Open(OpenChange::new(key, open)));
                    }
                }
                _ => {}
            }
        }
    }

    /// An AccessKit click on a toggle cell's sub-node flips that cell, as a click on it does.
    pub(super) fn toggle_by_access(&mut self, cx: &mut Cx<'_, C>, layout: &C::ItemLayout, place: Placement) {
        let wanted = |request: &craft_behavior::AccessRequest| {
            request.action == craft_behavior::AccessAction::Click && request.target.is_some_and(|part| part.key == TOGGLE_CELL)
        };
        while let Some(request) = cx.core.intents.take_access_where(wanted) {
            let Some(part) = request.target else { continue };
            let found = self.built_rows(layout, place).into_iter().find_map(|at| {
                let key = self.source.key(at);
                self.header
                    .toggle_columns()
                    .find(|(column, _)| TOGGLE_CELL.at(key_hash(&(&key, column))) == part)
                    .map(|(column, toggle)| (at, column, toggle))
            });
            if let Some((at, column, toggle)) = found {
                self.flip_toggle(cx, at, column, toggle);
            }
        }
    }
}
