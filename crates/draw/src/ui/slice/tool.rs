//! [`SliceTool`]: the Slice and Slice Select tools on the tool host (task SH-T4).

use craft_theme::Srgba;
use egui::{Color32, CursorIcon, Key, Modifiers, Painter, Shape, Stroke, StrokeKind, vec2};
use kurbo::{Point, Rect, Vec2};

use super::overlay::{number_text, outline};
use super::{
    SliceCommand, SliceEvent, SliceItem, SliceMode, SliceOrigin, SliceOverlay, SliceRefusal, SliceRules, SliceStyle, SliceToolLabels,
    SliceToolSettings,
};
use crate::geometry::slice::{HANDLES, fixed_ratio_pixels, handle_at, resized_rect};
use craft_ui::behavior::nudge::NudgeKeys;
use craft_ui::canvas::{
    CanvasTool, Constraint, Part, SnapGesture, ToolAction, ToolContext, ToolCursor, ToolKey, ToolParts, ToolPointer, marquee_pixels, resize_cursor,
    square_pixels,
};
use craft_ui::edit::{CancelReason, EditEvent, GestureId};
use craft_ui::localized::Labels as _;

/// The Slice and Slice Select tools. The app owns one value, sets its [`mode`](Self::mode) from
/// the selected tool and its snapshot (items, selection, lock, colour) each pass, hands it to
/// `ToolHost::show` and drains [`take_events`](Self::take_events). The tool never changes the
/// document.
///
/// Slice mode:
/// - Normal: press and drag; a drag no wider and no taller than `slice.click_slop` (2 document
///   pixels) is a click that selects the slice under the point (a `Select` event). Fixed Ratio:
///   [`fixed_ratio_pixels`](crate::geometry::slice::fixed_ratio_pixels). Fixed Size: exactly its
///   pixels, made by a click (`fixed_size_pixels`), previewed from the press. Normal uses
///   `Constraint::corners` and `marquee_pixels`. With [`SliceRules::modifiers`] ⇧ is square and ⌥
///   from the centre.
/// - Snaps the pointer (`SnapGesture::Point`), the press included in both modes.
///
/// Select mode:
/// - Click selects: stored slices win over auto slices, the latest on top; hidden auto slices
///   ([`hide_auto`](Self::hide_auto)) can't be picked. ⇧-click adds or removes with
///   [`SliceRules::multi_select`].
/// - A drag moves the whole slice, also when the press hit a piece of a covered slice; auto and
///   layer slices don't move (the press only selects).
/// - A handle of a selected user slice (8: corners first, `handle.hit` reach, as
///   [`handle_at`](crate::geometry::slice::handle_at)) resizes it with
///   [`resized_rect`](crate::geometry::slice::resized_rect), the travel measured from the handle's
///   own point so a snapped pointer is where the edge ends up.
/// - Delete or Backspace without modifiers, with a stored (user or layer) slice selected, reports
///   `Command(Delete)` and takes the key, so Layer › Delete never sees it (an auto slice: nothing,
///   the key is not taken). Double click and Enter report `Command(Options)`. Arrows move a
///   selected user slice by `NudgeKeys::from_key` while the canvas has the focus.
///
/// Locked ([`locked`](Self::locked)): a press draws and moves nothing, clears the selection
/// (`Select { number: None }`, as PhotoCraft) and reports `Refused(Locked)`.
///
/// Keyboard parts: the slices are one Tab stop (`CanvasTool::parts`), group [`SliceToolLabels::group`],
/// each slice a part named as [`SliceToolLabels::slice`] ("Slice 03, user slice, 120 by 80 at 40, 40");
/// Page keys pick, arrows move the picked user slice, Delete deletes it, Enter opens its options.
///
/// Paint: the [`SliceOverlay`](super::SliceOverlay) of the items, the selected slice's handles
/// (`Handle` squares in `overlay.handle_fill` with the selected colour's edge), and a new slice's
/// preview (white 1 pt rect under the dashed slice-colour line).
#[derive(Debug, Default)]
pub struct SliceTool {
    mode: SliceMode,
    items: Vec<SliceItem>,
    selected: Option<usize>,
    locked: bool,
    color: Option<Srgba>,
    numbers: bool,
    hide_auto: bool,
    settings: SliceToolSettings,
    rules: SliceRules,
    labels: Option<SliceToolLabels>,
    events: Vec<SliceEvent>,
    drag: Option<Drag>,
}

