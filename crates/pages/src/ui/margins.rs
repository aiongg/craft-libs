//! [`MarginHandles`] (N3.11): the page margins as draggable edges on the rulers (Word, Excel's
//! Page Layout view): the grey part of each ruler is the margin, the white the text area; the
//! pointer over an edge shows a resize cursor and the margin's name, and a drag moves the margin
//! as one edit gesture (one undo step).
//!
//! ```ignore
//! let r = MarginHandles::new("margins", &placed, &setup).header_footer(Some((36.0, 36.0))).show(ui, top_ruler, left_ruler);
//! match r.edit {
//!     Some(EditEvent::Commit { value, .. }) => doc.set_margin(value.edge, value.points),
//!     Some(EditEvent::Update { value, .. }) => doc.preview_margin(value.edge, value.points),
//!     _ => {}
//! }
//! ```

use std::ops::{Range, RangeInclusive};

use craft_ui::edit::{EditEvent, EditPolicy, Gesture};
use craft_ui::localized::units::{Length, LengthUnit, UnitContext};
use craft_ui::localized::{Labels as _, LocaleNumbers, fmt};
use craft_ui::primitives::Interactive;
use craft_ui::theme::{Tokens, TokensOf as _};
use egui::accesskit::{Action, ActionData, Role};
use egui::{CursorIcon, EventFilter, Key, Modifiers, PointerButton, Rect, Sense, Ui};
use kurbo::Point;

use super::style::MarginHandlesStyle;
use super::view::PlacedPage;
use crate::setup::Margins;

/// The least text width (or height) a margin drag leaves, points: one inch (Word). GridCraft
/// leaves 36 pt; an app sets its own with [`MarginHandles::min_text`].
pub const MIN_TEXT: f64 = 72.0;

/// The step a dragged margin snaps to, points: 1/16 inch (Word). Alt drags freely.
pub const SNAP: f64 = 4.5;

craft_ui::words! {
    /// The edges' names, shown as tooltips and AccessKit names.
    #[non_exhaustive]
    pub struct MarginLabels {
        left: "pages.leftMargin" = "Left Margin";
        right: "pages.rightMargin" = "Right Margin";
        top: "pages.topMargin" = "Top Margin";
        bottom: "pages.bottomMargin" = "Bottom Margin";
        header: "pages.headerMargin" = "Header Margin";
        footer: "pages.footerMargin" = "Footer Margin";
        /// The drag tooltip: the edge's name and its value ("Left Margin: 1.25 in").
        drag: "pages.marginDrag" = "{name}: {value}";
    }
}

/// An edge a margin drag moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MarginEdge {
    Left,
    Right,
    Top,
    Bottom,
    /// The header's distance from the top edge.
    Header,
    /// The footer's distance from the bottom edge.
    Footer,
}

/// A margin's new value: the stored edge (on an even page of mirrored margins the visible left
/// edge is the stored right margin) and its points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MarginChange {
    pub edge: MarginEdge,
    pub points: f64,
}

/// What a margin drag works against: the page, its margins and the drag's rules.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MarginRules {
    /// Page width and height, points.
    pub page: (f64, f64),
    pub margins: Margins,
    pub gutter: f64,
    /// An even page of mirrored margins.
    pub mirrored_even: bool,
    /// The snap step, points; 0 for none.
    pub snap: f64,
    /// The least text width or height left, points.
    pub min_text: f64,
    /// The header's distance from the top edge, points: the top margin never drops below it (0
    /// with no header).
    pub header: f64,
    /// The footer's distance from the bottom edge, points: the bottom margin never drops below it.
    pub footer: f64,
}

/// The values the stored `edge` can take under `rules`, points: from 0 (or, for the top and
/// bottom margins, the header and footer distances) to the largest that leaves
/// `rules.min_text` of text and the gutter; a header or footer stays within its page margin.
pub fn margin_range(edge: MarginEdge, rules: &MarginRules) -> RangeInclusive<f64> {
    let (width, height) = rules.page;
    let m = &rules.margins;
    let (least, most) = match edge {
        MarginEdge::Left => (0.0, width - m.right - rules.gutter - rules.min_text),
        MarginEdge::Right => (0.0, width - m.left - rules.gutter - rules.min_text),
        MarginEdge::Top => (rules.header, height - m.bottom - rules.min_text),
        MarginEdge::Bottom => (rules.footer, height - m.top - rules.min_text),
        MarginEdge::Header => (0.0, m.top),
        MarginEdge::Footer => (0.0, m.bottom),
    };
    let least = least.max(0.0);
    least..=most.max(least)
}

