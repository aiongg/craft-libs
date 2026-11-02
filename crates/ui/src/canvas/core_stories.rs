//! Stories of the step 3 canvas core. Not yet in the canvas stories list: registering them would
//! change `tests/dumps/stories/registry.txt` and run their bodies in
//! `every_story_shows_with_its_default_args`. The fill task that lands the stories registers
//! them and regenerates the dump.

//! Stories of the step 3 canvas core: navigation, the raster display, the pasteboard, split views
//! and the tool host, each on the story canvas with generated demo documents.

use egui::{Color32, Sense, Ui, vec2};
use kurbo::{Point, Rect, Vec2};

use super::navigate::{PageKey, WheelNavigation, pan};
use super::story::{DOC, StoryCanvas, ViewArgs, document, document_at, document_with, story_canvas_id};
use super::view::CanvasView;
use super::zoom::{self, ZoomAnimation};
use super::{
    CanvasGrid, CanvasTool, CheckerSize, Checkerboard, HandTool, Marquee, MarqueeLook, MomentaryKeys, MomentaryTool, NavigationLabels,
    NavigationOptions, PageBorder, PageFrame, Pasteboard, PasteboardColor, PixelGrid, PixelRect, RasterDisplay, RasterSource, Readout, RotateView,
    RotateViewTool, SnapGesture, SnapOptions, SnapTargets, SplitLayout, SplitPane, SplitViews, ToolAction, ToolContext, ToolCursor, ToolGesture,
    ToolHost, ToolPointer, ViewHistory, ViewPlace, ZoomTool,
};
use crate::controls::Button;
use crate::edit::CancelReason;
use crate::localized::{Labels as _, LocaleNumbers};
use crate::primitives::{Content, Text};
use crate::story::{Level, Story, StoryArgs, StoryChoice, Surface};
use crate::theme::{ToEgui as _, Tokens, TokensOf};
use crate::typed_story;

