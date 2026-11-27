//! Step 15, settle.

use super::apply::{ValueEdit, apply_values};
use crate::contract::{Component, Cx, ValueOf};

/// Step 15, settle: child edits adopted with `cx.adopt` join the run's events and are applied, and the
/// open state of the component's popups is written. A reveal the run asked for stays in
/// `CxCore::reveal`, which the driver hands to the nearest scroll owner (item 6: it bubbles to the
/// nearest `ScrollNode`).
pub(crate) fn settle<C: Component>(comp: &mut C, cx: &mut Cx<'_, C>) -> ValueEdit {
    let adopted = std::mem::take(&mut cx.core.paint.borrow_mut().adopted);
    for event in adopted.into_iter().filter_map(|adopted| adopted.take::<ValueOf<C>>()) {
        if let Some(event) = cx.absorb(event) {
            cx.events.push(event);
        }
    }
    let edit = apply_values(comp, cx);
    if C::OVERLAYS {
        let id = cx.core.id;
        crate::contract::overlay::write_open(&mut *cx.core.memory, id);
    }
    edit
}
