//! [`ReadingView`] (N3.20): Word's Read Mode: the document reflowed into full-height columns side
//! by side, turned one column at a time with the arrows, the wheel, the page-turn buttons or a
//! swipe, as a book.
//!
//! The app lays its text out into pages of [`reading_size`] and gives them to a
//! [`PageView`](super::view::PageView)-like strip; this module has the geometry and the turning.

use std::time::Duration;

use craft_ui::canvas::textures::{Priority, TextureCacheOptions, ThumbPaint, ThumbRequest, ThumbnailOptions, Thumbnails, texture_side};
use craft_ui::controls::{ControlSize, IconButton};
use craft_ui::direction;
use craft_ui::icons::Icon;
use craft_ui::localized::Labels as _;
use craft_ui::theme::{Tokens, TokensOf as _};
use egui::accesskit::Role;
use egui::{Event, EventFilter, Key, Modifiers, MouseWheelUnit, PointerButton, Pos2, Rect, Sense, Ui, Vec2, pos2, vec2};

use super::style::ReadingStyle;

/// One column per this much width, screen points.
const COLUMN_TARGET: f32 = 295.0;
/// The least column width, screen points.
const COLUMN_MIN: f32 = 192.0;
use super::view::PageSource;
use crate::layout::SCREEN_POINTS_PER_POINT;

/// How far a swipe along the columns must go to turn, as a share of a column's width.
const SWIPE_SHARE: f32 = 0.2;

/// The most turns one pass's wheel makes either way.
const MOST_TURNS: i32 = 5;

/// A touchpad turns a column per this many points of wheel, and the points it keeps are capped at
/// [`TOUCHPAD_CAP`].
const TOUCHPAD_TURN: f32 = 55.0;
const TOUCHPAD_CAP: f32 = 300.0;

/// The animation's time constant, 1/18 s, and the longest frame step it takes (a late frame never
/// overshoots).
const ANIMATION_RATE: f32 = 18.0;
const LONGEST_STEP: f32 = 0.1;

/// The offset within which an animation snaps to its column boundary, screen points.
const SNAP: f32 = 0.5;

/// The keys a focused view keeps: the arrows turn columns rather than move the focus.
const KEY_FILTER: EventFilter = EventFilter { horizontal_arrows: true, vertical_arrows: false, tab: false, escape: false };

/// The column widths Word's View › Column Width offers, page points (Narrow to Wide).
pub const REQUESTED_WIDTH: std::ops::RangeInclusive<f32> = 144.0..=612.0;

/// The narrowest column a chosen width gives way to on a narrow window, screen points
/// (WordCraft `reading_size`: `.max(144.0)` pixels, whatever the scale).
const NARROWEST_FIT: f32 = 144.0;

/// The most columns side by side (WordCraft).
pub const MAX_COLUMNS: usize = 5;

/// A window resize reflows the columns once the viewport has held still this long, seconds
/// (WordCraft's `RESIZE_SETTLE`).
pub const RESIZE_SETTLE: f64 = 0.16;

craft_ui::words! {
    /// The view's words (Word's Read Mode).
    #[non_exhaustive]
    pub struct ReadingLabels {
        previous: "reading.previous" = "Previous Screen";
        next: "reading.next" = "Next Screen";
    }
}

/// The reading zooms, as Word's View › Column Width offers them (WordCraft clamps 0.75–1.5).
pub const READING_ZOOM: std::ops::RangeInclusive<f32> = 0.75..=1.5;

