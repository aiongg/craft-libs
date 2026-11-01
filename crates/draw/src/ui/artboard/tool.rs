//! [`ArtboardTool`]: the Artboard tool on the tool host (task SH-T5).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::Painter;
use kurbo::{Point, Rect};

use super::{ArtboardEvent, ArtboardItem, ArtboardRules, ArtboardToolLabels};
use crate::ui::add_beside::AddSide;
use craft_ui::canvas::{CanvasTool, SnapGesture, ToolAction, ToolContext, ToolCursor, ToolKey, ToolParts, ToolPointer};
use craft_ui::edit::GestureId;

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

/// A gesture between passes.
#[derive(Clone, Debug)]
enum Drag {
    New { id: GestureId, start: Point, at: Point, began: bool },
    Move { id: GestureId, indices: Vec<usize>, start: Point, at: Point, copy: bool, began: bool },
    Resize { id: GestureId, index: usize, rect: Rect, handle: [i8; 2], start: Point, at: Point, keep: bool },
    Marquee { start: Point, at: Point },
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
        // pc: crates/ui-egui/src/tools/artboard_tool.rs:74-77 `board_at`.
        todo!()
    }
}

impl CanvasTool for ArtboardTool {
    fn cursor(&self, cx: &ToolContext<'_>) -> ToolCursor {
        // pc: crates/ui-egui/src/tools/artboard_tool.rs:245-255 `cursor` (`tool_cursor::resize` by
        // handle; the pointing hand over a "+" button).
        todo!()
    }

    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        // pc: crates/ui-egui/src/tools/artboard_tool.rs:184-243 `pointer` (`Down`: ⌘ is the Move
        // tool's :186-190; "+" first, then handles, then inside, then a new board).
        // inv-vectorcraft: crates/tools/src/xform/artboard.rs:1-22 (Illustrator's rules).
        todo!()
    }

    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/artboard_tool.rs:176-182 `follow` (⇧ tracked live while
        // resizing), :155-165 `preview`.
        todo!()
    }

    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/artboard_tool.rs:184-243 `pointer` (`Up`: one event per
        // gesture; a drag under `CLICK_PX` draws nothing :24).
        todo!()
    }

    fn hover(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // new: keeps `hovered_plus` for the cursor and the button's hover look.
        todo!()
    }

    fn cancel(&mut self, cx: &mut ToolContext<'_>) {
        // new: the running `Draw`, `Move` or `Resize` reports `Cancel`; drops the drag.
        todo!()
    }

    fn key(&mut self, key: &ToolKey, cx: &mut ToolContext<'_>) -> bool {
        // inv-vectorcraft: crates/tools/src/xform/artboard.rs:15-18 (arrows by the keyboard increment,
        // ⇧ ten times; Delete; Esc leaves).
        // new: Enter `OpenOptions`, Insert `AddBeside` on the right (AccessKit keyboard path).
        todo!()
    }

    fn paint(&self, painter: &Painter, cx: &ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/artboard_tool.rs:257-291 `draw_overlay` (preview 1.5 pt accent
        // :266, the "+" buttons :269-281, 8 handles 7 × 7 white with a 1 pt accent edge :284-288).
        todo!()
    }

    fn snap_gesture(&self) -> Option<SnapGesture> {
        // new: `Point` while drawing or resizing, `Rect { at_press }` while moving.
        todo!()
    }

    fn parts(&self, cx: &ToolContext<'_>) -> Option<ToolParts> {
        // new: one part per board (key = its index), named by `ArtboardToolLabels::board`.
        todo!()
    }

    fn picked(&self) -> Option<u64> {
        self.selected.first().map(|&i| i as u64)
    }

    fn pick(&mut self, key: Option<u64>, cx: &mut ToolContext<'_>) {
        // new: reports `Select { indices: [picked] }`.
        todo!()
    }
}
