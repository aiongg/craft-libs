//! Stories of the brush cursor and the resize gesture. Fake
//! tips built by the story, so every state shows without an engine. The cursor story paints the
//! [`ToolCursor`] the library returns with the painter (the host would show it as a bitmap), so
//! the shapes show on any backend.
//!
//! - `primitives/brush-cursor/cursor`: round, traced (a star), thin ellipse and pixel tips; Normal and
//!   Full Size; view rotation, flip and aspect sliders; Caps Lock, painting and centre toggles;
//!   the second ring.
//! - `primitives/brush-resize/resize`: a canvas on a [`ToolHost`] where Ctrl + Alt + drag and Alt +
//!   right-drag resize the brush, and a Play button that injects either drag: diameter, the hardness, opacity and
//!   density levels, tint, hard and soft falloff, the readout.

use egui::{Color32, Modifiers, Painter, Pos2, Shape, Stroke, Ui, vec2};
use kurbo::{Point, Rect, Vec2};

use super::{
    BrushCursor, BrushDimensions, BrushLevel, BrushResize, BrushResizeLabels, BrushResizeOptions, BrushStyle, BrushTip, BrushTipOutline, TipFalloff,
    TipPose, TipRings,
};
use crate::canvas::story::{DOC, StoryCanvas, ViewArgs, document};
use crate::canvas::{
    CanvasTool, CursorHost, CursorOptions, CursorStyle, PaintingCursor, ToolAction, ToolContext, ToolCursor, ToolHost, ToolInput, ToolPointer,
};
use crate::controls::Button;
use crate::edit::CancelReason;
use crate::edit::EditEvent;
use crate::localized::{Labels as _, Units};
use crate::story::{self, Level, Story, StoryArgs, StoryChoice, StoryLabels, Surface};
use crate::theme::{Tokens, TokensOf as _};
use crate::typed_story;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["BrushCursor", "BrushResize"];

/// Points of the star tip's outline.
const STAR_POINTS: usize = 5;

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "primitives/brush-cursor/cursor",
            Level::Primitives,
            "BrushCursor",
            "Cursor",
            CursorArgs {
                view: ViewArgs::default(),
                tip: TipArg::Round,
                reach: ReachArg::Normal,
                diameter: 80.0,
                hardness: 0.6,
                angle: 0.0,
                squash: 0.3,
                round_cell: true,
                caps_lock: false,
                painting: false,
                crosshair_only_while_painting: true,
                center: false,
                second_ring: false,
            },
            show_cursor
        )
        .on(Surface::Canvas),
        typed_story!(
            "primitives/brush-resize/resize",
            Level::Primitives,
            "BrushResize",
            "Resize",
            ResizeArgs {
                view: ViewArgs::default(),
                level: LevelArg::Hardness,
                falloff: FalloffArg::Soft,
                tint: TintArg::Default,
                diameter: 60.0,
                start_level: 0.5,
                max_diameter: 200.0,
                drag_seconds: 2.0,
                drag: DragArg::CtrlAlt,
            },
            show_resize
        )
        .on(Surface::Canvas),
    ]
}