/// The column size to lay out at, page points, for a viewport `avail` at `scale` screen points
/// per point. With a chosen column width (`requested`, within [`REQUESTED_WIDTH`] page points)
/// that width, at most the window less two gaps; otherwise about one column per
/// `style.column_target` screen points, 1 to [`MAX_COLUMNS`], each at least `style.column_min`.
/// A chosen width gives way to a window narrower than it, but not below `NARROWEST_FIT` screen
/// points. The height leaves the gap and `style.nav_bar` (WordCraft's `READ_NAV_BAR`, #1772).
// wordcraft: crates/ui-egui/src/canvas/reading.rs:49 `reading_size`.
pub fn reading_size(avail: Vec2, scale: f32, style: &ReadingStyle, requested: Option<f32>) -> (f32, f32) {
    let scale = if scale.is_finite() && scale > 0.0 { scale } else { 1.0 };
    let width = if avail.x.is_finite() { avail.x.max(0.0) } else { 0.0 };
    let height = if avail.y.is_finite() { avail.y.max(0.0) } else { 0.0 };
    let gap = style.column_gap;
    let column = match requested.filter(|points| points.is_finite()) {
        Some(points) => {
            let wanted = points.clamp(*REQUESTED_WIDTH.start(), *REQUESTED_WIDTH.end()) * scale;
            wanted.min((width - 2.0 * gap).max(NARROWEST_FIT))
        }
        None => {
            let count = (width / style.column_target).round().clamp(1.0, MAX_COLUMNS as f32);
            ((width - (count + 1.0) * gap) / count).max(style.column_min)
        }
    };
    (column / scale, (height - gap - style.nav_bar).max(style.column_min) / scale)
}

/// The part of `full` the columns scroll in: as many whole columns as fit, centred, so none is
/// cut at the right edge (#1772); half a point of overrun allowed before a column is dropped.
// wordcraft: crates/ui-egui/src/canvas/reading.rs:70 `reading_viewport`.
pub fn reading_viewport(full: Rect, column: f32, gap: f32, scale: f32) -> Rect {
    if !column.is_finite() || column <= 0.0 || !full.width().is_finite() {
        return full;
    }
    let step = column + gap;
    let slack = if scale.is_finite() { (SNAP * scale).clamp(0.0, 4.0) } else { 0.0 };
    // A hair of tolerance: columns sized to fill the view exactly come back from points as
    // 2.9999998 of them, and flooring that would drop a whole column.
    let whole = ((full.width() - gap) / (step - slack) + 1e-3).floor().max(1.0);
    let width = (whole * step + gap).min(full.width());
    Rect::from_min_size(pos2(full.center().x - width / 2.0, full.min.y), vec2(width, full.height()))
}

/// Whole wheel turns in this pass's events while the view is hovered: one notch is one column
/// (browsers send three lines a notch), touchpad pixels add up in `accum` to a turn every 55 pt;
/// at most 5 either way. ⌘/Ctrl + wheel is zoom, not a turn.
// wordcraft: crates/ui-egui/src/canvas/reading.rs:141 `read_wheel_turns`.
pub fn wheel_turns(events: &[Event], hovered: bool, accum: &mut f32) -> i32 {
    if !hovered {
        return 0;
    }
    let mut turns = 0i32;
    for event in events {
        let Event::MouseWheel { unit, delta, modifiers, .. } = event else { continue };
        if modifiers.command || modifiers.ctrl || !delta.is_finite() {
            continue;
        }
        let d = delta.x + delta.y;
        match unit {
            MouseWheelUnit::Line => {
                // Browsers send three lines for one notch.
                let notches = if cfg!(target_arch = "wasm32") { (d / 3.0).round() } else { d.round() };
                turns = turns.saturating_sub(notches.clamp(-(MOST_TURNS as f32), MOST_TURNS as f32) as i32);
                *accum = 0.0;
            }
            MouseWheelUnit::Page => {
                turns -= d.signum() as i32;
                *accum = 0.0;
            }
            MouseWheelUnit::Point => {
                *accum = (*accum + d).clamp(-TOUCHPAD_CAP, TOUCHPAD_CAP);
                if accum.abs() >= TOUCHPAD_TURN {
                    turns -= accum.signum() as i32;
                    // The excess is dropped: a large tick must not turn twice on the next event.
                    *accum = 0.0;
                }
            }
        }
    }
    turns.clamp(-MOST_TURNS, MOST_TURNS)
}

