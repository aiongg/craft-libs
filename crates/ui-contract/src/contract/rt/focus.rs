//! The focus service on craft-ui-rt (docs/contract-rt.md sections 5 and 10): the stops, focus
//! scopes and roving groups a node's run declares are kept with the node, and the service walks the
//! kept nodes when a key or a request needs them. A kept node keeps its stop, so the Tab order needs
//! no run. The service moves ui-rt's key focus; the focus ring is painted from the node's
//! `FocusVisible` fact, so a focus move repaints the two nodes and runs none.
//!
//! - Tab: a scope that traps (a popup, a dialog, a leaf with a trapping scope) wraps Tab among its
//!   own stops; otherwise the window root moves along the window's order, which leaves out the stops
//!   inside trapping scopes and enters the topmost trapping layer child when focus is outside it.
//! - A roving group is one Tab stop: Tab lands on the member that last had focus (else the first),
//!   and the owner moves focus among the members with the arrows its `Roving` takes.
//! - F6 moves focus to the next pane (`ScopeKind::Pane`), to the stop that last had focus there.
//! - A request (an assistive technology's Focus) is refused outside the topmost trapping layer
//!   child; one that names a scope or a group focuses its first stop (a group's lead).
//! - A layer child whose scope says so takes focus when it appears (`Initial`) and gives it back to
//!   the stop it came from when it goes with focus inside (`restore`).

use std::sync::atomic::{AtomicU64, Ordering};

use craft_behavior::keys::intent;
use craft_behavior::{Intent, KeyRole, NavOutline, NavStep, Roving, ScopeKind};
use craft_ui_rt::{BuildCx, EventCx, Handled, Key, KeyInput, NodeId, Rect, Runtime, Stroke};

use super::node::{ComponentNode, Kept, ScopeNode};
use super::overlay::LayerScope;
use crate::runtime::focus::{FocusPolicy, Initial, KeyUse};

/// What a node's last run declared for focus and keys; part of its [`Kept`].
#[derive(Clone, Debug, Default)]
pub(crate) struct FocusKept {
    /// The node's own shape is a Tab stop that takes these keys while focused.
    pub(crate) stop: Option<KeyUse>,
    /// The scope the node opens around its content; `FocusPolicy::NONE` opens none.
    pub(crate) scope: FocusPolicy,
    /// The roving group over the stops in the node's content.
    pub(crate) roving: Option<Roving>,
    /// The run asked for the next Escape ([`super::Cx::hold_escape`]): an open draft or gesture.
    pub(crate) holds_escape: bool,
    /// The ring the node paints while it shows focus.
    pub(crate) ring: Option<Stroke>,
    /// When the node last took focus, in the focus sequence; 0 when it never did.
    pub(crate) seq: u64,
}

/// The focus sequence: each focus gain takes the next number, so a group or a pane goes back to the
/// stop that had focus last.
static SEQ: AtomicU64 = AtomicU64::new(1);

impl FocusKept {
    /// Notes that the node took focus.
    pub(crate) fn took_focus(&mut self) {
        self.seq = SEQ.fetch_add(1, Ordering::Relaxed);
    }

    fn traps(&self) -> bool {
        self.scope.trap
    }
}

/// What a run's `Cx` reads of focus: `focused` is the node's tracked `Focused` fact (only a stop
/// reads it), `visible` its untracked `FocusVisible` (ui-rt's paint phase draws the ring from the
/// fact itself), `at` when it took focus.
pub(crate) fn facts(kept: &FocusKept, focused: bool, visible: bool) -> crate::runtime::focus::Facts {
    crate::runtime::focus::Facts { focused, visible: focused && visible, at: kept.seq, ..Default::default() }
}

/// A stop found by a walk.
#[derive(Clone, Copy, Debug)]
struct StopAt {
    node: NodeId,
    /// The roving group it belongs to: the nearest group owner above it.
    group: Option<NodeId>,
    /// The innermost scope around it, an index into [`Walk::scopes`].
    scope: Option<usize>,
    seq: u64,
}

/// A scope found by a walk.
#[derive(Clone, Copy, Debug)]
struct ScopeAt {
    node: NodeId,
    policy: FocusPolicy,
    parent: Option<usize>,
    /// It is a layer child (a popup, a menu, a dialog, a toast stack).
    layer: bool,
}

