//! Stories of the nudge, the move gesture and the stroke constraints (task KS2; task RG2
//! registers them). A fake page with rectangles and strokes, so every state shows without an app.
//! Sample text is a story word where one exists; the plain strings are the axis and flag names in
//! the status lines.
//!
//! - `primitives/nudge/arrows`: a rectangle on a fake page; arrows move it 1 unit, ⇧ 10, a ⌘ base
//!   toggle, the ⌥ copy flag in a status line; a text field beside it takes the arrows when focused.
//! - `primitives/move-gesture/layers`: the four-direction ⇧ lock with a locked-axis readout and the
//!   ⌥ copy flag.
//! - `primitives/move-gesture/pixels`: the eight-direction lock.
//! - `primitives/move-gesture/stroke`: `StrokeLock` and the Gradient tool's `snap_ray` over a drawn
//!   stroke, with the Shift-click `StraightLinePreview`.
#![expect(dead_code, reason = "registered by task RG2")]

use egui::{Modifiers, Painter, Shape, Stroke, Ui};
use kurbo::{Point, Rect, Vec2};

use super::move_gesture::{ConstrainStyle, Directions, MoveGesture, MoveValue, StrokeLock, snap_ray};
use super::nudge::{self, NudgeKeys};
use crate::canvas::story::{DOC, ViewArgs, document};
use crate::canvas::straight_line::{LineGuideStyle, StraightLinePreview};
use crate::canvas::{CanvasTool, CanvasView, ToolAction, ToolContext, ToolCursor, ToolHost, ToolPointer};
use crate::controls::{Button, TextField};
use crate::edit::EditEvent;
use crate::localized::{Labels as _, LocaleNumbers};
use crate::primitives::{Content, Text};
use crate::story::{Level, Story, StoryArgs, StoryChoice, StoryLabels, Surface};
use crate::theme::{ToEgui as _, Tokens, TokensOf as _, UiTokens};
use crate::typed_story;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["NudgeKeys", "MoveGesture", "StrokeLock"];

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "primitives/nudge/arrows",
            Level::Primitives,
            "NudgeKeys",
            "Arrows",
            NudgeArgs {
                view: ViewArgs::default(),
                step: nudge::STEP,
                big_step: nudge::BIG_STEP,
                command_base: false,
                alt_copies: true,
                width: 120.0,
                height: 80.0,
            },
            show_nudge
        )
        .on(Surface::Canvas),
        typed_story!(
            "primitives/move-gesture/layers",
            Level::Primitives,
            "MoveGesture",
            "Layers",
            MoveArgs { view: ViewArgs::default(), width: 140.0, height: 90.0 },
            show_layers
        )
        .on(Surface::Canvas),
        typed_story!(
            "primitives/move-gesture/pixels",
            Level::Primitives,
            "MoveGesture",
            "Pixels",
            MoveArgs { view: ViewArgs::default(), width: 140.0, height: 90.0 },
            show_pixels
        )
        .on(Surface::Canvas),
        typed_story!(
            "primitives/move-gesture/stroke",
            Level::Primitives,
            "StrokeLock",
            "Stroke",
            StrokeArgs { view: ViewArgs::default(), tool: StrokeToolArg::Paint, width: 12.0 },
            show_stroke
        )
        .on(Surface::Canvas),
    ]
}

// --- shared parts ---

/// The fill of the story's rectangles: the accent, translucent.
fn fill(tokens: &UiTokens, strength: f32) -> egui::Color32 {
    tokens.color.accent.solid.to_egui().gamma_multiply(strength)
}

/// Paints `rect` (document units) through `view`.
fn paint_rect(painter: &Painter, view: &CanvasView, tokens: &UiTokens, rect: Rect, strength: f32) {
    let stroke = Stroke::new(tokens.component.line_guide.width, tokens.color.accent.solid.to_egui());
    painter.add(Shape::convex_polygon(view.quad(rect).to_vec(), fill(tokens, strength), stroke));
}

/// Remembers the story's state under a name of this module's id space.
fn state_key(ui: &Ui, name: &str) -> egui::Id {
    ui.make_persistent_id(("craft_ui::behavior::story", name))
}

/// A muted status line.
fn status(ui: &mut Ui, text: String) {
    ui.add(Text::new(text).content(Content::Muted));
}

// --- nudge ---

#[derive(StoryArgs, Debug, PartialEq)]
struct NudgeArgs {
    #[arg(flatten)]
    view: ViewArgs,
    #[arg(range = 0.5..=5.0)]
    step: f64,
    #[arg(name = "shift step", range = 5.0..=50.0)]
    big_step: f64,
    #[arg(name = "command base")]
    command_base: bool,
    #[arg(name = "alt copies")]
    alt_copies: bool,
    #[arg(range = 20.0..=300.0)]
    width: f64,
    #[arg(range = 20.0..=300.0)]
    height: f64,
}