/// One animation step toward an exact column boundary (time constant 1/18 s); a late frame never
/// overshoots, and the last half point snaps.
// wordcraft: crates/ui-egui/src/canvas/reading.rs:130 `reading_animation_step`.
pub fn animation_step(current: f32, target: f32, dt: f32) -> f32 {
    if !current.is_finite() || !target.is_finite() {
        return target.max(0.0);
    }
    let t = 1.0 - (-ANIMATION_RATE * dt.clamp(0.0, LONGEST_STEP)).exp();
    let next = current + (target - current) * t;
    if (target - next).abs() <= SNAP { target } else { next }
}

/// The reading view's state, kept by the app.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ReadingState {
    /// The column at the left edge.
    pub column: usize,
    /// Where turns are going (several ticks queue columns rather than rounding a half-turned
    /// position).
    pub target: Option<usize>,
    /// The scroll position in screen points while a turn animates.
    pub offset: f32,
    /// The wheel turns not yet applied, in points; see `wheel_turns`.
    pub wheel: f32,
    /// The text at the start of the first column shown (app-defined position), kept across
    /// reflows instead of the editing caret.
    pub anchor: Option<u64>,
    /// Where the swipe in progress began (egui clears the press origin in the pass that
    /// releases the button, so the end of a swipe can't read it).
    swipe_from: Option<Pos2>,
}

/// The columns, built per pass over pages the app laid out at [`reading_size`].
///
/// - Columns `style.column_gap` apart (WordCraft's `GAP`) in a single row, scrolled
///   horizontally inside [`reading_viewport`]; the page-turn buttons (previous, next: craft-ui
///   `IconButton`s named by [`ReadingLabels`]) at the bottom end in the nav bar; ← / → /
///   Page Up / Page Down / space turn a column, Home / End go to the ends (one Tab stop: the
///   view; the buttons are their own).
/// - In a right-to-left interface the row mirrors: the first column is at the right, the nav
///   bar at the left with the chevrons swapped, and a swipe toward the left turns back, so
///   an arrow or swipe always moves toward the side it points at.
/// - A resize keeps the columns until [`RESIZE_SETTLE`] has passed, then the app reflows.
/// - The text stays selectable; the app's editing caret doesn't move the columns.
pub struct ReadingView<'a> {
    id_salt: egui::IdSalt,
    source: &'a dyn PageSource,
    state: &'a mut ReadingState,
    zoom: f32,
    name: Option<String>,
    labels: Option<ReadingLabels>,
}

/// What the view did this pass.
#[derive(Debug)]
pub struct ReadingViewResponse {
    #[allow(missing_docs)]
    pub response: egui::Response,
    /// The viewport has held still for [`RESIZE_SETTLE`] after a resize: the app reflows to a new
    /// [`reading_size`] in this pass. While it settles the view asks egui for one repaint at the
    /// settle time (`request_repaint_after`), never a timed poll.
    pub reflow: bool,
}

impl<'a> ReadingView<'a> {
    /// The view's sizes, from the tokens.
    pub fn style(tokens: &craft_ui::theme::UiTokens) -> ReadingStyle {
        ReadingStyle {
            column_gap: tokens.space.gap.section,
            nav_bar: tokens.size.control.md + 2.0 * tokens.space.pad.control,
            column_target: COLUMN_TARGET,
            column_min: COLUMN_MIN,
        }
    }

    /// A view of the pages of `source`, with `state` kept by the app between passes.
    pub fn new(id_salt: impl egui::AsIdSalt, source: &'a dyn PageSource, state: &'a mut ReadingState) -> Self {
        ReadingView { id_salt: egui::IdSalt::new(id_salt), source, state, zoom: 1.0, name: None, labels: None }
    }

