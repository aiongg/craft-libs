//! Stories of the full-window regions.
//!
//! The image is a generated scene (a gradient with a disc, a rectangle and a transparent corner)
//! and the overlay a generated coverage mask at half resolution, so every state shows without an
//! app. Sample text is a story word. The size
//! (Compact, Comfortable) is the gallery's appearance setting.
//!
//! - `patterns/full-window-side/frame`: title strip, options bar, a `ToolRail` with a group gap, one
//!   Wide `FullWindowSide` with header and footer, the canvas. Args: footer at top or bottom, a
//!   second side column, right-to-left.
//! - `patterns/full-window-canvas/canvas`: fit, zoom steps, pan, the Hand and Zoom tools, the
//!   checkerboard, a texel-scaled `RasterOverlay` with a tint, the `FullWindowZoomBar` with a
//!   progress bar and with a note, `PreviewStatus` pending and failed.
//! - `patterns/tool-rail/rail-in-dialog`: `ToolRail` with group gaps and a shared-letter slot
//!   inside a `FullWindowDialog`, `pick_by_key` on.

use egui::{Color32, ColorImage, Rect, Sense, Ui, Vec2, vec2};
use kurbo::Size;

use super::{
    ButtonRole, DialogLabels, DialogState, FooterButton, FullWindowCanvas, FullWindowDialog, FullWindowFx, FullWindowLabels, FullWindowSide,
    FullWindowTitle, FullWindowView, FullWindowZoomBar, Outcome, PreviewStatus, RasterOverlay, SideFooter, SideWidth,
};
use crate::canvas::{CanvasTool, CheckerSize, Checkerboard, HandTool, NavigationOptions, PixelRect, RasterSource, ZoomTool};
use crate::controls::Slider;
use crate::direction::{self, Direction};
use crate::icons::Icon;
use crate::localized::Labels as _;
use crate::patterns::{LastUsed, OptionsBar, RailTool, ToolRail, ToolSlot};
use crate::story::{self, Level, PatternStoryLabels, Story, StoryArgs, StoryChoice, StoryLabels};
use crate::typed_story;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["FullWindowSide", "FullWindowCanvas"];

crate::words! {
    /// The captions of the full-window stories, English by default.
    #[non_exhaustive]
    pub(crate) struct FullWindowStoryLabels {
        workspace: "fullWindowStory.workspace" = "Liquify";
        properties: "fullWindowStory.properties" = "Properties";
        preview: "fullWindowStory.preview" = "Preview";
        preview_note: "fullWindowStory.previewNote" = "Preview 12 ms";
        out_of_memory: "fullWindowStory.outOfMemory" = "out of memory";
        pencil: "fullWindowStory.pencil" = "Pencil";
        hand: "fullWindowStory.hand" = "Hand";
        current_tool: "fullWindowStory.currentTool" = "Tool: {tool}";
    }
}

/// The generated image's size in pixels.
const IMAGE: [u32; 2] = [192, 128];
/// The overlay mask's pixels per overlay texel.
const MASK_TEXEL: u32 = 2;

type Tool = &'static str;

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "patterns/full-window-side/frame",
            Level::Patterns,
            "FullWindowSide",
            "Frame",
            FrameArgs { open: true, footer_top: false, second_side: false, rtl: false, width: 760.0, height: 380.0 },
            show_frame
        ),
        typed_story!(
            "patterns/full-window-canvas/canvas",
            Level::Patterns,
            "FullWindowCanvas",
            "Canvas",
            CanvasArgs {
                open: true,
                tool: ToolArg::None,
                checkerboard: true,
                overlay: true,
                tint: true,
                zoom_bar: true,
                preview: PreviewArg::Idle,
                note: true,
                width: 760.0,
                height: 380.0,
            },
            show_canvas
        ),
        typed_story!(
            "patterns/tool-rail/rail-in-dialog",
            Level::Patterns,
            "ToolRail",
            "Rail in a dialog",
            RailArgs { open: true, columns: 1, pick_by_key: true, width: 520.0, height: 380.0 },
            show_rail_in_dialog
        ),
    ]
}

