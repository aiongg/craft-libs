//! Scopes (docs/contract-rt.md section 2.3): the window root, and the keyed scopes `MemoView` and `Lazy` show
//! ([`show_scope`]). A memo scope runs its closure only when it must; otherwise it returns its kept value and keeps
//! its subtree. Inside a scroll body either scope out of view keeps only its extent.

use craft_ui_rt::{BuildCx, Pos2, Rect, Vec2};

use super::env::EnvCells;
use super::node::{Place, ScopeNode};
use super::pending::JobBoard;
use super::response::{begin_reads, end_reads};
use super::sweep::Sweep;
use super::ui::{Flow, Ui};
use crate::contract::{Id, IdSource, Shown};
use crate::contract::{MemoKey, MemoProps};
use crate::runtime::shift::RecordShift;

/// The content of a window's inline root: call it from the closure passed to
/// `Driver::step_with`/`Runtime::frame_with`. The root scope runs whenever ui-rt runs that closure:
/// input reached a node under it, or a read in it changed (it reads the viewport and the tokens,
/// and the interaction facts its code reads from responses).
pub fn window_root<R>(cx: &mut BuildCx<'_>, f: impl FnOnce(&mut Ui<'_, '_>) -> R) -> R {
    window_root_with(cx, WindowEnv::default(), f)
}

/// What the host gives a window root: cells it writes (a language change, a theme switch) and the board
/// its jobs deliver through. A window with none gets cells of its own and a board no thread wakes.
#[derive(Default, Clone)]
pub struct WindowEnv {
    /// The env cells of the window.
    pub cells: Option<EnvCells>,
    /// The background jobs of the window.
    pub jobs: Option<JobBoard>,
    /// The app's commands: the window root runs the command a key no node took is bound to.
    pub commands: Option<super::commands::CommandBoard>,
}

/// [`window_root`] with what the app host gives every window (`craft_ui::app`): its env cells, job board and
/// command board. Public at `runtime::windows::window_root_with`.
pub fn window_root_with<R>(cx: &mut BuildCx<'_>, env: WindowEnv, f: impl FnOnce(&mut Ui<'_, '_>) -> R) -> R {
    let window = cx.window();
    cx.inline(ScopeNode { key: 0 }, |kept, state, cx| {
        cx.count_scope();
        if state.new {
            cx.mark_changed();
        }
        let cells = match (env.cells, kept.env) {
            (Some(given), _) => given,
            (None, Some(own)) => own,
            (None, None) => {
                let tokens = cx.env().tokens;
                EnvCells::owned(cx, tokens)
            }
        };
        kept.env = Some(cells);
        kept.window_root = true;
        cx.provide(cells);
        crate::fonts::refresh_facts(cx, cells);
        let ids = cx.slot("craft-node-ids", super::ids::NodeIds::default);
        let ids = cx.peek_state(ids).clone();
        cx.provide(ids);
        if let Some(jobs) = env.jobs {
            cx.provide(jobs);
        }
        if let Some(commands) = env.commands {
            cx.provide(commands.clone());
            kept.commands = Some(commands);
        }
        crate::contract::context::Context::enter_root(cx, Some(cells));
        let reads = begin_reads(true);
        let viewport = cx.env().viewport.get(cx);
        let tokens = super::env::tokens(cx);
        let gap = tokens.space.gap.stack;
        let region = Rect::from_min_size(Pos2::ZERO, viewport);
        let sweep = match kept.sweep {
            Some(cell) => cell,
            None => *kept.sweep.insert(cx.state(Sweep::default())),
        };
        let flow = Flow::new(Id::NULL.with(("craft-window", window)), Pos2::ZERO, region, gap, tokens).with_sweep(Some(sweep));
        let mut ui = Ui::new(cx, flow);
        let out = f(&mut ui);
        let Ui { cx, flow, .. } = ui;
        end_reads(reads, cx);
        if kept.placed != flow.placed || kept.size != viewport {
            kept.placed = flow.placed;
            kept.size = viewport;
            cx.request_layout();
        }
        out
    })
}

/// What a keyed scope is: a `MemoView` (its closure runs only when its key, place or reads moved, or a node under it
/// is marked) or a `Lazy` (its closure runs whenever its owner's build reaches it in view).
pub(crate) struct ScopeShow {
    /// The scope's craft `Id` source and its ui-rt type key.
    pub(crate) id: IdSource,
    pub(crate) type_key: u64,
    /// The memo key (props, `fn`, scope); `None` for a `Lazy`.
    pub(crate) memo: Option<u64>,
    /// The extent down the scroll axis before the first run.
    pub(crate) estimate: f32,
    /// The scroll body glides until the scope starts at the view's start.
    pub(crate) reveal: bool,
    /// Out of a scroll body's view the scope keeps only its extent (`MemoView`, `Lazy`); an env scope always shows.
    pub(crate) lazy: bool,
}

/// How a kept value moves with its scope (`Shift::shifted`); `None` keeps it as it is.
pub(crate) type ShiftFn<R> = Option<fn(&R, &RecordShift) -> R>;

/// What a memo scope keeps of its last run: the value and where the scope started, so a keep at another position
/// returns the value moved (`Shift`).
struct KeptValue<R> {
    value: R,
    origin: Pos2,
}