/// The stored `edge` set to `points`: snapped to `rules.snap` (none when 0) and kept within
/// [`margin_range`]. A value that isn't finite leaves the edge as it is.
pub fn clamp_margin(edge: MarginEdge, points: f64, rules: &MarginRules) -> MarginChange {
    let m = &rules.margins;
    let current = match edge {
        MarginEdge::Left => m.left,
        MarginEdge::Right => m.right,
        MarginEdge::Top => m.top,
        MarginEdge::Bottom => m.bottom,
        MarginEdge::Header => rules.header,
        MarginEdge::Footer => rules.footer,
    };
    let range = margin_range(edge, rules);
    if !points.is_finite() || !range.end().is_finite() {
        return MarginChange { edge, points: current };
    }
    let snapped = if rules.snap > 0.0 { (points / rules.snap).round() * rules.snap } else { points };
    MarginChange { edge, points: snapped.clamp(*range.start(), *range.end()) }
}

/// The margin a drag of the visible `edge` to `at` points from the page's left (or top) edge
/// gives: snapped to `rules.snap` (none when 0), never negative, leaving `rules.min_text` between
/// it and the opposite margin (and the gutter). On an even page of mirrored margins the pointer
/// is reflected across the page and the edge swapped (WordCraft #2914). A header or footer stays
/// within its page margin, and the top and bottom margins stay beyond them.
// wordcraft: crates/ui-egui/src/canvas/margins.rs:37 `dragged_margin`, :54 `margin_drag`;
// gridcraft: crates/ui-egui/src/paper_view.rs:695 `margin_from_drag`.
pub fn dragged_margin(edge: MarginEdge, at: f64, rules: &MarginRules) -> MarginChange {
    let swapped = rules.mirrored_even && matches!(edge, MarginEdge::Left | MarginEdge::Right);
    let (edge, at) = match edge {
        MarginEdge::Left if swapped => (MarginEdge::Right, rules.page.0 - at),
        MarginEdge::Right if swapped => (MarginEdge::Left, rules.page.0 - at),
        edge => (edge, at),
    };
    let (width, height) = rules.page;
    let raw = match edge {
        MarginEdge::Left => at - rules.gutter,
        MarginEdge::Right => width - at,
        MarginEdge::Top | MarginEdge::Header => at,
        MarginEdge::Bottom | MarginEdge::Footer => height - at,
    };
    clamp_margin(edge, raw, rules)
}

/// What the handles did this pass.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MarginResponse {
    pub edit: Option<EditEvent<MarginChange>>,
}

/// The margin edges of one page (the page the caret is on, or the first page in view) on the
/// horizontal and vertical ruler strips.
///
/// - The strips are the ruler rects of craft-ui's [`Rulers`](craft_ui::canvas::Rulers) the app
///   shows (its `RulersResponse`). The rulers paint the margin shade themselves: the app gives
///   them [`spans`](Self::spans) through `Rulers::margins`.
/// - Each edge grabs within [`MarginHandlesStyle::edge_grab`] of it on its ruler; the cursor is
///   ResizeHorizontal or ResizeVertical; the tooltip names it, and while dragging shows the value
///   in [`unit`](Self::unit) through craft-ui's length formatting (`LengthUnit`, its decimals and
///   suffix: "1.25 in", "3.18 cm").
/// - A drag is one edit gesture, [`EditEvent`]: Begin with the first value, Update per move,
///   Commit on release with [`dragged_margin`]'s value; Esc and focus loss Cancel. At most one
///   event a pass. The view doesn't follow the caret during the drag (the page would move under
///   the pointer).
/// - Keys: each edge is a Tab stop after the rulers (an AccessKit Splitter named by
///   [`MarginLabels`], its value the margin in the unit, its range [`margin_range`]); ← / → (↑ /
///   ↓ on the vertical ruler) move the edge one [`snap`](Self::snap) step toward the arrow as one
///   `EditEvent::Set`, ⇧ ten steps. A screen reader's Increment and Decrement step the margin's
///   value (a larger margin for Increment, on every edge), SetValue sets it. Word reaches
///   margins from the keyboard only through Page Setup; the suite's rule is every control from
///   the keyboard.
pub struct MarginHandles<'a> {
    id_salt: egui::IdSalt,
    page: &'a PlacedPage,
    margins: Margins,
    gutter: f64,
    mirrored_even: bool,
    header_footer: Option<(f64, f64)>,
    snap: f64,
    min_text: f64,
    unit: LengthUnit,
    labels: Option<MarginLabels>,
}

