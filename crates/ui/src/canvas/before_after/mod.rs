//! Before and after views of an edit: side by side, stacked, wipes and hold-to-show-original
//! ([`BeforeAfter`]).
//!
//! The layout and the divider are the library's; the pictures are the host's. The host fits its
//! image into a pane ([`BeforeAfter::panes`]), then [`show`](BeforeAfter::show) senses the divider
//! and calls the host's painter once per side, clipped to that side, and draws the divider line,
//! its knob and the captions over them.
//!
//! ```no_run
//! use craft_ui::canvas::before_after::{BeforeAfter, BeforeAfterMode, BeforeAfterSide};
//! # fn fit(pane: egui::Rect) -> egui::Rect { pane }
//! # fn demo(ui: &mut egui::Ui, area: egui::Rect, split: &mut f32) {
//! let compare = BeforeAfter::new("develop-compare", BeforeAfterMode::Split).fraction(*split).original_while(egui::Key::Backslash);
//! let [_, after_pane] = compare.panes(ui, area);
//! let image = fit(after_pane);
//! let out = compare.show(ui, area, image, |_painter, pane| match pane.side {
//!     BeforeAfterSide::Before => { /* paint the original into pane.image */ }
//!     BeforeAfterSide::After => { /* paint the edited image into pane.image */ }
//! });
//! // View state, not an undo step: keep the latest value.
//! if let Some(value) = out.fraction.and_then(|e| e.value().copied()) {
//!     *split = value;
//! }
//! # }
//! ```

pub(crate) mod stories;
mod style;
#[cfg(test)]
mod tests;

use craft_theme::TypeRole;
use egui::accesskit::{Action, ActionData, Role};
use egui::{Align2, AsIdSalt, CursorIcon, IdSalt, Key, Modifiers, Painter, PointerButton, Pos2, Rect, Response, Sense, Ui, WidgetInfo, pos2, vec2};

use crate::controls::slider::{KEY_FILTER, tap};
use crate::direction::{self, Step};
use crate::edit::{EditEvent, EditPolicy, Gesture};
use crate::focus::key_owner::canvas_keys_free;
use crate::icons::{self, Icon, IconSize};
use crate::localized::{Labels as _, LocaleNumbers};
use crate::primitives::{FocusRing, Interactive};
use crate::theme::{Tokens, TokensOf, UiTokens};

pub use style::BeforeAfterStyle;

/// How [`BeforeAfter`] lays out the two pictures (Lightroom's Before/After views, LightCraft's
/// `BeforeAfter`, FigCraft's compare modes).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BeforeAfterMode {
    /// Only the after picture; hold-to-show-original still works (LightCraft's Off).
    #[default]
    After,
    /// Only the before picture (LightCraft's Show Original).
    Original,
    /// Two panes in a row, before at the reading start, `before_after.gap_side` apart.
    SideBySide,
    /// Two panes, before above, `before_after.gap_stacked` apart.
    Stacked,
    /// One picture wiped by a vertical divider, before on the reading-start side (FigCraft's
    /// Overlay).
    Split,
    /// One picture wiped by a horizontal divider, before above.
    SplitStacked,
}

impl BeforeAfterMode {
    /// Every mode, in Lightroom's menu order after the single views.
    pub const ALL: [BeforeAfterMode; 6] = [
        BeforeAfterMode::After,
        BeforeAfterMode::Original,
        BeforeAfterMode::SideBySide,
        BeforeAfterMode::Stacked,
        BeforeAfterMode::Split,
        BeforeAfterMode::SplitStacked,
    ];

    /// Whether the mode shows two panes ([`SideBySide`](Self::SideBySide),
    /// [`Stacked`](Self::Stacked)).
    pub fn has_two_panes(self) -> bool {
        // lightcraft: crates/ui-egui/src/panels/detail.rs:157 (`split`).
        matches!(self, BeforeAfterMode::SideBySide | BeforeAfterMode::Stacked)
    }

    /// Whether the mode is a wipe with a divider ([`Split`](Self::Split),
    /// [`SplitStacked`](Self::SplitStacked)).
    pub fn has_divider(self) -> bool {
        // lightcraft: crates/ui-egui/src/panels/detail.rs:158 (`split_view`).
        matches!(self, BeforeAfterMode::Split | BeforeAfterMode::SplitStacked)
    }
}

/// Which picture a [`BeforeAfterPane`] holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BeforeAfterSide {
    Before,
    After,
}

