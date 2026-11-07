//! The focus service: stops, scopes and roving groups registered per pass, the focused stop per arena root,
//! focus on press, focus after removal and programmatic focus (spec 8.3, 9.4, 9.5).
//!
//! egui's focus sits on an arena root. The focused member or part is runtime data keyed by that root, and
//! every focus read of component code goes through [`is_focused`].

mod nav;
mod policy;
mod press;
pub(crate) mod registration;

pub(crate) use nav::{enter_row, leave_row, roving_move, row_has_stops};
pub(crate) use registration::{Group, Stop};

use crate::runtime::egui::focus as egui_focus;
use crate::runtime::frame::PerViewport;
use craft_behavior::{DocKey, PartRef};
use egui::{Context, Id, ViewportId};
use std::collections::HashMap;
use std::sync::Arc;

pub use crate::runtime::keys::Intents;
pub use craft_behavior::{Letters, Roving};
pub use policy::{ActivateKeys, EditSet, EnterRule, FocusPolicy, FocusTarget, Initial, KeepKeys, KeyUse};

/// The shape the focus ring takes around a part.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FocusShape {
    /// The part's rect.
    Rect,
    /// The circle inside the part's rect.
    Circle,
}

/// A focus stop: a component, or one of its parts.
pub(crate) type StopKey = (Id, Option<PartRef>);

/// A registered stop with its place in the scope tree.
#[derive(Clone, Debug)]
pub(crate) struct StopRec {
    pub(crate) stop: Stop,
    pub(crate) scope: Option<Id>,
}

#[derive(Clone, Debug)]
pub(crate) struct ScopeRec {
    pub(crate) policy: FocusPolicy,
    pub(crate) parent: Option<Id>,
    pub(crate) label: Option<Arc<str>>,
    /// The stop that held focus when the scope was first pushed, which `restore` returns to.
    pub(crate) opener: Option<StopKey>,
}

/// What the end hook recorded for the next input hook, which reads only `RawInput` and records.
#[derive(Clone, Debug, Default)]
pub(crate) struct FocusedRecord {
    pub(crate) keys: Option<KeyUse>,
    pub(crate) claims: Vec<craft_keymap::Chord>,
    /// The component of the focused stop: only its state-dependent claims hold.
    pub(crate) focused: Option<Id>,
    /// The focused stop switches pages with Ctrl+Tab.
    pub(crate) pages: bool,
    pub(crate) menu_bar: bool,
}

/// A change to egui's focus, applied after the state lock is released.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Act {
    Focus(Id, egui_focus::KeyFilter),
    Surrender(Id),
}

/// What the begin hook reads from egui before it takes the state lock.
pub(crate) struct BeginIn {
    pub(crate) focused: Option<Id>,
    pub(crate) press: Option<press::PressAt>,
    pub(crate) nav: crate::runtime::keys::NavKeys,
    pub(crate) tab_all_controls: bool,
}

#[derive(Default)]
pub(crate) struct State {
    pub(crate) stops: Vec<StopRec>,
    pub(crate) last_stops: Vec<StopRec>,
    pub(crate) scopes: HashMap<Id, ScopeRec>,
    pub(crate) last_scopes: HashMap<Id, ScopeRec>,
    pub(crate) scope_order: Vec<Id>,
    pub(crate) last_scope_order: Vec<Id>,
    pub(super) open: Vec<(Id, usize)>,
    pub(crate) groups: HashMap<Id, Group>,
    pub(crate) last_groups: HashMap<Id, Group>,
    pub(crate) focused: HashMap<Id, StopKey>,
    pub(crate) focus_now: Option<Id>,
    pub(super) pending: Option<FocusTarget>,
    pub(super) docs: HashMap<DocKey, Id>,
    pub(super) revealed: Option<StopKey>,
    pub(super) seen: HashMap<Id, u64>,
    pub(crate) visible: bool,
    pub(crate) record: FocusedRecord,
    pub(super) menu_bar: bool,
    pub(crate) press: press::PressState,
}

fn viewport(ctx: &Context) -> ViewportId {
    crate::runtime::egui::pass::current_viewport(ctx)
}

pub(crate) fn with<R>(ctx: &Context, f: impl FnOnce(&mut State) -> R) -> R {
    PerViewport::<State>::with(ctx, viewport(ctx), f)
}