/// A gesture between passes.
#[derive(Clone, Debug)]
enum Drag {
    /// A new slice from the press `start`; `last` is the rectangle the last `Update` carried.
    New {
        id: GestureId,
        start: Point,
        at: Point,
        modifiers: Modifiers,
        began: bool,
        last: Option<Rect>,
    },
    Move {
        id: GestureId,
        number: usize,
        rect: Rect,
        start: Point,
        at: Point,
        began: bool,
    },
    Resize {
        id: GestureId,
        number: usize,
        rect: Rect,
        handle: [i8; 2],
        start: Point,
        at: Point,
        began: bool,
    },
    /// A press on a slice that doesn't move (auto, layer): only its click count matters.
    Hold,
}

impl SliceTool {
    /// The Slice tool with numbers shown and Photoshop's rules.
    pub fn new() -> Self {
        SliceTool { numbers: true, ..SliceTool::default() }
    }

    /// Slice or Select.
    pub fn mode(&mut self, mode: SliceMode) -> &mut Self {
        self.mode = mode;
        self
    }

    /// This pass's slices, in the app's order (stored slices before auto slices).
    pub fn items(&mut self, items: &[SliceItem]) -> &mut Self {
        self.items.clear();
        self.items.extend_from_slice(items);
        self
    }

    /// The selected slice's number.
    pub fn selected(&mut self, number: Option<usize>) -> &mut Self {
        self.selected = number;
        self
    }

    /// View › Lock Slices.
    pub fn locked(&mut self, locked: bool) -> &mut Self {
        self.locked = locked;
        self
    }

    /// The slice colour preference (Guides, Grid & Slices); `overlay.slice` when `None`.
    pub fn color(&mut self, color: Option<Srgba>) -> &mut Self {
        self.color = color;
        self
    }

    /// Show Slice Numbers.
    pub fn numbers(&mut self, numbers: bool) -> &mut Self {
        self.numbers = numbers;
        self
    }

    /// Hide Auto Slices.
    pub fn hide_auto(&mut self, hide: bool) -> &mut Self {
        self.hide_auto = hide;
        self
    }

    /// The Style, Width and Height a new slice is drawn with.
    pub fn settings(&mut self, settings: SliceToolSettings) -> &mut Self {
        self.settings = settings;
        self
    }

    /// Which product's rules the tools follow; Photoshop's by default.
    pub fn rules(&mut self, rules: SliceRules) -> &mut Self {
        self.rules = rules;
        self
    }

    /// The words; the installed ones by default.
    pub fn labels(&mut self, labels: SliceToolLabels) -> &mut Self {
        self.labels = Some(labels);
        self
    }

    /// The events since the last call, in order.
    pub fn take_events(&mut self) -> Vec<SliceEvent> {
        std::mem::take(&mut self.events)
    }

    fn push(&mut self, event: SliceEvent) {
        self.events.push(event);
    }

    /// The selected slice in this pass's items.
    fn selected_item(&self) -> Option<&SliceItem> {
        let number = self.selected?;
        self.items.iter().find(|i| i.number == number)
    }

    /// The selected slice when it is a user slice, the only kind with handles.
    fn user_selection(&self) -> Option<&SliceItem> {
        self.selected_item().filter(|i| i.origin == SliceOrigin::User)
    }

    /// Fixed Size draws by a click.
    fn fixed_size(&self) -> bool {
        matches!(self.settings.style, Constraint::Size(..))
    }

    /// How far from a handle's centre a press grabs it, in document units.
    fn reach(style: &SliceStyle, cx: &ToolContext<'_>) -> f64 {
        f64::from(style.handle_hit) / 2.0 / cx.view.zoom.max(f64::EPSILON)
    }

    /// The slice a Select press at `at` picks: the topmost stored slice containing it, then the
    /// topmost auto slice unless hidden.
    fn slice_at(&self, at: Point) -> Option<&SliceItem> {
        // pc: crates/ui-egui/src/tools/slice_ui.rs:100-105 `slice_at`, :172-185 (a covered piece
        // moves its whole slice).
        let hit = |item: &&SliceItem| item.rect.x0 <= at.x && at.x < item.rect.x1 && item.rect.y0 <= at.y && at.y < item.rect.y1;
        let mut hits = self.items.iter().rev().filter(hit);
        // No stored slice contains the point, so any hit left is an auto slice.
        hits.clone().find(|i| i.origin != SliceOrigin::Auto).or_else(|| hits.next().filter(|_| !self.hide_auto))
    }

