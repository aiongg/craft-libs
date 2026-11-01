//! [`ArtboardTool`]: the Artboard tool on the tool host (task SH-T5).

use egui::{CursorIcon, Key, Modifiers, Painter, Pos2, Shape, StrokeKind, vec2};
use kurbo::{Point, Rect, Vec2};

use super::{ArtboardEvent, ArtboardItem, ArtboardRules, ArtboardToolLabels};
use crate::geometry::artboard::{drawn_rect, resized_keep_ratio, resized_rect};
use crate::geometry::slice::HANDLES;
use crate::ui::add_beside::{AddBesideHandles, AddSide};
use crate::ui::crop::hit_rect;
use craft_ui::behavior::nudge::NudgeKeys;
use craft_ui::canvas::quad::BoxPart;
use craft_ui::canvas::{CanvasTool, Part, SnapGesture, ToolAction, ToolContext, ToolCursor, ToolKey, ToolParts, ToolPointer, resize_cursor};
use craft_ui::edit::{CancelReason, EditEvent, GestureId};
use craft_ui::localized::Labels as _;

/// The Artboard tool. The app owns the value, sets the snapshot (boards, selection, rules) each
/// pass, hands it to `ToolHost::show` and drains [`take_events`](Self::take_events). The tool never
/// changes the document.
///
/// Pointer (Photoshop's rules, [`ArtboardRules::PHOTOSHOP`]):
/// - A press on a "+" button beside the selected board ([`AddBesideHandles::at`](crate::ui::add_beside::AddBesideHandles::at))
///   reports `AddBeside` (⌥: `copy`).
/// - A press on a handle of the selected board (8: corners first, `handle.hit` reach, through
///   `hit_rect`) resizes it ([`resized_rect`](crate::geometry::artboard::resized_rect); ⇧ on a
///   corner keeps proportions with [`resized_keep_ratio`](crate::geometry::artboard::resized_keep_ratio),
///   also when ⇧ goes down mid-drag; ⌥ about the centre). A locked board does not resize.
/// - A press inside a board selects it (`Select`) and a drag moves it with its contents
///   (`Move`, whole-pixel offsets). A locked board selects but never moves.
/// - A press on the pasteboard draws a board ([`drawn_rect`](crate::geometry::artboard::drawn_rect));
///   a drag under `artboard.click_slop` (3 screen points) draws nothing.
/// - Esc cancels the drag; ⌘ held is the app's Move tool (a momentary tool on the host).
///
/// Illustrator's rules ([`ArtboardRules::ILLUSTRATOR`]) add: ⇧-click toggles a board in the
/// selection; ⇧-drag inside a board draws a new one; ⇧-drag on the pasteboard is a marquee that
/// selects the boards it touches; ⌥-drag moves a copy; a click on an object outside the boards
/// reports `FitObject`; Esc with no drag reports `Leave`; a double click on an empty part of a
/// board reports `OpenOptions`.
///
/// Keys while the canvas has the focus and the tool is active: arrows move the selected boards by
/// [`ArtboardRules::arrow_step`] (⇧ ten times, `NudgeKeys::from_key`), ⌥+arrows resize from the
/// end edges (`Resize`); Delete reports `Delete`; Enter `OpenOptions`; Insert adds a board on the
/// right of the picked one.
///
/// Keyboard parts: the boards are one Tab stop, group [`ArtboardToolLabels::group`], each board a part
/// named as [`ArtboardToolLabels::board`] ("Artboard 2, 1080 by 1920 at 0, 0") with its selected state;
/// Page keys pick a board.
///
/// Cursor: resize arrows over the selected board's handles, turned with the view
/// (`resize_cursor`); the pointing hand over a "+" button; the move arrow inside a board.
///
/// Paint: the drag preview (`artboard.preview_line`, 1.5 pt in `accent.solid`), the selected
/// board's 8 handles (`Handle` squares: `overlay.handle_fill`, 1 pt accent edge), and the "+"
/// buttons ([`AddBesideHandles::paint`](crate::ui::add_beside::AddBesideHandles::paint)) when no
/// drag runs. Frames and names are the `Pasteboard`'s.
#[derive(Debug, Default)]
pub struct ArtboardTool {
    items: Vec<ArtboardItem>,
    selected: Vec<usize>,
    rules: ArtboardRules,
    labels: Option<ArtboardToolLabels>,
    events: Vec<ArtboardEvent>,
    drag: Option<Drag>,
    /// The "+" button under the pointer, for the cursor and the hover look.
    hovered_plus: Option<AddSide>,
}

