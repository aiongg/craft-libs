//! [`RulerTool`]: the Ruler tool on the canvas tool host, with its overlay and keyboard parts.

use std::f64::consts::FRAC_PI_4;

use egui::{Modifiers, Painter, Pos2, Stroke, vec2};
use kurbo::Point;

use super::model::{MIN_LINE, RulerLine, RulerPart};
use super::style::MeasureStyle;
use craft_ui::behavior::move_gesture::snap_direction;
use craft_ui::behavior::nudge::NudgeKeys;
use craft_ui::canvas::straight_line::StraightLinePreview;
use craft_ui::canvas::{CanvasTool, CursorHost, Part, RulerUnits, ToolAction, ToolContext, ToolCursor, ToolKey, ToolParts, ToolPointer};
use craft_ui::edit::{CancelReason, EditEvent, GestureId};
use craft_ui::localized::Labels as _;

craft_ui::words! {
    /// The Ruler tool's AccessKit names, English by default.
    #[non_exhaustive]
    pub struct RulerToolLabels {
        #[allow(missing_docs)]
        group: "rulerTool.group" = "Ruler";
        #[allow(missing_docs)]
        start: "rulerTool.start" = "Start point";
        #[allow(missing_docs)]
        end: "rulerTool.end" = "End point";
        #[allow(missing_docs)]
        protractor: "rulerTool.protractor" = "Protractor point";
        /// The group's value; `{length}` and `{angle}` are formatted numbers.
        value: "rulerTool.value" = "Length {length}, angle {angle}°";
    }
}

/// One pass's input: the line as the app stores it, and how its values read.
pub struct RulerInput<'a> {
    line: Option<RulerLine>,
    units: Option<(&'a RulerUnits, &'a RulerUnits)>,
}

impl<'a> RulerInput<'a> {
    /// The pass over `line`, the document's Ruler line (`None`: no line yet).
    pub fn new(line: Option<RulerLine>) -> Self {
        Self { line, units: None }
    }

    /// The ruler units of the parts' values ("x, y") and of the group's length; default: document
    /// pixels.
    pub fn units(mut self, x: &'a RulerUnits, y: &'a RulerUnits) -> Self {
        self.units = Some((x, y));
        self
    }
}

/// The Ruler tool, kept by the app while the tool is picked.
///
/// - Pointer: a press within `measure.hit` screen points of an end grabs it (the protractor's end
///   first, then the end, then the start); ⌥-press at an end starts the protractor arm at that
///   vertex (when the far end was hit, start and end swap first); elsewhere a new line starts.
///   ⇧ moves the point onto the nearest 45° ray about the fixed end (about the vertex for the
///   protractor arm), keeping its distance from that end as PhotoCraft does (Gradient projects
///   instead, [`snap_ray`](craft_ui::behavior::move_gesture::snap_ray)). A new line shorter than [`MIN_LINE`](super::MIN_LINE)
///   document units is cleared on release. The reach is in screen points at every zoom.
/// - Keyboard: the line is one Tab stop through the host with the parts Start point, End point
///   and, with a protractor, Protractor point (values "x, y" in the ruler unit; the group's value
///   "Length L, angle A°"); Page Up and Down pick a part, the arrows move it 1 unit and ⇧ 10
///   ([`NudgeKeys::from_key`](craft_ui::behavior::nudge::NudgeKeys::from_key)); Delete or
///   Backspace clears the line. The pick moves only while the parts group has the keyboard
///   ([`ToolContext::parts_focused`]); with the canvas focused the arrows move the whole line.
/// - Events: the line's changes, in order, from [`take_events`](Self::take_events) after
///   `ToolHost::show`: a drag (Begin, Update, Commit; Cancel on Esc, which puts the line back),
///   an arrow press or Delete (`Set`; `None` clears). The app stores the value; it is not an
///   undo step.
/// - Look: each arm as the line guide (halo, then line) and a cross at each end
///   (`measure.handle_arm`), the picked part ringed while the keyboard has it.
/// - Cursor: the crosshair.
#[derive(Clone, Debug, Default)]
pub struct RulerTool {
    /// The part the drag moves, and whether it started a new line.
    drag: Option<(RulerPart, bool)>,
    gesture: Option<GestureId>,
    /// The line at the press, for Esc.
    before: Option<RulerLine>,
    picked: Option<RulerPart>,
    labels: Option<RulerToolLabels>,
    events: Vec<EditEvent<Option<RulerLine>>>,
}

impl RulerTool {
    /// A tool with no line.
    pub fn new() -> Self {
        Self::default()
    }

    /// The AccessKit names; default: [`RulerToolLabels`] of the installed words.
    pub fn labels(&mut self, labels: RulerToolLabels) -> &mut Self {
        self.labels = Some(labels);
        self
    }

    /// The line's changes since the last call, in order.
    pub fn take_events(&mut self) -> Vec<EditEvent<Option<RulerLine>>> {
        std::mem::take(&mut self.events)
    }

    /// This pass's tool for the host.
    pub fn pass<'a>(&'a mut self, input: RulerInput<'a>) -> RulerPass<'a> {
        let line = input.line.filter(RulerLine::is_finite);
        RulerPass { tool: self, input, line }
    }
}

