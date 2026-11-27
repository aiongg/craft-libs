//! Items that are members of the collection's run (`Collection::MEMBERS`, docs/contract-rt.md 15.5): each row is a
//! part `(ITEM_NODE, key hash)` of the collection's node, not a node. The driver runs each member's hooks at its row
//! with the input the collection's node took for that part (`rt::run::run_member`), records its paint per pointer
//! look into the row's own lists, and reports it as a sub-node of the collection's. ui-rt's paint shows each row's
//! list for the look of that row's part (`PaintCx::hovered_part`), so a hover over a member row repaints the
//! collection's node from kept lists and runs nothing; a row's state change runs the collection.

use super::{ITEM_NODE, ItemsHooks, key_hash};
use crate::contract::Context;
use crate::contract::Ui;
use crate::contract::collection::ShownRow;
use crate::contract::rt::items::{MemberLooks, MemberRun};
use crate::contract::{Collection, Cx, Internal, ItemOpener, ItemOutput as _, Meta as _};
use crate::primitives::Id;
use crate::runtime::access::{self, SetPosition};
use crate::runtime::env::Env;
use crate::style::Flags;
use craft_ui_rt::Rect;

/// A visible item of a members collection, built by the row loop and run once every row is placed.
pub(super) struct Member<K, I> {
    pub at: usize,
    pub key: K,
    pub item: I,
    pub row: Rect,
    /// The row's clip: below the frozen rows.
    pub clip: Rect,
    /// The row's flags: selected, lead, expanded, current, dragging, drop target.
    pub flags: Flags,
    pub position: SetPosition,
    pub renaming: bool,
    pub openers: Vec<ItemOpener>,
    /// The member's place in the run's shown rows.
    pub shown: usize,
}

impl<'r, C: Collection> ItemsHooks<'_, 'r, C> {
    /// Runs each member at its row as a part of the collection's node, keeps its recorded looks for the driver
    /// (`take_member_looks`), leaves the hovered member's tooltip for step 16 and reports its output as the
    /// collection's item events.
    pub(super) fn show_members(
        &mut self,
        comp: &mut C,
        ui: &mut Ui,
        cx: &mut Cx<'_, C>,
        ((ctx, id, rtl), _env): ((&Context, Id, bool), &Env),
        members: Vec<Member<C::Key, C::Item<'r>>>,
        shown_rows: &mut [ShownRow<C::Key>],
    ) {
        let enabled = cx.core.resolved.enabled;
        self.member_looks.clear();
        for member in members {
            let Member { at, key, mut item, row, clip, flags, position, renaming, openers, shown } = member;
            let member_id = item_id(id, &key);
            let on = enabled && item.common_mut(Internal::new()).enabled.unwrap_or(true);
            let part = ITEM_NODE.at(key_hash(&key));
            let run = MemberRun { owner: id, id: member_id, part, rect: row, clip, position, flags, enabled: on };
            let ran = crate::contract::rt::run::run_member(item, ui, &run);
            self.member_looks.push(ran.looks);
            if ran.tip.is_some() {
                cx.core.member_tip = ran.tip;
            }
            if let Some(shown) = shown_rows.get_mut(shown) {
                shown.openers = self.item_openers(ui, ctx, id, enabled, openers);
            }
            let label = ran.output.label_rect().unwrap_or(row);
            self.report_item_output(ctx, id, &key, ran.output);
            if renaming {
                self.rename_row(comp, ui, (ctx, id, rtl), (at, &key), label);
            }
        }
    }

    /// The member rows' looks this run recorded, for the collection node's `Kept.members`: the driver takes them after
    /// `content`. Empty for a collection of band rows.
    pub(crate) fn take_member_looks(&mut self) -> Vec<MemberLooks> {
        std::mem::take(&mut self.member_looks)
    }
}

/// The id of the item `key` of the collection `id`: a member's owner id in the arena, as a run item's id.
pub(super) fn item_id<K: std::hash::Hash>(id: Id, key: &K) -> Id {
    crate::runtime::memory::sub(id, ("key", key_hash(key)))
}

/// The access node of the item `key` of the members collection `id` (a tab strip, a swatch table): the sub-node of its
/// item part, which [`focused_node`](crate::runtime::access::focused_node) reports while the item is the lead of the
/// focused collection.
pub fn item_node<K: std::hash::Hash>(id: Id, key: &K) -> Id {
    access::sub_node_id(id, ITEM_NODE.at(key_hash(key)))
}