/// A gesture between passes: what the press took, and where the pointer is now.
#[derive(Clone, Debug)]
struct Drag {
    kind: Kind,
    id: GestureId,
    /// The press on screen, for the click slop.
    press: Pos2,
    /// The press in document units: the snapped point, or for a move the raw one (the host snaps
    /// the moving rectangle against it).
    start: Point,
    at: Point,
    /// The modifiers of the latest sample: ⇧ and ⌥ may change mid-drag.
    modifiers: Modifiers,
    /// The drag has left the click slop and reported `Begin`.
    began: bool,
    /// The last value reported, so a still pointer sends no update.
    sent: Option<Val>,
}

#[derive(Clone, Debug)]
enum Kind {
    /// A new board. `toggle`: Illustrator's ⇧-press inside board `i`, which toggles it in the
    /// selection when it stays a click.
    New { toggle: Option<usize> },
    /// The boards `indices` move. `select_on_click`: the press was inside a board of a larger
    /// selection, which a click narrows to that board.
    Move { indices: Vec<usize>, copy: bool, select_on_click: Option<usize> },
    /// Board `index`, whose rect was `rect` at the press, resized by `handle`.
    Resize { index: usize, rect: Rect, handle: [i8; 2] },
    /// Illustrator's ⇧-drag on the pasteboard.
    Marquee,
}

/// A gesture's value: the board's rect, or the offset of a move.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Val {
    Rect(Rect),
    By(Vec2),
}

#[derive(Clone, Copy)]
enum Phase {
    Begin,
    Update,
    /// `changed`: the value differs from the start.
    Commit(bool),
    Cancel,
}

fn edit<T>(phase: Phase, id: GestureId, modifiers: Modifiers, value: Option<T>) -> Option<EditEvent<T>> {
    Some(match phase {
        Phase::Begin => EditEvent::begin(id),
        Phase::Update => EditEvent::update(id, value?, modifiers),
        Phase::Commit(changed) => EditEvent::commit(id, value?, changed, modifiers),
        Phase::Cancel => EditEvent::cancel(id, CancelReason::Escape),
    })
}

/// The event of `phase` for a drag whose value is `value`; a marquee reports none.
fn event(d: &Drag, phase: Phase, value: Option<Val>) -> Option<ArtboardEvent> {
    let rect = || match value {
        Some(Val::Rect(r)) => Some(r),
        _ => None,
    };
    let by = || match value {
        Some(Val::By(v)) => Some(v),
        _ => None,
    };
    match &d.kind {
        Kind::New { .. } => Some(ArtboardEvent::Draw(edit(phase, d.id, d.modifiers, rect())?)),
        Kind::Move { indices, copy, .. } => {
            Some(ArtboardEvent::Move { indices: indices.clone(), event: edit(phase, d.id, d.modifiers, by())?, copy: *copy })
        }
        Kind::Resize { index, .. } => Some(ArtboardEvent::Resize { index: *index, event: edit(phase, d.id, d.modifiers, rect())? }),
        Kind::Marquee => None,
    }
}

