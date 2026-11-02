//! Select and Mask's tools, their options and the canvas tool (task WS-SM2).

use egui::{CursorIcon, Key, Modifiers, Painter, PointerButton, Shape, Stroke};

use craft_ui::canvas::brush::{BrushCursor, BrushDimensions, BrushResize, BrushResizeLabels, BrushResizeOptions, BrushStyle, BrushTip, TipFalloff};
use craft_ui::canvas::draft::{DraftEvent, DraftKind, DraftPoint, DraftReach, OutlineDraft, turn_to_45};
use craft_ui::canvas::kurbo::{Point, Rect};
use craft_ui::canvas::{CanvasTool, CursorOptions, Marquee, MarqueeLook, ToolAction, ToolContext, ToolCursor, ToolKey, ToolPointer};
use craft_ui::edit::{CancelReason, EditEvent, GestureId};
use craft_ui::icons::Icon;
use craft_ui::localized::{Labels as _, Units};
use craft_ui::patterns::{RailTool, ToolSlot};
use craft_ui::theme::ToEgui as _;

use super::SelectMaskEvent;
use super::SelectMaskLabels;
use super::edits::{EditMode, ObjectTarget, RefineStroke, SelectMaskEdit, round_point};
use crate::ui::select::SelectToolStyle;

/// The distance between the points of a freehand Lasso, in document pixels: a sampling step of the
/// edit's points, not a size on screen (PhotoCraft's).
const FREEHAND_STEP: f64 = 0.5;

/// The tools of Select and Mask's strip, with Photoshop's letters. Quick Selection and Object
/// Selection share the W slot, Lasso and Polygonal Lasso the L slot.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
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

    /// W R B W L L H Z.
    pub fn shortcut(self) -> char {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:114 `tip`.
        match self {
            SelectMaskTool::Quick | SelectMaskTool::Object => 'W',
            SelectMaskTool::RefineEdge => 'R',
            SelectMaskTool::Brush => 'B',
            SelectMaskTool::Lasso | SelectMaskTool::PolygonalLasso => 'L',
            SelectMaskTool::Hand => 'H',
            SelectMaskTool::Zoom => 'Z',
        }
    }

    /// The strip icon, by meaning (PhotoCraft's `icon` names).
    pub(super) fn icon(self) -> Icon {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:101 `icon`.
        match self {
            SelectMaskTool::Quick => Icon::QuickSelection,
            SelectMaskTool::RefineEdge => Icon::BrushCleaning,
            SelectMaskTool::Brush => Icon::Brush,
            SelectMaskTool::Object => Icon::ObjectSelect,
            SelectMaskTool::Lasso => Icon::Lasso,
            SelectMaskTool::PolygonalLasso => Icon::Pentagon,
            SelectMaskTool::Hand => Icon::Hand,
            SelectMaskTool::Zoom => Icon::ZoomIn,
        }
    }

    /// The tool's name in `labels`.
    pub(super) fn label(self, labels: &SelectMaskLabels) -> &str {
        match self {
            SelectMaskTool::Quick => &labels.quick,
            SelectMaskTool::RefineEdge => &labels.refine_edge,
            SelectMaskTool::Brush => &labels.brush,
            SelectMaskTool::Object => &labels.object,
            SelectMaskTool::Lasso => &labels.lasso,
            SelectMaskTool::PolygonalLasso => &labels.polygonal_lasso,
            SelectMaskTool::Hand => &labels.hand,
            SelectMaskTool::Zoom => &labels.zoom,
        }
    }

    /// The tool as a strip entry.
    fn entry(self, labels: &SelectMaskLabels) -> RailTool<SelectMaskTool> {
        RailTool::new(self, self.icon(), self.label(labels)).shortcut(self.shortcut())
    }

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
    /// Lasso; a right-click opens the flyout), `group_end` after it, H, Z. Labels from `labels`,
    /// letters as each tool's `shortcut`; `ToolRail::pick_by_key` gives W, L and their Shift
    /// cycles.
    pub fn slots(labels: &SelectMaskLabels) -> Vec<ToolSlot<SelectMaskTool>> {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:1845-1868 (the strip, the shared L
        // slot with its corner triangle and menu), :101 `icon`.
        let slot = |id: u32, tools: &[SelectMaskTool]| ToolSlot::new(id, tools.iter().map(|tool| tool.entry(labels)).collect());
        vec![
            slot(0, &[SelectMaskTool::Quick, SelectMaskTool::Object]),
            slot(1, &[SelectMaskTool::RefineEdge]),
            slot(2, &[SelectMaskTool::Brush]),
            slot(3, &[SelectMaskTool::Lasso, SelectMaskTool::PolygonalLasso]).group_end(true),
            slot(4, &[SelectMaskTool::Hand]),
            slot(5, &[SelectMaskTool::Zoom]),
        ]
    }
}