/// The stops, scopes and groups under one node, in tree order (layers bottom first).
#[derive(Default)]
struct Walk {
    stops: Vec<StopAt>,
    scopes: Vec<ScopeAt>,
    groups: Vec<(NodeId, Roving)>,
}

/// The kept facts of `node`, whichever contract element it is. `own` stands in for the node whose
/// hook is running, whose `Local` ui-rt cannot give.
fn kept_of<'a>(rt: &'a Runtime, node: NodeId, own: Option<(NodeId, &'a Kept)>) -> Option<(&'a Kept, bool)> {
    if let Some((id, kept)) = own
        && id == node
    {
        return Some((kept, false));
    }
    if let Some(kept) = rt.local::<ComponentNode>(node) {
        return Some((kept, false));
    }
    if let Some(kept) = rt.local::<ScopeNode>(node) {
        return Some((kept, false));
    }
    rt.local::<LayerScope>(node).map(|lk| (&lk.kept, true))
}

impl Walk {
    /// Walks `from` and everything under it.
    fn of(rt: &Runtime, from: NodeId, own: Option<(NodeId, &Kept)>, own_is_layer: bool) -> Walk {
        let mut walk = Walk::default();
        walk.visit(rt, from, own, own_is_layer, None, None);
        walk
    }

    fn visit(&mut self, rt: &Runtime, node: NodeId, own: Option<(NodeId, &Kept)>, own_is_layer: bool, group: Option<NodeId>, scope: Option<usize>) {
        let (mut group, mut scope) = (group, scope);
        if let Some((kept, layer)) = kept_of(rt, node, own) {
            let layer = layer || (own_is_layer && own.is_some_and(|(id, _)| id == node));
            let f = &kept.focus;
            if f.stop.is_some() {
                self.stops.push(StopAt { node, group, scope, seq: f.seq });
            }
            if f.scope != FocusPolicy::NONE {
                self.scopes.push(ScopeAt { node, policy: f.scope, parent: scope, layer });
                scope = Some(self.scopes.len() - 1);
            }
            if let Some(roving) = f.roving {
                self.groups.push((node, roving));
                group = Some(node);
            }
        }
        for &child in rt.children(node) {
            self.visit(rt, child, own, own_is_layer, group, scope);
        }
    }

    /// Whether scope `inner` is `outer` or lies inside it.
    fn within(&self, inner: Option<usize>, outer: usize) -> bool {
        let mut at = inner;
        while let Some(i) = at {
            if i == outer {
                return true;
            }
            at = self.scopes[i].parent;
        }
        false
    }

    /// Whether a trapping scope other than `except` lies around `scope`.
    fn trapped(&self, scope: Option<usize>, except: Option<usize>) -> bool {
        let mut at = scope;
        while let Some(i) = at {
            if Some(i) != except && self.scopes[i].policy.trap {
                return true;
            }
            at = self.scopes[i].parent;
        }
        false
    }

    /// The topmost trapping layer child.
    fn top_trap(&self) -> Option<usize> {
        self.scopes.iter().rposition(|s| s.layer && s.policy.trap)
    }

    /// The scope index whose node is `node`.
    fn scope_of(&self, node: NodeId) -> Option<usize> {
        self.scopes.iter().position(|s| s.node == node)
    }

    /// The Tab order of the stops `keep` admits: a roving group is one entry, its member with focus,
    /// else the one that had focus last, else its first.
    fn order(&self, focused: Option<NodeId>, keep: impl Fn(&StopAt) -> bool) -> Vec<NodeId> {
        let admitted: Vec<&StopAt> = self.stops.iter().filter(|s| keep(s)).collect();
        let mut out = Vec::with_capacity(admitted.len());
        let mut seen: Vec<NodeId> = Vec::new();
        for s in &admitted {
            let Some(g) = s.group else {
                out.push(s.node);
                continue;
            };
            if seen.contains(&g) {
                continue;
            }
            seen.push(g);
            let members = admitted.iter().filter(|m| m.group == Some(g));
            let lead = members.clone().find(|m| Some(m.node) == focused).or_else(|| members.clone().filter(|m| m.seq > 0).max_by_key(|m| m.seq));
            out.push(lead.map_or(s.node, |m| m.node));
        }
        out
    }
}

