//! [`MagneticLassoTool`]: an outline that snaps to the edges the app's [`EdgeTracer`] finds (task
//! SE-F10).

use craft_ui::canvas::draft::DraftReach;
use craft_ui::canvas::kurbo::{Point, Rect};
use craft_ui::canvas::{
    CanvasTool, Marquee, MarqueeLook, PointerInk, SelectionBadge, SelectionOperation, ToolAction, ToolContext, ToolCursor, ToolKey, ToolNode,
    ToolPointer, selection_cursor,
};
use craft_ui::edit::CancelReason;
use craft_ui::localized::Words;
use egui::{CursorIcon, Key, Modifiers, Painter, Stroke, StrokeKind, vec2};
use serde::{Deserialize, Serialize};

use super::style::SelectToolStyle;
use super::{OUTLINE_POINTS, SelectEvent, SelectLabels, SelectPhase, SelectToolPass, SelectionProbe, SelectionShape, labels_in};

/// The detection settings of one edge query.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EdgeSearch {
    /// How far from the pointer edges are looked for, in document pixels (Width, narrowed by
    /// stylus pressure when [`MagneticSettings::pressure_width`] is on).
    pub width: f64,
    /// The weakest edge that counts, 0..=1 (Contrast / 100).
    pub contrast: f64,
}

/// The app's edge finder over the pixels being traced (PhotoCraft's `EdgeSource` over
/// `photocraft_algo::magnetic`, the active layer's pixels or the composite). The tool calls it on
/// the UI thread; the app refreshes its snapshot when the document or the layer changes, between
/// outlines.
pub trait EdgeTracer {
    /// The edge point near `at` within `search.width`, or `None` for none (the tool uses `at`).
    fn snap(&mut self, at: Point, search: EdgeSearch) -> Option<Point>;

    /// The path along the edges from `from` to `to`, both included, following the pointer's
    /// recent `guide` points. Empty when it finds none: the tool draws a straight segment.
    fn trace(&mut self, from: Point, to: Point, guide: &[Point], search: EdgeSearch) -> Vec<Point>;
}

/// The Magnetic Lasso's options bar, persisted by the app (decisions §2.4). The options bar edits
/// [`MagneticLassoTool::settings_mut`]; `[` and `]` change `width` in the tool.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MagneticSettings {
    /// Width in document pixels, 1..=256. Default 10.
    pub width: f32,
    /// Contrast in percent, 1..=100. Default 10.
    pub contrast: f32,
    /// Frequency 0..=100: how often points fasten by themselves. Default 57.
    pub frequency: f32,
    /// Stylus Pressure: pressing harder narrows the width (a pen only). Default off.
    pub pressure_width: bool,
}

impl Default for MagneticSettings {
    // pc: crates/ui-egui/src/state/tool_options.rs:454-457 (10, 10, 57, false).
    fn default() -> Self {
        Self { width: 10.0, contrast: 10.0, frequency: 57.0, pressure_width: false }
    }
}

impl MagneticSettings {
    /// The range of `width`.
    pub const WIDTH_RANGE: (f32, f32) = (1.0, 256.0);

    /// How far the outline runs before a point fastens by itself, in document pixels at `zoom`
    /// screen points per pixel: `(8 + (100 − frequency) · 0.9) / zoom` (8 points at 100, 98 at 0,
    /// 47 at 57). A frequency that is not a number counts as 57; `zoom` at least 0.01.
    pub fn spacing(&self, zoom: f64) -> f64 {
        // pc: crates/ui-egui/src/tools/magnetic_lasso_ui.rs:71-77 `spacing`.
        let frequency = if self.frequency.is_finite() { f64::from(self.frequency.clamp(0.0, 100.0)) } else { 57.0 };
        (8.0 + (100.0 - frequency) * 0.9) / zoom.max(0.01)
    }

