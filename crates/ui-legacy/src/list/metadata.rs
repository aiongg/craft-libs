//! [`MetadataTable`]: name and value rows in groups, with a filter field and an optional check
//! per field for the app's column picker.

use std::sync::Arc;

use egui::accesskit::Role;
use egui::text::LayoutJob;
use egui::{Align, AsIdSalt, Color32, CornerRadius, EventFilter, Id, IdSalt, Key, Layout, Modifiers, Rect, Response, Sense, Ui, UiBuilder, vec2};

use crate::behavior::{ClaimedKey, KeyClaim, ListKey, ListSelection, RowInfo, Selection, SelectionMode};
use crate::controls::checkbox::paint_box;
use crate::controls::{CheckState, Checkbox, SearchField};
use crate::localized::SharedLabels as _;
use crate::primitives::{Content, FocusRing, Interactive, Text};
use crate::theme::{ToEgui as _, Tokens, TokensOf as _, UiTokens, role_format};
use craft_theme::{ControlSize, TypeRole};

/// The most of the table the name column takes.
const NAME_SHARE: f32 = 1.0 / 3.0;
/// Rows shown before the table scrolls.
const MAX_ROWS: f32 = 18.0;

crate::words! {
    /// The table's words, English by default.
    #[non_exhaustive]
    pub struct MetadataLabels {
        /// The filter field's placeholder and accessible name (LightCraft's All Metadata).
        filter: "metadata.filter" = "Filter fields";
        /// The body with no rows at all.
        none: "metadata.none" = "No metadata found";
        /// The body when the filter hides every row.
        no_match: "metadata.noMatch" = "No fields match";
        /// The accessible name of a field's check in the column picker: `{name}` the field.
        show_field: "metadata.showField" = "Show {name}";
        /// The name of the filter field's clear button.
        clear_filter: "metadata.clearFilter" = "Clear";
        /// The accessible name of the table.
        table: "metadata.table" = "Metadata";
    }
}

/// One field: its name and its value, as the app formats them.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct MetadataRow {
    /// "Exposure Time", "dc:creator".
    pub name: String,
    /// "1/250 s"; may run to several lines, which wrap.
    pub value: String,
    /// What the column picker stores for this field, when that is not the name (the name
    /// may be translated or renamed).
    pub key: Option<String>,
    /// The column picker shows the field checked and its check can't be changed (FilmCraft's Name
    /// column).
    pub locked: bool,
}

impl MetadataRow {
    /// A row showing `name` and `value`, unlocked and with no storage key.
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self { name: name.into(), value: value.into(), key: None, locked: false }
    }

    /// The column picker stores `key` for this field instead of its name.
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// The column picker shows this field checked and can't change it.
    pub fn locked(mut self) -> Self {
        self.locked = true;
        self
    }

    /// What the column picker stores for this field: its key, else its name.
    pub fn picker_key(&self) -> &str {
        self.key.as_deref().unwrap_or(&self.name)
    }
}

/// A titled group of fields ("Camera", "EXIF", "XMP"); a group with an empty name has no title
/// line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetadataGroup {
    /// The group's title, or empty for a group without a title line.
    pub name: String,
    /// The group's fields, in display order.
    pub rows: Vec<MetadataRow>,
}

/// What the table reported this pass: a composite of the filter field, the checks and the rows.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct MetadataTableOutput {
    /// The rows: their focus and their rect.
    pub response: Response,
    /// A check of the column picker changed: the chosen keys have changed.
    pub chosen_changed: bool,
}

/// The table's sizes and colours, from [`MetadataTable::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct MetadataStyle {
    /// Between the name and value columns: `space.gap.section`.
    pub column_gap: f32,
    /// Between rows: `space.gap.tight`.
    pub row_gap: f32,
    /// Above a group's name, after the first group: `space.gap.row`.
    pub group_gap: f32,
    /// The rows' tallest height before they scroll: a fixed row count.
    pub max_height: f32,
    /// The fill of every other row: `ListStyle::stripe`.
    pub stripe: Color32,
    /// The names' text role, and the line height a row takes from it.
    pub name: TypeRole,
    /// The names' line height: the `name` role's.
    pub name_line: f32,
    /// The values' text role.
    pub value: TypeRole,
    /// The values' colour: `color.text.normal`.
    pub value_text: Color32,
    /// A group name's text role.
    pub group: TypeRole,
}

