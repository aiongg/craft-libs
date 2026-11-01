//! [`ColumnBar`] (N3.19): a bar split into column shares with draggable dividers, as PdfCraft's
//! Create Table of Fields sets its column widths: each column a share of the whole, a divider
//! between two columns moving only those two.

use craft_ui::direction;
use craft_ui::edit::{EditEvent, EditPolicy, Gesture};
use craft_ui::localized::Labels as _;
use craft_ui::primitives::Interactive;
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _, corner_radius, style_format};
use egui::accesskit::{Action, Role};
use egui::text::LayoutJob;
use egui::{CursorIcon, EventFilter, Key, Modifiers, PointerButton, Rect, Response, Sense, StrokeKind, Ui, pos2, vec2};

use super::style::ColumnBarStyle;

craft_ui::words! {
    /// The bar's words.
    #[non_exhaustive]
    pub struct ColumnBarLabels {
        divider: "columnBar.divider" = "Column {n} width";
        column: "columnBar.column" = "Column {letter}";
    }
}

/// What the bar did this pass.
#[derive(Debug)]
pub struct ColumnBarResponse {
    pub response: Response,
    /// A divider gesture or key: the new widths.
    pub edit: Option<EditEvent<Vec<f64>>>,
}

/// The smallest share a column keeps when a divider moves, or a quarter of what the two columns
/// have between them when that is less, so narrow columns can still be resized (PdfCraft's
/// `MIN_SHARE`).
pub const MIN_SHARE: f64 = 0.04;

/// The share an arrow key moves the focused divider by (PdfCraft: 0.01).
pub const KEY_STEP: f64 = 0.01;

/// `widths` (any unit) as shares of their sum; equal shares when the sum isn't positive and
/// finite.
// pdfcraft: crates/ui-egui/src/tools/field_table_ui.rs:100 `FieldTableDraft::shares`.
pub fn shares(widths: &[f64]) -> Vec<f64> {
    let total: f64 = widths.iter().sum();
    let equal = 1.0 / widths.len().max(1) as f64;
    widths.iter().map(|w| if total > 0.0 && total.is_finite() { w / total } else { equal }).collect()
}

/// Move the divider after column `i` by `delta` (a share of the whole): the two columns it
/// divides keep [`MIN_SHARE`] each (or a quarter of their pair), the others keep theirs. Never
/// panics on narrow pairs (`max` then `min`, never `clamp`).
// pdfcraft: crates/ui-egui/src/tools/field_table_ui.rs:109 `FieldTableDraft::move_divider`.
pub fn move_divider(widths: &mut [f64], i: usize, delta: f64) {
    let total: f64 = widths.iter().sum();
    let (Some(a), Some(b)) = (widths.get(i).copied(), widths.get(i + 1).copied()) else { return };
    if !(total > 0.0 && total.is_finite() && delta.is_finite()) {
        return;
    }
    let pair = (a + b) / total;
    let least = MIN_SHARE.min(pair / 4.0);
    let left = (a / total + delta).max(least).min(pair - least);
    widths[i] = left * total;
    widths[i + 1] = (pair - left) * total;
}

/// The bar over the app's `widths`; it reports edits, it never writes them.
///
/// - Look ([`ColumnBarStyle`]): the bar outlined, rounded; each divider a line, the active
///   divider (hovered, dragged or focused) in the accent. Labels (the app's, "A", "B", … by
///   default, [`ColumnBarLabels::column`]) are centred in their share when they fit.
/// - A divider grabs within the style's `grab` but never past the middle of a column; a drag is
///   one edit gesture, [`EditEvent`] (Begin, Update per move, Commit with the new widths; Esc
///   and focus loss Cancel), at most one event a pass.
/// - Keys: each divider is a Tab stop (an AccessKit Splitter named by
///   [`ColumnBarLabels::divider`], "Column {n} width"); ← / → move the focused one by
///   [`KEY_STEP`] as one [`EditEvent::Set`], mirrored in a right-to-left interface.
pub struct ColumnBar<'a> {
    id_salt: egui::IdSalt,
    widths: &'a [f64],
    names: Option<&'a [String]>,
    labels: Option<ColumnBarLabels>,
}

