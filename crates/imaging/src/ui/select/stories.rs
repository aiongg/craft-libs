//! Stories of the selection tools (task ST-SE1): not registered. Registering them (task RG2) adds
//! `select::stories` to the crate's `register!` list and [`COMPONENTS`] to the registry's
//! component list, and regenerates the registry dump.
//!
//! Each story runs one tool inside a `ToolHost` over [`MockImage`], as an app would: the tool is
//! wrapped in a [`SelectToolPass`] with a [`SelectionProbe`] over the story's fake selection, and
//! the events it reports are drained after `ToolHost::show` and applied to that selection. The
//! fake selection (a list of operations over shapes, in the hard coverage a marquee or outline
//! gives) starts as a rectangle round the picture's house, so a press inside it drags, an Add or
//! Subtract shows, and a replaced selection is hidden while an outline is open. Its ants are a
//! [`SelectionOverlay`]. The Magnetic Lasso's [`EdgeTracer`] reads the picture: it snaps to the
//! house's outline where the colour across it differs by the Contrast option.
//!
//! The Play button injects the same input a hand would (`ToolHost::inject`), so each story runs
//! without a pointer. Button captions are story words; the status line under the picture is Debug text.
//!
//! | Story id | Shows |
//! |---|---|
//! | `patterns/select/marquee` | The four kinds; Normal, Fixed Ratio and Fixed Size; ⇧ and ⌥ after the press; a drag inside the selection; a click that deselects |
//! | `patterns/select/lasso` | Freehand and ⌥ straight segments; Backspace retracts; the replaced selection hidden while drawing |
//! | `patterns/select/polygon-lasso` | Click by click; ⇧ at 45°; closing on the first point; Backspace retracts |
//! | `patterns/select/magnetic-lasso` | The tracer on the house's edges; Width, Contrast and Frequency from the options; fastening points, the closing ring |

use craft_ui::canvas::kurbo::{Point, Rect, Size, Vec2};
use craft_ui::canvas::{CanvasId, CanvasTool, Constraint, RasterSource as _, SelectionOperation, ToolHost, ToolInput};
use craft_ui::controls::Button;
use craft_ui::edit::EditEvent;
use craft_ui::localized::Labels as _;
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, Story, StoryArgs, StoryChoice, StoryLabels, Surface};
use craft_ui::typed_story;
use egui::{Modifiers, Sense, Ui};

use super::{
    EdgeSearch, EdgeTracer, LassoTool, MagneticLassoTool, MarqueeKind, MarqueeTool, PolygonLassoTool, SelectEvent, SelectPhase, SelectToolPass,
    SelectionProbe, SelectionShape,
};
use crate::trace::{MaskRect, MaskSource};
use crate::ui::mock_image::MockImage;
use crate::ui::selection::{SelectionOverlay, SelectionOverlayLabels};

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["MarqueeTool", "LassoTool", "PolygonLassoTool", "MagneticLassoTool"];

craft_ui::words! {
    /// The captions of the selection tool stories, English by default.
    #[non_exhaustive]
    pub(crate) struct SelectToolStoryLabels {
        reset: "selectToolStory.reset" = "Reset selection";
    }
}

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "patterns/select/marquee",
            Level::Patterns,
            "MarqueeTool",
            "Marquee",
            MarqueeArgs {
                width: 1600.0,
                kind: KindArg::Rectangular,
                style: StyleArg::Normal,
                ratio_width: 4.0,
                ratio_height: 3.0,
                size_width: 400.0,
                size_height: 300.0,
                operation: OperationArg::New,
                play: MarqueePlay::Draw,
                square: false,
                centre: false,
                seconds: 2.0,
            },
            show_marquee
        )
        .on(Surface::Canvas),
        typed_story!(
            "patterns/select/lasso",
            Level::Patterns,
            "LassoTool",
            "Lasso",
            LassoArgs { width: 1600.0, operation: OperationArg::New, straight: false, seconds: 3.0 },
            show_lasso
        )
        .on(Surface::Canvas),
        typed_story!(
            "patterns/select/polygon-lasso",
            Level::Patterns,
            "PolygonLassoTool",
            "Polygonal Lasso",
            PolygonArgs { width: 1600.0, operation: OperationArg::New, constrain: false, seconds: 3.0 },
            show_polygon
        )
        .on(Surface::Canvas),
        typed_story!(
            "patterns/select/magnetic-lasso",
            Level::Patterns,
            "MagneticLassoTool",
            "Magnetic Lasso",
            MagneticArgs { width: 1600.0, edge_width: 10.0, contrast: 10.0, frequency: 57.0, operation: OperationArg::New, seconds: 4.0 },
            show_magnetic
        )
        .on(Surface::Canvas),
    ]
}

