//! Select and Mask's tools, their options and the canvas tool.

use craft_behavior::geometry::draft::{DraftEvent, DraftKind, DraftPoint, DraftReach, OutlineDraft, turn_to_45};
use craft_behavior::kurbo::Point;
use craft_behavior::{CancelReason, Cursor, CursorBadge, CursorIcon, GestureId, PaintedCursor, PointerButton};
use craft_keymap::Mods;
use craft_ui::canvas::{CanvasTool, ToolAction, ToolContext, ToolKey, ToolPath, ToolPointer};
use craft_ui::canvas_controls::CanvasControlWords;
use craft_ui::canvas_controls::brush::{
    BrushCursor, BrushCursorOptions, BrushDimensions, BrushLevel, BrushResize, BrushResizeOptions, BrushTip, PaintingCursor,
};
use craft_ui::primitives::{Draw, PaintShape};
use craft_ui::rails::{RailTool, ToolSlot};
use egui::Pos2;

use super::SelectMaskEvent;
use super::commands::SelectMaskCommands;
use super::edits::{EditMode, ObjectTarget, RefineStroke, SelectMaskEdit, round_point};

/// The distance between the points of a freehand Lasso, in document pixels: a sampling step of the
/// edit's points, not a size on screen (PhotoCraft's).
const FREEHAND_STEP: f64 = 0.5;

/// How near a press closes a Polygonal Lasso outline, in screen points (PhotoCraft's 6; the kernel's is 8).
const CLOSE_REACH: f64 = 6.0;

/// An Object Selection drag shorter than this many screen points is a click.
const CLICK_REACH: f64 = 4.0;

/// The share of its colour the brush stroke being drawn shows: the accent at this opacity.
const TRAIL_SHARE: f32 = 0.43;

/// The arm of the crosshair that replaces a brush outline too small to read, in points.
const CROSSHAIR_REACH: f32 = 6.0;

/// The brush outline options Photoshop's defaults give.
pub(super) fn default_cursor_options() -> BrushCursorOptions {
    BrushCursorOptions::new(PaintingCursor::default(), CROSSHAIR_REACH)
}

/// What a tool edit or stroke reports as it goes: the gesture's lifecycle with the value built from it.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum EditPhase<T> {
    /// The gesture began. The value is the first point's edit, when it makes one.
    #[non_exhaustive]
    Begin {
        /// The gesture.
        id: GestureId,
        /// The edit at the press.
        value: Option<T>,
    },
    /// The gesture grew (only with Real-Time Refinement on): the edit so far.
    #[non_exhaustive]
    Update {
        /// The gesture.
        id: GestureId,
        /// The edit so far.
        value: T,
    },
    /// The gesture ended with this edit. The workspace has already recorded it in the session.
    #[non_exhaustive]
    Commit {
        /// The gesture.
        id: GestureId,
        /// The finished edit.
        value: T,
    },
    /// The gesture ended without an edit: Esc, a lost pointer, a tool switch, an undo, or the limits refused it.
    #[non_exhaustive]
    Cancel {
        /// The gesture.
        id: GestureId,
        /// Why it ended.
        reason: CancelReason,
    },
    /// A button made this edit at once (Select Subject, Clear Selection, Invert).
    #[non_exhaustive]
    Set {
        /// The edit's id.
        id: GestureId,
        /// The edit.
        value: T,
    },
}

impl<T> EditPhase<T> {
    /// The gesture began, with the edit at the press when it makes one.
    pub fn begin(id: GestureId, value: Option<T>) -> Self {
        Self::Begin { id, value }
    }

    /// The gesture grew: the edit so far.
    pub fn update(id: GestureId, value: T) -> Self {
        Self::Update { id, value }
    }

    /// The gesture ended with `value`.
    pub fn commit(id: GestureId, value: T) -> Self {
        Self::Commit { id, value }
    }

    /// The gesture ended without an edit, for `reason`.
    pub fn cancel(id: GestureId, reason: CancelReason) -> Self {
        Self::Cancel { id, reason }
    }

    /// A button made `value` at once.
    pub fn set(id: GestureId, value: T) -> Self {
        Self::Set { id, value }
    }
}