/// A key and value table, after LightCraft's All Metadata dialog, with FilmCraft's Metadata
/// Display column picker.
///
/// ```ignore
/// let out = MetadataTable::new("all-metadata", &groups).filter(&mut dialog.search).show(ui);
/// ```
///
/// - **Rows.** Each group's name on a line of its own in the `BodyStrong` role, its fields under
///   it in two columns: the name in the `Label` role and `text.muted`, the value in the `Body`
///   role, wrapping; striped rows (`ListStyle::stripe`), 16 points between the columns and 3 between
///   rows (LightCraft's grid spacing). The name column is as wide as the widest name, at most a
///   third of the table. A group's title has `space.gap.row` above it, except the first.
///   Values are selectable text, so the user copies part of one with the
///   platform's keys.
/// - **Filter** ([`filter`](Self::filter)): a [`SearchField`](crate::controls::SearchField) over
///   the rows keeps the fields whose name or value holds the typed text, ignoring case
///   ([`filter_metadata`]); a group with none left is hidden. The field keeps the focus while
///   the user types; ↓ moves into the rows.
/// - **Column picker** ([`choose`](Self::choose)): the box of a
///   [`Checkbox`](crate::controls::Checkbox) (painted by `paint_box` on an
///   [`Interactive`](crate::primitives::Interactive), as the row has its own focus stop) before
///   each field, checked while the field's [`picker_key`](MetadataRow::picker_key) is in
///   `chosen`; checking adds the key at the end, unchecking removes it. A
///   [locked](MetadataRow::locked) field shows checked and ignores clicks and Space. The app
///   shows the chosen fields as columns (FilmCraft's Project panel Metadata Display).
/// - **Empty.** [`MetadataLabels::none`] with no rows, [`MetadataLabels::no_match`] when the
///   filter hides them all, in `text.muted`.
/// - **Keys and AccessKit.** The rows are one Tab stop, a `Table` with a `Row` per field and
///   `Cell`s for name and value; ↑ ↓ Page Up Page Down Home End move the focused row and scroll
///   it into view ([`ListSelection::target`]). With
///   the column picker, Space toggles the focused row's check (claimed with
///   [`KeyClaim`](crate::behavior::KeyClaim), so the app's Space hand doesn't take it). The
///   filter's typing and the checks are handled in the pass they arrive, in order.
pub struct MetadataTable<'a> {
    id_salt: IdSalt,
    groups: &'a [MetadataGroup],
    filter: Option<&'a mut String>,
    chosen: Option<&'a mut Vec<String>>,
    max_height: Option<f32>,
    labels: Option<MetadataLabels>,
}

impl<'a> MetadataTable<'a> {
    /// A table of `groups` with no filter, no checks chosen and no height cap; `id_salt` keeps its state.
    pub fn new(id_salt: impl AsIdSalt, groups: &'a [MetadataGroup]) -> Self {
        Self { id_salt: IdSalt::new(id_salt), groups, filter: None, chosen: None, max_height: None, labels: None }
    }

    /// A filter field over the rows, editing `text` (the host keeps it, so it survives the
    /// dialog being redrawn).
    pub fn filter(mut self, text: &'a mut String) -> Self {
        self.filter = Some(text);
        self
    }

    /// A check before each field, for the keys in `chosen`.
    pub fn choose(mut self, chosen: &'a mut Vec<String>) -> Self {
        self.chosen = Some(chosen);
        self
    }

    /// The most the rows take before they scroll (LightCraft's dialog: 440).
    pub fn max_height(mut self, height: f32) -> Self {
        self.max_height = Some(height);
        self
    }