// --- the generated scene ---

/// The image: a gradient, a disc, a rectangle and a transparent top-left corner.
struct Scene;

impl RasterSource for Scene {
    fn size(&self) -> [u32; 2] {
        IMAGE
    }

    fn revision(&self) -> u64 {
        1
    }

    fn read(&self, region: PixelRect, factor: u32, out: &mut ColorImage) {
        let [columns, rows] = out.size;
        let [width, height] = IMAGE.map(|v| v as i32);
        let step = factor as i32;
        for row in 0..rows {
            let y = region.y0 + row as i32 * step;
            for column in 0..columns {
                let x = region.x0 + column as i32 * step;
                let (dx, dy) = (x - width * 5 / 8, y - height * 7 / 16);
                out.pixels[row * columns + column] = if x + y < height / 3 {
                    Color32::TRANSPARENT
                } else if dx * dx + dy * dy < (height / 4) * (height / 4) {
                    Color32::from_rgb(232, 150, 60)
                } else if (width / 8..width * 3 / 8).contains(&x) && (height * 11 / 16..height * 7 / 8).contains(&y) {
                    Color32::from_rgb(40, 150, 140)
                } else {
                    Color32::from_rgb((x * 255 / width) as u8, (y * 255 / height) as u8, 140)
                };
            }
        }
    }
}

/// A coverage mask at `MASK_TEXEL` pixels per texel: an ellipse over the disc's side of the image.
struct Coverage;

impl RasterSource for Coverage {
    fn size(&self) -> [u32; 2] {
        IMAGE.map(|v| v / MASK_TEXEL)
    }

    fn revision(&self) -> u64 {
        1
    }

    fn read(&self, region: PixelRect, factor: u32, out: &mut ColorImage) {
        let [columns, rows] = out.size;
        let [width, height] = self.size().map(|v| v as i32);
        let step = factor as i32;
        for row in 0..rows {
            let y = region.y0 + row as i32 * step - height / 2;
            for column in 0..columns {
                let x = region.x0 + column as i32 * step - width * 5 / 8;
                let inside = (x * x) / 4 + y * y < (height / 3) * (height / 3);
                out.pixels[row * columns + column] = if inside { Color32::WHITE } else { Color32::TRANSPARENT }; // design: allow(coverage is multiplied by the tint)
            }
        }
    }
}

// --- shared parts ---

/// Applies the right-to-left arg when it changes.
fn apply_direction(ctx: &egui::Context, applied: &mut Option<bool>, rtl: bool) {
    if *applied != Some(rtl) {
        direction::apply(ctx, if rtl { Direction::Rtl } else { Direction::Ltr });
        *applied = Some(rtl);
    }
}

/// The footer every story column carries, at the top or the bottom of the column.
fn footer<'a>(ui: &Ui, top: bool) -> SideFooter<'a> {
    let labels = DialogLabels::of(ui);
    let buttons = vec![FooterButton::new(ButtonRole::Default, labels.ok), FooterButton::new(ButtonRole::Cancel, labels.cancel)];
    if top { SideFooter::top(buttons) } else { SideFooter::bottom(buttons) }
}

/// Runs `body` in a `FullWindowDialog` over an area at the story's origin; a closing outcome
/// clears `open`.
fn workspace(
    ui: &mut Ui,
    name: &'static str,
    state: &mut DialogState,
    open: &mut bool,
    size: Vec2,
    title: String,
    body: impl FnOnce(&mut Ui, &mut FullWindowFx),
) {
    if crate::story::dialog_shown(ui, *open) {
        let area = Rect::from_min_size(ui.max_rect().min, vec2(size.x.min(ui.available_width()), size.y));
        ui.allocate_rect(area, Sense::hover());
        let shown = FullWindowDialog::new(name).title(title).rect(area).show(ui.ctx(), state, &mut (), |ui, (), fx| body(ui, fx));
        if shown.outcome.is_some_and(Outcome::closes) {
            crate::story::dialog_closed(ui, open);
        }
    }
}