/// The tools of Select and Mask's strip, with Photoshop's letters. Quick Selection and Object
/// Selection share the W slot, Lasso and Polygonal Lasso the L slot.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SelectMaskTool {
    /// W.
    #[default]
    Quick,
    /// R.
    RefineEdge,
    /// B.
    Brush,
    /// W (Shift+W cycles with Quick Selection).
    Object,
    /// L.
    Lasso,
    /// L (Shift+L cycles with Lasso).
    PolygonalLasso,
    /// H.
    Hand,
    /// Z.
    Zoom,
}

impl SelectMaskTool {
    /// The eight tools, in strip order.
    pub const ALL: [SelectMaskTool; 8] = [
        SelectMaskTool::Quick,
        SelectMaskTool::RefineEdge,
        SelectMaskTool::Brush,
        SelectMaskTool::Object,
        SelectMaskTool::Lasso,
        SelectMaskTool::PolygonalLasso,
        SelectMaskTool::Hand,
        SelectMaskTool::Zoom,
    ];

    /// Paints with a round brush whose size `[` and `]` change: Quick Selection, Refine Edge
    /// Brush, Brush.
    pub fn brushes(self) -> bool {
        matches!(self, SelectMaskTool::Quick | SelectMaskTool::RefineEdge | SelectMaskTool::Brush)
    }

    /// Adds to or subtracts from the selection (the bar's + and −): all but Hand and Zoom.
    pub fn edits(self) -> bool {
        !matches!(self, SelectMaskTool::Hand | SelectMaskTool::Zoom)
    }

    /// The strip: slot W (Quick Selection, Object Selection), R, B, slot L (Lasso, Polygonal
    /// Lasso; `ending_group`), H, Z. Each tool runs its command, which gives its label, icon and letter.
    pub fn slots() -> Vec<ToolSlot<SelectMaskTool>> {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:1845-1868 (the strip, the shared L
        // slot with its corner triangle and menu), :101 `icon`.
        let tools = |tools: &[SelectMaskTool]| tools.iter().map(|tool| RailTool::new(*tool, SelectMaskCommands::of(*tool))).collect::<Vec<_>>();
        vec![
            ToolSlot::new(0, tools(&[SelectMaskTool::Quick, SelectMaskTool::Object])),
            ToolSlot::new(1, tools(&[SelectMaskTool::RefineEdge])),
            ToolSlot::new(2, tools(&[SelectMaskTool::Brush])),
            ToolSlot::ending_group(3, tools(&[SelectMaskTool::Lasso, SelectMaskTool::PolygonalLasso])),
            ToolSlot::new(4, tools(&[SelectMaskTool::Hand])),
            ToolSlot::new(5, tools(&[SelectMaskTool::Zoom])),
        ]
    }
}

/// The diameters the three brush tools remember apart (1..=5000; 30 each when the workspace
/// opens). Not in the history, not persisted.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct BrushSizes {
    /// The Quick Selection diameter.
    pub quick: f64,
    /// The Refine Edge Brush diameter.
    pub refine: f64,
    /// The Brush diameter.
    pub brush: f64,
}

impl Default for BrushSizes {
    fn default() -> Self {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:749-751.
        Self { quick: 30.0, refine: 30.0, brush: 30.0 }
    }
}

impl BrushSizes {
    /// The sizes' range.
    pub const RANGE: std::ops::RangeInclusive<f64> = 1.0..=5000.0;

    /// The size of `tool` (the Brush's for a tool without its own).
    pub fn get(&self, tool: SelectMaskTool) -> f64 {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:467 `size`.
        match tool {
            SelectMaskTool::Quick => self.quick,
            SelectMaskTool::RefineEdge => self.refine,
            _ => self.brush,
        }
    }

    /// Sets the size of a brush tool, rounded and clamped to [`Self::RANGE`]; a non-finite value
    /// or another tool changes nothing.
    pub fn set(&mut self, tool: SelectMaskTool, size: f64) {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:475 `set_size`.
        if !size.is_finite() {
            return;
        }
        let size = size.round().clamp(*Self::RANGE.start(), *Self::RANGE.end());
        match tool {
            SelectMaskTool::Quick => self.quick = size,
            SelectMaskTool::RefineEdge => self.refine = size,
            SelectMaskTool::Brush => self.brush = size,
            _ => {}
        }
    }

