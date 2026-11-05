//! [`TableSizePicker`] (N3.24): Insert › Table's hover grid: hover a rows-by-columns square to
//! size the table, click to insert; the label above reads "4×3 Table" (columns × rows) or Insert
//! Table, and the page shows a live preview of the hovered table (Word).

use craft_ui::behavior::{GridKey, GridNav};
use craft_ui::direction;
use craft_ui::localized::{Labels as _, LocaleNumbers, fmt};
use craft_ui::primitives::{Interactive, Surface, SurfaceEdge, SurfaceKind};
use craft_ui::theme::{StrokeExt as _, ToEgui as _, Tokens, TokensOf as _, role_format, scale};
use egui::accesskit::Role;
use egui::{EventFilter, Key, Modifiers, Rect, Sense, Ui, UiBuilder, vec2};

/// The grid's size: 8 rows by 10 columns (Word and PowerPoint).
pub const ROWS: usize = 8;
/// The grid's width in squares.
pub const COLUMNS: usize = 10;

craft_ui::words! {
    /// The picker's words.
    #[non_exhaustive]
    pub struct TableSizePickerLabels {
        insert_table: "tableSize.insert" = "Insert Table";
        size: "tableSize.size" = "{cols}×{rows} Table";
    }
}

/// What the picker did: the hovered `(rows, columns)` for the live preview, and the size clicked
/// (the menu closes then).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TableSizeResponse {
    /// The `(rows, columns)` under the pointer or the focused square, for a live preview.
    pub hovered: Option<(usize, usize)>,
    /// The `(rows, columns)` clicked or chosen with Enter.
    pub picked: Option<(usize, usize)>,
}

/// The picker inside a menu, built per pass.
///
/// - The pointer hovers a square only on the square itself; the gap between squares hovers none
///   (WordCraft's picker).
/// - Squares up to the hovered one fill with the checked fill and an accent outline; others the
///   input fill and strong border.
/// - Squares of [`TableSizeStyle`](crate::ui::TableSizeStyle)'s cell and gap.
/// - Keys: the grid is one Tab stop; arrows move the hovered square through craft-ui's
///   [`GridNav`] (`GridNav::wrapped(ROWS * COLUMNS, COLUMNS)`, its
///   right-to-left mirroring), Enter inserts it, Esc closes the menu. It is an AccessKit Grid
///   whose value reads like the label.
pub struct TableSizePicker {
    id_salt: egui::IdSalt,
    labels: Option<TableSizePickerLabels>,
    preset: Option<(usize, usize)>,
}

impl TableSizePicker {
    /// The picker's sizes, from the tokens.
    pub fn style(tokens: &craft_ui::theme::UiTokens) -> crate::ui::TableSizeStyle {
        let t = &tokens.component.pages;
        crate::ui::TableSizeStyle { cell: t.table_size_cell, gap: t.table_size_gap, label: craft_theme::TypeRole::Body }
    }

    /// A picker with no preset size.
    pub fn new(id_salt: impl egui::AsIdSalt) -> Self {
        TableSizePicker { id_salt: egui::IdSalt::new(id_salt), labels: None, preset: None }
    }