/// Where one side is painted, in screen points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct BeforeAfterPane {
    pub side: BeforeAfterSide,
    /// The pane: half of the area (two panes) or the whole area.
    pub area: Rect,
    /// Where the side may draw: the pane, or its side of the divider in a wipe, cut to the area.
    pub clip: Rect,
    /// Where the picture goes: the host's `image` (fitted to the after pane) moved to this pane's
    /// centre for two panes, as it is otherwise.
    pub image: Rect,
}

/// What [`BeforeAfter::show`] reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct BeforeAfterOutput {
    /// The mode shown this pass: [`BeforeAfterMode::Original`] while the hold key is down, else
    /// the host's.
    pub mode: BeforeAfterMode,
    /// The before side; `None` in [`BeforeAfterMode::After`].
    pub before: Option<BeforeAfterPane>,
    /// The after side; `None` in [`BeforeAfterMode::Original`].
    pub after: Option<BeforeAfterPane>,
    /// A divider drag (`Begin`, `Update`, `Commit`; Esc cancels) or one key press's `Set`, as the
    /// fraction 0..=1 measured from the reading start (0: all after, 1: all before). View state:
    /// the host keeps the value and makes no undo step.
    pub fraction: Option<EditEvent<f32>>,
    /// The divider's knob in a wipe (a Tab stop), for focus requests.
    pub divider: Option<Response>,
}

crate::words! {
    /// The captions and AccessKit names of [`BeforeAfter`], English by default.
    #[non_exhaustive]
    pub struct BeforeAfterLabels {
        before: "beforeAfter.before" = "Before";
        after: "beforeAfter.after" = "After";
        divider: "beforeAfter.divider" = "Compare position";
    }
}

/// A before and after view of the host's two pictures with a draggable, keyboard-operable divider.
///
/// - Layout ([`panes`](Self::panes)): two panes split the area minus `before_after.gap_side`
///   across (side by side) or `before_after.gap_stacked` down (stacked); before is at the reading
///   start, so it is on the right in a right-to-left interface. A wipe or a single view has one
///   pane, the whole area. With [`captions`](Self::captions) each pane gives up a caption strip at
///   its top (`type.label` line height plus `before_after.caption_gap`).
/// - Painting ([`show`](Self::show)): the host's painter runs for before, then after, each with a
///   painter clipped to [`BeforeAfterPane::clip`]; then the divider line (`before_after.line`
///   over an `overlay.halo` halo, the full height or width of the shown picture), the knob
///   (`before_after.knob` across, `accent.strong`, a two-way arrow in `text.on_strong`) at the
///   line's middle, and the captions (`type.label`, `text.muted`) above each pane, or at the two
///   ends of the picture's top in a wipe.
/// - Divider, wipes only: a press on the picture (or only near the line, with
///   [`drag_anywhere`](Self::drag_anywhere) off) moves the divider to the pointer and drags it,
///   with the `ResizeHorizontal` or `ResizeVertical` pointer. The fraction runs across the shown
///   part of the picture (the picture cut to the area), from the reading start: in a
///   right-to-left interface 0 is the right edge. The picture itself never mirrors.
/// - Keys, knob focused (one Tab stop): ← and → (↑ and ↓ for a horizontal divider) move it 1 %,
///   with ⇧ 10 %, Home to 0 % and End to 100 %; in a right-to-left interface ← moves toward the end.
///   Each press is one `Set`.
/// - Hold to show the original ([`original_while`](Self::original_while)): while the key is down
///   and the keyboard is the app's (no text field, menu or modal: `focus::key_owner`), the view
///   is [`BeforeAfterMode::Original`]; the host's mode comes back on release and nothing is stored.
/// - AccessKit: the knob is a `Slider` named [`BeforeAfterLabels::divider`] with the percentage
///   as its value; each pane is an `Image` named by its caption word.
pub struct BeforeAfter {
    id_salt: IdSalt,
    mode: BeforeAfterMode,
    fraction: f32,
    original_key: Option<Key>,
    captions: bool,
    drag_anywhere: bool,
    labels: Option<BeforeAfterLabels>,
}

impl BeforeAfter {
    /// A view in `mode`, divider in the middle.
    pub fn new(id_salt: impl AsIdSalt, mode: BeforeAfterMode) -> Self {
        Self { id_salt: IdSalt::new(id_salt), mode, fraction: 0.5, original_key: None, captions: false, drag_anywhere: true, labels: None }
    }