    /// The search for one query with pen `pressure` (`None` for a mouse): Width, narrowed to
    /// `1 + (width − 1) · (1 − pressure)` when [`pressure_width`](Self::pressure_width) is on and a
    /// pen sample carries a pressure; Contrast / 100.
    pub fn search(&self, pressure: Option<f32>) -> EdgeSearch {
        // pc: crates/ui-egui/src/tools/magnetic_lasso_ui.rs:60-69 `settings`.
        let mut width = f64::from(self.width);
        if self.pressure_width
            && let Some(pressure) = pressure.filter(|p| p.is_finite())
        {
            width = 1.0 + (width - 1.0) * f64::from(1.0 - pressure.clamp(0.0, 1.0));
        }
        EdgeSearch { width, contrast: f64::from(self.contrast) / 100.0 }
    }
}

/// The outline being drawn.
#[derive(Clone, Debug)]
struct MagneticOutline {
    /// The fastened outline, from the first fastening point to the last.
    path: Vec<Point>,
    /// Indices into `path` of the fastening points (the first is 0).
    anchors: Vec<usize>,
    /// The live segment from the last fastening point to the edge near the pointer.
    live: Vec<Point>,
    /// Where the pointer has been since the last fastening point.
    trail: Vec<Point>,
    /// The operation the first click latched.
    operation: SelectionOperation,
    /// The document the outline is drawn on.
    document: u64,
    /// An ⌥-drag is drawing a freehand segment.
    freehand: bool,
    /// The button is down (a drag follows the edges or draws freehand).
    down: bool,
    /// The pointer's last position (the trail ends at a fastened point instead after one fastens).
    pointer: Point,
}

impl MagneticOutline {
    /// The outline and the live segment after its start: what is drawn and counted.
    fn points(&self) -> impl Iterator<Item = Point> + '_ {
        self.path.iter().chain(self.live.iter().skip(1)).copied()
    }

    /// The pointer's position `q` joins the trail, keeping its points `gap` apart.
    fn extend_trail(&mut self, q: Point, width: f64) {
        // pc: magnetic_lasso_ui.rs:205-212 `extend_trail`.
        let gap = (width / 4.0).clamp(1.0, 8.0);
        match self.trail.len() {
            n if n >= 2 && self.trail[n - 2].distance(q) < gap => self.trail[n - 1] = q,
            _ => self.trail.push(q),
        }
    }

    /// The live segment fastens: its end becomes a fastening point.
    fn fasten_live(&mut self) {
        // pc: magnetic_lasso_ui.rs:262-279 `fasten_live`.
        let live = std::mem::take(&mut self.live);
        let before = self.path.len();
        for q in live.into_iter().skip(1) {
            if self.path.last().is_none_or(|l| l.distance(q) > 1e-9) {
                self.path.push(q);
            }
        }
        if self.path.len() > before {
            self.anchors.push(self.path.len() - 1);
        }
        if let Some(&end) = self.path.last() {
            self.trail = vec![end];
        }
    }

    /// An ⌥-drag ends: where it stopped is a fastening point.
    fn end_freehand(&mut self) {
        // pc: magnetic_lasso_ui.rs:286-292 `end_freehand`.
        if std::mem::take(&mut self.freehand) && self.anchors.last().is_some_and(|&a| a + 1 < self.path.len()) {
            self.anchors.push(self.path.len() - 1);
        }
    }

    /// `p` is within `radius` of the first point of an outline long enough to close.
    fn near_start(&self, p: Point, radius: f64) -> bool {
        // pc: magnetic_lasso_ui.rs:297-300 `near_start`.
        self.path.first().is_some_and(|&first| first.distance(p) <= radius) && length(&self.path) + length(&self.live) > 2.0 * radius
    }
}

/// The length of the polyline `p`.
fn length(p: &[Point]) -> f64 {
    p.windows(2).map(|w| w[0].distance(w[1])).sum()
}

/// The polyline `p` up to `len` along it (ending at that point) and the point, when more than a
/// pixel of it lies beyond.
fn cut(p: &[Point], len: f64) -> Option<(Vec<Point>, Point)> {
    // pc: magnetic_lasso_ui.rs:238-260 `cut`.
    if length(p) < len + 1.0 {
        return None;
    }
    let mut head = Vec::new();
    let mut acc = 0.0;
    for w in p.windows(2) {
        let (a, b) = (w[0], w[1]);
        head.push(a);
        let d = a.distance(b);
        if acc + d >= len && d > 0.0 {
            let point = a.lerp(b, (len - acc) / d);
            head.push(point);
            return Some((head, point));
        }
        acc += d;
    }
    None
}

