//! `Lazy` on craft-ui-rt: a child of a scroll body that runs only while it is in view. Out of view
//! its content does not run and its nodes go; it keeps the extent it had when it last ran (its
//! node's size), or takes the app's estimate before its first run. In view it is a scope whose
//! closure runs whenever its owner's build reaches it; its children are kept nodes that cost a key
//! check. A wheel step inside the band its scroll body built moves kept content and runs nothing;
//! one that leaves the band builds the body again, and only the `Lazy` children entering the band
//! build their content.

use std::hash::{Hash, Hasher};

use craft_ui_rt::{Rect, Vec2};

use super::key::type_key;
use super::node::ScopeNode;
use super::ui::{Flow, Ui};
use crate::contract::IdSource;

/// A child of a scroll body that runs only while it is in view (module docs).
#[must_use]
pub struct Lazy {
    key: u64,
    estimate: f32,
    reveal: bool,
}

impl Lazy {
    /// The child keyed `key` among its siblings. `estimate` is how long it is down the scroll
    /// axis before it first ran.
    pub fn new(key: impl Hash, estimate: f32) -> Lazy {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        key.hash(&mut hasher);
        Lazy { key: hasher.finish(), estimate, reveal: false }
    }

    /// When `reveal` holds, the scroll body glides until the child starts at the view's start, as
    /// far as the content lets it. A wheel or drag ends the glide.
    pub fn reveal(mut self, reveal: bool) -> Lazy {
        self.reveal = reveal;
        self
    }

    /// Runs `add` in the child's flow, or takes its kept extent out of view; `None` when it did not
    /// run.
    pub fn show<R>(self, ui: &mut Ui<'_, '_>, mut add: impl FnMut(&mut Ui<'_, '_>) -> R) -> Option<R> {
        let (id, child) = ui.flow.child(type_key::<Lazy>(), IdSource::Id(ui.flow.id.with(("lazy", self.key))));
        let start = ui.flow.next_min();
        let room = ui.flow.room();
        let estimate = Vec2::new(room.x, self.estimate);
        if self.reveal {
            ui.flow.reveal = Some(start);
        }
        let in_view = |extent: Vec2, view: Option<Rect>| view.is_none_or(|v| v.intersects(Rect::from_min_size(start, extent)));
        let region = Rect::from_min_size(start, Vec2::new(room.x, f32::INFINITY));
        let gap = ui.flow.tokens.space.gap.stack;
        if let Some(under) = ui.flow.measuring {
            let peeked =
                under.and_then(|u| ui.cx.peek::<ScopeNode>(u, child)).map(|p| (p.node, (p.local.id != egui::Id::NULL).then_some(p.local.size)));
            let extent = peeked.and_then(|(_, size)| size).unwrap_or(estimate);
            if !in_view(extent, ui.flow.view) {
                ui.flow.allocate(None, extent, false);
                return None;
            }
            let flow = Flow::new(id, start, region, gap, ui.flow.tokens.clone()).in_view(ui.flow.view).measuring(peeked.map(|(n, _)| n));
            let mut inner = Ui::new(ui.cx, flow);
            let out = add(&mut inner);
            let size = inner.flow.content_size();
            ui.flow.absorb(&inner.flow);
            ui.flow.allocate(None, size, false);
            return Some(out);
        }
        let Ui { cx, flow } = ui;
        cx.keyed(child, |cx| {
            cx.inline(ScopeNode { key: 0 }, |kept, _, cx| {
                let shown = kept.id != egui::Id::NULL;
                let extent = if shown { kept.size } else { estimate };
                if !in_view(extent, flow.view) {
                    flow.limited = true;
                    flow.allocate(Some(cx.node()), extent, false);
                    if !kept.placed.is_empty() {
                        kept.placed.clear();
                        cx.request_layout();
                    }
                    return None;
                }
                cx.count_scope();
                let mut inner = Ui::new(cx, Flow::new(id, start, region, gap, flow.tokens.clone()).in_view(flow.view));
                let out = add(&mut inner);
                let Ui { cx, flow: inner } = inner;
                let size = inner.content_size();
                flow.absorb(&inner);
                flow.allocate(Some(cx.node()), size, false);
                if kept.placed != inner.placed || kept.size != size || !shown {
                    kept.placed = inner.placed;
                    kept.size = size;
                    kept.id = id;
                    cx.mark_changed();
                }
                Some(out)
            })
        })
    }
}
