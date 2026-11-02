//! Stories of the Content-Aware Fill workspace (task ST-WS2): not registered. Registering them
//! (task RG2) adds `fill::stories` to the crate's `register!` list,
//! `ContentAwareFillWorkspace` to `COMPONENTS` and [`FillStoryLabels`] to `WORDS`, and
//! regenerates the registry dump.
//!
//! The stories run over `MockImage` through [`MockFill`], a fake `FillSource`: the fill area is
//! the house's rectangle changed by the session's lasso edits, the sampling plane is built from
//! the sampling option and the brush strokes, the sampling window is the fill area with a margin,
//! and the preview is the image shifted sideways (no engine). Both planes are story texels of
//! [`TEXEL`] document pixels. The workspace asks for a preview and the story answers at once
//! (or never, or with a failure, as the `preview` arg says). `brush` and `preview` run the
//! workspace as a dialog; `lasso` runs the options bar and the canvas tool in a frame of the
//! story, because the workspace opens on the Sampling Brush and keeps its tool private. Every
//! string goes through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `patterns/content-aware-fill/brush` | The sampling tint, the three sampling options, Indicates both ways |
//! | `patterns/content-aware-fill/lasso` | The fill area's ants, the four operations, Expand and Contract |
//! | `patterns/content-aware-fill/preview` | The Preview panel framed on the sampling window, recomputing, failed |

use craft_ui::canvas::kurbo;
use craft_ui::canvas::{Checkerboard, PixelRect, RasterSource, SelectionOperation};
use craft_ui::dialog::full_window::FullWindowCanvas;
use craft_ui::localized::Labels as _;
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, Story, StoryArgs, StoryChoice, Surface, claim, dialog_closed, dialog_shown, lock, shared};
use craft_ui::typed_story;
use egui::{Color32, ColorImage, Ui, vec2};

use super::{
    ContentAwareFillWorkspace, FillAreaEdit, FillBar, FillCanvas, FillLimits, FillOpen, FillOverlay, FillSession, FillSettings, FillSource, FillTool,
    FillToolOptions, SamplingMode, SamplingPlane,
};
use crate::ui::mock_image::MockImage;
use crate::ui::select_mask::EditMode;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["ContentAwareFillWorkspace"];

craft_ui::words! {
    /// The captions of the Content-Aware Fill stories, English by default.
    #[non_exhaustive]
    pub(crate) struct FillStoryLabels {
        layer: "fillStory.layer" = "Layer 1";
        failed: "fillStory.failed" = "Not enough memory for the preview";
        hint_lasso: "fillStory.hintLasso" = "Draw around an area to change the fill area; Shift adds, Alt subtracts.";
    }
}

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "patterns/content-aware-fill/brush",
            Level::Patterns,
            "ContentAwareFillWorkspace",
            "Brush",
            BrushArgs { open: true, sampling: SamplingArg::Auto, indicates: IndicatesArg::SamplingArea, show_tint: true, opacity: 50.0 },
            show_brush
        )
        .on(Surface::Canvas),
        typed_story!(
            "patterns/content-aware-fill/lasso",
            Level::Patterns,
            "ContentAwareFillWorkspace",
            "Lasso",
            LassoArgs { operation: OperationArg::New, grow_by: 12.0 },
            show_lasso
        )
        .on(Surface::Canvas),
        typed_story!(
            "patterns/content-aware-fill/preview",
            Level::Patterns,
            "ContentAwareFillWorkspace",
            "Preview",
            PreviewArgs { open: true, preview: PreviewArg::Ready },
            show_preview
        )
        .on(Surface::Canvas),
    ]
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum SamplingArg {
    Auto,
    Rectangular,
    Custom,
}