/// The title strip: the workspace's name, its subject and the view's zoom.
fn title(ui: &Ui, view: &FullWindowView) -> String {
    let words = StoryLabels::of(ui);
    let own = FullWindowStoryLabels::of(ui);
    FullWindowTitle::new(own.workspace.as_str()).subject(words.photo).zoom(view.zoom_percent()).text(ui.ctx())
}

/// The slots of the rail stories: one letter shared by two tools, and a gap after them.
fn slots(ui: &Ui) -> Vec<ToolSlot<Tool>> {
    let words = PatternStoryLabels::of(ui);
    let own = FullWindowStoryLabels::of(ui);
    vec![
        ToolSlot::new(0, vec![RailTool::new("move", Icon::Move, words.move_tool.as_str()).shortcut("V")]),
        ToolSlot::new(
            1,
            vec![
                RailTool::new("brush", Icon::Brush, words.brush.as_str()).shortcut("B"),
                RailTool::new("pencil", Icon::Pencil, own.pencil.as_str()).shortcut("B"),
            ],
        )
        .group_end(true),
        ToolSlot::new(2, vec![RailTool::new("eraser", Icon::Eraser, words.eraser.as_str()).shortcut("E")]),
        ToolSlot::new(3, vec![RailTool::new("hand", Icon::Hand, own.hand.as_str()).shortcut("H")]),
        ToolSlot::new(4, vec![RailTool::new("zoom", Icon::ZoomIn, words.zoom.as_str()).shortcut("Z")]),
    ]
}

/// The rail's current tool and its last-used memory.
#[derive(Clone)]
struct Rail {
    current: Tool,
    last: LastUsed<Tool>,
}

impl Default for Rail {
    fn default() -> Self {
        Self { current: "move", last: LastUsed::default() }
    }
}

/// Draws the rail docked in the dialog and returns the current tool.
fn rail(ui: &mut Ui, name: &str, state: &mut Rail, columns: usize, pick_by_key: bool) -> Tool {
    let slots = slots(ui);
    let out = ToolRail::new(name).columns(columns).pick_by_key(pick_by_key).show(ui, &slots, &state.current, &mut state.last, |_, _| {});
    state.current = out.select.unwrap_or(state.current);
    state.current
}

/// The Hand and Zoom tools (they hold gesture state).
#[derive(Clone)]
struct Tools {
    hand: HandTool,
    zoom: ZoomTool,
}

impl Tools {
    fn new(navigation: NavigationOptions) -> Self {
        let doc = Size::new(f64::from(IMAGE[0]), f64::from(IMAGE[1]));
        Self { hand: HandTool::new(doc, navigation), zoom: ZoomTool::new(doc, navigation) }
    }

    fn pick(&mut self, tool: Tool) -> Option<&mut dyn CanvasTool> {
        match tool {
            "hand" => Some(&mut self.hand),
            "zoom" => Some(&mut self.zoom),
            _ => None,
        }
    }
}

/// What one instance of a full-window story keeps.
struct Kept {
    dialog: DialogState,
    view: FullWindowView,
    tools: Tools,
    rail: Rail,
    levels: [f64; 2],
    rtl: Option<bool>,
}

impl Kept {
    fn new() -> Self {
        Self {
            dialog: DialogState::default(),
            view: FullWindowView::default(),
            tools: Tools::new(NavigationOptions::default()),
            rail: Rail::default(),
            levels: [50.0, 100.0],
            rtl: None,
        }
    }
}