#[derive(StoryArgs, Debug, PartialEq)]
struct NavigationArgs {
    #[arg(flatten)]
    view: ViewArgs,
    #[arg(name = "zoom with wheel")]
    zoom_with_wheel: bool,
    #[arg(name = "animated zoom")]
    animated_zoom: bool,
    #[arg(name = "scrubby zoom")]
    scrubby_zoom: bool,
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum CheckerArg {
    None,
    Small,
    Medium,
    Large,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct RasterArgs {
    #[arg(flatten)]
    view: ViewArgs,
    checker: CheckerArg,
    #[arg(name = "pixel grid")]
    pixel_grid: bool,
    #[arg(name = "large document")]
    large: bool,
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum BorderArg {
    DropShadow,
    Line,
    None,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct PasteboardArgs {
    #[arg(flatten)]
    view: ViewArgs,
    border: BorderArg,
    #[arg(range = 1..=4)]
    pages: usize,
    names: bool,
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum LayoutArg {
    One,
    Two,
    Four,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct SplitArgs {
    layout: LayoutArg,
    linked: bool,
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum DemoToolArg {
    Marquee,
    Hand,
    Zoom,
    RotateView,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct ToolHostArgs {
    #[arg(flatten)]
    view: ViewArgs,
    tool: DemoToolArg,
    snap: bool,
}

pub(super) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "primitives/canvas/navigation",
            Level::Primitives,
            "Canvas",
            "Navigation",
            NavigationArgs { view: ViewArgs::default(), zoom_with_wheel: false, animated_zoom: true, scrubby_zoom: true },
            show_navigation
        )
        .on(Surface::Canvas),
        typed_story!(
            "primitives/canvas/raster",
            Level::Primitives,
            "Canvas",
            "Raster display",
            RasterArgs { view: ViewArgs::default(), checker: CheckerArg::Medium, pixel_grid: true, large: false },
            show_raster
        )
        .on(Surface::Canvas),
        typed_story!(
            "primitives/canvas/pasteboard",
            Level::Primitives,
            "Canvas",
            "Pasteboard",
            PasteboardArgs { view: ViewArgs::default(), border: BorderArg::DropShadow, pages: 2, names: true },
            show_pasteboard
        )
        .on(Surface::Canvas),
        typed_story!(
            "primitives/canvas/split",
            Level::Primitives,
            "Canvas",
            "Split views",
            SplitArgs { layout: LayoutArg::Two, linked: false },
            show_split
        )
        .on(Surface::Canvas),
        typed_story!(
            "primitives/canvas/tool-host",
            Level::Primitives,
            "Canvas",
            "Tool host",
            ToolHostArgs { view: ViewArgs::default(), tool: DemoToolArg::Marquee, snap: true },
            show_tool_host
        )
        .on(Surface::Canvas),
    ]
}

/// A story view kept between passes: the app's view (its target when a zoom eases) and the args
/// it was seeded from.
#[derive(Clone, Copy)]
struct Kept {
    view: CanvasView,
    seeded: ViewArgs,
}

/// The story canvas under a view that tools and wheel change. The args seed it; it is seeded again
/// when they change. Returns the canvas, drawn under the eased view, and the app's view (the
/// target).
fn kept_canvas(ui: &mut Ui, args: ViewArgs, label: &str, animated: bool) -> (StoryCanvas, CanvasView) {
    let key = ui.scope_id().with(("core-story-view", label));
    let kept: Option<Kept> = ui.data(|d| d.get_temp(key));
    let ctx = ui.ctx().clone();
    let canvas_id = story_canvas_id(ui, label);
    let mut target = None;
    let canvas = document_with(ui, args, label, 0.0, |rect, ppp| {
        let seeded =
            CanvasView { rect, center: DOC.center(), zoom: args.zoom / 100.0 / ppp, rotation: args.rotation, flip_x: args.flip, aspect: args.aspect };
        let view = match kept {
            Some(k) if k.seeded == args => CanvasView { rect, ..k.view },
            _ => seeded,
        };
        target = Some(view);
        ZoomAnimation::shown(&ctx, canvas_id, &view, animated)
    });
    let target = target.unwrap_or(canvas.view);
    (canvas, target)
}

fn remember(ui: &Ui, label: &str, view: CanvasView, seeded: ViewArgs) {
    let key = ui.scope_id().with(("core-story-view", label));
    ui.data_mut(|d| d.insert_temp(key, Kept { view, seeded }));
}

crate::words! {
    /// The core stories' words, English by default.
    #[non_exhaustive]
    pub struct CoreStoryLabels {
        /// The zoom readout under a story canvas; `{n}` is the percentage.
        zoom_percent: "canvasStory.zoomPercent" = "{n}%";
    }
}

/// The zoom as a percentage under the canvas.
fn zoom_readout(ui: &mut Ui, view: &CanvasView) {
    let numbers = LocaleNumbers::of(ui);
    let percent = zoom::device_zoom(view, ui.pixels_per_point()) * 100.0;
    let text = craft_i18n::fmt(&CoreStoryLabels::of(ui).zoom_percent, &[("n", &numbers.format(percent, 0..=1))]);
    ui.add(Text::new(text).content(Content::Muted));
}

/// The tools of the navigation story and the places its Zoom tool jumped between.
#[derive(Clone)]
struct NavigationTools {
    hand: HandTool,
    zoom: ZoomTool,
    history: ViewHistory<()>,
}

/// The view's place in the story's history: the story has one document, so one key.
fn place(view: &CanvasView) -> ViewPlace<()> {
    ViewPlace { key: (), center: view.center, zoom: view.zoom }
}

/// A document on the story canvas with wheel, pinch, page keys, the Hand tool's drag and glide,
/// the momentary keys (Space the Hand, Cmd+Space Zoom In, Alt+Space Zoom Out), Go Back and Go
/// Forward through the Zoom tool's jumps, the Rotate View compass (turn the view with the
/// rotation arg) and a zoom readout.
fn show_navigation(ui: &mut Ui, args: &mut NavigationArgs) {
    let prefs = NavigationOptions {
        zoom_with_wheel: args.zoom_with_wheel,
        animated_zoom: args.animated_zoom,
        scrubby_zoom: args.scrubby_zoom,
        ..NavigationOptions::default()
    };
    let ctx = ui.ctx().clone();
    let size = DOC.size();
    WheelNavigation::configure(&ctx);
    let (canvas, mut view) = kept_canvas(ui, args.view, "Navigation", prefs.animated_zoom);
    let (response, shown) = (&canvas.response, canvas.view);
    let ppp = ui.pixels_per_point();
    if let Some(action) = WheelNavigation::read(&ctx, response, &prefs) {
        let pointer = response.hover_pos().unwrap_or(shown.rect.center());
        WheelNavigation::apply(&mut view, action, pointer, size, &prefs, ppp);
    }
    crate::focus::key_owner::register_canvas(&ctx, response.id);
    PageKey::handle(&ctx, &mut view, size, prefs.overscroll);
    let tool_key = ui.scope_id().with("navigation-tools");
    let mut tools: NavigationTools = ctx.data(|d| d.get_temp(tool_key)).unwrap_or_else(|| {
        let mut history = ViewHistory::default();
        history.visit(place(&view));
        NavigationTools { hand: HandTool::new(size, prefs), zoom: ZoomTool::new(size, prefs), history }
    });
    tools.hand.size = size;
    tools.hand.prefs = prefs;
    tools.zoom.size = size;
    tools.zoom.prefs = prefs;
    let temporary = MomentaryKeys::default().read(&ctx, canvas.id, false);
    let zooming = matches!(temporary, Some(MomentaryTool::ZoomIn | MomentaryTool::ZoomOut));
    tools.zoom.out = temporary == Some(MomentaryTool::ZoomOut);
    let before = view;
    let tool: &mut dyn CanvasTool = if zooming { &mut tools.zoom } else { &mut tools.hand };
    // The host works on the target view: a pan that begins while the zoom eases keeps the target zoom.
    let hosted = ToolHost::new(canvas.id).document(size).navigation(prefs).show(ui, &view, response, tool);
    if let Some(v) = hosted.view {
        view = CanvasView { rect: view.rect, ..v };
    }
    // A Zoom tool click is a jump: Go Back returns to where it left.
    if zooming && hosted.gesture == Some(ToolGesture::Commit) && place(&view) != place(&before) {
        tools.history.leave(place(&before));
        tools.history.visit(place(&view));
    }
    let tokens = Tokens::of(ui);
    if RotateView::compass(ui, &shown, &tokens).reset {
        view.rotation = 0.0;
    }
    let labels = NavigationLabels::of(ui);
    ui.horizontal(|ui| {
        for (step, label) in [(-1, &labels.go_back), (1, &labels.go_forward)] {
            if ui.add_enabled(tools.history.can_go(step, |_| true), Button::new(label.as_str())).clicked()
                && let Some(to) = tools.history.go(step, |_| true)
            {
                if prefs.animated_zoom {
                    ZoomAnimation::begin(&ctx, canvas.id, &view, None);
                }
                view.center = to.center;
                view.zoom = to.zoom;
            }
        }
        zoom_readout(ui, &view);
    });
    ctx.data_mut(|d| d.insert_temp(tool_key, tools));
    remember(ui, "Navigation", view, args.view);
}

/// The generated document of the raster story: a colour gradient that fades to transparent at
/// the left edge, with fine stripes in a large document so the detail texture shows at 100 %.
struct Gradient {
    size: [u32; 2],
    stripes: bool,
}

impl RasterSource for Gradient {
    fn size(&self) -> [u32; 2] {
        self.size
    }

    fn revision(&self) -> u64 {
        u64::from(self.size[0])
    }

    fn read(&self, region: PixelRect, factor: u32, out: &mut egui::ColorImage) {
        let [width, height] = self.size.map(|v| v as f32);
        let [columns, rows] = out.size;
        for row in 0..rows {
            let y = region.y0 as f32 + (row as u32 * factor) as f32;
            for column in 0..columns {
                let x = region.x0 as f32 + (column as u32 * factor) as f32;
                let stripe = if self.stripes && ((x as u32 / 4) + (y as u32 / 4)).is_multiple_of(2) { 0.82 } else { 1.0 };
                let channel = |v: f32| (v.clamp(0.0, 1.0) * stripe * 255.0).round() as u8;
                let alpha = (x / (width * 0.35)).clamp(0.0, 1.0);
                out.pixels[row * columns + column] =
                    Color32::from_rgba_unmultiplied(channel(x / width), channel(y / height), channel(0.55), (alpha * 255.0).round() as u8);
            }
        }
    }
}

/// A generated gradient document through `RasterDisplay` over the checkerboard, with the pixel
/// grid; `large` makes it 6000 × 4000 so the detail texture shows at 100 %.
fn show_raster(ui: &mut Ui, args: &mut RasterArgs) {
    let canvas = document(ui, args.view, "Raster display");
    let size = if args.large { [6000, 4000] } else { [DOC.width() as u32, DOC.height() as u32] };
    let source = Gradient { size, stripes: args.large };
    let checker = Checkerboard {
        size: match args.checker {
            CheckerArg::None => CheckerSize::None,
            CheckerArg::Small => CheckerSize::Small,
            CheckerArg::Medium => CheckerSize::Medium,
            CheckerArg::Large => CheckerSize::Large,
        },
        ..Checkerboard::default()
    };
    let id = ui.scope_id().with("raster-story");
    let _ = RasterDisplay::new(id).checkerboard(checker).show(ui, &canvas.view, &source);
    if args.pixel_grid {
        PixelGrid::show(ui.ctx(), canvas.id, &ui.painter_at(canvas.view.rect), &canvas.view, &source);
    }
}

/// Pages side by side on the pasteboard, named, with the border and a click on a name or a
/// secondary click on the pasteboard reported below.
fn show_pasteboard(ui: &mut Ui, args: &mut PasteboardArgs) {
    const PAGE: (f64, f64) = (200.0, 300.0);
    const GAP: f64 = 40.0;
    let count = args.pages.clamp(1, 4);
    let total = count as f64 * PAGE.0 + (count - 1) as f64 * GAP;
    let pages: Vec<PageFrame> = (0..count)
        .map(|i| {
            let x = -total / 2.0 + i as f64 * (PAGE.0 + GAP);
            let page = PageFrame::new(Rect::new(x, -PAGE.1 / 2.0, x + PAGE.0, PAGE.1 / 2.0)).fill(Color32::WHITE);
            if args.names { page.name(format!("Page {}", i + 1)) } else { page }
        })
        .collect();
    let canvas = document_at(ui, args.view, "Pasteboard", 0.0, Point::ORIGIN);
    let border = match args.border {
        BorderArg::DropShadow => PageBorder::DropShadow,
        BorderArg::Line => PageBorder::Line,
        BorderArg::None => PageBorder::None,
    };
    let sensed = Pasteboard::new("pasteboard-story", &pages).color(PasteboardColor::Default).border(border).show(ui, &canvas.view);
    let key = ui.scope_id().with("pasteboard-story-note");
    let mut note: String = ui.data(|d| d.get_temp(key)).unwrap_or_default();
    if let Some(page) = sensed.name_clicked {
        note = format!("Page {}", page + 1);
    } else if let Some(at) = sensed.context_at {
        note = format!("{:.0}, {:.0}", at.x, at.y);
    }
    ui.add(Text::new(note.clone()).content(Content::Muted));
    ui.data_mut(|d| d.insert_temp(key, note));
}

/// One pane's document: the page under the pane's own view with a cross through it.
fn paint_pane(ui: &mut Ui, view: &CanvasView, number: usize, active: bool) {
    let tokens = Tokens::of(ui);
    let painter = ui.painter_at(view.rect);
    painter.rect_filled(view.rect, 0.0, tokens.color.surface.canvas.to_egui());
    painter.add(egui::Shape::convex_polygon(view.quad(DOC).to_vec(), Color32::WHITE, egui::Stroke::NONE));
    let line = egui::Stroke::new(1.0, Color32::from_gray(160));
    let [a, b, c, d] = view.quad(DOC);
    painter.line_segment([a, c], line);
    painter.line_segment([b, d], line);
    let tint = if active { Content::Normal } else { Content::Muted };
    ui.put(egui::Rect::from_min_size(view.rect.min + vec2(8.0, 6.0), vec2(80.0, 20.0)), Text::new(number.to_string()).content(tint));
}

/// The canvas area in one, two or four views: a press activates a pane, a drag pans it, the
/// wheel zooms it, and with `linked` the other panes follow the active one's centre and zoom.
fn show_split(ui: &mut Ui, args: &mut SplitArgs) {
    let key = ui.scope_id().with("split-story");
    let (mut panes, mut active, last): (Vec<SplitPane>, usize, Option<LayoutArg>) = ui.data(|d| d.get_temp(key)).unwrap_or_default();
    let layout = match args.layout {
        LayoutArg::One => SplitLayout::One,
        LayoutArg::Two => SplitLayout::Two,
        LayoutArg::Four => SplitLayout::Four,
    };
    let (full, _) = ui.allocate_exact_size(vec2(ui.available_width().clamp(240.0, 720.0), 360.0), Sense::hover());
    let ppp = ui.pixels_per_point();
    let rects = layout.rects(Tokens::of(ui).component.split.gap, full);
    if last != Some(args.layout) || panes.len() != rects.len() {
        panes = rects
            .iter()
            .map(|r| {
                let mut view = CanvasView::new(*r, DOC.center(), 1.0);
                zoom::fit_view(&mut view, DOC.size(), ppp);
                SplitPane { view, linked: false }
            })
            .collect();
        active = 0;
    }
    let prefs = NavigationOptions { zoom_with_wheel: true, ..NavigationOptions::default() };
    let mut views: Vec<CanvasView> = panes.iter().map(|p| p.view).collect();
    let sensed = SplitViews::new("split-story-views", layout).active(active).show(ui, full, |ui, i, r| {
        views[i].rect = r;
        let response = ui.interact(r, ui.scope_id().with(("pane", i)), Sense::click_and_drag());
        if response.dragged() {
            pan(&mut views[i], response.drag_delta());
        }
        if let Some(action) = WheelNavigation::read(ui.ctx(), &response, &prefs) {
            let at = response.hover_pos().unwrap_or(r.center());
            WheelNavigation::apply(&mut views[i], action, at, DOC.size(), &prefs, ppp);
        }
        paint_pane(ui, &views[i], i + 1, i == active);
    });
    if let Some(i) = sensed.activated {
        active = i;
    }
    for (i, pane) in panes.iter_mut().enumerate() {
        pane.view = views[i];
        pane.linked = args.linked && i != active;
    }
    let main = panes[active].view;
    SplitViews::follow(&main, &mut panes);
    ui.data_mut(|d| d.insert_temp(key, (panes, active, Some(args.layout))));
}

/// The selection tool of the tool host story: a marquee that snaps its corners, reads out its
/// size beside the pointer and keeps the last selection.
#[derive(Clone, Debug, Default)]
struct MarqueeDemo {
    start: Option<Point>,
    end: Point,
    selection: Option<Rect>,
    snap: bool,
}

impl MarqueeDemo {
    fn rect(&self) -> Option<Rect> {
        self.start.map(|start| Rect::from_points(start, self.end))
    }
}

impl CanvasTool for MarqueeDemo {
    fn cursor(&self, _cx: &ToolContext<'_>) -> ToolCursor {
        ToolCursor::System(egui::CursorIcon::Crosshair)
    }

    fn press(&mut self, pointer: &ToolPointer, _cx: &mut ToolContext<'_>) -> ToolAction {
        self.start = Some(pointer.doc);
        self.end = pointer.doc;
        ToolAction::Claimed
    }

    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        self.end = pointer.doc;
        if let Some(r) = self.rect() {
            cx.readout(Readout::new("tool-host-story").row("W:", format!("{:.0}", r.width())).row("H:", format!("{:.0}", r.height())));
        }
    }

    fn release(&mut self, pointer: &ToolPointer, _cx: &mut ToolContext<'_>) {
        self.end = pointer.doc;
        self.selection = self.rect().filter(|r| r.width() >= 1.0 && r.height() >= 1.0);
        self.start = None;
    }

    fn cancel(&mut self, _reason: CancelReason, _cx: &mut ToolContext<'_>) {
        self.start = None;
    }

    fn paint(&self, painter: &egui::Painter, cx: &ToolContext<'_>) {
        if let Some(r) = self.rect().or(self.selection) {
            Marquee::rect(r, "Selection").paint(painter, cx.view, MarqueeLook::Ants, cx.time);
        }
    }

    fn snap_gesture(&self) -> Option<SnapGesture> {
        self.snap.then_some(SnapGesture::Point)
    }
}

/// The tools of the tool host story; each keeps its own gesture state between passes.
#[derive(Clone)]
struct DemoTools {
    marquee: MarqueeDemo,
    hand: HandTool,
    zoom: ZoomTool,
    rotate: RotateViewTool,
    /// The tool the host has been showing.
    shown: DemoToolArg,
}

impl DemoTools {
    fn pick(&mut self, arg: DemoToolArg) -> &mut dyn CanvasTool {
        match arg {
            DemoToolArg::Marquee => &mut self.marquee,
            DemoToolArg::Hand => &mut self.hand,
            DemoToolArg::Zoom => &mut self.zoom,
            DemoToolArg::RotateView => &mut self.rotate,
        }
    }
}

/// Small demo tools on `ToolHost`: a marquee (autoscroll, snapping, readout), the Hand, the Zoom
/// tool (click, hold, scrub) and Rotate View.
fn show_tool_host(ui: &mut Ui, args: &mut ToolHostArgs) {
    const GRID: f64 = 40.0;
    let prefs = NavigationOptions::default();
    let ctx = ui.ctx().clone();
    let (canvas, mut view) = kept_canvas(ui, args.view, "Tool host", prefs.animated_zoom);
    let (response, shown) = (&canvas.response, canvas.view);
    let key = ui.scope_id().with("tool-host-tools");
    let mut tools: DemoTools = ctx.data(|d| d.get_temp(key)).unwrap_or_else(|| DemoTools {
        marquee: MarqueeDemo::default(),
        hand: HandTool::new(DOC.size(), prefs),
        zoom: ZoomTool::new(DOC.size(), prefs),
        rotate: RotateViewTool::default(),
        shown: args.tool,
    });
    tools.marquee.snap = args.snap;
    // Switching tools ends the gesture of the tool that has it.
    if tools.shown != args.tool {
        let old = tools.shown;
        tools.shown = args.tool;
        if let Some(v) = ToolHost::cancel(&ctx, canvas.id, tools.pick(old), CancelReason::Removed) {
            view = CanvasView { rect: view.rect, ..v };
        }
    }
    CanvasGrid::new("tool-host-story-grid").spacing(Vec2::new(GRID, GRID)).bounds(DOC).show(ui, &shown);
    let targets = SnapTargets::new().with_document(&SnapOptions::default(), DOC, [], Some((Point::ORIGIN, Vec2::new(GRID, GRID))));
    let mut host = ToolHost::new(canvas.id).document(DOC.size()).navigation(prefs);
    if args.snap {
        host = host.snap(&targets);
    }
    let hosted = host.show(ui, &view, response, tools.pick(args.tool));
    ctx.data_mut(|d| d.insert_temp(key, tools));
    if let Some(v) = hosted.view {
        view = CanvasView { rect: view.rect, ..v };
    }
    let tokens = Tokens::of(ui);
    if RotateView::compass(ui, &shown, &tokens).reset {
        view.rotation = 0.0;
    }
    zoom_readout(ui, &view);
    remember(ui, "Tool host", view, args.view);
}
