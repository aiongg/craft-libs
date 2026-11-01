//! [`SliceTool`]: the Slice and Slice Select tools on the tool host (task SH-T4).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_theme::Srgba;
use egui::Painter;
use kurbo::{Point, Rect};

use super::{SliceEvent, SliceItem, SliceMode, SliceRules, SliceToolLabels, SliceToolSettings};
use craft_ui::canvas::{CanvasTool, SnapGesture, ToolAction, ToolContext, ToolCursor, ToolKey, ToolParts, ToolPointer};
use craft_ui::edit::GestureId;

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
/// - Snaps the pointer (`SnapGesture::Point`).
///
/// Select mode:
/// - Click selects: stored slices win over auto slices, the latest on top; hidden auto slices
///   ([`hide_auto`](Self::hide_auto)) can't be picked. ⇧-click adds or removes with
///   [`SliceRules::multi_select`].
/// - A drag moves the whole slice, also when the press hit a piece of a covered slice; auto and
///   layer slices don't move (the press only selects).
/// - A handle of a selected user slice (8: corners first, `handle.hit` reach, as
///   [`handle_at`](crate::geometry::slice::handle_at)) resizes it with
///   [`resized_rect`](crate::geometry::slice::resized_rect).
/// - Delete or Backspace without modifiers, with a slice selected, reports
///   `Command(Delete)` and takes the key, so Layer › Delete never sees it (an auto slice: nothing,
///   the key is not taken). Double click and Enter report `Command(Options)`. Arrows move a
///   selected user slice by `NudgeKeys::from_key` while the canvas has the focus.
///
/// Locked ([`locked`](Self::locked)): a press selects, draws and moves nothing and reports
/// `Refused(Locked)`.
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
    colour: Option<Srgba>,
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
    New { id: GestureId, start: Point, at: Point, began: bool },
    Move { id: GestureId, number: usize, rect: Rect, start: Point, at: Point, began: bool },
    Resize { id: GestureId, number: usize, rect: Rect, handle: [i8; 2], start: Point, at: Point },
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
    pub fn colour(&mut self, colour: Option<Srgba>) -> &mut Self {
        self.colour = colour;
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

    /// The slice a Select press at `at` picks: the topmost stored slice containing it, then the
    /// topmost auto slice unless hidden.
    fn pick(&self, at: Point) -> Option<&SliceItem> {
        // pc: crates/ui-egui/src/tools/slice_ui.rs:100-105 `slice_at`, :172-185 (a covered piece
        // moves its whole slice).
        todo!()
    }

    /// The new slice from `start` to `at` under the settings and `modifiers`.
    fn new_rect(&self, start: Point, at: Point, modifiers: egui::Modifiers) -> Rect {
        // pc: crates/ui-egui/src/tools/slice_ui.rs:113-148 `new_rect`.
        todo!()
    }
}

impl CanvasTool for SliceTool {
    fn cursor(&self, cx: &ToolContext<'_>) -> ToolCursor {
        // new: Slice: the crosshair; Select: a resize arrow over a handle of the selected user
        // slice (`resize_cursor`, turned with the view), the move arrow over a movable slice, else
        // the default arrow.
        todo!()
    }

    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        // pc: crates/ui-egui/src/tools/slice_ui.rs:152-200 `pointer` (`Down`: locked at :161-166,
        // Slice starts `New`, Select picks, a handle starts `Resize`).
        todo!()
    }

    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/slice_ui.rs:152-231 `pointer` (`Drag`).
        todo!()
    }

    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/slice_ui.rs:152-231 `pointer` (`Up`: Normal needs a drag over 2 px,
        // Fixed Size a click); `ToolPointer::clicks` == 2 reports `Command(Options)`.
        todo!()
    }

    fn cancel(&mut self, cx: &mut ToolContext<'_>) {
        // new: the running `Draw`, `Move` or `Resize` reports `Cancel`; drops the drag.
        todo!()
    }

    fn key(&mut self, key: &ToolKey, cx: &mut ToolContext<'_>) -> bool {
        // pc: crates/ui-egui/src/tools/slice_ui.rs:339-368 `keys` (Delete or Backspace, no modifier,
        // Select, a selected slice; auto: nothing).
        // new: Enter reports `Command(Options)`; arrows through `NudgeKeys::from_key`.
        todo!()
    }

    fn paint(&self, painter: &Painter, cx: &ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/slice_ui.rs:242-321 `draw_overlay` (handles at :271-281, the
        // drag previews at :305-319). Lead call 4: handles are `handle.size` squares in
        // `overlay.handle_fill` with the selected colour's edge, not PhotoCraft's 5 × 5 filled ones.
        // new: `SliceOverlay::paint`, then the handles and the preview with `SliceTool::style`.
        todo!()
    }

    fn snap_gesture(&self) -> Option<SnapGesture> {
        // new: `Point` while drawing or resizing, `Rect { at_press }` while moving.
        todo!()
    }

    fn parts(&self, cx: &ToolContext<'_>) -> Option<ToolParts> {
        // new: one part per visible slice (key = its number), named by `SliceToolLabels::slice`.
        todo!()
    }

    fn picked(&self) -> Option<u64> {
        self.selected.map(|n| n as u64)
    }

    fn pick(&mut self, key: Option<u64>, cx: &mut ToolContext<'_>) {
        // new: reports `Select { number }` for the picked part.
        todo!()
    }
}
