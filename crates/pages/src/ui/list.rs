//! [`PageList`] (N3.21): Illustrator's Artboards panel and InDesign's page list as one list: a
//! numbered row per page or artboard with its name, picked, reordered by dragging or with the
//! stacking keys, renamed in place, with New and Delete in the footer and a lock column.
//!
//! A composite of craft-ui's list pieces, never a copy: [`List`] with
//! [`ListRow`]s (a [`ToggleColumn::lock`]
//! column, the number painted in the row's glyph slot, the options icon as a trailing item),
//! `List::reorder` for drags and stacking keys, `List`'s rename for names, and a
//! [`PanelFooter`] with New and Delete as drop-aware buttons.
//!
//! ```ignore
//! let out = PageList::new("artboards", &rows, &mut picked, active).show(ui);
//! for event in out.events {
//!     match event {
//!         PageListEvent::Go(i) => doc.set_active_artboard(i),
//!         PageListEvent::Fit(i) => view.fit_artboard(i),
//!         PageListEvent::Rename { row, text } => doc.rename_artboards(&picked, row, &text),
//!         PageListEvent::Reorder(EditEvent::Commit { value, .. }) => doc.move_artboards(value),
//!         PageListEvent::Lock(edit) => doc.apply_locks(edit),
//!         PageListEvent::New { duplicate, options } => doc.new_artboard(duplicate, options),
//!         _ => {}
//!     }
//! }
//! ```

use std::cell::RefCell;

use craft_ui::behavior::{Axis, ClaimedKey, KeyClaim, ListDrag, Move, Reorder, RowInfo, Selection, SelectionChange};
use craft_ui::composites::PanelFooter;
use craft_ui::controls::{ControlSize, IconButton};
use craft_ui::edit::EditEvent;
use craft_ui::icons::Icon;
use craft_ui::list::{ContextMenuRequest, List, ListEvent, ListRow, RowEvent, ToggleColumn, ToggleEdit};
use craft_ui::localized::{Labels as _, LocaleNumbers};
use craft_ui::theme::{Tokens, TokensOf as _, role_format};
use egui::text::LayoutJob;
use egui::{Key, Modifiers, PointerButton, Rect, Response, Ui, vec2};

craft_ui::words! {
    /// The list's words (VectorCraft's strings).
    #[non_exhaustive]
    pub struct PageListLabels {
        artboards: "pageList.artboards" = "Artboards";
        new_artboard: "pageList.new" = "New Artboard";
        delete: "pageList.delete" = "Delete Artboard";
        options: "pageList.options" = "Artboard Options";
        name: "pageList.name" = "Name";
        lock: "pageList.lock" = "Lock";
    }
}

/// One row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageListRow {
    #[allow(missing_docs)]
    pub name: String,
    /// Whether the row is locked against edits.
    pub locked: bool,
}

/// What the list reports, in the order it happened this pass.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum PageListEvent {
    /// The picked rows changed (applied to the selection given), as `ListEvent::Selection`.
    Selection(SelectionChange<usize>),
    /// A click on a row, or the lead moved by a key: it becomes the active page or artboard.
    Go(usize),
    /// Enter on the lead row, or a double-click on a row's name that doesn't rename it (the
    /// kernel's meaning).
    Activate(usize),
    /// A double-click on a row's number: fit it in the window (Illustrator).
    Fit(usize),
    /// A name typed in place (F2, a double-click on the name, the context menu's Rename): trimmed,
    /// never empty and never unchanged. With several rows picked (and `row` one of them) the field
    /// opens empty, and the app gives the name to all of them numbered in list order as
    /// `{text}-1`, `{text}-2` (VectorCraft `artboards.rs:243`).
    Rename {
        /// The row that was renamed.
        row: usize,
        /// The new name.
        text: String,
    },
    /// A drag of rows or a stacking key (⌘/Ctrl+] and [), as `List` reports reorders.
    Reorder(EditEvent<Move<usize>>),
    /// A click or a sweep in the lock column, as `List` reports toggle columns.
    Lock(EditEvent<ToggleEdit<usize>>),
    /// New, or a row dropped on New (`duplicate`, the dragged rows' `grabbed` row); `options`
    /// with Alt held (open the options for the new one first).
    New {
        /// The row dragged onto New, to duplicate; `None` for a plain New.
        duplicate: Option<usize>,
        /// Whether Alt was held: open the options for the new row first.
        options: bool,
    },
    /// Delete (the button, the Delete key, or rows dropped on it): the picked rows that aren't
    /// locked, or the dropped rows.
    Delete(Vec<usize>),
    /// The options icon: a click on the active row's, a double-click on another's (a single click
    /// on another's is [`Go`](Self::Go)).
    Options(usize),
    /// The app's menu is asked for (`List`'s context menu request).
    ContextMenu(ContextMenuRequest<usize>),
}

