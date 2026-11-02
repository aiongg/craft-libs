//! Stories of the Connector tool and bar (task ST-SH2; task RG2 registers them). Registering them
//! adds `stories()` to craft-draw's stories, the components to `COMPONENTS` and
//! [`ConnectorStoryLabels`] to `WORDS`.
//!
//! Every story uses fake targets (three boxes, one with numbered sites); every string goes
//! through story words. The tool story runs the tool as an app does: it keeps the connectors,
//! applies each committed draw, reattach and reshape, and selects the connector it made.
//!
//! | Story id | Shows |
//! |---|---|
//! | `primitives/connector-tool/tool` | Straight, elbow and curved; attach points; reattach; reshape; the "+" buttons |
//! | `primitives/connector-tool/routes` | The route gallery: every side pair and boxes in the way |
//! | `patterns/connector-bar/bar` | Routing, caps, line style |

use egui::epaint::CubicBezierShape;
use egui::{Color32, Painter, Pos2, Rect as ScreenRect, Shape, Stroke, Ui};
use kurbo::{Point, Rect, Size, Vec2};

use super::{
    ConnectionSite, ConnectionTarget, ConnectorBar, ConnectorBarOutput, ConnectorEnd, ConnectorEndKind, ConnectorEvent, ConnectorItem, ConnectorLine,
    ConnectorTool, ConnectorToolLabels,
};
use crate::geometry::connector::{ConnectionSide, RouteEnd, Routing, route, route_along};
use crate::ui::stroke::{StrokeChoice, StrokeChoiceRow};
use craft_ui::canvas::{CanvasView, ToolHost};
use craft_ui::controls::{Button, MixedValue};
use craft_ui::edit::{EditEvent, describe};
use craft_ui::localized::Labels as _;
use craft_ui::patterns::OptionsBar;
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, Story, StoryArgs, StoryChoice, StoryLabels, Surface, lock, shared};
use craft_ui::testing::canvas_story::{DOC, ViewArgs, document};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _, UiTokens};
use craft_ui::typed_story;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["ConnectorTool", "ConnectorBar"];

craft_ui::words! {
    /// The captions of the connector stories, English by default.
    #[non_exhaustive]
    pub(crate) struct ConnectorStoryLabels {
        /// The kinds of tool event in the status line.
        draw: "connectorStory.draw" = "Draw";
        reattach_start: "connectorStory.reattachStart" = "Reattach start";
        reattach_end: "connectorStory.reattachEnd" = "Reattach end";
        reshape: "connectorStory.reshape" = "Reshape";
        /// A press on a "+" button: `{side}`.
        add_connected: "connectorStory.addConnected" = "Add beside: {side}";
        select_next: "connectorStory.selectNext" = "Select next connector";
        /// The status line's start: `{count}` connectors, `{event}` the last event.
        status: "connectorStory.status" = "{count} connectors · {event}";
        arrowhead_none: "connectorStory.arrowheadNone" = "None";
        arrowhead_arrow: "connectorStory.arrowheadArrow" = "Arrow";
        arrowhead_dot: "connectorStory.arrowheadDot" = "Dot";
        arrowhead_diamond: "connectorStory.arrowheadDiamond" = "Diamond";
        /// The bar's last choice: `{choice}`.
        chose: "connectorStory.chose" = "Chose: {choice}";
    }
}

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "primitives/connector-tool/tool",
            Level::Primitives,
            "ConnectorTool",
            "Tool",
            ToolArgs { view: ViewArgs::default(), routing: RoutingArg::Elbow, snap_free_end: true, add_buttons: true },
            show_tool
        )
        .on(Surface::Canvas),
        typed_story!(
            "primitives/connector-tool/routes",
            Level::Primitives,
            "ConnectorTool",
            "Routes",
            RoutesArgs { view: ViewArgs::default(), routing: RoutingArg::Elbow, layout: LayoutArg::Diagonal },
            show_routes
        )
        .on(Surface::Canvas),
        typed_story!(
            "patterns/connector-bar/bar",
            Level::Patterns,
            "ConnectorBar",
            "Bar",
            BarArgs { routing: RoutingArg::Straight, dashed: false, weight: 2.0, caps: true, mixed: false },
            show_bar
        ),
    ]
}

