//! [`SwatchPalette`]: a grid of colour wells that is a radio group.

use craft_theme::{ControlSize, Srgba};
use egui::accesskit::Role;
use egui::{Align, AsIdSalt, Id, IdSalt, Key, Layout, Rect, Response, Sense, Ui, UiBuilder, Vec2};

use crate::behavior::{GridNav, RadioSelection};
use crate::controls::{Swatch, TooltipExt as _};
use crate::direction;
use crate::edit::{EditPolicy, EditResponse, Gesture};
use crate::theme::{Tokens, TokensOf as _};

/// A grid of [`Swatch`]es from the host's list of named colours, wrapping to the width, that
/// chooses one colour as a radio group does:
///
/// ```
/// use craft_theme::Srgba;
/// use craft_ui::composites::SwatchPalette;
/// # egui::__run_test_ui(|ui| { let mut color = Srgba::rgb(0.9, 0.3, 0.3);
/// let swatches = [("Red", Srgba::rgb(0.9, 0.3, 0.3)), ("Blue", Srgba::rgb(0.2, 0.4, 0.9))];
/// let picked = SwatchPalette::new("palette", "Swatches", color, swatches).show(ui);
/// if let Some(c) = picked.event.as_ref().and_then(|e| e.value()) {
///     color = *c;
/// }
/// # });
/// ```
///
/// - The swatch equal to `color` (compared in 8 bits, the first when several are) is selected.
///   Wells keep [`Swatch`]'s own room round them (6 pt between two), so the selection ring and
///   the focus ring show; the name is the tooltip.
/// - A click, Enter or Space on a swatch, or a screen reader's click, sets its colour as one
///   [`Set`](crate::edit::EditEvent::Set).
/// - The palette is one Tab stop: Tab enters on the selected swatch (the first when none is) and
///   the next Tab leaves. The arrows move the focus through the swatches as a grid, without
///   selecting: Left and Right along a row and on to the next or previous row (mirrored right to
///   left), Up and Down to the swatch in the same column of the row above or below, by the
///   layout the palette has wrapped to, Home and End to the first and last. In a dialog's
///   [`FieldTab`](crate::behavior::FieldTab) scope the palette is a non-field control: Ctrl+Tab
///   reaches it.
/// - AccessKit: a `RadioGroup` named `label` with the number of swatches; each swatch a
///   `RadioButton` named by its name, with the hex colour as its value. The focused swatch is the
///   focused node; a screen reader's Focus action on any swatch moves the palette's stop to it.
#[must_use = "show the palette with `show`"]
pub struct SwatchPalette {
    id_salt: IdSalt,
    label: String,
    color: Srgba,
    swatches: Vec<(String, Srgba)>,
    size: ControlSize,
    enabled: bool,
}

