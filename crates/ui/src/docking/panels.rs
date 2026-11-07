//! `DockPanels`: what a dock asks of the host for each panel.

use super::spec::PanelSpec;
use crate::contract::BodyCx;
use crate::menu::PanelRow;
use craft_behavior::GroupId;
use egui::{Rect, Ui};

/// The host's panels. A dock calls, in one pass and for each group it shows:
///
/// 1. `group_view`, with the group's rect. When it returns true the host showed the group itself: the dock shows
///    no tab strip and no panel body for that group, and calls neither `group_header` nor `body` for it.
/// 2. Otherwise `group_header` once, inside the group's header region before its tab strip, then `spec` for each of
///    its panels, then `body` for the panels whose bodies are shown. `panel_rows` runs when a panel's menu opens.
///
/// A panel's `spec` is read every pass, so a host may change a label or a limit between passes.
pub trait DockPanels<P> {
    /// What the dock reads of `panel`.
    fn spec(&self, panel: &P) -> PanelSpec;

    /// Shows `panel`'s body in `ui`.
    fn body(&mut self, panel: &P, ui: &mut Ui, cx: &mut BodyCx<'_>);

    /// Shows `group` itself in `rect` in place of its tab strip and body, and returns true; false (the default)
    /// leaves the group to the dock. A canvas group uses it to draw the document at the group's rect.
    fn group_view(&mut self, _group: GroupId, _ui: &mut Ui, _rect: Rect, _cx: &mut BodyCx<'_>) -> bool {
        false
    }

    /// Shows the host's controls in the header of `group` before its tab strip (the default shows none). It does
    /// not run for a group whose `group_view` returned true.
    fn group_header(&mut self, _group: GroupId, _ui: &mut Ui, _cx: &mut BodyCx<'_>) {}

    /// The host's rows of `panel`'s menu, shown before the dock's own rows (the default has none). Command rows
    /// run by themselves; a `PanelRow::Item` row is a dock row.
    fn panel_rows(&mut self, _panel: &P) -> Vec<PanelRow> {
        Vec::new()
    }
}