    /// Sets the translated labels; the default reads them from the interface locale.
    pub fn labels(mut self, labels: TableSizePickerLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Shows the square at `(rows, columns)` hovered while no pointer is over the grid and the grid
    /// has no focus, as a story's or a test's stand-in for a pointer. Out-of-range sizes are
    /// clamped to the grid.
    pub fn hovered(mut self, size: (usize, usize)) -> Self {
        self.preset = Some((size.0.clamp(1, ROWS), size.1.clamp(1, COLUMNS)));
        self
    }

    // wordcraft: crates/ui-egui/src/dialogs/forms.rs:335 `table_grid_picker`; deckcraft:
    // crates/ui-egui/src/ribbon/insert.rs:136 `table_grid_picker`.
    /// Paints the grid and returns what was hovered and picked.
    pub fn show(self, ui: &mut Ui) -> TableSizeResponse {
        let tokens = Tokens::of(ui);
        let style = Self::style(&tokens);
        let labels = self.labels.clone().unwrap_or_else(|| TableSizePickerLabels::of(ui));
        let numbers = LocaleNumbers::of(ui);
        let step = style.cell + style.gap;
        let nav = GridNav::wrapped(ROWS * COLUMNS, COLUMNS).rtl(direction::is_rtl(ui));
        let id = Interactive::id_in(ui, self.id_salt);
        let lead_key = id.with("lead");
        let mut lead: Option<usize> = ui.data(|d| d.get_temp(lead_key)).flatten();

        // The label's row is reserved above the grid and filled once this pass's lead is known.
        let size = vec2(COLUMNS as f32 * step - style.gap, ROWS as f32 * step - style.gap);
        let label_format = role_format(&tokens, style.label, tokens.color.text.normal.to_egui());
        let (_, label_rect) = ui.allocate_space(vec2(size.x, label_format.line_height.unwrap_or(label_format.font_id.size)));
        let (_, rect) = ui.allocate_space(size);

        let focused = ui.memory(|m| m.has_focus(id));
        let mut close = false;
        if focused {
            ui.memory_mut(|m| m.set_focus_lock_filter(id, EventFilter { horizontal_arrows: true, vertical_arrows: true, ..Default::default() }));
            let from = lead.unwrap_or(0);
            ui.input_mut(|i| {
                for (key, grid_key) in [
                    (Key::ArrowRight, GridKey::Right),
                    (Key::ArrowLeft, GridKey::Left),
                    (Key::ArrowDown, GridKey::Down),
                    (Key::ArrowUp, GridKey::Up),
                    (Key::Home, GridKey::Home),
                    (Key::End, GridKey::End),
                ] {
                    if i.consume_key(Modifiers::NONE, key) {
                        lead = Some(nav.step(lead.unwrap_or(from), grid_key, None).index);
                    }
                }
                close = i.consume_key(Modifiers::NONE, Key::Escape);
            });
            lead.get_or_insert(from);
        }

        let forward = direction::forward(ui);
        let start = direction::start_x(ui, rect);
        // A point in the gap between squares is on no square (WordCraft's `table_grid_picker`
        // reports a hover only from a square's own response).
        let cell_at = |p: egui::Pos2| {
            let (across, down) = ((p.x - start) * forward, p.y - rect.top());
            let (column, row) = ((across / step).floor(), (down / step).floor());
            let on_square = across - column * step < style.cell && down - row * step < style.cell;
            let in_grid = column >= 0.0 && row >= 0.0 && (column as usize) < COLUMNS && (row as usize) < ROWS;
            (on_square && in_grid).then(|| row as usize * COLUMNS + column as usize)
        };
        let text = |lead: Option<usize>| match lead {
            Some(i) => {
                let count = |n: usize| numbers.format(n as f64, 0..=0);
                fmt(&labels.size, &[("cols", &count(i % COLUMNS + 1)), ("rows", &count(i / COLUMNS + 1))])
            }
            None => labels.insert_table.clone(),
        };

        let pointer_moved = ui.input(|i| i.pointer.delta() != vec2(0.0, 0.0));
        let response =
            Interactive::with_id(id, Role::Grid, labels.insert_table.as_str()).value(text(lead)).sense(Sense::click()).interact_at(ui, rect);
        let pointer = response.response().hover_pos().and_then(cell_at);
        match pointer {
            Some(cell) if pointer_moved || !focused => lead = Some(cell),
            None if !focused => lead = self.preset.map(|(rows, columns)| (rows - 1) * COLUMNS + columns - 1),
            _ => {}
        }
        let clicked = response.response().clicked();
        response.value(text(lead)).finish(ui);
        ui.data_mut(|d| d.insert_temp(lead_key, lead));

        let picked = if clicked { lead.map(|i| (i / COLUMNS + 1, i % COLUMNS + 1)) } else { None };
        if close {
            ui.close();
        }

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let hovered = lead.map(|i| (i / COLUMNS, i % COLUMNS));
            let accent = tokens.color.accent.solid.to_egui();
            let off = Surface::style(&tokens, SurfaceKind::Sunken);
            let mut on = off;
            on.fill = tokens.color.state.selected.to_egui();
            on.edge = SurfaceEdge::Border(scale::Stroke::W1.egui(painter.ctx(), accent));
            for row in 0..ROWS {
                for column in 0..COLUMNS {
                    let x = start + forward * column as f32 * step;
                    let cell = Rect::from_x_y_ranges(x.min(x + forward * style.cell)..=x.max(x + forward * style.cell), {
                        let y = rect.top() + row as f32 * step;
                        y..=y + style.cell
                    });
                    let chosen = hovered.is_some_and(|(r, c)| row <= r && column <= c);
                    painter.extend((if chosen { on } else { off }).shapes(cell, painter.pixels_per_point()));
                }
            }
        }
        ui.scope_builder(UiBuilder::new().max_rect(label_rect), |ui| {
            ui.label(egui::RichText::new(text(lead)).font(label_format.font_id.clone()).color(label_format.color))
        });
        TableSizeResponse { hovered: lead.map(|i| (i / COLUMNS + 1, i % COLUMNS + 1)), picked }
    }
}
