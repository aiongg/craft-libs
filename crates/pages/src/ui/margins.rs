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

use craft_ui::edit::{EditEvent, EditPolicy, Gesture};
use craft_ui::localized::units::{Length, LengthUnit, UnitContext};
use craft_ui::primitives::Interactive;
use craft_ui::theme::{Tokens, TokensOf as _};
use egui::accesskit::{Action, ActionData, Role};
use egui::{CursorIcon, EventFilter, Key, Modifiers, PointerButton, Rect, Sense, Ui};

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
}

/// The margin a drag of the visible `edge` to `at` points from the page's left (or top) edge
/// gives: snapped to `rules.snap` (none when 0), never negative, leaving `rules.min_text` between
/// it and the opposite margin (and the gutter). On an even page of mirrored margins the pointer
/// is reflected across the page and the edge swapped (WordCraft #2914).
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
    let m = &rules.margins;
    let (raw, room, current) = match edge {
        MarginEdge::Left => (at - rules.gutter, width - m.right - rules.gutter - rules.min_text, m.left),
        MarginEdge::Right => (width - at, width - m.left - rules.gutter - rules.min_text, m.right),
        MarginEdge::Top => (at, height - m.bottom - rules.min_text, m.top),
        MarginEdge::Bottom => (height - at, height - m.top - rules.min_text, m.bottom),
        // A header or footer sits within its page margin.
        MarginEdge::Header => (at, m.top, 0.0),
        MarginEdge::Footer => (height - at, m.bottom, 0.0),
    };
    if !raw.is_finite() || !room.is_finite() {
        return MarginChange { edge, points: current };
    }
    let snapped = if rules.snap > 0.0 { (raw / rules.snap).round() * rules.snap } else { raw };
    MarginChange { edge, points: snapped.clamp(0.0, room.max(0.0)) }
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
///   shows (its `RulersResponse`); the margins shade them with [`MarginHandlesStyle::shade`].
/// - Each edge grabs within [`MarginHandlesStyle::edge_grab`] of it on its ruler; the cursor is
///   ResizeHorizontal or ResizeVertical; the tooltip names it, and while dragging shows the value
///   in [`unit`](Self::unit) through craft-ui's length formatting (`LengthUnit`, its decimals and
///   suffix: "1.25 in", "3.18 cm").
/// - A drag is one edit gesture, [`EditEvent`]: Begin with the first value, Update per move,
///   Commit on release with [`dragged_margin`]'s value; Esc and focus loss Cancel. At most one
///   event a pass. The view doesn't follow the caret during the drag (the page would move under
///   the pointer).
/// - Keys: each edge is a Tab stop after the rulers (an AccessKit Splitter named by
///   [`MarginLabels`], its value the margin in the unit); ← / → (↑ / ↓ on the vertical ruler)
///   move it one [`snap`](Self::snap) step as one `EditEvent::Set`, ⇧ ten steps. Word reaches
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

    /// The handles' sizes and colours, from the tokens.
    pub fn style(tokens: &craft_ui::theme::UiTokens) -> MarginHandlesStyle {
        MarginHandlesStyle::of(tokens)
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

    /// Draw the margin shading on the two ruler strips (from the app's `RulersResponse`) and run
    /// the drags and keys.
    // wordcraft: crates/ui-egui/src/canvas/margins.rs:67 `margin_handle`; gridcraft:
    // crates/ui-egui/src/paper_view.rs:779 `ruler_margins`.
    pub fn show(self, ui: &mut Ui, horizontal: Rect, vertical: Rect) -> MarginResponse {
        let tokens = Tokens::of(ui);
        let style = Self::style(&tokens);
        let labels = self.labels.clone().unwrap_or_default();
        self.shade(ui, &style, horizontal, vertical);
        let mut edit = None;
        for visible in self.edges() {
            let strip = if visible.horizontal() { horizontal } else { vertical };
            let event = self.edge(ui, &style, &labels, visible, strip);
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
        MarginRules {
            page: self.page_size(),
            margins: self.margins,
            gutter: self.gutter,
            mirrored_even: self.mirrored_even,
            snap,
            min_text: self.min_text,
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

    /// The margin parts of the two strips, within the page.
    fn shade(&self, ui: &Ui, style: &MarginHandlesStyle, horizontal: Rect, vertical: Rect) {
        let text = |a: MarginEdge, b: MarginEdge| (self.screen(a, self.points(self.stored(a))), self.screen(b, self.points(self.stored(b))));
        let (left, right) = text(MarginEdge::Left, MarginEdge::Right);
        let (top, bottom) = text(MarginEdge::Top, MarginEdge::Bottom);
        let page = self.page.rect;
        let painter = ui.painter();
        for (min, max) in [(page.min.x, left), (right, page.max.x)] {
            painter.rect_filled(Rect::from_x_y_ranges(min..=max, horizontal.y_range()).intersect(horizontal), 0.0, style.shade);
        }
        for (min, max) in [(page.min.y, top), (bottom, page.max.y)] {
            painter.rect_filled(Rect::from_x_y_ranges(vertical.x_range(), min..=max).intersect(vertical), 0.0, style.shade);
        }
    }

    /// `points` in the display unit with its suffix ("1.25 in").
    fn shown(&self, points: f64) -> String {
        let value = UnitContext::default().convert(Length { value: points, unit: LengthUnit::Points }, self.unit).unwrap_or(points);
        format!("{value:.*} {}", self.unit.decimals(), self.unit.suffix())
    }

    /// One edge: its grab area on `strip`, drag, keys and screen reader requests.
    fn edge(
        &self,
        ui: &mut Ui,
        style: &MarginHandlesStyle,
        labels: &MarginLabels,
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
        let room = self.rules(0.0);
        let largest = dragged_margin(visible, if horizontal { room.page.0 } else { room.page.1 }, &room).points;
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
                delta += step * (requests(Action::Increment) - requests(Action::Decrement));
                let value = i.accesskit_action_requests(response.id, Action::SetValue).filter_map(|r| match r.data {
                    Some(ActionData::NumericValue(v)) if v.is_finite() => Some(v),
                    _ => None,
                });
                to = value.last();
            });
            let current = *gesture.shown();
            let wanted = match to {
                Some(value) => Some(dragged_margin(visible, value, &self.rules(0.0))),
                None if delta != 0.0 => Some(dragged_margin(visible, position(&current) + delta, &self.rules(self.snap))),
                None => None,
            };
            if let Some(value) = wanted {
                response.request_focus();
                gesture.set(value);
            }
        }
        let shown = *gesture.shown();
        let value_text = self.shown(shown.points);
        sensed.value(value_text.clone()).numeric(shown.points, 0.0..=largest, self.snap).finish(ui);
        if response.hovered() || response.dragged() {
            ui.ctx().set_cursor_icon(if horizontal { CursorIcon::ResizeHorizontal } else { CursorIcon::ResizeVertical });
        }
        if response.dragged() {
            egui::Tooltip::always_open(ui.ctx().clone(), ui.layer_id(), response.id.with("value"), egui::PopupAnchor::Pointer)
                .show(|ui| ui.label(format!("{name}: {value_text}")));
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
