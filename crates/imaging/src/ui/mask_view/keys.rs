//! [`MaskViewKeys`]: the view keys of Select and Mask (task SE-F6).

use egui::{Context, Event, Key, Popup};

use super::{MaskViewChange, MaskViewMode, MaskViewState};

/// The view keys, read once per pass from the context's input while the workspace is open.
///
/// With no ⌘, Ctrl or ⌥ held, on a press (not a repeat): a view's letter (O M V A T K Y) picks
/// it, F picks the next view, J toggles Show Edge, P toggles Show Original; X held hides the view
/// until it is released. Nothing is read while a text field has the keyboard or a popup is open
/// (the View Mode drop-down, a menu): `hide_view` is then cleared. R, `[` and `]` (the Refine Edge
/// Brush) are the workspace's.
pub struct MaskViewKeys;

impl MaskViewKeys {
    /// Applies this pass's keys to `state` and returns the changes, in order.
    pub fn read(ctx: &Context, state: &mut MaskViewState) -> Vec<MaskViewChange> {
        let mut changes = Vec::new();
        if ctx.text_edit_focused() || Popup::is_any_open(ctx) {
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
