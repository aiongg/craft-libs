//! Focus movement: Tab and F6, roving groups, initial focus, restore, programmatic requests and focus after
//! removal. Every movement reads last pass's registrations and returns the egui changes to apply.

use super::{Act, BeginIn, FocusTarget, Initial, State, StopKey, StopRec};
use crate::runtime::egui::focus as egui_focus;
use craft_behavior::focus::after_removal;
use craft_behavior::nav::step;
use craft_behavior::{MoveMods, NavMove, NavOutline, NavStep, ScopeKind};
use egui::{Context, Id};
use std::collections::HashSet;

/// One Tab stop: a component or part, or a roving group entered at its arrival member.
enum Entry {
    Stop(StopRec),
    Group(Id, StopRec),
}

impl Entry {
    fn arrival(&self) -> &StopRec {
        match self {
            Entry::Stop(rec) | Entry::Group(_, rec) => rec,
        }
    }

    fn holds(&self, current: &StopRec) -> bool {
        match self {
            Entry::Stop(rec) => rec.stop.key() == current.stop.key(),
            Entry::Group(group, _) => current.stop.group == Some(*group),
        }
    }
}

/// A roving group's members as a flat outline of enabled positions.
struct Members(usize);

impl NavOutline for Members {
    fn len(&self) -> usize {
        self.0
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
}

impl State {
    pub(super) fn rotate(&mut self) {
        self.last_stops = std::mem::take(&mut self.stops);
        self.last_scopes = std::mem::take(&mut self.scopes);
        self.last_scope_order = std::mem::take(&mut self.scope_order);
        self.last_groups = std::mem::take(&mut self.groups);
        self.open.clear();
    }

    pub(super) fn find_stop(&self, key: StopKey) -> Option<StopRec> {
        self.stops.iter().chain(&self.last_stops).find(|rec| rec.stop.key() == key).cloned()
    }

    /// The stop focus is on: the recorded member or part of the focused root, else the root's own stop.
    pub(super) fn stop_of_focus(&self) -> Option<StopRec> {
        let root = self.focus_now?;
        let recorded = self.focused.get(&root).and_then(|key| self.find_stop(*key));
        recorded
            .or_else(|| self.find_stop((root, None)))
            .or_else(|| self.stops.iter().chain(&self.last_stops).find(|rec| rec.stop.root == root).cloned())
    }

    /// Moves focus to `rec`: the egui widget is its root, and the stop is recorded under that root.
    pub(super) fn focus_stop(&mut self, rec: StopRec) -> Vec<Act> {
        let stop = &rec.stop;
        let roving = stop.group.is_some();
        let keys = stop.keys;
        let escape = keys.claims.iter().any(|chord| chord.key == craft_keymap::Key::Escape);
        let filter = egui_focus::filter(
            keys.keep == super::KeepKeys::Tab,
            roving || matches!(keys.keep, super::KeepKeys::InlineArrows | super::KeepKeys::BlockArrows),
            roving || keys.keep == super::KeepKeys::BlockArrows,
            escape,
        );
        self.focused.insert(stop.root, stop.key());
        self.focus_now = Some(stop.root);
        self.revealed = Some(stop.key());
        vec![Act::Focus(stop.root, filter)]
    }

    fn in_scope(&self, scope: Option<Id>, root: Option<Id>) -> bool {
        let Some(root) = root else { return true };
        let mut at = scope;
        while let Some(id) = at {
            if id == root {
                return true;
            }
            at = self.last_scopes.get(&id).and_then(|rec| rec.parent);
        }
        false
    }

    /// The arrival member of a roving group: its selected member, else the first.
    fn arrival(&self, group: &super::Group) -> Option<StopRec> {
        let members = || self.last_stops.iter().filter(|rec| rec.stop.group == Some(group.id));
        group.selected.and_then(|id| members().find(|rec| rec.stop.id == id)).or_else(|| members().next()).cloned()
    }

    fn entries(&self, scope_root: Option<Id>, all_controls: bool) -> Vec<Entry> {
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for rec in self.last_stops.iter().filter(|rec| self.in_scope(rec.scope, scope_root)) {
            match rec.stop.group.and_then(|group| self.last_groups.get(&group)) {
                Some(group) if group.row => {}
                Some(group) => {
                    if let Some(arrival) = seen.insert(group.id).then(|| self.arrival(group)).flatten() {
                        out.push(Entry::Group(group.id, arrival));
                    }
                }
                None if all_controls || rec.stop.text => out.push(Entry::Stop(rec.clone())),
                None => {}
            }
        }
        out
    }

    /// The innermost trapping scope above `rec`, else the last trapping scope shown.
    fn trap_of(&self, rec: Option<&StopRec>) -> Option<Id> {
        let mut at = rec.and_then(|rec| rec.scope);
        while let Some(id) = at {
            let scope = self.last_scopes.get(&id)?;
            if scope.policy.trap {
                return Some(id);
            }
            at = scope.parent;
        }
        if rec.is_some() {
            return None;
        }
        self.last_scope_order.iter().rev().find(|id| self.last_scopes.get(id).is_some_and(|scope| scope.policy.trap)).copied()
    }

