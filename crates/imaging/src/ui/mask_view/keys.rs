//! [`MaskViewKeys`]: the view keys of Select and Mask.

use craft_ui::focus::key_owner;
use egui::{Event, Key, Ui};

use super::{MaskViewChange, MaskViewMode, MaskViewState};

/// The view keys, read once per pass from the context's input while the workspace is open.
///
/// With no ⌘, Ctrl or ⌥ held, on a press (not a repeat): a view's letter (O M V A T K Y) picks
/// it, F picks the next view, J toggles Show Edge, P toggles Show Original; X held hides the view
/// until it is released. Nothing is read while a text field has the keyboard or a popup is open
/// (the View Mode drop-down, a menu, a popover) or the workspace is not the top modal layer
/// ([`key_owner::keys_free`], the gate of `FullWindowFx::keys_free`): `hide_view` is then cleared. R, `[` and `]` (the Refine Edge
/// Brush) are the workspace's.
pub struct MaskViewKeys;

impl MaskViewKeys {
    /// Applies this pass's keys to `state` and returns the changes, in order. `ui` is the
    /// workspace's: its layer must be the top modal layer for the keys to be free.
    pub fn read(ui: &Ui, state: &mut MaskViewState) -> Vec<MaskViewChange> {
        let ctx = ui.ctx();
        let mut changes = Vec::new();
        if !key_owner::keys_free(ctx, ui.layer_id()) {
            if state.hide_view {
                state.apply(MaskViewChange::HideView(false));
                changes.push(MaskViewChange::HideView(false));
            }
            return changes;
        }
        let (pressed, hold_x) = ctx.input(|i| {
            let plain = !i.modifiers.command && !i.modifiers.ctrl && !i.modifiers.alt;
            let keys: Vec<Key> = i
                .events
                .iter()
                .filter_map(|e| match e {
                    Event::Key { key, pressed: true, repeat: false, .. } if plain => Some(*key),
                    _ => None,
                })
                .collect();
            (keys, plain && i.key_down(Key::X))
        });
        for key in pressed {
            let change = if key == Key::F {
                MaskViewChange::Mode(state.mode.next())
            } else if key == Key::J {
                MaskViewChange::ShowEdge(!state.show_edge)
            } else if key == Key::P {
                MaskViewChange::ShowOriginal(!state.show_original)
            } else if let Some(mode) = MaskViewMode::ALL.into_iter().find(|m| m.key() == key) {
                MaskViewChange::Mode(mode)
            } else {
                continue;
            };
            state.apply(change);
            changes.push(change);
        }
        if hold_x != state.hide_view {
            state.apply(MaskViewChange::HideView(hold_x));
            changes.push(MaskViewChange::HideView(hold_x));
        }
        changes
    }
}