/// The handle a corner or edge part stands for: −1, 0 or 1 per axis.
fn handle_of(part: BoxPart) -> Option<[i8; 2]> {
    match part {
        BoxPart::Corner(i) => Some([[-1, -1], [1, -1], [1, 1], [-1, 1]][usize::from(i % 4)]),
        BoxPart::Edge(i) => Some([[0, -1], [1, 0], [0, 1], [-1, 0]][usize::from(i % 4)]),
        _ => None,
    }
}

/// Where handle `handle` of `rect` is.
fn handle_point(rect: Rect, handle: [i8; 2]) -> Point {
    let axis = |lo: f64, hi: f64, h: i8| match h {
        -1 => lo,
        1 => hi,
        _ => (lo + hi) / 2.0,
    };
    Point::new(axis(rect.x0, rect.x1, handle[0]), axis(rect.y0, rect.y1, handle[1]))
}

fn contains(rect: Rect, at: Point) -> bool {
    let r = rect.abs();
    at.x >= r.x0 && at.x < r.x1 && at.y >= r.y0 && at.y < r.y1
}

fn overlaps(a: Rect, b: Rect) -> bool {
    let (a, b) = (a.abs(), b.abs());
    a.x0 <= b.x1 && a.x1 >= b.x0 && a.y0 <= b.y1 && a.y1 >= b.y0
}

impl ArtboardTool {
    /// A tool with Photoshop's rules.
    pub fn new() -> Self {
        Self::default()
    }

    /// This pass's boards, in the app's order.
    pub fn items(&mut self, items: &[ArtboardItem]) -> &mut Self {
        self.items.clear();
        self.items.extend_from_slice(items);
        self
    }

    /// The selected boards (the active board first).
    pub fn selected(&mut self, indices: &[usize]) -> &mut Self {
        self.selected.clear();
        self.selected.extend_from_slice(indices);
        self
    }

    /// Which product's rules the tool follows; Photoshop's by default.
    pub fn rules(&mut self, rules: ArtboardRules) -> &mut Self {
        self.rules = rules;
        self
    }

    /// The words; the installed ones by default.
    pub fn labels(&mut self, labels: ArtboardToolLabels) -> &mut Self {
        self.labels = Some(labels);
        self
    }

    /// The events since the last call, in order.
    pub fn take_events(&mut self) -> Vec<ArtboardEvent> {
        std::mem::take(&mut self.events)
    }

    /// The board a press at `at` lands in: the last (topmost) board containing it.
    fn board_at(&self, at: Point) -> Option<usize> {
        // pc: crates/ui-egui/src/tools/artboard_tool.rs:84-88 `board_at` (`.rev().find`; edges
        // inclusive low, exclusive high).
        self.items.iter().rposition(|b| contains(b.rect, at))
    }

    /// The active board: the first selected one this pass's boards still have.
    fn active(&self) -> Option<usize> {
        self.selected.first().copied().filter(|&i| i < self.items.len())
    }

    /// The selected boards that exist and are not locked.
    fn movable(&self) -> Vec<usize> {
        self.selected.iter().copied().filter(|&i| self.items.get(i).is_some_and(|b| !b.locked)).collect()
    }

    /// The "+" button of board `index` under the screen point `at`.
    fn plus_at(&self, index: usize, at: Pos2, cx: &ToolContext<'_>) -> Option<AddSide> {
        let style = AddBesideHandles::style(cx.tokens);
        AddBesideHandles::new("artboard-add-beside", self.items.get(index)?.rect).at(cx.view, &style, at)
    }

    /// The handle of the active board under `at`, when the board can be resized.
    fn handle_at(&self, at: Point, cx: &ToolContext<'_>) -> Option<(usize, [i8; 2])> {
        let index = self.active().filter(|&i| !self.items[i].locked)?;
        let reach = cx.view.doc_len(Self::style(cx.tokens).handle_hit / 2.0);
        handle_of(hit_rect(self.items[index].rect, at, reach)).map(|h| (index, h))
    }

