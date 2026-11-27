//! Step 7, parts: the parts a run declares, kept with the node as hit shapes, and the facts later
//! steps read of them.

use super::measure::{MeasureIn, measure_cx};
use crate::contract::cx::{Decls, PartExtras, PartKeys, PartsCx, Toggle, gesture_key};
use crate::contract::rt::{KeptPart, kept_part_id};
use crate::contract::{Component, Cx, ValueOf};
use craft_behavior::{Gesture, PartRef, Roving};
use craft_ui_rt::{HitShape, Pos2, Rect};

/// What step 7 gathers.
#[derive(Default)]
pub(crate) struct PartsStep {
    pub decls: Decls,
    pub rovings: Vec<Roving>,
    pub scrolls: Vec<crate::contract::cx::ScrollDecl>,
    pub text: crate::contract::text::TextStep,
}

/// Runs `Component::parts` over the run's rect and content rect.
/// `interact` is the node and its interaction state (`PartsCx::hovered`, `focus_visible`); `items`
/// what the shape laid out before (`ShapeHooks::geometry`); `extra` the shape's own parts after the
/// component's.
#[allow(clippy::too_many_arguments)]
pub(crate) fn declare_parts<C: Component>(
    comp: &mut C,
    cx: &mut Cx<'_, C>,
    rect: Rect,
    parts: &mut PartsStep,
    input: &MeasureIn<'_>,
    interact: Option<(craft_ui_rt::NodeId, craft_ui_rt::Interact)>,
    items: Option<std::rc::Rc<dyn std::any::Any>>,
    extra: &mut dyn FnMut(&mut C, Rect, &mut PartsCx<'_, C>),
) {
    let id = cx.core.id;
    let gesture_running = cx.core.memory.recall::<Gesture<ValueOf<C>>>(gesture_key(id)).is_running();
    let measure = measure_cx::<C>(input, cx.style, cx.core.resolved, cx.shown.as_deref());
    let mut pcx = PartsCx {
        content_rect: cx.core.content_rect,
        flow: cx.core.flow,
        decls: &mut parts.decls,
        rovings: &mut parts.rovings,
        scrolls: &mut parts.scrolls,
        text: &mut parts.text,
        interact,
        gesture_running,
        measure,
        items,
        stage: Some((&*cx.core.memory, id)),
    };
    comp.parts(rect, &mut pcx);
    extra(comp, rect, &mut pcx);
}

/// The declared parts as the node keeps them: hit shapes in the node's coordinates (`origin` is the
/// node's top left), with the ui-rt part id paint records them under.
pub(crate) fn kept_parts(parts: &PartsStep, origin: Pos2) -> Vec<KeptPart> {
    parts
        .decls
        .iter()
        .map(|(decl, extras)| {
            let part = PartRef { key: decl.key, index: decl.index };
            KeptPart {
                part,
                id: kept_part_id(part),
                shape: HitShape::Rect(decl.shape.bounds().translate(Pos2::ZERO - origin)),
                sense: decl.sense,
                cursor: decl.cursor.map(Into::into),
                exclusive: extras.exclusive,
            }
        })
        .collect()
}

/// The key and index of every declared part whose extras `has` accepts.
pub(crate) fn declared_with(parts: &PartsStep, has: impl Fn(&PartExtras) -> bool) -> PartKeys {
    parts.decls.iter().filter(|(_, extras)| has(extras)).map(|(decl, _)| (decl.key, decl.index)).collect()
}

/// The focus shapes the parts declared, by key and index.
pub(crate) fn focus_shapes(parts: &PartsStep) -> Vec<((craft_behavior::PartKey, Option<u64>), crate::runtime::focus::FocusShape)> {
    parts.decls.iter().filter_map(|(decl, extras)| extras.focus_shape.clone().map(|shape| ((decl.key, decl.index), shape))).collect()
}

/// The popup toggles the parts declared.
pub(crate) fn toggles(parts: &PartsStep) -> Vec<Toggle> {
    parts
        .decls
        .iter()
        .filter_map(|(decl, extras)| {
            extras.toggles.map(|toggle| Toggle { part: PartRef { key: decl.key, index: decl.index }, decl: toggle, open: false })
        })
        .collect()
}

/// Whether `part` was declared inactive (it paints its data dimmed and senses nothing).
pub(crate) fn inactive(parts: &PartsStep, part: PartRef) -> bool {
    parts.decls.iter().any(|(decl, _)| decl.key == part.key && decl.index == part.index && decl.inactive)
}
