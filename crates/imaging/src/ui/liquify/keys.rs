//! [`LiquifyKeys`]: Liquify's own keys (task WS-LQ1).

use egui::{Context, Event, Key, Modifiers};

use super::session::{LiquifyGlobal, LiquifySettings};

/// The keys Liquify reads beside the tool letters (the strip's `pick_by_key`), the zoom keys (the
/// canvas's `FullWindowZoomKeys`) and ⌘Z (`take_step`):
///
/// | Key | Result |
/// |---|---|
/// | `[` | [`LiquifyBrush::smaller`](super::LiquifyBrush::smaller) |
/// | `]` | [`LiquifyBrush::larger`](super::LiquifyBrush::larger) |
/// | ⌘H | Show Mask on or off (the mask itself stays) |
/// | ⌘I | [`LiquifyGlobal::InvertAll`] |
/// | ⌘D | [`LiquifyGlobal::MaskNone`] |
///
/// Read only while the workspace is the top modal layer and no text field edits
/// (`focus::key_owner`); each key is consumed.
pub struct LiquifyKeys;

impl LiquifyKeys {
    /// Applies [ ] and ⌘H to `settings` (returning whether they changed) and returns the operations
    /// of ⌘I and ⌘D, in the order pressed, for the workspace to record and report.
    pub fn read(ctx: &Context, settings: &mut LiquifySettings) -> (bool, Vec<LiquifyGlobal>) {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:740 `keys` (⌘H :755, ⌘I :759, ⌘D :762,
        // brackets :765-771).
        let mut changed = false;
        let mut globals = Vec::new();
        if ctx.text_edit_focused() {
            return (changed, globals);
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::H)) {
            settings.show_mask = !settings.show_mask;
            changed = true;
        }
        // ⌘I and ⌘D are recorded in the order pressed: read the order first, then consume one
        // event per entry.
        let pressed: Vec<Key> = ctx.input(|i| {
            i.events
                .iter()
                .filter_map(|e| match e {
                    Event::Key { key: key @ (Key::I | Key::D), pressed: true, modifiers, .. } if modifiers.command && !modifiers.alt => Some(*key),
                    _ => None,
                })
                .collect()
        });
        for key in pressed {
            if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, key)) {
                globals.push(if key == Key::I { LiquifyGlobal::InvertAll } else { LiquifyGlobal::MaskNone });
            }
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::OpenBracket)) {
            settings.brush = settings.brush.smaller();
            changed = true;
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::CloseBracket)) {
            settings.brush = settings.brush.larger();
            changed = true;
        }
        (changed, globals)
    }
}