/// The Ruler tool for one pass of the host ([`RulerTool::pass`]).
pub struct RulerPass<'a> {
    tool: &'a mut RulerTool,
    input: RulerInput<'a>,
    /// The line as this pass's events leave it: the input's, then every change the tool reported.
    line: Option<RulerLine>,
}

impl RulerPass<'_> {
    /// `line` becomes the line and `event` goes out.
    fn report(&mut self, line: Option<RulerLine>, event: EditEvent<Option<RulerLine>>) {
        self.line = line;
        self.tool.events.push(event);
    }

    /// `to` on the nearest 45° ray from `fixed`, at its distance from `fixed` (`to` itself when
    /// it is on `fixed` or not finite).
    fn on_ray(fixed: Point, to: Point) -> Point {
        // pc: crates/ui-egui/src/tools/analysis_ui.rs:219 `constrain`.
        match snap_direction(fixed, to, FRAC_PI_4) {
            Some(dir) => fixed + dir * (to - fixed).hypot(),
            None => to,
        }
    }

    /// The line with the grabbed `part` moved to `to`, ⇧ on 45° rays about the fixed point: the end
    /// for the start, the start for the end and the protractor.
    fn moved(line: RulerLine, part: RulerPart, to: Point, shift: bool) -> RulerLine {
        let constrain = |fixed: Point| if shift { Self::on_ray(fixed, to) } else { to };
        let mut line = line;
        match part {
            RulerPart::Start => line.start = constrain(line.end),
            RulerPart::End => line.end = constrain(line.start),
            RulerPart::Protractor => line.protractor = Some(constrain(line.start)),
        }
        line
    }

    /// The units of the parts' values and the group's length.
    fn units(&self, cx: &ToolContext<'_>) -> (RulerUnits, RulerUnits) {
        match self.input.units {
            Some((x, y)) => (x.clone(), y.clone()),
            None => (RulerUnits::pixels(cx.numbers.clone()), RulerUnits::pixels(cx.numbers.clone())),
        }
    }

    /// Moves the picked part (while the parts group has the keyboard), or the whole line, by a
    /// key's nudge.
    fn nudge(&mut self, key: &ToolKey, line: RulerLine, parts_focused: bool) -> bool {
        let Some(nudge) = NudgeKeys::new().from_key(key) else { return false };
        let mut next = line;
        match self.tool.picked.filter(|&part| parts_focused && line.point(part).is_some()) {
            Some(RulerPart::Start) => next.start += nudge.by,
            Some(RulerPart::End) => next.end += nudge.by,
            Some(RulerPart::Protractor) => next.protractor = line.protractor.map(|q| q + nudge.by),
            None => {
                next.start += nudge.by;
                next.end += nudge.by;
                next.protractor = line.protractor.map(|q| q + nudge.by);
            }
        }
        self.report(Some(next), EditEvent::set(Some(next), key.modifiers));
        true
    }
}