impl<'a> MarginHandles<'a> {
    /// The handles of `page`, which has `margins`.
    pub fn new(id_salt: impl egui::AsIdSalt, page: &'a PlacedPage, margins: Margins) -> Self {
        MarginHandles {
            id_salt: egui::IdSalt::new(id_salt),
            page,
            margins,
            gutter: 0.0,
            mirrored_even: false,
            header_footer: None,
            snap: SNAP,
            min_text: MIN_TEXT,
            unit: LengthUnit::Inches,
            labels: None,
        }
    }

    /// The handles' sizes, from the tokens.
    pub fn style(tokens: &craft_ui::theme::UiTokens) -> MarginHandlesStyle {
        let c = &tokens.component;
        MarginHandlesStyle { edge_grab: c.pages.margin_handle_grab }
    }

    /// The unit the drag tooltip and the AccessKit value show (the app's ruler unit).
    pub fn unit(mut self, unit: LengthUnit) -> Self {
        self.unit = unit;
        self
    }

    pub fn gutter(mut self, gutter: f64) -> Self {
        self.gutter = gutter;
        self
    }

    /// The page is an even page of mirrored margins.
    pub fn mirrored_even(mut self, on: bool) -> Self {
        self.mirrored_even = on;
        self
    }

    /// The header and footer distances, as two more edges on the vertical ruler (Excel).
    pub fn header_footer(mut self, distances: Option<(f64, f64)>) -> Self {
        self.header_footer = distances;
        self
    }

    pub fn snap(mut self, points: f64) -> Self {
        self.snap = points;
        self
    }

    pub fn min_text(mut self, points: f64) -> Self {
        self.min_text = points;
        self
    }