// ---- args ----

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum KindArg {
    Rectangular,
    Elliptical,
    #[choice(name = "Single Row")]
    SingleRow,
    #[choice(name = "Single Column")]
    SingleColumn,
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum StyleArg {
    Normal,
    #[choice(name = "Fixed Ratio")]
    FixedRatio,
    #[choice(name = "Fixed Size")]
    FixedSize,
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum OperationArg {
    New,
    Add,
    Subtract,
    Intersect,
}

impl OperationArg {
    fn operation(self) -> SelectionOperation {
        match self {
            OperationArg::New => SelectionOperation::New,
            OperationArg::Add => SelectionOperation::Add,
            OperationArg::Subtract => SelectionOperation::Subtract,
            OperationArg::Intersect => SelectionOperation::Intersect,
        }
    }
}

/// What the Marquee's Play button does.
#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum MarqueePlay {
    Draw,
    #[choice(name = "Drag Inside")]
    DragInside,
    Click,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct MarqueeArgs {
    /// The picture's width in document pixels.
    #[arg(range = 200.0..=3200.0)]
    width: f64,
    kind: KindArg,
    style: StyleArg,
    /// Fixed Ratio's width.
    #[arg(range = 1.0..=16.0)]
    ratio_width: f64,
    /// Fixed Ratio's height.
    #[arg(range = 1.0..=16.0)]
    ratio_height: f64,
    /// Fixed Size's width in document pixels.
    #[arg(range = 1.0..=3200.0)]
    size_width: f64,
    /// Fixed Size's height in document pixels.
    #[arg(range = 1.0..=3200.0)]
    size_height: f64,
    /// The options bar's mode.
    operation: OperationArg,
    /// What Play injects.
    play: MarqueePlay,
    /// Play holds ⇧ after the press.
    square: bool,
    /// Play holds ⌥ after the press.
    centre: bool,
    /// How long Play takes, in seconds.
    #[arg(range = 0.5..=10.0)]
    seconds: f64,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct LassoArgs {
    /// The picture's width in document pixels.
    #[arg(range = 200.0..=3200.0)]
    width: f64,
    /// The options bar's mode.
    operation: OperationArg,
    /// Play puts ⌥ straight segments after the freehand start.
    straight: bool,
    /// How long Play takes, in seconds.
    #[arg(range = 0.5..=10.0)]
    seconds: f64,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct PolygonArgs {
    /// The picture's width in document pixels.
    #[arg(range = 200.0..=3200.0)]
    width: f64,
    /// The options bar's mode.
    operation: OperationArg,
    /// Play holds ⇧ for each click.
    constrain: bool,
    /// How long Play takes, in seconds.
    #[arg(range = 0.5..=10.0)]
    seconds: f64,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct MagneticArgs {
    /// The picture's width in document pixels.
    #[arg(range = 200.0..=3200.0)]
    width: f64,
    /// The option Width, in document pixels.
    #[arg(range = 1.0..=256.0)]
    edge_width: f64,
    /// The option Contrast, in percent.
    #[arg(range = 1.0..=100.0)]
    contrast: f64,
    /// The option Frequency.
    #[arg(range = 0.0..=100.0)]
    frequency: f64,
    /// The options bar's mode.
    operation: OperationArg,
    /// How long Play takes, in seconds.
    #[arg(range = 0.5..=10.0)]
    seconds: f64,
}

// ---- the fake selection ----

/// The fake document's selection: shapes combined in order, then moved by `offset`. It is the
/// tools' [`SelectionProbe`] and the ants' [`MaskSource`].
#[derive(Clone, Debug)]
struct FakeSelection {
    size: [u32; 2],
    parts: Vec<(SelectionOperation, SelectionShape)>,
    offset: [i32; 2],
    revision: u64,
}

impl FakeSelection {
    /// A rectangle round the picture's house.
    fn house(image: &MockImage) -> Self {
        let b = image.subject_bounds();
        let rect = Rect::new(f64::from(b.x0), f64::from(b.y0), f64::from(b.x1), f64::from(b.y1));
        Self { size: image.size(), parts: vec![(SelectionOperation::New, SelectionShape::Rectangle(rect))], offset: [0, 0], revision: 1 }
    }

    fn apply(&mut self, operation: SelectionOperation, shape: SelectionShape) {
        if operation == SelectionOperation::New {
            self.parts.clear();
            self.offset = [0, 0];
        }
        self.parts.push((operation, shape));
        self.revision += 1;
    }

    fn clear(&mut self) {
        self.parts.clear();
        self.offset = [0, 0];
        self.revision += 1;
    }

    fn shift(&mut self, delta: [i32; 2]) {
        self.offset = [self.offset[0] + delta[0], self.offset[1] + delta[1]];
        self.revision += 1;
    }

    /// The coverage at document point `(x, y)`.
    fn coverage(&self, x: f64, y: f64) -> f32 {
        let p = Point::new(x - f64::from(self.offset[0]), y - f64::from(self.offset[1]));
        self.parts.iter().fold(0.0, |acc, (operation, shape)| {
            let c = shape_coverage(shape, p);
            match operation {
                SelectionOperation::New => c,
                SelectionOperation::Add => acc.max(c),
                SelectionOperation::Subtract => acc * (1.0 - c),
                SelectionOperation::Intersect => acc.min(c),
            }
        })
    }
}

fn shape_coverage(shape: &SelectionShape, p: Point) -> f32 {
    let inside = match shape {
        SelectionShape::Rectangle(r) => r.x0 <= p.x && p.x < r.x1 && r.y0 <= p.y && p.y < r.y1,
        SelectionShape::Ellipse(r) => {
            let (c, rx, ry) = (r.center(), r.width() / 2.0, r.height() / 2.0);
            rx > 0.0 && ry > 0.0 && ((p.x - c.x) / rx).powi(2) + ((p.y - c.y) / ry).powi(2) <= 1.0
        }
        SelectionShape::Polygon(points) => {
            // Even-odd: count the edges a ray to the right crosses.
            let mut inside = false;
            for (i, a) in points.iter().enumerate() {
                let b = points[(i + 1) % points.len()];
                if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x) {
                    inside = !inside;
                }
            }
            inside
        }
    };
    f32::from(inside)
}

fn shape_bounds(shape: &SelectionShape) -> Rect {
    match shape {
        SelectionShape::Rectangle(r) | SelectionShape::Ellipse(r) => *r,
        SelectionShape::Polygon(points) => {
            let first = points.first().copied().unwrap_or(Point::ZERO);
            points.iter().fold(Rect::from_points(first, first), |bounds, &p| bounds.union_pt(p))
        }
    }
}

impl MaskSource for FakeSelection {
    fn size(&self) -> [u32; 2] {
        self.size
    }

    fn revision(&self) -> u64 {
        self.revision
    }

    fn content_bounds(&self) -> MaskRect {
        let Some(bounds) = self.parts.iter().map(|(_, shape)| shape_bounds(shape)).reduce(|a, b| a.union(b)) else {
            return MaskRect::new(0, 0, 0, 0);
        };
        let [w, h] = self.size.map(|n| i32::try_from(n).unwrap_or(i32::MAX));
        let [ox, oy] = self.offset;
        let clamp = |v: f64, shift: i32, limit: i32| (v as i32).saturating_add(shift).clamp(0, limit);
        MaskRect::new(
            clamp(bounds.x0.floor(), ox, w),
            clamp(bounds.y0.floor(), oy, h),
            clamp(bounds.x1.ceil(), ox, w),
            clamp(bounds.y1.ceil(), oy, h),
        )
    }

    fn read_row(&self, y: i32, x0: i32, step: u32, out: &mut [f32]) {
        let y = f64::from(y) + 0.5;
        for (i, value) in out.iter_mut().enumerate() {
            *value = self.coverage(f64::from(x0) + 0.5 + i as f64 * f64::from(step), y);
        }
    }
}

impl SelectionProbe for FakeSelection {
    fn document(&self) -> u64 {
        1
    }

    fn size(&self) -> [u32; 2] {
        self.size
    }

    fn has_selection(&self) -> bool {
        !self.parts.is_empty()
    }

    fn inside_selection(&self, at: Point) -> bool {
        self.coverage(at.x, at.y) > 0.0
    }
}

// ---- the Magnetic Lasso's edge finder ----

/// An [`EdgeTracer`] over the picture: the edges are the house's outline, an edge counts where the
/// brightness differs across it by at least the search's contrast.
#[derive(Clone, Copy, Debug)]
struct PictureEdges {
    image: MockImage,
}

impl PictureEdges {
    /// How far either side of the outline the brightness is compared, as a fraction of the
    /// picture's width.
    const PROBE: f64 = 0.002;

    fn luma(&self, p: Point) -> f64 {
        let [r, g, b, _] = self.image.pixel(p.x, p.y);
        (0.299 * f64::from(r) + 0.587 * f64::from(g) + 0.114 * f64::from(b)) / 255.0
    }

    /// The outline's unit normal at `p`, pointing out of the house.
    fn normal(&self, p: Point) -> Vec2 {
        let h = 1.0;
        let d = |dx: f64, dy: f64| self.image.subject_distance(p.x + dx, p.y + dy);
        let g = Vec2::new(d(h, 0.0) - d(-h, 0.0), d(0.0, h) - d(0.0, -h));
        if g.hypot() > 0.0 { g / g.hypot() } else { Vec2::new(0.0, -1.0) }
    }

    /// The point of the outline nearest `p`, `None` beyond the search's width or where the edge is
    /// weaker than its contrast.
    fn edge_near(&self, p: Point, search: EdgeSearch) -> Option<Point> {
        let mut q = p;
        for _ in 0..4 {
            let d = self.image.subject_distance(q.x, q.y);
            q -= self.normal(q) * d;
        }
        let reach = Self::PROBE * f64::from(self.image.size()[0]);
        let n = self.normal(q);
        let contrast = (self.luma(q + n * reach) - self.luma(q - n * reach)).abs();
        ((q - p).hypot() <= search.width && contrast >= search.contrast).then_some(q)
    }
}

impl EdgeTracer for PictureEdges {
    fn snap(&mut self, at: Point, search: EdgeSearch) -> Option<Point> {
        self.edge_near(at, search)
    }

    fn trace(&mut self, from: Point, to: Point, _guide: &[Point], search: EdgeSearch) -> Vec<Point> {
        // Walk the straight line a pixel at a time and pull each sample onto the edge.
        let steps = from.distance(to).ceil().max(1.0) as usize;
        let mut path = vec![from];
        for i in 1..steps {
            let sample = from.lerp(to, i as f64 / steps as f64);
            let point = self.edge_near(sample, search).unwrap_or(sample);
            if path.last().is_none_or(|last| last.distance(point) >= 1.0) {
                path.push(point);
            }
        }
        path.push(to);
        path
    }
}

// ---- scripts ----

#[derive(Clone, Copy, Debug)]
enum Phase {
    Down,
    Move,
    Up,
}

/// One injected pointer sample.
#[derive(Clone, Copy, Debug)]
struct Step {
    phase: Phase,
    at: Point,
    modifiers: Modifiers,
}

impl Step {
    fn new(phase: Phase, at: Point, modifiers: Modifiers) -> Self {
        Self { phase, at, modifiers }
    }

    fn input(self) -> ToolInput {
        match self.phase {
            Phase::Down => ToolInput::Down { doc: self.at, pressure: 1.0, tilt: [0.0, 0.0], rotation: 0.0, modifiers: self.modifiers },
            Phase::Move => ToolInput::Move { doc: self.at, pressure: 1.0, tilt: [0.0, 0.0], rotation: 0.0, modifiers: self.modifiers },
            Phase::Up => ToolInput::Up { doc: self.at, modifiers: self.modifiers },
        }
    }
}

/// A click: press and release at `at`.
fn click(at: Point, modifiers: Modifiers) -> [Step; 2] {
    [Step::new(Phase::Down, at, modifiers), Step::new(Phase::Up, at, modifiers)]
}

/// Moves from `from` (exclusive) to `to` (inclusive) in `n` equal parts.
fn moves(from: Point, to: Point, n: usize, modifiers: Modifiers) -> impl Iterator<Item = Step> {
    (1..=n).map(move |i| Step::new(Phase::Move, from.lerp(to, i as f64 / n as f64), modifiers))
}

/// A running script: the steps, when it began, how long it takes and how many went out.
#[derive(Clone, Debug)]
struct Script {
    steps: Vec<Step>,
    start: f64,
    seconds: f64,
    sent: usize,
}

/// The points round the house's outline, `n` of them from the top, each `grow` document pixels
/// outside it.
fn outline_points(image: &MockImage, n: usize, grow: f64) -> Vec<Point> {
    let b = image.subject_bounds();
    let centre = Point::new(f64::from(b.x0 + b.x1) / 2.0, f64::from(b.y0 + b.y1) / 2.0);
    let far = f64::from(b.x1 - b.x0).hypot(f64::from(b.y1 - b.y0));
    (0..n)
        .map(|k| {
            let angle = k as f64 / n as f64 * std::f64::consts::TAU - std::f64::consts::FRAC_PI_2;
            let direction = Vec2::new(angle.cos(), angle.sin());
            let (mut inside, mut outside) = (0.0, far);
            for _ in 0..BISECTIONS {
                let mid = (inside + outside) / 2.0;
                let p = centre + direction * mid;
                if image.subject_distance(p.x, p.y) < 0.0 {
                    inside = mid;
                } else {
                    outside = mid;
                }
            }
            centre + direction * (outside + grow)
        })
        .collect()
}

// ---- the session ----

/// What a story keeps between passes.
#[derive(Clone, Debug)]
struct Session<T> {
    tool: T,
    selection: FakeSelection,
    script: Option<Script>,
    /// The offset of a selection drag in progress.
    dragging: Option<[i32; 2]>,
    /// The last event, for the line under the picture.
    last: Option<SelectEvent>,
}

impl<T> Session<T> {
    fn new(tool: T, image: &MockImage) -> Self {
        Self { tool, selection: FakeSelection::house(image), script: None, dragging: None, last: None }
    }
}

/// What the stories need from a tool value.
trait StoryTool: Clone + Send + Sync + 'static {
    fn set_operation(&mut self, bar: SelectionOperation);
    fn take_events(&mut self) -> Vec<SelectEvent>;
    fn phase(&self) -> SelectPhase;
    /// An outline being drawn replaces the selection.
    fn replaces_selection(&self) -> bool {
        false
    }
}

macro_rules! story_tool {
    ($tool:ty $(, $replaces:ident)?) => {
        impl StoryTool for $tool {
            fn set_operation(&mut self, bar: SelectionOperation) {
                <$tool>::set_operation(self, bar);
            }

            fn take_events(&mut self) -> Vec<SelectEvent> {
                <$tool>::take_events(self)
            }

            fn phase(&self) -> SelectPhase {
                <$tool>::phase(self)
            }
            $(
                fn $replaces(&self) -> bool {
                    <$tool>::$replaces(self)
                }
            )?
        }
    };
}

story_tool!(MarqueeTool);
story_tool!(LassoTool);
story_tool!(PolygonLassoTool, replaces_selection);
story_tool!(MagneticLassoTool<PictureEdges>, replaces_selection);

/// The picture size for a width in document pixels, in the default picture's proportions.
fn picture_size(width: f64) -> [u32; 2] {
    let [dw, dh] = MockImage::default().size();
    let width = width.round().max(1.0);
    [width as u32, (width * f64::from(dh) / f64::from(dw)).round().max(1.0) as u32]
}

/// The session kept under `salt`, started with `tool` when there is none or the picture's size
/// changed.
fn session<T: StoryTool>(ui: &Ui, salt: &'static str, image: &MockImage, tool: impl FnOnce() -> T) -> (egui::Id, Session<T>) {
    let id = ui.make_persistent_id(("select-story", salt));
    let kept = ui.data(|d| d.get_temp::<Session<T>>(id)).filter(|s| s.selection.size == image.size());
    (id, kept.unwrap_or_else(|| Session::new(tool(), image)))
}

/// The Play and reset buttons. Returns whether Play was pressed.
fn buttons<T>(ui: &mut Ui, session: &mut Session<T>, image: &MockImage) -> bool {
    let play = StoryLabels::of(ui).play;
    let reset = SelectToolStoryLabels::of(ui).reset.clone();
    let mut pressed = false;
    ui.horizontal(|ui| {
        pressed = ui.add(Button::new(play.as_str())).clicked() && session.script.is_none();
        if ui.add(Button::new(reset.as_str())).clicked() {
            session.selection = FakeSelection::house(image);
        }
    });
    pressed
}

/// Sends the steps of the running script that are due.
fn run_script(ui: &Ui, canvas: CanvasId, script: &mut Option<Script>) {
    let Some(run) = script else { return };
    let t = ((ui.input(|i| i.time) - run.start) / run.seconds.max(0.1)).clamp(0.0, 1.0);
    let due = ((t * run.steps.len() as f64) as usize + 1).min(run.steps.len());
    for step in &run.steps[run.sent..due] {
        ToolHost::inject(ui.ctx(), canvas, step.input());
    }
    run.sent = due;
    if due >= run.steps.len() {
        *script = None;
    }
    ui.ctx().request_repaint();
}

/// The picture, the selection's ants, the tool on a `ToolHost`, and the line under them that names
/// the tool's phase and the last event; then the events applied to the fake selection.
fn show_tool<T: StoryTool>(ui: &mut Ui, salt: &'static str, image: &MockImage, operation: SelectionOperation, session: &mut Session<T>)
where
    for<'a> SelectToolPass<'a, T>: CanvasTool,
{
    session.tool.set_operation(operation);
    let (response, view) = image.canvas(ui, ("select-story", salt, "picture"), ui.available_width(), None, None);
    let id = CanvasId::new(ui, ("select-story", salt, "canvas"));
    let canvas = id.interact(ui, response.rect, Sense::click_and_drag() | Sense::FOCUSABLE);
    run_script(ui, id, &mut session.script);

    SelectionOverlay::new(("select-story", salt, "ants"), &session.selection)
        .labels(SelectionOverlayLabels::of(ui))
        .visible(!session.tool.replaces_selection())
        .offset(session.dragging.unwrap_or([0, 0]))
        .show(ui, &response, &view);
    let [w, h] = image.size();
    let mut pass = SelectToolPass::new(&mut session.tool, &session.selection);
    let _ = ToolHost::new(id).document(Size::new(f64::from(w), f64::from(h))).show(ui, &view, &canvas, &mut pass);

    for event in session.tool.take_events() {
        match &event {
            SelectEvent::Commit { shape, operation } => session.selection.apply(*operation, shape.clone()),
            SelectEvent::Deselect => session.selection.clear(),
            SelectEvent::DragSelection(edit) => match edit {
                EditEvent::Update { value, .. } => session.dragging = Some(value.delta),
                EditEvent::Commit { value, changed, .. } => {
                    session.dragging = None;
                    if *changed {
                        session.selection.shift(value.delta);
                    }
                }
                EditEvent::Cancel { .. } => session.dragging = None,
                _ => {}
            },
        }
        session.last = Some(event);
        ui.ctx().request_repaint();
    }
    let line = format!("{:?} | {:?}", session.tool.phase(), session.last);
    ui.add(Text::new(line.as_str()).content(Content::Muted));
}

// ---- the stories ----

fn show_marquee(ui: &mut Ui, args: &mut MarqueeArgs) {
    let image = MockImage::new(picture_size(args.width));
    let kind = match args.kind {
        KindArg::Rectangular => MarqueeKind::Rectangle,
        KindArg::Elliptical => MarqueeKind::Ellipse,
        KindArg::SingleRow => MarqueeKind::SingleRow,
        KindArg::SingleColumn => MarqueeKind::SingleColumn,
    };
    let (id, mut s) = session(ui, "marquee", &image, || MarqueeTool::new(kind));
    s.tool.set_kind(kind);
    s.tool.set_constraint(match args.style {
        StyleArg::Normal => Constraint::Free,
        StyleArg::FixedRatio => Constraint::Ratio(args.ratio_width, args.ratio_height),
        StyleArg::FixedSize => Constraint::Size(args.size_width, args.size_height),
    });
    if buttons(ui, &mut s, &image) {
        let [w, h] = image.size().map(f64::from);
        let at = |x: f64, y: f64| Point::new(x * w, y * h);
        // ⇧ and ⌥ count when they come after the press.
        let held = Modifiers { shift: args.square, alt: args.centre, ..Modifiers::NONE };
        let mut steps = Vec::new();
        match args.play {
            MarqueePlay::Draw => {
                let (a, b) = (at(0.2, 0.25), at(0.8, 0.85));
                steps.push(Step::new(Phase::Down, a, Modifiers::NONE));
                steps.extend(moves(a, b, DRAG_STEPS, held));
                steps.push(Step::new(Phase::Up, b, held));
            }
            MarqueePlay::DragInside => {
                let (a, b) = (at(0.5, 0.5), at(0.65, 0.4));
                steps.push(Step::new(Phase::Down, a, Modifiers::NONE));
                steps.extend(moves(a, b, DRAG_STEPS, Modifiers::NONE));
                steps.push(Step::new(Phase::Up, b, Modifiers::NONE));
            }
            MarqueePlay::Click => steps.extend(click(at(0.15, 0.15), Modifiers::NONE)),
        }
        s.script = Some(Script { steps, start: ui.input(|i| i.time), seconds: args.seconds, sent: 0 });
    }
    show_tool(ui, "marquee", &image, args.operation.operation(), &mut s);
    ui.data_mut(|d| d.insert_temp(id, s));
}

fn show_lasso(ui: &mut Ui, args: &mut LassoArgs) {
    let image = MockImage::new(picture_size(args.width));
    let (id, mut s) = session(ui, "lasso", &image, LassoTool::new);
    if buttons(ui, &mut s, &image) {
        let points = outline_points(&image, FREEHAND_POINTS, f64::from(image.size()[0]) * GROW);
        let mut steps = vec![Step::new(Phase::Down, points[0], Modifiers::NONE)];
        if args.straight {
            // Freehand for the first quarter, then ⌥ latches: each click is a straight segment, and
            // letting ⌥ go with the button up closes.
            let alt = Modifiers::ALT;
            let quarter = points.len() / 4;
            for pair in points[..=quarter].windows(2) {
                steps.extend(moves(pair[0], pair[1], SEGMENT_STEPS, Modifiers::NONE));
            }
            steps.push(Step::new(Phase::Move, points[quarter], alt));
            steps.push(Step::new(Phase::Up, points[quarter], alt));
            for &p in points[quarter + 1..].iter().step_by(STRAIGHT_STRIDE) {
                steps.extend(click(p, alt));
            }
            steps.push(Step::new(Phase::Move, points[0], Modifiers::NONE));
        } else {
            for pair in points.windows(2) {
                steps.extend(moves(pair[0], pair[1], SEGMENT_STEPS, Modifiers::NONE));
            }
            steps.extend(moves(points[points.len() - 1], points[0], SEGMENT_STEPS, Modifiers::NONE));
            steps.push(Step::new(Phase::Up, points[0], Modifiers::NONE));
        }
        s.script = Some(Script { steps, start: ui.input(|i| i.time), seconds: args.seconds, sent: 0 });
    }
    show_tool(ui, "lasso", &image, args.operation.operation(), &mut s);
    ui.data_mut(|d| d.insert_temp(id, s));
}

fn show_polygon(ui: &mut Ui, args: &mut PolygonArgs) {
    let image = MockImage::new(picture_size(args.width));
    let (id, mut s) = session(ui, "polygon-lasso", &image, PolygonLassoTool::new);
    if buttons(ui, &mut s, &image) {
        let points = outline_points(&image, POLYGON_POINTS, f64::from(image.size()[0]) * GROW);
        let held = Modifiers { shift: args.constrain, ..Modifiers::NONE };
        let mut steps: Vec<Step> = points.iter().flat_map(|&p| click(p, held)).collect();
        // The last click lands on the first point and closes the outline.
        steps.extend(click(points[0], held));
        s.script = Some(Script { steps, start: ui.input(|i| i.time), seconds: args.seconds, sent: 0 });
    }
    show_tool(ui, "polygon-lasso", &image, args.operation.operation(), &mut s);
    ui.data_mut(|d| d.insert_temp(id, s));
}

fn show_magnetic(ui: &mut Ui, args: &mut MagneticArgs) {
    let image = MockImage::new(picture_size(args.width));
    let (id, mut s) = session(ui, "magnetic-lasso", &image, || MagneticLassoTool::new(PictureEdges { image }));
    let settings = s.tool.settings_mut();
    settings.width = args.edge_width as f32;
    settings.contrast = args.contrast as f32;
    settings.frequency = args.frequency as f32;
    if buttons(ui, &mut s, &image) {
        // The pointer runs beside the edge, within Width of it; the tool pulls the outline on.
        let points = outline_points(&image, MAGNETIC_POINTS, args.edge_width * OFF_EDGE);
        let mut steps = vec![Step::new(Phase::Down, points[0], Modifiers::NONE)];
        for pair in points.windows(2) {
            steps.extend(moves(pair[0], pair[1], SEGMENT_STEPS, Modifiers::NONE));
        }
        steps.push(Step::new(Phase::Up, points[points.len() - 1], Modifiers::NONE));
        // A click on the first point closes the outline along the edges.
        steps.extend(click(points[0], Modifiers::NONE));
        s.script = Some(Script { steps, start: ui.input(|i| i.time), seconds: args.seconds, sent: 0 });
    }
    show_tool(ui, "magnetic-lasso", &image, args.operation.operation(), &mut s);
    ui.data_mut(|d| d.insert_temp(id, s));
}

/// How far outside the house the scripted pointer runs, as a fraction of the picture's width.
const GROW: f64 = 0.004;
/// How many points the scripted freehand, polygon and magnetic outlines have.
const FREEHAND_POINTS: usize = 24;
const POLYGON_POINTS: usize = 8;
const MAGNETIC_POINTS: usize = 16;
/// The Lasso's ⌥ clicks take every this many of the outline's points.
const STRAIGHT_STRIDE: usize = 3;
/// The scripted Magnetic Lasso pointer's distance from the edge, as a share of the Width option.
const OFF_EDGE: f64 = 0.5;
/// The moves of a scripted drag, and of each segment between two outline points.
const DRAG_STEPS: usize = 16;
const SEGMENT_STEPS: usize = 3;
/// The halvings that find the house's edge along a ray.
const BISECTIONS: usize = 32;
