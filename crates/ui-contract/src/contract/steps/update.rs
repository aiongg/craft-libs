//! Step 9, update: the library's part around `Component::update`.

use super::label::scrub_press;
use crate::contract::cx::CxCore;
use crate::contract::cx::gesture_key;
use crate::contract::{Component, Cx, Resolved, ValueOf};
use craft_behavior::{CancelReason, Gesture, GesturePhase, Input, Intent, PartRef, PressCancel};
use craft_ui_rt::Rect;

/// Whether the component takes activations (a click, a key, AccessKit `Click`): a disabled or read-only one drops them.
pub(crate) fn takes_activation(resolved: &Resolved<'_>) -> bool {
    resolved.enabled && !resolved.read_only
}

/// Whether the component takes focus: enabled, or disabled with a reason it shows on focus (its
/// tooltip's `disabled_text`, or its bound command's hint).
pub(crate) fn takes_focus(resolved: &Resolved<'_>) -> bool {
    resolved.enabled
        || resolved.tooltip.is_some_and(|tooltip| tooltip.spec().disabled_text.is_some())
        || resolved.command.and_then(|command| command.hint.as_deref()).is_some_and(|hint| !hint.is_empty())
}

/// Step 9, update, the part the library writes: the activation intent is the driver's (taken here so
/// it is not left for the scopes), and an AccessKit click on a part is a click on the component.
pub(crate) fn update_inputs(core: &mut CxCore<'_>) {
    if core.intents.take(Intent::Activate) && takes_activation(core.resolved) {
        core.activation.get_or_insert(crate::contract::Activation { by_key: true, mods: craft_keymap::Mods::NONE });
    }
    if core.intents.any_access_click() && takes_activation(core.resolved) {
        core.activation.get_or_insert(crate::contract::Activation { by_key: false, mods: craft_keymap::Mods::NONE });
        core.part_clicked = true;
    }
}

/// Step 9, before the component: a gesture cancelled under its press is live again once the press is released.
pub(crate) fn gesture_gate<C: Component>(comp: &C, cx: &mut Cx<'_, C>) {
    let held = cx.core.press_held() || scrub_press(comp, cx).is_some_and(|(press, _)| press.phase.button_down());
    let slot = gesture_key(cx.core.id);
    let gesture = cx.core.memory.shown::<Gesture<ValueOf<C>>>(slot);
    if gesture.dead && !held {
        gesture.dead = false;
    }
}

/// Step 9, the edit intents the library takes before `C::update`: Escape or a cancelled press ends a running gesture
/// (Escape as `C::ESCAPE` says), and a reset becomes a `Set` of the binding's reset value, unless the value is already
/// that. In a dialog Escape is the dialog's Cancel, which discards the gesture with the dialog's other edits: a gesture
/// there ends on a cancelled press only.
pub(crate) fn edit_intents<C: Component>(comp: &mut C, cx: &mut Cx<'_, C>) {
    use crate::contract::cx::Edit as _;
    if cx.gesture().is_some_and(|phase| matches!(phase, GesturePhase::Begin | GesturePhase::Update)) {
        // An Escape the field does not hold goes up its path to the scopes around it (a dialog's
        // Cancel); one it holds (`Cx::hold_escape`) ends the gesture here.
        let reason = if !cx.core.escape_to_list && cx.core.intents.take(Intent::Cancel) {
            Some(CancelReason::Escape)
        } else {
            cx.core.press_cancelled().map(|reason| match reason {
                PressCancel::Escape => CancelReason::Escape,
                PressCancel::Removed => CancelReason::Removed,
                PressCancel::Host => CancelReason::HostChanged,
                _ => CancelReason::Blur,
            })
        };
        match reason {
            Some(CancelReason::Escape) if C::ESCAPE == crate::contract::EscapeEdit::Commit => cx.commit(),
            Some(reason) => cx.cancel(reason),
            None => {}
        }
    }
    let reset = cx.core.intents.take_reset();
    let double = double_click(cx.core).filter(|_| !matches!(C::INPUT, Input::Text | Input::Editor));
    if reset.is_some() || double.is_some() {
        let part = match reset {
            Some(whole) => cx.core.active.filter(|_| !whole),
            None => double.flatten(),
        };
        if let Some(part) = part.filter(|part| cx.core.own_reset_parts.contains(&(part.key, part.index))) {
            cx.core.reset_part = Some(part);
        } else if let Some(reset) = comp.bind().and_then(|bind| bind.reset_value().cloned())
            && cx.value() != Some(&reset)
        {
            cx.set(reset);
        }
    }
}

/// A second press of a double click went down on the component's root (`Some(None)`) or one of its value parts
/// (`Some(Some(part))`) this pass (10.2).
fn double_click(core: &CxCore<'_>) -> Option<Option<PartRef>> {
    if core.press.double_pressed() {
        return Some(None);
    }
    core.value_parts
        .iter()
        .find(|key| core.results.get(key).is_some_and(|result| result.press.double_pressed()))
        .map(|&(key, index)| Some(PartRef { key, index }))
}

/// A context request a run recorded: where the context menu opens, and the part it is for.
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub(crate) struct ContextRequest(pub Option<(Rect, Option<PartRef>)>);

/// The memory slot of a node's context request.
pub(crate) fn context_slot(id: crate::contract::Id) -> crate::contract::Id {
    crate::runtime::memory::sub(id, "context")
}

/// Step 9, after `update`: a secondary click on the component or a `ContextMenu` intent it did not
/// take records a context request that the overlay step and the popups read in this run (9.2 rule
/// 3): at the part under the pointer for a click, at the active part for the intent, else at the
/// component.
pub(crate) fn record_context(core: &mut CxCore<'_>) {
    let part = if core.intents.untaken(&Intent::ContextMenu) {
        core.active
    } else if core.response.secondary_clicked() {
        core.results.iter().find(|(_, result)| result.hovered).map(|(&(key, index), _)| PartRef { key, index })
    } else {
        *core.memory.shown::<ContextRequest>(context_slot(core.id)) = ContextRequest(None);
        return;
    };
    let at = part.and_then(|part| core.results.get(&(part.key, part.index)).map(|result| result.rect)).unwrap_or(core.rect);
    *core.memory.shown::<ContextRequest>(context_slot(core.id)) = ContextRequest(Some((at, part)));
}