impl SwatchPalette {
    /// A palette named `label` (its accessible name) of `swatches`, each a name and a colour,
    /// choosing the host's `color`.
    pub fn new<S: Into<String>>(
        id_salt: impl AsIdSalt,
        label: impl Into<String>,
        color: Srgba,
        swatches: impl IntoIterator<Item = (S, Srgba)>,
    ) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            label: label.into(),
            color,
            swatches: swatches.into_iter().map(|(name, color)| (name.into(), color)).collect(),
            size: ControlSize::Md,
            enabled: true,
        }
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// A palette in table mode: small cells that touch, selected by index, for a colour table
    /// whose entries may repeat a colour. See [`SwatchTable`].
    pub fn table<S: Into<String>>(id_salt: impl AsIdSalt, label: impl Into<String>, swatches: impl IntoIterator<Item = (S, Srgba)>) -> SwatchTable {
        SwatchTable {
            id_salt: IdSalt::new(id_salt),
            label: label.into(),
            swatches: swatches.into_iter().map(|(name, color)| (name.into(), color)).collect(),
            selected: None,
            columns: None,
            enabled: true,
        }
    }

    /// Whether the palette takes input; a disabled one draws its swatches faded.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Shows the palette and returns this frame's edit: a [`Set`](crate::edit::EditEvent::Set) of
    /// the colour clicked. The response covers the swatches and is
    /// [changed](egui::Response::changed) with it.
    pub fn show(self, ui: &mut Ui) -> EditResponse<Srgba> {
        let id = ui.make_persistent_id(self.id_salt);
        let layout = if direction::is_rtl(ui) { Layout::right_to_left(Align::Min) } else { Layout::left_to_right(Align::Min) }.with_main_wrap(true);
        let builder = UiBuilder::new().scope_id(id).layout(layout).accessibility_role(Role::RadioGroup).accessibility_label(self.label.clone());
        let enabled = self.enabled && ui.is_enabled();
        let mut gesture = Gesture::load(ui, id, &self.color, EditPolicy::default()).enabled(enabled);
        let inner = ui.scope_builder(builder, |ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            let ids: Vec<_> = (0..self.swatches.len()).map(|i| (Swatch::id_in(ui, (id, i)), enabled)).collect();
            let selected = self.swatches.iter().position(|(_, c)| c.to_rgba() == self.color.to_rgba());
            let swatch_ids: Vec<Id> = ids.iter().map(|&(id, _)| id).collect();
            let stop = GridNav::roving_stop(ui, &swatch_ids, selected, enabled);
            let mut radios = RadioSelection::new(ids, selected);
            radios.report_size(ui, id);
            radios.click_from_input(ui);
            let chosen = radios.selected().filter(|_| radios.changed());
            if let Some(i) = chosen {
                gesture.set(self.swatches[i].1);
            }
            let shown = chosen.or(selected);
            let mut covered: Option<egui::Response> = None;
            for (i, (name, color)) in self.swatches.iter().enumerate() {
                let swatch = Swatch::new((id, i), name.as_str(), color.to_rgba())
                    .size(self.size)
                    .radio(shown == Some(i))
                    .position_in_set(i)
                    .tab_stop(i == stop)
                    .enabled(enabled);
                let response = ui.add(swatch).tooltip(name.as_str());
                covered = Some(match covered {
                    Some(all) => all.union(response),
                    None => response,
                });
            }
            (covered, chosen.is_some())
        });
        let (covered, changed) = inner.inner;
        let mut response =
            covered.unwrap_or_else(|| ui.interact(Rect::from_min_size(inner.response.rect.min, Vec2::ZERO), id.with("swatches"), Sense::hover()));
        if changed {
            response.mark_changed();
        }
        EditResponse::new(response, gesture.finish())
    }
}

/// The table mode of a [`SwatchPalette`], from [`SwatchPalette::table`]: a colour table such as a
/// GIF's, with one cell per entry.
///
/// ```
/// use craft_theme::Srgba;
/// use craft_ui::composites::SwatchPalette;
/// # egui::__run_test_ui(|ui| { let mut selected = None;
/// let entries = (0..256).map(|i| (format!("Index {i}"), Srgba::rgb(i as f32 / 255.0, 0.0, 0.5)));
/// let table = SwatchPalette::table("table", "Color Table", entries).selected(selected).columns(16).show(ui);
/// selected = table.selected.or(selected);
/// if let Some(index) = table.open {
///     // open the host's colour picker for entry `index`
/// }
/// # });
/// ```
///
/// - Cells are `swatch_palette.table_cell` points square and touch; each is a [`Swatch`] whose
///   colour is the entry's (a translucent entry shows over the checkerboard), and the name is the
///   tooltip. The cells wrap to the width, or to [`columns`](SwatchTable::columns).
/// - The host passes the selected index; the selected cell draws the selection ring inside its
///   outline. Entries are told apart by index, so duplicates select one cell.
/// - A click, Space or a screen reader's click reports the cell's index as
///   [`selected`](SwatchTableResponse::selected) when it is not the selected one. A double-click
///   or Enter reports [`open`](SwatchTableResponse::open). The table edits no colour.
/// - Keys are the palette's: one Tab stop (the selected cell, else the first) and the arrows
///   through the cells as a grid, without selecting.
/// - AccessKit: a `ListBox` named `label` with the number of cells; each cell a `ListBoxOption`
///   named by its name, selected when it is the selected one, with the hex colour as its value.
#[must_use = "show the table with `show`"]
pub struct SwatchTable {
    id_salt: IdSalt,
    label: String,
    swatches: Vec<(String, Srgba)>,
    selected: Option<usize>,
    columns: Option<usize>,
    enabled: bool,
}