    /// The value the drag would report now.
    fn value(d: &Drag) -> Option<Val> {
        match &d.kind {
            Kind::New { .. } => {
                let r = drawn_rect(d.start, d.at);
                (r.width() >= 1.0 && r.height() >= 1.0).then_some(Val::Rect(r))
            }
            Kind::Move { .. } => {
                let by = d.at - d.start;
                Some(Val::By(Vec2::new(by.x.round(), by.y.round())))
            }
            Kind::Resize { rect, handle, .. } => {
                // ⇧ is read every pass, so it may go down or up mid-drag; ⌥ resizes about the centre.
                let by = d.at - d.start;
                let (keep, centre) = (d.modifiers.shift, d.modifiers.alt);
                Some(Val::Rect(if keep { resized_keep_ratio(*rect, *handle, by, centre) } else { resized_rect(*rect, *handle, by, centre) }))
            }
            Kind::Marquee => None,
        }
    }

    /// Takes the pointer's sample into the drag: begins it past the click slop, and reports the
    /// new value when `update`.
    fn advance(&mut self, d: &mut Drag, pointer: &ToolPointer, slop: f32, update: bool) {
        let at = if matches!(d.kind, Kind::Move { .. }) { pointer.raw } else { pointer.doc };
        if !at.is_finite() {
            return;
        }
        d.at = at;
        d.modifiers = pointer.modifiers;
        if !d.began && pointer.screen.distance(d.press) > slop && (matches!(d.kind, Kind::Marquee) || Self::value(d).is_some()) {
            d.began = true;
            self.events.extend(event(d, Phase::Begin, None));
        }
        if d.began && update {
            let value = Self::value(d);
            if value.is_some() && value != d.sent {
                d.sent = value;
                self.events.extend(event(d, Phase::Update, value));
            }
        }
    }

    /// The outlines the drag would leave.
    fn previews(&self, d: &Drag) -> Vec<Rect> {
        match (&d.kind, Self::value(d)) {
            (Kind::New { .. }, Some(Val::Rect(r))) | (Kind::Resize { .. }, Some(Val::Rect(r))) => vec![r],
            (Kind::Move { indices, .. }, Some(Val::By(by))) => indices.iter().filter_map(|&i| self.items.get(i)).map(|b| b.rect + by).collect(),
            (Kind::Marquee, _) => vec![Rect::from_points(d.start, d.at)],
            _ => Vec::new(),
        }
    }

    /// The screen rect of board `index`, for a part.
    fn board_part(&self, index: usize, cx: &ToolContext<'_>, labels: &ArtboardToolLabels) -> Option<Part> {
        let b = self.items.get(index)?;
        let r = b.rect.abs();
        let n = |v: f64| cx.numbers.format(v, 0..=0);
        let name = craft_i18n::fmt(
            &labels.board,
            &[("index", &(index + 1).to_string()), ("width", &n(r.width())), ("height", &n(r.height())), ("x", &n(r.x0)), ("y", &n(r.y0))],
        );
        Some(Part { key: index as u64, name, value: None, rect: egui::Rect::from_points(&cx.view.quad(r)), selected: self.selected.contains(&index) })
    }
}