/// The label of a rail tool.
fn tool_label(ui: &Ui, tool: Tool) -> String {
    let words = PatternStoryLabels::of(ui);
    let own = FullWindowStoryLabels::of(ui);
    match tool {
        "brush" => words.brush,
        "pencil" => own.pencil,
        "eraser" => words.eraser,
        "hand" => own.hand,
        "zoom" => words.zoom,
        _ => words.move_tool,
    }
}

// --- frame ---

#[derive(StoryArgs, Debug, PartialEq)]
struct FrameArgs {
    open: bool,
    #[arg(name = "footer top")]
    footer_top: bool,
    #[arg(name = "second side")]
    second_side: bool,
    rtl: bool,
    #[arg(range = 480.0..=1100.0)]
    width: f32,
    #[arg(range = 280.0..=700.0)]
    height: f32,
}

/// The whole frame: options bar, tool rail, a Wide column with its footer (at the top or the
/// bottom), optionally a second Standard column, and the canvas with the rail's tool.
fn show_frame(ui: &mut Ui, args: &mut FrameArgs) {
    let shared = story::shared(ui, "frame", Kept::new);
    let mut kept = story::lock(&shared);
    let Kept { dialog, view, tools, rail: rail_state, levels, rtl } = &mut *kept;
    apply_direction(ui.ctx(), rtl, args.rtl);
    let title = title(ui, view);
    let words = StoryLabels::of(ui);
    let own = FullWindowStoryLabels::of(ui);
    let patterns = PatternStoryLabels::of(ui);
    let (footer_top, second_side) = (args.footer_top, args.second_side);
    workspace(ui, "story-frame", dialog, &mut args.open, vec2(args.width, args.height), title, |ui, fx| {
        OptionsBar::new("frame-bar").show(ui, |bar| {
            bar.leading(|ui| drop(OptionsBar::tool_icon(ui, Icon::Brush, &patterns.brush)));
            bar.body(|ui| {
                ui.label(&words.size);
            });
        });
        let tool = rail(ui, "frame-tools", rail_state, 1, true);
        FullWindowSide::new("frame-properties").width(SideWidth::Wide).header(own.properties.as_str()).footer(footer(ui, footer_top)).show(
            ui,
            fx,
            |ui| {
                for (level, label) in levels.iter_mut().zip([&words.size, &words.opacity]) {
                    if let Some(&value) =
                        Slider::new(label.as_str(), label.as_str(), *level, 1.0..=100.0).show(ui).event.as_ref().and_then(|e| e.value())
                    {
                        *level = value;
                    }
                }
            },
        );
        if second_side {
            FullWindowSide::new("frame-extra").header(words.tools.as_str()).show(ui, fx, |ui| {
                ui.label(&words.image);
            });
        }
        let mut canvas = FullWindowCanvas::new("frame-view", view).source(&Scene).checkerboard(Checkerboard::default());
        if let Some(tool) = tools.pick(tool) {
            canvas = canvas.tool(tool);
        }
        canvas.show(ui);
    });
}

// --- canvas ---

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum ToolArg {
    None,
    Hand,
    Zoom,
}