    pub fn labels(mut self, labels: MarginLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The margin parts of the page as document ranges along the x axis and along the y axis,
    /// for `Rulers::margins`; `origin` is the page's top-left corner in the rulers' document
    /// space. The ranges follow the visible edges (an even page of mirrored margins swaps them).
    pub fn spans(&self, origin: Point) -> (Vec<Range<f64>>, Vec<Range<f64>>) {
        let (width, height) = self.page_size();
        let at = |edge: MarginEdge| self.position(edge, self.points(self.stored(edge)));
        let (left, right) = (at(MarginEdge::Left), at(MarginEdge::Right));
        let (top, bottom) = (at(MarginEdge::Top), at(MarginEdge::Bottom));
        (vec![origin.x..origin.x + left, origin.x + right..origin.x + width], vec![origin.y..origin.y + top, origin.y + bottom..origin.y + height])
    }

    /// Run the drags and keys of the edges on the two ruler strips (from the app's
    /// `RulersResponse`).
    // wordcraft: crates/ui-egui/src/canvas/margins.rs:67 `margin_handle`; gridcraft:
    // crates/ui-egui/src/paper_view.rs:779 `ruler_margins`.
    pub fn show(self, ui: &mut Ui, horizontal: Rect, vertical: Rect) -> MarginResponse {
        let tokens = Tokens::of(ui);
        let style = Self::style(&tokens);
        let labels = self.labels.clone().unwrap_or_else(|| MarginLabels::of(ui));
        let numbers = LocaleNumbers::of(ui);
        let mut edit = None;
        for visible in self.edges() {
            let strip = if visible.horizontal() { horizontal } else { vertical };
            let event = self.edge(ui, &style, &labels, &numbers, visible, strip);
            edit = edit.or(event);
        }
        MarginResponse { edit }
    }

    /// The edges on the rulers, in Tab order.
    fn edges(&self) -> Vec<MarginEdge> {
        let mut edges = vec![MarginEdge::Left, MarginEdge::Right, MarginEdge::Top, MarginEdge::Bottom];
        if self.header_footer.is_some() {
            edges.extend([MarginEdge::Header, MarginEdge::Footer]);
        }
        edges
    }

    /// The page's size, points.
    fn page_size(&self) -> (f64, f64) {
        let rect = self.page.rect;
        (f64::from(rect.width()) / self.page.scale, f64::from(rect.height()) / self.page.scale)
    }

    fn rules(&self, snap: f64) -> MarginRules {
        let (header, footer) = self.header_footer.unwrap_or_default();
        MarginRules {
            page: self.page_size(),
            margins: self.margins,
            gutter: self.gutter,
            mirrored_even: self.mirrored_even,
            snap,
            min_text: self.min_text,
            header,
            footer,
        }
    }

    /// The stored edge a visible one edits.
    fn stored(&self, visible: MarginEdge) -> MarginEdge {
        match visible {
            MarginEdge::Left if self.mirrored_even => MarginEdge::Right,
            MarginEdge::Right if self.mirrored_even => MarginEdge::Left,
            edge => edge,
        }
    }

    /// The stored value of `stored`, points.
    fn points(&self, stored: MarginEdge) -> f64 {
        let (header, footer) = self.header_footer.unwrap_or_default();
        match stored {
            MarginEdge::Left => self.margins.left,
            MarginEdge::Right => self.margins.right,
            MarginEdge::Top => self.margins.top,
            MarginEdge::Bottom => self.margins.bottom,
            MarginEdge::Header => header,
            MarginEdge::Footer => footer,
        }
    }

    /// Where the visible `edge` lies for a stored value of `points`: points from the page's left
    /// (horizontal edges) or top (vertical ones).
    fn position(&self, visible: MarginEdge, points: f64) -> f64 {
        let (width, height) = self.page_size();
        match visible {
            MarginEdge::Left if self.mirrored_even => points,
            MarginEdge::Left => points + self.gutter,
            MarginEdge::Right if self.mirrored_even => width - (points + self.gutter),
            MarginEdge::Right => width - points,
            MarginEdge::Top | MarginEdge::Header => points,
            MarginEdge::Bottom | MarginEdge::Footer => height - points,
        }
    }

    /// The screen coordinate along the edge's axis.
    fn screen(&self, visible: MarginEdge, points: f64) -> f32 {
        let origin = if visible.horizontal() { self.page.rect.min.x } else { self.page.rect.min.y };
        origin + (self.position(visible, points) * self.page.scale) as f32
    }

    /// `points` in the display unit with its suffix ("1.25 in"), the number in the locale's
    /// digits.
    fn shown(&self, numbers: &LocaleNumbers, points: f64) -> String {
        let value = UnitContext::default().convert(Length { value: points, unit: LengthUnit::Points }, self.unit).unwrap_or(points);
        format!("{} {}", numbers.format(value, self.unit.decimals()..=self.unit.decimals()), self.unit.suffix())
    }

    /// One edge: its grab area on `strip`, drag, keys and screen reader requests.
    fn edge(
        &self,
        ui: &mut Ui,
        style: &MarginHandlesStyle,
        labels: &MarginLabels,
        numbers: &LocaleNumbers,
        visible: MarginEdge,
        strip: Rect,
    ) -> Option<EditEvent<MarginChange>> {
        let stored = self.stored(visible);
        let host = MarginChange { edge: stored, points: self.points(stored) };
        let horizontal = visible.horizontal();
        let at = self.screen(visible, host.points);
        let grab = if horizontal {
            Rect::from_x_y_ranges((at - style.edge_grab)..=(at + style.edge_grab), strip.y_range())
        } else {
            Rect::from_x_y_ranges(strip.x_range(), (at - style.edge_grab)..=(at + style.edge_grab))
        };
        let name = visible.name(labels);
        let range = margin_range(stored, &self.rules(0.0));
        let sensed = Interactive::with_id(ui.make_persistent_id((self.id_salt, visible)), Role::Splitter, name)
            .sense(Sense::drag())
            .state_layer(false)
            .focus_ring(false)
            .interact_at(ui, grab);
        let response = sensed.response().clone();
        let mut gesture = Gesture::load(ui, response.id, &host, EditPolicy::default()).event_filter(KEY_FILTER);
        let modifiers = ui.input(|i| i.modifiers);
        let snap = if modifiers.alt { 0.0 } else { self.snap };
        let rules = self.rules(snap);
        let position = |value: &MarginChange| self.position(visible, value.points);
        if response.drag_started_by(PointerButton::Primary) {
            gesture.begin_with_value();
        }
        if response.dragged_by(PointerButton::Primary)
            && let Some(anchor) = gesture.anchor().copied()
            && let Some((pos, origin)) = response.interact_pointer_pos().zip(ui.input(|i| i.pointer.press_origin()))
        {
            let moved = if horizontal { pos.x - origin.x } else { pos.y - origin.y };
            gesture.update(dragged_margin(visible, position(&anchor) + f64::from(moved) / self.page.scale, &rules), modifiers);
        }
        if response.drag_stopped() {
            gesture.commit();
        }
        let dragging = response.dragged_by(PointerButton::Primary);
        if !dragging {
            let step = if self.snap > 0.0 { self.snap } else { SNAP };
            let mut delta = 0.0;
            let mut steps = 0.0;
            let mut to = None;
            if ui.memory(|m| m.has_focus(response.id)) {
                let keys = if horizontal { [Key::ArrowRight, Key::ArrowLeft] } else { [Key::ArrowDown, Key::ArrowUp] };
                ui.input_mut(|i| {
                    for (key, sign) in keys.into_iter().zip([1.0, -1.0]) {
                        let tens = i.count_and_consume_key(Modifiers::SHIFT, key) as f64;
                        let ones = i.count_and_consume_key(Modifiers::NONE, key) as f64;
                        delta += sign * step * (10.0 * tens + ones);
                    }
                });
            }
            ui.input(|i| {
                let requests = |action| i.num_accesskit_action_requests(response.id, action) as f64;
                steps = requests(Action::Increment) - requests(Action::Decrement);
                let value = i.accesskit_action_requests(response.id, Action::SetValue).filter_map(|r| match r.data {
                    Some(ActionData::NumericValue(v)) if v.is_finite() => Some(v),
                    _ => None,
                });
                to = value.last();
            });
            let current = *gesture.shown();
            let wanted = if let Some(value) = to {
                Some(clamp_margin(stored, value, &self.rules(0.0)))
            } else if steps != 0.0 || delta != 0.0 {
                let mut value = current;
                if steps != 0.0 {
                    value = clamp_margin(stored, value.points + step * steps, &self.rules(self.snap));
                }
                if delta != 0.0 {
                    value = dragged_margin(visible, position(&value) + delta, &self.rules(self.snap));
                }
                Some(value)
            } else {
                None
            };
            if let Some(value) = wanted {
                response.request_focus();
                gesture.set(value);
            }
        }
        let shown = *gesture.shown();
        let value_text = self.shown(numbers, shown.points);
        sensed.value(value_text.clone()).numeric(shown.points, range, self.snap).finish(ui);
        if response.hovered() || response.dragged() {
            ui.ctx().set_cursor_icon(if horizontal { CursorIcon::ResizeHorizontal } else { CursorIcon::ResizeVertical });
        }
        if response.dragged() {
            egui::Tooltip::always_open(ui.ctx().clone(), ui.layer_id(), response.id.with("value"), egui::PopupAnchor::Pointer)
                .show(|ui| ui.label(fmt(&labels.drag, &[("name", name), ("value", &value_text)])));
        } else {
            response.on_hover_text(name);
        }
        gesture.finish()
    }
}

/// The keys a focused edge keeps: the arrows step it rather than move the focus.
const KEY_FILTER: EventFilter = EventFilter { horizontal_arrows: true, vertical_arrows: true, tab: false, escape: false };

impl MarginEdge {
    /// Whether the edge is on the horizontal ruler.
    fn horizontal(self) -> bool {
        matches!(self, MarginEdge::Left | MarginEdge::Right)
    }

    fn name(self, labels: &MarginLabels) -> &str {
        match self {
            MarginEdge::Left => &labels.left,
            MarginEdge::Right => &labels.right,
            MarginEdge::Top => &labels.top,
            MarginEdge::Bottom => &labels.bottom,
            MarginEdge::Header => &labels.header,
            MarginEdge::Footer => &labels.footer,
        }
    }
}