pub(crate) fn with_viewport<R>(ctx: &Context, viewport: ViewportId, f: impl FnOnce(&mut State) -> R) -> R {
    PerViewport::<State>::with(ctx, viewport, f)
}

pub(crate) fn apply(ctx: &Context, acts: Vec<Act>) {
    for act in acts {
        match act {
            Act::Focus(id, filter) => egui_focus::focus_with_filter(ctx, id, filter),
            Act::Surrender(id) => egui_focus::surrender(ctx, id),
        }
    }
}

/// Registers a stop of the running pass in the innermost open scope, in registration order.
pub(crate) fn register_stop(ctx: &Context, stop: Stop) {
    with(ctx, |state| {
        let scope = state.open.last().map(|(id, _)| *id);
        state.stops.push(StopRec { stop, scope });
    });
}

/// Registers a roving group of the running pass.
pub(crate) fn register_group(ctx: &Context, group: Group) {
    with(ctx, |state| {
        state.groups.insert(group.id, group);
    });
}

/// The place of the component `id`'s own stop among last pass's members of the roving group `group`.
pub(crate) fn group_position(ctx: &Context, id: Id, group: Id) -> Option<crate::runtime::access::SetPosition> {
    with(ctx, |state| {
        let members = || state.last_stops.iter().filter(|rec| rec.stop.group == Some(group) && rec.stop.part.is_none());
        let index = members().position(|rec| rec.stop.id == id)?;
        Some(crate::runtime::access::SetPosition::new(index).of(members().count()))
    })
}

/// Opens the scope `id` for the stops registered until the matching [`pop_scope`].
pub(crate) fn push_scope(ctx: &Context, id: Id, policy: FocusPolicy, label: Option<Arc<str>>) {
    push_scope_from(ctx, id, policy, label, None);
}

/// `push_scope` for a scope whose opener the caller noted before egui's modal gate took focus away from it.
pub(crate) fn push_scope_from(ctx: &Context, id: Id, policy: FocusPolicy, label: Option<Arc<str>>, noted: Option<StopKey>) {
    let focused = egui_focus::focused(ctx);
    with(ctx, |state| {
        let parent = state.open.last().map(|(parent, _)| *parent);
        let opener = noted.or_else(|| focused.and_then(|root| state.focused.get(&root).copied().or(Some((root, None)))));
        let opener = state.last_scopes.get(&id).map_or(opener, |last| last.opener.or(opener));
        state.scopes.insert(id, ScopeRec { policy, parent, label, opener });
        state.scope_order.push(id);
        let first = state.stops.len();
        state.open.push((id, first));
    });
}

/// Closes the innermost open scope. A scope pushed for the first time (or again after a gap of a frame)
/// moves focus to its initial stop, in this pass.
pub(crate) fn pop_scope(ctx: &Context) {
    let frame = crate::runtime::egui::pass::frame_nr(ctx, viewport(ctx));
    let acts = with(ctx, |state| nav::initial_focus(state, frame));
    apply(ctx, acts);
}

/// Registers that a menu bar can take the menu-bar keys of the next input hook (spec 9.7).
#[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "the component pipeline (F3) registers and reads it"))]
pub(crate) fn register_menu_bar(ctx: &Context) {
    with(ctx, |state| state.menu_bar = true);
}

/// Registers the component that `FocusTarget::Doc(key)` focuses.
#[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "the component pipeline (F3) registers and reads it"))]
pub(crate) fn register_doc(ctx: &Context, key: DocKey, id: Id) {
    with(ctx, |state| {
        state.docs.insert(key, id);
    });
}

/// Moves focus to `target` at the next pass's begin hook, leaves modality unchanged and asks the enclosing
/// scroll owner to reveal it. The one public path to programmatic focus.
pub fn request(ctx: &Context, target: FocusTarget) {
    with(ctx, |state| state.pending = Some(target));
    ctx.request_repaint();
}

/// Applies `target` now, for an AccessKit `Focus` action, which moves focus in the pass that carries it.
pub(crate) fn request_now(ctx: &Context, target: FocusTarget) {
    let acts = with(ctx, |state| state.apply_target(target));
    apply(ctx, acts);
}