    /// `[` (`larger` false) and `]`: × 0.9 or × 1.1, at least one pixel of change.
    pub fn step(&mut self, tool: SelectMaskTool, larger: bool) {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:1405-1412.
        let size = self.get(tool);
        self.set(tool, if larger { (size * 1.1).max(size + 1.0) } else { (size * 0.9).min(size - 1.0) });
    }
}

/// The tools' options: the bar's values, outside the history and not persisted.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SelectMaskToolOptions {
    /// + (true) or − in the bar.
    pub add: bool,
    /// The diameters of the three brush tools.
    pub sizes: BrushSizes,
    /// The Brush's hardness, 0..=100 (100).
    pub hardness: f64,
    /// Sample All Layers (Quick Selection, Refine Edge Brush, Object Selection, Select Subject).
    pub sample_all_layers: bool,
}

impl Default for SelectMaskToolOptions {
    fn default() -> Self {
        Self { add: true, sizes: BrushSizes::default(), hardness: 100.0, sample_all_layers: false }
    }
}

/// The state of Select and Mask's canvas between passes: the gesture being drawn, the Polygonal
/// Lasso's polygon, the brush resize, the events not yet taken.
#[derive(Clone, Debug)]
pub struct SelectMaskCanvas {
    gesture: Option<(GestureId, SelectMaskTool, bool, Vec<Point>)>,
    polygon: Option<OutlineDraft>,
    /// The polygon's mode (+ or −, reversed by Alt at its first click).
    polygon_add: bool,
    resize: BrushResize,
    /// The size and hardness a resize started from, put back when it is cancelled.
    resize_start: Option<(f64, f64)>,
    /// Preferences › Cursors for the brush outline.
    cursor_options: BrushCursorOptions,
    /// The brush stroke just released, painted until the preview shows it
    /// ([`drop_trail`](Self::drop_trail)).
    trail: Option<(SelectMaskTool, Vec<Point>)>,
    events: Vec<SelectMaskEvent>,
    /// The id the next gesture takes.
    next: GestureId,
}

impl Default for SelectMaskCanvas {
    fn default() -> Self {
        Self {
            gesture: None,
            polygon: None,
            polygon_add: true,
            resize: BrushResize::default(),
            resize_start: None,
            cursor_options: default_cursor_options(),
            trail: None,
            events: Vec::new(),
            next: GestureId::default(),
        }
    }
}

impl SelectMaskCanvas {
    /// An idle canvas.
    pub fn new() -> Self {
        Self::default()
    }

    /// The canvas tool for this pass: `tool` with `options` (the resize writes the brush size).
    /// `real_time` reports `Update`s while a stroke is drawn.
    pub fn tool<'a>(&'a mut self, tool: SelectMaskTool, options: &'a mut SelectMaskToolOptions, real_time: bool) -> SelectMaskCanvasTool<'a> {
        SelectMaskCanvasTool { canvas: self, tool, options, real_time }
    }

    /// The user's Preferences › Cursors, which shape the brush outline; Photoshop's until set.
    pub fn cursor_options(&mut self, options: BrushCursorOptions) {
        self.cursor_options = options;
    }

    /// The events since the last call, in order.
    pub fn take_events(&mut self) -> Vec<SelectMaskEvent> {
        std::mem::take(&mut self.events)
    }

    /// Whether a Polygonal Lasso polygon is open: Enter closes it and Esc drops it before the
    /// dialog sees them.
    pub fn polygon_open(&self) -> bool {
        self.polygon.is_some()
    }

    /// Enter: closes an open polygon (3 points or more) into a Lasso edit. Returns whether there
    /// was a polygon.
    pub fn close_polygon(&mut self) -> bool {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:870 `enter`, :590 `close_polygon`.
        let Some(mut draft) = self.polygon.take() else { return false };
        if let Some(DraftEvent::Closed { points, .. }) = draft.close() {
            self.commit_polygon(&points);
        }
        true
    }

