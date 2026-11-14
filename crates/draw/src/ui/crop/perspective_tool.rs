//! [`PerspectiveCropTool`]: the Perspective Crop tool's background presses. A box drag draws an upright quad, and four
//! clicks place its corners one by one. Once a quad exists, [`PerspectiveCropQuad`](super::PerspectiveCropQuad)
//! takes the presses on its corners and inside, and a press elsewhere does nothing.

use std::sync::Arc;

use craft_behavior::kurbo::Point;
use craft_behavior::{CancelReason, Cursor, CursorIcon, EditEvent, GestureId, Mixed};
use craft_ui::canvas::{CanvasTool, ToolAction, ToolContext, ToolKey, ToolPath, ToolPointer};
use craft_ui::patterns::form_dialogs::{Param, ParamDialog, ParamKind};
use craft_ui::primitives::{Draw, PaintShape, StrokeWidth};
use egui::Pos2;

use super::perspective_crop::{PerspectiveQuad, ordered};
use super::words::CropWords;

/// The key of the Perspective Crop To... command form.
const PERSPECTIVE_TO_KEY: &str = "draw.crop.perspectiveTo";

/// The keys of the form's four corner rows, clockwise from the top left.
const CORNER_KEYS: [&str; 4] = ["topLeft", "topRight", "bottomRight", "bottomLeft"];

/// What the Perspective Crop tool reports, from [`PerspectiveCropTool::take_events`].
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum PerspectiveCropEvent {
    /// The tool drew the quad: a box drag (`Begin`, `Commit`; `Cancel` on Esc) or the fourth click (`Set`).
    Edit(EditEvent<PerspectiveQuad>),
}

/// A press the tool owns, on the background.
#[derive(Clone, Copy, Debug)]
struct Press {
    id: GestureId,
    /// The press in document space.
    anchor: Point,
    /// The press on screen.
    origin: Pos2,
    /// The quad before the press.
    before: PerspectiveQuad,
    began: bool,
}

/// The Perspective Crop tool, kept by the app. It owns the quad: show it in the canvas body as
/// `PerspectiveCropQuad::new(label, Bind::bound(tool.quad_mut()))`, and read the tool's own edits with
/// [`take_events`](Self::take_events).
///
/// - With no quad, a drag draws an upright box (none while it has no area), and four clicks place corners one by one;
///   the fourth makes the quad, corners put in order. A drag clears the clicks.
/// - Esc drops the pending clicks. Once a quad exists, the quad's control takes Enter, Esc and the arrows
///   ([`PerspectiveCropOutput::apply`](super::PerspectiveCropOutput::apply) and
///   [`discard`](super::PerspectiveCropOutput::discard)).
/// - Its keyboard path is the Perspective Crop To... command ([`perspective_to_form`]).
#[derive(Clone, Debug, Default)]
pub struct PerspectiveCropTool {
    quad: PerspectiveQuad,
    clicks: Vec<Point>,
    press: Option<Press>,
    events: Vec<PerspectiveCropEvent>,
}

impl PerspectiveCropTool {
    /// A tool with no quad.
    pub fn new() -> Self {
        Self::default()
    }

    /// The quad.
    pub fn quad(&self) -> &PerspectiveQuad {
        &self.quad
    }

    /// The quad, for the control that edits it in place.
    pub fn quad_mut(&mut self) -> &mut PerspectiveQuad {
        &mut self.quad
    }

    /// Sets the quad (the form's answer, an undo step) and drops the pending clicks.
    pub fn set(&mut self, quad: PerspectiveQuad) {
        self.quad = quad;
        self.clicks.clear();
    }

    /// Drops the quad and the pending clicks: the active document changed, or the crop was applied.
    pub fn clear(&mut self) {
        self.set(None);
        self.press = None;
    }

    /// The corners clicked so far, before the fourth makes the quad.
    pub fn placing(&self) -> usize {
        self.clicks.len()
    }

    /// The events since the last call, in order.
    pub fn take_events(&mut self) -> Vec<PerspectiveCropEvent> {
        std::mem::take(&mut self.events)
    }
}

/// The upright quad of a box drawn from `anchor` to `to`; none while the box has no area.
fn boxed(anchor: Point, to: Point) -> PerspectiveQuad {
    let (x0, y0, x1, y1) = (anchor.x.min(to.x), anchor.y.min(to.y), anchor.x.max(to.x), anchor.y.max(to.y));
    (x1 > x0 && y1 > y0).then_some([Point::new(x0, y0), Point::new(x1, y0), Point::new(x1, y1), Point::new(x0, y1)])
}

/// What the host hands the perspective crop pass each frame.
#[derive(Clone, Copy, Debug, Default)]
#[non_exhaustive]
pub struct PerspectiveInput<'a> {
    /// Whether the quad's grid shows.
    pub show_grid: bool,
    /// Ties the input to the pass's lifetime until the shield (tl.T44, not in this checkout) joins it.
    pub marker: std::marker::PhantomData<&'a ()>,
}

/// This pass's perspective crop tool for the host, borrowing its input; it is the canvas tool while the pass lives.
pub struct PerspectiveCropPass<'a> {
    tool: &'a mut PerspectiveCropTool,
    input: PerspectiveInput<'a>,
}

impl PerspectiveCropTool {
    /// This pass's tool for the host, borrowing `input`'s data.
    pub fn pass<'a>(&'a mut self, input: PerspectiveInput<'a>) -> PerspectiveCropPass<'a> {
        PerspectiveCropPass { tool: self, input }
    }
}