/// Shows a keyed scope in the flow of `ui` (module docs). `None` when it is out of the view of the scroll body it is
/// in: it keeps its extent then, and its nodes go. `Shown::fresh` is true when `content` ran in this build.
pub(crate) fn show_scope<R: Clone + 'static>(
    ui: &mut Ui<'_, '_>,
    scope: ScopeShow,
    shift: ShiftFn<R>,
    content: &mut dyn FnMut(&mut Ui<'_, '_>) -> R,
) -> Option<Shown<R>> {
    let (id, child) = ui.flow.child(scope.type_key, scope.id);
    let start = ui.flow.next_min();
    let room = ui.flow.room();
    let place = Place { room };
    if scope.reveal {
        ui.flow.reveal = Some(start);
    }
    let estimate = Vec2::new(room.x, scope.estimate);
    let in_view = |extent: Vec2, view: Option<Rect>| !scope.lazy || view.is_none_or(|v| v.intersects(Rect::from_min_size(start, extent)));
    // A memo scope's content is a layout room of its own (its place's room); a lazy one runs in the column below it.
    let region = match scope.memo {
        Some(_) => Rect::from_min_size(start, room),
        None => Rect::from_min_size(start, Vec2::new(room.x, f32::INFINITY)),
    };
    let gap = ui.flow.tokens.space.gap.stack;
    if let Some(under) = ui.flow.measuring {
        let peeked = under.and_then(|u| ui.cx.peek::<ScopeNode>(u, child));
        let node = peeked.as_ref().map(|p| p.node);
        let shown = peeked.as_ref().filter(|p| p.local.id != Id::NULL).map(|p| p.local.size);
        let extent = shown.unwrap_or(estimate);
        if !in_view(extent, ui.flow.view) {
            ui.flow.allocate(None, extent, false);
            return None;
        }
        // A kept memo scope answers with its kept size and value.
        let kept = peeked
            .filter(|p| scope.memo.is_some() && p.local.key == scope.memo.map(u128::from))
            .filter(|p| !p.state.path && !p.state.marked && p.local.place == place)
            .and_then(|p| kept_value::<R>(p.local, start, shift).map(|v| (v, p.local.size)));
        if let Some((value, size)) = kept {
            ui.flow.allocate(None, size, false);
            return Some(Shown { value, fresh: false });
        }
        let flow = Flow::new(id, start, region, gap, ui.flow.tokens.clone()).in_view(ui.flow.view).measuring(node);
        let mut inner = Ui::new(ui.cx, flow);
        let value = content(&mut inner);
        let size = inner.flow.content_size();
        ui.flow.absorb(&inner.flow);
        ui.flow.allocate(None, size, false);
        return Some(Shown { value, fresh: true });
    }
    let Ui { cx, flow, .. } = ui;
    let key = scope.memo.unwrap_or(0);
    cx.keyed(child, |cx| {
        cx.inline(ScopeNode { key }, |kept, state, cx| {
            let shown = kept.id != Id::NULL;
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
            if scope.memo.is_some()
                && shown
                && !state.must_run()
                && !state.path
                && kept.place == place
                && let Some(value) = kept_value::<R>(kept, start, shift)
            {
                cx.keep_children();
                crate::layout::replay(&crate::contract::Context::of_build(cx), &kept.column_reports);
                flow.allocate(Some(cx.node()), kept.size, false);
                return Some(Shown { value, fresh: false });
            }
            cx.count_scope();
            if state.new {
                cx.mark_changed();
            }
            let tokens = if scope.memo.is_some() { super::env::tokens(cx) } else { flow.tokens.clone() };
            let reads = scope.memo.map(|_| begin_reads(false));
            let mut inner = Ui::new(cx, Flow::new(id, start, region, gap, tokens).in_view(flow.view).with_sweep(flow.sweep));
            let ctx = inner.ctx().clone();
            crate::layout::begin_capture(&ctx);
            let value = content(&mut inner);
            kept.column_reports = crate::layout::end_capture(&ctx);
            let Ui { cx, flow: inner, .. } = inner;
            if let Some(reads) = reads {
                end_reads(reads, cx);
            }
            let size: Vec2 = inner.content_size();
            flow.absorb(&inner);
            flow.allocate(Some(cx.node()), size, false);
            if kept.placed != inner.placed || kept.size != size || !shown {
                kept.placed = inner.placed;
                kept.size = size;
                cx.mark_changed();
                cx.request_layout();
            }
            kept.id = id;
            kept.note_id(cx, id);
            kept.place = place;
            if scope.memo.is_some() {
                kept.key = Some(u128::from(key));
                kept.value = Some(Box::new(KeptValue { value: value.clone(), origin: start }));
            }
            Some(Shown { value, fresh: true })
        })
    })
}

/// A memo scope's kept value, moved to `start` when the scope starts elsewhere than where it ran.
fn kept_value<R: Clone + 'static>(kept: &super::node::Kept, start: Pos2, shift: ShiftFn<R>) -> Option<R> {
    let kept = kept.value.as_ref()?.downcast_ref::<KeptValue<R>>()?;
    let offset = start - kept.origin;
    Some(match shift {
        Some(shift) if offset != Vec2::ZERO => shift(&kept.value, &RecordShift::by(offset)),
        _ => kept.value.clone(),
    })
}

/// A memo scope keyed by `props` with no out-of-view rule: an env scope's subtree. Its closure runs only when the
/// props key or the place moved, a read in its last run changed, or a node under it was marked.
pub(crate) fn memo<P: MemoProps, R: Clone + 'static>(ui: &mut Ui<'_, '_>, props: &P, mut content: impl FnMut(&mut Ui<'_, '_>, &P) -> R) -> R {
    let mut key = MemoKey::new();
    key.feed_hash(&P::identity());
    props.key(&mut key);
    let scope = ScopeShow {
        id: IdSource::Auto,
        type_key: super::key::type_key::<P>(),
        memo: Some(super::key::fold(key.finish())),
        estimate: 0.0,
        reveal: false,
        lazy: false,
    };
    match show_scope(ui, scope, None, &mut |ui| content(ui, props)) {
        Some(shown) => shown.value,
        None => unreachable!("a scope with no out-of-view rule always shows"),
    }
}