    /// Words other than the installed ones.
    pub fn labels(mut self, labels: MetadataLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // lightcraft: crates/ui-egui/src/dialogs.rs:662 all_metadata_body
    /// The table's sizes and colours.
    pub fn style(tokens: &UiTokens) -> MetadataStyle {
        let space = &tokens.space;
        MetadataStyle {
            column_gap: space.gap.section,
            row_gap: space.gap.tight,
            group_gap: space.gap.row,
            max_height: MAX_ROWS * tokens.size.control.md,
            stripe: crate::list::ListRow::<()>::style(tokens).stripe,
            name: TypeRole::Label,
            name_line: tokens.typography.get(TypeRole::Label).line_height,
            value: TypeRole::Body,
            value_text: tokens.color.text.normal.to_egui(),
            group: TypeRole::BodyStrong,
        }
    }

    // filmcraft: crates/ui-egui/src/panels/project_dialogs.rs:124 metadata_display
    /// Draws the table and returns what the user did this pass.
    pub fn show(self, ui: &mut Ui) -> MetadataTableOutput {
        let Self { id_salt, groups, filter, chosen, max_height, labels } = self;
        let labels = labels.map_or_else(|| MetadataLabels::shared(ui), Arc::new);
        let tokens = Tokens::of(ui);
        let scope = ui.scope_builder(UiBuilder::new().id_salt(id_salt), |ui| {
            let rows_id = ui.scope_id().with("rows");
            let mut query = String::new();
            if let Some(text) = filter {
                let edit = SearchField::new("filter", labels.filter.as_str(), text.as_str(), labels.clear_filter.as_str())
                    .placeholder(labels.filter.as_str())
                    .show(ui);
                if let Some(value) = edit.event.as_ref().and_then(|event| event.value()) {
                    text.clone_from(value);
                }
                if edit.response.has_focus() && ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::ArrowDown)) {
                    ui.memory_mut(|memory| memory.request_focus(rows_id));
                }
                query.clone_from(text);
            }
            let visible = filter_metadata(groups, &query);
            Body {
                rows_id,
                groups: &visible,
                chosen,
                max_height: max_height.unwrap_or(MetadataTable::style(&tokens).max_height),
                labels: &labels,
                tokens: &tokens,
            }
            .show(ui, !groups.iter().all(|g| g.rows.is_empty()))
        });
        MetadataTableOutput { response: scope.response, chosen_changed: scope.inner }
    }
}

/// The rows below the filter field.
struct Body<'a> {
    rows_id: Id,
    groups: &'a [MetadataGroup],
    chosen: Option<&'a mut Vec<String>>,
    max_height: f32,
    labels: &'a MetadataLabels,
    tokens: &'a crate::theme::UiTokens,
}

fn toggle(chosen: &mut Vec<String>, key: &str) {
    match chosen.iter().position(|c| c == key) {
        Some(at) => {
            chosen.remove(at);
        }
        None => chosen.push(key.to_owned()),
    }
}

