//! Content-Aware Fill's tools and the canvas tool (task WS-CA1).

use egui::{CursorIcon, Key, Modifiers, Painter, PointerButton};

use craft_ui::canvas::brush::{BrushCursor, BrushDimensions, BrushResize, BrushResizeLabels, BrushResizeOptions, BrushStyle, BrushTip, TipFalloff};
use craft_ui::canvas::draft::{DraftEvent, DraftKind, DraftPoint, OutlineDraft};
use craft_ui::canvas::kurbo::Point;
use craft_ui::canvas::{
    CanvasTool, CursorOptions, Marquee, MarqueeLook, Readout, SelectionOperation, ToolAction, ToolContext, ToolCursor, ToolKey, ToolPointer,
};
use craft_ui::edit::{CancelReason, EditEvent, GestureId};
use craft_ui::icons::Icon;
use craft_ui::localized::{Labels as _, Unit};
use craft_ui::patterns::{RailTool, ToolSlot};

use super::FillEvent;
use super::FillLabels;
use super::lasso::FillAreaEdit;
use super::session::{FillLimits, FillSession, SamplingStroke};
use crate::ui::select::SelectToolStyle;
use crate::ui::select_mask::EditMode;

/// The tools of Content-Aware Fill's strip: Sampling Brush (B), Lasso (L), Hand (H), Zoom (Z).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FillTool {
    #[default]
    SamplingBrush,
    Lasso,
    Hand,
    Zoom,
}

impl FillTool {
    pub const ALL: [FillTool; 4] = [FillTool::SamplingBrush, FillTool::Lasso, FillTool::Hand, FillTool::Zoom];

    /// B, L, H, Z.
    pub fn shortcut(self) -> char {
        // pc: crates/ui-egui/src/dialogs/caf_ui/controls.rs:189 `tool_tip`, :173-177 (the letters in `keys`).
        match self {
            FillTool::SamplingBrush => 'B',
            FillTool::Lasso => 'L',
            FillTool::Hand => 'H',
            FillTool::Zoom => 'Z',
        }
    }

    /// The strip icon, from the library's set (PhotoCraft's `tool_icon` names, by meaning).
    fn icon(self) -> Icon {
        // pc: crates/ui-egui/src/dialogs/caf_ui/controls.rs:180 `tool_icon`.
        match self {
            FillTool::SamplingBrush => Icon::Brush,
            FillTool::Lasso => Icon::Lasso,
            FillTool::Hand => Icon::Hand,
            FillTool::Zoom => Icon::ZoomIn,
        }
    }

    /// One slot per tool, `group_end` after the Lasso; labels from `labels`, letters as shortcuts.
    pub fn slots(labels: &FillLabels) -> Vec<ToolSlot<FillTool>> {
        // pc: crates/ui-egui/src/dialogs/caf_ui/controls.rs:180 `tool_icon`, :189 `tool_tip`.
        (0_u32..)
            .zip(Self::ALL)
            .map(|(id, tool)| {
                let label = match tool {
                    FillTool::SamplingBrush => &labels.sampling_brush,
                    FillTool::Lasso => &labels.lasso,
                    FillTool::Hand => &labels.hand,
                    FillTool::Zoom => &labels.zoom,
                };
                let entry = RailTool::new(tool, tool.icon(), label.as_str()).shortcut(tool.shortcut());
                ToolSlot::new(id, vec![entry]).group_end(tool == FillTool::Lasso)
            })
            .collect()
    }
}

/// The tools' options: the bar's values, outside the history and not persisted.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FillToolOptions {
    /// The bar's + (true) or −; the sampling option sets it (Custom adds) and Alt reverses it.
    pub brush_add: bool,
    /// The Sampling Brush's diameter, 1..=5000; a tenth of the sampling window's side when the
    /// workspace opens (20 px for a 50 px selection).
    pub brush_size: f64,
    /// The Lasso's operation in the bar (Shift adds, Alt subtracts, both intersect at the press).
    pub lasso: SelectionOperation,
    /// Expand and Contract's amount, 1..=100 px (3 when the workspace opens; PhotoCraft
    /// `caf_ui/lifecycle.rs:30`).
    pub grow_by: f64,
}

impl FillToolOptions {
    /// The largest brush diameter the keys and the resize gesture set.
    pub const SIZE_MAX: f64 = 5000.0;

