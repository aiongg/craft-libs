//! Step 17, finish: the response's click facts and the bound command's fire.

use crate::contract::cx::CxCore;
use crate::runtime::commands::{CommandBoard, FirePhase, FireSource, Fired};
use craft_behavior::{EditEvent, Mixed};
use craft_ui_rt::BuildCx;

/// Step 17, finish, before `C::finish`: the response reports a key or part activation as a click and
/// a value change as a change, and the bound command of an activation fires once, at once, on the
/// board's handler (contract-rt 15.8): with the source `Cx::activate` injected, else as a component
/// activation. `changed` is `value_changed` of the run's events.
pub(crate) fn before_finish(
    core: &mut CxCore<'_>,
    activated: bool,
    injected: Option<FireSource>,
    changed: bool,
    board: Option<&CommandBoard>,
    cx: &mut BuildCx<'_>,
) {
    let by_key = activated && core.activation.is_some_and(|activation| activation.by_key);
    core.response.note_activation(by_key || core.part_clicked, changed);
    let (Some(command), Some(board)) = (core.command.as_deref().filter(|_| activated), board) else { return };
    let source = injected.unwrap_or(FireSource::Component);
    let mods = core.activation.map_or(craft_keymap::Mods::NONE, |activation| activation.mods);
    let fired = Fired::new(command.command.clone(), source, FirePhase::Press, cx.now()).mods(mods);
    board.run(&fired, cx.store_mut());
}

/// Whether a `Set` or a `Commit` of the run moved the value: the response then reports a change.
pub(crate) fn value_changed<V: PartialEq>(events: &[EditEvent<V>]) -> bool {
    events.iter().any(|event| match event {
        EditEvent::Commit { before, after, .. } | EditEvent::Set { before, after, .. } => !matches!(before, Mixed::Same(held) if held == after),
        _ => false,
    })
}