/// The pen's pressure for `pointer`, `None` for a mouse (pressure 1, no tilt, no rotation).
///
/// A guess: the egui fork's pointer input carries no pointer kind (pen or mouse), so `ToolPointer`
/// has none either. A pen that reports full pressure and no tilt reads as a mouse; the width then
/// stays as set, which is also what a mouse gets.
fn pen_pressure(pointer: &ToolPointer) -> Option<f32> {
    let pen = pointer.pressure != 1.0 || pointer.tilt != [0.0; 2] || pointer.rotation != 0.0 || pointer.eraser;
    pen.then_some(pointer.pressure)
}

/// Photoshop's Magnetic Lasso: a selection outline that snaps to edges as the pointer moves.
///
/// Pointer (document units through the host; the tool takes button-up moves too):
/// - The first click sets the first fastening point at the edge near the pointer
///   ([`EdgeTracer::snap`]) and latches the operation.
/// - Moving the pointer (button up or down) traces the live segment from the last fastening point
///   to the edge near the pointer ([`EdgeTracer::trace`], guided by the pointer's trail); points
///   fasten by themselves every [`MagneticSettings::spacing`] along it. A jump is walked in steps
///   of `spacing / 2` (at most 256) so points keep fastening along it.
/// - A click fastens a point; ⌥-click adds a straight segment; ⌥-drag draws freehand. Pressing ⌥
///   during a drag anchors a point at the pointer and draws freehand from there; releasing it goes
///   back to the edges.
/// - A click within `outline_draft.close` / zoom of the first point (once the outline is longer
///   than twice that) closes along the edges (⌥ straight); so does a double-click, and Enter
///   (⌥ Enter straight).
///
/// Keys: Backspace or Delete remove the last fastening point (the first cancels); Esc cancels
/// (during a press too: the host's `cancel(Escape)` drops the outline, any other reason keeps it); `[`
/// and `]` change the width by 1 px within 1..=256. The tool keeps Backspace, Delete and Enter from
/// the app's shortcuts while an outline is open.
///
/// The closed outline is [`SelectEvent::Commit`] with a `SelectionShape::Polygon` (PhotoCraft's
/// `select.magneticLasso` with `trace: false`): the selection is exactly the outline drawn. An
/// outline of fewer than 3 points commits nothing. A document change or [`reset`](Self::reset)
/// drops it. No selection drag: a press inside the selection draws.
///
/// Drawn: the outline solid (`MarqueeLook::Drawing`), a white 5 pt square with a 1 pt black
/// outline at each fastening point, and over the first point the closing ring beside the pointer
/// (`SelectToolStyle`). The cursor is the crosshair; with Other Cursors › Precise
/// (`set_precise_cursor`) the width circle too.
#[derive(Clone, Debug)]
pub struct MagneticLassoTool<T> {
    tracer: T,
    settings: MagneticSettings,
    bar: SelectionOperation,
    outline: Option<MagneticOutline>,
    /// The modifiers of the last pass, to see ⌥ pressed or released during a drag.
    modifiers: Modifiers,
    /// Other Cursors › Precise: the width circle shows round the crosshair.
    precise: bool,
    labels: Option<SelectLabels>,
    events: Vec<SelectEvent>,
}

impl<T: EdgeTracer> MagneticLassoTool<T> {
    /// A Magnetic Lasso over `tracer`, with the default settings.
    pub fn new(tracer: T) -> Self {
        Self {
            tracer,
            settings: MagneticSettings::default(),
            bar: SelectionOperation::New,
            outline: None,
            modifiers: Modifiers::NONE,
            precise: false,
            labels: None,
            events: Vec::new(),
        }
    }

    /// The app's edge finder, to refresh its snapshot.
    pub fn tracer_mut(&mut self) -> &mut T {
        &mut self.tracer
    }

    /// The options bar's values.
    pub fn settings(&self) -> &MagneticSettings {
        &self.settings
    }

    /// The options bar's values, to edit (and to restore the persisted ones).
    pub fn settings_mut(&mut self) -> &mut MagneticSettings {
        &mut self.settings
    }

