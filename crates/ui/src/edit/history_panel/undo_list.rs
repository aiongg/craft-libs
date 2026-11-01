//! `UndoList`: the Undo drop-down list over `StepRows`.

use egui::accesskit::Role;
use egui::emath::GuiRounding as _;
use egui::text::{LayoutJob, TextWrapping};
use egui::{Align2, AsIdSalt, Id, IdSalt, Key, Modifiers, Rect, Response, Sense, Ui, UiBuilder, pos2, vec2};

use super::HistoryPanel;
use super::labels::{UNDO_ACTIONS, UndoListLabels};
use crate::direction;
use crate::edit::steps::StepRows;
use crate::focus::key_owner::{KeyOwner, OwnerGate, owner};
use crate::list::ListRow;
use crate::localized::words::{Labels as _, Words};
use crate::primitives::Interactive;
use crate::theme::{Tokens, TokensOf as _, role_format};
use craft_theme::Stroke as Hairline;

/// Rows shown before the list scrolls (WordCraft `chrome.rs:159`).
const VISIBLE_ROWS: f32 = 18.0;

/// What the undo list reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum UndoListEvent {
    /// Undo this many actions: the marked row and every newer one (WordCraft's
    /// `edit.undoTo {count}`). The list has closed its popup.
    UndoThrough(usize),
}

/// What [`UndoList::show`] returns.
#[derive(Debug)]
#[non_exhaustive]
#[must_use = "the app undoes what the list reports"]
pub struct UndoListOutput {
    /// The list.
    pub response: Response,
    /// The pass's event, if any.
    pub event: Option<UndoListEvent>,
}

/// WordCraft's Undo drop-down: the actions that can be undone, newest first, in a popup the app
/// opens from its own drop-down arrow with `Popup::menu`.
///
/// ```ignore
/// Popup::menu(&arrow).show(|ui| {
///     if let Some(UndoListEvent::UndoThrough(count)) = UndoList::new("undo-list", &rows).show(ui).event {
///         app.run("edit.undoTo", json!({ "count": count }));
///     }
/// });
/// ```
///
/// - **Rows.** One row per past state, from the current state back to row 1 (row 0, "Open", is
///   not an action), labels only (no source column, no icons), `list.row_height` high, the
///   popup `history.undo_list_width` wide ([`HistoryPanel::style`](super::HistoryPanel::style)).
///   At most 18 rows show; more scroll, keeping the marked row in view.
/// - **Marking.** Hover or Down and Up mark rows `0..=i` (row 0 is the newest), filled with
///   `list.hover`. A click or Enter reports `UndoThrough(i + 1)` and closes the popup
///   (`Ui::close`).
/// - **Footer.** "Undo {n} Actions" (a plural word: "Undo 1 Action") while rows are marked; while
///   none is, a Cancel row that closes the popup without an event. Esc closes without an event.
/// - **AccessKit.** A `ListBox` named [`UndoListLabels::list`] ("Undo History"); rows named by
///   their labels; the footer text is a label node whose text follows the count; the Cancel row
///   is a `Button`.
#[must_use = "show the list with `show`"]
pub struct UndoList<'a> {
    id_salt: IdSalt,
    steps: &'a dyn StepRows,
    labels: Option<UndoListLabels>,
    preset: Option<Option<usize>>,
}

impl<'a> UndoList<'a> {
    /// A list over `steps`.
    pub fn new(id_salt: impl AsIdSalt, steps: &'a dyn StepRows) -> Self {
        Self { id_salt: IdSalt::new(id_salt), steps, labels: None, preset: None }
    }

    /// Starts the list with `marked` as its mark (row 0 is the newest), for stories. Only the
    /// first show of the list uses it; a mark cleared by closing stays cleared.
    pub fn marked(mut self, marked: Option<usize>) -> Self {
        self.preset = Some(marked);
        self
    }