impl PerspectiveCropPass<'_> {
    /// Shows the quad over the canvas at `canvas_rect` and reports its edits.
    pub fn show_quad(&mut self, ui: &mut egui::Ui, canvas_rect: egui::Rect) -> super::PerspectiveCropOutput {
        let _ = (&self.tool, &self.input, ui, canvas_rect);
        todo!("tl.T48")
    }
}

impl CanvasTool for PerspectiveCropPass<'_> {
    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        self.tool.press(pointer, cx)
    }

    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        self.tool.drag(pointer, cx);
    }

    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        self.tool.release(pointer, cx);
    }

    fn cancel(&mut self, reason: CancelReason, cx: &mut ToolContext<'_>) {
        self.tool.cancel(reason, cx);
    }

    fn key(&mut self, key: &ToolKey, cx: &mut ToolContext<'_>) -> bool {
        self.tool.key(key, cx)
    }

    fn paint(&self, draw: &Draw<'_>, cx: &ToolContext<'_>) {
        self.tool.paint(draw, cx);
    }

    fn cursor(&self, cx: &ToolContext<'_>) -> Cursor {
        self.tool.cursor(cx)
    }

    fn command_form(&self) -> ToolPath {
        self.tool.command_form()
    }
}

impl CanvasTool for PerspectiveCropTool {
    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        let Some(space) = cx.space() else { return ToolAction::Ignored };
        let anchor = space.view.to_doc(pointer.pos);
        if self.quad.is_some() || !(anchor.x.is_finite() && anchor.y.is_finite()) {
            return ToolAction::Ignored;
        }
        let Some(id) = cx.gesture_id() else { return ToolAction::Ignored };
        self.press = Some(Press { id, anchor, origin: pointer.pos, before: self.quad, began: false });
        ToolAction::Claimed
    }

    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        let (Some(mut press), Some(space)) = (self.press, cx.space()) else { return };
        if !press.began && (pointer.pos - press.origin).abs().max_elem() > cx.slop {
            press.began = true;
            self.clicks.clear();
            let label: Arc<str> = Arc::from(cx.words::<CropWords>().perspective_group.as_str());
            self.events.push(PerspectiveCropEvent::Edit(EditEvent::begin(press.id, Mixed::Same(press.before), label, None)));
        }
        if press.began {
            self.quad = boxed(press.anchor, space.view.to_doc(pointer.pos));
            self.events.push(PerspectiveCropEvent::Edit(EditEvent::update(press.id, self.quad)));
        }
        self.press = Some(press);
    }

    fn release(&mut self, _: &ToolPointer, cx: &mut ToolContext<'_>) {
        let Some(press) = self.press.take() else { return };
        if press.began {
            self.events.push(PerspectiveCropEvent::Edit(EditEvent::commit(press.id, Mixed::Same(press.before), self.quad)));
            return;
        }
        self.clicks.push(press.anchor);
        if let [a, b, c, d] = self.clicks[..] {
            self.clicks.clear();
            let quad = Some(ordered([a, b, c, d]));
            let label: Arc<str> = Arc::from(cx.words::<CropWords>().perspective_group.as_str());
            self.quad = quad;
            self.events.push(PerspectiveCropEvent::Edit(EditEvent::set(Mixed::Same(press.before), quad, label, None)));
        }
    }

    fn cancel(&mut self, reason: CancelReason, _: &mut ToolContext<'_>) {
        let Some(press) = self.press.take() else { return };
        if press.began {
            self.quad = press.before;
            self.events.push(PerspectiveCropEvent::Edit(EditEvent::cancel(press.id, reason, Some(Mixed::Same(press.before)))));
        }
    }

    fn key(&mut self, key: &ToolKey, _: &mut ToolContext<'_>) -> bool {
        match key {
            ToolKey::Cancel if !self.clicks.is_empty() => {
                self.clicks.clear();
                true
            }
            _ => false,
        }
    }

    /// The corners clicked so far, joined by a line.
    fn paint(&self, draw: &Draw<'_>, cx: &ToolContext<'_>) {
        let (Some(space), Some(style)) = (cx.space(), cx.style()) else { return };
        let dots: Vec<Pos2> = self.clicks.iter().map(|click| space.view.to_screen(*click)).collect();
        let handle = style.handle();
        let line = style.color(style.tokens.color.overlay.line);
        if dots.len() > 1 {
            draw.stroke(&PaintShape::Path(dots.clone()), StrokeWidth::Medium, line);
        }
        for dot in dots {
            let shape = PaintShape::Circle(dot, handle.side / 2.0);
            draw.fill(&shape, handle.fill.off.rest);
            draw.stroke(&shape, StrokeWidth::Thin, line);
        }
    }

    fn cursor(&self, _: &ToolContext<'_>) -> Cursor {
        Cursor::System(if self.quad.is_none() { CursorIcon::Crosshair } else { CursorIcon::Default })
    }

    fn command_form(&self) -> ToolPath {
        ToolPath::Command("the Perspective Crop To... command")
    }
}

/// The Perspective Crop To... command's form: the four corners of `quad`, clockwise from the top left, in document
/// units. Without a quad the corners start at the origin.
pub fn perspective_to_form(words: &CropWords, quad: &PerspectiveQuad) -> ParamDialog<'static> {
    let corners = quad.unwrap_or([Point::ZERO; 4]);
    let names = [&words.perspective_top_left, &words.perspective_top_right, &words.perspective_bottom_right, &words.perspective_bottom_left];
    let params: Vec<Param> = names
        .into_iter()
        .zip(corners)
        .zip(CORNER_KEYS)
        .map(|((name, corner), key)| Param::new(key, name.clone(), ParamKind::point([corner.x, corner.y])))
        .collect();
    ParamDialog::command(PERSPECTIVE_TO_KEY, words.perspective_crop_to.clone(), params)
}