    /// The options bar's operation for outlines begun from now on.
    pub fn operation(&mut self, bar: SelectionOperation) -> &mut Self {
        self.bar = bar;
        self
    }

    /// Preferences › Cursors › Other Cursors is Precise (`CursorOptions::other_precise`): the
    /// width circle shows round the crosshair.
    pub fn precise_cursor(&mut self, precise: bool) -> &mut Self {
        self.precise = precise;
        self
    }

    /// The words of the outline's accessible name and value. Without them the tool reads the
    /// installed ones.
    pub fn labels(&mut self, labels: SelectLabels) -> &mut Self {
        self.labels = Some(labels);
        self
    }

    /// The tool as the `CanvasTool` for one pass, asking `probe` about the document.
    pub fn pass<'a>(&'a mut self, probe: &'a dyn SelectionProbe) -> SelectToolPass<'a, Self> {
        SelectToolPass::new(self, probe)
    }

    /// The events since the last call, in order.
    pub fn take_events(&mut self) -> Vec<SelectEvent> {
        std::mem::take(&mut self.events)
    }

    /// Drop the outline without an event (a tool switch).
    pub fn reset(&mut self) {
        self.outline = None;
    }

    /// What the tool is doing: an outline with its point count.
    pub fn phase(&self) -> SelectPhase {
        // new: `Outline { points: path + live }` while an outline is open.
        self.outline.as_ref().map_or(SelectPhase::Idle, |o| SelectPhase::Outline { points: o.points().count() })
    }

    /// An outline with the operation New is open: the app hides the selection's ants.
    pub fn replaces_selection(&self) -> bool {
        // pc: crates/ui-egui/src/tools/magnetic_lasso_ui.rs:363-366 `replaces_selection`.
        self.outline.as_ref().is_some_and(|o| o.operation == SelectionOperation::New)
    }

    /// Close the outline: the live segment fastens, then the last point joins the first along the
    /// edges, or with a straight segment with `straight`; `Commit` when 3 or more points.
    fn close(&mut self, straight: bool) {
        // pc: crates/ui-egui/src/tools/magnetic_lasso_ui.rs:301-332 `close`, `commit`.
        let Some(mut o) = self.outline.take() else { return };
        o.end_freehand();
        o.fasten_live();
        if !straight && let (Some(&first), Some(&last)) = (o.path.first(), o.path.last()) {
            let mut closing = self.tracer.trace(last, first, &[], self.settings.search(None));
            // The outline ends where it starts.
            closing.pop();
            o.path.extend(closing.into_iter().skip(1));
        }
        if o.path.len() >= 3 {
            self.events.push(SelectEvent::Commit { shape: SelectionShape::Polygon(o.path), operation: o.operation });
        }
    }

    /// Remove the last fastening point and the outline back to the one before; with only the first
    /// left, cancel. The live segment is traced to the pointer again without fastening.
    fn remove_last_point(&mut self) {
        // pc: crates/ui-egui/src/tools/magnetic_lasso_ui.rs:340-361 `remove_last_point`.
        let Some(o) = self.outline.as_mut() else { return };
        o.end_freehand();
        if o.anchors.len() <= 1 {
            self.outline = None;
            return;
        }
        o.anchors.pop();
        let keep = o.anchors.last().map_or(1, |&a| a + 1);
        o.path.truncate(keep);
        o.live.clear();
        let pointer = o.trail.last().copied();
        o.trail = o.path.last().copied().into_iter().collect();
        // The live segment runs from the earlier point to the pointer again; it fastens only when
        // the pointer moves on, so Backspace can keep removing points.
        if let Some(p) = pointer {
            self.follow(p, self.settings.search(None), 0.0, false);
        }
    }