/// What the nudge story keeps: the rectangle's corner, the copies left behind, the text field's
/// text and the last nudge.
#[derive(Clone, Debug)]
struct NudgeState {
    origin: Point,
    copies: Vec<Point>,
    text: String,
    last: Option<nudge::Nudge>,
}

/// A rectangle on the page: click the page to give it the keyboard, then the arrows nudge it. The
/// text field beside the status takes the arrows while it has focus.
fn show_nudge(ui: &mut Ui, args: &mut NudgeArgs) {
    let words = StoryLabels::of(ui);
    let canvas = document(ui, args.view, &words.document);
    let ctx = ui.ctx().clone();
    let key = state_key(ui, "nudge");
    let mut state: NudgeState = ctx.data(|d| d.get_temp(key)).unwrap_or_else(|| NudgeState {
        origin: DOC.center() - Vec2::new(args.width / 2.0, args.height / 2.0),
        copies: Vec::new(),
        text: words.photo.clone(),
        last: None,
    });
    if canvas.response.has_focus() {
        let base = if args.command_base { Modifiers::COMMAND } else { Modifiers::NONE };
        let keys = NudgeKeys::new().steps(args.step, args.big_step).base(base).alt(args.alt_copies).layer(ui.layer_id());
        if let Some(nudge) = keys.read(&ctx) {
            if nudge.alt {
                state.copies.push(state.origin);
            }
            state.origin += nudge.by;
            state.last = Some(nudge);
        }
    }
    let tokens = Tokens::of(ui);
    let painter = ui.painter_at(canvas.view.rect);
    let size = Vec2::new(args.width, args.height);
    for copy in &state.copies {
        paint_rect(&painter, &canvas.view, &tokens, Rect::from_origin_size(*copy, size.to_size()), 0.25);
    }
    paint_rect(&painter, &canvas.view, &tokens, Rect::from_origin_size(state.origin, size.to_size()), 0.5);
    ui.horizontal(|ui| {
        let out = TextField::new("nudge-story-text", words.text.as_str(), &state.text).show(ui);
        if let Some(text) = out.event.as_ref().and_then(|e| e.value()) {
            state.text = text.clone();
        }
        let numbers = LocaleNumbers::of(ui);
        let mut line = format!("{}, {}", numbers.format(state.origin.x, 0..=1), numbers.format(state.origin.y, 0..=1));
        if let Some(last) = state.last {
            if last.shift {
                line.push_str(" · Shift");
            }
            if last.alt {
                line.push_str(" · Copy");
            }
        }
        status(ui, line);
    });
    ctx.data_mut(|d| d.insert_temp(key, state));
}

// --- move ---

#[derive(StoryArgs, Debug, PartialEq)]
struct MoveArgs {
    #[arg(flatten)]
    view: ViewArgs,
    #[arg(range = 20.0..=300.0)]
    width: f64,
    #[arg(range = 20.0..=300.0)]
    height: f64,
}

/// The Move tool of the stories: rectangles to drag by a [`MoveGesture`]. With ⌥ held at the press
/// a drag leaves the original and moves a copy.
#[derive(Clone, Debug)]
struct MoveTool {
    directions: Directions,
    rects: Vec<Rect>,
    grabbed: Option<usize>,
    gesture: Option<MoveGesture>,
    value: MoveValue,
}

impl MoveTool {
    fn new(directions: Directions, rects: Vec<Rect>) -> Self {
        Self { directions, rects, grabbed: None, gesture: None, value: MoveValue::default() }
    }

    fn drop_gesture(&mut self) {
        self.grabbed = None;
        self.gesture = None;
        self.value = MoveValue::default();
    }
}

