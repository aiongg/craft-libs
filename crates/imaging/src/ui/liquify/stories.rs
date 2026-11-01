//! Stories of the Liquify workspace (task ST-WS1): not registered. Registering them (task RG2)
//! adds `liquify::stories` to the crate's `register!` list, `LiquifyWorkspace` to `COMPONENTS` and
//! [`LiquifyStoryLabels`] to `WORDS`, and regenerates the registry dump.
//!
//! The stories run the workspace over `MockImage` (`ui::mock_image`) through a fake
//! `LiquifySource`: the picture itself (the strokes only reach the app's field, which the stories
//! do not have), the house as the freeze mask, a backdrop in grey and a gently displaced mesh. The
//! workspace covers the gallery window, as it does in an app; the `open` arg (or the Overview
//! tile's button) shows it. An arg that changes the settings opens a fresh workspace. The stroke in
//! progress, the brush outline and the Alt + right-drag resize with its readout are the pointer's:
//! draw on the picture to see them. Every string goes through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `patterns/liquify/tools` | The tools in their groups; the picked tool and the brush (args) |
//! | `patterns/liquify/view-options` | The mesh at the three sizes, the mask overlay, the backdrop in the three modes |

#![expect(dead_code, reason = "task RG2 registers the stories")]

use std::sync::{Arc, Mutex};

use craft_ui::canvas::kurbo::Point;
use craft_ui::canvas::{PixelRect, RasterSource};
use craft_ui::dialog::Outcome;
use craft_ui::localized::Labels as _;
use craft_ui::story::{Level, Story, StoryArgs, StoryChoice, StoryLabels, Surface, dialog_closed, dialog_shown};
use craft_ui::typed_story;
use egui::{Color32, ColorImage, Ui};

use super::{
    BackdropMode, BackdropUse, LiquifyBrush, LiquifyMesh, LiquifyOpen, LiquifySettings, LiquifySource, LiquifyTool, LiquifyWorkspace, MeshSize,
};
use crate::ui::mock_image::MockImage;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["LiquifyWorkspace"];

craft_ui::words! {
    /// The captions of the Liquify stories, English by default.
    #[non_exhaustive]
    pub(crate) struct LiquifyStoryLabels {
        layer_sky: "liquifyStory.layerSky" = "Sky";
        layer_house: "liquifyStory.layerHouse" = "House";
    }
}

/// The freeze mask's pixels per texel: coarse, since the mask is soft.
const MASK_TEXEL: u32 = 4;
/// The mesh's node spacing in document pixels at Small, Medium and Large (a story's picture is
/// 1600 px wide).
const MESH_SPACING: [f64; 3] = [32.0, 64.0, 128.0];
/// The displacement the mesh shows, as a share of the picture's width, and the reach of its bulge
/// round the house as a share of the picture's width.
const BULGE: f64 = 0.03;
const BULGE_REACH: f64 = 0.18;
/// The ids of the two layers the backdrop can use.
const LAYER_SKY: u64 = 1;
const LAYER_HOUSE: u64 = 2;

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "patterns/liquify/tools",
            Level::Patterns,
            "LiquifyWorkspace",
            "Tools",
            ToolsArgs { open: true, tool: ToolArg::ForwardWarp, size: 100.0, density: 50.0, pressure: 100.0, rate: 80.0 },
            show_tools
        )
        .on(Surface::Own),
        typed_story!(
            "patterns/liquify/view-options",
            Level::Patterns,
            "LiquifyWorkspace",
            "View options",
            ViewArgs {
                open: true,
                show_mesh: true,
                mesh_size: MeshArg::Medium,
                show_mask: true,
                show_backdrop: true,
                mode: ModeArg::InFront,
                opacity: 50.0,
                layer: LayerArg::AllLayers,
            },
            show_view_options
        )
        .on(Surface::Own),
    ]
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum ToolArg {
    #[choice(name = "Forward Warp")]
    ForwardWarp,
    Reconstruct,
    Smooth,
    #[choice(name = "Twirl Clockwise")]
    TwirlClockwise,
    #[choice(name = "Twirl Counterclockwise")]
    TwirlCounterclockwise,
    Pucker,
    Bloat,
    #[choice(name = "Push Left")]
    PushLeft,
    Freeze,
    Thaw,
    #[choice(name = "Freeze Lasso")]
    FreezeLasso,
    Hand,
    Zoom,
}

