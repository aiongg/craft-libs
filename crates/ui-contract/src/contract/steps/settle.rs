//! Step 15, settle.

use super::apply::{ValueEdit, apply_values};
use crate::contract::{Component, Cx, ValueOf};

/// Step 15, settle: child edits adopted with `cx.adopt` join the run's events and are applied. The
/// popups' open state stays on the node (`Kept::popups`). A reveal the run asked for stays in
/// `CxCore::reveal`, which the driver hands to the nearest scroll owner (item 6: it bubbles to the
/// nearest `ScrollNode`).
pub(crate) fn settle<C: Component>(comp: &mut C, cx: &mut Cx<'_, C>) -> ValueEdit {
    let adopted = std::mem::take(&mut cx.core.paint.borrow_mut().adopted);
    for event in adopted.into_iter().filter_map(|adopted| adopted.take::<ValueOf<C>>()) {
        if let Some(event) = cx.absorb(event) {
            cx.events.push(event);
        }
    }
    apply_values(comp, cx)
}