impl Body<'_> {
    /// Draws the rows; true when a check changed `chosen`.
    fn show(self, ui: &mut Ui, any_fields: bool) -> bool {
        let Body { rows_id, groups, mut chosen, max_height, labels, tokens } = self;
        let style = MetadataTable::style(tokens);
        let total: usize = groups.iter().map(|g| g.rows.len()).sum();
        if total == 0 {
            let text = if any_fields { &labels.no_match } else { &labels.none };
            ui.add(Text::new(text.as_str()).content(Content::Muted));
            return false;
        }
        let flat_rows: Vec<&MetadataRow> = groups.iter().flat_map(|g| g.rows.iter()).collect();
        let names: Vec<&str> = flat_rows.iter().map(|r| r.name.as_str()).collect();
        let lead_key = rows_id.with("lead");
        let mut lead = ui.data(|d| d.get_temp::<usize>(lead_key)).unwrap_or(0).min(total - 1);
        let focused = ui.memory(|m| m.has_focus(rows_id));
        let mut changed = false;
        let mut scroll_to_lead = false;
        if focused && ui.is_enabled() {
            if chosen.is_some() {
                KeyClaim::claim(ui.ctx(), rows_id, &[ClaimedKey::exact(Modifiers::NONE, Key::Space)]);
            }
            let space = chosen.is_some() && ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Space));
            let order: Vec<RowInfo<usize>> = (0..total).map(RowInfo::leaf).collect();
            let lead_only = Selection::only(lead);
            let page = (ui.available_height().min(max_height) / (style.name_line + style.row_gap)) as usize;
            let steps = [Key::ArrowDown, Key::ArrowUp, Key::PageDown, Key::PageUp, Key::Home, Key::End].into_iter().filter_map(|key| {
                let pressed = ui.input_mut(|input| input.consume_key(Modifiers::NONE, key));
                pressed.then(|| ListKey::from_key(key, Modifiers::NONE, false)).flatten()
            });
            for key in steps.collect::<Vec<_>>() {
                if let ListKey::Select { step, .. } = key
                    && let Some(to) = ListSelection::new(SelectionMode::Single).target(&lead_only, order.as_slice(), step, page)
                {
                    lead = to;
                    scroll_to_lead = true;
                }
            }
            if space && let Some(chosen) = chosen.as_deref_mut() {
                let row = flat_rows[lead];
                if !row.locked {
                    toggle(chosen, row.picker_key());
                    changed = true;
                }
            }
        }

        let check_style = Checkbox::style(tokens, ControlSize::Md);
        let name_font = role_format(tokens, style.name, Color32::PLACEHOLDER);
        let value_format = role_format(tokens, style.value, style.value_text);
        let full_width = ui.available_width();
        let check_width = if chosen.is_some() { check_style.side + check_style.gap } else { 0.0 };
        let widest = names
            .iter()
            .map(|name| ui.fonts_mut(|f| f.layout_no_wrap((*name).to_owned(), name_font.font_id.clone(), Color32::PLACEHOLDER).size().x))
            .fold(0.0, f32::max);
        let name_width = widest.min(full_width * NAME_SHARE).ceil();
        let value_width = (full_width - check_width - name_width - style.column_gap).max(0.0);
        let stripe = style.stripe;
        let name_line = style.name_line;
        let rtl = crate::direction::is_rtl(ui);

        let builder = UiBuilder::new()
            .scope_id(rows_id)
            .sense(Sense::FOCUSABLE)
            .accessibility_role(Role::Table)
            .accessibility_label(labels.table.clone())
            .layout(Layout::top_down(Align::Min));
        let mut lead_node = None;
        let table = ui.scope_builder(builder, |ui| {
            egui::ScrollArea::vertical().id_salt("scroll").max_height(max_height).auto_shrink([false, true]).content_margin(0.0).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = style.row_gap;
                let mut flat = 0;
                for (group_at, group) in groups.iter().enumerate() {
                    if !group.name.is_empty() {
                        if group_at > 0 {
                            ui.add_space(style.group_gap);
                        }
                        ui.add(Text::new(group.name.as_str()).role(style.group));
                    }
                    for (at, row) in group.rows.iter().enumerate() {
                        let mut job = LayoutJob::single_section(row.value.clone(), value_format.clone());
                        job.wrap.max_width = value_width;
                        let value_height = ui.fonts_mut(|f| f.layout_job(job)).size().y;
                        let height = value_height.max(name_line).max(if chosen.is_some() { check_style.side } else { 0.0 });
                        let (rect, _) = ui.allocate_exact_size(vec2(full_width, height), Sense::hover());
                        if ui.is_rect_visible(rect) && at % 2 == 1 {
                            ui.painter().rect_filled(rect, CornerRadius::ZERO, stripe);
                        }
                        let slot = |from: f32, width: f32| {
                            let (min, max) = if rtl {
                                (rect.right() - from - width, rect.right() - from)
                            } else {
                                (rect.left() + from, rect.left() + from + width)
                            };
                            Rect::from_min_max(egui::pos2(min, rect.top()), egui::pos2(max, rect.bottom()))
                        };
                        let key = format!("{}/{}", group.name, row.name);
                        let row_scope =
                            ui.scope_builder(UiBuilder::new().id_salt(("row", &key)).max_rect(rect).accessibility_role(Role::Row), |ui| {
                                if let Some(chosen) = chosen.as_deref_mut() {
                                    let side = check_style.side;
                                    let box_slot = slot(0.0, side);
                                    let box_rect = Rect::from_center_size(box_slot.center(), vec2(side, side));
                                    let name = craft_i18n::fmt(&labels.show_field, &[("name", row.name.as_str())]);
                                    let sensed = Interactive::salted(IdSalt::new(("check", &key)), Role::CheckBox, name)
                                        .sense(Sense::CLICK)
                                        .enabled(!row.locked)
                                        .state_layer(false)
                                        .interact_at(ui, box_rect);
                                    let clicked = sensed.response().clicked();
                                    if clicked {
                                        toggle(chosen, row.picker_key());
                                        changed = true;
                                    }
                                    let state = CheckState::from(row.locked || chosen.iter().any(|c| c == row.picker_key()));
                                    let response = sensed.checked(state == CheckState::Checked).finish(ui);
                                    if ui.is_rect_visible(box_rect) {
                                        paint_box(ui, &check_style, box_rect, state, &response);
                                    }
                                }
                                ui.scope_builder(UiBuilder::new().max_rect(slot(check_width, name_width)).accessibility_role(Role::Cell), |ui| {
                                    ui.add(Text::new(row.name.as_str()).role(TypeRole::Label).content(Content::Muted).truncate());
                                });
                                ui.scope_builder(
                                    UiBuilder::new()
                                        .max_rect(slot(check_width + name_width + style.column_gap, value_width))
                                        .accessibility_role(Role::Cell),
                                    |ui| {
                                        ui.add(Text::new(row.value.as_str()));
                                    },
                                );
                                ui.scope_id()
                            });
                        let pressed_here = ui.input(|i| {
                            i.pointer.primary_pressed() && i.pointer.interact_pos().is_some_and(|p| rect.contains(p) && ui.clip_rect().contains(p))
                        });
                        if pressed_here {
                            lead = flat;
                            ui.memory_mut(|m| m.request_focus(rows_id));
                        }
                        if flat == lead {
                            lead_node = Some(row_scope.inner);
                            FocusRing::paint_if(ui, rect, CornerRadius::ZERO, focused);
                            if scroll_to_lead {
                                ui.scroll_to_rect(rect, None);
                            }
                        }
                        flat += 1;
                    }
                }
            });
        });
        ui.data_mut(|d| d.insert_temp(lead_key, lead));
        let bounds = table.response.rect;
        ui.ctx().accesskit_node_builder(rows_id, |node| {
            node.set_bounds(egui::accesskit::Rect {
                x0: bounds.min.x.into(),
                y0: bounds.min.y.into(),
                x1: bounds.max.x.into(),
                y1: bounds.max.y.into(),
            });
            node.remove_action(egui::accesskit::Action::Click);
            match lead_node.filter(|_| focused) {
                Some(id) => node.set_active_descendant(id.accesskit_id()),
                None => node.clear_active_descendant(),
            }
        });
        if focused {
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(rows_id, EventFilter { horizontal_arrows: true, vertical_arrows: true, tab: false, escape: false });
            });
        }
        changed
    }
}

/// The groups with only the fields whose name or value holds `query` (trimmed, any case); groups
/// left empty are dropped. An empty query keeps everything.
// lightcraft: crates/ui-egui/src/dialogs.rs:670 (keep)
pub fn filter_metadata(groups: &[MetadataGroup], query: &str) -> Vec<MetadataGroup> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return groups.to_vec();
    }
    let keeps = |row: &MetadataRow| row.name.to_lowercase().contains(&query) || row.value.to_lowercase().contains(&query);
    groups
        .iter()
        .filter_map(|group| {
            let rows: Vec<MetadataRow> = group.rows.iter().filter(|row| keeps(row)).cloned().collect();
            (!rows.is_empty()).then(|| MetadataGroup { name: group.name.clone(), rows })
        })
        .collect()
}