    /// Esc: drops an open polygon. Returns whether there was one.
    pub fn drop_polygon(&mut self) -> bool {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:881 `escape`.
        self.polygon.take().is_some()
    }

    /// The preview shows the released stroke (or failed to): stops painting it. A new press or a
    /// cancel drops it too.
    pub fn drop_trail(&mut self) {
        self.trail = None;
    }

    /// A released brush stroke is still painted, waiting for the preview.
    pub fn trail_shown(&self) -> bool {
        self.trail.is_some()
    }

    /// A gesture, a polygon or a resize is in progress: the history waits.
    pub fn busy(&self) -> bool {
        self.gesture.is_some() || self.polygon.is_some() || self.resize.active()
    }

    /// The brush resize in progress: where it started and its values.
    pub(super) fn resize(&self) -> &BrushResize {
        &self.resize
    }

    /// The id the next gesture takes.
    fn next_id(&mut self) -> GestureId {
        self.next = self.next.next();
        self.next
    }

    /// Reports the closed polygon as one gesture: `Begin` and `Commit` together.
    fn commit_polygon(&mut self, points: &[Point]) {
        let mode = if self.polygon_add { EditMode::Add } else { EditMode::Subtract };
        let edit = SelectMaskEdit::Lasso { points: points.iter().map(|p| round_point([p.x, p.y])).collect(), mode };
        let id = self.next_id();
        report(&mut self.events, false, id, Phase::Begin, Some(Built::Edit(edit.clone())));
        report(&mut self.events, false, id, Phase::Commit, Some(Built::Edit(edit)));
    }

    /// Drops the polygon once its draft is empty again.
    fn forget_closed_polygon(&mut self) {
        if self.polygon.as_ref().is_some_and(|draft| !draft.is_open()) {
            self.polygon = None;
        }
    }
}

/// An edit or a Refine Edge Brush stroke, built from a gesture.
#[derive(Clone, Debug)]
enum Built {
    Edit(SelectMaskEdit),
    Refine(RefineStroke),
}

/// Which event of a gesture to make.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Begin,
    Update,
    Commit,
    Cancel(CancelReason),
}

/// The event of `phase` for `value`: `Update` and `Commit` need a value.
fn make<T>(id: GestureId, phase: Phase, value: Option<T>) -> Option<EditPhase<T>> {
    match phase {
        Phase::Begin => Some(EditPhase::Begin { id, value }),
        Phase::Update => value.map(|value| EditPhase::Update { id, value }),
        Phase::Commit => value.map(|value| EditPhase::Commit { id, value }),
        Phase::Cancel(reason) => Some(EditPhase::Cancel { id, reason }),
    }
}

/// Appends the event of `phase`. `refine` says which kind a value-less event belongs to.
fn report(events: &mut Vec<SelectMaskEvent>, refine: bool, id: GestureId, phase: Phase, built: Option<Built>) {
    match built {
        Some(Built::Edit(edit)) => events.extend(make(id, phase, Some(edit)).map(SelectMaskEvent::Edit)),
        Some(Built::Refine(stroke)) => events.extend(make(id, phase, Some(stroke)).map(SelectMaskEvent::Refine)),
        None if refine => events.extend(make::<RefineStroke>(id, phase, None).map(SelectMaskEvent::Refine)),
        None => events.extend(make::<SelectMaskEdit>(id, phase, None).map(SelectMaskEvent::Edit)),
    }
}

