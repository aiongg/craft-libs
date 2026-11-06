//! Focus on press (spec 8.3): the press is classified at the begin hook from last pass's shapes, and the
//! classified focus move applies at the end hook.

use super::{Act, State, StopKey};
use crate::runtime::egui::focus as egui_focus;
use craft_behavior::focus::{PressFocus, PressShape, classify_press};
use craft_behavior::PartRef;
use egui::{Context, Id, LayerId, Pos2, Rect};

/// Where a primary press began and the egui layer on top there.
pub(crate) struct PressAt {
    pos: Pos2,
    layer: Option<LayerId>,
}

pub(crate) fn press_at(ctx: &Context) -> Option<PressAt> {
    egui_focus::primary_press_origin(ctx).map(|pos| PressAt { pos, layer: egui_focus::layer_at(ctx, pos) })
}

/// A shape that senses a press, as the pointer service's shape record holds it.
// F2-MERGE: pointer (the shape record and its `resolve` belong to `runtime::pointer`).
#[doc(hidden)]
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct PressShapeRecord {
    /// The egui layer the shape is on.
    pub layer: LayerId,
    /// The shape's rect in screen points.
    pub rect: Rect,
    /// What the shape's sense says about focus.
    pub shape: PressShape,
    /// The component the shape belongs to.
    pub owner: Id,
    /// The part, when the shape is a focusable part.
    pub part: Option<PartRef>,
    /// The enclosing pane, which takes focus from a text part on a kind 3 press.
    pub pane: Option<Id>,
}

impl PressShapeRecord {
    /// A shape of `owner` on `layer` at `rect`.
    pub fn new(layer: LayerId, rect: Rect, shape: PressShape, owner: Id) -> Self {
        PressShapeRecord { layer, rect, shape, owner, part: None, pane: None }
    }

    /// Marks the shape as a focusable part.
    pub fn part(mut self, part: PartRef) -> Self {
        self.part = Some(part);
        self
    }

    /// Names the enclosing pane.
    pub fn pane(mut self, pane: Id) -> Self {
        self.pane = Some(pane);
        self
    }
}

/// Records a shape of the running pass; the next pass resolves its press over it.
#[doc(hidden)]
pub fn note_press_shape(ctx: &Context, record: PressShapeRecord) {
    super::with(ctx, |state| state.press.shapes.push(record));
}

#[derive(Clone, Debug)]
struct Plan {
    kind: PressFocus,
    owner: Option<Id>,
    part: Option<PartRef>,
    pane: Option<Id>,
    was_text: bool,
}

#[derive(Default)]
pub(crate) struct PressState {
    pub(super) shapes: Vec<PressShapeRecord>,
    last_shapes: Vec<PressShapeRecord>,
    plan: Option<Plan>,
    pub(super) commit_now: Option<Id>,
    focus_at_begin: Option<Id>,
}

impl PressState {
    pub(super) fn rotate(&mut self, focused: Option<Id>) {
        self.last_shapes = std::mem::take(&mut self.shapes);
        self.plan = None;
        self.commit_now = None;
        self.focus_at_begin = focused;
    }
}

impl State {
    /// Classifies this pass's primary press by the shape that wins it on the top layer, and marks a focused
    /// text part to commit its draft when the press leaves it.
    pub(super) fn classify_press(&mut self, at: &PressAt) {
        let winner = self.press.last_shapes.iter().rev().find(|shape| at.layer.is_none_or(|top| top == shape.layer) && shape.rect.contains(at.pos));
        let kind = classify_press(winner.map(|shape| shape.shape));
        let (owner, part, pane) = winner.map_or((None, None, None), |shape| (Some(shape.owner), shape.part, shape.pane));
        let text_stop = self.stop_of_focus().filter(|stop| stop.stop.text).map(|stop| stop.stop.id);
        let was_text = text_stop.is_some() && kind.commits_draft();
        self.press.commit_now = text_stop.filter(|_| was_text);
        self.press.plan = Some(Plan { kind, owner, part, pane, was_text });
    }

    /// The press's focus move at the end hook; `focused` is egui's focus now.
    pub(super) fn apply_press(&mut self, focused: Option<Id>) -> Vec<Act> {
        let Some(plan) = self.press.plan.take() else { return Vec::new() };
        match plan.kind {
            PressFocus::Take => self.take_focus(&plan),
            PressFocus::Release if plan.was_text => match plan.pane {
                Some(pane) => {
                    self.focused.remove(&focused.unwrap_or(pane));
                    vec![Act::Focus(pane, egui::EventFilter::default())]
                }
                None => focused.map(Act::Surrender).into_iter().collect(),
            },
            PressFocus::Unclaimed if focused == self.press.focus_at_begin => focused.map(Act::Surrender).into_iter().collect(),
            _ => Vec::new(),
        }
    }

    fn take_focus(&mut self, plan: &Plan) -> Vec<Act> {
        let Some(owner) = plan.owner else { return Vec::new() };
        let key: StopKey = (owner, plan.part);
        match self.find_stop(key).or_else(|| self.find_stop((owner, None))) {
            Some(stop) => self.focus_stop(stop),
            None => vec![Act::Focus(owner, egui::EventFilter::default())],
        }
    }
}