/// The stop focus is on, in the viewport whose pass is running: the focused member or part of the egui
/// root, else the root itself.
pub(crate) fn focused_stop(ctx: &Context) -> Option<StopKey> {
    let root = egui_focus::focused(ctx)?;
    with(ctx, |state| Some(state.focused.get(&root).copied().unwrap_or((root, None))))
}

/// The registration of the stop `key`.
pub(crate) fn stop_rec(ctx: &Context, key: StopKey) -> Option<StopRec> {
    with(ctx, |state| state.find_stop(key))
}

/// The registration of the stop focus is on.
pub(crate) fn focused_rec(ctx: &Context) -> Option<StopRec> {
    let focused = egui_focus::focused(ctx);
    with(ctx, |state| {
        state.focus_now = focused;
        state.stop_of_focus()
    })
}

/// Whether the focus stop is a child of a collection row's roving group.
pub(crate) fn in_row_child(ctx: &Context) -> bool {
    with(ctx, |state| {
        let group = state.stop_of_focus().and_then(|rec| rec.stop.group);
        group.and_then(|group| state.last_groups.get(&group)).is_some_and(|group| group.row)
    })
}

/// Whether the focus stop is the component `id`: the focused member or item of its arena, or a root that
/// holds focus while none of its members is the stop. A root's own focused part counts as the root.
pub(crate) fn is_focused(ctx: &Context, id: Id) -> bool {
    focused_stop(ctx).is_some_and(|(focused, _)| focused == id)
}

/// `is_focused` and focus arrived by keyboard, so the ring shows.
pub(crate) fn focus_visible(ctx: &Context, id: Id) -> bool {
    is_focused(ctx, id) && with(ctx, |state| state.visible)
}

/// The part of `id` that is the focus stop, when one is.
pub(crate) fn active_part(ctx: &Context, id: Id) -> Option<PartRef> {
    focused_stop(ctx).filter(|(focused, _)| *focused == id).and_then(|(_, part)| part)
}

/// The text part that commits its draft in this pass because a press moved focus away from it (8.3).
#[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "the component pipeline (F3) registers and reads it"))]
pub(crate) fn commit_now(ctx: &Context) -> Option<Id> {
    with(ctx, |state| state.press.commit_now)
}

/// Sets modality: keyboard use shows focus rings, a pointer press hides them.
pub(crate) fn set_visible(ctx: &Context, viewport: ViewportId, visible: bool) {
    with_viewport(ctx, viewport, |state| state.visible = visible);
}

/// What the input hook may read: the viewport's focus record from its last end hook.
pub(crate) fn record(ctx: &Context, viewport: ViewportId) -> FocusedRecord {
    with_viewport(ctx, viewport, |state| state.record.clone())
}

/// The component the last `request` or Tab move asked the enclosing scroll owner to reveal.
#[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "the component pipeline (F3) registers and reads it"))]
pub(crate) fn take_reveal(ctx: &Context) -> Option<StopKey> {
    with(ctx, |state| state.revealed.take())
}

/// The begin hook: applies a pending request and this pass's Tab and F6 moves, then classifies the press.
pub(crate) fn begin(ctx: &Context, tab_all_controls: bool) {
    let input =
        BeginIn { focused: egui_focus::focused(ctx), press: press::press_at(ctx), nav: crate::runtime::keys::take_nav(ctx), tab_all_controls };
    egui_focus::clear_directional_focus(ctx);
    let (acts, announcements) = with(ctx, |state| nav::begin(state, input));
    apply(ctx, acts);
    for text in announcements {
        crate::runtime::frame::announce(ctx, text);
    }
}

/// The end hook: applies the press's focus move, returns focus to the opener of a scope that went away,
/// moves it off a stop that was not registered, and records what the next input hook reads.
pub(crate) fn end(ctx: &Context) {
    let focused = egui_focus::focused(ctx);
    let acts = with(ctx, |state| nav::end(state, focused));
    apply(ctx, acts);
}

/// Whether the focused stop sits inside `scope` or a scope opened from inside it; `None` when nothing is focused.
pub(crate) fn focus_in_scope(ctx: &Context, scope: Id) -> Option<bool> {
    let rec = focused_rec(ctx)?;
    Some(with(ctx, |state| {
        let mut at = rec.scope;
        while let Some(id) = at {
            if id == scope {
                return true;
            }
            at = state.last_scopes.get(&id).and_then(|record| record.parent);
        }
        false
    }))
}