// --- cursor ---

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum TipArg {
    Round,
    Traced,
    #[choice(name = "Thin ellipse")]
    ThinEllipse,
    Pixels,
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum ReachArg {
    Normal,
    #[choice(name = "Full size")]
    FullSize,
    Precise,
}

impl ReachArg {
    fn painting(self) -> PaintingCursor {
        match self {
            ReachArg::Normal => PaintingCursor::NormalBrushTip,
            ReachArg::FullSize => PaintingCursor::FullSizeBrushTip,
            ReachArg::Precise => PaintingCursor::Precise,
        }
    }
}

#[derive(StoryArgs, Debug, PartialEq)]
struct CursorArgs {
    #[arg(flatten)]
    view: ViewArgs,
    tip: TipArg,
    reach: ReachArg,
    #[arg(range = 8.0..=240.0)]
    diameter: f64,
    /// Where the Normal outline sits between the middle and the full reach.
    #[arg(range = 0.0..=1.0)]
    hardness: f64,
    /// The tip's turn in degrees.
    #[arg(range = -180.0..=180.0)]
    angle: f64,
    /// The thin ellipse's short axis as a share of the long one.
    #[arg(range = 0.1..=1.0)]
    squash: f64,
    #[arg(name = "round pixel tip")]
    round_cell: bool,
    #[arg(name = "caps lock")]
    caps_lock: bool,
    painting: bool,
    #[arg(name = "crosshair only while painting")]
    crosshair_only_while_painting: bool,
    center: bool,
    #[arg(name = "second ring")]
    second_ring: bool,
}

/// A star of `STAR_POINTS` points in radius units, `scale` times the full reach.
fn star(scale: f32) -> Vec<[f32; 2]> {
    let corners = STAR_POINTS * 2;
    (0..corners)
        .map(|i| {
            let (sin, cos) = (std::f32::consts::TAU * i as f32 / corners as f32 - std::f32::consts::FRAC_PI_2).sin_cos();
            let r = if i % 2 == 0 { scale } else { scale * 0.45 }; // design: allow(the star's inner radius ratio)
            [cos * r, sin * r]
        })
        .collect()
}

/// The tip and pose the args describe, with the pointer at the document point `at`.
fn tip_of(args: &CursorArgs, at: Point) -> (BrushTip, TipPose) {
    let radius = args.diameter / 2.0;
    let normal = radius * (0.5 + 0.5 * args.hardness);
    let turned = TipPose::pen(args.angle, None);
    match args.tip {
        TipArg::Round => (BrushTip::round(args.diameter, normal), turned),
        TipArg::Traced => {
            let ratio = (normal / radius) as f32;
            let outline = BrushTipOutline::Traced { normal: TipRings::new(vec![star(ratio)]), full: TipRings::new(vec![star(1.0)]) };
            (BrushTip { diameter: args.diameter, outline }, turned)
        }
        TipArg::ThinEllipse => (BrushTip::round(args.diameter, normal), TipPose { x_axis: turned.x_axis, y_axis: turned.y_axis * args.squash }),
        TipArg::Pixels => {
            let (x, y, side) = ((at.x - radius).round(), (at.y - radius).round(), args.diameter.round());
            let outline = BrushTipOutline::Pixels { cell: Rect::new(x, y, x + side, y + side), round: args.round_cell };
            (BrushTip { diameter: args.diameter, outline }, TipPose::upright())
        }
    }
}

/// Paints the cursor the library returned at `at`, as the host's bitmap would look.
fn paint_cursor(painter: &Painter, at: Pos2, cursor: &ToolCursor, style: &CursorStyle) {
    let (line, halo) = (Stroke::new(style.stroke, style.line), Stroke::new(style.stroke * 2.0, style.halo));
    let cross = |arm: f32, gap: f32| {
        for dir in [vec2(1.0, 0.0), vec2(-1.0, 0.0), vec2(0.0, 1.0), vec2(0.0, -1.0)] {
            let ends = [at + dir * gap, at + dir * arm];
            painter.line_segment(ends, halo);
            painter.line_segment(ends, line);
        }
    };
    match cursor {
        ToolCursor::Crosshair { length, gap } => cross(*length, *gap),
        ToolCursor::Circle { radius, centre } => {
            painter.circle_stroke(at, *radius, halo);
            painter.circle_stroke(at, *radius, line);
            if *centre {
                cross(style.precise_reach / 2.0, 0.0);
            }
        }
        ToolCursor::Outline { rings, centre } => {
            for ring in rings.iter() {
                let points: Vec<Pos2> = ring.iter().map(|v| at + *v).collect();
                painter.add(Shape::closed_line(points.clone(), halo));
                painter.add(Shape::closed_line(points, line));
            }
            if *centre {
                cross(style.precise_reach / 2.0, 0.0);
            }
        }
        _ => {}
    }
}

/// The brush cursor over a document: it follows the pointer (the document's middle while the
/// pointer is away), under the cursor options the args set.
fn show_cursor(ui: &mut Ui, args: &mut CursorArgs) {
    let document_name = StoryLabels::of(ui).document;
    let canvas = document(ui, args.view, &document_name);
    let view = &canvas.view;
    let tokens = Tokens::of(ui);
    let (style, cursor_style) = (BrushStyle::of(&tokens), CursorHost::style(&tokens));
    let at = canvas.response.hover_pos().unwrap_or_else(|| view.to_screen(DOC.center()));
    let (tip, pose) = tip_of(args, view.to_doc(at));
    let mut cursor = BrushCursor::new(&tip).pose(pose).centre(args.center).painting(args.painting);
    if args.second_ring {
        cursor = cursor.inner_ring(args.diameter / 4.0);
    }
    let options = CursorOptions {
        painting: args.reach.painting(),
        show_only_crosshair_while_painting: args.crosshair_only_while_painting,
        ..CursorOptions::default()
    };
    let shown = cursor.cursor(&style, &options, view, args.caps_lock);
    let painter = ui.painter_at(view.rect);
    paint_cursor(&painter, at, &shown, &cursor_style);
    cursor.paint_pixels(&painter, view, &style);
}

// --- resize ---

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum LevelArg {
    Hardness,
    Opacity,
    Density,
}

impl LevelArg {
    fn level(self) -> BrushLevel {
        match self {
            LevelArg::Hardness => BrushLevel::Hardness,
            LevelArg::Opacity => BrushLevel::Opacity,
            LevelArg::Density => BrushLevel::Density,
        }
    }
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum FalloffArg {
    Hard,
    Soft,
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum TintArg {
    Default,
    Green,
}

/// The drag Play injects.
#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum DragArg {
    #[choice(name = "Ctrl + Alt + drag")]
    CtrlAlt,
    #[choice(name = "Alt + right-drag")]
    AltRight,
}

impl DragArg {
    fn button(self) -> egui::PointerButton {
        match self {
            DragArg::CtrlAlt => egui::PointerButton::Primary,
            DragArg::AltRight => egui::PointerButton::Secondary,
        }
    }

    fn modifiers(self) -> Modifiers {
        match self {
            DragArg::CtrlAlt => Modifiers { ctrl: true, alt: true, ..Modifiers::NONE },
            DragArg::AltRight => Modifiers::ALT,
        }
    }
}

#[derive(StoryArgs, Debug, PartialEq)]
struct ResizeArgs {
    #[arg(flatten)]
    view: ViewArgs,
    level: LevelArg,
    falloff: FalloffArg,
    tint: TintArg,
    /// The brush's diameter when the story starts or this arg changes.
    #[arg(range = 1.0..=400.0)]
    diameter: f64,
    /// The brush's level (and the soft falloff's hardness) at the same moments.
    #[arg(name = "start level", range = 0.0..=1.0)]
    start_level: f64,
    #[arg(name = "max diameter", range = 10.0..=400.0)]
    max_diameter: f64,
    #[arg(name = "drag seconds", range = 0.5..=6.0)]
    drag_seconds: f64,
    /// The drag the Play button injects.
    drag: DragArg,
}

/// The resize tool of the story: the brush's values, the gesture and the args' look.
#[derive(Clone, Debug, Default)]
struct ResizeTool {
    resize: BrushResize,
    dims: BrushDimensions,
    level: BrushLevel,
    max_diameter: f64,
    /// The soft falloff's hardness for the levels other than Hardness; `None` for a hard tip.
    soft: Option<f64>,
    tint: Option<Color32>,
    labels: BrushResizeLabels,
}

impl ResizeTool {
    fn falloff(&self, dims: BrushDimensions) -> TipFalloff {
        let radius = dims.diameter / 2.0;
        match self.soft {
            None => TipFalloff::hard(radius),
            Some(_) if self.level == BrushLevel::Hardness => TipFalloff::from_hardness(radius, dims.level),
            Some(hardness) => TipFalloff::from_hardness(radius, hardness),
        }
    }

    fn keep(&mut self, event: Option<EditEvent<BrushDimensions>>) {
        if let Some(EditEvent::Commit { value, .. }) = event {
            self.dims = value;
        }
    }
}

impl CanvasTool for ResizeTool {
    fn takes(&self, button: egui::PointerButton, modifiers: Modifiers) -> bool {
        BrushResize::takes(button, modifiers)
    }

    fn cursor(&self, _cx: &ToolContext<'_>) -> ToolCursor {
        if self.resize.active() { ToolCursor::Hidden } else { ToolCursor::System(egui::CursorIcon::Crosshair) }
    }

    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        if !BrushResize::chord(pointer.button, pointer.modifiers) {
            return ToolAction::Ignored;
        }
        let mut options = BrushResizeOptions::new(&BrushStyle::of(cx.tokens), self.max_diameter).level(self.level);
        if let Some(tint) = self.tint {
            options = options.tint(tint);
        }
        match self.resize.begin(cx.view, pointer.screen, self.dims, &options) {
            Some(_) => ToolAction::Claimed,
            None => ToolAction::Ignored,
        }
    }

    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        self.resize.track(cx.view, pointer.screen, pointer.modifiers);
        if let Some(readout) = self.resize.readout(&Units::of_ctx(&cx.canvas.ctx), &self.labels) {
            cx.readout(readout);
        }
    }

    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        let event = self.resize.finish(cx.view, pointer.screen, pointer.modifiers);
        self.keep(event);
    }

    fn cancel(&mut self, _reason: CancelReason, _cx: &mut ToolContext<'_>) {
        self.resize.cancel();
    }

    fn paint(&self, painter: &Painter, cx: &ToolContext<'_>) {
        let dims = self.resize.dimensions().unwrap_or(self.dims);
        self.resize.paint(painter, cx.view, &BrushStyle::of(cx.tokens), &self.falloff(dims));
    }
}