/// The entry after (or before) `focused` in `order`, wrapping; the first (or last) when focus is
/// not in it.
fn next_in(order: &[NodeId], focused: Option<NodeId>, forward: bool) -> Option<NodeId> {
    if order.is_empty() {
        return None;
    }
    let len = order.len();
    let at = focused.and_then(|f| order.iter().position(|&n| n == f));
    let i = match at {
        Some(i) if forward => (i + 1) % len,
        Some(i) => (i + len - 1) % len,
        None if forward => 0,
        None => len - 1,
    };
    Some(order[i])
}

fn plain_tab(k: &KeyInput) -> bool {
    k.down && k.key == Key::Tab && !(k.mods.cmd || k.mods.ctrl || k.mods.alt)
}

/// The window root's key stage, the last claimant on every key's path: Tab and Shift+Tab move
/// along the window's order, F6 and Shift+F6 between panes. Escape and the other keys go on (an
/// unclaimed Escape dismisses the topmost layer child that asks, in ui-rt).
pub(crate) fn window_keys(root: &Kept, cx: &mut EventCx<'_>, k: &KeyInput) -> Handled {
    if plain_tab(k) {
        return window_tab(root, cx, !k.mods.shift);
    }
    if k.down && k.key == Key::F6 && !(k.mods.cmd || k.mods.ctrl || k.mods.alt) {
        return pane_step(root, cx, !k.mods.shift);
    }
    Handled::No
}

fn window_walk(root: &Kept, cx: &EventCx<'_>) -> Option<Walk> {
    let rt = cx.runtime();
    let top = rt.window_node(cx.window())?;
    Some(Walk::of(rt, top, Some((cx.node(), root)), false))
}

/// Tab at the window root: into the topmost trapping layer child when focus is outside it, else
/// along the stops no trapping scope holds.
fn window_tab(root: &Kept, cx: &mut EventCx<'_>, forward: bool) -> Handled {
    let Some(walk) = window_walk(root, cx) else { return Handled::No };
    let focused = cx.focused();
    let order = match walk.top_trap() {
        Some(trap) if !walk.stops.iter().any(|s| Some(s.node) == focused && walk.within(s.scope, trap)) => {
            walk.order(focused, |s| walk.within(s.scope, trap) && !walk.trapped(s.scope, Some(trap)))
        }
        _ => walk.order(focused, |s| !walk.trapped(s.scope, None)),
    };
    if let Some(to) = next_in(&order, focused, forward) {
        cx.focus_node(to);
    }
    Handled::Yes
}

/// Tab inside a trapping scope whose node is handling the key (`own`): wraps among the scope's
/// stops, leaving out those of trapping scopes inside it. A trap with no stop keeps Tab.
pub(crate) fn trap_tab(own: &Kept, is_layer: bool, cx: &mut EventCx<'_>, k: &KeyInput) -> Handled {
    if !plain_tab(k) || !own.focus.traps() {
        return Handled::No;
    }
    let node = cx.node();
    let walk = Walk::of(cx.runtime(), node, Some((node, own)), is_layer);
    let Some(me) = walk.scope_of(node) else { return Handled::No };
    let focused = cx.focused();
    let order = walk.order(focused, |s| walk.within(s.scope, me) && !walk.trapped(s.scope, Some(me)));
    if let Some(to) = next_in(&order, focused, !k.mods.shift) {
        cx.focus_node(to);
    }
    Handled::Yes
}

/// F6: the next pane after the one focus is in, at its stop that had focus last (else its first).
fn pane_step(root: &Kept, cx: &mut EventCx<'_>, forward: bool) -> Handled {
    let Some(walk) = window_walk(root, cx) else { return Handled::No };
    let panes: Vec<usize> =
        (0..walk.scopes.len()).filter(|&i| walk.scopes[i].policy.kind == ScopeKind::Pane && !walk.trapped(Some(i), None)).collect();
    if panes.is_empty() {
        return Handled::No;
    }
    let focused = cx.focused();
    let here = walk.stops.iter().find(|s| Some(s.node) == focused).and_then(|s| panes.iter().rposition(|&p| walk.within(s.scope, p)));
    let len = panes.len();
    for step in 1..=len {
        let i = match here {
            Some(h) if forward => (h + step) % len,
            Some(h) => (h + len * step - step) % len,
            None if forward => step - 1,
            None => len - step,
        };
        let pane = panes[i];
        let stops: Vec<&StopAt> = walk.stops.iter().filter(|s| walk.within(s.scope, pane) && !walk.trapped(s.scope, None)).collect();
        let target = stops.iter().filter(|s| s.seq > 0).max_by_key(|s| s.seq).or(stops.first());
        if let Some(t) = target {
            cx.focus_node(t.node);
            return Handled::Yes;
        }
    }
    Handled::Yes
}