impl ToolArg {
    fn tool(self) -> Tool {
        match self {
            ToolArg::None => "none",
            ToolArg::Hand => "hand",
            ToolArg::Zoom => "zoom",
        }
    }
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum PreviewArg {
    Idle,
    Pending,
    #[choice(name = "half done")]
    HalfDone,
    Failed,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct CanvasArgs {
    open: bool,
    tool: ToolArg,
    checkerboard: bool,
    overlay: bool,
    tint: bool,
    #[arg(name = "zoom bar")]
    zoom_bar: bool,
    preview: PreviewArg,
    note: bool,
    #[arg(range = 480.0..=1100.0)]
    width: f32,
    #[arg(range = 280.0..=700.0)]
    height: f32,
}

/// The canvas on its own: the zoom keys (⌘+, ⌘−, ⌘0, ⌘1), wheel, pinch and Space pan work as in
/// the real workspaces. The preview arg builds a `PreviewStatus` the way an app's request
/// bookkeeping would, and the zoom bar shows its progress or, when idle, the note.
fn show_canvas(ui: &mut Ui, args: &mut CanvasArgs) {
    let shared = story::shared(ui, "canvas", Kept::new);
    let mut kept = story::lock(&shared);
    let Kept { dialog, view, tools, .. } = &mut *kept;
    let title = title(ui, view);
    let own = FullWindowStoryLabels::of(ui);
    let mut status = PreviewStatus::new();
    let ticket = status.request();
    match args.preview {
        PreviewArg::Idle => drop(status.ready(ticket)),
        PreviewArg::Pending => {}
        PreviewArg::HalfDone => status.report_progress(ticket, 0.5),
        PreviewArg::Failed => drop(status.failed(ticket, own.out_of_memory.as_str())),
    }
    let (tool, checkerboard, overlay, tint, zoom_bar, note) =
        (args.tool.tool(), args.checkerboard, args.overlay, args.tint, args.zoom_bar, args.note);
    let words = StoryLabels::of(ui);
    let labels = FullWindowLabels::of(ui);
    workspace(ui, "story-canvas", dialog, &mut args.open, vec2(args.width, args.height), title, |ui, fx| {
        FullWindowSide::new("canvas-preview").header(own.preview.as_str()).footer(footer(ui, false)).show(ui, fx, |ui| {
            ui.label(&words.image);
            status.show(ui, &labels);
        });
        if zoom_bar {
            let mut bar = FullWindowZoomBar::new("canvas-zoom", view).progress(status.progress());
            if note {
                bar = bar.note(own.preview_note.as_str());
            }
            bar.show(ui);
        }
        let mask = Coverage;
        let mut canvas = FullWindowCanvas::new("canvas-view", view).source(&Scene);
        if checkerboard {
            canvas = canvas.checkerboard(Checkerboard { size: CheckerSize::Medium, ..Checkerboard::default() });
        }
        if overlay {
            let mut shown = RasterOverlay::new(&mask, MASK_TEXEL);
            if tint {
                shown = shown.tint(Color32::from_rgba_unmultiplied(142, 208, 106, 128));
            }
            canvas = canvas.overlay(shown);
        }
        if let Some(tool) = tools.pick(tool) {
            canvas = canvas.tool(tool);
        }
        canvas.show(ui);
    });
}

// --- rail in a dialog ---

#[derive(StoryArgs, Debug, PartialEq)]
struct RailArgs {
    open: bool,
    #[arg(range = 1..=2)]
    columns: usize,
    #[arg(name = "pick by key")]
    pick_by_key: bool,
    #[arg(range = 360.0..=900.0)]
    width: f32,
    #[arg(range = 280.0..=700.0)]
    height: f32,
}

/// The `ToolRail` docked at the start edge of a `FullWindowDialog`. Letters pick tools while the
/// dialog is the top layer: B picks Brush, Shift+B cycles Brush and Pencil.
fn show_rail_in_dialog(ui: &mut Ui, args: &mut RailArgs) {
    let (columns, pick_by_key) = (args.columns, args.pick_by_key);
    let words = StoryLabels::of(ui);
    let shared = story::shared(ui, "rail-in-dialog", Kept::new);
    let mut kept = story::lock(&shared);
    let Kept { dialog, rail: rail_state, .. } = &mut *kept;
    workspace(ui, "story-rail-in-dialog", dialog, &mut args.open, vec2(args.width, args.height), words.tools.clone(), |ui, fx| {
        let current = rail(ui, "rail-in-dialog-tools", rail_state, columns, pick_by_key);
        let line = craft_i18n::fmt(&FullWindowStoryLabels::of(ui).current_tool, &[("tool", &tool_label(ui, current))]);
        FullWindowSide::new("rail-side").footer(footer(ui, false)).show(ui, fx, |ui| {
            ui.label(line);
        });
    });
}