impl CanvasTool for ArtboardTool {
    fn cursor(&self, cx: &ToolContext<'_>) -> ToolCursor {
        // pc: crates/ui-egui/src/tools/artboard_tool.rs:238-248 `cursor` (`tool_cursor::resize` by
        // handle; the pointing hand over a "+" button).
        // new: the move arrow inside an unlocked board (spec "Cursor").
        let Some(p) = cx.pointer else { return ToolCursor::System(CursorIcon::Default) };
        if self.drag.is_some() {
            return ToolCursor::System(CursorIcon::Default);
        }
        if self.rules.plus_buttons
            && let Some(index) = self.active()
            && self.plus_at(index, p.screen, cx).is_some()
        {
            return ToolCursor::System(CursorIcon::PointingHand);
        }
        if let Some((_, h)) = self.handle_at(p.doc, cx) {
            return ToolCursor::System(resize_cursor(cx.view, Vec2::new(f64::from(h[0]), f64::from(h[1]))));
        }
        match self.board_at(p.doc) {
            Some(i) if !self.items[i].locked => ToolCursor::System(CursorIcon::Move),
            _ => ToolCursor::System(CursorIcon::Default),
        }
    }

    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        // pc: crates/ui-egui/src/tools/artboard_tool.rs:173-223 `pointer` (`Down`: "+" first, then
        // handles, then inside, then a new board; the ⌘ test at :168 is the host's momentary Move).
        // inv-vectorcraft: crates/tools/src/xform/artboard.rs:1-22 (Illustrator's rules).
        let m = pointer.modifiers;
        let (at, id) = (pointer.doc, GestureId::next());
        let drag = |kind: Kind, start: Point| Drag { kind, id, press: pointer.screen, start, at: start, modifiers: m, began: false, sent: None };
        self.drag = None;
        let active = self.active();

