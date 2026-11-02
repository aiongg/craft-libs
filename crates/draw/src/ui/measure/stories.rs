//! Stories of the Ruler tool, its bar and the distances between boxes (task ST-TO2; task RG2
//! registers them). A fake picture stands for the layers, and the tool, the bar and the guides run
//! on it as an app would run them. Sample words and the status lines are story words; the case
//! names are choice names.
//!
//! - `primitives/ruler-tool/line`: the Ruler in a `ToolHost` over a fake picture: draw a line, grab
//!   an end, ⇧ for 45° steps, a tiny line clears on release, the keyboard parts.
//! - `primitives/ruler-tool/protractor`: a line with its protractor arm; ⌥ from an end starts
//!   another, and the status line shows the angle between the arms.
//! - `patterns/ruler-bar/readouts`: the bar with and without a line or a protractor, and Use
//!   Measurement Scale at a fake factor.
//! - `primitives/box-distances/cases`: the distances of two boxes apart, overlapping, diagonal and
//!   inside the canvas, drawn by `SmartGuides`.

use egui::{Painter, Shape, Stroke, Ui};
use kurbo::{Point, Rect, Size, Vec2};

use super::{RulerBar, RulerBarLabels, RulerBarOutput, RulerInput, RulerLine, RulerTool, RulerToolLabels, box_distances};
use craft_ui::canvas::{CanvasView, RulerUnits, SmartGuides, ToolHost};
use craft_ui::controls::Button;
use craft_ui::edit::EditEvent;
use craft_ui::localized::{Labels as _, LocaleNumbers};
use craft_ui::patterns::OptionsBar;
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, Story, StoryArgs, StoryChoice, StoryLabels, Surface};
use craft_ui::testing::canvas_story::{DOC, ViewArgs, document};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _, UiTokens};
use craft_ui::typed_story;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["RulerTool", "RulerBar", "box_distances"];

craft_ui::words! {
    /// The status lines of the Ruler stories, English by default.
    #[non_exhaustive]
    pub(crate) struct MeasureStoryLabels {
        no_line: "measureStory.noLine" = "No line";
        one_line: "measureStory.oneLine" = "L {l1} · A {angle}°";
        two_lines: "measureStory.twoLines" = "L1 {l1} · L2 {l2} · A {angle}°";
        begin: "measureStory.begin" = "Begin";
        update: "measureStory.update" = "Update";
        stored: "measureStory.stored" = "Stored";
        cleared: "measureStory.cleared" = "Cleared";
        cancelled: "measureStory.cancelled" = "Cancelled";
        straighten_layer: "measureStory.straightenLayer" = "Straighten Layer";
        clear: "measureStory.clear" = "Clear";
        distances: "measureStory.distances" = "{lines} lines · {distances} distances";
    }
}

/// What the last event of a story was.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Note {
    Begin,
    Update,
    Stored,
    Cleared,
    Cancelled,
    StraightenLayer,
    Clear,
}

impl Note {
    fn text(self, labels: &MeasureStoryLabels) -> &str {
        match self {
            Note::Begin => &labels.begin,
            Note::Update => &labels.update,
            Note::Stored => &labels.stored,
            Note::Cleared => &labels.cleared,
            Note::Cancelled => &labels.cancelled,
            Note::StraightenLayer => &labels.straighten_layer,
            Note::Clear => &labels.clear,
        }
    }
}

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "primitives/ruler-tool/line",
            Level::Primitives,
            "RulerTool",
            "Line",
            LineArgs { view: ViewArgs::default(), with_line: true, width: 220.0, height: 140.0 },
            show_line
        )
        .on(Surface::Canvas),
        typed_story!(
            "primitives/ruler-tool/protractor",
            Level::Primitives,
            "RulerTool",
            "Protractor",
            ProtractorArgs { view: ViewArgs::default(), arm: 140.0, angle: 60.0, width: 220.0, height: 140.0 },
            show_protractor
        )
        .on(Surface::Canvas),
        typed_story!(
            "patterns/ruler-bar/readouts",
            Level::Patterns,
            "RulerBar",
            "Readouts",
            BarArgs { shown: ShownArg::Line, use_scale: false, factor: 0.25, length: 120.0, angle: 30.0 },
            show_bar
        ),
        typed_story!(
            "primitives/box-distances/cases",
            Level::Primitives,
            "box_distances",
            "Cases",
            DistanceArgs { view: ViewArgs::default(), case: CaseArg::Apart, gap: 60.0, width: 100.0, height: 70.0 },
            show_distances
        )
        .on(Surface::Canvas),
    ]
}