/// The diameters the three brush tools remember apart (1..=5000; 30 each when the workspace
/// opens). Not in the history, not persisted.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BrushSizes {
    pub quick: f64,
    pub refine: f64,
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
pub struct SelectMaskToolOptions {
    /// + (true) or − in the bar.
    pub add: bool,
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
#[derive(Debug, Default)]
pub struct SelectMaskCanvas {
    gesture: Option<(GestureId, SelectMaskTool, bool, Vec<Point>)>,
    polygon: Option<OutlineDraft>,
    /// The polygon's mode (+ or −, reversed by Alt at its first click).
    polygon_add: bool,
    resize: BrushResize,
    /// The size and hardness a resize started from, put back when it is cancelled.
    resize_start: Option<(f64, f64)>,
    /// Preferences › Cursors for the brush outline.
    cursor_options: CursorOptions,
    events: Vec<SelectMaskEvent>,
}

impl SelectMaskCanvas {
    pub fn new() -> Self {
        Self::default()
    }

    /// The canvas tool for this pass: `tool` with `options` (the resize writes the brush size).
    /// `real_time` reports `Update`s while a stroke is drawn.
    pub fn tool<'a>(&'a mut self, tool: SelectMaskTool, options: &'a mut SelectMaskToolOptions, real_time: bool) -> SelectMaskCanvasTool<'a> {
        SelectMaskCanvasTool { canvas: self, tool, options, real_time }
    }

    /// The user's Preferences › Cursors, which shape the brush outline; the defaults until set.
    pub fn cursor_options(&mut self, options: CursorOptions) {
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
            self.commit_polygon(&points, Modifiers::NONE);
        }
        true
    }

    /// Esc: drops an open polygon. Returns whether there was one.
    pub fn drop_polygon(&mut self) -> bool {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:881 `escape`.
        self.polygon.take().is_some()
    }

    /// A gesture, a polygon or a resize is in progress: the history waits.
    pub fn busy(&self) -> bool {
        self.gesture.is_some() || self.polygon.is_some() || self.resize.active()
    }

