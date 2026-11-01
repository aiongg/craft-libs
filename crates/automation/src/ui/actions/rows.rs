//! One set, action or step row with its include and dialog cells, and the tree's display order.
#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::behavior::DisplayOrder;
use craft_ui::list::ListRow;
use egui::{Response, Ui};

use super::labels::ActionsLabels;
use super::panel::{ActionSelection, ActionsHost, ActionsState};
use super::style::ActionsPanelStyle;
use crate::actions::{ActionInclude, ActionList};

/// The rows the tree shows this pass, in order, with their depths: loose actions, then each set
/// header and (unless collapsed) its actions; an expanded action's steps and its pending steps
/// after it. Built once per pass from the list, the state and the host.
pub(super) struct ActionRows {
    pub(super) rows: Vec<ActionSelection>,
    pub(super) depths: Vec<usize>,
    pub(super) groups: Vec<bool>,
}

impl ActionRows {
    /// The rows of `list` as `state` and `host` show them: sets from
    /// [`ActionList::sets`] (empty ones included), actions in
    /// [`ActionList::display_order`], an action inside a set at depth 1 (its steps at 2), a loose
    /// action at 0 (its steps at 1); every action is a group (it has a disclosure even with no
    /// steps, as PhotoCraft's), every set too. The recording action is expanded whatever `state`
    /// says.
    pub(super) fn build(list: &ActionList, state: &ActionsState, host: Option<&dyn ActionsHost>) -> Self {
        // pc: crates/ui-egui/src/panels/actions.rs:962-981 (display order, set headers before their first action, collapsed sets);
        // pc: crates/ui-egui/src/panels/actions.rs:947-955 (the recording row expands and is revealed).
        todo!()
    }

    /// The row height at `at`: `set_row_height`, `row_height` or `step_row_height`.
    pub(super) fn height(&self, at: usize, style: &ActionsPanelStyle) -> f32 {
        todo!()
    }
}

impl DisplayOrder<ActionSelection> for ActionRows {
    fn len(&self) -> usize {
        self.rows.len()
    }

    fn key(&self, at: usize) -> &ActionSelection {
        &self.rows[at]
    }

    fn depth(&self, at: usize) -> usize {
        self.depths.get(at).copied().unwrap_or(0)
    }

    fn has_children(&self, at: usize) -> bool {
        self.groups.get(at).copied().unwrap_or(false)
    }
}

/// The `ListRow` of `key`, wrapped by the panel in a `TreeRow`:
/// - a set: `Icon::Folder` or `Icon::FolderOpen`, its name, "{n} actions" (`ACTIONS`) as a
///   trailing text in `text.faint`;
/// - an action: `Icon::Play` glyph, its name (`renaming` when [`ActionsState::renaming`] is this
///   action), the trailing text ([`ActionsLabels::shortcut_steps`] or the step count,
///   [`ActionsLabels::partial_steps`] when partial), the recording dot (a 4 pt
///   `status.bad.solid` circle) as a trailing item while recording into it, the accessible name
///   with [`ActionsLabels::recording_suffix`] then;
/// - a step: the host's label, dimmed when switched off.
///
/// The include and dialog cells go in a leading slot of two `actions.check_size` + 5 wide cells
/// before the tree's indent, drawn by [`check_cell`]: `// needs: A ListRow::leading_item(width,
/// add)` (risk 6, see the spec's As built). Set rows leave the slot empty.
pub(super) fn row<'r>(
    key: &ActionSelection,
    list: &'r ActionList,
    state: &ActionsState,
    host: Option<&'r dyn ActionsHost>,
    labels: &ActionsLabels,
    style: &ActionsPanelStyle,
) -> ListRow<'r, ActionSelection> {
    // pc: crates/ui-egui/src/panels/actions.rs:830 `draw_set_header`, :1000-1042 (action row), :1213 `draw_step_label`.
    todo!()
}

/// What a check cell shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CheckKind {
    /// The include cell: a box `check_size` square, radius 2, 1 pt `text.faint` outline, a 1.5 pt
    /// check in `text.normal` when on, a dash when partial.
    Include(ActionInclude),
    /// The dialog cell: `Icon::Note` (Lucide "message-square") at `check_size`, `icon` colour when
    /// on, `text.faint` at 0.45 when off.
    Dialog(bool),
}

/// One include or dialog cell in `ui` (a 16-point square box): a click reports through the
/// returned response. AccessKit: a `CheckBox` named `name` (the row's label filled into
/// [`ActionsLabels::include`] or [`ActionsLabels::dialog`]), toggled on, off or `Mixed`; its
/// tooltip is the matching `*_tooltip` word. A `ToggleColumn` is not used: it has no mixed state
/// and is a `Button` (risk 6).
pub(super) fn check_cell(ui: &mut Ui, kind: CheckKind, name: &str, tooltip: &str, style: &ActionsPanelStyle) -> Response {
    // pc: crates/ui-egui/src/panels/actions.rs:1225-1251 `draw_step_controls`, :315 `paint_check`.
    todo!()
}