/// The members of a group, in tree order, as `NavOutline` positions.
struct Members {
    rects: Vec<Option<Rect>>,
}

impl NavOutline for Members {
    fn len(&self) -> usize {
        self.rects.len()
    }
    fn depth(&self, _: usize) -> usize {
        0
    }
    fn has_children(&self, _: usize) -> bool {
        false
    }
    fn open(&self, _: usize) -> bool {
        false
    }
    fn is_section(&self, _: usize) -> bool {
        false
    }
    fn enabled(&self, _: usize) -> bool {
        true
    }
    fn rect(&self, at: usize) -> Option<Rect> {
        self.rects.get(at).copied().flatten()
    }
}

/// A key a member of the group the handling node owns (`own`) did not take: the arrows, Home and
/// End and the Page keys the group's `Roving` takes move focus among its members.
pub(crate) fn rove(own: &Kept, cx: &mut EventCx<'_>, k: &KeyInput) -> Handled {
    let Some(roving) = own.focus.roving else { return Handled::No };
    if !k.down {
        return Handled::No;
    }
    let node = cx.node();
    let Some(Intent::Move(mv, mods)) = intent(cx.runtime().os(), k.key, k.mods, KeyRole::new()) else { return Handled::No };
    if !roving.takes(mv, mods) {
        return Handled::No;
    }
    let walk = Walk::of(cx.runtime(), node, Some((node, own)), false);
    let members: Vec<NodeId> = walk.stops.iter().filter(|s| s.group == Some(node)).map(|s| s.node).collect();
    let Some(at) = members.iter().position(|&m| m == cx.target()) else { return Handled::No };
    let outline = Members { rects: members.iter().map(|&m| cx.window_rect(m)).collect() };
    let mut state = roving.state(Some(at));
    match craft_behavior::nav::step(roving.shape, &outline, &mut state, mv, mods) {
        NavStep::Moved(to) => {
            cx.focus_node(members[to]);
            Handled::Yes
        }
        NavStep::Stay if !roving.hands_back => Handled::Yes,
        _ => Handled::No,
    }
}

/// A request for focus on `node` (an assistive technology's Focus): refused outside the topmost
/// trapping layer child; a stop takes it, a scope or a group gives it to its first stop (a group's
/// lead). `No` when nothing under the node takes focus, so ui-rt tries the stops above it.
pub(crate) fn request(own: &Kept, cx: &mut EventCx<'_>) -> Handled {
    let node = cx.node();
    let rt = cx.runtime();
    let Some(top) = rt.window_node(cx.window()) else { return Handled::No };
    let window = Walk::of(rt, top, Some((node, own)), false);
    if let Some(trap) = window.top_trap() {
        let trap_node = window.scopes[trap].node;
        let mut at = Some(node);
        let mut inside = false;
        while let Some(n) = at {
            if n == trap_node {
                inside = true;
                break;
            }
            at = rt.parent(n);
        }
        if !inside {
            return Handled::Yes;
        }
    }
    if own.focus.stop.is_some() {
        cx.focus_node(node);
        return Handled::Yes;
    }
    let walk = Walk::of(rt, node, Some((node, own)), false);
    match walk.order(None, |_| true).first() {
        Some(&to) => {
            cx.focus_node(to);
            Handled::Yes
        }
        None => Handled::No,
    }
}

/// A layer child whose run just made it appear (`cx` builds it; `own` is its kept facts): notes
/// where focus returns when it goes with focus inside, and moves focus in as its scope's `Initial`
/// says once the build pass has made its content. `Default` and `Key` take the first stop until
/// stops carry names.
pub(crate) fn enter_layer(own: &Kept, cx: &mut BuildCx<'_>) {
    let policy = own.focus.scope;
    if policy.restore {
        let opener = cx.runtime().focused(cx.window());
        cx.set_focus_return(opener);
    }
    match policy.initial {
        Initial::Keep | Initial::KeepAnchor => {}
        Initial::FirstField => cx.focus_into(true),
        _ => cx.focus_into(false),
    }
}