/// A scripted drag in progress: when it began, whether the press went out and which drag it is.
#[derive(Clone, Copy, Debug)]
struct Script {
    start: f64,
    pressed: bool,
    drag: DragArg,
}

/// What the story keeps between passes.
#[derive(Clone, Debug)]
struct ResizeState {
    tool: ResizeTool,
    script: Option<Script>,
}

/// Feeds the scripted drag to the host: a press in the document's middle, moves right and down,
/// the release, with the modifiers and button of the script's drag. One input per pass.
fn run_script(ui: &Ui, canvas: &StoryCanvas, script: &mut Option<Script>, seconds: f64) {
    let Some(run) = script else { return };
    let ctx = ui.ctx();
    let t = ((ctx.input(|i| i.time) - run.start) / seconds.max(0.1)).clamp(0.0, 1.0);
    let from = DOC.center();
    let to = from + Vec2::new(DOC.width() * 0.15, DOC.height() * 0.15); // design: allow(a share of the document, not a size)
    let doc = from + (to - from) * t;
    let (button, modifiers) = (run.drag.button(), run.drag.modifiers());
    let input = if !run.pressed {
        run.pressed = true;
        ToolInput::Down { button, doc: from, pressure: 1.0, tilt: [0.0, 0.0], rotation: 0.0, modifiers }
    } else if t < 1.0 {
        ToolInput::Move { doc, pressure: 1.0, tilt: [0.0, 0.0], rotation: 0.0, modifiers }
    } else {
        *script = None;
        ToolInput::Up { doc, modifiers }
    };
    ToolHost::inject(ctx, canvas.id, input);
}