    /// The mode of a stroke pressed with the bar at `brush_add`: Alt reverses it.
    ///
    /// The brush takes the primary button always and the secondary while Alt is held
    /// ([`CanvasTool::takes`]), so an Alt + primary press reaches it and reverses the mode, while
    /// Alt + right-drag resizes.
    pub fn stroke_mode(brush_add: bool, alt: bool) -> EditMode {
        if brush_add != alt { EditMode::Add } else { EditMode::Subtract }
    }

    /// `[`: × 0.9 rounded, at least 1. `]`: × 1.1 rounded, at least a pixel larger, at most 5000.
    pub fn step_size(&mut self, larger: bool) {
        // pc: crates/ui-egui/src/dialogs/caf_ui/controls.rs:167-172 (PhotoCraft's `]` adds the
        // pixel after the cap, so at 5000 it reaches 5001; the cap holds here).
        let size = self.brush_size;
        self.brush_size =
            if larger { (size * 1.1).min(Self::SIZE_MAX).round().max(size + 1.0).min(Self::SIZE_MAX) } else { (size * 0.9).max(1.0).round() };
    }
}

/// The state of Content-Aware Fill's canvas between passes: the stroke being painted, the lasso
/// being drawn, the brush resize, the events not yet taken.
#[derive(Debug, Default)]
pub struct FillCanvas {
    stroke: Option<(GestureId, SamplingStroke)>,
    lasso: Option<(OutlineDraft, SelectionOperation)>,
    resize: BrushResize,
    events: Vec<FillEvent>,
    /// The points limit was reported for the stroke in progress.
    refused: bool,
}

impl FillCanvas {
    pub fn new() -> Self {
        Self::default()
    }

    /// The canvas tool for this pass. `session` and `limits` decide whether a stroke may begin
    /// or take another point.
    pub fn tool<'a>(
        &'a mut self,
        tool: FillTool,
        options: &'a mut FillToolOptions,
        session: &'a FillSession,
        limits: &'a FillLimits,
    ) -> FillCanvasTool<'a> {
        FillCanvasTool { canvas: self, tool, options, session, limits }
    }

    pub fn take_events(&mut self) -> Vec<FillEvent> {
        std::mem::take(&mut self.events)
    }

    pub fn busy(&self) -> bool {
        self.stroke.is_some() || self.lasso.is_some() || self.resize.active()
    }
}

/// [`FillCanvas`] as the `CanvasTool` of one pass.
///
/// - **Sampling Brush**: press, drag, release is one [`SamplingStroke`] (the bar's mode, reversed
///   by Alt at the press; points equal to the last are skipped), reported as
///   [`FillEvent::Stroke`] Begin, Update with each new point (the app paints its sampling plane
///   as the stroke goes), Commit. A stroke the limits refuse ([`FillSession::may_begin_stroke`])
///   does not begin: `FillEvent::Refused` with the message. A stroke that reaches the points
///   limit stops taking points.
/// - **Lasso**: an [`OutlineDraft`] of `DraftKind::Freehand`; the operation is the bar's, or
///   Shift (Add), Alt (Subtract), Shift+Alt (Intersect) at the press; the release reports
///   [`FillEvent::FillArea`] with [`FillAreaEdit::lasso`](super::FillAreaEdit::lasso) when it has
///   3 points or more.
/// - **Brush resize**: Alt + right-drag or Control + Alt + drag, size only, `max_diameter` 5000.
/// - Paint: the lasso path, the brush outline (`BrushCursor`; PhotoCraft's hand-drawn 1.5 pt black
///   and 0.75 pt white circle is replaced).
pub struct FillCanvasTool<'a> {
    canvas: &'a mut FillCanvas,
    tool: FillTool,
    options: &'a mut FillToolOptions,
    session: &'a FillSession,
    limits: &'a FillLimits,
}

