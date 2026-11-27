//! Scopes (docs/contract-rt.md section 2.3): the window root and `MemoView`. A scope runs its
//! closure only when it must; otherwise it returns its kept value and keeps its subtree.

use craft_ui_rt::{BuildCx, Pos2, Rect, Vec2};
use egui::Id;

use super::key::{fold, type_key};
use super::node::{Place, ScopeNode};
use super::ui::{Flow, Ui};
use crate::contract::{IdSource, MemoKey, MemoProps};

/// The content of a window's inline root: call it from the closure passed to
/// `Driver::step_with`/`Runtime::frame_with`. The root scope runs whenever ui-rt runs that closure:
/// input reached a node under it, or a read in it changed (it reads the viewport and the tokens).
pub fn window_root<R>(cx: &mut BuildCx<'_>, f: impl FnOnce(&mut Ui<'_, '_>) -> R) -> R {
    let window = cx.window();
    cx.inline(ScopeNode { key: 0 }, |kept, state, cx| {
        cx.count_scope();
        if state.new {
            cx.mark_changed();
        }
        let viewport = cx.env().viewport.get(cx);
        let tokens = cx.tokens();
        let gap = tokens.space.gap.stack;
        let region = Rect::from_min_size(Pos2::ZERO, viewport);
        let mut ui = Ui::new(cx, Flow::new(Id::NULL.with(("craft-window", window)), Pos2::ZERO, region, gap, tokens));
        let out = f(&mut ui);
        let Ui { cx, flow } = ui;
        if kept.placed != flow.placed || kept.size != viewport {
            kept.placed = flow.placed;
            kept.size = viewport;
            cx.request_layout();
        }
        out
    })
}

/// A memoized view: its closure runs only when its props key or its place moved, a read in its last
/// run changed, or a node under it was marked. Otherwise `show` returns the last run's value and the
/// subtree stays as it is, at the cost of a key check.
pub struct MemoView<P> {
    id: IdSource,
    props: P,
}

impl<P: MemoProps> MemoView<P> {
    /// A view keyed by `props`.
    pub fn new(props: P) -> MemoView<P> {
        MemoView { id: IdSource::Auto, props }
    }

    /// Where the view's id comes from.
    pub fn id_source(mut self, id: IdSource) -> MemoView<P> {
        self.id = id;
        self
    }

    /// Runs `view` with the props, or returns its kept value. In a measuring run a kept view
    /// answers with its kept size and value; one that must run runs `view` as a measuring run.
    pub fn show<R: Clone + 'static>(self, ui: &mut Ui<'_, '_>, mut view: impl FnMut(&mut Ui<'_, '_>, &P) -> R) -> R {
        let (id, child) = ui.flow.child(type_key::<MemoView<P>>(), self.id);
        let mut key = MemoKey::new();
        key.feed_hash(&P::identity());
        self.props.key(&mut key);
        let key = fold(key.finish());
        let place = Place { room: ui.flow.room() };
        let origin = ui.flow.next_min();
        if let Some(under) = ui.flow.measuring {
            let peeked = under.and_then(|u| ui.cx.peek::<ScopeNode>(u, child));
            let node = peeked.as_ref().map(|p| p.node);
            let kept = peeked
                .filter(|p| !p.state.path && !p.state.marked && p.local.place == place)
                .and_then(|p| p.local.value.as_ref().and_then(|v| v.downcast_ref::<R>()).map(|v| (v.clone(), p.local.size)));
            if let Some((value, size)) = kept {
                ui.flow.allocate(None, size, false);
                return value;
            }
            let tokens = ui.flow.tokens.clone();
            let gap = tokens.space.gap.stack;
            let mut inner =
                Ui::new(ui.cx, Flow::new(id, origin, Rect::from_min_size(origin, place.room), gap, tokens).in_view(ui.flow.view).measuring(node));
            let out = view(&mut inner, &self.props);
            let size = inner.flow.content_size();
            ui.flow.absorb(&inner.flow);
            ui.flow.allocate(None, size, false);
            return out;
        }
        let Ui { cx, flow } = ui;
        cx.keyed(child, |cx| {
            cx.inline(ScopeNode { key }, |kept, state, cx| {
                let value = kept.value.as_ref().and_then(|v| v.downcast_ref::<R>());
                if let Some(value) = value.filter(|_| !state.must_run() && !state.path && kept.place == place) {
                    cx.keep_children();
                    flow.allocate(Some(cx.node()), kept.size, false);
                    return value.clone();
                }
                cx.count_scope();
                if state.new {
                    cx.mark_changed();
                }
                let tokens = cx.tokens();
                let gap = tokens.space.gap.stack;
                let region = Rect::from_min_size(origin, place.room);
                let mut inner = Ui::new(cx, Flow::new(id, origin, region, gap, tokens).in_view(flow.view));
                let out = view(&mut inner, &self.props);
                let Ui { cx, flow: inner } = inner;
                let size: Vec2 = inner.content_size();
                flow.absorb(&inner);
                flow.allocate(Some(cx.node()), size, false);
                if kept.placed != inner.placed || kept.size != size {
                    kept.placed = inner.placed;
                    kept.size = size;
                    cx.request_layout();
                }
                kept.id = id;
                kept.place = place;
                kept.value = Some(Box::new(out.clone()));
                out
            })
        })
    }
}