// --- args ---

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum RoutingArg {
    Straight,
    Elbow,
    Curved,
}

impl RoutingArg {
    fn routing(self) -> Routing {
        match self {
            RoutingArg::Straight => Routing::Straight,
            RoutingArg::Elbow => Routing::Elbow,
            RoutingArg::Curved => Routing::Curved,
        }
    }
}

/// Where the second box of a gallery cell lies.
#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum LayoutArg {
    /// Below and to the right of the first.
    Diagonal,
    /// Beside the first, on the same row.
    Beside,
    /// Under the first, overlapping its lower edge.
    Overlapping,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct ToolArgs {
    #[arg(flatten)]
    view: ViewArgs,
    /// The routing of new connectors and of the selected one.
    routing: RoutingArg,
    /// Holds ⇧ to 45° steps for a free end.
    #[arg(name = "snap free end")]
    snap_free_end: bool,
    /// Shows the "+" buttons around the first box.
    #[arg(name = "add buttons")]
    add_buttons: bool,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct RoutesArgs {
    #[arg(flatten)]
    view: ViewArgs,
    routing: RoutingArg,
    layout: LayoutArg,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct BarArgs {
    routing: RoutingArg,
    dashed: bool,
    #[arg(range = 1.0..=6.0)]
    weight: f64,
    /// Shows the start and end cap menus.
    caps: bool,
    /// Shows the caps as mixed across a selection.
    mixed: bool,
}

// --- painting ---

/// A route as a line, a curve as its cubic.
fn paint_route(painter: &Painter, view: &CanvasView, points: &[Point], routing: Routing, stroke: Stroke) {
    let screen: Vec<Pos2> = points.iter().map(|&p| view.to_screen(p)).collect();
    match (routing, screen.as_slice()) {
        (Routing::Curved, &[a, b, c, d]) => {
            painter.add(CubicBezierShape::from_points_stroke([a, b, c, d], false, Color32::TRANSPARENT, stroke));
        }
        _ => {
            painter.add(Shape::line(screen, stroke));
        }
    }
}

/// A box of the fake board.
fn paint_box(painter: &Painter, view: &CanvasView, tokens: &UiTokens, rect: Rect) {
    let accent = tokens.color.accent.solid.to_egui();
    let stroke = Stroke::new(tokens.component.line_guide.width, accent);
    painter.add(Shape::convex_polygon(view.quad(rect).to_vec(), accent.gamma_multiply(0.15), stroke));
}

// --- the fake board ---

/// A box at fractions of the story document: its left, top, width and height.
fn frac(x: f64, y: f64, w: f64, h: f64) -> Rect {
    Rect::from_origin_size((DOC.width() * x, DOC.height() * y), Size::new(DOC.width() * w, DOC.height() * h))
}

/// Three boxes; the third has six numbered sites (two on each long side and one on each short
/// side), as DeckCraft's shapes have.
fn fake_targets() -> Vec<ConnectionTarget> {
    let third = frac(0.38, 0.62, 0.26, 0.28);
    let site = |point: Point, side: ConnectionSide, index: u32| ConnectionSite { point, side, index };
    let (x0, x1, y0, y1) = (third.x0, third.x1, third.y0, third.y1);
    let mid = third.center();
    let sites = vec![
        site(Point::new(x0 + third.width() / 4.0, y0), ConnectionSide::Top, 0),
        site(Point::new(x1 - third.width() / 4.0, y0), ConnectionSide::Top, 1),
        site(Point::new(x1, mid.y), ConnectionSide::Right, 2),
        site(Point::new(x1 - third.width() / 4.0, y1), ConnectionSide::Bottom, 3),
        site(Point::new(x0 + third.width() / 4.0, y1), ConnectionSide::Bottom, 4),
        site(Point::new(x0, mid.y), ConnectionSide::Left, 5),
    ];
    vec![
        ConnectionTarget::new(1, frac(0.08, 0.12, 0.24, 0.26)),
        ConnectionTarget::new(2, frac(0.66, 0.1, 0.26, 0.26)),
        ConnectionTarget { sites, ..ConnectionTarget::new(3, third) },
    ]
}

fn glued(target: u64, side: ConnectionSide) -> ConnectorEnd {
    ConnectorEnd::Glued { target, side, site: None, at: None }
}

/// What the tool story keeps: the tool, the connectors an app would store and the selection.
struct Board {
    tool: ConnectorTool,
    targets: Vec<ConnectionTarget>,
    items: Vec<ConnectorItem>,
    selected: usize,
    next: u64,
    routing: RoutingArg,
    note: String,
}

impl Board {
    fn new(routing: RoutingArg) -> Self {
        let first = ConnectorItem {
            id: 1,
            from: glued(1, ConnectionSide::Right),
            to: glued(2, ConnectionSide::Left),
            routing: routing.routing(),
            path: None,
        };
        let second = ConnectorItem {
            id: 2,
            from: glued(2, ConnectionSide::Bottom),
            to: ConnectorEnd::Glued { target: 3, side: ConnectionSide::Right, site: Some(2), at: None },
            routing: routing.routing(),
            path: None,
        };
        Board { tool: ConnectorTool::new(), targets: fake_targets(), items: vec![first, second], selected: 0, next: 3, routing, note: String::new() }
    }

    fn route_end(&self, end: &ConnectorEnd) -> Option<RouteEnd> {
        match *end {
            ConnectorEnd::Free(p) => Some(RouteEnd::Point(p)),
            ConnectorEnd::Glued { target, side, site, at } => {
                let t = self.targets.iter().find(|t| t.id == target)?;
                let b = t.bounds;
                if let Some(f) = at {
                    return Some(RouteEnd::On(b, side, Point::new(b.x0 + f[0] * b.width(), b.y0 + f[1] * b.height())));
                }
                match site.and_then(|i| t.side_sites().into_iter().find(|s| s.index == i)) {
                    Some(s) => Some(RouteEnd::On(b, side, s.point)),
                    None => Some(RouteEnd::Box(b, side)),
                }
            }
        }
    }

    fn points(&self, item: &ConnectorItem) -> Option<Vec<Point>> {
        route_along(self.route_end(&item.from)?, self.route_end(&item.to)?, item.routing, item.path.as_ref())
    }

    /// Applies what the tool reported, as an app does.
    fn apply(&mut self, words: &ConnectorStoryLabels, events: Vec<ConnectorEvent>) {
        for event in events {
            match event {
                ConnectorEvent::Draw(edit) => {
                    self.note = format!("{} {}", words.draw, describe(&edit));
                    if let EditEvent::Commit { value, changed: true, .. } = edit {
                        self.items.push(ConnectorItem { id: self.next, from: value.from, to: value.to, routing: value.routing, path: None });
                        self.next += 1;
                        self.selected = self.items.len() - 1;
                    }
                }
                ConnectorEvent::Reattach { end, event } => {
                    let name = if end == ConnectorEndKind::Start { &words.reattach_start } else { &words.reattach_end };
                    self.note = format!("{name} {}", describe(&event));
                    if let EditEvent::Commit { value, changed: true, .. } = event
                        && let Some(item) = self.items.get_mut(self.selected)
                    {
                        match end {
                            ConnectorEndKind::Start => item.from = value,
                            ConnectorEndKind::End => item.to = value,
                        }
                        item.path = None;
                    }
                }
                ConnectorEvent::Reshape(edit) => {
                    self.note = format!("{} {}", words.reshape, describe(&edit));
                    if let EditEvent::Commit { value, changed: true, .. } = edit
                        && let Some(item) = self.items.get_mut(self.selected)
                    {
                        item.path = Some(value);
                    }
                }
                ConnectorEvent::AddConnected { side } => {
                    self.note = craft_i18n::fmt(&words.add_connected, &[("side", &format!("{side:?}"))]);
                }
            }
        }
    }

    /// Sets the routing of the selected connector to the arg's.
    fn route_as(&mut self, routing: RoutingArg) {
        if routing != self.routing {
            self.routing = routing;
            if let Some(item) = self.items.get_mut(self.selected) {
                item.routing = routing.routing();
                item.path = None;
            }
        }
    }
}

// --- tool ---

fn show_tool(ui: &mut Ui, args: &mut ToolArgs) {
    let words = StoryLabels::of(ui);
    let own = ConnectorStoryLabels::of(ui);
    let canvas = document(ui, args.view, &words.document);
    let board = shared(ui, "connector-tool", || Board::new(args.routing));
    let mut board = lock(&board);
    board.route_as(args.routing);

    let tokens = Tokens::of(ui);
    let painter = ui.painter_at(canvas.view.rect);
    let style = ConnectorTool::style(&tokens);
    let muted = Stroke::new(style.line.width, Content::Muted.color(&tokens).to_egui());
    for t in &board.targets {
        paint_box(&painter, &canvas.view, &tokens, t.bounds);
    }
    for (at, item) in board.items.iter().enumerate() {
        if at != board.selected
            && let Some(points) = board.points(item)
        {
            paint_route(&painter, &canvas.view, &points, item.routing, muted);
        }
    }

    let board = &mut *board;
    let selected = board.items.get(board.selected).cloned();
    let first = board.targets.first().map(|t| t.bounds);
    board
        .tool
        .labels(ConnectorToolLabels::of(ui))
        .targets(&board.targets)
        .routing(args.routing.routing())
        .selected(selected.as_ref())
        .connected(if args.add_buttons { first } else { None })
        .snap_free_end(args.snap_free_end);
    let _ = ToolHost::new(canvas.id).document(DOC.size()).show(ui, &canvas.view, &canvas.response, &mut board.tool);
    let events = board.tool.take_events();
    board.apply(&own, events);

    ui.horizontal(|ui| {
        if ui.add(Button::new(own.select_next.as_str())).clicked() && !board.items.is_empty() {
            board.selected = (board.selected + 1) % board.items.len();
        }
        if ui.add(Button::new(words.reset.as_str())).clicked() {
            *board = Board::new(args.routing);
        }
    });
    // On a line of its own: beside the buttons it is wider than a gallery tile.
    let event = if board.note.is_empty() { words.no_event.clone() } else { board.note.clone() };
    let line = craft_i18n::fmt(&own.status, &[("count", &board.items.len().to_string()), ("event", &event)]);
    ui.add(Text::new(line).content(Content::Muted));
}

// --- routes ---

/// The gallery: a cell per pair of sides (rows are the first box's side, columns the second's),
/// each with two boxes placed by `layout` and the route between them.
fn show_routes(ui: &mut Ui, args: &mut RoutesArgs) {
    let words = StoryLabels::of(ui);
    let canvas = document(ui, args.view, &words.document);
    let tokens = Tokens::of(ui);
    let painter = ui.painter_at(canvas.view.rect);
    let style = ConnectorTool::style(&tokens);
    let ink = Stroke::new(style.line.width, Content::Normal.color(&tokens).to_egui());
    let cell = Size::new(DOC.width() / 4.0, DOC.height() / 4.0);
    let (first, second) = match args.layout {
        LayoutArg::Diagonal => ((0.08, 0.1, 0.3, 0.3), (0.6, 0.6, 0.3, 0.3)),
        LayoutArg::Beside => ((0.06, 0.3, 0.26, 0.4), (0.68, 0.3, 0.26, 0.4)),
        LayoutArg::Overlapping => ((0.2, 0.08, 0.5, 0.42), (0.3, 0.4, 0.5, 0.42)),
    };
    let place = |origin: Vec2, (x, y, w, h): (f64, f64, f64, f64)| {
        Rect::from_origin_size((origin.x + cell.width * x, origin.y + cell.height * y), Size::new(cell.width * w, cell.height * h))
    };
    for (row, &from_side) in ConnectionSide::ALL.iter().enumerate() {
        for (column, &to_side) in ConnectionSide::ALL.iter().enumerate() {
            let origin = Vec2::new(cell.width * column as f64, cell.height * row as f64);
            let (a, b) = (place(origin, first), place(origin, second));
            paint_box(&painter, &canvas.view, &tokens, a);
            paint_box(&painter, &canvas.view, &tokens, b);
            if let Some(points) = route(RouteEnd::Box(a, from_side), RouteEnd::Box(b, to_side), args.routing.routing()) {
                paint_route(&painter, &canvas.view, &points, args.routing.routing(), ink);
            }
        }
    }
}

// --- bar ---

/// The fake arrowheads: an id per head and the painter of its sample.
const HEADS: [&str; 4] = ["none", "arrow", "dot", "diamond"];

fn heads(words: &ConnectorStoryLabels) -> Vec<StrokeChoice> {
    [&words.arrowhead_none, &words.arrowhead_arrow, &words.arrowhead_dot, &words.arrowhead_diamond]
        .into_iter()
        .zip(HEADS)
        .map(|(label, id)| StrokeChoice::new(id, label.clone()))
        .collect()
}

/// Paints a cap's sample: a line with the head at its right end, or at its left for `start`.
fn paint_head(tokens: &UiTokens, start: bool) -> impl Fn(&Painter, ScreenRect, &StrokeChoice) + use<> {
    let ink = Content::Normal.color(tokens).to_egui();
    let width = ConnectorTool::style(tokens).line.width;
    move |painter, rect, choice| {
        let stroke = Stroke::new(width, ink);
        let y = rect.center().y;
        let size = rect.height() / 3.0;
        let (tip, tail) = if start { (rect.left_center(), rect.right_center()) } else { (rect.right_center(), rect.left_center()) };
        painter.line_segment([tail, tip], stroke);
        let back = if start { 1.0 } else { -1.0 };
        let base = Pos2::new(tip.x + back * size, y);
        match choice.id.as_str() {
            "arrow" => {
                painter.line_segment([tip, Pos2::new(base.x, y - size)], stroke);
                painter.line_segment([tip, Pos2::new(base.x, y + size)], stroke);
            }
            "dot" => {
                painter.circle_filled(Pos2::new(tip.x + back * size / 2.0, y), size / 2.0, ink);
            }
            "diamond" => {
                painter.add(Shape::convex_polygon(
                    vec![
                        tip,
                        Pos2::new(tip.x + back * size, y - size / 2.0),
                        Pos2::new(tip.x + back * 2.0 * size, y),
                        Pos2::new(tip.x + back * size, y + size / 2.0),
                    ],
                    ink,
                    stroke,
                ));
            }
            _ => {}
        }
    }
}

struct BarKept {
    start: String,
    end: String,
    chose: String,
}

fn show_bar(ui: &mut Ui, args: &mut BarArgs) {
    let own = ConnectorStoryLabels::of(ui);
    let tokens = Tokens::of(ui);
    let state = shared(ui, "bar", || BarKept { start: "none".into(), end: "arrow".into(), chose: String::new() });
    let mut kept = lock(&state);
    let choices = heads(&own);
    let label = ConnectorToolLabels::of(ui).group;
    let routing = args.routing.routing();
    let line = ConnectorLine { dashed: args.dashed, weight: args.weight };
    let mut out = ConnectorBarOutput::default();
    OptionsBar::new("connector-story-options").label(label).show(ui, |slots| {
        let mut bar = ConnectorBar::new("connector-story-bar", routing, line);
        if args.caps {
            let selected = |id: &'static str, mixed: bool| if mixed { MixedValue::Mixed } else { MixedValue::Same(id) };
            let start_id = HEADS.iter().copied().find(|h| *h == kept.start).unwrap_or("none");
            let end_id = HEADS.iter().copied().find(|h| *h == kept.end).unwrap_or("none");
            let start = StrokeChoiceRow::new("start", "", &choices).selected(selected(start_id, args.mixed)).paint(paint_head(&tokens, true));
            let end = StrokeChoiceRow::new("end", "", &choices).selected(selected(end_id, args.mixed)).paint(paint_head(&tokens, false));
            bar = bar.caps(start, end);
        }
        bar.output(&mut out).add_to(slots);
    });
    if let Some(r) = out.routing {
        args.routing = match r {
            Routing::Straight => RoutingArg::Straight,
            Routing::Elbow => RoutingArg::Elbow,
            Routing::Curved => RoutingArg::Curved,
        };
        kept.chose = format!("{r:?}");
    }
    if let Some(id) = out.start_cap {
        kept.start.clone_from(&id);
        kept.chose = id;
    }
    if let Some(id) = out.end_cap {
        kept.end.clone_from(&id);
        kept.chose = id;
    }
    if let Some(l) = out.line {
        args.dashed = l.dashed;
        args.weight = l.weight;
        kept.chose = format!("{l:?}");
    }
    if !kept.chose.is_empty() {
        ui.add(Text::new(craft_i18n::fmt(&own.chose, &[("choice", &kept.chose)])).content(Content::Muted));
    }
}
