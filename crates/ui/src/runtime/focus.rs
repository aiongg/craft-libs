//! The focus service: stops, scopes and roving groups registered per pass, the focused stop per arena root,
//! focus on press, focus after removal and programmatic focus (spec 8.3, 9.4, 9.5).
//!
//! egui's focus sits on an arena root. The focused member or part is runtime data keyed by that root, and
//! every focus read of component code goes through [`is_focused`].

mod nav;
mod policy;
mod press;

pub(crate) use nav::roving_move;

use crate::runtime::egui::focus as egui_focus;
use crate::runtime::frame::PerViewport;
use craft_behavior::{DocKey, KeyRole, PartRef};
use egui::{Context, Id, ViewportId};
use std::collections::HashMap;
use std::sync::Arc;

pub use crate::runtime::keys::Intents;
pub use craft_behavior::{Letters, Roving};
pub use policy::{ActivateKeys, EditSet, EnterRule, FocusPolicy, FocusTarget, Initial, KeepKeys, KeyUse};

/// A focus stop: a component, or one of its parts.
pub(crate) type StopKey = (Id, Option<PartRef>);

/// A stop as registered at step 7 of the pipeline.
#[doc(hidden)]
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Stop {
    /// The component.
    pub id: Id,
    /// The focusable part, `None` for the component itself.
    pub part: Option<PartRef>,
    /// The egui widget that holds focus for the stop: the component, or the arena root of a member.
    pub root: Id,
    /// The component's declared keys.
    pub keys: KeyUse,
    /// What the table's rows differ by.
    pub role: KeyRole,
    /// The stop is a text or editor part, which holds a draft.
    pub text: bool,
    /// The stop is its scope's default (`Initial::Default`).
    pub default: bool,
    /// The name `Initial::Key` matches.
    pub name: Option<&'static str>,
    /// The roving group the stop belongs to.
    pub group: Option<Id>,
}

impl Stop {
    /// A stop of component `id` with `keys`, its own egui root.
    pub fn new(id: Id, keys: KeyUse) -> Stop {
        Stop { id, part: None, root: id, keys, role: KeyRole::new(), text: false, default: false, name: None, group: None }
    }

    /// A stop on a part of the component.
    pub fn part(mut self, part: PartRef) -> Self {
        self.part = Some(part);
        self
    }

    /// A stop whose focus the arena root `root` holds.
    pub fn root(mut self, root: Id) -> Self {
        self.root = root;
        self
    }

    /// The table role of the stop.
    pub fn role(mut self, role: KeyRole) -> Self {
        self.role = role;
        self
    }

    /// Marks the stop as a text part.
    pub fn text(mut self) -> Self {
        self.text = true;
        self
    }

    /// Marks the stop as its scope's default.
    pub fn default_stop(mut self) -> Self {
        self.default = true;
        self
    }

    /// Names the stop for `Initial::Key`.
    pub fn named(mut self, name: &'static str) -> Self {
        self.name = Some(name);
        self
    }

    /// Joins the roving group `group`.
    pub fn in_group(mut self, group: Id) -> Self {
        self.group = Some(group);
        self
    }

    pub(crate) fn key(&self) -> StopKey {
        (self.id, self.part)
    }
}

/// A roving group's registration: one Tab stop whose members take moves.
#[doc(hidden)]
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct Group {
    /// The group.
    pub id: Id,
    /// Its rules.
    pub roving: Roving,
    /// The member Tab arrives at, else the first.
    pub selected: Option<Id>,
    /// The group is a collection row's children: Tab leaves the collection and Forward enters the row.
    pub row: bool,
}

impl Group {
    /// A group of `roving` rules.
    pub fn new(id: Id, roving: Roving) -> Group {
        Group { id, roving, selected: None, row: false }
    }

    /// The member Tab arrives at.
    pub fn selected(mut self, member: Id) -> Self {
        self.selected = Some(member);
        self
    }

    /// Marks a row's children.
    pub fn row(mut self) -> Self {
        self.row = true;
        self
    }
}

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
    pub(crate) menu_bar: bool,
}

/// A change to egui's focus, applied after the state lock is released.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Act {
    Focus(Id, egui::EventFilter),
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
#[doc(hidden)]
pub fn register_stop(ctx: &Context, stop: Stop) {
    with(ctx, |state| {
        let scope = state.open.last().map(|(id, _)| *id);
        state.stops.push(StopRec { stop, scope });
    });
}

/// Registers a roving group of the running pass.
#[doc(hidden)]
pub fn register_group(ctx: &Context, group: Group) {
    with(ctx, |state| {
        state.groups.insert(group.id, group);
    });
}

/// Opens the scope `id` for the stops registered until the matching [`pop_scope`].
#[doc(hidden)]
pub fn push_scope(ctx: &Context, id: Id, policy: FocusPolicy, label: Option<Arc<str>>) {
    let focused = egui_focus::focused(ctx);
    with(ctx, |state| {
        let parent = state.open.last().map(|(parent, _)| *parent);
        let opener = focused.and_then(|root| state.focused.get(&root).copied().or(Some((root, None))));
        let opener = state.last_scopes.get(&id).map_or(opener, |last| last.opener.or(opener));
        state.scopes.insert(id, ScopeRec { policy, parent, label, opener });
        state.scope_order.push(id);
        let first = state.stops.len();
        state.open.push((id, first));
    });
}

/// Closes the innermost open scope. A scope pushed for the first time (or again after a gap of a frame)
/// moves focus to its initial stop, in this pass.
#[doc(hidden)]
pub fn pop_scope(ctx: &Context) {
    let frame = crate::runtime::egui::pass::frame_nr(ctx, viewport(ctx));
    let acts = with(ctx, |state| nav::initial_focus(state, frame));
    apply(ctx, acts);
}

/// Registers that a menu bar can take the menu-bar keys of the next input hook (spec 9.7).
#[doc(hidden)]
pub fn register_menu_bar(ctx: &Context) {
    with(ctx, |state| state.menu_bar = true);
}

/// Registers the component that `FocusTarget::Doc(key)` focuses.
#[doc(hidden)]
pub fn register_doc(ctx: &Context, key: DocKey, id: Id) {
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
#[doc(hidden)]
pub fn focused_stop(ctx: &Context) -> Option<StopKey> {
    let root = egui_focus::focused(ctx)?;
    with(ctx, |state| Some(state.focused.get(&root).copied().unwrap_or((root, None))))
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
#[doc(hidden)]
pub fn is_focused(ctx: &Context, id: Id) -> bool {
    focused_stop(ctx).is_some_and(|(focused, _)| focused == id)
}

/// `is_focused` and focus arrived by keyboard, so the ring shows.
#[doc(hidden)]
pub fn focus_visible(ctx: &Context, id: Id) -> bool {
    is_focused(ctx, id) && with(ctx, |state| state.visible)
}

/// The part of `id` that is the focus stop, when one is.
#[doc(hidden)]
pub fn active_part(ctx: &Context, id: Id) -> Option<PartRef> {
    focused_stop(ctx).filter(|(focused, _)| *focused == id).and_then(|(_, part)| part)
}

/// The text part that commits its draft in this pass because a press moved focus away from it (8.3).
#[doc(hidden)]
pub fn commit_now(ctx: &Context) -> Option<Id> {
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
#[doc(hidden)]
pub fn take_reveal(ctx: &Context) -> Option<StopKey> {
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
