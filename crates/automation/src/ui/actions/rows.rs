//! One set, action or step row with its include and dialog cells, and the tree's display order.

use craft_ui::behavior::DisplayOrder;
use craft_ui::controls::CheckState;
use craft_ui::edit::EditEvent;
use craft_ui::icons::Icon;
use craft_ui::list::{ListRow, ListRowStyle, ToggleColumn, ToggleEdit};
use craft_ui::localized::words::{Counted, Words};
use craft_ui::theme::{ToEgui as _, UiTokens, role_format};
use egui::{Color32, Context, Key, KeyboardShortcut, Label, Modifiers, RichText, Sense, vec2};

use super::labels::{ACTIONS, ActionsLabels, STEPS};
use super::panel::{ActionSelection, ActionsEvent, ActionsHost, ActionsPanelState};
use super::style::ActionsPanelStyle;
use crate::actions::{ActionInclude, ActionList};

/// The rows the tree shows this pass, in order, with their depths: loose actions, then each set
/// header and (unless collapsed) its actions; an expanded action's steps and its pending steps
/// after it. Built once per pass from the list, the state and the host.
pub(super) struct ActionRows {
    pub(super) rows: Vec<ActionSelection>,
    pub(super) depths: Vec<usize>,
    pub(super) groups: Vec<bool>,
    /// Whether the group at each position shows its children.
    pub(super) opens: Vec<bool>,
}

/// Whether the set `name` shows its actions: not collapsed, or holding the action being recorded.
fn set_open(list: &ActionList, state: &ActionsPanelState, host: Option<&dyn ActionsHost>, name: &str) -> bool {
    let recording = host.and_then(|host| host.recording());
    !state.settings.collapsed_sets.iter().any(|set| set == name) || recording.is_some_and(|action| list.set_of(action) == Some(name))
}

/// Whether the action `index` shows its steps: expanded, or being recorded.
fn action_open(list: &ActionList, state: &ActionsPanelState, host: Option<&dyn ActionsHost>, index: usize) -> bool {
    host.and_then(|host| host.recording()) == Some(index) || list.action(index).is_some_and(|action| state.expanded.contains(&action.name))
}

impl ActionRows {
    /// The rows of `list` as `state` and `host` show them: sets from
    /// [`ActionList::sets`] (empty ones included), actions in
    /// [`ActionList::display_order`], an action inside a set at depth 1 (its steps at 2), a loose
    /// action at 0 (its steps at 1); every action is a group (it has a disclosure even with no
    /// steps, as PhotoCraft's), every set too. The recording action is expanded whatever `state`
    /// says.
    pub(super) fn build(list: &ActionList, state: &ActionsPanelState, host: Option<&dyn ActionsHost>) -> Self {
        // pc: crates/ui-egui/src/panels/actions.rs:962-981 (display order, set headers before their first action, collapsed sets);
        // pc: crates/ui-egui/src/panels/actions.rs:947-955 (the recording row expands and is revealed).
        let mut out = Self { rows: Vec::new(), depths: Vec::new(), groups: Vec::new(), opens: Vec::new() };
        let order = list.display_order();
        for &index in order.iter().filter(|&&index| list.set_of(index).is_none()) {
            out.push_action(list, state, host, index, 0);
        }
        for set in list.sets() {
            let open = set_open(list, state, host, set);
            out.push(ActionSelection::Set(set.to_owned()), 0, true, open);
            if open {
                for &index in order.iter().filter(|&&index| list.set_of(index) == Some(set)) {
                    out.push_action(list, state, host, index, 1);
                }
            }
        }
        out
    }

    fn push(&mut self, row: ActionSelection, depth: usize, group: bool, open: bool) {
        self.rows.push(row);
        self.depths.push(depth);
        self.groups.push(group);
        self.opens.push(open);
    }

    fn push_action(&mut self, list: &ActionList, state: &ActionsPanelState, host: Option<&dyn ActionsHost>, index: usize, depth: usize) {
        let open = action_open(list, state, host, index);
        self.push(ActionSelection::Action(index), depth, true, open);
        if open {
            let shown = list.step_count(index) + host.map_or(0, |host| host.pending_steps(index).len());
            for step in 0..shown {
                self.push(ActionSelection::Step(index, step), depth + 1, false, false);
            }
        }
    }

