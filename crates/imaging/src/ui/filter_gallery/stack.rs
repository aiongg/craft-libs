//! [`EffectStack`]: the effect-layer stack under the settings (task WS-FG2).

use egui::{AsIdSalt, IdSalt, Key, KeyboardShortcut, Modifiers, Response, Ui};

use craft_ui::behavior::{Axis, DropPosition, Move, Reorder, RowInfo, Selection, SelectionMode};
use craft_ui::composites::PanelFooter;
use craft_ui::edit::EditEvent;
use craft_ui::icons::Icon;
use craft_ui::list::{List, ListEvent, ListRow, RowEvent, ToggleColumn};
use craft_ui::theme::{Tokens, TokensOf as _};

use super::FilterGalleryLabels;
use super::session::{EffectChange, EffectLayer};

/// What [`EffectStack::show`] reports.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct EffectStackResponse {
    pub response: Response,
    /// The row picked (click, Up, Down).
    pub selected: Option<usize>,
    /// An eye, a move, New or Delete.
    pub change: Option<EffectChange>,
}

/// The stack (lead call 39): a library `List` `filter_gallery.stack_height` high, top row last
/// applied (so the rows show `effects` in reverse), each row with an eye column
/// (`ToggleColumn::eye`) and the filter's name; the selected row is the list's selection. Under it
/// a `PanelFooter` with Move up, Move down, New effect layer, Delete effect layer (disabled with
/// one layer). Rows reorder by drag (`Reorder`). The rows are as high as the list's rows.
///
/// Keyboard: Up and Down select, Space toggles the eye, ⌘] and ⌘[ move the row. AccessKit: a
/// list named "Effect Layers"; rows named by their filter with the eye as a toggle.
#[must_use = "show the stack with `show`"]
pub struct EffectStack<'a> {
    id_salt: IdSalt,
    effects: &'a [EffectLayer],
    selected: usize,
    labels_of: &'a dyn Fn(&str) -> String,
    labels: &'a FilterGalleryLabels,
}

impl<'a> EffectStack<'a> {
    pub fn new(
        id_salt: impl AsIdSalt,
        effects: &'a [EffectLayer],
        selected: usize,
        labels_of: &'a dyn Fn(&str) -> String,
        labels: &'a FilterGalleryLabels,
    ) -> Self {
        Self { id_salt: IdSalt::new(id_salt), effects, selected, labels_of, labels }
    }

    pub fn show(self, ui: &mut Ui) -> EffectStackResponse {
        // pc: crates/ui-egui/src/dialogs/gallery_ui.rs:780, :832-881 (list 190 high, rows 24 with the eye,
        // the four buttons).
        let Self { id_salt, effects, selected, labels_of, labels } = self;
        let count = effects.len();
        let id = ui.make_persistent_id(id_salt);
        let height = Tokens::of(ui).component.filter_gallery.stack_height;
        // The top row is applied last: the display order runs from the last effect to the first.
        let order: Vec<RowInfo<usize>> = (0..count).rev().map(RowInfo::leaf).collect();
        let mut selection = Selection::only(selected.min(count.saturating_sub(1)));
        let eye_name = format!("{} {{row}}", labels.visibility);
        let space = KeyboardShortcut::new(Modifiers::NONE, Key::Space);

        let out = List::new(id.with("list"), labels.effect_layers.as_str(), order.as_slice())
            .selection(&mut selection)
            .mode(SelectionMode::Single)
            .allow_empty(false)
            .max_height(height)
            .reorder(Reorder::new(Axis::Vertical))
            .show(ui, |_, &index| {
                let effect = effects.get(index);
                let visible = effect.is_none_or(|e| e.visible);
                let name = effect.map_or_else(String::new, |e| labels_of(&e.filter));
                ListRow::new(index, name).dimmed(!visible).toggle(ToggleColumn::eye(eye_name.clone(), visible).key(space))
            });

        let mut change = None;
        for event in &out.events {
            match event {
                ListEvent::Row(RowEvent::Toggle(edit)) => {
                    if let Some(toggle) = edit.value()
                        && effects.get(toggle.row).is_some_and(|e| e.visible != toggle.on)
                    {
                        change = Some(EffectChange::Visible { index: toggle.row, visible: toggle.on });
                    }
                }
                ListEvent::Reorder(edit) => {
                    if let Some(mv) = finished(edit)
                        && let Some((from, to)) = effect_move(count, mv)
                    {
                        change = Some(EffectChange::Move { from, to });
                    }
                }
                _ => {}
            }
        }

        let current = selected.min(count.saturating_sub(1));
        let footer = PanelFooter::show(ui, |ui| {
            let mut footer = None;
            // The row runs from the trailing end: Delete first, Move up last.
            let delete = ui.add_enabled_ui(count > 1, |ui| PanelFooter::button(ui, Icon::Trash, &labels.delete_layer)).inner;
            let add = PanelFooter::button(ui, Icon::NewLayer, &labels.new_layer);
            let down = ui.add_enabled_ui(current > 0, |ui| PanelFooter::button(ui, Icon::ChevronDown, &labels.move_down)).inner;
            let up = ui.add_enabled_ui(current + 1 < count, |ui| PanelFooter::button(ui, Icon::ChevronUp, &labels.move_up)).inner;
            if delete.clicked() {
                footer = Some(EffectChange::Delete);
            } else if add.clicked() {
                footer = Some(EffectChange::Add);
            } else if down.clicked() {
                footer = Some(EffectChange::Move { from: current, to: current - 1 });
            } else if up.clicked() {
                footer = Some(EffectChange::Move { from: current, to: current + 1 });
            }
            footer
        });

        let picked = selection.lead.filter(|&lead| lead != selected && lead < count);
        EffectStackResponse { response: out.response, selected: picked, change: footer.or(change) }
    }
}

/// The move a finished drag or stacking key makes.
fn finished(edit: &EditEvent<Move<usize>>) -> Option<&Move<usize>> {
    match edit {
        EditEvent::Set { value, .. } | EditEvent::Commit { value, changed: true, .. } => Some(value),
        _ => None,
    }
}

/// `mv` of display rows as the effect indices to move from and to: the display order runs from the
/// last effect to the first.
fn effect_move(count: usize, mv: &Move<usize>) -> Option<(usize, usize)> {
    let &[from] = mv.items.as_slice() else { return None };
    let mut display: Vec<usize> = (0..count).rev().collect();
    display.retain(|&index| index != from);
    let target = display.iter().position(|&index| index == mv.target);
    let at = match (mv.position, target) {
        (DropPosition::Before, Some(at)) => at,
        (DropPosition::After, Some(at)) => at + 1,
        // Dropped onto itself, or into a row: the order stays.
        _ => return None,
    };
    display.insert(at, from);
    let shown = display.iter().position(|&index| index == from)?;
    let to = count - 1 - shown;
    (to != from).then_some((from, to))
}
