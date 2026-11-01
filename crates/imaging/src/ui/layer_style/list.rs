//! The effect list on the dialog's left (task AD-8).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land (task AD-8)")]

use egui::Ui;

use super::labels::LayerStyleLabels;
use crate::layer_style::{EffectId, EffectKind, EffectSpec, LayerStyleDraft, LayerStylePage};

/// What the list asked for in one pass; the dialog applies it to a copy of the draft.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum ListAction {
    /// Show this page (a row click, Enter on the lead row).
    Select(LayerStylePage),
    /// An instance's check box (a click on it, Space on the lead row):
    /// [`LayerStyleDraft::toggle`].
    Toggle(EffectId),
    /// A click on an absent effect's row, or `+`: [`LayerStyleDraft::add_instance`].
    Add(EffectKind),
    /// `−`: [`LayerStyleDraft::remove_instance`].
    Remove(EffectId),
}

/// Draws the list, `layer_style.list_width` wide: the Styles and Blending Options rows, then one
/// row per instance in `specs` order (several instances of a kind in the style's order), and one
/// dimmed row with an empty check box per kind the layer lacks (`text.faint`; a click adds it,
/// turned on, #1357).
///
/// It is a library `List` of `ListRow`s at the list token's row height (lead call L16), each
/// effect row with a check box at the row start, its label in `type.body` (`text.muted` while the
/// effect is off, even when its page is selected, #2288), and for kinds that
/// [allow several](EffectKind::allows_several) a `+` icon button (Sm) at the row end and a `−`
/// before it while the kind has more than one instance.
///
/// Keys: one Tab stop; ↑ and ↓ move the lead, Space toggles the lead's check box (adds an absent
/// effect), Enter selects its page. ⌘] and ⌘[ are not used: the order is fixed. AccessKit: a
/// `ListBox` named [`LayerStyleLabels::effects`] of `CheckBox` rows named by the effect, with
/// their positions; `+` and `−` are buttons named [`LayerStyleLabels::add_instance`] and
/// [`LayerStyleLabels::remove_instance`].
pub(super) fn effect_list(
    ui: &mut Ui,
    draft: &LayerStyleDraft,
    specs: &[EffectSpec],
    labels: &LayerStyleLabels,
    enabled: bool,
) -> Option<ListAction> {
    // pc: crates/ui-egui/src/dialogs/layer_style.rs:838-938 (the left column: Styles and Blending
    // Options rows, absent rows, instance rows, `+` and `−` at 8 and 22 from the row end).
    todo!()
}