impl FillCanvasTool<'_> {
    /// Drops the stroke (a `Cancel` event), the lasso and the resize without committing.
    fn drop_gesture(&mut self, reason: CancelReason) {
        let canvas = &mut *self.canvas;
        if let Some((id, _)) = canvas.stroke.take() {
            canvas.events.push(FillEvent::Stroke(EditEvent::cancel(id, reason)));
        }
        canvas.lasso = None;
        canvas.resize.cancel();
        canvas.refused = false;
    }

    /// Adds `at` to the stroke in progress and reports it. A point equal to the last is skipped;
    /// at the points limit the stroke takes no more points and the message is reported once.
    fn extend(&mut self, at: Point, modifiers: Modifiers) {
        let canvas = &mut *self.canvas;
        let Some((id, stroke)) = canvas.stroke.as_mut() else { return };
        let point = [at.x, at.y];
        if stroke.points.last() == Some(&point) {
            return;
        }
        if self.session.points_used(Some(stroke)) >= self.limits.points {
            if !canvas.refused {
                canvas.refused = true;
                canvas.events.push(FillEvent::Refused(self.limits.points_message()));
            }
            return;
        }
        stroke.points.push(point);
        canvas.events.push(FillEvent::Stroke(EditEvent::update(*id, stroke.clone(), modifiers)));
    }

    /// The resize's readout beside the pointer: the diameter only (the brush has no hardness).
    fn show_readout(&self, cx: &mut ToolContext<'_>) {
        let Some(dimensions) = self.canvas.resize.dimensions() else { return };
        let labels = BrushResizeLabels::of_ctx(&cx.canvas.ctx);
        let unit = Unit::new("{n} px");
        let pixels = format!("{}{}{}", unit.prefix(), cx.numbers.format(dimensions.diameter.round().max(0.0), 0..=0), unit.suffix());
        cx.readout(Readout::new("fill-brush-resize").row(labels.diameter.as_str(), pixels));
    }
}

fn sample(pointer: &ToolPointer, cx: &ToolContext<'_>) -> DraftPoint {
    DraftPoint::new(pointer.doc, pointer.modifiers, cx.view.zoom)
}