    /// The divider's place, 0..=1 from the reading start (0: all after, 1: all before); clamped.
    pub fn fraction(mut self, fraction: f32) -> Self {
        self.fraction = fraction;
        self
    }

    /// Shows only the original while `key` is held (LightCraft's Backslash).
    pub fn original_while(mut self, key: Key) -> Self {
        self.original_key = Some(key);
        self
    }

    /// Shows the "Before" and "After" captions above the panes; off by default.
    pub fn captions(mut self, on: bool) -> Self {
        self.captions = on;
        self
    }

    /// Whether a press anywhere on the picture moves the divider (FigCraft's compare); true by
    /// default. Off, only a press within `before_after.knob` / 2 of the line does, so the host's
    /// own drag (LightCraft's loupe pans) keeps the rest.
    pub fn drag_anywhere(mut self, on: bool) -> Self {
        self.drag_anywhere = on;
        self
    }

    /// The captions and AccessKit names; the installed words by default.
    pub fn labels(mut self, labels: BeforeAfterLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The `[before, after]` pane rects of this view in `area` (the same rect twice for a wipe or
    /// a single view), with the caption strips taken off when captions are on. The host fits its
    /// picture into the after pane and passes that rect to [`show`](Self::show).
    pub fn panes(&self, ui: &Ui, area: Rect) -> [Rect; 2] {
        // lightcraft: crates/ui-egui/src/panels/detail.rs:475-494 `view_areas` (half the width minus 6,
        // half the height minus 14: gaps 12 and 28); figcraft: crates/ui-egui/src/panels/dev_compare.rs:320-323
        // (room for the labels above). Gaps from `BeforeAfter::style`; RTL swaps the side-by-side
        // panes.
        let tokens = Tokens::of(ui);
        let style = Self::style(&tokens);
        self.rects(self.shown_mode(ui), area, &style, self.strip(&tokens, &style), direction::is_rtl(ui))
    }

    /// Lays out, senses the divider and the hold key, runs `paint` for each shown side with a
    /// painter clipped to it, then draws the divider, the knob and the captions over them. `image`
    /// is the picture's rect fitted to the after pane ([`panes`](Self::panes)); a wipe uses it
    /// as it is for both sides.
    pub fn show(self, ui: &mut Ui, area: Rect, image: Rect, mut paint: impl FnMut(&Painter, &BeforeAfterPane)) -> BeforeAfterOutput {
        // lightcraft: crates/ui-egui/src/panels/detail.rs:245 (Backslash shows the original), :476-494
        // (pane rects), :430-449 `draw_wipe_before` (before on its side of the line), :637-647
        // `draw_wipe_line`; crates/ui-egui/src/region.rs:379-400 `wipe_line`, `window_clip`.
        // figcraft: crates/ui-egui/src/panels/dev_compare.rs:336-358 (one placement, before clipped to
        // the split, a click or drag anywhere on the picture sets it, `ResizeHorizontal`, 2 pt line,
        // 20 pt knob with a two-way glyph).
        // new: keys, RTL and AccessKit as the type's doc says; the look from `BeforeAfter::style`.
        let tokens = Tokens::of(ui);
        let style = Self::style(&tokens);
        let labels = self.labels.clone().unwrap_or_else(|| BeforeAfterLabels::of(ui));
        let (mode, rtl, enabled) = (self.shown_mode(ui), direction::is_rtl(ui), ui.is_enabled());
        let strip = self.strip(&tokens, &style);
        let rects = self.rects(mode, area, &style, strip, rtl);
        let host = if self.fraction.is_finite() { self.fraction.clamp(0.0, 1.0) } else { 0.5 };
        let base = ui.make_persistent_id(self.id_salt);
        let first = Sides::of(mode, rects, image, host, rtl);

        // The pane nodes come first so the picture's drag is the later, top widget.
        for (pane, name, side) in [(&first.before, labels.before.as_str(), "before"), (&first.after, labels.after.as_str(), "after")] {
            if let Some(pane) = pane {
                ui.interact(pane.clip, base.with(side), Sense::hover()).widget_info(|| WidgetInfo::labeled(Role::Image, enabled, name));
            }
        }

        let mut fraction = host;
        let mut event = None;
        let mut divider = None;
        let mut knob = None;
        if let Some(wipe) = first.wipe {
            let knob_rect = Rect::from_center_size(wipe.knob_centre(), vec2(style.knob, style.knob));
            let sensed = Interactive::salted(self.id_salt, Role::Slider, labels.divider.as_str())
                .sense(Sense::focusable_noninteractive())
                .enabled(enabled)
                .state_layer(false)
                .focus_ring(false)
                .interact_at(ui, knob_rect);
            let response = sensed.response().clone();
            let reach = if self.drag_anywhere { wipe.shown } else { wipe.reach(style.knob) };
            let options = egui::InteractOptions { enabled, ..Default::default() };
            let picture = ui.interact_opt(reach, base.with("picture"), Sense::drag(), options);

            let mut gesture = Gesture::load(ui, response.id, &host, EditPolicy::default()).event_filter(KEY_FILTER).enabled(enabled);
            let tapped = tap(ui, &picture);
            if picture.drag_started_by(PointerButton::Primary) {
                response.request_focus();
                gesture.begin();
            }
            if picture.dragged_by(PointerButton::Primary)
                && gesture.is_active()
                && let Some(pos) = picture.interact_pointer_pos()
            {
                gesture.update(wipe.fraction_at(pos, rtl), ui.input(|i| i.modifiers));
            }
            if picture.drag_stopped() {
                gesture.commit();
            }
            if let Some(pos) = tapped {
                response.request_focus();
                gesture.set(wipe.fraction_at(pos, rtl));
            }
            if enabled && !picture.dragged() {
                let focused = ui.memory(|m| m.has_focus(response.id));
                if let Some(to) = self.key_target(ui, response.id, focused, wipe.stacked, *gesture.shown()) {
                    gesture.set(to);
                }
            }
            if enabled && (picture.hovered() || picture.dragged()) {
                ui.ctx().set_cursor_icon(if wipe.stacked { CursorIcon::ResizeVertical } else { CursorIcon::ResizeHorizontal });
            }
            fraction = *gesture.shown();
            event = gesture.finish();
            knob = Some((sensed, wipe.stacked));
        }

        let sides = Sides::of(mode, rects, image, fraction, rtl);
        for pane in [&sides.before, &sides.after].into_iter().flatten() {
            paint(&ui.painter_at(pane.clip), pane);
        }
        if self.captions {
            self.paint_captions(ui, &style, &labels, &sides, strip);
        }
        if let (Some(wipe), Some((sensed, stacked))) = (sides.wipe, knob) {
            let percent = (fraction * 100.0).round();
            let text = format!("{}%", LocaleNumbers::of(ui).format(f64::from(percent), 0..=0));
            let response = sensed.numeric(f64::from(percent), 0.0..=100.0, 1.0).value(text).finish(ui);
            let painter = ui.painter();
            let line = wipe.line(style.line);
            let across = if stacked { vec2(0.0, style.halo_width) } else { vec2(style.halo_width, 0.0) };
            painter.rect_filled(line.expand2(across), 0.0, style.halo);
            painter.rect_filled(line, 0.0, style.line_colour);
            let centre = wipe.knob_centre();
            painter.circle_filled(centre, style.knob / 2.0, style.knob_fill);
            let glyph = if stacked { Icon::SwapVertical } else { Icon::SwapHorizontal };
            icons::paint(ui, Rect::from_center_size(centre, vec2(style.knob, style.knob)), glyph, IconSize::Sm, style.knob_glyph);
            if response.has_focus() && FocusRing::visible(ui.ctx()) {
                ui.painter().extend(FocusRing::circle_shapes(&tokens, centre, style.knob / 2.0, None));
            }
            divider = Some(response);
        }

        BeforeAfterOutput { mode, before: sides.before, after: sides.after, fraction: event, divider }
    }

    /// The mode shown this pass: the original while the hold key is down and the canvas has the
    /// keyboard.
    fn shown_mode(&self, ui: &Ui) -> BeforeAfterMode {
        match self.original_key {
            Some(key) if ui.input(|i| i.key_down(key)) && canvas_keys_free(ui.ctx()) => BeforeAfterMode::Original,
            _ => self.mode,
        }
    }

    /// The height a pane gives up to its caption.
    fn strip(&self, tokens: &UiTokens, style: &BeforeAfterStyle) -> f32 {
        if self.captions { tokens.typography.get(TypeRole::Label).line_height + style.caption_gap } else { 0.0 }
    }

    /// The `[before, after]` rects of `mode` in `area`, each less its caption `strip`.
    fn rects(&self, mode: BeforeAfterMode, area: Rect, style: &BeforeAfterStyle, strip: f32, rtl: bool) -> [Rect; 2] {
        let cut = |r: Rect| Rect::from_min_max(pos2(r.min.x, (r.min.y + strip).min(r.max.y)), r.max);
        match mode {
            BeforeAfterMode::SideBySide => {
                let width = ((area.width() - style.gap_side) / 2.0).max(0.0);
                let left = Rect::from_min_size(area.min, vec2(width, area.height()));
                let right = Rect::from_min_size(pos2(area.max.x - width, area.min.y), vec2(width, area.height()));
                let [before, after] = if rtl { [right, left] } else { [left, right] };
                [cut(before), cut(after)]
            }
            BeforeAfterMode::Stacked => {
                let height = ((area.height() - style.gap_stacked) / 2.0).max(0.0);
                let top = Rect::from_min_size(area.min, vec2(area.width(), height));
                let bottom = Rect::from_min_size(pos2(area.min.x, area.max.y - height), vec2(area.width(), height));
                [cut(top), cut(bottom)]
            }
            _ => [cut(area); 2],
        }
    }

    /// The fraction this pass's keys and screen-reader requests give the focused divider, if any.
    fn key_target(&self, ui: &Ui, id: egui::Id, focused: bool, stacked: bool, now: f32) -> Option<f32> {
        let sign = |key| match direction::step(ui, key) {
            Some(Step::Next) => 1.0,
            Some(Step::Previous) => -1.0,
            None => 0.0,
        };
        let keys = if stacked {
            [(Key::ArrowDown, 1.0), (Key::ArrowUp, -1.0)]
        } else {
            [(Key::ArrowRight, sign(Key::ArrowRight)), (Key::ArrowLeft, sign(Key::ArrowLeft))]
        };
        let (mut delta, mut to) = (0.0_f32, None);
        if focused {
            ui.input_mut(|i| {
                let mut count = |modifiers, key| i.count_and_consume_key(modifiers, key) as f32;
                for (key, sign) in keys {
                    // Shift first: `Modifiers::NONE` also matches a press with Shift held.
                    delta += sign * (10.0 * count(Modifiers::SHIFT, key) + count(Modifiers::NONE, key));
                }
                if count(Modifiers::NONE, Key::Home) > 0.0 {
                    to = Some(0.0);
                }
                if count(Modifiers::NONE, Key::End) > 0.0 {
                    to = Some(1.0);
                }
            });
        }
        ui.input(|i| {
            delta += i.num_accesskit_action_requests(id, Action::Increment) as f32 - i.num_accesskit_action_requests(id, Action::Decrement) as f32;
            let set = i.accesskit_action_requests(id, Action::SetValue).filter_map(|r| match r.data {
                Some(ActionData::NumericValue(v)) if v.is_finite() => Some(v as f32 / 100.0),
                _ => None,
            });
            if let Some(v) = set.last() {
                to = Some(v);
            }
        });
        let target = to.or((delta != 0.0).then(|| now + delta / 100.0))?;
        // Whole thousandths, so repeated 1 % steps don't drift.
        Some(((target.clamp(0.0, 1.0)) * 1000.0).round() / 1000.0)
    }

    /// The captions: above each pane, or at the two ends of the picture's top in a wipe.
    fn paint_captions(&self, ui: &Ui, style: &BeforeAfterStyle, labels: &BeforeAfterLabels, sides: &Sides, strip: f32) {
        let painter = ui.painter();
        let (start, end) = (direction::align2(ui, Align2::LEFT_TOP), direction::align2(ui, Align2::RIGHT_TOP));
        let put = |text: &str, x: f32, top: f32, align: Align2| {
            painter.text(pos2(x, top), align, text, style.caption_font.clone(), style.caption_colour);
        };
        if let Some(wipe) = sides.wipe {
            let top = wipe.pane.min.y - strip;
            put(labels.before.as_str(), direction::start_x(ui, wipe.shown), top, start);
            put(labels.after.as_str(), direction::end_x(ui, wipe.shown), top, end);
            return;
        }
        for (pane, text) in [(&sides.before, labels.before.as_str()), (&sides.after, labels.after.as_str())] {
            if let Some(pane) = pane {
                put(text, direction::start_x(ui, pane.area), pane.area.min.y - strip, start);
            }
        }
    }
}

/// Where the divider of a wipe is, in screen points.
#[derive(Clone, Copy, Debug)]
struct Wipe {
    /// The pane the wipe fills.
    pane: Rect,
    /// The shown part of the picture: the picture cut to the pane.
    shown: Rect,
    /// A horizontal divider (`SplitStacked`).
    stacked: bool,
    /// The divider's coordinate across the picture: x, or y when stacked.
    at: f32,
}

impl Wipe {
    /// The divider line, `width` thick, over the full length of the shown picture.
    fn line(&self, width: f32) -> Rect {
        let (r, half) = (self.shown, width / 2.0);
        if self.stacked {
            Rect::from_min_max(pos2(r.min.x, self.at - half), pos2(r.max.x, self.at + half))
        } else {
            Rect::from_min_max(pos2(self.at - half, r.min.y), pos2(self.at + half, r.max.y))
        }
    }

    /// The part of the picture within `knob` / 2 of the line.
    fn reach(&self, knob: f32) -> Rect {
        self.line(knob).intersect(self.shown)
    }

    fn knob_centre(&self) -> Pos2 {
        if self.stacked { pos2(self.shown.center().x, self.at) } else { pos2(self.at, self.shown.center().y) }
    }

    /// The fraction (from the reading start) of a pointer at `pos`.
    fn fraction_at(&self, pos: Pos2, rtl: bool) -> f32 {
        let r = self.shown;
        let t = if self.stacked {
            (pos.y - r.min.y) / r.height().max(f32::EPSILON)
        } else if rtl {
            (r.max.x - pos.x) / r.width().max(f32::EPSILON)
        } else {
            (pos.x - r.min.x) / r.width().max(f32::EPSILON)
        };
        t.clamp(0.0, 1.0)
    }
}

/// The two sides of a pass and the wipe, if the mode is one.
struct Sides {
    before: Option<BeforeAfterPane>,
    after: Option<BeforeAfterPane>,
    wipe: Option<Wipe>,
}

impl Sides {
    fn of(mode: BeforeAfterMode, rects: [Rect; 2], image: Rect, fraction: f32, rtl: bool) -> Self {
        let pane = |side, area: Rect, clip: Rect, image| BeforeAfterPane { side, area, clip, image };
        let [before_rect, after_rect] = rects;
        match mode {
            BeforeAfterMode::After => Self { before: None, after: Some(pane(BeforeAfterSide::After, after_rect, after_rect, image)), wipe: None },
            BeforeAfterMode::Original => {
                Self { before: Some(pane(BeforeAfterSide::Before, before_rect, before_rect, image)), after: None, wipe: None }
            }
            BeforeAfterMode::SideBySide | BeforeAfterMode::Stacked => {
                let moved = image.translate(before_rect.center() - after_rect.center());
                Self {
                    before: Some(pane(BeforeAfterSide::Before, before_rect, before_rect, moved)),
                    after: Some(pane(BeforeAfterSide::After, after_rect, after_rect, image)),
                    wipe: None,
                }
            }
            BeforeAfterMode::Split | BeforeAfterMode::SplitStacked => {
                let area = before_rect;
                let stacked = mode == BeforeAfterMode::SplitStacked;
                let cut = image.intersect(area);
                let shown = if cut.width() > 0.0 && cut.height() > 0.0 { cut } else { area };
                let (at, before_clip, after_clip) = if stacked {
                    let at = shown.min.y + fraction * shown.height();
                    (at, Rect::from_min_max(area.min, pos2(area.max.x, at)), Rect::from_min_max(pos2(area.min.x, at), area.max))
                } else if rtl {
                    let at = shown.max.x - fraction * shown.width();
                    (at, Rect::from_min_max(pos2(at, area.min.y), area.max), Rect::from_min_max(area.min, pos2(at, area.max.y)))
                } else {
                    let at = shown.min.x + fraction * shown.width();
                    (at, Rect::from_min_max(area.min, pos2(at, area.max.y)), Rect::from_min_max(pos2(at, area.min.y), area.max))
                };
                Self {
                    before: Some(pane(BeforeAfterSide::Before, area, before_clip, image)),
                    after: Some(pane(BeforeAfterSide::After, area, after_clip, image)),
                    wipe: Some(Wipe { pane: area, shown, stacked, at }),
                }
            }
        }
    }
}
