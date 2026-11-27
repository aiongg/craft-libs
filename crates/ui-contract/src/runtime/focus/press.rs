use craft_behavior::PartRef;
use craft_behavior::Winner;

use egui::Id;

/// What won a primary press at the begin hook.
pub(crate) struct PressAt {
    winner: Option<Winner<Id>>,
    /// The arena root the winner belongs to.
    root: Option<Id>,
    /// The egui widget that takes the press.
    target: Option<Id>,
    /// Every part under the press point on its layer, sensing or not, from last pass's shapes: `(owner, part)`.
    under: Vec<(Id, Option<PartRef>)>,
    /// The press landed in a popup layer opened above the layer of the widget that holds egui focus.
    over_focus: bool,
}

#[derive(Default)]
pub(crate) struct PressState {
    plan: Option<Plan>,
    pub(super) commit_now: Option<Id>,
    /// The widget a Tab or Shift+Tab moved focus off this pass, and whether the move was forward.
    pub(super) tabbed_from: Option<(Id, bool)>,
    /// Tab, Shift+Tab or F6 moved focus this pass.
    pub(super) by_keys: bool,
    focus_at_begin: Option<Id>,
    /// The parts declared `holds_drafts` in the running pass, `(owner, part)`.
    holding: Vec<(Id, PartRef)>,
    /// The parts declared `holds_drafts` in the last pass, whose shapes the press is classified on.
    last_holding: Vec<(Id, PartRef)>,
    /// The text stop whose draft this pass's press left unapplied.
    held_draft: Option<Id>,
}

impl PressState {
    /// Whether a part under the press is one last pass declared `holds_drafts`.
    fn holds(&self, under: &[(Id, Option<PartRef>)]) -> bool {
        under.iter().any(|(owner, part)| part.is_some_and(|part| self.last_holding.contains(&(*owner, part))))
    }
}