impl CanvasTool for MoveTool {
    fn cursor(&self, _cx: &ToolContext<'_>) -> ToolCursor {
        ToolCursor::System(egui::CursorIcon::Move)
    }

    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        let Some(hit) = self.rects.iter().rposition(|r| r.contains(pointer.doc)) else { return ToolAction::Ignored };
        let (gesture, _) = MoveGesture::begin(&ConstrainStyle::of(cx.tokens), pointer.doc, pointer.modifiers, self.directions);
        self.gesture = Some(gesture);
        self.grabbed = Some(hit);
        self.value = MoveValue::default();
        ToolAction::Claimed
    }

    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        if let Some(gesture) = &mut self.gesture
            && let Some(event) = gesture.drag(pointer.doc, pointer.modifiers, cx.view.zoom)
            && let Some(value) = event.value()
        {
            self.value = *value;
        }
    }

    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        let (Some(gesture), Some(at)) = (self.gesture.take(), self.grabbed.take()) else { return };
        if let EditEvent::Commit { value, .. } = gesture.end(pointer.doc, pointer.modifiers, cx.view.zoom) {
            let moved = self.rects[at] + value.delta;
            if value.copy {
                self.rects.push(moved);
            } else {
                self.rects[at] = moved;
            }
            self.value = value;
        }
    }

    fn cancel(&mut self, _cx: &mut ToolContext<'_>) {
        if let Some(gesture) = self.gesture.take() {
            gesture.cancel();
        }
        self.drop_gesture();
    }

    fn paint(&self, painter: &Painter, cx: &ToolContext<'_>) {
        for (at, rect) in self.rects.iter().enumerate() {
            let moving = self.grabbed == Some(at) && self.gesture.is_some();
            paint_rect(painter, cx.view, cx.tokens, *rect, if moving && !self.value.copy { 0.15 } else { 0.5 });
            if moving {
                paint_rect(painter, cx.view, cx.tokens, *rect + self.value.delta, 0.5);
            }
        }
    }
}

/// The locked axis of a move: free without ⇧.
fn axis(delta: Vec2, shift: bool) -> &'static str {
    if !shift || delta == Vec2::ZERO {
        "Free"
    } else if delta.y == 0.0 {
        "Horizontal"
    } else if delta.x == 0.0 {
        "Vertical"
    } else {
        "Diagonal"
    }
}

/// The move canvas: drag a rectangle, ⇧ locks the direction, ⌥ held at the press copies. Reset
/// puts the rectangles back.
fn show_move(ui: &mut Ui, args: &MoveArgs, name: &str, directions: Directions, count: usize) {
    let words = StoryLabels::of(ui);
    let canvas = document(ui, args.view, &words.document);
    let ctx = ui.ctx().clone();
    let key = state_key(ui, name);
    let size = Vec2::new(args.width, args.height).to_size();
    let fresh = || {
        let first = Rect::from_origin_size(DOC.center() - Vec2::new(args.width, args.height / 2.0), size);
        let offset = Vec2::new(args.width / 2.0, args.height / 2.0);
        MoveTool::new(directions, (0..count).map(|i| first + offset * i as f64).collect())
    };
    let mut tool: MoveTool = ctx.data(|d| d.get_temp(key)).unwrap_or_else(fresh);
    let _ = ToolHost::new(canvas.id).document(DOC.size()).show(ui, &canvas.view, &canvas.response, &mut tool);
    let shift = ctx.input(|i| i.modifiers.shift);
    ui.horizontal(|ui| {
        if ui.add(Button::new(words.reset.as_str())).clicked() {
            tool = fresh();
        }
        let mut line = axis(tool.value.delta, shift).to_owned();
        if tool.value.copy {
            line.push_str(" · Copy");
        }
        status(ui, line);
    });
    ctx.data_mut(|d| d.insert_temp(key, tool));
}

/// Two overlapping layers, four directions under ⇧.
fn show_layers(ui: &mut Ui, args: &mut MoveArgs) {
    show_move(ui, args, "layers", Directions::Four, 2);
}

/// Selected pixels, eight directions under ⇧.
fn show_pixels(ui: &mut Ui, args: &mut MoveArgs) {
    show_move(ui, args, "pixels", Directions::Eight, 1);
}