// --- shared parts ---

/// Remembers the story's state under a name of this module's id space.
fn state_key(ui: &Ui, name: &str) -> egui::Id {
    ui.make_persistent_id(("craft_draw::measure::stories", name))
}

/// A muted status line.
fn status(ui: &mut Ui, text: String) {
    ui.add(Text::new(text).content(Content::Muted));
}

/// Paints `rect` (document units) through `view`: a fill and an outline in the accent.
fn paint_rect(painter: &Painter, view: &CanvasView, tokens: &UiTokens, rect: Rect, strength: f32) {
    let accent = tokens.color.accent.solid.to_egui();
    let stroke = Stroke::new(tokens.component.line_guide.width, accent);
    painter.add(Shape::convex_polygon(view.quad(rect).to_vec(), accent.gamma_multiply(strength), stroke));
}

/// The fake picture, centred on the story document.
fn picture(width: f64, height: f64) -> Rect {
    Rect::from_center_size(DOC.center(), Size::new(width, height))
}

// --- ruler tool ---

#[derive(StoryArgs, Debug, PartialEq)]
struct LineArgs {
    #[arg(flatten)]
    view: ViewArgs,
    /// Starts with a line across the picture; off starts with none.
    #[arg(name = "with line")]
    with_line: bool,
    /// The picture's size in document units.
    #[arg(range = 40.0..=600.0)]
    width: f64,
    #[arg(range = 40.0..=380.0)]
    height: f64,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct ProtractorArgs {
    #[arg(flatten)]
    view: ViewArgs,
    /// The length of both arms in document units.
    #[arg(range = 20.0..=300.0)]
    arm: f64,
    /// The angle between the arms in degrees.
    #[arg(range = 5.0..=175.0)]
    angle: f64,
    /// The picture's size in document units.
    #[arg(range = 40.0..=600.0)]
    width: f64,
    #[arg(range = 40.0..=380.0)]
    height: f64,
}

/// What a Ruler story keeps: the tool, the line as an app stores it, the line before the running
/// gesture, the last event's name, and the args the line was made from.
#[derive(Clone, Debug)]
struct RulerKept {
    tool: RulerTool,
    line: Option<RulerLine>,
    saved: Option<RulerLine>,
    note: Option<Note>,
    seed: Vec<f64>,
}

impl RulerKept {
    fn new(line: Option<RulerLine>, seed: Vec<f64>) -> Self {
        Self { tool: RulerTool::new(), line, saved: None, note: None, seed }
    }