    /// The pointer is at `q`: trace the live segment to the edge near it and, with `fasten`, fasten
    /// points behind the pointer while the live segment is longer than the spacing (at most 64 per
    /// call).
    fn follow(&mut self, q: Point, search: EdgeSearch, spacing: f64, fasten: bool) {
        // pc: crates/ui-egui/src/tools/magnetic_lasso_ui.rs:205-284 `extend_trail` (gap `width / 4`
        // within 1..=8), `follow`, `cut`, `fasten_live`.
        let Self { tracer, outline, .. } = self;
        let Some(o) = outline.as_mut() else { return };
        o.extend_trail(q, search.width);
        let target = tracer.snap(q, search).unwrap_or(q);
        for _ in 0..64 {
            let Some(&from) = o.path.last() else { return };
            let mut live = tracer.trace(from, target, &o.trail, search);
            if live.is_empty() {
                live = vec![from, target];
            }
            // The part of the outline `spacing` behind its start no longer changes as the pointer
            // moves: it fastens.
            let Some((head, point)) = fasten.then(|| cut(&live, spacing)).flatten() else {
                o.live = live;
                return;
            };
            o.path.extend(head.into_iter().skip(1));
            o.anchors.push(o.path.len() - 1);
            // Edges are looked for along the pointer's path from where it passed the new point.
            let near = o.trail.iter().enumerate().min_by(|a, b| a.1.distance(point).total_cmp(&b.1.distance(point))).map_or(0, |(i, _)| i);
            let rest: Vec<Point> = o.trail.iter().skip(near + 1).copied().collect();
            o.trail = std::iter::once(point).chain(rest).collect();
        }
    }

    /// The pointer moved to `p` (button up or down): freehand adds points, otherwise the outline
    /// follows the edges, walking a long jump in steps.
    fn moved(&mut self, pointer: &ToolPointer, zoom: f64) {
        // pc: crates/ui-egui/src/tools/magnetic_lasso_ui.rs:164-193 `moved`.
        let p = pointer.doc;
        let Some(o) = self.outline.as_mut() else { return };
        o.pointer = p;
        if o.freehand {
            // ⌥-drag: the pointer's path is the outline.
            if o.path.last().is_none_or(|q| q.distance(p) >= 0.5) {
                o.path.push(p);
            }
            o.trail = vec![p];
            return;
        }
        let search = self.settings.search(pen_pressure(pointer));
        let spacing = self.settings.spacing(zoom);
        let step = (spacing / 2.0).max(1.0);
        let last = o.trail.last().copied().unwrap_or(p);
        // A long jump (a fast flick, automation) is walked in steps, so points keep fastening
        // along it.
        let n = ((last.distance(p) / step).ceil() as usize).clamp(1, 256);
        for k in 1..=n {
            self.follow(last.lerp(p, k as f64 / n as f64), search, spacing, true);
        }
    }

    /// The button went up or the gesture was cancelled: an ⌥-drag ends.
    fn end_gesture(&mut self) {
        if let Some(o) = self.outline.as_mut() {
            o.down = false;
            o.end_freehand();
        }
    }
}