impl ToolArg {
    fn tool(self) -> LiquifyTool {
        match self {
            ToolArg::ForwardWarp => LiquifyTool::ForwardWarp,
            ToolArg::Reconstruct => LiquifyTool::Reconstruct,
            ToolArg::Smooth => LiquifyTool::Smooth,
            ToolArg::TwirlClockwise => LiquifyTool::TwirlClockwise,
            ToolArg::TwirlCounterclockwise => LiquifyTool::TwirlCounterclockwise,
            ToolArg::Pucker => LiquifyTool::Pucker,
            ToolArg::Bloat => LiquifyTool::Bloat,
            ToolArg::PushLeft => LiquifyTool::PushLeft,
            ToolArg::Freeze => LiquifyTool::Freeze,
            ToolArg::Thaw => LiquifyTool::Thaw,
            ToolArg::FreezeLasso => LiquifyTool::FreezeLasso,
            ToolArg::Hand => LiquifyTool::Hand,
            ToolArg::Zoom => LiquifyTool::Zoom,
        }
    }
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum MeshArg {
    Small,
    Medium,
    Large,
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum ModeArg {
    #[choice(name = "In Front")]
    InFront,
    Behind,
    Blend,
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum LayerArg {
    #[choice(name = "All Layers")]
    AllLayers,
    Sky,
    House,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct ToolsArgs {
    /// Whether the workspace is open.
    open: bool,
    /// The picked tool.
    tool: ToolArg,
    /// The brush diameter in document pixels.
    #[arg(range = 1.0..=3000.0)]
    size: f64,
    /// The brush density in percent.
    #[arg(range = 0.0..=100.0)]
    density: f64,
    /// The brush pressure in percent.
    #[arg(range = 0.0..=100.0)]
    pressure: f64,
    /// The brush rate in percent.
    #[arg(range = 0.0..=100.0)]
    rate: f64,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct ViewArgs {
    /// Whether the workspace is open.
    open: bool,
    show_mesh: bool,
    mesh_size: MeshArg,
    show_mask: bool,
    show_backdrop: bool,
    /// Show Backdrop › Mode.
    mode: ModeArg,
    /// Show Backdrop › Opacity in percent.
    #[arg(range = 0.0..=100.0)]
    opacity: f64,
    /// Show Backdrop › Use.
    layer: LayerArg,
}

// --- the fake app ---

/// The freeze mask: the house, white with the frozen amount as alpha.
struct FreezeMask(MockImage);

impl RasterSource for FreezeMask {
    fn size(&self) -> [u32; 2] {
        self.0.size().map(|v| v.div_ceil(MASK_TEXEL))
    }

    fn revision(&self) -> u64 {
        self.0.revision()
    }

    fn read(&self, region: PixelRect, factor: u32, out: &mut ColorImage) {
        let [columns, rows] = out.size;
        let (factor, texel) = (f64::from(factor.max(1)), f64::from(MASK_TEXEL));
        for row in 0..rows {
            let y = (f64::from(region.y0) + (row as f64 + 0.5) * factor) * texel;
            for column in 0..columns {
                let x = (f64::from(region.x0) + (column as f64 + 0.5) * factor) * texel;
                out.pixels[row * columns + column] = Color32::from_white_alpha((self.0.subject(x, y) * 255.0).round() as u8);
            }
        }
    }
}

/// The picture in grey: the backdrop is another rendering of the same document.
struct GreyBackdrop(MockImage);

impl RasterSource for GreyBackdrop {
    fn size(&self) -> [u32; 2] {
        self.0.size()
    }

    fn revision(&self) -> u64 {
        self.0.revision()
    }

    fn read(&self, region: PixelRect, factor: u32, out: &mut ColorImage) {
        self.0.read(region, factor, out);
        for pixel in &mut out.pixels {
            let [r, g, b, a] = pixel.to_array();
            let grey = ((u32::from(r) * 30 + u32::from(g) * 59 + u32::from(b) * 11) / 100) as u8;
            *pixel = Color32::from_rgba_unmultiplied(grey, grey, grey, a);
        }
    }
}

/// The app's side over `MockImage`.
struct StorySource {
    image: MockImage,
    mask: FreezeMask,
    backdrop: GreyBackdrop,
    layers: [(u64, String); 2],
}

impl StorySource {
    fn new(image: MockImage, labels: &LiquifyStoryLabels) -> Self {
        Self {
            image,
            mask: FreezeMask(image),
            backdrop: GreyBackdrop(image),
            layers: [(LAYER_SKY, labels.layer_sky.clone()), (LAYER_HOUSE, labels.layer_house.clone())],
        }
    }
}

impl LiquifySource for StorySource {
    fn image(&self) -> &dyn RasterSource {
        &self.image
    }

    fn freeze_mask(&self) -> Option<(&dyn RasterSource, u32)> {
        Some((&self.mask, MASK_TEXEL))
    }

    fn mesh(&self, size: MeshSize) -> Option<LiquifyMesh> {
        let [width, height] = self.image.size().map(f64::from);
        let spacing = MESH_SPACING[MeshSize::ALL.iter().position(|s| *s == size).unwrap_or(0)];
        let columns = (width / spacing).ceil() as usize + 1;
        let rows = (height / spacing).ceil() as usize + 1;
        let bounds = self.image.subject_bounds();
        let centre = Point::new(f64::from(bounds.x0 + bounds.x1) / 2.0, f64::from(bounds.y0 + bounds.y1) / 2.0);
        let (bulge, reach) = (BULGE * width, BULGE_REACH * width);
        // A Bloat over the house: the nodes move away from its middle, most near it.
        let points = (0..rows * columns)
            .map(|i| {
                let p = Point::new((i % columns) as f64 * spacing, (i / columns) as f64 * spacing);
                let (dx, dy) = (p.x - centre.x, p.y - centre.y);
                let pull = (-(dx * dx + dy * dy) / (2.0 * reach * reach)).exp();
                let length = dx.hypot(dy).max(1.0);
                Point::new(p.x + dx / length * bulge * pull, p.y + dy / length * bulge * pull)
            })
            .collect();
        Some(LiquifyMesh { columns, rows, points })
    }

    fn backdrop(&self) -> Option<&dyn RasterSource> {
        Some(&self.backdrop)
    }

    fn layers(&self) -> Vec<(u64, String)> {
        self.layers.to_vec()
    }
}

// --- the kept workspace ---

/// The workspace a story keeps between passes, with the settings it was opened with: other
/// settings open a fresh one.
struct Kept {
    seed: LiquifySettings,
    workspace: LiquifyWorkspace,
}

type Slot = Arc<Mutex<Option<Kept>>>;

/// Shows the workspace with `settings` while the story is open.
fn show_workspace(ui: &mut Ui, open: &mut bool, name: &'static str, settings: LiquifySettings) {
    let id = ui.make_persistent_id(("craft_imaging::liquify::story", name));
    let slot: Slot = ui.data_mut(|d| d.get_temp_mut_or_insert_with::<Slot>(id, Slot::default).clone());
    let Ok(mut kept) = slot.lock() else { return };
    if !dialog_shown(ui, *open) {
        *kept = None;
        return;
    }
    let image = MockImage::default();
    if kept.as_ref().is_none_or(|k| k.seed != settings) {
        let subject = StoryLabels::of(ui).photo;
        let open = LiquifyOpen::new(image.size(), subject).settings(settings.clone());
        *kept = Some(Kept { seed: settings, workspace: LiquifyWorkspace::new(open) });
    }
    let source = StorySource::new(image, &LiquifyStoryLabels::of(ui));
    if let Some(kept) = kept.as_mut() {
        let out = kept.workspace.show(ui.ctx(), &source);
        if out.outcome.is_some_and(Outcome::closes) {
            dialog_closed(ui, open);
        }
    }
}

fn show_tools(ui: &mut Ui, args: &mut ToolsArgs) {
    let settings = LiquifySettings {
        tool: args.tool.tool(),
        brush: LiquifyBrush { size: args.size, density: args.density, pressure: args.pressure, rate: args.rate },
        ..LiquifySettings::default()
    };
    show_workspace(ui, &mut args.open, "tools", settings);
}

fn show_view_options(ui: &mut Ui, args: &mut ViewArgs) {
    let settings = LiquifySettings {
        show_mesh: args.show_mesh,
        mesh_size: match args.mesh_size {
            MeshArg::Small => MeshSize::Small,
            MeshArg::Medium => MeshSize::Medium,
            MeshArg::Large => MeshSize::Large,
        },
        show_mask: args.show_mask,
        show_backdrop: args.show_backdrop,
        backdrop: match args.layer {
            LayerArg::AllLayers => BackdropUse::AllLayers,
            LayerArg::Sky => BackdropUse::Layer(LAYER_SKY),
            LayerArg::House => BackdropUse::Layer(LAYER_HOUSE),
        },
        backdrop_mode: match args.mode {
            ModeArg::InFront => BackdropMode::InFront,
            ModeArg::Behind => BackdropMode::Behind,
            ModeArg::Blend => BackdropMode::Blend,
        },
        backdrop_opacity: args.opacity,
        ..LiquifySettings::default()
    };
    show_workspace(ui, &mut args.open, "view-options", settings);
}