    pub(super) fn tab(&mut self, forward: bool, all_controls: bool) -> Vec<Act> {
        let current = self.stop_of_focus();
        let entries = self.entries(self.trap_of(current.as_ref()), all_controls);
        let Some(next) = cycle(&entries, current.as_ref(), forward) else { return Vec::new() };
        let arrival = entries[next].arrival().clone();
        self.focus_stop(arrival)
    }

    /// F6: the first stop of the next pane, with its label as the announcement.
    pub(super) fn pane(&mut self, forward: bool, all_controls: bool) -> (Vec<Act>, Vec<String>) {
        let panes: Vec<Id> = self
            .last_scope_order
            .iter()
            .copied()
            .filter(|id| self.last_scopes.get(id).is_some_and(|scope| scope.policy.kind == ScopeKind::Pane))
            .collect();
        let current = self.stop_of_focus();
        let here = current.as_ref().and_then(|rec| self.pane_of(rec));
        let at = here.and_then(|pane| panes.iter().position(|id| *id == pane));
        let next = match (at, forward) {
            (Some(at), true) => (at + 1) % panes.len(),
            (Some(at), false) => (at + panes.len() - 1) % panes.len(),
            (None, true) => 0,
            (None, false) => panes.len().saturating_sub(1),
        };
        let Some(pane) = panes.get(next).copied() else { return (Vec::new(), Vec::new()) };
        let Some(entry) = self.entries(Some(pane), all_controls).into_iter().next() else { return (Vec::new(), Vec::new()) };
        let label = self.last_scopes.get(&pane).and_then(|scope| scope.label.as_ref()).map(|label| label.to_string());
        (self.focus_stop(entry.arrival().clone()), label.into_iter().collect())
    }

    pub(super) fn pane_of(&self, rec: &StopRec) -> Option<Id> {
        let mut at = rec.scope;
        while let Some(id) = at {
            let scope = self.last_scopes.get(&id)?;
            if scope.policy.kind == ScopeKind::Pane {
                return Some(id);
            }
            at = scope.parent;
        }
        None
    }

    pub(super) fn apply_target(&mut self, target: FocusTarget) -> Vec<Act> {
        let key = match target {
            FocusTarget::Id(id) => (id, None),
            FocusTarget::Part(id, part) => (id, Some(part)),
            FocusTarget::Doc(doc) => match self.docs.get(&doc) {
                Some(id) => (*id, None),
                None => return Vec::new(),
            },
        };
        let found = self.find_stop(key).or_else(|| self.stops.iter().chain(&self.last_stops).find(|rec| rec.stop.id == key.0).cloned());
        match found {
            Some(rec) => self.focus_stop(rec),
            None => {
                self.revealed = Some(key);
                self.focus_now = Some(key.0);
                vec![Act::Focus(key.0, Default::default())]
            }
        }
    }

    /// A roving move of the group of `from`: focus moves to the member the navigation core picks.
    fn roving_move(&mut self, from: StopKey, mv: NavMove, mods: MoveMods, horizontal: bool) -> Option<Vec<Act>> {
        let current = self.find_stop(from)?;
        let group = *self.last_groups.get(&current.stop.group?)?;
        let mv = match (horizontal && group.rtl, mv) {
            (true, NavMove::Back) => NavMove::Forward,
            (true, NavMove::Forward) => NavMove::Back,
            (_, mv) => mv,
        };
        if !group.roving.takes(mv) {
            return None;
        }
        let members: Vec<StopRec> = self.last_stops.iter().filter(|rec| rec.stop.group == Some(group.id)).cloned().collect();
        let at = members.iter().position(|rec| rec.stop.key() == from);
        let mut state = group.roving.state(at);
        match step(group.roving.shape, &Members(members.len()), &mut state, mv, mods) {
            NavStep::Moved(to) => Some(self.focus_stop(members[to].clone())),
            _ => Some(Vec::new()),
        }
    }

    /// The stop that takes focus when the focused stop was not registered this pass.
    fn removed_focus(&self, focused: Option<Id>) -> Option<StopRec> {
        let root = focused?;
        let key = *self.focused.get(&root)?;
        if self.stops.iter().any(|rec| rec.stop.key() == key) {
            return None;
        }
        let gone = self.last_stops.iter().find(|rec| rec.stop.key() == key)?;
        let order: Vec<StopKey> = self.last_stops.iter().filter(|rec| rec.scope == gone.scope).map(|rec| rec.stop.key()).collect();
        let registered = |candidate: &StopKey| self.stops.iter().any(|rec| rec.stop.key() == *candidate);
        match after_removal(&order, &key, registered) {
            Some(next) => self.stops.iter().find(|rec| rec.stop.key() == *next).cloned(),
            None => self.stops.iter().find(|rec| rec.scope == gone.scope).cloned(),
        }
    }