/// The list's result.
#[derive(Debug)]
#[must_use = "the app applies the list's events"]
pub struct PageListOutput {
    /// The list's area.
    pub response: Response,
    /// What the user did this pass, in order.
    pub events: Vec<PageListEvent>,
}

/// The list, built per pass (behaviour: Photoshop's Layers and Artboards panel rules where they
/// differ from Illustrator's, then VectorCraft).
///
/// - Rows are `ListRow`s: the number in the glyph slot (`ListRow::glyph_sized`, its width the
///   widest number's, drawn against the slot's right edge as VectorCraft does, so names line up),
///   the name, the options icon as a trailing item, the lock as a toggle column.
///   The active row is the list's lead and shows the selected fill; picked rows the highlight.
///   Names wider than the row are elided by `ListRow`.
/// - Pointer, keys, rename and drag are `List`'s: click activates and picks; ⇧-click extends,
///   ⌘/Ctrl-click toggles; one Tab stop, ↑/↓ move the active row, F2 renames, Enter activates,
///   Delete and Backspace delete (claimed from the app's shortcuts while the list has focus), ⌘/Ctrl+] and [ move the picked rows, the Menu key and ⇧F10 open the menu.
/// - Footer: New and Delete are `PanelFooter::drop_button`s taking this list's `ListDrag`
///   (a row dropped on New duplicates the grabbed row; on Delete deletes the dragged rows).
/// - AccessKit: `List`'s ListBox named "Artboards"; the rename field is named "Name".
pub struct PageList<'a> {
    id_salt: egui::IdSalt,
    rows: &'a [PageListRow],
    picked: &'a mut Selection<usize>,
    active: usize,
    rename: Option<usize>,
    labels: Option<PageListLabels>,
}

impl<'a> PageList<'a> {
    /// A list of `rows` with `picked` as the selection and `active` as the page or artboard being edited.
    pub fn new(id_salt: impl egui::AsIdSalt, rows: &'a [PageListRow], picked: &'a mut Selection<usize>, active: usize) -> Self {
        PageList { id_salt: egui::IdSalt::new(id_salt), rows, picked, active, rename: None, labels: None }
    }

    /// The id the list keeps its memory under (`List::id_in` of the same salt).
    pub fn id_in(ui: &Ui, id_salt: impl egui::AsIdSalt) -> egui::Id {
        craft_ui::list::List::id_in(ui, id_salt)
    }

    /// Start renaming `row` in this pass (the app's Rename command), through `List::start_rename`.
    pub fn start_rename(mut self, row: usize) -> Self {
        self.rename = Some(row);
        self
    }