/// What the gesture of `tool` over `points` makes, or `None` while it makes nothing (a Lasso with
/// fewer than 3 points). `zoom` turns an Object Selection drag into screen points.
fn build(tool: SelectMaskTool, options: &SelectMaskToolOptions, add: bool, points: &[Point], zoom: f64) -> Option<Built> {
    // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:554 `finish`.
    let mode = if add { EditMode::Add } else { EditMode::Subtract };
    let first = *points.first()?;
    let list = || points.iter().map(|p| round_point([p.x, p.y])).collect::<Vec<_>>();
    Some(match tool {
        SelectMaskTool::Quick => {
            Built::Edit(SelectMaskEdit::Quick { points: list(), size: options.sizes.quick, mode, sample_all_layers: options.sample_all_layers })
        }
        SelectMaskTool::Brush => Built::Edit(SelectMaskEdit::Brush { points: list(), size: options.sizes.brush, hardness: options.hardness, mode }),
        SelectMaskTool::RefineEdge => Built::Refine(RefineStroke { points: list(), size: options.sizes.refine, mode }),
        SelectMaskTool::Lasso | SelectMaskTool::PolygonalLasso if points.len() >= 3 => Built::Edit(SelectMaskEdit::Lasso { points: list(), mode }),
        SelectMaskTool::Object => {
            let last = *points.last()?;
            let (width, height) = (last.x - first.x, last.y - first.y);
            let target = if width.abs().max(height.abs()) * zoom < CLICK_REACH {
                ObjectTarget::Point(round_point([first.x, first.y]))
            } else {
                let (origin, size) = (round_point([first.x, first.y]), round_point([width, height]));
                ObjectTarget::Rect([origin[0], origin[1], size[0], size[1]])
            };
            Built::Edit(SelectMaskEdit::Object { target, mode, sample_all_layers: options.sample_all_layers })
        }
        SelectMaskTool::Lasso | SelectMaskTool::PolygonalLasso | SelectMaskTool::Hand | SelectMaskTool::Zoom => return None,
    })
}

/// [`SelectMaskCanvas`] as the `CanvasTool` of one pass.
///
/// - **Quick Selection, Brush, Refine Edge Brush**: press, drag, release is one stroke; a point
///   joins when it is at least `size / 8` (at least 1) pixels from the last. Reported as
///   [`SelectMaskEvent::Edit`] (or [`SelectMaskEvent::Refine`] for the Refine Edge Brush) Begin,
///   Update with the points so far (only with Real-Time Refinement on), Commit with the finished
///   `SelectMaskEdit` or `RefineStroke` (the bar's mode, reversed by Alt at the press; the
///   tool's size; the Brush's hardness; Sample All Layers). The workspace records the commit
///   within the limits.
/// - **Lasso**: a freehand outline (points 0.5 px apart), committed as `SelectMaskEdit::Lasso`
///   when it has 3 points or more.
/// - **Polygonal Lasso**: an [`OutlineDraft`] of `DraftKind::Polygonal` that closes within 6 screen points of its
///   first vertex: a click adds a vertex, a click on the first vertex or a double click closes; Shift turns a vertex
///   to 45 degrees; Enter closes, Escape drops, Backspace and Delete remove the last vertex.
/// - **Object Selection**: a click, or a drag shorter than 4 screen points, picks the object under
///   the press (`ObjectTarget::Point`); a longer drag gives `ObjectTarget::Rect` from the press
///   to the release.
/// - **Brush resize**: Alt + right-drag or Control + Alt + drag ([`BrushResize`] with
///   `BrushLevel::Hardness` for the Brush; the size only for the others, `max_diameter` 5000).
/// - Paint: the stroke as discs while the gesture is open, the outline and rectangle as marching ants, the polygon
///   with its rubber band; the brush outline is the pointer (`BrushCursor`). The resize preview is a canvas control
///   the workspace shows from [`SelectMaskCanvas::resize`].
pub struct SelectMaskCanvasTool<'a> {
    canvas: &'a mut SelectMaskCanvas,
    tool: SelectMaskTool,
    options: &'a mut SelectMaskToolOptions,
    real_time: bool,
}