    /// The opener of a scope that went away while focus was inside it.
    fn restored_focus(&self, focused: Option<Id>) -> Option<StopRec> {
        let root = focused?;
        let key = *self.focused.get(&root).unwrap_or(&(root, None));
        let inside = self.last_stops.iter().find(|rec| rec.stop.key() == key)?;
        let mut at = inside.scope;
        while let Some(id) = at {
            let scope = self.last_scopes.get(&id)?;
            if !self.scopes.contains_key(&id) && scope.policy.restore {
                return scope.opener.and_then(|opener| self.stops.iter().find(|rec| rec.stop.key() == opener).cloned());
            }
            at = scope.parent;
        }
        None
    }
}

fn cycle(entries: &[Entry], current: Option<&StopRec>, forward: bool) -> Option<usize> {
    if entries.is_empty() {
        return None;
    }
    let at = current.and_then(|rec| entries.iter().position(|entry| entry.holds(rec)));
    Some(match (at, forward) {
        (Some(at), true) => (at + 1) % entries.len(),
        (Some(at), false) => (at + entries.len() - 1) % entries.len(),
        (None, true) => 0,
        (None, false) => entries.len() - 1,
    })
}

/// Closes the innermost open scope; a scope shown for the first time moves focus to its initial stop.
pub(super) fn initial_focus(state: &mut State, frame: u64) -> Vec<Act> {
    let Some((scope, first)) = state.open.pop() else { return Vec::new() };
    state.seen.retain(|_, last| *last + 1 >= frame);
    let previous = state.seen.insert(scope, frame);
    if previous.is_some_and(|last| last + 1 >= frame) {
        return Vec::new();
    }
    let Some(policy) = state.scopes.get(&scope).map(|rec| rec.policy) else { return Vec::new() };
    let stops = &state.stops[first.min(state.stops.len())..];
    let pick = match policy.initial {
        Initial::First => stops.first(),
        Initial::Default => stops.iter().find(|rec| rec.stop.default).or_else(|| stops.first()),
        Initial::Key(name) => stops.iter().find(|rec| rec.stop.name == Some(name)),
        _ => None,
    };
    match pick.cloned() {
        Some(rec) => state.focus_stop(rec),
        None => Vec::new(),
    }
}

/// The begin hook's focus moves: a pending request, Tab and F6, then the press classification.
pub(super) fn begin(state: &mut State, input: BeginIn) -> (Vec<Act>, Vec<String>) {
    state.rotate();
    state.focus_now = input.focused;
    state.press.rotate(input.focused);
    let mut acts = Vec::new();
    let mut announcements = Vec::new();
    if let Some(target) = state.pending.take() {
        acts.extend(state.apply_target(target));
    }
    for forward in input.nav.tabs {
        acts.extend(state.tab(forward, input.tab_all_controls));
    }
    for forward in input.nav.panes {
        let (moved, said) = state.pane(forward, input.tab_all_controls);
        acts.extend(moved);
        announcements.extend(said);
    }
    if let Some(at) = &input.press {
        state.classify_press(at);
    }
    (acts, announcements)
}

/// The end hook's focus moves, and the record the next input hook reads.
pub(super) fn end(state: &mut State, focused: Option<Id>) -> Vec<Act> {
    let mut acts = state.apply_press(focused);
    if acts.is_empty() {
        let next = state.restored_focus(focused).or_else(|| state.removed_focus(focused));
        if let Some(rec) = next {
            acts = state.focus_stop(rec);
        }
    }
    let now = acts.last().map_or(focused, |act| match act {
        Act::Focus(id, _) => Some(*id),
        Act::Surrender(_) => None,
    });
    state.focus_now = now;
    state.focused.retain(|root, _| Some(*root) == now);
    let stop = state.stop_of_focus().map(|rec| rec.stop);
    let keys = stop.as_ref().map(|stop| stop.keys);
    state.record = super::FocusedRecord {
        claims: keys.map(|keys| keys.claims.to_vec()).unwrap_or_default(),
        focused: stop.as_ref().map(|stop| stop.id),
        pages: stop.as_ref().is_some_and(|stop| stop.role.pages),
        keys,
        menu_bar: std::mem::take(&mut state.menu_bar),
    };
    acts
}

/// Applies a roving move of the group that `from` belongs to. `None` when `from` is no group member or the
/// group does not take the move; the caller then routes the move to the component.
pub(crate) fn roving_move(ctx: &Context, from: StopKey, mv: NavMove, mods: MoveMods, horizontal: bool) -> Option<()> {
    let acts = super::with(ctx, |state| state.roving_move(from, mv, mods, horizontal))?;
    super::apply(ctx, acts);
    Some(())
}