    /// Reports the closed polygon as one gesture: `Begin` and `Commit` together.
    fn commit_polygon(&mut self, points: &[Point], modifiers: Modifiers) {
        let mode = if self.polygon_add { EditMode::Add } else { EditMode::Subtract };
        let edit = SelectMaskEdit::Lasso { points: points.iter().map(|p| round_point([p.x, p.y])).collect(), mode };
        let id = GestureId::next();
        report(&mut self.events, false, id, Phase::Begin, Some(Built::Edit(edit.clone())), modifiers);
        report(&mut self.events, false, id, Phase::Commit, Some(Built::Edit(edit)), modifiers);
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
    Cancel,
}

/// The event of `phase` for `value`: `Update` and `Commit` need a value.
fn make<T>(id: GestureId, phase: Phase, value: Option<T>, modifiers: Modifiers) -> Option<EditEvent<T>> {
    match phase {
        Phase::Begin => Some(value.map_or_else(|| EditEvent::begin(id), |value| EditEvent::begin_with(id, value))),
        Phase::Update => value.map(|value| EditEvent::update(id, value, modifiers)),
        Phase::Commit => value.map(|value| EditEvent::commit(id, value, true, modifiers)),
        Phase::Cancel => Some(EditEvent::cancel(id, CancelReason::Escape)),
    }
}

/// Appends the event of `phase`. `refine` says which kind a value-less event belongs to.
fn report(events: &mut Vec<SelectMaskEvent>, refine: bool, id: GestureId, phase: Phase, built: Option<Built>, modifiers: Modifiers) {
    match built {
        Some(Built::Edit(edit)) => events.extend(make(id, phase, Some(edit), modifiers).map(SelectMaskEvent::Edit)),
        Some(Built::Refine(stroke)) => events.extend(make(id, phase, Some(stroke), modifiers).map(SelectMaskEvent::Refine)),
        None if refine => events.extend(make::<RefineStroke>(id, phase, None, modifiers).map(SelectMaskEvent::Refine)),
        None => events.extend(make::<SelectMaskEdit>(id, phase, None, modifiers).map(SelectMaskEvent::Edit)),
    }
}

/// What the gesture of `tool` over `points` makes, or `None` while it makes nothing (a Lasso with
/// fewer than 3 points). `zoom` turns an Object Selection drag into screen points.
fn build(tool: SelectMaskTool, options: &SelectMaskToolOptions, add: bool, points: &[Point], zoom: f64, click_reach: f64) -> Option<Built> {
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
            let target = if width.abs().max(height.abs()) * zoom < click_reach {
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
/// - **Polygonal Lasso**: an [`OutlineDraft`] of `DraftKind::Polygonal` with a close reach of 6
///   screen points: a click adds a vertex, a click on the first vertex or a double click closes;
///   Backspace and Delete remove the last vertex (`ToolContext::keep_keys` while it is open).
/// - **Object Selection**: a click, or a drag shorter than 4 screen points, picks the object under
///   the press (`ObjectTarget::Point`); a longer drag gives `ObjectTarget::Rect` from the press
///   to the release.
/// - **Brush resize**: Alt + right-drag or Control + Alt + drag ([`BrushResize`] with
///   `BrushLevel::Hardness` for the Brush; the size only for the others, `max_diameter` 5000).
/// - Paint: the stroke or outline while the gesture is open, the polygon with its rubber band, the
///   resize at its anchor; the brush outline is the pointer (`BrushCursor`).
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

    /// What the gesture over `points` makes now.
    fn built(&self, add: bool, points: &[Point], cx: &ToolContext<'_>) -> Option<Built> {
        build(self.tool, self.options, add, points, cx.view.zoom, f64::from(cx.tokens.component.select_mask.click_reach))
    }

    /// Writes the values of a resize to the options; a cancelled resize puts the start back.
    fn apply_resize(&mut self, event: Option<EditEvent<BrushDimensions>>) {
        let tool = self.tool;
        match event {
            Some(EditEvent::Update { value, .. } | EditEvent::Commit { value, .. }) => {
                self.options.sizes.set(tool, value.diameter);
                if tool == SelectMaskTool::Brush {
                    self.options.hardness = (value.level * 100.0).clamp(0.0, 100.0);
                }
            }
            Some(EditEvent::Cancel { .. }) => {
                if let Some((size, hardness)) = self.canvas.resize_start.take() {
                    self.options.sizes.set(tool, size);
                    self.options.hardness = hardness;
                }
            }
            _ => {}
        }
    }

    /// The resize's readout beside the pointer. Only the Brush has a hardness to show.
    fn resize_readout(&self, cx: &mut ToolContext<'_>) {
        if self.tool != SelectMaskTool::Brush {
            return;
        }
        let labels = BrushResizeLabels::of_ctx(&cx.canvas.ctx);
        if let Some(readout) = self.canvas.resize.readout(&Units::of_ctx(&cx.canvas.ctx), &labels) {
            cx.readout(readout);
        }
    }

    fn begin_resize(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        // pc: crates/ui-egui/src/tools/brush_resize.rs:71 `pointer` (Down).
        let options = BrushResizeOptions::new(&BrushStyle::of(cx.tokens), *BrushSizes::RANGE.end());
        let start = BrushDimensions { diameter: self.options.sizes.get(self.tool), level: self.level() };
        if self.canvas.resize.begin(cx.view, pointer.screen, start, &options).is_none() {
            return ToolAction::Ignored;
        }
        self.canvas.resize_start = Some((start.diameter, self.options.hardness));
        self.resize_readout(cx);
        ToolAction::Claimed
    }

    fn press_polygon(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:590-603 `begin` (PolygonalLasso), :1796-1802.
        let reach = DraftReach {
            close: f64::from(cx.tokens.component.select_mask.close_reach),
            same_point: SelectToolStyle::of(cx.tokens).reach().same_point,
        };
        let add = self.options.add;
        let canvas = &mut *self.canvas;
        let draft = canvas.polygon.get_or_insert_with(|| OutlineDraft::new(DraftKind::Polygonal, reach));
        draft.set_reach(reach);
        let sample = DraftPoint::new(pointer.doc, pointer.modifiers, cx.view.zoom);
        let event = if draft.is_open() && pointer.clicks >= 2 { draft.close() } else { draft.press(sample, true) };
        match event {
            Some(DraftEvent::Started { intent }) => canvas.polygon_add = add != intent.alt,
            Some(DraftEvent::Closed { points, .. }) => canvas.commit_polygon(&points, pointer.modifiers),
            _ => {}
        }
        canvas.forget_closed_polygon();
        ToolAction::Claimed
    }

    /// Adds the pointer to the gesture and reports the stroke so far when Real-Time Refinement is on.
    fn extend(&mut self, pointer: &ToolPointer, cx: &ToolContext<'_>) {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:620 `extend`.
        let Some((id, tool, add, points)) = self.canvas.gesture.as_mut() else { return };
        let at = pointer.doc;
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
            let (id, add, points) = (*id, *add, points.clone());
            let built = self.built(add, &points, cx);
            report(&mut self.canvas.events, self.tool == SelectMaskTool::RefineEdge, id, Phase::Update, built, pointer.modifiers);
        }
    }
}

impl CanvasTool for SelectMaskCanvasTool<'_> {
    fn each_pass(&mut self, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:1767 `canvas_input`, :484 `pick` (a
        // polygon survives no other tool).
        if self.tool != SelectMaskTool::PolygonalLasso {
            self.canvas.polygon = None;
        }
        if let Some(draft) = self.canvas.polygon.as_mut()
            && draft.takes_retract()
        {
            cx.keep_keys();
        }
    }

    fn cursor(&self, cx: &ToolContext<'_>) -> ToolCursor {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:1730 `paint_cursor`.
        if self.canvas.resize.active() {
            return ToolCursor::Hidden;
        }
        match self.tool {
            tool if tool.brushes() => {
                let size = self.options.sizes.get(tool);
                let tip = BrushTip::round(size, size / 2.0);
                BrushCursor::new(&tip).painting(self.canvas.gesture.is_some()).cursor(
                    &BrushStyle::of(cx.tokens),
                    &self.canvas.cursor_options,
                    cx.view,
                    cx.caps_lock,
                )
            }
            SelectMaskTool::Hand => ToolCursor::Hand { closed: false },
            SelectMaskTool::Zoom => ToolCursor::Zoom { out: cx.modifiers.alt },
            _ => ToolCursor::System(CursorIcon::Crosshair),
        }
    }

    fn takes(&self, button: PointerButton, modifiers: Modifiers) -> bool {
        // new: the brush resize chord (`BrushResize::chord`), as Liquify's: the primary button
        // always, the secondary while Alt is held on a brush.
        button == PointerButton::Primary || (self.tool.brushes() && button == PointerButton::Secondary && modifiers.alt)
    }

    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:599 `begin`.
        let tool = self.tool;
        if !tool.edits() {
            return ToolAction::Ignored;
        }
        if tool.brushes() && BrushResize::chord(pointer.button, pointer.modifiers) {
            return self.begin_resize(pointer, cx);
        }
        if tool == SelectMaskTool::PolygonalLasso {
            return self.press_polygon(pointer, cx);
        }
        let add = self.options.add != pointer.modifiers.alt;
        let id = GestureId::next();
        let points = vec![pointer.doc];
        let built = self.built(add, &points, cx);
        self.canvas.gesture = Some((id, tool, add, points));
        report(&mut self.canvas.events, tool == SelectMaskTool::RefineEdge, id, Phase::Begin, built, pointer.modifiers);
        ToolAction::Claimed
    }

    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:620 `extend`.
        if self.canvas.resize.active() {
            let event = self.canvas.resize.track(cx.view, pointer.screen, pointer.modifiers);
            self.apply_resize(event);
            self.resize_readout(cx);
            return;
        }
        if self.tool == SelectMaskTool::PolygonalLasso {
            if let Some(draft) = self.canvas.polygon.as_mut()
                && draft.drag(DraftPoint::new(pointer.doc, pointer.modifiers, cx.view.zoom)).is_some()
            {
                cx.request_repaint();
            }
            return;
        }
        self.extend(pointer, cx);
    }

    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:1367-1370 (Up), :554 `finish`, :579
        // `end_gesture`.
        if self.canvas.resize.active() {
            let event = self.canvas.resize.finish(cx.view, pointer.screen, pointer.modifiers);
            self.apply_resize(event);
            self.canvas.resize_start = None;
            return;
        }
        if self.tool == SelectMaskTool::PolygonalLasso {
            let canvas = &mut *self.canvas;
            if let Some(draft) = canvas.polygon.as_mut()
                && let Some(DraftEvent::Closed { points, .. }) = draft.release(DraftPoint::new(pointer.doc, pointer.modifiers, cx.view.zoom))
            {
                canvas.commit_polygon(&points, pointer.modifiers);
            }
            canvas.forget_closed_polygon();
            return;
        }
        self.extend(pointer, cx);
        let Some((id, tool, add, points)) = self.canvas.gesture.take() else { return };
        let built = self.built(add, &points, cx);
        let phase = if built.is_some() { Phase::Commit } else { Phase::Cancel };
        report(&mut self.canvas.events, tool == SelectMaskTool::RefineEdge, id, phase, built, pointer.modifiers);
    }

    fn hover(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:1363-1366 (Move: the hover point, the
        // polygon's rubber band).
        let Some(draft) = self.canvas.polygon.as_mut() else { return };
        let mut at = DraftPoint::new(pointer.doc, pointer.modifiers, cx.view.zoom);
        if pointer.modifiers.shift
            && !pointer.modifiers.alt
            && let Some(&last) = draft.points().last()
        {
            at.at = turn_to_45(last, at.at);
        }
        if draft.hover(at).is_some() {
            cx.request_repaint();
        }
    }

    fn cancel(&mut self, _cx: &mut ToolContext<'_>) {
        // new: Esc or a lost pointer drops the gesture without an edit.
        let event = self.canvas.resize.cancel();
        self.apply_resize(event);
        self.canvas.resize_start = None;
        if let Some((id, tool, _, _)) = self.canvas.gesture.take() {
            report(&mut self.canvas.events, tool == SelectMaskTool::RefineEdge, id, Phase::Cancel, None, Modifiers::NONE);
        }
        // A click that was cut short leaves the polygon open: Esc between clicks is the dialog's.
        if let Some(draft) = self.canvas.polygon.as_mut()
            && draft.is_down()
            && let Some(&last) = draft.points().last()
        {
            draft.release(DraftPoint::new(last, Modifiers::NONE, 1.0));
        }
    }

    fn key(&mut self, key: &ToolKey, _cx: &mut ToolContext<'_>) -> bool {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:1413-1419 (Backspace, Delete).
        if !key.pressed || !key.modifiers.is_none() || !matches!(key.key, Key::Backspace | Key::Delete) {
            return false;
        }
        let Some(draft) = self.canvas.polygon.as_mut() else { return false };
        if draft.retract().is_none() {
            return false;
        }
        self.canvas.forget_closed_polygon();
        true
    }

    fn paint(&self, painter: &Painter, cx: &ToolContext<'_>) {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:1658 `paint_canvas` (trail, polygon).
        let canvas = &*self.canvas;
        if let Some((_, tool, _, points)) = &canvas.gesture {
            match tool {
                SelectMaskTool::Quick | SelectMaskTool::Brush | SelectMaskTool::RefineEdge => {
                    let ink = cx.tokens.component.select_mask.trail.to_egui();
                    let radius = (self.options.sizes.get(*tool) / 2.0 * cx.view.zoom) as f32;
                    let line: Vec<_> = points.iter().map(|p| cx.view.to_screen(*p)).collect();
                    for &at in &line {
                        painter.circle_filled(at, radius, ink);
                    }
                    if line.len() > 1 {
                        painter.add(Shape::line(line, Stroke::new(2.0 * radius, ink)));
                    }
                }
                SelectMaskTool::Lasso if points.len() >= 2 => {
                    Marquee::polyline(points.iter().copied(), "").paint(painter, cx.view, MarqueeLook::Drawing, cx.time);
                }
                SelectMaskTool::Object => {
                    if let (Some(&a), Some(&b)) = (points.first(), points.last()) {
                        Marquee::rect(Rect::from_points(a, b), "").paint(painter, cx.view, MarqueeLook::Drawing, cx.time);
                    }
                }
                _ => {}
            }
        }
        if let Some(draft) = &canvas.polygon {
            if draft.points().len() >= 2 {
                Marquee::polyline(draft.points().iter().copied(), "").paint(painter, cx.view, MarqueeLook::Drawing, cx.time);
            }
            if let Some([a, b]) = draft.rubber_band() {
                Marquee::segments([(a, b)], "").paint(painter, cx.view, MarqueeLook::Drawing, cx.time);
            }
        }
        if let Some(now) = canvas.resize.dimensions() {
            let radius = now.diameter / 2.0;
            let falloff = if self.tool == SelectMaskTool::Brush { TipFalloff::from_hardness(radius, now.level) } else { TipFalloff::hard(radius) };
            canvas.resize.paint(painter, cx.view, &BrushStyle::of(cx.tokens), &falloff);
        }
    }
}