impl SelectMaskCanvasTool<'_> {
    /// The hardness (0..=1) a resize of the tool starts from: the Brush's; the others are hard.
    fn level(&self) -> f64 {
        if self.tool == SelectMaskTool::Brush { self.options.hardness / 100.0 } else { 1.0 }
    }

    /// Ends the stroke, the outline and the resize in progress without recording them (the host's `cancel`, and the
    /// workspace when the host has no gesture to cancel). A cancelled resize puts the size back.
    pub(super) fn abandon(&mut self, reason: CancelReason) {
        if self.canvas.resize.cancel().is_some()
            && let Some((size, hardness)) = self.canvas.resize_start.take()
        {
            self.options.sizes.set(self.tool, size);
            self.options.hardness = hardness;
        }
        self.canvas.resize_start = None;
        self.canvas.trail = None;
        if let Some((id, tool, _, _)) = self.canvas.gesture.take() {
            report(&mut self.canvas.events, tool == SelectMaskTool::RefineEdge, id, Phase::Cancel(reason), None);
        }
        // A click that was cut short leaves the polygon open: Esc between clicks is the dialog's.
        if let Some(draft) = self.canvas.polygon.as_mut()
            && draft.is_down()
            && let Some(&last) = draft.points().last()
        {
            draft.release(DraftPoint::new(last, Mods::NONE, 1.0));
        }
    }

    /// What the gesture over `points` makes now.
    fn built(&self, tool: SelectMaskTool, add: bool, points: &[Point], zoom: f64) -> Option<Built> {
        build(tool, self.options, add, points, zoom)
    }

    /// Writes the values of a resize to the options.
    fn apply_resize(&mut self, dimensions: BrushDimensions) {
        self.options.sizes.set(self.tool, dimensions.diameter);
        if self.tool == SelectMaskTool::Brush {
            self.options.hardness = (dimensions.level * 100.0).clamp(0.0, 100.0);
        }
    }

    /// The resize's readout beside the pointer. Only the Brush has a hardness to show.
    fn resize_readout(&self, cx: &mut ToolContext<'_>) {
        if self.tool != SelectMaskTool::Brush {
            return;
        }
        let rows = self.canvas.resize.readout_rows(&cx.units(), &cx.words::<CanvasControlWords>());
        if !rows.is_empty() {
            cx.readout(rows);
        }
    }

    fn begin_resize(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        // pc: crates/ui-egui/src/tools/brush_resize.rs:71 `pointer` (Down).
        let Some(space) = cx.space() else { return ToolAction::Ignored };
        let options = BrushResizeOptions::new(*BrushSizes::RANGE.end()).level(BrushLevel::Hardness);
        let start = BrushDimensions::new(self.options.sizes.get(self.tool), self.level());
        if self.canvas.resize.begin(&space.view, pointer.pos, start, &options).is_none() {
            return ToolAction::Ignored;
        }
        self.canvas.resize_start = Some((start.diameter, self.options.hardness));
        self.resize_readout(cx);
        ToolAction::Claimed
    }

    fn press_polygon(&mut self, pointer: &ToolPointer, doc: Point, zoom: f64, cx: &ToolContext<'_>) -> ToolAction {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:590-603 `begin` (PolygonalLasso), :1796-1802.
        let reach = DraftReach::new(CLOSE_REACH, f64::from(cx.slop));
        let add = self.options.add;
        let canvas = &mut *self.canvas;
        let draft = canvas.polygon.get_or_insert_with(|| OutlineDraft::new(DraftKind::Polygonal, reach));
        draft.set_reach(reach);
        let sample = DraftPoint::new(doc, pointer.mods, zoom);
        let event = if draft.is_open() && pointer.clicks >= 2 { draft.close() } else { draft.press(sample, true) };
        match event {
            Some(DraftEvent::Started { intent, .. }) => canvas.polygon_add = add != intent.alt,
            Some(DraftEvent::Closed { points, .. }) => canvas.commit_polygon(&points),
            _ => {}
        }
        canvas.forget_closed_polygon();
        ToolAction::Claimed
    }

    /// Adds the pointer to the gesture and reports the stroke so far when Real-Time Refinement is on.
    fn extend(&mut self, at: Point, zoom: f64) {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:620 `extend`.
        let Some((id, tool, add, points)) = self.canvas.gesture.as_mut() else { return };
        if *tool == SelectMaskTool::Object {
            // Object Selection keeps the press and the current corner.
            points.truncate(1);
            points.push(at);
        } else {
            let step = if tool.brushes() { (self.options.sizes.get(*tool) / 8.0).max(1.0) } else { FREEHAND_STEP };
            if points.last().is_some_and(|last| (*last - at).hypot() < step) {
                return;
            }
            points.push(at);
        }
        if self.real_time {
            let (id, add, points, tool) = (*id, *add, points.clone(), *tool);
            let built = self.built(tool, add, &points, zoom);
            report(&mut self.canvas.events, tool == SelectMaskTool::RefineEdge, id, Phase::Update, built);
        }
    }
}