    /// The list's words (default English).
    pub fn labels(mut self, labels: UndoListLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Shows the list in the popup's `Ui`.
    pub fn show(self, ui: &mut Ui) -> UndoListOutput {
        let tokens = Tokens::of(ui);
        let style = ListRow::<()>::style(&tokens);
        let width = HistoryPanel::style(&tokens).undo_list_width;
        let labels = self.labels.unwrap_or_else(|| UndoListLabels::of(ui));
        let words = Words::of(ui);
        let id = ui.make_persistent_id(self.id_salt);
        let mark_id = id.with("mark");
        // Row `i` (0 is the newest) is state `current - i`; state 0 ("Open") is not an action.
        let steps = self.steps;
        let current = steps.current().min(steps.len().saturating_sub(1));
        let count = current;
        let state_of = |i: usize| current - i;

        let stored: Option<Option<usize>> = ui.data(|d| d.get_temp(mark_id));
        let mut mark: Option<usize> = stored.or(self.preset).flatten().filter(|&m| m < count);
        let mut event = None;
        let mut close = false;
        let mut moved_by_key = false;

        // The keys are the list's while it holds focus or its popup owns the keyboard.
        let holds_keys = owner(ui.ctx(), &OwnerGate::default()) == KeyOwner::Menu
            || ui.memory(|m| {
                // egui drops focus on Esc before the pass runs, so the previous frame's focus counts.
                let ours = |f: Id| f == id.with("cancel") || (0..count).any(|i| f == row_id(id, i));
                m.focused().is_some_and(ours) || ours_last_frame(m, id, count)
            });
        let (down, up, enter, escape) = if holds_keys {
            ui.input_mut(|i| {
                (
                    i.consume_key(Modifiers::NONE, Key::ArrowDown),
                    i.consume_key(Modifiers::NONE, Key::ArrowUp),
                    i.consume_key(Modifiers::NONE, Key::Enter),
                    i.consume_key(Modifiers::NONE, Key::Escape),
                )
            })
        } else {
            (false, false, false, false)
        };
        if down && count > 0 {
            mark = Some(mark.map_or(0, |m| (m + 1).min(count - 1)));
            moved_by_key = true;
        }
        if up {
            mark = mark.and_then(|m| m.checked_sub(1));
            moved_by_key = true;
        }
        if escape {
            close = true;
        }
        let pointer_moved = ui.input(|i| i.pointer.delta() != vec2(0.0, 0.0));

        ui.set_width(width);
        let scope = ui.scope_builder(
            UiBuilder::new().id_salt(self.id_salt).accessibility_role(Role::ListBox).accessibility_label(labels.list.clone()),
            |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let mut hovered = None;
                let mut clicked = None;
                let mut rects: Vec<Rect> = Vec::with_capacity(count);
                egui::ScrollArea::vertical().max_height(style.row_height * VISIBLE_ROWS).auto_shrink([false, true]).show(ui, |ui| {
                    for i in 0..count {
                        let label = steps.label(state_of(i));
                        let size = vec2(ui.available_width(), style.row_height);
                        let response = Interactive::with_id(row_id(id, i), Role::ListBoxOption, label.as_ref())
                            .sense(Sense::click())
                            .selected(mark.is_some_and(|m| i <= m))
                            .state_layer(false)
                            .focus_ring(false)
                            .interact(ui, size)
                            .finish(ui);
                        if response.contains_pointer() {
                            hovered = Some(i);
                        }
                        if response.clicked() {
                            clicked = Some(i);
                        }
                        if moved_by_key && mark == Some(i) {
                            ui.scroll_to_rect(response.rect, None);
                        }
                        rects.push(response.rect);
                    }
                });
                if pointer_moved {
                    mark = hovered;
                }
                for (i, rect) in rects.iter().enumerate() {
                    if !ui.is_rect_visible(*rect) {
                        continue;
                    }
                    if mark.is_some_and(|m| i <= m) {
                        ui.painter().rect_filled(*rect, 0.0, style.hover);
                    }
                    let label = steps.label(state_of(i));
                    paint_label(ui, &tokens, &style, *rect, label.into_owned());
                }
                if let Some(i) = clicked {
                    event = Some(UndoListEvent::UndoThrough(i + 1));
                    close = true;
                } else if enter && let Some(m) = mark {
                    event = Some(UndoListEvent::UndoThrough(m + 1));
                    close = true;
                }
                paint_rule(ui, &style);
                match mark {
                    None => {
                        let size = vec2(ui.available_width(), style.row_height);
                        let response = Interactive::with_id(id.with("cancel"), Role::Button, labels.cancel.as_str())
                            .sense(Sense::click())
                            .state_layer(false)
                            .interact(ui, size)
                            .finish(ui);
                        if ui.is_rect_visible(response.rect) {
                            if response.hovered() {
                                ui.painter().rect_filled(response.rect, 0.0, style.hover);
                            }
                            paint_label(ui, &tokens, &style, response.rect, labels.cancel.clone());
                        }
                        if response.clicked() {
                            close = true;
                        }
                    }
                    Some(m) => {
                        let text = words.counted(ui.ctx(), UNDO_ACTIONS, &labels.undo_actions, (m + 1) as u64);
                        let job = LayoutJob::single_section(text, role_format(&tokens, style.label_role, style.text));
                        ui.allocate_ui_with_layout(
                            vec2(ui.available_width(), style.row_height),
                            egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                            |ui| ui.label(job),
                        );
                    }
                }
            },
        );
        // A closed list reopens unmarked.
        ui.data_mut(|d| d.insert_temp(mark_id, if close { None } else { mark }));
        if close {
            ui.close();
        }
        UndoListOutput { response: scope.response, event }
    }
}

/// Draws `text` at the reading start of `rect`, inside the row padding, clipped to the row.
fn paint_label(ui: &Ui, tokens: &crate::theme::UiTokens, style: &crate::list::ListRowStyle, rect: Rect, text: String) {
    let inner = rect.shrink2(vec2(style.pad_x, 0.0));
    let mut job = LayoutJob::single_section(text, role_format(tokens, style.label_role, style.text));
    job.wrap = TextWrapping::truncate_at_width(inner.width().max(0.0));
    let galley = ui.painter().layout_job(job);
    let (anchor, align) = if direction::is_rtl(ui) {
        (pos2(inner.right(), rect.center().y), Align2::RIGHT_CENTER)
    } else {
        (pos2(inner.left(), rect.center().y), Align2::LEFT_CENTER)
    };
    let pos = align.anchor_size(anchor, galley.size()).min;
    ui.painter_at(rect).galley(pos, galley, style.text);
}

/// The id of row `i` under the list's `id`.
fn row_id(id: Id, i: usize) -> Id {
    id.with(("row", i))
}

/// The rule above the footer: one `list.divider` hairline across the list, as [`ListRow::rule`].
fn paint_rule(ui: &mut Ui, style: &crate::list::ListRowStyle) {
    let width = Hairline::Hairline.width(ui.pixels_per_point());
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), width), Sense::hover());
    if ui.is_rect_visible(rect) {
        let y = (rect.bottom() - width / 2.0).round_to_pixel_center(ui.pixels_per_point());
        ui.painter().hline(rect.x_range(), y, egui::Stroke::new(width, style.divider.color));
    }
}

/// Whether the cancel row or a list row had focus at the end of the previous frame.
fn ours_last_frame(memory: &egui::Memory, id: Id, count: usize) -> bool {
    memory.had_focus_last_frame(id.with("cancel")) || (0..count).any(|i| memory.had_focus_last_frame(row_id(id, i)))
}
