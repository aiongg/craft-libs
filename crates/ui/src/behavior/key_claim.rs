//! [`KeyClaim`]: the keys a focused control edits with, which the app's shortcuts leave to it.

use egui::{Context, Id, Key, KeyboardShortcut, Modifiers, ViewportId};

/// Keys a focused control edits with that are also app shortcuts: a curve editor's Delete (Edit ›
/// Clear in PhotoCraft) and `+`, `=` and `-`. The control claims them every pass it has keyboard
/// focus; the app's shortcut dispatch asks [`claimed`](KeyClaim::claimed) before it fires one, and
/// leaves a claimed key to the control. Every other focused control leaves its keys to the
/// shortcuts, as in Photoshop.
///
/// ```
/// use craft_ui::behavior::{ClaimedKey, KeyClaim};
/// use egui::{Key, KeyboardShortcut, Modifiers};
///
/// let delete = KeyboardShortcut::new(Modifiers::NONE, Key::Delete);
/// # egui::__run_test_ui(|ui| {
/// let response = ui.button("Graph");
/// response.request_focus();
/// KeyClaim::claim(ui.ctx(), response.id, &[ClaimedKey::exact(Modifiers::NONE, Key::Delete)]);
/// assert!(KeyClaim::claimed(ui.ctx(), &delete));
/// assert!(!KeyClaim::claimed(ui.ctx(), &KeyboardShortcut::new(Modifiers::SHIFT, Key::Delete)), "Shift+Delete (Cut) stays the app's");
/// # });
/// ```
pub struct KeyClaim;

/// A key a control claims, and how an app shortcut matches it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClaimedKey {
    pub shortcut: KeyboardShortcut,
    /// Whether the key also counts with Shift or Alt held (egui's
    /// [`Modifiers::matches_logically`]), for a key some keyboards type with Shift (`+` is
    /// Shift+`=` on many). Otherwise the modifiers match exactly, so Shift+Delete (Cut) stays
    /// the app's.
    pub logical: bool,
}

impl ClaimedKey {
    /// `key` with exactly `modifiers` held.
    pub const fn exact(modifiers: Modifiers, key: Key) -> Self {
        Self { shortcut: KeyboardShortcut::new(modifiers, key), logical: false }
    }

    /// `key` with `modifiers` held, and also with Shift or Alt; never with Ctrl or Cmd that
    /// `modifiers` lacks, so Cmd/Ctrl+`+` (zoom) stays the app's.
    pub const fn logical(modifiers: Modifiers, key: Key) -> Self {
        Self { shortcut: KeyboardShortcut::new(modifiers, key), logical: true }
    }

    fn matches(&self, shortcut: &KeyboardShortcut) -> bool {
        let (key, modifiers) = (self.shortcut.logical_key, self.shortcut.modifiers);
        shortcut.logical_key == key
            && match self.logical {
                true => shortcut.modifiers.matches_logically(modifiers),
                false => shortcut.modifiers.matches_exact(modifiers),
            }
    }
}

/// The claim of the control that had focus when it last claimed, and the pass it claimed in.
#[derive(Clone, Debug)]
struct Claim {
    owner: Id,
    keys: Vec<ClaimedKey>,
    viewport: ViewportId,
    pass: u64,
}

impl KeyClaim {
    /// Claims `keys` for the control `id` while it has keyboard focus; nothing otherwise. Call it
    /// every pass the control wants them: a claim lasts for its pass and the next, so a control
    /// that stops claiming (a curve editor in pencil mode) gets no key from an older claim.
    pub fn claim(ctx: &Context, id: Id, keys: &[ClaimedKey]) {
        if !ctx.memory(|m| m.has_focus(id)) {
            return;
        }
        let claim = Claim { owner: id, keys: keys.to_vec(), viewport: ctx.viewport_id(), pass: ctx.cumulative_pass_nr() };
        ctx.data_mut(|d| d.insert_temp(Id::unique(CLAIM), claim));
    }

    /// Whether the control with keyboard focus claimed `shortcut` in this pass or the last, so the
    /// app's shortcut dispatch leaves it alone.
    pub fn claimed(ctx: &Context, shortcut: &KeyboardShortcut) -> bool {
        let Some(claim) = ctx.data(|d| d.get_temp::<Claim>(Id::unique(CLAIM))) else { return false };
        claim.pass + 1 >= ctx.cumulative_pass_nr_for(claim.viewport)
            && ctx.memory(|m| m.has_focus(claim.owner))
            && claim.keys.iter().any(|k| k.matches(shortcut))
    }
}

const CLAIM: &str = "craft_ui::behavior::key_claim";

#[cfg(test)]
mod tests {
    use egui::{Context, RawInput, Rect, Sense, Ui, pos2, vec2};

    use super::*;

    /// One pass: the app's shortcut dispatch runs `dispatch` first, then a focused control claims
    /// `keys` when there are some.
    fn pass(ctx: &Context, id: Id, keys: &[ClaimedKey], dispatch: impl FnOnce(&Context)) {
        let mut dispatch = Some(dispatch);
        let output = ctx.run_ui(RawInput::default(), |ui: &mut Ui| {
            if let Some(dispatch) = dispatch.take() {
                dispatch(ui.ctx());
            }
            let response = ui.interact(Rect::from_min_size(pos2(0.0, 0.0), vec2(40.0, 20.0)), id, Sense::click());
            response.request_focus();
            if !keys.is_empty() {
                KeyClaim::claim(ui.ctx(), id, keys);
            }
        });
        output.drop_without_applying_deltas();
    }

    #[test]
    fn a_claim_lasts_while_renewed_and_matches_its_modifiers() {
        let ctx = Context::default();
        let id = Id::unique("graph");
        let keys = [ClaimedKey::exact(Modifiers::NONE, Key::Delete), ClaimedKey::logical(Modifiers::NONE, Key::Plus)];
        let claimed = |ctx: &Context, modifiers, key| KeyClaim::claimed(ctx, &KeyboardShortcut::new(modifiers, key));
        pass(&ctx, id, &keys, |_| {});
        pass(&ctx, id, &[], |ctx| {
            assert!(claimed(ctx, Modifiers::NONE, Key::Delete), "the last pass's claim counts");
            assert!(!claimed(ctx, Modifiers::SHIFT, Key::Delete), "exact: Shift+Delete stays the app's");
            assert!(claimed(ctx, Modifiers::SHIFT, Key::Plus), "logical: + is Shift+= on many keyboards");
            assert!(!claimed(ctx, Modifiers::COMMAND, Key::Plus), "Cmd/Ctrl++ stays the app's");
            assert!(!claimed(ctx, Modifiers::NONE, Key::A));
        });
        pass(&ctx, id, &[], |ctx| {
            assert!(ctx.memory(|m| m.has_focus(id)));
            assert!(!claimed(ctx, Modifiers::NONE, Key::Delete), "not renewed, the claim is gone though the focus stays");
        });
    }
}