impl SamplingArg {
    fn mode(self) -> SamplingMode {
        match self {
            SamplingArg::Auto => SamplingMode::Auto,
            SamplingArg::Rectangular => SamplingMode::Rectangular,
            SamplingArg::Custom => SamplingMode::Custom,
        }
    }
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum IndicatesArg {
    #[choice(name = "Sampling Area")]
    SamplingArea,
    #[choice(name = "Excluded Area")]
    ExcludedArea,
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

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum PreviewArg {
    Ready,
    Recomputing,
    Failed,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct BrushArgs {
    open: bool,
    /// The sampling option.
    sampling: SamplingArg,
    /// Indicates: the tint marks the sampled or the left-out area.
    indicates: IndicatesArg,
    #[arg(name = "show tint")]
    show_tint: bool,
    /// The tint's opacity, in percent.
    #[arg(range = 0.0..=100.0)]
    opacity: f64,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct LassoArgs {
    /// The Lasso's operation in the bar.
    operation: OperationArg,
    /// Expand and Contract's amount, in document pixels.
    #[arg(name = "grow by", range = 1.0..=100.0)]
    grow_by: f64,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct PreviewArgs {
    open: bool,
    /// How the story answers the workspace's preview request.
    preview: PreviewArg,
}

/// Document pixels per texel of the story's planes (a coarse proxy, as the app's).
const TEXEL: u32 = 16;
/// The sampling window's margin round the fill area, as a fraction of the area's size.
const WINDOW_MARGIN: f64 = 0.5;
/// The preview's sideways shift, in document pixels.
const SHIFT: f64 = 140.0;
/// The limits the stories give the workspace (the engine's).
const LIMITS: FillLimits = FillLimits { edits: 1000, points: 200_000 };
/// The lasso frame's height as a fraction of its width.
const FRAME_RATIO: f32 = 0.6;

/// The image shifted sideways: the story's preview.
struct Shifted(MockImage);

impl RasterSource for Shifted {
    fn size(&self) -> [u32; 2] {
        self.0.size()
    }

    fn revision(&self) -> u64 {
        self.0.revision()
    }

    fn read(&self, region: PixelRect, factor: u32, out: &mut ColorImage) {
        let [columns, rows] = out.size;
        let factor = f64::from(factor.max(1));
        for row in 0..rows {
            let y = f64::from(region.y0) + (row as f64 + 0.5) * factor;
            for column in 0..columns {
                let x = f64::from(region.x0) + (column as f64 + 0.5) * factor;
                let [r, g, b, a] = self.0.pixel(x - SHIFT, y);
                out.pixels[row * columns + column] = Color32::from_rgba_unmultiplied(r, g, b, a);
            }
        }
    }
}

/// A fake `FillSource` for a session.
struct MockFill {
    image: MockImage,
    preview: Shifted,
    sampling: SamplingPlane,
    fill_area: SamplingPlane,
    window: kurbo::Rect,
}

impl FillSource for MockFill {
    fn image(&self) -> &dyn RasterSource {
        &self.image
    }

    fn sampling(&self) -> Option<&SamplingPlane> {
        Some(&self.sampling)
    }

    fn fill_area(&self) -> Option<&SamplingPlane> {
        Some(&self.fill_area)
    }

    fn sampling_window(&self) -> Option<kurbo::Rect> {
        Some(self.window)
    }

    fn preview(&self) -> Option<&dyn RasterSource> {
        Some(&self.preview)
    }
}

fn texel_centre(column: u32, row: u32) -> [f64; 2] {
    let t = f64::from(TEXEL);
    [(f64::from(column) + 0.5) * t, (f64::from(row) + 0.5) * t]
}

/// Even-odd point in polygon.
fn inside(points: &[[f64; 2]], at: [f64; 2]) -> bool {
    let mut odd = false;
    for (i, a) in points.iter().enumerate() {
        let b = points[(i + 1) % points.len()];
        if (a[1] > at[1]) != (b[1] > at[1]) && at[0] < (b[0] - a[0]) * (at[1] - a[1]) / (b[1] - a[1]) + a[0] {
            odd = !odd;
        }
    }
    odd
}

/// The distance from `p` to the segment `a`..`b`.
fn segment_distance(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let length = dx * dx + dy * dy;
    let t = if length == 0.0 { 0.0 } else { (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / length).clamp(0.0, 1.0) };
    (p[0] - a[0] - t * dx).hypot(p[1] - a[1] - t * dy)
}

/// Every set cell's neighbours within `radius` cells set (`grow`), or the cells whose whole
/// neighbourhood is set (shrink).
fn grow(bits: &[bool], cells: [u32; 2], radius: i32, grow: bool) -> Vec<bool> {
    let [columns, rows] = cells.map(|v| v as i32);
    (0..rows)
        .flat_map(|y| (0..columns).map(move |x| (x, y)))
        .map(|(x, y)| {
            let mut any_set = false;
            let mut any_clear = false;
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    if dx * dx + dy * dy > radius * radius {
                        continue;
                    }
                    let (nx, ny) = (x + dx, y + dy);
                    let set = nx >= 0 && ny >= 0 && nx < columns && ny < rows && bits[(ny * columns + nx) as usize];
                    any_set |= set;
                    any_clear |= !set;
                }
            }
            if grow { any_set } else { !any_clear }
        })
        .collect()
}

impl MockFill {
    /// The planes for `session` over the house's rectangle.
    fn of(session: &FillSession, mode: SamplingMode) -> Self {
        let image = MockImage::default();
        let cells = image.size().map(|v| v.div_ceil(TEXEL));
        let at = |i: usize| texel_centre(i as u32 % cells[0], i as u32 / cells[0]);
        let count = (cells[0] * cells[1]) as usize;

        let house = image.subject_bounds();
        let mut area: Vec<bool> = (0..count)
            .map(|i| {
                let [x, y] = at(i);
                (f64::from(house.x0)..f64::from(house.x1)).contains(&x) && (f64::from(house.y0)..f64::from(house.y1)).contains(&y)
            })
            .collect();
        for edit in &session.fill_edits {
            match edit {
                FillAreaEdit::New { points } => area = (0..count).map(|i| inside(points, at(i))).collect(),
                FillAreaEdit::Add { points } => area = area.iter().enumerate().map(|(i, &set)| set || inside(points, at(i))).collect(),
                FillAreaEdit::Subtract { points } => area = area.iter().enumerate().map(|(i, &set)| set && !inside(points, at(i))).collect(),
                FillAreaEdit::Intersect { points } => area = area.iter().enumerate().map(|(i, &set)| set && inside(points, at(i))).collect(),
                FillAreaEdit::Expand { by } | FillAreaEdit::Contract { by } => {
                    let radius = ((by / f64::from(TEXEL)).ceil() as i32).max(1);
                    area = grow(&area, cells, radius, matches!(edit, FillAreaEdit::Expand { .. }));
                }
            }
        }

        // The fill area's bounds, and the window round them.
        let t = f64::from(TEXEL);
        let set: Vec<[f64; 2]> = (0..count).filter(|&i| area[i]).map(at).collect();
        let (min, max) =
            set.iter().fold(([f64::MAX; 2], [f64::MIN; 2]), |(lo, hi), p| ([lo[0].min(p[0]), lo[1].min(p[1])], [hi[0].max(p[0]), hi[1].max(p[1])]));
        let bounds = if set.is_empty() {
            kurbo::Rect::new(f64::from(house.x0), f64::from(house.y0), f64::from(house.x1), f64::from(house.y1))
        } else {
            kurbo::Rect::new(min[0] - t / 2.0, min[1] - t / 2.0, max[0] + t / 2.0, max[1] + t / 2.0)
        };
        let [w, h] = image.size().map(f64::from);
        let window = kurbo::Rect::new(
            (bounds.x0 - bounds.width() * WINDOW_MARGIN).max(0.0),
            (bounds.y0 - bounds.height() * WINDOW_MARGIN).max(0.0),
            (bounds.x1 + bounds.width() * WINDOW_MARGIN).min(w),
            (bounds.y1 + bounds.height() * WINDOW_MARGIN).min(h),
        );

        // The sampling area: the option's start, then the strokes in order.
        let mut sampled: Vec<bool> = (0..count)
            .map(|i| {
                let [x, y] = at(i);
                match mode {
                    SamplingMode::Auto => true,
                    SamplingMode::Rectangular => (window.x0..window.x1).contains(&x) && (window.y0..window.y1).contains(&y),
                    SamplingMode::Custom => false,
                }
            })
            .collect();
        for stroke in &session.strokes {
            let on = stroke.mode == EditMode::Add;
            let reach = stroke.size / 2.0;
            for (i, cell) in sampled.iter_mut().enumerate() {
                let p = at(i);
                let near = match stroke.points.as_slice() {
                    [] => false,
                    [only] => (p[0] - only[0]).hypot(p[1] - only[1]) <= reach,
                    points => points.windows(2).any(|pair| segment_distance(p, pair[0], pair[1]) <= reach),
                };
                if near {
                    *cell = on;
                }
            }
        }

        let plane = |bits| SamplingPlane { origin: [0, 0], size: cells, texel: TEXEL, bits };
        MockFill { image, preview: Shifted(image), sampling: plane(sampled), fill_area: plane(area), window }
    }
}

/// What a workspace story keeps: the workspace built from `signature`, and the planes of the
/// session they were made for.
struct Held {
    workspace: ContentAwareFillWorkspace,
    signature: (SamplingMode, bool, u64, bool),
    source: MockFill,
    planned: (FillSession, SamplingMode),
}

impl Held {
    fn new(settings: &FillSettings, subject: &str, signature: (SamplingMode, bool, u64, bool)) -> Self {
        let size = MockImage::default().size();
        let workspace = ContentAwareFillWorkspace::new(FillOpen::new(size, subject, LIMITS).settings(settings.clone()));
        let planned = (workspace.session().clone(), settings.sampling);
        Held { source: MockFill::of(&planned.0, planned.1), workspace, signature, planned }
    }
}

/// Runs the workspace as a dialog with `settings`, answering its preview requests as `preview`
/// says.
fn run_workspace(ui: &mut Ui, salt: &'static str, open: &mut bool, settings: FillSettings, preview: PreviewArg) {
    let words = FillStoryLabels::of(ui);
    if !(dialog_shown(ui, *open) && claim(ui.ctx(), "fill", ui.scope_id())) {
        return;
    }
    let signature = (settings.sampling, settings.excluded, settings.opacity.to_bits(), settings.show_overlay);
    let held = shared(ui, salt, || Held::new(&settings, &words.layer, signature));
    let mut held = lock(&held);
    if held.signature != signature {
        *held = Held::new(&settings, &words.layer, signature);
    }
    let ctx = ui.ctx().clone();
    let Held { workspace, source, planned, .. } = &mut *held;
    let out = workspace.show(&ctx, source);
    if let Some(ticket) = out.preview_wanted {
        match preview {
            PreviewArg::Ready => workspace.preview_ready(ticket),
            PreviewArg::Recomputing => {}
            PreviewArg::Failed => workspace.preview_failed(ticket, words.failed.clone()),
        }
    }
    let now = (workspace.session().clone(), workspace.settings().sampling);
    if *planned != now {
        *source = MockFill::of(&now.0, now.1);
        *planned = now;
    }
    if out.outcome.is_some() {
        // The next opening starts again.
        *held = Held::new(&settings, &words.layer, signature);
        dialog_closed(ui, open);
    }
}

fn show_brush(ui: &mut Ui, args: &mut BrushArgs) {
    let settings = FillSettings {
        sampling: args.sampling.mode(),
        excluded: args.indicates == IndicatesArg::ExcludedArea,
        show_overlay: args.show_tint,
        opacity: args.opacity,
        ..FillSettings::default()
    };
    run_workspace(ui, "brush", &mut args.open, settings, PreviewArg::Ready);
}

fn show_preview(ui: &mut Ui, args: &mut PreviewArgs) {
    let preview = args.preview;
    run_workspace(ui, "preview", &mut args.open, FillSettings::default(), preview);
}

/// What the lasso story keeps.
struct Lasso {
    session: FillSession,
    options: FillToolOptions,
    canvas: FillCanvas,
    view: craft_ui::dialog::full_window::FullWindowView,
}

fn show_lasso(ui: &mut Ui, args: &mut LassoArgs) {
    let words = FillStoryLabels::of(ui);
    let state = shared(ui, "lasso", || Lasso {
        session: FillSession::default(),
        options: FillToolOptions { brush_add: false, brush_size: 20.0, lasso: SelectionOperation::New, grow_by: 3.0 },
        canvas: FillCanvas::new(),
        view: craft_ui::dialog::full_window::FullWindowView::new(),
    });
    let mut state = lock(&state);
    let Lasso { session, options, canvas, view } = &mut *state;
    options.lasso = args.operation.operation();
    options.grow_by = args.grow_by;

    let bar = FillBar::new("fill-story-bar", FillTool::Lasso, options).show(ui);
    if let Some(edit) = bar.edit {
        drop(session.add_fill_edit(edit, &LIMITS));
    }
    if bar.reset_fill_area {
        session.fill_edits.clear();
    }
    ui.add(Text::new(words.hint_lasso.as_str()).content(Content::Muted));

    let source = MockFill::of(session, SamplingMode::Auto);
    let width = ui.available_width();
    ui.allocate_ui(vec2(width, width * FRAME_RATIO), |ui| {
        let mut tool = canvas.tool(FillTool::Lasso, options, session, &LIMITS);
        let shown =
            FullWindowCanvas::new("fill-story-lasso", view).source(&source.image).checkerboard(Checkerboard::default()).tool(&mut tool).show(ui);
        FillOverlay::new("fill-story-ants", None, Some(&source.fill_area)).tint(false).document(source.image.size()).show(ui, &shown.view);
    });
    for event in canvas.take_events() {
        if let super::FillEvent::FillArea(edit) = event {
            drop(session.add_fill_edit(edit, &LIMITS));
        }
    }
}