impl CanvasTool for SelectMaskCanvasTool<'_> {
    fn each_pass(&mut self, _: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:1767 `canvas_input`, :484 `pick` (a
        // polygon survives no other tool).
        if self.tool != SelectMaskTool::PolygonalLasso {
            self.canvas.polygon = None;
        }
    }

    fn cursor(&self, cx: &ToolContext<'_>) -> Cursor {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:1730 `paint_cursor`.
        if self.canvas.resize.active() {
            return Cursor::Painted(PaintedCursor::Hidden);
        }
        let Some(space) = cx.space() else { return Cursor::System(CursorIcon::Default) };
        if self.tool.brushes() {
            let size = self.options.sizes.get(self.tool);
            let tip = BrushTip::round(size, size / 2.0);
            // The bar's mode, reversed while Alt is held (the press reads it the same way).
            let badge = if self.options.add != cx.mods.alt { CursorBadge::Add } else { CursorBadge::Subtract };
            return BrushCursor::new(&tip).painting(self.canvas.gesture.is_some()).badge(Some(badge)).cursor(
                &self.canvas.cursor_options,
                &space.view,
                cx.caps_lock(),
            );
        }
        Cursor::Painted(PaintedCursor::Crosshair)
    }

    fn takes(&self, button: PointerButton, mods: Mods) -> bool {
        // The brush resize chord's secondary button is the brushes' only (a plain right-click stays
        // the canvas menu's).
        if self.tool.brushes() { BrushResize::takes(button, mods) } else { button == PointerButton::Primary }
    }

    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:599 `begin`.
        let tool = self.tool;
        let Some(space) = cx.space() else { return ToolAction::Ignored };
        if !tool.edits() {
            return ToolAction::Ignored;
        }
        if tool.brushes() && BrushResize::chord(pointer.button, pointer.mods) {
            return self.begin_resize(pointer, cx);
        }
        let doc = space.view.to_doc(pointer.pos);
        if tool == SelectMaskTool::PolygonalLasso {
            return self.press_polygon(pointer, doc, space.view.zoom, cx);
        }
        let add = self.options.add != pointer.mods.alt;
        let id = self.canvas.next_id();
        let points = vec![doc];
        let built = self.built(tool, add, &points, space.view.zoom);
        self.canvas.trail = None;
        self.canvas.gesture = Some((id, tool, add, points));
        report(&mut self.canvas.events, tool == SelectMaskTool::RefineEdge, id, Phase::Begin, built);
        ToolAction::Claimed
    }

    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:620 `extend`.
        let Some(space) = cx.space() else { return };
        if self.canvas.resize.active() {
            if let Some(step) = self.canvas.resize.track(&space.view, pointer.pos) {
                self.apply_resize(step.dimensions);
            }
            self.resize_readout(cx);
            return;
        }
        let doc = space.view.to_doc(pointer.pos);
        if self.tool == SelectMaskTool::PolygonalLasso && self.canvas.gesture.is_none() {
            if let Some(draft) = self.canvas.polygon.as_mut()
                && draft.drag(DraftPoint::new(doc, pointer.mods, space.view.zoom)).is_some()
            {
                cx.request_repaint();
            }
            return;
        }
        self.extend(doc, space.view.zoom);
    }

    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:1367-1370 (Up), :554 `finish`, :579
        // `end_gesture`.
        let Some(space) = cx.space() else { return };
        if self.canvas.resize.active() {
            if let Some(step) = self.canvas.resize.finish(&space.view, pointer.pos) {
                self.apply_resize(step.dimensions);
            }
            self.canvas.resize_start = None;
            return;
        }
        let doc = space.view.to_doc(pointer.pos);
        if self.tool == SelectMaskTool::PolygonalLasso && self.canvas.gesture.is_none() {
            let canvas = &mut *self.canvas;
            if let Some(draft) = canvas.polygon.as_mut()
                && let Some(DraftEvent::Closed { points, .. }) = draft.release(DraftPoint::new(doc, pointer.mods, space.view.zoom))
            {
                canvas.commit_polygon(&points);
            }
            canvas.forget_closed_polygon();
            return;
        }
        self.extend(doc, space.view.zoom);
        let Some((id, tool, add, points)) = self.canvas.gesture.take() else { return };
        // The value comes from the gesture's own tool: the strip may have changed since the press.
        let built = self.built(tool, add, &points, space.view.zoom);
        let phase = if built.is_some() { Phase::Commit } else { Phase::Cancel(CancelReason::Escape) };
        if built.is_some() && tool.brushes() {
            self.canvas.trail = Some((tool, points));
        }
        report(&mut self.canvas.events, tool == SelectMaskTool::RefineEdge, id, phase, built);
    }

    fn hover(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:1363-1366 (Move: the hover point, the
        // polygon's rubber band).
        let Some(space) = cx.space() else { return };
        let Some(draft) = self.canvas.polygon.as_mut() else { return };
        let mut at = DraftPoint::new(space.view.to_doc(pointer.pos), pointer.mods, space.view.zoom);
        if pointer.mods.shift
            && !pointer.mods.alt
            && let Some(&last) = draft.points().last()
        {
            at.at = turn_to_45(last, at.at);
        }
        if draft.hover(at).is_some() {
            cx.request_repaint();
        }
    }

    fn cancel(&mut self, reason: CancelReason, _: &mut ToolContext<'_>) {
        // new: Esc or a lost pointer drops the gesture without an edit.
        self.abandon(reason);
    }

    fn key(&mut self, key: &ToolKey, _: &mut ToolContext<'_>) -> bool {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:1413-1419 (Backspace, Delete), :870 `enter`, :881
        // `escape`. Only an open polygon takes keys.
        if self.canvas.polygon.is_none() {
            return false;
        }
        match key {
            ToolKey::Delete { .. } => {
                let retracted = self.canvas.polygon.as_mut().is_some_and(|draft| draft.retract().is_some());
                self.canvas.forget_closed_polygon();
                retracted
            }
            ToolKey::Submit => self.canvas.close_polygon(),
            ToolKey::Cancel => self.canvas.drop_polygon(),
            _ => false,
        }
    }

    fn paint(&self, draw: &Draw<'_>, cx: &ToolContext<'_>) {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:1658 `paint_canvas` (trail, polygon).
        let (Some(space), Some(style)) = (cx.space(), cx.style()) else { return };
        let canvas = &*self.canvas;
        let view = space.view;
        let ink = style.color(style.tokens.color.accent.solid.faded(TRAIL_SHARE));
        let phase = 0.0;
        let screen = |points: &[Point]| points.iter().map(|p| view.to_screen(*p)).collect::<Vec<Pos2>>();
        let stroke = |tool: SelectMaskTool, points: &[Point]| {
            let radius = (self.options.sizes.get(tool) / 2.0 * view.zoom) as f32;
            for at in screen(points) {
                draw.fill(&PaintShape::Circle(at, radius), ink);
            }
        };
        // The released stroke stays until the preview shows it.
        if canvas.gesture.is_none()
            && let Some((tool, points)) = &canvas.trail
        {
            stroke(*tool, points);
        }
        if let Some((_, tool, _, points)) = &canvas.gesture {
            match tool {
                SelectMaskTool::Quick | SelectMaskTool::Brush | SelectMaskTool::RefineEdge => stroke(*tool, points),
                SelectMaskTool::Lasso if points.len() >= 2 => cx.marching_ants(&screen(points), false, phase),
                SelectMaskTool::Object => {
                    if let (Some(&a), Some(&b)) = (points.first(), points.last()) {
                        let corners = [a, Point::new(b.x, a.y), b, Point::new(a.x, b.y)];
                        cx.marching_ants(&screen(&corners), true, phase);
                    }
                }
                _ => {}
            }
        }
        if let Some(draft) = &canvas.polygon {
            if draft.points().len() >= 2 {
                cx.marching_ants(&screen(draft.points()), false, phase);
            }
            if let Some([a, b]) = draft.rubber_band() {
                cx.marching_ants(&screen(&[a, b]), false, phase);
            }
        }
    }

    fn command_form(&self) -> ToolPath {
        match self.tool {
            SelectMaskTool::Object => ToolPath::Command("the Select Subject command"),
            _ => ToolPath::Freehand,
        }
    }
}