// --- stroke ---

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum StrokeToolArg {
    Paint,
    Gradient,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct StrokeArgs {
    #[arg(flatten)]
    view: ViewArgs,
    tool: StrokeToolArg,
    /// The stroke's width in document units.
    #[arg(range = 1.0..=40.0)]
    width: f64,
}

/// A painting tool and a gradient tool in one: ⇧ locks the stroke to 0°, 45° or 90° from where the
/// lock began ([`StrokeLock`]) or snaps the gradient line ([`snap_ray`]); ⇧ held over the page
/// with a stroke behind shows the line a click would paint ([`StraightLinePreview`]).
#[derive(Clone, Debug)]
struct StrokeDemo {
    tool: StrokeToolArg,
    width: f64,
    lock: Option<StrokeLock>,
    current: Vec<Point>,
    lines: Vec<Vec<Point>>,
    gradient: Option<(Point, Point)>,
    hover: Option<Point>,
    shift: bool,
}

impl StrokeDemo {
    fn last_end(&self) -> Option<Point> {
        self.lines.last().and_then(|line| line.last().copied())
    }

    fn painting(&self) -> bool {
        !self.current.is_empty()
    }
}

impl CanvasTool for StrokeDemo {
    fn cursor(&self, _cx: &ToolContext<'_>) -> ToolCursor {
        ToolCursor::System(egui::CursorIcon::Crosshair)
    }

    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        match self.tool {
            StrokeToolArg::Paint => {
                if pointer.modifiers.shift
                    && let Some(from) = self.last_end()
                {
                    self.lines.push(vec![from, pointer.doc]);
                    return ToolAction::Done;
                }
                self.lock = Some(StrokeLock::new(&ConstrainStyle::of(cx.tokens)));
                self.current = vec![pointer.doc];
            }
            StrokeToolArg::Gradient => self.gradient = Some((pointer.doc, pointer.doc)),
        }
        ToolAction::Claimed
    }

    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        let shift = pointer.modifiers.shift;
        self.shift = shift;
        match self.tool {
            StrokeToolArg::Paint => {
                let (Some(lock), Some(&last)) = (&mut self.lock, self.current.last()) else { return };
                let point = lock.constrain(last, pointer.doc, shift, cx.view.zoom);
                if point != last {
                    self.current.push(point);
                }
            }
            StrokeToolArg::Gradient => {
                if let Some((from, to)) = &mut self.gradient {
                    *to = if shift { snap_ray(*from, pointer.doc, std::f64::consts::FRAC_PI_4) } else { pointer.doc };
                }
            }
        }
    }

    fn release(&mut self, _pointer: &ToolPointer, _cx: &mut ToolContext<'_>) {
        if self.tool == StrokeToolArg::Paint && self.painting() {
            self.lines.push(std::mem::take(&mut self.current));
        }
        if let Some(lock) = &mut self.lock {
            lock.reset();
        }
    }

    fn hover(&mut self, pointer: &ToolPointer, _cx: &mut ToolContext<'_>) {
        self.hover = Some(pointer.doc);
        self.shift = pointer.modifiers.shift;
    }

    fn cancel(&mut self, _cx: &mut ToolContext<'_>) {
        self.current.clear();
        self.gradient = None;
        if let Some(lock) = &mut self.lock {
            lock.reset();
        }
    }

    fn paint(&self, painter: &Painter, cx: &ToolContext<'_>) {
        let colour = cx.tokens.color.accent.solid.to_egui();
        let width = self.width as f32 * cx.view.zoom as f32;
        for line in self.lines.iter().chain(std::iter::once(&self.current)) {
            let points = line.iter().map(|p| cx.view.to_screen(*p)).collect::<Vec<_>>();
            if points.len() > 1 {
                painter.add(Shape::line(points, Stroke::new(width, colour)));
            }
        }
        let guide = LineGuideStyle::of(cx.tokens);
        if let Some((from, to)) = self.gradient {
            StraightLinePreview::new(from, to).paint(painter, cx.view, &guide);
        }
        if self.tool == StrokeToolArg::Paint
            && self.shift
            && !self.painting()
            && let (Some(from), Some(to)) = (self.last_end(), self.hover)
        {
            StraightLinePreview::new(from, to).paint(painter, cx.view, &guide);
        }
    }
}

/// Drag to paint or to pull a gradient line; hold ⇧ to lock. With Paint, ⇧ over the page shows the
/// Shift-click line from the end of the last stroke, and a ⇧ click paints it.
fn show_stroke(ui: &mut Ui, args: &mut StrokeArgs) {
    let words = StoryLabels::of(ui);
    let canvas = document(ui, args.view, &words.document);
    let ctx = ui.ctx().clone();
    let key = state_key(ui, "stroke");
    let mut tool: StrokeDemo = ctx.data(|d| d.get_temp(key)).unwrap_or_else(|| StrokeDemo {
        tool: args.tool,
        width: args.width,
        lock: None,
        current: Vec::new(),
        lines: Vec::new(),
        gradient: None,
        hover: None,
        shift: false,
    });
    tool.width = args.width;
    if tool.tool != args.tool {
        tool.tool = args.tool;
        tool.gradient = None;
    }
    let _ = ToolHost::new(canvas.id).document(DOC.size()).show(ui, &canvas.view, &canvas.response, &mut tool);
    ui.horizontal(|ui| {
        if ui.add(Button::new(words.reset.as_str())).clicked() {
            tool.lines.clear();
            tool.current.clear();
            tool.gradient = None;
            tool.lock = None;
        }
        let locked = tool.lock.as_ref().is_some_and(StrokeLock::locked);
        status(ui, if locked { "Locked" } else { "Free" }.to_owned());
    });
    ctx.data_mut(|d| d.insert_temp(key, tool));
}