    /// The row height at `at`: `set_row_height`, `row_height` or `step_row_height`.
    pub(super) fn height(&self, at: usize, style: &ActionsPanelStyle) -> f32 {
        match self.rows.get(at) {
            Some(ActionSelection::Set(_)) => style.set_row_height,
            Some(ActionSelection::Step(..)) => style.step_row_height,
            Some(ActionSelection::Action(_)) | None => style.row_height,
        }
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

    fn open(&self, at: usize) -> bool {
        self.opens.get(at).copied().unwrap_or(false)
    }
}

/// The `ListRow` of `key`, wrapped by the panel in a `TreeRow`:
/// - a set: `Icon::Folder` or `Icon::FolderOpen`, its name, "{n} actions" (`ACTIONS`) as a
///   trailing text in `text.faint`;
/// - an action: `Icon::Play` glyph, its name (`renaming` when [`ActionsPanelState::renaming`] is this
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
    ctx: &Context,
    key: &ActionSelection,
    list: &'r ActionList,
    state: &ActionsPanelState,
    host: Option<&'r dyn ActionsHost>,
    labels: &ActionsLabels,
    style: &ActionsPanelStyle,
) -> ListRow<'r, ActionSelection> {
    // pc: crates/ui-egui/src/panels/actions.rs:830 `draw_set_header`, :1000-1042 (action row), :1213 `draw_step_label`.
    let tokens = craft_ui::theme::current(ctx);
    let row_style = ListRow::<ActionSelection>::style(&tokens);
    match key {
        ActionSelection::Set(name) => {
            let count = list.actions().iter().filter(|action| action.set.as_deref() == Some(name.as_str())).count();
            let icon = if set_open(list, state, host, name) { Icon::FolderOpen } else { Icon::Folder };
            let row = ListRow::new(key.clone(), name.clone()).icon(icon);
            trailing_text(row, ctx, &tokens, &row_style, counted(ctx, ACTIONS, count))
        }
        ActionSelection::Action(index) => {
            let Some(action) = list.action(*index) else { return ListRow::new(key.clone(), String::new()) };
            let mut row =
                ListRow::new(key.clone(), action.name.clone()).icon(Icon::Play).toggle(include_column(include_state(action.include()), labels));
            let steps = list.step_count(*index);
            let mut text = counted(ctx, STEPS, steps);
            if action.include() == ActionInclude::Partial {
                let enabled = action.enabled_steps().count();
                text = labels.partial_steps.replace("{enabled}", &enabled.to_string()).replace("{steps}", &text);
            }
            if let Some(shortcut) = host.and_then(|host| host.shortcut(action)).map(|choice| choice.text()) {
                text = labels.shortcut_steps.replace("{shortcut}", &shortcut).replace("{steps}", &text);
            }
            row = trailing_text(row, ctx, &tokens, &row_style, text);
            if host.and_then(|host| host.recording()) == Some(*index) {
                row = row.accessible_name(labels.recording_suffix.replace("{name}", &action.name));
                let (color, side) = (style.recording, row_style.icon_size);
                row = row.trailing_item(side, move |ui| {
                    let (rect, _) = ui.allocate_exact_size(vec2(side, side), Sense::hover());
                    ui.painter().circle_filled(rect.center(), side * 0.2, color);
                });
            }
            row
        }
        ActionSelection::Step(index, step) => {
            let Some(action) = list.action(*index) else { return ListRow::new(key.clone(), String::new()) };
            match action.steps.get(*step) {
                Some(recorded) => {
                    let enabled = action.step_enabled(*step);
                    let label = host.map_or_else(|| recorded.command.clone(), |host| host.step_label(recorded));
                    ListRow::new(key.clone(), label)
                        .dimmed(!enabled)
                        .toggle(include_column(if enabled { CheckState::Checked } else { CheckState::Unchecked }, labels))
                        .toggle(dialog_column(action.dialogs.contains(step), labels))
                }
                None => {
                    let pending = host.and_then(|host| host.pending_steps(*index).get(*step - action.steps.len()));
                    let label =
                        pending.map_or_else(String::new, |pending| host.map_or_else(|| pending.command.clone(), |host| host.step_label(pending)));
                    ListRow::new(key.clone(), label)
                }
            }
        }
    }
}

/// The include cell of an action or step row: a [`ToggleColumn::check_box`] named
/// [`ActionsLabels::include`] with the row's label for `{row}`, tooltip
/// [`ActionsLabels::include_tooltip`], key Space. The state of an action row is its
/// [`ActionInclude`] (`All` checked, `Partial` mixed, `None` unchecked); of a step row, checked
/// unless the step is in `Action::disabled`.
pub(super) fn include_column(state: CheckState, labels: &ActionsLabels) -> ToggleColumn {
    // pc: crates/ui-egui/src/panels/actions.rs:1225-1234 `draw_step_controls` (the check, `CheckBox`
    // "Include in playback: {label}"), :315 `paint_check`.
    ToggleColumn::check_box(labels.include.clone(), state)
        .key(KeyboardShortcut::new(Modifiers::NONE, Key::Space))
        .tooltip(labels.include_tooltip.clone())
}