    /// The new slice from `start` to `at` under the settings and `modifiers`.
    fn new_rect(&self, start: Point, at: Point, modifiers: egui::Modifiers) -> Rect {
        // pc: crates/ui-egui/src/tools/slice_ui.rs:113-148 `new_rect`.
        match self.settings.style {
            // The slice sits on the pixel the press is in and grows the way the drag points, so it
            // holds that pixel; a slice drawn left or up ends at the pixel's far edge.
            Constraint::Size(w, h) => {
                let (w, h) = (w.round().max(1.0), h.round().max(1.0));
                let sign = |d: f64| if d < 0.0 { -1.0 } else { 1.0 };
                let (dx, dy) = (sign(at.x - start.x), sign(at.y - start.y));
                let corner =
                    Point::new(if dx < 0.0 { start.x.ceil() } else { start.x.floor() }, if dy < 0.0 { start.y.ceil() } else { start.y.floor() });
                marquee_pixels(corner, Point::new(corner.x + dx * w, corner.y + dy * h))
            }
            Constraint::Ratio(w, h) => fixed_ratio_pixels(start, at, (w, h)),
            _ => {
                let (square, centre) = if self.rules.modifiers { (modifiers.shift, modifiers.alt) } else { (false, false) };
                let (a, b) = Constraint::Free.corners(start, at, square, centre);
                let rect = marquee_pixels(a, b);
                if square && !centre { square_pixels(rect, start, at) } else { rect }
            }
        }
    }

    /// Reports the new slice's `Begin` (past the click slop, or at the press for Fixed Size) and
    /// an `Update` when its rectangle changed.
    fn update_new(&mut self, slop: f64) {
        let Some(&Drag::New { id, start, at, modifiers, began, last }) = self.drag.as_ref() else { return };
        let rect = self.new_rect(start, at, modifiers);
        let drawn = rect.width() > 0.0 && rect.height() > 0.0;
        if !drawn || (!began && !self.fixed_size() && rect.width() <= slop && rect.height() <= slop) {
            return;
        }
        if !began {
            self.push(SliceEvent::Draw(EditEvent::begin(id)));
        }
        if last != Some(rect) {
            self.push(SliceEvent::Draw(EditEvent::update(id, rect, modifiers)));
        }
        if let Some(Drag::New { began, last, .. }) = self.drag.as_mut() {
            *began = true;
            *last = Some(rect);
        }
    }

    /// A drag's offset in whole document pixels.
    fn whole(by: Vec2) -> Vec2 {
        Vec2::new(by.x.round(), by.y.round())
    }

    /// The document point of `handle` on `rect`.
    fn handle_point(rect: Rect, handle: [i8; 2]) -> Point {
        let at = |lo: f64, hi: f64, h: i8| match h {
            -1 => lo,
            1 => hi,
            _ => (lo + hi) / 2.0,
        };
        Point::new(at(rect.x0, rect.x1, handle[0]), at(rect.y0, rect.y1, handle[1]))
    }

    /// `rect` with `handle` dragged to the pointer at `to`: the travel is measured from the handle's
    /// own point, so a snapped pointer is where the edge ends up, not the press's offset from it.
    fn resized(rect: Rect, handle: [i8; 2], to: Point) -> Rect {
        resized_rect(rect, handle, to - Self::handle_point(rect, handle))
    }

    fn resize_cursor(handle: [i8; 2], cx: &ToolContext<'_>) -> ToolCursor {
        ToolCursor::System(resize_cursor(cx.view, Vec2::new(f64::from(handle[0]), f64::from(handle[1]))))
    }
}

