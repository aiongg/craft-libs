//! [`LiquifyKeys`]: Liquify's own keys (task WS-LQ1).

use egui::Context;

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
    #[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-LQ1")]
    pub fn read(ctx: &Context, settings: &mut LiquifySettings) -> (bool, Vec<LiquifyGlobal>) {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:740 `keys` (⌘H :755, ⌘I :759, ⌘D :762,
        // brackets :765-771).
        todo!()
    }
}