/// The include cell's state for an action's [`ActionInclude`].
pub(super) fn include_state(include: ActionInclude) -> CheckState {
    // new: `All` → `Checked`, `Partial` → `Mixed`, `None` → `Unchecked`.
    match include {
        ActionInclude::All => CheckState::Checked,
        ActionInclude::Partial => CheckState::Mixed,
        ActionInclude::None => CheckState::Unchecked,
    }
}

/// The dialog cell of a step row: `ToggleColumn::new(name, Icon::Note, Icon::Note, on)` with
/// [`check`](ToggleColumn::check)`(true)`, named [`ActionsLabels::dialog`] with the row's label
/// for `{row}`, tooltip [`ActionsLabels::dialog_tooltip`]; `on` when the step is in
/// `Action::dialogs`. Like every toggle column it shows its icon when on and under the pointer
/// when off (PhotoCraft draws the off icon at `text.faint` × 0.45 at rest; the list's column look
/// replaces it).
pub(super) fn dialog_column(on: bool, labels: &ActionsLabels) -> ToggleColumn {
    // pc: crates/ui-egui/src/panels/actions.rs:1236-1250 `draw_step_controls` (the "message-square" icon, "Show dialog during playback: {label}").
    ToggleColumn::new(labels.dialog.clone(), Icon::Note, Icon::Note, on).check(true).tooltip(labels.dialog_tooltip.clone())
}

/// The panel event for a toggle column edit of `edit.row` (an `Update` or `Set`; `Begin`,
/// `Commit` and `Cancel` report nothing): column 0 on a step row [`ActionsEvent::ToggleStep`],
/// on an action row [`ActionsEvent::ToggleAll`]; column 1 on a step row
/// [`ActionsEvent::ToggleDialog`]. `None` when `edit.on` is the state the cell already shows (a
/// sweep crossing a cell in its state), since the list's events toggle.
pub(super) fn toggle_event(edit: &ToggleEdit<ActionSelection>, list: &ActionList) -> Option<ActionsEvent> {
    // new: maps the list's toggle columns to the model's toggles (ActionList::{toggle_step, toggle_all, toggle_dialog}).
    let (on, column) = (edit.on, edit.column);
    match (&edit.row, column) {
        (ActionSelection::Step(action, step), 0) => {
            let shown = list.action(*action)?.step_enabled(*step);
            (shown != on).then_some(ActionsEvent::ToggleStep { action: *action, step: *step })
        }
        (ActionSelection::Action(action), 0) => {
            let shown = list.action(*action)?.include() == ActionInclude::All;
            (shown != on).then_some(ActionsEvent::ToggleAll { action: *action })
        }
        (ActionSelection::Step(action, step), 1) => {
            let shown = list.action(*action)?.dialogs.contains(step);
            (shown != on).then_some(ActionsEvent::ToggleDialog { action: *action, step: *step })
        }
        _ => None,
    }
}

/// The toggle event of a row's [`EditEvent`]: an `Update` or a `Set` maps through
/// [`toggle_event`]; `Begin`, `Commit` and `Cancel` report nothing.
pub(super) fn toggle_edit_event(event: &EditEvent<ToggleEdit<ActionSelection>>, list: &ActionList) -> Option<ActionsEvent> {
    match event {
        EditEvent::Update { value, .. } | EditEvent::Set { value, .. } => toggle_event(value, list),
        _ => None,
    }
}

/// `n` in the count word `count`, in the installed words.
fn counted(ctx: &Context, count: Counted, n: usize) -> String {
    let words = Words::of_ctx(ctx);
    words.counted(ctx, count, &words.get(count.word), n as u64)
}

/// `row` with `text` at the trailing end in `text.faint`, dropped before the label gets narrower
/// than `list.min_label`.
fn trailing_text<'r>(
    row: ListRow<'r, ActionSelection>,
    ctx: &Context,
    tokens: &UiTokens,
    row_style: &ListRowStyle,
    text: String,
) -> ListRow<'r, ActionSelection> {
    let color = tokens.color.text.faint.to_egui();
    let font = role_format(tokens, row_style.detail_role, color).font_id;
    let width = ctx.fonts_mut(|fonts| fonts.layout_no_wrap(text.clone(), font.clone(), Color32::PLACEHOLDER).size().x);
    row.optional_item(width.ceil(), move |ui| {
        ui.add(Label::new(RichText::new(text).font(font).color(color)).selectable(false));
    })
}