/// Ctrl + Alt + drag or Alt + right-drag on the document resizes the brush: left and right set the diameter, up and
/// down the level, with the falloff drawn at the press and the readout beside the pointer. Play
/// runs the same drag by injected pointer events.
fn show_resize(ui: &mut Ui, args: &mut ResizeArgs) {
    let document_name = StoryLabels::of(ui).document;
    let canvas = document(ui, args.view, &document_name);
    let ctx = ui.ctx().clone();
    let seeded = BrushDimensions { diameter: args.diameter, level: args.start_level };
    story::reset_on(ui, "resize", &(args.diameter, args.start_level));
    let shared = story::shared(ui, "resize", || ResizeState { tool: ResizeTool { dims: seeded, ..ResizeTool::default() }, script: None });
    let mut state = story::lock(&shared);
    state.tool.level = args.level.level();
    state.tool.max_diameter = args.max_diameter;
    state.tool.soft = (args.falloff == FalloffArg::Soft).then_some(args.start_level);
    state.tool.tint = (args.tint == TintArg::Green).then_some(Color32::from_rgba_unmultiplied(0, 160, 60, 110)); // design: allow(the user's Brush Preview colour)
    state.tool.labels = BrushResizeLabels::of(ui);
    run_script(ui, &canvas, &mut state.script, args.drag_seconds);
    let _ = ToolHost::new(canvas.id).document(DOC.size()).show(ui, &canvas.view, &canvas.response, &mut state.tool);
    let play = StoryLabels::of(ui).play;
    if ui.add(Button::new(play.as_str())).clicked() && state.script.is_none() {
        state.script = Some(Script { start: ctx.input(|i| i.time), pressed: false, drag: args.drag });
    }
    if state.script.is_some() {
        ctx.request_repaint();
    }
}
