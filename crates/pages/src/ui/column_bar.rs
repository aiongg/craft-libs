//! [`ColumnBar`] (N3.19): a bar split into column shares with draggable dividers, as PdfCraft's
//! Create Table of Fields sets its column widths: each column a share of the whole, a divider
//! between two columns moving only those two.

use craft_ui::direction;
use craft_ui::edit::{EditEvent, EditPolicy, Gesture};
use craft_ui::localized::{Labels as _, LocaleNumbers, fmt};
use craft_ui::primitives::Interactive;
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _, corner_radius, role_format};
use egui::accesskit::{Action, ActionData, Role};
use egui::text::LayoutJob;
use egui::{CursorIcon, EventFilter, Key, Modifiers, PointerButton, Rect, Response, Sense, StrokeKind, Ui, pos2, vec2};

use super::style::ColumnBarStyle;

craft_ui::words! {
    /// The bar's words.
    #[non_exhaustive]
    pub struct ColumnBarLabels {
        divider: "columnBar.divider" = "Column {n} width";
        column: "columnBar.column" = "Column {letter}";
        percent: "columnBar.percent" = "{value}%";
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
///   divider (hovered, dragged or focused) in the accent. Each column paints its percentage
///   ([`ColumnBarLabels::percent`], PdfCraft `field_table_ui.rs:363`) when it is wider than the
///   style's `label_min`; its name (the app's, else [`ColumnBarLabels::column`]) is the hover text
///   and the description of the divider after it.
/// - A divider grabs within the style's `grab` but never past the middle of a column; a drag is
///   one edit gesture, [`EditEvent`] (Begin, Update per move, Commit with the new widths; Esc
///   and focus loss Cancel), at most one event a pass.
/// - Keys: each divider is a Tab stop (an AccessKit Splitter named by
///   [`ColumnBarLabels::divider`], "Column {n} width"); ← / → move the focused one by
///   [`KEY_STEP`] as one [`EditEvent::Set`]; a screen reader's SetValue sets the column's
///   percentage. In a right-to-left interface the bar mirrors (the first column is at the right),
///   so an arrow moves the divider toward the side it points at.
/// - Columns alternate the style's `fill` and `fill_alt` (PdfCraft tints them 0.11 and 0.055);
///   hovering a column names it, by the app's name or [`ColumnBarLabels::column`].
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
        use craft_ui::theme::ToEgui as _;
        let t = &tokens.component.pages;
        let c = &tokens.color;
        ColumnBarStyle {
            height: tokens.size.control.md,
            outline: egui::Stroke::new(t.column_bar_outline, c.line.control_edge.to_egui()),
            radius: tokens.shape.radius.control,
            divider: egui::Stroke::new(t.column_bar_divider, c.line.control_edge.to_egui()),
            divider_active: egui::Stroke::new(t.column_bar_divider_active, c.accent.solid.to_egui()),
            grab: t.column_bar_grab,
            fill: t.column_bar_fill.to_egui(),
            fill_alt: t.column_bar_fill_alt.to_egui(),
            label: craft_theme::TypeRole::Caption,
            label_min: t.column_bar_label_min,
        }
    }

    /// Column names (the app's field names) for the hover text and AccessKit.
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
        let numbers = LocaleNumbers::of(ui);
        let rtl = direction::is_rtl(ui);
        let forward = direction::forward(ui);
        let (bar, response) = ui.allocate_exact_size(vec2(ui.available_width(), style.height), Sense::hover());
        let base = shares(self.widths);
        let at = column_edges(&base, bar, rtl);
        let mut live = None;
        let mut active = None;
        let mut edit = None;
        for i in 0..self.widths.len().saturating_sub(1) {
            let reach = style.grab.min(base[i].min(base[i + 1]) as f32 * bar.width() / 2.0);
            let grab = Rect::from_x_y_ranges((at[i] - reach)..=(at[i] + reach), bar.y_range());
            let name = fmt(&labels.divider, &[("n", &numbers.format((i + 1) as f64, 0..=0))]);
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
                move_divider(&mut moved, i, f64::from((pos.x - origin.x) * forward) / f64::from(bar.width()));
                gesture.update(moved, modifiers);
            }
            if handle.drag_stopped() {
                gesture.commit();
            }
            if !handle.dragged_by(PointerButton::Primary) {
                let requested = requests(ui, &handle, rtl);
                let current = shares(gesture.shown()).get(i).copied().unwrap_or_default();
                let delta = match requested.value {
                    Some(percent) => percent / 100.0 - current,
                    None => requested.steps * KEY_STEP,
                };
                if delta != 0.0 {
                    let mut moved = gesture.shown().clone();
                    move_divider(&mut moved, i, delta);
                    handle.request_focus();
                    gesture.set(moved);
                }
            }
            if gesture.is_active() {
                live = Some(gesture.shown().clone());
            }
            let share = shares(gesture.shown()).get(i).copied().unwrap_or_default();
            sensed.numeric(share * 100.0, 0.0..=100.0, KEY_STEP * 100.0).finish(ui);
            let column_name = self.name(i, &labels);
            ui.ctx().accesskit_node_builder(handle.id, |node| node.set_description(column_name));
            if handle.hovered() || handle.dragged() {
                ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
            }
            if handle.hovered() || handle.dragged() || handle.has_focus() {
                active = Some(i);
            }
            edit = edit.or(gesture.finish());
        }
        let shown = shares(live.as_deref().unwrap_or(self.widths));
        let edges = column_edges(&shown, bar, rtl);
        self.paint(ui, &style, &tokens, &numbers, &labels, bar, (&shown, &edges), active);
        let response = match response.hover_pos().and_then(|at| edges.iter().position(|&x| (at.x - x) * forward < 0.0).or(edges.len().checked_sub(1)))
        {
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
            None => fmt(&labels.column, &[("letter", &letter(i))]),
        }
    }

    #[expect(clippy::too_many_arguments, reason = "one paint of the bar's parts")]
    fn paint(
        &self,
        ui: &Ui,
        style: &ColumnBarStyle,
        tokens: &craft_ui::theme::UiTokens,
        numbers: &LocaleNumbers,
        labels: &ColumnBarLabels,
        bar: Rect,
        (shares, edges): (&[f64], &[f32]),
        active: Option<usize>,
    ) {
        let painter = ui.painter_at(bar);
        let format = role_format(tokens, style.label, tokens.color.text.normal.to_egui());
        let mut start = direction::start_x(ui, bar);
        for (i, &end) in edges.iter().enumerate() {
            let column = Rect::from_x_y_ranges(start.min(end)..=start.max(end), bar.y_range());
            painter.rect_filled(column, 0.0, if i % 2 == 0 { style.fill } else { style.fill_alt });
            // PdfCraft paints each column's percentage when the column is wider than 32.
            let percent = numbers.format(shares.get(i).copied().unwrap_or_default() * 100.0, 0..=0);
            let text = fmt(&labels.percent, &[("value", &percent)]);
            let galley = painter.layout_job(LayoutJob::single_section(text, format.clone()));
            if column.width() > style.label_min && galley.size().x <= column.width() {
                painter.galley(column.center() - galley.size() / 2.0, galley, format.color);
            }
            start = end;
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

/// The far edge of each column of `shares` across `bar`, from its reading start (the left edge,
/// or the right edge right to left); the last is the bar's end.
fn column_edges(shares: &[f64], bar: Rect, rtl: bool) -> Vec<f32> {
    let sign = if rtl { -1.0 } else { 1.0 };
    let start = if rtl { bar.right() } else { bar.left() };
    let end = if rtl { bar.left() } else { bar.right() };
    let mut x = start;
    let last = shares.len().saturating_sub(1);
    shares
        .iter()
        .enumerate()
        .map(|(i, share)| {
            x += sign * *share as f32 * bar.width();
            if i == last { end } else { x.clamp(bar.left(), bar.right()) }
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

/// What this pass's arrow presses and screen reader requests ask of a divider.
struct Requested {
    /// Steps that grow the column before the divider (negative: shrink it): Increment and
    /// Decrement, and the arrows, which grow it when they point away from the reading start.
    steps: f64,
    /// A SetValue: the column's new percentage of the bar.
    value: Option<f64>,
}

/// The requests for the divider `handle`; an arrow counts while the divider has the focus.
fn requests(ui: &Ui, handle: &Response, rtl: bool) -> Requested {
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
        let count = |action| input.num_accesskit_action_requests(handle.id, action) as f64;
        steps += count(Action::Increment) - count(Action::Decrement);
        let value = input
            .accesskit_action_requests(handle.id, Action::SetValue)
            .filter_map(|request| match request.data {
                Some(ActionData::NumericValue(value)) if value.is_finite() => Some(value),
                _ => None,
            })
            .last();
        Requested { steps, value }
    })
}