    /// Sets the translated labels; the default reads them from the interface locale.
    pub fn labels(mut self, labels: PageListLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // vectorcraft: crates/ui-egui/src/panels/artboards.rs:396 `show`, :297 `select_range`, :311
    // `toggle`, :243 `rename`, :371 `duplicate`.
    /// Paints the list and returns its response and the events for the app to apply.
    pub fn show(self, ui: &mut Ui) -> PageListOutput {
        let PageList { id_salt, rows, picked, active, rename, labels } = self;
        let labels = labels.unwrap_or_else(|| PageListLabels::of(ui));
        let tokens = Tokens::of(ui);
        let style = ListRow::<usize>::style(&tokens);
        let numbers = LocaleNumbers::of(ui);
        let list_id = List::id_in(ui, id_salt);
        if active < rows.len() {
            picked.lead = Some(active);
        }

        let keys: Vec<RowInfo<usize>> = (0..rows.len()).map(RowInfo::leaf).collect();
        let reserved = PanelFooter::reserved_height(&PanelFooter::style(&tokens, ui.pixels_per_point())) + ui.spacing().item_spacing.y;
        let focused = ui.memory(|m| m.has_focus(list_id));
        let delete_keys = [ClaimedKey::exact(Modifiers::NONE, Key::Delete), ClaimedKey::exact(Modifiers::NONE, Key::Backspace)];
        KeyClaim::claim(ui.ctx(), list_id, &delete_keys);
        let delete_key = focused && ui.input_mut(|i| delete_keys.iter().fold(false, |any, key| key.consume(i) | any));
        let number_width = {
            let widest = numbers.format(rows.len().max(1) as f64, 0..=0);
            ui.painter().layout_job(LayoutJob::single_section(widest, role_format(&tokens, style.detail_role, style.detail))).size().x
        };
        let option_side = IconButton::style(&tokens, ControlSize::Sm).side;

        let items = picked.items.clone();
        let number_rects = RefCell::new(Vec::new());
        let option_clicks = RefCell::new(Vec::new());
        let mut list = List::new(id_salt, &labels.artboards, keys.as_slice())
            .selection(&mut *picked)
            .renamable(true)
            .rename_draft(|row| (items.len() > 1 && items.contains(row)).then(String::new))
            .reorder(Reorder::new(Axis::Vertical))
            .max_height((ui.available_height() - reserved).max(0.0));
        if let Some(row) = rename {
            list = list.start_rename(row);
        }
        let output = list.show(ui, |at, &row| {
            let selected = items.contains(&row);
            let number = numbers.format(row as f64 + 1.0, 0..=0);
            let color = if selected { style.selected_detail } else { style.detail };
            let format = role_format(&tokens, style.detail_role, color);
            let (number_rects, option_clicks, options) = (&number_rects, &option_clicks, &labels.options);
            ListRow::new(row, rows[at].name.as_str())
                .rename_field_name(&labels.name)
                .highlight(selected && row != active)
                .toggle(ToggleColumn::lock(&labels.lock, rows[at].locked))
                .glyph_sized(number_width, move |painter, rect| {
                    number_rects.borrow_mut().push((row, rect));
                    let galley = painter.layout_job(LayoutJob::single_section(number, format));
                    let size = galley.size();
                    painter.galley(rect.right_center() - vec2(size.x, size.y / 2.0), galley, color);
                })
                .trailing_item(option_side, move |ui| {
                    let response = ui.add(IconButton::new(Icon::Settings, options).size(ControlSize::Sm));
                    if response.clicked() {
                        option_clicks.borrow_mut().push((row, response.double_clicked()));
                    }
                })
        });

        let mut events = Vec::new();
        let double_click = ui.input(|i| i.pointer.button_double_clicked(PointerButton::Primary).then(|| i.pointer.interact_pos())).flatten();
        let on_number =
            |row: usize| double_click.is_some_and(|at| number_rects.borrow().iter().any(|(r, rect): &(usize, Rect)| *r == row && rect.contains(at)));
        // A key that moves the lead picks the row without a click; it becomes the active row too.
        let mut moved = None;
        for event in output.events {
            if !matches!(event, ListEvent::Clicked(_)) {
                events.extend(moved.take().map(PageListEvent::Go));
            }
            match event {
                ListEvent::Selection(change) => {
                    moved = change.row.filter(|row| change.selection.contains(row));
                    events.push(PageListEvent::Selection(change));
                }
                ListEvent::Clicked(row) => {
                    moved = None;
                    events.push(PageListEvent::Go(row));
                }
                ListEvent::LeadMoved(row) => events.push(PageListEvent::Go(row)),
                ListEvent::Activate(row) if on_number(row) => events.push(PageListEvent::Fit(row)),
                ListEvent::Activate(row) => events.push(PageListEvent::Activate(row)),
                ListEvent::Row(RowEvent::Renamed { row, text }) => {
                    // VectorCraft renames only to a name with something in it.
                    let text = text.trim();
                    if !text.is_empty() {
                        events.push(PageListEvent::Rename { row, text: text.to_owned() });
                    }
                }
                ListEvent::Row(RowEvent::Toggle(edit)) => events.push(PageListEvent::Lock(edit)),
                ListEvent::Reorder(edit) => events.push(PageListEvent::Reorder(edit)),
                ListEvent::ContextMenu(request) => events.push(PageListEvent::ContextMenu(request)),
                _ => {}
            }
        }
        events.extend(moved.map(PageListEvent::Go));

        // A click on the active row's options icon opens them; on another row's it makes that row
        // active, so a double click opens its options.
        for (row, double) in option_clicks.into_inner() {
            events.push(if double || row == active { PageListEvent::Options(row) } else { PageListEvent::Go(row) });
        }

        let unlocked = |from: &[usize]| from.iter().copied().filter(|&row| rows.get(row).is_some_and(|r| !r.locked)).collect::<Vec<_>>();
        let deletable = unlocked(&picked.items);
        if delete_key && !deletable.is_empty() {
            events.push(PageListEvent::Delete(deletable.clone()));
        }

        PanelFooter::show(ui, |ui| {
            let mine = |drag: &ListDrag<usize>| drag.list == list_id;
            let (delete, dropped) = PanelFooter::drop_button::<ListDrag<usize>>(ui, Icon::Trash, &labels.delete, !deletable.is_empty(), mine);
            if let Some(drag) = dropped {
                let dropped = unlocked(&drag.items);
                if !dropped.is_empty() {
                    events.push(PageListEvent::Delete(dropped));
                }
            } else if delete.clicked() {
                events.push(PageListEvent::Delete(deletable));
            }
            let (new, dropped) = PanelFooter::drop_button::<ListDrag<usize>>(ui, Icon::NewFile, &labels.new_artboard, true, mine);
            if let Some(drag) = dropped {
                events.push(PageListEvent::New { duplicate: Some(drag.grabbed), options: false });
            } else if new.clicked() {
                events.push(PageListEvent::New { duplicate: None, options: ui.input(|i| i.modifiers.alt) });
            }
        });

        PageListOutput { response: output.response, events }
    }
}