impl SwatchTable {
    /// The selected entry, if any; an index past the last entry selects none.
    pub fn selected(mut self, selected: Option<usize>) -> Self {
        self.selected = selected;
        self
    }

    /// A fixed number of cells to a row (at least one) instead of wrapping to the width.
    pub fn columns(mut self, columns: usize) -> Self {
        self.columns = Some(columns.max(1));
        self
    }

    /// Whether the table takes input; a disabled one draws its cells faded.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Shows the table and returns this frame's selection and open request.
    pub fn show(self, ui: &mut Ui) -> SwatchTableResponse {
        let id = ui.make_persistent_id(self.id_salt);
        let layout = if direction::is_rtl(ui) { Layout::right_to_left(Align::Min) } else { Layout::left_to_right(Align::Min) }.with_main_wrap(true);
        let builder = UiBuilder::new().scope_id(id).layout(layout).accessibility_role(Role::ListBox).accessibility_label(self.label.clone());
        let enabled = self.enabled && ui.is_enabled();
        let cell = Tokens::of(ui).component.swatch_palette.table_cell;
        let selected = self.selected.filter(|&i| i < self.swatches.len());
        let inner = ui.scope_builder(builder, |ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            if let Some(columns) = self.columns {
                // Half a point of slack keeps the last cell of a row from wrapping on rounding.
                ui.set_max_width(columns as f32 * cell + 0.5);
            }
            let cell_ids: Vec<Id> = (0..self.swatches.len()).map(|i| Swatch::id_in(ui, (id, i))).collect();
            let stop = GridNav::roving_stop(ui, &cell_ids, selected, enabled);
            ui.ctx().accesskit_node_builder(id, |node| node.set_size_of_set(cell_ids.len()));
            let enter = ui.input(|i| i.key_pressed(Key::Enter));
            let (mut picked, mut open) = (None, None);
            let mut covered: Option<Response> = None;
            for (i, (name, color)) in self.swatches.iter().enumerate() {
                let swatch = Swatch::new((id, i), name.as_str(), color.to_rgba())
                    .table_cell(cell)
                    .selected(selected == Some(i))
                    .position_in_set(i)
                    .tab_stop(i == stop)
                    .enabled(enabled);
                let response = ui.add(swatch).tooltip(name.as_str());
                if response.double_clicked() || (response.clicked() && enter) {
                    open = Some(i);
                } else if response.clicked() && selected != Some(i) {
                    picked = Some(i);
                }
                covered = Some(match covered {
                    Some(all) => all.union(response),
                    None => response,
                });
            }
            (covered, picked, open)
        });
        let (covered, picked, open) = inner.inner;
        let mut response =
            covered.unwrap_or_else(|| ui.interact(Rect::from_min_size(inner.response.rect.min, Vec2::ZERO), id.with("swatches"), Sense::hover()));
        if picked.is_some() {
            response.mark_changed();
        }
        SwatchTableResponse { response, selected: picked, open }
    }
}

/// What [`SwatchTable::show`] returns.
#[derive(Debug)]
#[non_exhaustive]
#[must_use = "the host applies the selection and the open request"]
pub struct SwatchTableResponse {
    /// The cells; [changed](Response::changed) when `selected` is set.
    pub response: Response,
    /// The entry a click, Space or a screen reader's click selected this frame; none when it was
    /// the selected one already.
    pub selected: Option<usize>,
    /// The entry a double-click or Enter asked to edit this frame.
    pub open: Option<usize>,
}