impl CanvasTool for SliceTool {
    fn cursor(&self, cx: &ToolContext<'_>) -> ToolCursor {
        // new: Slice: the crosshair; Select: a resize arrow over a handle of the selected user
        // slice (`resize_cursor`, turned with the view), the move arrow over a movable slice, else
        // the default arrow.
        let arrow = ToolCursor::System(CursorIcon::Default);
        if self.mode == SliceMode::Slice {
            return ToolCursor::System(CursorIcon::Crosshair);
        }
        match &self.drag {
            Some(Drag::Resize { handle, .. }) => return Self::resize_cursor(*handle, cx),
            Some(Drag::Move { .. }) => return ToolCursor::System(CursorIcon::Move),
            Some(_) => return arrow,
            None => {}
        }
        let Some(pointer) = cx.pointer.filter(|_| !self.locked) else { return arrow };
        let reach = Self::reach(&Self::style(cx.tokens), cx);
        if let Some(handle) = self.user_selection().and_then(|i| handle_at(i.rect, pointer.doc, reach)) {
            return Self::resize_cursor(handle, cx);
        }
        if self.slice_at(pointer.doc).is_some_and(|i| i.origin == SliceOrigin::User) { ToolCursor::System(CursorIcon::Move) } else { arrow }
    }

    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        // pc: crates/ui-egui/src/tools/slice_ui.rs:152-200 `pointer` (`Down`: locked at :161-166,
        // Slice starts `New`, Select picks, a handle starts `Resize`).
        if self.locked {
            // PhotoCraft clears the selection on a locked press (`slice_ui.rs:161-163`).
            self.selected = None;
            self.push(SliceEvent::Select { number: None, add: false, modifiers: pointer.modifiers });
            self.push(SliceEvent::Refused(SliceRefusal::Locked));
            return ToolAction::Done;
        }
        let at = pointer.doc;
        if self.mode == SliceMode::Slice {
            self.drag = Some(Drag::New { id: GestureId::next(), start: at, at, modifiers: pointer.modifiers, began: false, last: None });
            // Fixed Size is drawn from the press.
            self.update_new(f64::from(Self::style(cx.tokens).click_slop));
            return ToolAction::Claimed;
        }
        // A handle of the selected user slice resizes it.
        let reach = Self::reach(&Self::style(cx.tokens), cx);
        if let Some((number, rect, handle)) = self.user_selection().and_then(|i| Some((i.number, i.rect, handle_at(i.rect, at, reach)?))) {
            self.drag = Some(Drag::Resize { id: GestureId::next(), number, rect, handle, start: at, at, began: false });
            return ToolAction::Claimed;
        }
        let hit = self.slice_at(at).cloned();
        let add = self.rules.multi_select && pointer.modifiers.shift;
        let number = hit.as_ref().map(|i| i.number);
        if !add {
            self.selected = number;
        }
        self.push(SliceEvent::Select { number, add, modifiers: pointer.modifiers });
        // The whole slice moves, also when the press hits one piece of a covered slice.
        self.drag = match hit {
            Some(item) if item.origin == SliceOrigin::User => {
                Some(Drag::Move { id: GestureId::next(), number: item.number, rect: item.rect, start: at, at, began: false })
            }
            Some(_) => Some(Drag::Hold),
            None => return ToolAction::Done,
        };
        ToolAction::Claimed
    }

    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/slice_ui.rs:152-231 `pointer` (`Drag`).
        let Some(drag) = self.drag.clone() else { return };
        let to = pointer.doc;
        match drag {
            Drag::New { .. } => {
                if let Some(Drag::New { at, modifiers, .. }) = self.drag.as_mut() {
                    *at = to;
                    *modifiers = pointer.modifiers;
                }
                self.update_new(f64::from(Self::style(cx.tokens).click_slop));
            }
            Drag::Move { id, number, rect, start, at, began } => {
                let (by, before) = (Self::whole(to - start), Self::whole(at - start));
                if by != Vec2::ZERO || began {
                    if !began {
                        self.push(SliceEvent::Move { number, event: EditEvent::begin(id) });
                    }
                    if !began || by != before {
                        self.push(SliceEvent::Move { number, event: EditEvent::update(id, rect + by, pointer.modifiers) });
                    }
                }
                self.drag = Some(Drag::Move { id, number, rect, start, at: to, began: began || by != Vec2::ZERO });
            }
            Drag::Resize { id, number, rect, handle, start, at, began } => {
                let (now, before) = (Self::resized(rect, handle, to), Self::resized(rect, handle, at));
                // A press on a handle off its centre resizes only once the pointer moves.
                let moved = to != start;
                if (moved && now != rect) || began {
                    if !began {
                        self.push(SliceEvent::Resize { number, event: EditEvent::begin(id) });
                    }
                    if !began || now != before {
                        self.push(SliceEvent::Resize { number, event: EditEvent::update(id, now, pointer.modifiers) });
                    }
                }
                self.drag = Some(Drag::Resize { id, number, rect, handle, start, at: to, began: began || (moved && now != rect) });
            }
            Drag::Hold => {}
        }
    }

    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/slice_ui.rs:152-231 `pointer` (`Up`: Normal needs a drag over 2 px,
        // Fixed Size a click); `ToolPointer::clicks` == 2 reports `Command(Options)`.
        let Some(drag) = self.drag.take() else { return };
        let (to, modifiers) = (pointer.doc, pointer.modifiers);
        let mut options = false;
        match drag {
            Drag::New { id, start, began, .. } => {
                let slop = f64::from(Self::style(cx.tokens).click_slop);
                let rect = self.new_rect(start, to, modifiers);
                let drawn = rect.width() > 0.0 && rect.height() > 0.0 && (began || self.fixed_size() || rect.width() > slop || rect.height() > slop);
                if drawn {
                    if !began {
                        self.push(SliceEvent::Draw(EditEvent::begin(id)));
                    }
                    self.push(SliceEvent::Draw(EditEvent::commit(id, rect, true, modifiers)));
                } else if began {
                    self.push(SliceEvent::Draw(EditEvent::cancel(id, CancelReason::External)));
                } else {
                    // A click selects the slice under it.
                    let number = self.slice_at(to).map(|i| i.number);
                    let add = self.rules.multi_select && modifiers.shift;
                    if !add {
                        self.selected = number;
                    }
                    self.push(SliceEvent::Select { number, add, modifiers });
                    options = number.is_some();
                }
            }
            Drag::Move { id, number, rect, start, began, .. } => {
                let by = Self::whole(to - start);
                if began || by != Vec2::ZERO {
                    if !began {
                        self.push(SliceEvent::Move { number, event: EditEvent::begin(id) });
                    }
                    self.push(SliceEvent::Move { number, event: EditEvent::commit(id, rect + by, by != Vec2::ZERO, modifiers) });
                } else {
                    options = true;
                }
            }
            Drag::Resize { id, number, rect, handle, start, began, .. } => {
                let now = Self::resized(rect, handle, to);
                let changed = to != start && now != rect;
                if began || changed {
                    if !began {
                        self.push(SliceEvent::Resize { number, event: EditEvent::begin(id) });
                    }
                    self.push(SliceEvent::Resize { number, event: EditEvent::commit(id, now, changed, modifiers) });
                }
            }
            Drag::Hold => options = true,
        }
        if options && pointer.clicks == 2 {
            self.push(SliceEvent::Command(SliceCommand::Options));
        }
    }

    fn cancel(&mut self, reason: CancelReason, _cx: &mut ToolContext<'_>) {
        // new: the running `Draw`, `Move` or `Resize` reports `Cancel`; drops the drag.
        match self.drag.take() {
            Some(Drag::New { id, began: true, .. }) => self.push(SliceEvent::Draw(EditEvent::cancel(id, reason))),
            Some(Drag::Move { id, number, began: true, .. }) => self.push(SliceEvent::Move { number, event: EditEvent::cancel(id, reason) }),
            Some(Drag::Resize { id, number, began: true, .. }) => self.push(SliceEvent::Resize { number, event: EditEvent::cancel(id, reason) }),
            _ => {}
        }
    }

    fn key(&mut self, key: &ToolKey, _cx: &mut ToolContext<'_>) -> bool {
        // pc: crates/ui-egui/src/tools/slice_ui.rs:339-368 `keys` (Delete or Backspace, no modifier,
        // Select, a selected slice; auto: nothing).
        // new: Enter reports `Command(Options)`; arrows through `NudgeKeys::from_key`.
        if !key.pressed || self.drag.is_some() || self.mode != SliceMode::Select {
            return false;
        }
        let Some(item) = self.selected_item().cloned() else { return false };
        let user = item.origin == SliceOrigin::User;
        if key.modifiers.is_none() {
            match key.key {
                Key::Delete | Key::Backspace => {
                    // An auto slice is made from the user slices around it: it goes when they do.
                    if !item.origin.stored() {
                        return false;
                    }
                    self.push(if self.locked { SliceEvent::Refused(SliceRefusal::Locked) } else { SliceEvent::Command(SliceCommand::Delete) });
                    return true;
                }
                Key::Enter => {
                    self.push(SliceEvent::Command(SliceCommand::Options));
                    return true;
                }
                _ => {}
            }
        }
        let Some(nudge) = NudgeKeys::new().from_key(key).filter(|_| user) else { return false };
        if self.locked {
            self.push(SliceEvent::Refused(SliceRefusal::Locked));
            return true;
        }
        let rect = item.rect + nudge.by;
        if let Some(own) = self.items.iter_mut().find(|i| i.number == item.number) {
            own.rect = rect;
        }
        self.push(SliceEvent::Move { number: item.number, event: EditEvent::set(rect, key.modifiers) });
        true
    }

    fn paint(&self, painter: &Painter, cx: &ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/slice_ui.rs:242-321 `draw_overlay` (handles at :271-281, the
        // drag previews at :305-319). Lead call 4: handles are `handle.size` squares in
        // `overlay.handle_fill` with the selected colour's edge, not PhotoCraft's 5 × 5 filled ones.
        // new: `SliceOverlay::paint`, then the handles and the preview with `SliceTool::style`.
        let style = Self::style(cx.tokens);
        SliceOverlay::new(&self.items)
            .selected(self.selected)
            .color(self.color)
            .numbers(self.numbers)
            .auto(!self.hide_auto)
            .paint(painter, cx.view, &style, cx.numbers);
        if let Some(item) = self.user_selection() {
            for handle in HANDLES {
                let body = egui::Rect::from_center_size(cx.view.to_screen(Self::handle_point(item.rect, handle)), vec2(style.handle, style.handle));
                painter.rect_filled(body, 0.0, style.handle_fill);
                painter.rect_stroke(body, 0.0, style.handle_edge, StrokeKind::Inside);
            }
        }
        let selected = Stroke::new(style.selected_outline, style.selected);
        match &self.drag {
            Some(Drag::New { start, at, modifiers, .. }) => {
                let quad = cx.view.quad(self.new_rect(*start, *at, *modifiers));
                let color = self.color.map_or(style.color, Color32::from);
                painter.add(Shape::closed_line(quad.to_vec(), Stroke::new(style.outline, Color32::WHITE)));
                let points = [quad[0], quad[1], quad[2], quad[3], quad[0]];
                painter.add(Shape::dashed_line(&points, Stroke::new(style.outline, color), style.drag_dash.0, style.drag_dash.1));
            }
            Some(Drag::Move { rect, start, at, .. }) => outline(painter, cx.view, *rect + Self::whole(*at - *start), selected),
            Some(Drag::Resize { rect, handle, start, at, .. }) => {
                let now = if at == start { *rect } else { Self::resized(*rect, *handle, *at) };
                outline(painter, cx.view, now, selected);
            }
            Some(Drag::Hold) | None => {}
        }
    }

    fn snap_gesture(&self) -> Option<SnapGesture> {
        // new: `Point` while drawing or resizing and before a press in either mode (a Select press
        // can start a resize, whose handle point is the snapped pointer), `Rect { at_press }` while
        // moving.
        match &self.drag {
            Some(Drag::New { .. } | Drag::Resize { .. }) | None => Some(SnapGesture::Point),
            Some(Drag::Move { rect, .. }) => Some(SnapGesture::Rect { at_press: *rect }),
            Some(Drag::Hold) => None,
        }
    }

    fn parts(&self, cx: &ToolContext<'_>) -> Option<ToolParts> {
        // new: one part per visible slice (key = its number), named by `SliceToolLabels::slice`.
        let labels = self.labels.clone().unwrap_or_else(|| SliceToolLabels::of_ctx(&cx.canvas.ctx));
        let numbers = cx.numbers.clone().ungrouped();
        let count = |value: f64| numbers.format(value, 0..=0);
        let parts: Vec<Part> = self
            .items
            .iter()
            .filter(|i| !(self.hide_auto && i.origin == SliceOrigin::Auto))
            .map(|item| {
                let origin = match item.origin {
                    SliceOrigin::User => &labels.user,
                    SliceOrigin::Auto => &labels.auto,
                    SliceOrigin::Layer => &labels.layer,
                };
                let name = craft_i18n::fmt(
                    &labels.slice,
                    &[
                        ("number", &number_text(&numbers, item.number)),
                        ("origin", origin),
                        ("width", &count(item.rect.width())),
                        ("height", &count(item.rect.height())),
                        ("x", &count(item.rect.x0)),
                        ("y", &count(item.rect.y0)),
                    ],
                );
                Part {
                    key: item.number as u64,
                    name,
                    value: None,
                    rect: egui::Rect::from_points(&cx.view.quad(item.rect)),
                    selected: self.selected == Some(item.number),
                }
            })
            .collect();
        (!parts.is_empty()).then(|| ToolParts::new(labels.group.clone(), parts))
    }

    fn picked(&self) -> Option<u64> {
        self.selected.map(|n| n as u64)
    }

    fn pick(&mut self, key: Option<u64>, _cx: &mut ToolContext<'_>) {
        // new: reports `Select { number }` for the picked part.
        let number = key.map(|k| k as usize);
        self.selected = number;
        self.push(SliceEvent::Select { number, add: false, modifiers: Modifiers::NONE });
    }
}
