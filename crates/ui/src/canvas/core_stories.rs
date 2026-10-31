//! Stories of the step 3 canvas core. Not yet in the canvas stories list: registering them would
//! change `tests/dumps/stories/registry.txt` and run their bodies in
//! `every_story_shows_with_its_default_args`. The fill task that lands the stories registers
//! them and regenerates the dump.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::Ui;

use super::story::ViewArgs;
use crate::story::{Level, Story, StoryArgs, StoryChoice, Surface};
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

/// A document on the story canvas with wheel, pinch, page keys, Space (Hand), Cmd+Space (Zoom),
/// the Zoom tool's hold and drag, Rotate View with its compass, and a zoom readout.
fn show_navigation(ui: &mut Ui, args: &mut NavigationArgs) {
    todo!()
}

/// A generated gradient document through `RasterDisplay` over the checkerboard, with the pixel
/// grid; `large` makes it 6000 × 4000 so the detail texture shows at 100 %.
fn show_raster(ui: &mut Ui, args: &mut RasterArgs) {
    todo!()
}

fn show_pasteboard(ui: &mut Ui, args: &mut PasteboardArgs) {
    todo!()
}

fn show_split(ui: &mut Ui, args: &mut SplitArgs) {
    todo!()
}

/// Small demo tools on `ToolHost`: a marquee (autoscroll, snapping, readout), the Hand, the Zoom
/// tool (click, hold, scrub) and Rotate View.
fn show_tool_host(ui: &mut Ui, args: &mut ToolHostArgs) {
    todo!()
}
