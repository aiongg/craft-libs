//! Stories of the Content-Aware Fill workspace, its panel and its options bar.
//!
//! The workspace stories run it over `MockImage` through a fake `FillSource`: the picture itself, a sampling plane
//! that covers the picture but a window round the house, and the fill area as a box over the house. The stroke and
//! the lasso in progress, the brush outline and the Alt + right-drag resize with its readout are the pointer's: draw
//! on the picture to see them.

use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use egui::Ui;

use craft_behavior::kurbo::Rect;
use craft_ui::canvas::RasterSource;
use craft_ui::contract::Bind;
use craft_ui::runtime::pending::Job;
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};

use super::{
    ContentAwareFillWorkspace, FillBar, FillLimits, FillOpen, FillPanel, FillPreviewRequest, FillSettings, FillSource, FillTool, FillToolOptions,
    SamplingMode, SamplingPlane,
};
use crate::ui::mock_image::MockImage;
use crate::ui::wide_angle::PreviewDone;

/// Document pixels per cell of the story planes.
const TEXEL: u32 = 8;
/// The story limits: PhotoCraft's.
const LIMITS: FillLimits = FillLimits { edits: 1000, points: 200_000 };
/// The share of the picture's width the fill area box covers, round the house.
const FILL_AREA_REACH: f64 = 0.16;
/// How long the loading story's job runs before it gives up, in seconds.
const LOADING_SECONDS: u64 = 3600;
/// How far along the loading story's job reports to be.
const LOADING_PROGRESS: f32 = 0.4;

/// What the story's preview job does.
#[derive(Clone, Copy, PartialEq)]
enum Preview {
    Loading,
    Failed,
}

/// The planes over `image` with the cells inside `inside` (document pixels) covered.
fn plane(image: MockImage, inside: impl Fn(f64, f64) -> bool) -> SamplingPlane {
    let size = image.size();
    let (columns, rows) = (size.width.div_ceil(TEXEL), size.height.div_ceil(TEXEL));
    let texel = f64::from(TEXEL);
    let bits =
        (0..rows * columns).map(|i| inside(f64::from(i % columns) * texel + texel / 2.0, f64::from(i / columns) * texel + texel / 2.0)).collect();
    SamplingPlane { origin: [0, 0], size: [columns, rows], texel: TEXEL, bits }
}

/// The app's side over `MockImage`.
struct StorySource {
    image: MockImage,
    sampling: SamplingPlane,
    fill_area: SamplingPlane,
    window: Rect,
    preview: Preview,
}

impl StorySource {
    fn new(image: MockImage, preview: Preview) -> Self {
        let bounds = image.subject_bounds();
        let centre = (f64::from(bounds.x0 + bounds.x1) / 2.0, f64::from(bounds.y0 + bounds.y1) / 2.0);
        let reach = FILL_AREA_REACH * f64::from(image.size().width);
        let in_box = move |x: f64, y: f64| (x - centre.0).abs() < reach && (y - centre.1).abs() < reach;
        Self {
            image,
            sampling: plane(image, |x, y| !in_box(x, y)),
            fill_area: plane(image, in_box),
            window: Rect::new(centre.0 - 2.0 * reach, centre.1 - 2.0 * reach, centre.0 + 2.0 * reach, centre.1 + 2.0 * reach),
            preview,
        }
    }
}

impl FillSource for StorySource {
    fn image(&self) -> &dyn RasterSource {
        &self.image
    }

    fn sampling(&self) -> Option<&SamplingPlane> {
        Some(&self.sampling)
    }

    fn fill_area(&self) -> Option<&SamplingPlane> {
        Some(&self.fill_area)
    }

    fn sampling_window(&self) -> Option<Rect> {
        Some(self.window)
    }

    fn preview(&self) -> Option<&dyn RasterSource> {
        Some(&self.image)
    }