impl CanvasTool for RulerPass<'_> {
    fn cursor(&self, cx: &ToolContext<'_>) -> ToolCursor {
        // new: the precise crosshair (`ToolCursor::Crosshair` with the context's cursor style), as
        // the other measuring tools of the host.
        let (length, gap) = CursorHost::style(cx.tokens).crosshair;
        ToolCursor::Crosshair { length, gap }
    }

    /// Grabs an end, starts the protractor arm, or starts a new line (Begin).
    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        // pc: crates/ui-egui/src/tools/analysis_ui.rs:250 (Down), :210 `tolerance` (7 / zoom).
        let reach = cx.view.doc_len(MeasureStyle::of(cx.tokens).hit);
        let at = pointer.doc;
        let near = |q: Point| q.distance(at) <= reach;
        let before = self.line;
        let (line, part, new, picked) = match before {
            // ⌥ at an end starts the protractor arm at that vertex.
            Some(mut line) if pointer.modifiers.alt && (near(line.start) || near(line.end)) => {
                if near(line.end) && !near(line.start) {
                    std::mem::swap(&mut line.start, &mut line.end);
                }
                line.protractor = Some(at);
                (line, RulerPart::Protractor, false, Some(RulerPart::Protractor))
            }
            Some(line) => match line.part_at(at, reach) {
                Some(part) => (line, part, false, Some(part)),
                None => (RulerLine::new(at, at), RulerPart::End, true, None),
            },
            None => (RulerLine::new(at, at), RulerPart::End, true, None),
        };
        let id = GestureId::next();
        self.tool.drag = Some((part, new));
        self.tool.gesture = Some(id);
        self.tool.before = before;
        self.tool.picked = picked;
        let event = if Some(line) == before { EditEvent::begin(id) } else { EditEvent::begin_with(id, Some(line)) };
        self.report(Some(line), event);
        ToolAction::Claimed
    }

    /// Moves the grabbed point, ⇧ onto 45° rays about the fixed end (Update).
    fn drag(&mut self, pointer: &ToolPointer, _cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/analysis_ui.rs:273 (Move), :219 `constrain`.
        let (Some((part, _)), Some(id), Some(line)) = (self.tool.drag, self.tool.gesture, self.line) else { return };
        if !pointer.doc.is_finite() {
            return;
        }
        let next = Self::moved(line, part, pointer.doc, pointer.modifiers.shift);
        if next != line {
            self.report(Some(next), EditEvent::update(id, Some(next), pointer.modifiers));
        }
    }

    /// Commit; a new line under [`MIN_LINE`](super::MIN_LINE) commits `None`.
    fn release(&mut self, pointer: &ToolPointer, _cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/analysis_ui.rs:283 (Up).
        let (Some((part, new)), Some(id)) = (self.tool.drag.take(), self.tool.gesture.take()) else { return };
        let before = self.tool.before.take();
        let Some(mut line) = self.line else { return };
        if pointer.doc.is_finite() {
            line = Self::moved(line, part, pointer.doc, pointer.modifiers.shift);
        }
        let value = (!(new && line.start.distance(line.end) < MIN_LINE)).then_some(line);
        self.report(value, EditEvent::commit(id, value, value != before, pointer.modifiers));
    }

    /// Esc: Cancel; the app puts the line of the press back.
    fn cancel(&mut self, reason: CancelReason, _cx: &mut ToolContext<'_>) {
        // new: the edit lifecycle (decisions §2.5).
        self.tool.drag = None;
        let before = self.tool.before.take();
        if let Some(id) = self.tool.gesture.take() {
            self.report(before, EditEvent::cancel(id, reason));
        }
    }

    /// Arrows move the picked part while the parts group has the keyboard, else the whole line; Delete and Backspace clear.
    fn key(&mut self, key: &ToolKey, cx: &mut ToolContext<'_>) -> bool {
        // new: lead call 25 (PhotoCraft has no keyboard path).
        let Some(line) = self.line else { return false };
        if !key.pressed || self.tool.drag.is_some() {
            return false;
        }
        if matches!(key.key, egui::Key::Delete | egui::Key::Backspace) && key.modifiers.matches_exact(Modifiers::NONE) {
            self.report(None, EditEvent::set(None, key.modifiers));
            self.tool.picked = None;
            return true;
        }
        self.nudge(key, line, cx.parts_focused)
    }

    /// The arms and the end crosses in `MeasureStyle`.
    fn paint(&self, painter: &Painter, cx: &ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/analysis_ui.rs:416 `draw_overlay` (the Ruler half), :408
        // `handle`.
        let Some(line) = self.line else { return };
        let style = MeasureStyle::of(cx.tokens);
        let cross = |at: Point| {
            let c = cx.view.to_screen(at);
            for (width, color) in [(style.line.halo, style.line.halo_color), (style.line.width, style.line.line)] {
                let arm = style.handle_arm;
                painter.line_segment([c - vec2(arm, 0.0), c + vec2(arm, 0.0)], Stroke::new(width, color));
                painter.line_segment([c - vec2(0.0, arm), c + vec2(0.0, arm)], Stroke::new(width, color));
            }
        };
        StraightLinePreview::new(line.start, line.end).paint(painter, cx.view, &style.line);
        cross(line.end);
        if let Some(q) = line.protractor {
            StraightLinePreview::new(line.start, q).paint(painter, cx.view, &style.line);
            cross(q);
        }
        cross(line.start);
    }

    /// The line's parts; `None` without a line.
    fn parts(&self, cx: &ToolContext<'_>) -> Option<ToolParts> {
        // new: lead call 33; values through the units, the group's value from `RulerLine::reading`.
        let line = self.line?;
        let labels = self.tool.labels.clone().unwrap_or_else(|| RulerToolLabels::of_ctx(&cx.canvas.ctx));
        let (x, y) = self.units(cx);
        let size = 2.0 * MeasureStyle::of(cx.tokens).hit;
        let parts = [(RulerPart::Start, &labels.start), (RulerPart::End, &labels.end), (RulerPart::Protractor, &labels.protractor)]
            .into_iter()
            .filter_map(|(part, name)| {
                let at = line.point(part)?;
                let centre: Pos2 = cx.view.to_screen(at);
                Some(Part {
                    key: part.key(),
                    name: name.clone(),
                    value: Some(format!("{}, {}", x.format_value(at.x, 0.0), y.format_value(at.y, 0.0))),
                    rect: egui::Rect::from_center_size(centre, vec2(size, size)),
                    selected: self.tool.picked == Some(part),
                })
            })
            .collect();
        let reading = line.reading();
        let value =
            craft_i18n::fmt(&labels.value, &[("length", &x.format_value(reading.l1, 0.0)), ("angle", &cx.numbers.format(reading.angle, 0..=1))]);
        Some(ToolParts::new(labels.group.clone(), parts).value(value))
    }

    fn picked(&self) -> Option<u64> {
        self.tool.picked.map(RulerPart::key)
    }

    fn pick(&mut self, key: Option<u64>, _cx: &mut ToolContext<'_>) {
        self.tool.picked = key.and_then(RulerPart::of_key);
    }
}
