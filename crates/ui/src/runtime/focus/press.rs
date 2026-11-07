//! Focus on press (spec 8.3): the press is classified at the begin hook from last pass's shapes, and the
//! classified focus move applies at the end hook.

use super::{Act, State, StopKey};
use crate::runtime::egui::focus as egui_focus;
use crate::runtime::pointer;
use craft_behavior::PartRef;
use craft_behavior::Winner;
use craft_behavior::focus::{PressFocus, classify_press};
use egui::{Context, Id};

/// What won a primary press at the begin hook.
pub(crate) struct PressAt {
    winner: Option<Winner<Id>>,
    /// The arena root the winner belongs to.
    root: Option<Id>,
}

pub(crate) fn press_at(ctx: &Context) -> Option<PressAt> {
    egui_focus::primary_press_origin(ctx).map(|pos| {
        let (winner, root) = pointer::press_winner(ctx, pos, pointer::pass_pointer(ctx).press_targets).unzip();
        PressAt { winner, root }
    })
}

#[derive(Clone, Debug)]
struct Plan {
    kind: PressFocus,
    owner: Option<Id>,
    /// The arena root of the owner: the widget egui focus falls back to when the owner has no stop.
    root: Option<Id>,
    part: Option<PartRef>,
    pane: Option<Id>,
    was_text: bool,
}

#[derive(Default)]
pub(crate) struct PressState {
    plan: Option<Plan>,
    pub(super) commit_now: Option<Id>,
    focus_at_begin: Option<Id>,
}

impl PressState {
    pub(super) fn rotate(&mut self, focused: Option<Id>) {
        self.plan = None;
        self.commit_now = None;
        self.focus_at_begin = focused;
    }
}

impl State {
    /// Classifies this pass's primary press by the shape that wins it on the top layer, and marks a focused
    /// text part to commit its draft when the press leaves it.
    pub(super) fn classify_press(&mut self, at: &PressAt) {
        let winner = at.winner.as_ref();
        let kind = classify_press(winner.map(|winner| winner.sense));
        let (owner, part) = winner.map_or((None, None), |winner| (Some(winner.owner), winner.part));
        let text_stop = self.stop_of_focus().filter(|stop| stop.stop.text).map(|stop| stop.stop.id);
        let was_text = text_stop.is_some() && kind.commits_draft();
        self.press.commit_now = text_stop.filter(|_| was_text);
        let pane = self.stop_of_focus().filter(|_| was_text).and_then(|rec| self.pane_of(&rec));
        self.press.plan = Some(Plan { kind, owner, root: at.root, part, pane, was_text });
    }

    /// The press's focus move at the end hook; `focused` is egui's focus now.
    pub(super) fn apply_press(&mut self, focused: Option<Id>) -> Vec<Act> {
        let Some(plan) = self.press.plan.take() else { return Vec::new() };
        match plan.kind {
            PressFocus::Take => self.take_focus(&plan),
            PressFocus::Release if plan.was_text => match plan.pane {
                Some(pane) => {
                    self.focused.remove(&focused.unwrap_or(pane));
                    vec![Act::Focus(pane, Default::default())]
                }
                None => focused.map(Act::Surrender).into_iter().collect(),
            },
            PressFocus::Unclaimed if focused == self.press.focus_at_begin && !self.in_trapping_scope() => focused.map(Act::Surrender).into_iter().collect(),
            _ => Vec::new(),
        }
    }

    /// Whether the focused stop sits in a scope that traps focus (a dialog or modal): an unclaimed press, such as one on
    /// its scrim, leaves focus inside it.
    fn in_trapping_scope(&self) -> bool {
        let mut at = self.stop_of_focus().and_then(|rec| rec.scope);
        while let Some(id) = at {
            let Some(scope) = self.scopes.get(&id).or_else(|| self.last_scopes.get(&id)) else { return false };
            if scope.policy.trap {
                return true;
            }
            at = scope.parent;
        }
        false
    }

    fn take_focus(&mut self, plan: &Plan) -> Vec<Act> {
        let Some(owner) = plan.owner else { return Vec::new() };
        let key: StopKey = (owner, plan.part);
        match self.find_stop(key).or_else(|| self.find_stop((owner, None))) {
            Some(stop) => self.focus_stop(stop),
            None => vec![Act::Focus(plan.root.unwrap_or(owner), Default::default())],
        }
    }
}
