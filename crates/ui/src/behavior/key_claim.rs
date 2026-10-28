//! [`KeyClaim`]: the keys a focused control edits with, which the app's shortcuts leave to it.

use egui::{Context, Id, KeyboardShortcut};

/// Keys a focused control edits with that are also app shortcuts: a curve editor's Delete (Edit ›
/// Clear in PhotoCraft) and `+`, `=` and `-`. The control claims them every frame it has keyboard
/// focus; the app's shortcut dispatch asks [`claimed`](KeyClaim::claimed) before it fires one, and
/// leaves a claimed key to the control. Every other focused control leaves its keys to the
/// shortcuts, as in Photoshop.
///
/// ```
/// use craft_ui::behavior::KeyClaim;
/// use egui::{Key, KeyboardShortcut, Modifiers};
///
/// let delete = KeyboardShortcut::new(Modifiers::NONE, Key::Delete);
/// # egui::__run_test_ui(|ui| {
/// let response = ui.button("Graph");
/// response.request_focus();
/// KeyClaim::claim(ui.ctx(), response.id, &[delete]);
/// assert!(KeyClaim::claimed(ui.ctx(), &delete));
/// # });
/// ```
pub struct KeyClaim;

/// The claim of the control that had focus when it last claimed.
#[derive(Clone, Debug, Default)]
struct Claim {
    owner: Option<Id>,
    keys: Vec<KeyboardShortcut>,
}

impl KeyClaim {
    /// Claims `keys` for the control `id` while it has keyboard focus; nothing otherwise. Call it
    /// every frame the control is shown. A key claimed without a modifier also counts with Shift
    /// or Alt held (`+` is Shift+`=` on many keyboards), not with Ctrl or Cmd.
    pub fn claim(ctx: &Context, id: Id, keys: &[KeyboardShortcut]) {
        if !ctx.memory(|m| m.has_focus(id)) {
            return;
        }
        ctx.data_mut(|d| d.insert_temp(Id::unique(CLAIM), Claim { owner: Some(id), keys: keys.to_vec() }));
    }

    /// Whether the control with keyboard focus claimed `shortcut`, so the app's shortcut
    /// dispatch leaves it alone. True only while that control keeps the focus.
    pub fn claimed(ctx: &Context, shortcut: &KeyboardShortcut) -> bool {
        let claim = ctx.data(|d| d.get_temp::<Claim>(Id::unique(CLAIM))).unwrap_or_default();
        let Some(owner) = claim.owner else { return false };
        ctx.memory(|m| m.has_focus(owner))
            && claim.keys.iter().any(|k| k.logical_key == shortcut.logical_key && shortcut.modifiers.matches_logically(k.modifiers))
    }
}

const CLAIM: &str = "craft_ui::behavior::key_claim";