impl<'a> ColumnBar<'a> {
    pub fn new(id_salt: impl egui::AsIdSalt, widths: &'a [f64]) -> Self {
        ColumnBar { id_salt: egui::IdSalt::new(id_salt), widths, names: None, labels: None }
    }

    /// The bar's sizes and colours, from the tokens.
    pub fn style(tokens: &craft_ui::theme::UiTokens) -> ColumnBarStyle {
        ColumnBarStyle::of(tokens)
    }

    /// Column names drawn in their shares (the app's field names).
    pub fn names(mut self, names: &'a [String]) -> Self {
        self.names = Some(names);
        self
    }

    pub fn labels(mut self, labels: ColumnBarLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // pdfcraft: crates/ui-egui/src/tools/field_table_ui.rs:338 `column_bar`, :376 `column_dividers`.
    pub fn show(self, ui: &mut Ui) -> ColumnBarResponse {
        let tokens = Tokens::of(ui);
        let style = Self::style(&tokens);
        let labels = self.labels.clone().unwrap_or_else(|| ColumnBarLabels::of(ui));
        let (bar, response) = ui.allocate_exact_size(vec2(ui.available_width(), style.height), Sense::hover());
        let base = shares(self.widths);
        let at = column_edges(&base, bar);
        let mut live = None;
        let mut active = None;
        let mut edit = None;
        for i in 0..self.widths.len().saturating_sub(1) {
            let reach = style.grab.min(base[i].min(base[i + 1]) as f32 * bar.width() / 2.0);
            let grab = Rect::from_x_y_ranges((at[i] - reach)..=(at[i] + reach), bar.y_range());
            let name = labels.divider.replace("{n}", &(i + 1).to_string());
            let sensed = Interactive::with_id(ui.make_persistent_id((self.id_salt, i)), Role::Splitter, name)
                .sense(Sense::drag())
                .state_layer(false)
                .focus_ring(false)
                .interact_at(ui, grab);
            let handle = sensed.response().clone();
            let mut gesture = Gesture::load(ui, handle.id, &self.widths.to_vec(), EditPolicy::default()).event_filter(KEY_FILTER);
            let modifiers = ui.input(|input| input.modifiers);
            if handle.drag_started_by(PointerButton::Primary) {
                gesture.begin_with_value();
            }
            if handle.dragged_by(PointerButton::Primary)
                && let Some(mut moved) = gesture.anchor().cloned()
                && let Some((pos, origin)) = handle.interact_pointer_pos().zip(ui.input(|input| input.pointer.press_origin()))
            {
                move_divider(&mut moved, i, f64::from(pos.x - origin.x) / f64::from(bar.width()));
                gesture.update(moved, modifiers);
            }
            if handle.drag_stopped() {
                gesture.commit();
            }
            if !handle.dragged_by(PointerButton::Primary) {
                let steps = key_steps(ui, &handle, direction::is_rtl(ui));
                if steps != 0.0 {
                    let mut moved = gesture.shown().clone();
                    move_divider(&mut moved, i, steps * KEY_STEP);
                    handle.request_focus();
                    gesture.set(moved);
                }
            }
            if gesture.is_active() {
                live = Some(gesture.shown().clone());
            }
            let share = shares(gesture.shown()).get(i).copied().unwrap_or_default();
            sensed.numeric(share * 100.0, 0.0..=100.0, KEY_STEP * 100.0).finish(ui);
            if handle.hovered() || handle.dragged() {
                ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
            }
            if handle.hovered() || handle.dragged() || handle.has_focus() {
                active = Some(i);
            }
            edit = edit.or(gesture.finish());
        }
        let shown = shares(live.as_deref().unwrap_or(self.widths));
        let edges = column_edges(&shown, bar);
        self.paint(ui, &style, &tokens, bar, &edges, active);
        let response = match response.hover_pos().and_then(|at| edges.iter().position(|&x| at.x < x).or(edges.len().checked_sub(1))) {
            Some(column) if response.hovered() => {
                let text = self.name(column, &labels);
                response.on_hover_text(text)
            }
            _ => response,
        };
        ColumnBarResponse { response, edit }
    }

    /// Column `i`'s name: the app's, else its spreadsheet letter.
    fn name(&self, i: usize, labels: &ColumnBarLabels) -> String {
        match self.names.and_then(|names| names.get(i)) {
            Some(name) => name.clone(),
            None => labels.column.replace("{letter}", &letter(i)),
        }
    }

    fn paint(&self, ui: &Ui, style: &ColumnBarStyle, tokens: &craft_ui::theme::UiTokens, bar: Rect, edges: &[f32], active: Option<usize>) {
        let painter = ui.painter_at(bar);
        let format = style_format(tokens.families(), tokens.typography.label, tokens.color.text.normal.to_egui());
        let mut left = bar.left();
        for (i, &right) in edges.iter().enumerate() {
            let text = match self.names.and_then(|names| names.get(i)) {
                Some(name) => name.clone(),
                None => letter(i),
            };
            let galley = painter.layout_job(LayoutJob::single_section(text, format.clone()));
            if galley.size().x <= right - left {
                painter.galley(pos2((left + right) / 2.0, bar.center().y) - galley.size() / 2.0, galley, format.color);
            }
            left = right;
        }
        painter.rect_stroke(bar, corner_radius(style.radius), style.outline, StrokeKind::Inside);
        for (i, &x) in edges.iter().enumerate().take(edges.len().saturating_sub(1)) {
            let stroke = if active == Some(i) { style.divider_active } else { style.divider };
            painter.line_segment([pos2(x, bar.top() + style.outline.width), pos2(x, bar.bottom() - style.outline.width)], stroke);
        }
    }
}

/// The keys a focused divider keeps: the arrows move it rather than the focus.
const KEY_FILTER: EventFilter = EventFilter { horizontal_arrows: true, vertical_arrows: false, tab: false, escape: false };

/// The right edge of each column of `shares` across `bar`; the last is the bar's edge.
fn column_edges(shares: &[f64], bar: Rect) -> Vec<f32> {
    let mut x = bar.left();
    let last = shares.len().saturating_sub(1);
    shares
        .iter()
        .enumerate()
        .map(|(i, share)| {
            x += *share as f32 * bar.width();
            if i == last { bar.right() } else { x.min(bar.right()) }
        })
        .collect()
}

/// Column `i`'s spreadsheet letter: A to Z, then AA, AB, ….
fn letter(i: usize) -> String {
    let mut n = i + 1;
    let mut out = Vec::new();
    while n > 0 {
        n -= 1;
        out.push(char::from(b'A' + (n % 26) as u8));
        n /= 26;
    }
    out.iter().rev().collect()
}

/// The steps toward the right this pass's arrow presses and screen reader requests ask of the
/// divider `handle` (negative: left); an arrow counts while the divider has the focus.
fn key_steps(ui: &Ui, handle: &Response, rtl: bool) -> f64 {
    let sign = if rtl { -1.0 } else { 1.0 };
    let mut steps = 0.0;
    if handle.has_focus() {
        ui.input_mut(|input| {
            steps += sign
                * (input.count_and_consume_key(Modifiers::NONE, Key::ArrowRight) as f64
                    - input.count_and_consume_key(Modifiers::NONE, Key::ArrowLeft) as f64);
        });
    }
    ui.input(|input| {
        let requests = |action| input.num_accesskit_action_requests(handle.id, action) as f64;
        steps += requests(Action::Increment) - requests(Action::Decrement);
    });
    steps
}