    /// Stores what the tool reported, as the app does: the line is document state outside the
    /// undo history.
    fn apply(&mut self, event: &EditEvent<Option<RulerLine>>) {
        match event {
            EditEvent::Begin { value, .. } => {
                self.saved = self.line;
                if let Some(value) = value {
                    self.line = *value;
                }
                self.note = Some(Note::Begin);
            }
            EditEvent::Update { value, .. } => {
                self.line = *value;
                self.note = Some(Note::Update);
            }
            EditEvent::Commit { value, .. } | EditEvent::Set { value, .. } => {
                self.line = *value;
                self.note = Some(if value.is_some() { Note::Stored } else { Note::Cleared });
            }
            EditEvent::Cancel { .. } => {
                self.line = self.saved;
                self.note = Some(Note::Cancelled);
            }
            _ => {}
        }
    }
}

/// The Ruler over `size`, with the status line below. `initial` is the line a fresh story (or
/// Reset) starts with; a change of `seed` starts over.
fn show_ruler(ui: &mut Ui, view: ViewArgs, name: &str, size: Size, initial: Option<RulerLine>, seed: Vec<f64>) {
    let words = StoryLabels::of(ui);
    let own = MeasureStoryLabels::of(ui);
    let canvas = document(ui, view, &words.document);
    let ctx = ui.ctx().clone();
    let key = state_key(ui, name);
    let mut kept: RulerKept =
        ctx.data(|d| d.get_temp(key)).filter(|kept: &RulerKept| kept.seed == seed).unwrap_or_else(|| RulerKept::new(initial, seed.clone()));

    let tokens = Tokens::of(ui);
    paint_rect(&ui.painter_at(canvas.view.rect), &canvas.view, &tokens, picture(size.width, size.height), 0.25);

    let numbers = LocaleNumbers::of(ui).as_ref().clone();
    let pixels = RulerUnits::pixels(numbers);
    let labels = RulerToolLabels::of(ui);
    kept.tool.labels(labels);
    let mut pass = kept.tool.pass(RulerInput::new(kept.line).units(&pixels, &pixels));
    let _ = ToolHost::new(canvas.id).document(DOC.size()).show(ui, &canvas.view, &canvas.response, &mut pass);
    let events = kept.tool.take_events();
    for event in &events {
        kept.apply(event);
    }

    ui.horizontal(|ui| {
        if ui.add(Button::new(words.reset.as_str())).clicked() {
            kept = RulerKept::new(initial, seed.clone());
        }
        let numbers = LocaleNumbers::of(ui);
        let text = match kept.line {
            None => own.no_line.clone(),
            Some(line) => {
                let r = line.reading();
                let angle = numbers.format(r.angle, 1..=1);
                match r.l2 {
                    Some(l2) => own
                        .two_lines
                        .replace("{l1}", &numbers.format(r.l1, 1..=1))
                        .replace("{l2}", &numbers.format(l2, 1..=1))
                        .replace("{angle}", &angle),
                    None => own.one_line.replace("{l1}", &numbers.format(r.l1, 1..=1)).replace("{angle}", &angle),
                }
            }
        };
        status(ui, kept.note.map_or_else(|| text.clone(), |note| format!("{text} · {}", note.text(&own))));
    });
    ctx.data_mut(|d| d.insert_temp(key, kept));
}

fn show_line(ui: &mut Ui, args: &mut LineArgs) {
    let size = Size::new(args.width, args.height);
    let frame = picture(size.width, size.height);
    let initial = args.with_line.then(|| RulerLine::new(Point::new(frame.x0, frame.y0), Point::new(frame.x1, frame.y1)));
    show_ruler(ui, args.view, "line", size, initial, vec![args.width, args.height, f64::from(u8::from(args.with_line))]);
}

fn show_protractor(ui: &mut Ui, args: &mut ProtractorArgs) {
    let size = Size::new(args.width, args.height);
    let frame = picture(size.width, size.height);
    let start = Point::new(frame.center().x - args.arm / 2.0, frame.center().y);
    let turn = args.angle.to_radians();
    let initial = RulerLine { start, end: start + Vec2::new(args.arm, 0.0), protractor: Some(start + Vec2::new(turn.cos(), -turn.sin()) * args.arm) };
    show_ruler(ui, args.view, "protractor", size, Some(initial), vec![args.width, args.height, args.arm, args.angle]);
}

// --- bar ---

/// What the bar shows.
#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum ShownArg {
    #[choice(name = "No line")]
    NoLine,
    Line,
    Protractor,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct BarArgs {
    shown: ShownArg,
    /// Starts with Use Measurement Scale checked.
    #[arg(name = "use scale")]
    use_scale: bool,
    /// The fake scale's factor: document units to the scale's units.
    #[arg(range = 0.01..=10.0)]
    factor: f64,
    /// The first arm's length in document units.
    #[arg(range = 1.0..=600.0)]
    length: f64,
    /// The first arm's angle in degrees, counter-clockwise.
    #[arg(range = -180.0..=180.0)]
    angle: f64,
}

/// What the bar story keeps: the scale check box, the last button and the args it began from.
#[derive(Clone, Debug)]
struct BarKept {
    use_scale: bool,
    note: Option<Note>,
    seed: (bool, ShownArg),
}

/// The bar in an options bar over a line made from the args. Straighten Layer and Clear report
/// in the status line; the line is the args' alone.
fn show_bar(ui: &mut Ui, args: &mut BarArgs) {
    let words = StoryLabels::of(ui);
    let own = MeasureStoryLabels::of(ui);
    let ctx = ui.ctx().clone();
    let key = state_key(ui, "bar");
    let seed = (args.use_scale, args.shown);
    let mut kept: BarKept =
        ctx.data(|d| d.get_temp(key)).filter(|kept: &BarKept| kept.seed == seed).unwrap_or(BarKept { use_scale: args.use_scale, note: None, seed });

    let turn = args.angle.to_radians();
    let start = Point::new(100.0, 100.0);
    let end = start + Vec2::new(turn.cos(), -turn.sin()) * args.length;
    let line = match args.shown {
        ShownArg::NoLine => None,
        ShownArg::Line => Some(RulerLine::new(start, end)),
        ShownArg::Protractor => Some(RulerLine { start, end, protractor: Some(start + Vec2::new(args.length, 0.0)) }),
    };
    let labels = RulerBarLabels::of(ui);
    let mut out = RulerBarOutput::default();
    OptionsBar::new("ruler-bar").label(RulerToolLabels::of(ui).group).show(ui, |bar| {
        RulerBar::new(line, &mut kept.use_scale).scale(args.factor).labels(labels).output(&mut out).add_to(bar);
    });
    if out.straighten_layer {
        kept.note = Some(Note::StraightenLayer);
    }
    if out.clear {
        kept.note = Some(Note::Clear);
    }
    ui.horizontal(|ui| {
        if ui.add(Button::new(words.reset.as_str())).clicked() {
            kept.use_scale = args.use_scale;
            kept.note = None;
        }
        if let Some(note) = kept.note {
            status(ui, note.text(&own).to_owned());
        }
    });
    ctx.data_mut(|d| d.insert_temp(key, kept));
}

// --- distances ---

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum CaseArg {
    Apart,
    Overlapping,
    Diagonal,
    #[choice(name = "Inside the canvas")]
    Inside,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct DistanceArgs {
    #[arg(flatten)]
    view: ViewArgs,
    case: CaseArg,
    /// The space between the boxes in document units.
    #[arg(range = 5.0..=200.0)]
    gap: f64,
    /// The first box's size in document units.
    #[arg(range = 20.0..=250.0)]
    width: f64,
    #[arg(range = 20.0..=180.0)]
    height: f64,
}

/// The two boxes of a case: `a` the selection's bounds, `b` the hovered layer or the canvas.
fn boxes(args: &DistanceArgs) -> (Rect, Rect) {
    let size = Size::new(args.width, args.height);
    let c = DOC.center();
    match args.case {
        CaseArg::Apart => {
            let a = Rect::from_origin_size(Point::new(c.x - args.width - args.gap / 2.0, c.y - args.height / 2.0), size);
            (a, a + Vec2::new(args.width + args.gap, 0.0))
        }
        CaseArg::Overlapping => {
            let a = Rect::from_origin_size(Point::new(c.x - args.width * 0.75, c.y - args.height * 0.75), size);
            (a, a + Vec2::new(args.width / 2.0, args.height / 2.0))
        }
        CaseArg::Diagonal => {
            let a = Rect::from_origin_size(Point::new(c.x - args.width - args.gap / 2.0, c.y - args.height - args.gap / 2.0), size);
            (a, a + Vec2::new(args.width + args.gap, args.height + args.gap))
        }
        CaseArg::Inside => (Rect::from_center_size(c, size), DOC),
    }
}

/// The boxes with the lines and distances `box_distances` gives, drawn by `SmartGuides` as the
/// Move tool does while ⌘ is held.
fn show_distances(ui: &mut Ui, args: &mut DistanceArgs) {
    let words = StoryLabels::of(ui);
    let own = MeasureStoryLabels::of(ui);
    let canvas = document(ui, args.view, &words.document);
    let tokens = Tokens::of(ui);
    let (a, b) = boxes(args);
    let painter = ui.painter_at(canvas.view.rect);
    if args.case != CaseArg::Inside {
        paint_rect(&painter, &canvas.view, &tokens, b, 0.1);
    }
    paint_rect(&painter, &canvas.view, &tokens, a, 0.35);
    let pixels = RulerUnits::pixels(LocaleNumbers::of(ui).as_ref().clone());
    let (lines, measures) = box_distances(a, b, |len, _| pixels.format_value(len, 0.0));
    SmartGuides::new(&lines, &measures).bounds(a).show(ui, &canvas.view);
    status(ui, own.distances.replace("{lines}", &lines.len().to_string()).replace("{distances}", &measures.len().to_string()));
}