impl CanvasTool for FillCanvasTool<'_> {
    fn cursor(&self, cx: &ToolContext<'_>) -> ToolCursor {
        // pc: crates/ui-egui/src/dialogs/caf_ui/view.rs:405-409 (the brush circle).
        match self.tool {
            FillTool::SamplingBrush => {
                if self.canvas.resize.active() {
                    return ToolCursor::Hidden;
                }
                // A hard round brush: the 50 % contour is the edge. The preferences are the
                // library's defaults (the workspace has no cursor options of its own).
                let size = self.options.brush_size;
                let tip = BrushTip::round(size, size / 2.0);
                BrushCursor::new(&tip).painting(self.canvas.stroke.is_some()).cursor(
                    &BrushStyle::of(cx.tokens),
                    &CursorOptions::default(),
                    cx.view,
                    cx.caps_lock,
                )
            }
            FillTool::Lasso => ToolCursor::System(CursorIcon::Crosshair),
            FillTool::Hand | FillTool::Zoom => ToolCursor::System(CursorIcon::Default),
        }
    }

    fn takes(&self, button: PointerButton, modifiers: Modifiers) -> bool {
        // new: the brush resize chord (`BrushResize::chord`): the primary button always, the
        // secondary while Alt is held on the sampling brush.
        button == PointerButton::Primary || (self.tool == FillTool::SamplingBrush && button == PointerButton::Secondary && modifiers.alt)
    }

    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        // pc: crates/ui-egui/src/dialogs/caf_ui/controls.rs:109 (brush Down), :118-127 (lasso Down).
        match self.tool {
            FillTool::SamplingBrush => {
                if BrushResize::chord(pointer.button, pointer.modifiers) {
                    let options = BrushResizeOptions::new(&BrushStyle::of(cx.tokens), FillToolOptions::SIZE_MAX);
                    let start = BrushDimensions { diameter: self.options.brush_size, ..BrushDimensions::default() };
                    if self.canvas.resize.begin(cx.view, pointer.screen, start, &options).is_none() {
                        return ToolAction::Ignored;
                    }
                    self.show_readout(cx);
                    return ToolAction::Claimed;
                }
                if let Err(message) = self.session.may_begin_stroke(self.limits) {
                    self.canvas.events.push(FillEvent::Refused(message));
                    return ToolAction::Done;
                }
                // Alt at the press reverses the bar's mode.
                let mode = FillToolOptions::stroke_mode(self.options.brush_add, pointer.modifiers.alt);
                let stroke = SamplingStroke { mode, size: self.options.brush_size, points: vec![[pointer.doc.x, pointer.doc.y]] };
                let id = GestureId::next();
                self.canvas.refused = false;
                self.canvas.events.push(FillEvent::Stroke(EditEvent::begin_with(id, stroke.clone())));
                self.canvas.stroke = Some((id, stroke));
                ToolAction::Claimed
            }
            FillTool::Lasso => {
                let mut draft = OutlineDraft::new(DraftKind::Freehand, SelectToolStyle::of(cx.tokens).reach());
                // The fill area always has something to combine with: Shift adds, Alt subtracts,
                // both intersect, else the bar's operation.
                match draft.press(sample(pointer, cx), true) {
                    Some(DraftEvent::Started { intent }) => {
                        let operation = SelectionOperation::resolve(self.options.lasso, intent, true);
                        self.canvas.lasso = Some((draft, operation));
                        ToolAction::Claimed
                    }
                    _ => ToolAction::Ignored,
                }
            }
            FillTool::Hand | FillTool::Zoom => ToolAction::Ignored,
        }
    }

    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/dialogs/caf_ui/controls.rs:110-113, :128-134.
        if self.canvas.resize.active() {
            self.canvas.resize.track(cx.view, pointer.screen, pointer.modifiers);
            self.show_readout(cx);
        } else if self.canvas.stroke.is_some() {
            self.extend(pointer.doc, pointer.modifiers);
        } else if let Some((draft, _)) = self.canvas.lasso.as_mut() {
            draft.drag(sample(pointer, cx));
        }
    }

    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/dialogs/caf_ui/controls.rs:114-117, :135-141.
        if self.canvas.resize.active() {
            if let Some(EditEvent::Commit { value, .. }) = self.canvas.resize.finish(cx.view, pointer.screen, pointer.modifiers) {
                self.options.brush_size = value.diameter;
            }
            return;
        }
        if self.canvas.stroke.is_some() {
            self.extend(pointer.doc, pointer.modifiers);
            if let Some((id, stroke)) = self.canvas.stroke.take() {
                self.canvas.events.push(FillEvent::Stroke(EditEvent::commit(id, stroke, true, pointer.modifiers)));
            }
            return;
        }
        let sample = sample(pointer, cx);
        let Some((mut draft, operation)) = self.canvas.lasso.take() else { return };
        if let Some(DraftEvent::Closed { points, .. }) = draft.release(sample) {
            let points = points.iter().map(|p| [p.x, p.y]).collect();
            if let Some(edit) = FillAreaEdit::lasso(operation, points) {
                self.canvas.events.push(FillEvent::FillArea(edit));
            }
        }
    }

    fn hover(&mut self, _pointer: &ToolPointer, _cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/dialogs/caf_ui/controls.rs:111 (`hover`) only fed PhotoCraft's own
        // brush circle; the host draws the cursor from `cursor`, so nothing to track.
    }

    fn cancel(&mut self, reason: CancelReason, _cx: &mut ToolContext<'_>) {
        // new: Esc or a lost pointer drops the stroke or lasso.
        self.drop_gesture(reason);
    }

    fn key(&mut self, key: &ToolKey, _cx: &mut ToolContext<'_>) -> bool {
        // new: Esc cancels a stroke or lasso in progress (the host does it first during a
        // gesture; this covers a key that arrives with one still open).
        if key.pressed && key.key == Key::Escape && self.canvas.busy() {
            self.drop_gesture(CancelReason::Escape);
            return true;
        }
        false
    }

    fn paint(&self, painter: &Painter, cx: &ToolContext<'_>) {
        // pc: crates/ui-egui/src/dialogs/caf_ui/view.rs:399-404 (the lasso path, 1 pt
        // `t.accent`), :405-409 (the brush circle, drawn by `cursor`).
        if let Some((draft, _)) = &self.canvas.lasso
            && draft.points().len() >= 2
        {
            Marquee::polyline(draft.points().iter().copied(), "").paint(painter, cx.view, MarqueeLook::Drawing, cx.time);
        }
        if let Some(dimensions) = self.canvas.resize.dimensions() {
            let falloff = TipFalloff::hard(dimensions.diameter / 2.0);
            self.canvas.resize.paint(painter, cx.view, &BrushStyle::of(cx.tokens), &falloff);
        }
    }
}