impl<T: EdgeTracer> CanvasTool for SelectToolPass<'_, MagneticLassoTool<T>> {
    /// Drops an outline begun on another document; keeps the keys while an outline is open.
    fn each_pass(&mut self, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/magnetic_lasso_ui.rs:368-374 `frame`.
        let (tool, probe) = (&mut *self.tool, self.probe);
        if tool.outline.as_ref().is_some_and(|o| o.document != probe.document()) {
            tool.outline = None;
        }
        let before = std::mem::replace(&mut tool.modifiers, cx.modifiers);
        if let Some(o) = tool.outline.as_mut()
            && o.down
            && before.alt != cx.modifiers.alt
        {
            if cx.modifiers.alt {
                // ⌥ pressed during the drag: the pointer's last place anchors, freehand
                // goes on from it (Photoshop).
                let p = o.pointer;
                o.fasten_live();
                if o.path.last().is_none_or(|q| q.distance(p) > 1e-9) {
                    o.path.push(p);
                    o.anchors.push(o.path.len() - 1);
                }
                o.trail = vec![p];
                o.freehand = true;
            } else {
                // ⌥ released: the edges again, from where the freehand stopped.
                o.end_freehand();
                o.live.clear();
                o.trail = o.path.last().copied().into_iter().collect();
            }
        }
        if tool.outline.is_some() {
            cx.keep_keys();
        }
    }

    /// The system crosshair; with `set_precise_cursor`
    /// `paint` adds the width circle (`width · zoom`, at least `select.width_ring_min`) round it, a 1 pt dark ring
    /// outside a 1 pt light one (`SelectToolStyle::width_ring_dark`, `width_ring_light`).
    fn cursor(&self, cx: &ToolContext<'_>) -> ToolCursor {
        // pc: crates/ui-egui/src/tools/magnetic_lasso_ui.rs:440-449 `cursor`.
        let tool = &*self.tool;
        if let Some(pointer) = cx.pointer {
            let badge = match &tool.outline {
                Some(o) => o.operation.badge(),
                None if cx.modifiers.command => SelectionBadge::New,
                None => SelectionOperation::resolve(tool.bar, cx.modifiers, self.probe.has_selection()).badge(),
            };
            selection_cursor(&cx.canvas.ctx, pointer.screen, badge, PointerInk::System);
        }
        ToolCursor::System(CursorIcon::Crosshair)
    }

    /// The first click starts an outline; later ones close near the first point, add a straight
    /// segment with ⌥ (and freehand while dragging), or fasten a point; a double-click closes.
    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        // pc: crates/ui-egui/src/tools/magnetic_lasso_ui.rs:120-162 `down`.
        let (tool, probe) = (&mut *self.tool, self.probe);
        let search = tool.settings.search(pen_pressure(pointer));
        let p = pointer.doc;
        let Some(o) = tool.outline.as_mut() else {
            // ⌘ makes it the Move tool (the app's rule).
            if pointer.modifiers.command {
                return ToolAction::Ignored;
            }
            let start = tool.tracer.snap(p, search).unwrap_or(p);
            tool.outline = Some(MagneticOutline {
                path: vec![start],
                anchors: vec![0],
                live: Vec::new(),
                trail: vec![p],
                operation: SelectionOperation::resolve(tool.bar, pointer.modifiers, probe.has_selection()),
                document: probe.document(),
                freehand: false,
                down: true,
                pointer: p,
            });
            return ToolAction::Claimed;
        };
        let radius = DraftReach::of(cx.tokens).close / cx.view.zoom.max(0.01);
        if pointer.clicks >= 2 || o.near_start(p, radius) {
            tool.close(pointer.modifiers.alt);
            return ToolAction::Claimed;
        }
        o.down = true;
        o.pointer = p;
        if pointer.modifiers.alt {
            // A straight segment to here; dragging on draws freehand.
            if o.path.last().is_none_or(|q| q.distance(p) > 1e-9) {
                o.path.push(p);
                o.anchors.push(o.path.len() - 1);
            }
            o.live.clear();
            o.trail = vec![p];
            o.freehand = true;
        } else {
            // A click fastens a point at the edge near the pointer.
            tool.follow(p, search, 0.0, false);
            if let Some(o) = tool.outline.as_mut() {
                o.fasten_live();
            }
        }
        ToolAction::Claimed
    }

    /// An ⌥-drag adds freehand points (0.5 px apart); otherwise the outline follows the edges.
    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/magnetic_lasso_ui.rs:164-193 `moved`.
        self.tool.moved(pointer, cx.view.zoom);
    }

    /// Ends an ⌥-drag: where it stopped fastens.
    fn release(&mut self, _pointer: &ToolPointer, _cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/magnetic_lasso_ui.rs:286-292 `end_freehand`.
        self.tool.end_gesture();
    }

    /// Button-up moves follow the edges too.
    fn hover(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/magnetic_lasso_ui.rs:164-193 `moved`.
        if self.tool.outline.is_some() {
            self.tool.moved(pointer, cx.view.zoom);
            cx.request_repaint();
        }
    }

    /// Esc during a press drops the whole outline (Photoshop), as Esc between clicks does. A lost
    /// pointer or a `ToolHost::cancel` for another reason ends an ⌥-drag as `release` would and
    /// keeps the outline (the app calls [`reset`](MagneticLassoTool::reset) to drop it).
    fn cancel(&mut self, reason: CancelReason, _cx: &mut ToolContext<'_>) {
        // new: ends an ⌥-drag as `release` would; the outline stays unless Esc.
        self.tool.end_gesture();
        if reason == CancelReason::Escape {
            self.tool.outline = None;
        }
    }

    /// Enter (⌥ Enter), Esc, Backspace and Delete while an outline is open; `[` and `]` always.
    fn key(&mut self, key: &ToolKey, _cx: &mut ToolContext<'_>) -> bool {
        // pc: crates/ui-egui/src/tools/magnetic_lasso_ui.rs:376-412 `keys`.
        let tool = &mut *self.tool;
        if !key.pressed {
            return false;
        }
        let none = key.modifiers.is_none();
        if tool.outline.is_some() {
            match key.key {
                Key::Enter if none || key.modifiers.matches_exact(Modifiers::ALT) => {
                    tool.close(key.modifiers.alt);
                    return true;
                }
                Key::Escape if none => {
                    tool.outline = None;
                    return true;
                }
                Key::Backspace | Key::Delete if none => {
                    tool.remove_last_point();
                    return true;
                }
                _ => {}
            }
        }
        let step = match key.key {
            Key::OpenBracket if none => -1.0,
            Key::CloseBracket if none => 1.0,
            _ => return false,
        };
        let (min, max) = MagneticSettings::WIDTH_RANGE;
        let width = &mut tool.settings.width;
        *width = if width.is_finite() { (width.round() + step).clamp(min, max) } else { 10.0 };
        true
    }

    /// The outline, the fastening points and the closing ring.
    fn paint(&self, painter: &Painter, cx: &ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/magnetic_lasso_ui.rs:414-438 `draw`.
        let tool = &*self.tool;
        let style = SelectToolStyle::of(cx.tokens);
        let pointer = cx.pointer.map(|p| p.screen);
        if let Some(o) = &tool.outline {
            let points: Vec<Point> = o.points().collect();
            if points.len() >= 2 {
                Marquee::polyline(points, "").paint(painter, cx.view, MarqueeLook::Drawing, cx.time);
            }
            for p in o.anchors.iter().filter_map(|&i| o.path.get(i)).map(|&p| cx.view.to_screen(p)) {
                let r = egui::Rect::from_center_size(p, vec2(style.fastening, style.fastening));
                painter.rect_filled(r, 0.0, style.fastening_fill);
                painter.rect_stroke(r, 0.0, Stroke::new(1.0, style.fastening_stroke), StrokeKind::Outside);
            }
            if let Some(h) = pointer
                && o.near_start(cx.view.to_doc(h), DraftReach::of(cx.tokens).close / cx.view.zoom.max(0.01))
            {
                // Photoshop's closing cursor: a small circle beside the pointer.
                let c = h + vec2(style.ring_offset, style.ring_offset);
                painter.circle_stroke(c, style.ring_radius, Stroke::new(style.ring_halo, style.ring_halo_color));
                painter.circle_stroke(c, style.ring_radius, Stroke::new(style.ring_line, style.ring_line_color));
            }
        }
        if tool.precise
            && let Some(h) = pointer
        {
            let (min, max) = MagneticSettings::WIDTH_RANGE;
            let r = (tool.settings.width.clamp(min, max) * cx.view.zoom as f32).max(style.width_ring_min);
            painter.circle_stroke(h, r + style.width_ring_gap, Stroke::new(1.0, style.width_ring_dark));
            painter.circle_stroke(h, r, Stroke::new(1.0, style.width_ring_light));
        }
    }

    /// The outline being drawn as one AccessKit object: named `SelectLabels::outline`, valued
    /// `OUTLINE_POINTS` of its points ("5 points"), over the points' bounds; `None` while no
    /// outline is open.
    fn node(&self, cx: &ToolContext<'_>) -> Option<ToolNode> {
        // new: SE deviation 3 (no `Ui` on the host); `ToolNode::new(..).value(..).bounds(..)`.
        let o = self.tool.outline.as_ref()?;
        let first = *o.path.first()?;
        let (bounds, count) = o.points().fold((Rect::from_points(first, first), 0_u64), |(rect, n), p| (rect.union_pt(p), n + 1));
        let (ctx, words) = (&cx.canvas.ctx, Words::of_ctx(&cx.canvas.ctx));
        let value = words.counted(ctx, OUTLINE_POINTS, &words.get(OUTLINE_POINTS.word), count);
        Some(ToolNode::new(labels_in(self.tool.labels.as_ref(), ctx).outline).value(value).bounds(bounds))
    }
}