    /// Sets the translated labels; the default reads them from the interface locale.
    pub fn labels(mut self, labels: ReadingLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Sets the page zoom; 1.0 is the default size.
    pub fn zoom(mut self, zoom: f32) -> Self {
        self.zoom = zoom;
        self
    }

    /// The document's name for the AccessKit Document node.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    // wordcraft: crates/ui-egui/src/canvas/reading.rs:25 `Settle::apply`, :119 `turn_read_page`,
    // :84 `place_read_pages`.
    /// Shows the columns and navigation and reports page turns and reflow requests.
    pub fn show(self, ui: &mut Ui) -> ReadingViewResponse {
        let ReadingView { id_salt, source, state, zoom, name, labels } = self;
        let ctx = ui.ctx().clone();
        let tokens = Tokens::of(ui);
        let style = Self::style(&tokens);
        let labels = labels.unwrap_or_else(|| ReadingLabels::of(ui));
        let rect = ui.available_rect_before_wrap();
        ui.advance_cursor_after_rect(rect);
        let id = ui.make_persistent_id(id_salt);
        let response = ui.interact(rect, id, Sense::click_and_drag() | Sense::FOCUSABLE);
        ctx.accesskit_node_builder(response.id, |node| {
            node.set_role(Role::Document);
            if let Some(name) = &name {
                node.set_label(name.as_str());
            }
        });
        let rtl = direction::is_rtl(ui);
        let forward = direction::forward(ui);
        let reflow = settled(&ctx, id, rect.size());
        let count = source.page_count();
        if count == 0 {
            return ReadingViewResponse { response, reflow };
        }
        let last = count - 1;
        let scale = screen_scale(zoom);
        let column = source.page_size(0).width as f32 * scale;
        let gap = style.column_gap;
        let step = column + gap;
        let viewport = reading_viewport(rect, column, gap, scale);

        let current = state.target.unwrap_or(state.column).min(last);
        let mut goal = None;
        let events = ui.input(|input| input.events.clone());
        let turns = wheel_turns(&events, response.hovered(), &mut state.wheel);
        if response.hovered() {
            ui.input_mut(|input| input.smooth_scroll_delta = Vec2::ZERO);
        }
        if response.drag_started_by(PointerButton::Primary) {
            state.swipe_from = response.interact_pointer_pos();
        }
        let swiped = if response.drag_stopped() {
            let end = response.interact_pointer_pos().or_else(|| ctx.input(|input| input.pointer.latest_pos()));
            swipe(state.swipe_from.take(), end, column, forward)
        } else {
            0
        };
        let mut turn = turns + swiped;
        if response.has_focus() {
            ui.memory_mut(|memory| memory.set_focus_lock_filter(response.id, KEY_FILTER));
            let (back, forward, start, end) = keys(ui, rtl);
            turn += forward - back;
            goal = if start {
                Some(0)
            } else if end {
                Some(last)
            } else {
                None
            };
        }
        let side = IconButton::style(&tokens, ControlSize::Md).side;
        let nav = Rect::from_min_max(pos2(rect.min.x, rect.max.y - style.nav_bar), rect.max);
        let next_x = if rtl { nav.min.x + gap } else { nav.max.x - gap - side };
        let next_at = Rect::from_min_size(pos2(next_x, nav.center().y - side / 2.0), vec2(side, side));
        let previous_at = next_at.translate(vec2(-forward * (side + ui.spacing().item_spacing.x), 0.0));
        let (previous_icon, next_icon) = if rtl { (Icon::ChevronRight, Icon::ChevronLeft) } else { (Icon::ChevronLeft, Icon::ChevronRight) };
        let previous = IconButton::new(previous_icon, labels.previous.as_str()).enabled(current > 0).show_at(ui, previous_at);
        let next = IconButton::new(next_icon, labels.next.as_str()).enabled(current < last).show_at(ui, next_at);
        turn += i32::from(next.clicked()) - i32::from(previous.clicked());
        if turn != 0 {
            goal = Some((current as i64 + i64::from(turn)).clamp(0, last as i64) as usize);
        }
        if goal.is_some() {
            state.target = goal;
        }

        match state.target {
            Some(target) => {
                let goal_offset = target.min(last) as f32 * step;
                let dt = ui.input(|input| input.stable_dt);
                state.offset = animation_step(state.offset, goal_offset, dt);
                state.column = ((state.offset / step).round().max(0.0) as usize).min(last);
                if state.offset == goal_offset {
                    state.column = target.min(last);
                    state.target = None;
                } else {
                    ctx.request_repaint();
                }
            }
            None => {
                state.column = state.column.min(last);
                state.offset = state.column as f32 * step;
            }
        }

        let thumbs = Thumbnails::<usize>::in_context(
            &ctx,
            (id_salt, "pictures"),
            ThumbnailOptions { max_side: texture_side(&ctx), cache: TextureCacheOptions::pages(), ..ThumbnailOptions::default() },
        );
        let ppp = ctx.pixels_per_point();
        let painter = ui.painter_at(viewport);
        let top = viewport.top() + gap;
        for page in 0..count {
            let size = source.page_size(page);
            let size = vec2(size.width as f32, size.height as f32) * scale;
            let along = gap + page as f32 * step - state.offset;
            let x = if rtl { viewport.right() - along - size.x } else { viewport.left() + along };
            let at = Rect::from_min_size(pos2(x, top), size);
            let shown = at.intersects(viewport);
            let near = !shown && [-step, step].into_iter().any(|by| at.translate(vec2(by, 0.0)).intersects(viewport));
            if !shown && !near {
                continue;
            }
            let priority = if shown { Priority::VISIBLE } else { Priority::NEAR };
            let request = ThumbRequest::for_cell(size.x.max(size.y), ppp);
            let picture = thumbs.state(&ctx, source, &page, &request, priority);
            if shown {
                thumbs.paint(&painter, at, &picture, &ThumbPaint::default().checker(false));
            }
        }
        ReadingViewResponse { response, reflow }
    }
}

/// Screen points per page point at `zoom`.
fn screen_scale(zoom: f32) -> f32 {
    zoom * SCREEN_POINTS_PER_POINT as f32
}

/// The columns a swipe from `start` to `end` turns (positive: forward), if it ran along the
/// columns for [`SWIPE_SHARE`] of a column. `forward` is 1 (columns run to the right) or -1
/// (right to left): a swipe against the columns' direction turns forward.
fn swipe(start: Option<Pos2>, end: Option<Pos2>, column: f32, forward: f32) -> i32 {
    match start.zip(end) {
        Some((start, end)) if (end.x - start.x).abs() >= column * SWIPE_SHARE && (end.x - start.x).abs() > (end.y - start.y).abs() => {
            if (end.x - start.x) * forward < 0.0 { 1 } else { -1 }
        }
        _ => 0,
    }
}

/// This pass's keys of a focused view: the turns back and forward they ask, and whether Home or
/// End was pressed. The arrows turn the other way in a right-to-left interface.
fn keys(ui: &Ui, rtl: bool) -> (i32, i32, bool, bool) {
    ui.input_mut(|input| {
        let mut count = |key| input.count_and_consume_key(Modifiers::NONE, key) as i32;
        let (left, right) = (count(Key::ArrowLeft), count(Key::ArrowRight));
        let (up, down, space) = (count(Key::PageUp), count(Key::PageDown), count(Key::Space));
        let (home, end) = (count(Key::Home) > 0, count(Key::End) > 0);
        let (before, after) = if rtl { (right, left) } else { (left, right) };
        (before + up, after + down + space, home, end)
    })
}

/// Whether the view has held a new size for [`RESIZE_SETTLE`] since it changed, once per change.
/// It asks for one repaint at the settle time while it waits.
fn settled(ctx: &egui::Context, id: egui::Id, size: Vec2) -> bool {
    let key = id.with("settle");
    let now = ctx.input(|input| input.time);
    let (mut seen, mut changed_at, mut pending): (Vec2, f64, bool) = ctx.data(|data| data.get_temp(key)).unwrap_or((size, now, false));
    if (seen - size).length_sq() > SNAP {
        (seen, changed_at, pending) = (size, now, true);
    }
    let mut reflow = false;
    if pending {
        let wait = RESIZE_SETTLE - (now - changed_at);
        if wait <= 0.0 {
            (reflow, pending) = (true, false);
        } else {
            ctx.request_repaint_after(Duration::from_secs_f64(wait));
        }
    }
    ctx.data_mut(|data| data.insert_temp(key, (seen, changed_at, pending)));
    reflow
}