        if self.rules.plus_buttons
            && let Some(index) = active
            && let Some(side) = self.plus_at(index, pointer.screen, cx)
        {
            self.events.push(ArtboardEvent::AddBeside { index, side, copy: m.alt });
            return ToolAction::Done;
        }
        if let Some((index, handle)) = self.handle_at(at, cx) {
            self.drag = Some(drag(Kind::Resize { index, rect: self.items[index].rect, handle }, at));
            return ToolAction::Claimed;
        }
        let Some(i) = self.board_at(at) else {
            // The pasteboard: Illustrator's ⇧-drag is a marquee over boards, any other drag draws.
            let kind = if self.rules.multi_select && m.shift { Kind::Marquee } else { Kind::New { toggle: None } };
            self.drag = Some(drag(kind, at));
            return ToolAction::Claimed;
        };
        if self.rules.multi_select && m.shift {
            // ⇧-click toggles the board; ⇧-drag draws a new one.
            self.drag = Some(drag(Kind::New { toggle: Some(i) }, at));
            return ToolAction::Claimed;
        }
        if self.rules.multi_select && pointer.clicks >= 2 {
            self.events.push(ArtboardEvent::OpenOptions { index: i });
            return ToolAction::Done;
        }
        let only_this = self.selected == [i];
        if self.items[i].locked {
            // A locked board selects but never moves.
            if !only_this {
                self.events.push(ArtboardEvent::Select { indices: vec![i], modifiers: m });
            }
            return ToolAction::Done;
        }
        let mut indices = vec![i];
        let mut select_on_click = None;
        if self.selected.contains(&i) {
            if self.selected.len() > 1 {
                indices = self.movable();
                select_on_click = Some(i);
            }
        } else {
            self.events.push(ArtboardEvent::Select { indices: vec![i], modifiers: m });
        }
        let copy = self.rules.alt_copies && m.alt;
        self.drag = Some(drag(Kind::Move { indices, copy, select_on_click }, pointer.raw));
        ToolAction::Claimed
    }

    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/artboard_tool.rs:163-170 `follow` (⇧ tracked live while
        // resizing), :143-152 `preview`.
        let Some(mut d) = self.drag.take() else { return };
        self.advance(&mut d, pointer, Self::style(cx.tokens).click_slop, true);
        if let Kind::Resize { handle, .. } = d.kind {
            let dir = Vec2::new(f64::from(handle[0]), f64::from(handle[1]));
            cx.set_cursor(ToolCursor::System(resize_cursor(cx.view, dir)));
        }
        self.drag = Some(d);
    }

    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/artboard_tool.rs:224-236 `pointer` (`Up`: one event per
        // gesture; a drag under `CLICK_PX` draws nothing :24).
        let Some(mut d) = self.drag.take() else { return };
        self.advance(&mut d, pointer, Self::style(cx.tokens).click_slop, false);
        let m = pointer.modifiers;
        if !d.began {
            // A click.
            match d.kind {
                Kind::New { toggle: Some(i) } => {
                    let mut indices = self.selected.clone();
                    if let Some(at) = indices.iter().position(|&k| k == i) {
                        indices.remove(at);
                    } else {
                        indices.insert(0, i);
                    }
                    self.events.push(ArtboardEvent::Select { indices, modifiers: m });
                }
                Kind::New { toggle: None } if self.rules.fit_object => self.events.push(ArtboardEvent::FitObject { at: d.start }),
                Kind::Move { select_on_click: Some(i), .. } => self.events.push(ArtboardEvent::Select { indices: vec![i], modifiers: m }),
                _ => {}
            }
            return;
        }
        let value = Self::value(&d);
        let changed = match (&d.kind, value) {
            (Kind::New { .. }, _) => true,
            (Kind::Move { copy, .. }, Some(Val::By(by))) => by != Vec2::ZERO || *copy,
            (Kind::Resize { rect, .. }, Some(Val::Rect(now))) => now != *rect,
            _ => false,
        };
        match d.kind {
            Kind::Marquee => {
                let touched = Rect::from_points(d.start, d.at);
                let indices = (0..self.items.len()).filter(|&i| overlaps(self.items[i].rect, touched)).collect();
                self.events.push(ArtboardEvent::Select { indices, modifiers: m });
            }
            // A board dragged to nothing wide or tall is no board.
            _ if value.is_none() => self.events.extend(event(&d, Phase::Cancel, None)),
            _ => self.events.extend(event(&d, Phase::Commit(changed), value)),
        }
    }

    fn hover(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // new: keeps `hovered_plus` for the cursor and the button's hover look.
        self.hovered_plus = if self.rules.plus_buttons { self.active().and_then(|i| self.plus_at(i, pointer.screen, cx)) } else { None };
    }

    fn cancel(&mut self, _cx: &mut ToolContext<'_>) {
        // new: the running `Draw`, `Move` or `Resize` reports `Cancel`; drops the drag.
        if let Some(d) = self.drag.take()
            && d.began
        {
            self.events.extend(event(&d, Phase::Cancel, None));
        }
    }

    fn key(&mut self, key: &ToolKey, _cx: &mut ToolContext<'_>) -> bool {
        // inv-vectorcraft: crates/tools/src/xform/artboard.rs:15-18 (arrows by the keyboard increment,
        // ⇧ ten times; Delete; Esc leaves).
        // new: Enter `OpenOptions`, Insert `AddBeside` on the right (AccessKit keyboard path);
        // ⌥+arrows resize from the end edges.
        if !key.pressed || self.drag.is_some() {
            return false;
        }
        let m = key.modifiers;
        if key.key == Key::Escape && m.matches_exact(Modifiers::NONE) {
            if self.rules.escape_leaves {
                self.events.push(ArtboardEvent::Leave);
                return true;
            }
            return false;
        }
        let Some(active) = self.active() else { return false };
        let plain = m.matches_exact(Modifiers::NONE);
        match key.key {
            Key::Delete | Key::Backspace if plain => {
                let indices = self.selected.iter().copied().filter(|&i| i < self.items.len()).collect();
                self.events.push(ArtboardEvent::Delete { indices });
                return true;
            }
            Key::Enter if plain => {
                self.events.push(ArtboardEvent::OpenOptions { index: active });
                return true;
            }
            Key::Insert if plain || m.matches_exact(Modifiers::ALT) => {
                self.events.push(ArtboardEvent::AddBeside { index: active, side: AddSide::Right, copy: m.alt });
                return true;
            }
            _ => {}
        }
        let step = self.rules.arrow_step;
        let Some(nudge) = NudgeKeys::new().steps(step, 10.0 * step).alt(true).from_key(key) else { return false };
        if nudge.alt {
            // The end edges (right, bottom) move; a locked board stays.
            if !self.items[active].locked {
                let handle = if nudge.by.x == 0.0 { [0, 1] } else { [1, 0] };
                let rect = resized_rect(self.items[active].rect, handle, nudge.by, false);
                self.events.push(ArtboardEvent::Resize { index: active, event: EditEvent::set(rect, m) });
            }
        } else {
            let indices = self.movable();
            if !indices.is_empty() {
                self.events.push(ArtboardEvent::Move { indices, event: EditEvent::set(nudge.by, m), copy: false });
            }
        }
        true
    }

    fn paint(&self, painter: &Painter, cx: &ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/artboard_tool.rs:250-284 `draw_overlay` (preview 1.5 pt accent
        // :259, the "+" buttons :262-274, 8 handles 7 × 7 white with a 1 pt accent edge :277-281).
        let style = Self::style(cx.tokens);
        let view = cx.view;
        let outline = |r: Rect| Shape::closed_line(view.quad(r.abs()).to_vec(), style.preview);
        let mut handles = self.active().filter(|&i| !self.items[i].locked).map(|i| self.items[i].rect);
        if let Some(d) = &self.drag {
            for r in self.previews(d) {
                painter.add(outline(r));
            }
            handles = match (&d.kind, Self::value(d)) {
                (Kind::Resize { .. }, Some(Val::Rect(r))) => Some(r),
                (Kind::Move { indices, .. }, Some(Val::By(by))) => self.active().filter(|i| indices.contains(i)).map(|i| self.items[i].rect + by),
                (Kind::New { .. } | Kind::Marquee, _) => None,
                _ => handles,
            };
        } else if self.rules.plus_buttons
            && let Some(i) = self.active()
        {
            AddBesideHandles::new("artboard-add-beside", self.items[i].rect).paint(
                painter,
                view,
                &AddBesideHandles::style(cx.tokens),
                self.hovered_plus,
            );
        }
        let Some(rect) = handles else { return };
        for h in HANDLES {
            let square = egui::Rect::from_center_size(view.to_screen(handle_point(rect, h)), vec2(style.handle, style.handle));
            painter.rect_filled(square, 0.0, style.handle_fill);
            painter.rect_stroke(square, 0.0, style.handle_edge, StrokeKind::Inside);
        }
    }

    fn snap_gesture(&self) -> Option<SnapGesture> {
        // new: `Point` while drawing or resizing, `Rect { at_press }` while moving.
        match &self.drag {
            Some(Drag { kind: Kind::Move { indices, .. }, .. }) => {
                let rects = indices.iter().filter_map(|&i| self.items.get(i)).map(|b| b.rect.abs());
                rects.reduce(|a, b| a.union(b)).map(|at_press| SnapGesture::Rect { at_press })
            }
            _ => Some(SnapGesture::Point),
        }
    }

    fn parts(&self, cx: &ToolContext<'_>) -> Option<ToolParts> {
        // new: one part per board (key = its index), named by `ArtboardToolLabels::board`.
        if self.items.is_empty() {
            return None;
        }
        let labels = self.labels.clone().unwrap_or_else(|| ArtboardToolLabels::of_ctx(&cx.canvas.ctx));
        let parts = (0..self.items.len()).filter_map(|i| self.board_part(i, cx, &labels)).collect();
        Some(ToolParts::new(labels.group.clone(), parts))
    }

    fn picked(&self) -> Option<u64> {
        self.selected.first().map(|&i| i as u64)
    }

    fn pick(&mut self, key: Option<u64>, _cx: &mut ToolContext<'_>) {
        // new: reports `Select { indices: [picked] }`.
        let Some(i) = key.and_then(|k| usize::try_from(k).ok()).filter(|&i| i < self.items.len()) else { return };
        if self.selected != [i] {
            self.events.push(ArtboardEvent::Select { indices: vec![i], modifiers: Modifiers::NONE });
        }
    }
}
