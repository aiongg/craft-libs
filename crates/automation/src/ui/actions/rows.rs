//! One set, action or step row with its include and dialog cells, and the tree's display order.
#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::behavior::DisplayOrder;
use craft_ui::controls::CheckState;
use craft_ui::list::{ListRow, ToggleColumn, ToggleEdit};

use super::labels::ActionsLabels;
use super::panel::{ActionSelection, ActionsEvent, ActionsHost, ActionsState};
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
/// The include and dialog cells are the row's [toggle columns](ToggleColumn), at the reading start
/// before the tree's indent (lead call: one component with the Layers eye and History's source):
/// - column 0, the include cell on action and step rows: [`include_column`];
/// - column 1, the dialog cell on step rows: [`dialog_column`].
///
/// Set rows have no toggle columns, so their disclosure sits at the leading edge, as PhotoCraft's
/// set header's triangle does.
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

/// The include cell of an action or step row: a [`ToggleColumn::check_box`] named
/// [`ActionsLabels::include`] with the row's label for `{row}`, tooltip
/// [`ActionsLabels::include_tooltip`], key Space. The state of an action row is its
/// [`ActionInclude`] (`All` checked, `Partial` mixed, `None` unchecked); of a step row, checked
/// unless the step is in `Action::disabled`.
pub(super) fn include_column(state: CheckState, labels: &ActionsLabels) -> ToggleColumn {
    // pc: crates/ui-egui/src/panels/actions.rs:1225-1234 `draw_step_controls` (the check, `CheckBox`
    // "Include in playback: {label}"), :315 `paint_check`.
    todo!()
}

/// The include cell's state for an action's [`ActionInclude`].
pub(super) fn include_state(include: ActionInclude) -> CheckState {
    // new: `All` → `Checked`, `Partial` → `Mixed`, `None` → `Unchecked`.
    todo!()
}

/// The dialog cell of a step row: `ToggleColumn::new(name, Icon::Note, Icon::Note, on)` with
/// [`check`](ToggleColumn::check)`(true)`, named [`ActionsLabels::dialog`] with the row's label
/// for `{row}`, tooltip [`ActionsLabels::dialog_tooltip`]; `on` when the step is in
/// `Action::dialogs`. Like every toggle column it shows its icon when on and under the pointer
/// when off (PhotoCraft draws the off icon at `text.faint` × 0.45 at rest; the list's column look
/// replaces it).
pub(super) fn dialog_column(on: bool, labels: &ActionsLabels) -> ToggleColumn {
    // pc: crates/ui-egui/src/panels/actions.rs:1236-1250 `draw_step_controls` (the "message-square" icon, "Show dialog during playback: {label}").
    todo!()
}

/// The panel event for a toggle column edit of `edit.row` (an `Update` or `Set`; `Begin`,
/// `Commit` and `Cancel` report nothing): column 0 on a step row [`ActionsEvent::ToggleStep`],
/// on an action row [`ActionsEvent::ToggleAll`]; column 1 on a step row
/// [`ActionsEvent::ToggleDialog`]. `None` when `edit.on` is the state the cell already shows (a
/// sweep crossing a cell in its state), since the list's events toggle.
pub(super) fn toggle_event(edit: &ToggleEdit<ActionSelection>, list: &ActionList) -> Option<ActionsEvent> {
    // new: maps the list's toggle columns to the model's toggles (ActionList::{toggle_step, toggle_all, toggle_dialog}).
    todo!()
}