    fn preview_job(&self, _: &FillPreviewRequest) -> Job<PreviewDone, String> {
        match self.preview {
            Preview::Loading => Job::new(|progress, cancel| {
                progress.report(LOADING_PROGRESS);
                for _ in 0..LOADING_SECONDS {
                    if cancel.is_cancelled() {
                        break;
                    }
                    std::thread::sleep(Duration::from_secs(1));
                }
                Ok(PreviewDone { timing: Duration::ZERO })
            }),
            Preview::Failed => Job::new(|_, _| Err("nothing to sample from: the sampling area has no pixels outside the fill area".to_owned())),
        }
    }
}

/// The workspace a story keeps between passes, compared by identity.
#[derive(Clone)]
struct Kept(Arc<Mutex<ContentAwareFillWorkspace>>);

impl PartialEq for Kept {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

/// Shows the workspace over the mock picture with `settings` and a preview job that `preview` says how it ends.
fn show_workspace(ui: &mut Ui, data: &mut StoryData, settings: FillSettings, preview: Preview) -> StoryOutput {
    let image = MockImage::default();
    let size = image.size();
    let open = FillOpen::new([size.width, size.height], "House", LIMITS).settings(settings);
    let kept = data.value("workspace", Kept(Arc::new(Mutex::new(ContentAwareFillWorkspace::new(open))))).clone();
    let mut workspace = kept.0.lock().unwrap_or_else(PoisonError::into_inner);
    workspace.show(ui, &StorySource::new(image, preview)).into()
}

#[story(ContentAwareFillWorkspace, covers = [Default], drive = Drive::Never)]
fn workspace_with_the_sampling_tint(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    show_workspace(ui, data, FillSettings::default(), Preview::Loading)
}

#[story(ContentAwareFillWorkspace, covers = [], drive = Drive::Never)]
fn workspace_with_the_fill_area_outlined(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    show_workspace(ui, data, FillSettings { show_overlay: false, sampling: SamplingMode::Custom, ..FillSettings::default() }, Preview::Loading)
}

#[story(ContentAwareFillWorkspace, covers = [], drive = Drive::Never)]
fn workspace_with_the_preview_loading(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    show_workspace(ui, data, FillSettings::default(), Preview::Loading)
}

#[story(ContentAwareFillWorkspace, covers = [], drive = Drive::Never)]
fn workspace_with_the_preview_failed(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    show_workspace(ui, data, FillSettings::default(), Preview::Failed)
}

#[story(FillPanel<'static>, covers = [Default], drive = Drive::Never)]
fn panel(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let settings = data.value("settings", FillSettings::default());
    FillPanel::new(Bind::bound(settings), SamplingMode::Auto).show(ui).into()
}

#[story(FillPanel<'static>, covers = [], drive = Drive::Never)]
fn panel_with_an_error(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let settings = data.value("settings", FillSettings::default());
    FillPanel::new(Bind::bound(settings), SamplingMode::Custom)
        .error(Some("at most 1000 sampling strokes: undo some, or reset the sampling area"))
        .show(ui)
        .into()
}

/// The bar of `tool` over options a story keeps.
fn bar(ui: &mut Ui, data: &mut StoryData, tool: FillTool) -> StoryOutput {
    let options = data.value(
        "options",
        FillToolOptions {
            brush_add: true,
            brush_size: FillOpen::BRUSH_SIZE,
            lasso: craft_behavior::geometry::selection_op::SelectionOperation::New,
            grow_by: FillToolOptions::GROW_BY,
        },
    );
    FillBar::new(tool, options).show(ui).into()
}

#[story(FillBar<'static>, covers = [Default], drive = Drive::Never)]
fn bar_of_the_sampling_brush(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    bar(ui, data, FillTool::SamplingBrush)
}

#[story(FillBar<'static>, covers = [], drive = Drive::Never)]
fn bar_of_the_lasso(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    bar(ui, data, FillTool::Lasso)
}
