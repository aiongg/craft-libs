//! The exact UI drawn to an image without a window (`screen-testing` feature, desktop), for tests,
//! reports and the CLI.
//!
//! [`Render`] builds an egui_kittest harness of a given size and scale on the process's one wgpu
//! device ([`wgpu_renderer`](super::wgpu_renderer)), runs the app's UI until it settles and hands
//! back an [`egui::ColorImage`], the type craft-automation's screenshot reply encodes. The harness
//! answers `ViewportCommand::Screenshot` itself, so an app's own screenshot path runs too.
//! [`Screens`](super::Screens) is the per-language variant; [`image_of`] renders any harness,
//! one of those included.
//!
//! ```ignore
//! use craft_ui::testing::render::Render;
//!
//! let render = Render::new([1440.0, 900.0]).pixels_per_point(2.0).setup(|ctx| app_theme(ctx));
//! let image = render.image(|ui| about_dialog(ui))?;
//!
//! // A whole app: logic and ui in one closure, as the app's frame calls them.
//! let mut harness = render.harness(|ui, app: &mut App| { app.logic(ui.ctx()); app.ui(ui) }, app)?;
//! harness.pump_idle(Duration::from_secs(10), |h| h.state().busy());
//! let image = craft_ui::testing::render::image_of(&mut harness)?;
//! ```
//!
//! On a machine without a GPU, wgpu needs a software adapter (lavapipe on Linux CI).

use std::rc::Rc;

use egui::{ColorImage, Context, Ui, Vec2};
use egui_kittest::{Harness, HarnessBuilder, LazyRenderer};

/// The longest image edge in pixels.
// lightcraft: crates/ui-egui/src/softpaint.rs:134
pub const MAX_IMAGE_EDGE: usize = 16_384;
/// The most pixels in one image.
// lightcraft: crates/ui-egui/src/softpaint.rs:135
pub const MAX_IMAGE_PIXELS: usize = 64_000_000;
/// Frames [`Render::image`] runs before it gives up on the UI settling.
// pc: crates/ui-egui/src/panels/layer_tree_ui/tests.rs:30 (with_max_steps(64))
pub const MAX_STEPS: u64 = 64;

/// Why no image came.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RenderError {
    /// The size or scale can't make an image. The message is LightCraft's, word for word:
    /// `viewport dimensions and scale must be finite and positive`,
    /// `scaled viewport edges must be between 1 and 16384 pixels`, or
    /// `scaled viewport exceeds 64000000 pixels`.
    Size(String),
    /// The UI still asked for frames after the step limit:
    /// `the UI did not settle within {steps} frames`.
    NotSettled {
        /// The step limit that was exceeded.
        steps: u64,
    },
    /// wgpu failed (no adapter, a lost device): `rendering failed: {kittest's message}`.
    Renderer(String),
}

impl std::fmt::Display for RenderError {
    /// The messages each variant's doc gives.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // new: test and CLI output, not interface text
        match self {
            Self::Size(message) => f.write_str(message),
            Self::NotSettled { steps } => write!(f, "the UI did not settle within {steps} frames"),
            Self::Renderer(message) => write!(f, "rendering failed: {message}"),
        }
    }
}

impl std::error::Error for RenderError {}

type Setup = Rc<dyn Fn(&Context)>;

/// Images of a UI at one size and scale. Build it once, then take an [`Self::image`] or a
/// [`Self::harness`] per case.
#[derive(Clone)]
pub struct Render {
    size: Vec2,
    pixels_per_point: f32,
    max_steps: u64,
    max_texture_side: Option<usize>,
    setup: Option<Setup>,
}

impl Render {
    /// A render of `size` points at scale 1, with up to [`MAX_STEPS`] frames to settle.
    pub fn new(size: impl Into<Vec2>) -> Self {
        // new: the builder Screens uses (testing/screens.rs:102), one size and scale
        Self { size: size.into(), pixels_per_point: 1.0, max_steps: MAX_STEPS, max_texture_side: None, setup: None }
    }

    /// Physical pixels per point (2.0 for a Retina screenshot).
    pub fn pixels_per_point(mut self, pixels_per_point: f32) -> Self {
        // pc: crates/ui-egui/src/panels/layer_tree_ui/tests.rs:30
        self.pixels_per_point = pixels_per_point;
        self
    }

    /// Frames to wait for the UI to settle before [`RenderError::NotSettled`].
    pub fn max_steps(mut self, max_steps: u64) -> Self {
        // pc: crates/ui-egui/src/panels/layer_tree_ui/tests.rs:30
        self.max_steps = max_steps;
        self
    }

    /// The largest texture the app may upload (a canvas drawing a whole document as one
    /// texture). Applied from the second frame on: the builder runs the first frame before the
    /// harness input can be changed.
    pub fn max_texture_side(mut self, pixels: usize) -> Self {
        // designcraft: crates/ui-egui/src/lib.rs:356
        self.max_texture_side = Some(pixels);
        self
    }

    /// Called on the context before the first frame: the app's fonts, theme and anything else it
    /// stores on the context.
    pub fn setup(mut self, setup: impl Fn(&Context) + 'static) -> Self {
        // new: as Screens::setup (testing/screens.rs:131) without the locale
        self.setup = Some(Rc::new(setup));
        self
    }

    /// A kittest builder with this size, scale, step limit, setup and the shared wgpu renderer,
    /// for apps that build their own harness (`build_eframe`). Checks the size first.
    pub fn builder<State>(&self) -> Result<HarnessBuilder<State>, RenderError> {
        // new: Screens::harness_state's builder chain (testing/screens.rs:152)
        viewport_pixels(self.size, self.pixels_per_point)?;
        let setup = self.setup.clone();
        Ok(Harness::builder()
            .with_size(self.size)
            .with_pixels_per_point(self.pixels_per_point)
            .with_max_steps(self.max_steps)
            .renderer(LazyRenderer::new(super::wgpu_renderer))
            .with_context_setup(move |ctx| {
                if let Some(setup) = setup {
                    setup(ctx);
                }
            }))
    }

    /// A harness drawing `app` with `state`, after one frame (and [`Self::max_texture_side`]
    /// applied).
    pub fn harness<'a, State>(&self, app: impl FnMut(&mut Ui, &mut State) + 'a, state: State) -> Result<Harness<'a, State>, RenderError> {
        // designcraft: crates/ui-egui/src/lib.rs:339
        let builder = self.builder::<State>()?;
        let Some(side) = self.max_texture_side else {
            return Ok(builder.build_ui_state(app, state));
        };
        // The builder runs a frame before the texture limit can be raised: the app skips it.
        let mut app = app;
        let mut first = true;
        let mut harness = builder.build_ui_state(
            move |ui, state| {
                if std::mem::take(&mut first) {
                    return;
                }
                app(ui, state);
            },
            state,
        );
        harness.input_mut().max_texture_side = Some(side);
        harness.step();
        Ok(harness)
    }

    /// Draw `app`, run until it stops asking for frames (`Harness::try_run`), and render.
    pub fn image(&self, app: impl FnMut(&mut Ui)) -> Result<ColorImage, RenderError> {
        // lightcraft: crates/ui-egui/src/headless.rs:452 (snapshot: settle, then paint)
        let mut app = app;
        let mut harness = self.harness(move |ui, (): &mut ()| app(ui), ())?;
        harness.try_run().map_err(|_| RenderError::NotSettled { steps: self.max_steps })?;
        image_of(&mut harness)
    }
}

/// Render `harness`'s current frame (`Harness::render`) as an unmultiplied RGBA image.
pub fn image_of<State>(harness: &mut Harness<'_, State>) -> Result<ColorImage, RenderError> {
    // new: kittest's RgbaImage into egui's ColorImage (ColorImage::from_rgba_unmultiplied)
    let image = harness.render().map_err(RenderError::Renderer)?;
    let [width, height] = [image.width() as usize, image.height() as usize];
    Ok(ColorImage::from_rgba_unmultiplied([width, height], image.as_raw()))
}

/// The image size in pixels for `size` points at `pixels_per_point`: each edge rounded like
/// egui's scissor (`(edge * pixels_per_point).round()`), within `1..=`[`MAX_IMAGE_EDGE`], and
/// at most [`MAX_IMAGE_PIXELS`] in all. Refuses instead of clamping.
pub fn viewport_pixels(size: Vec2, pixels_per_point: f32) -> Result<[usize; 2], RenderError> {
    // lightcraft: crates/ui-egui/src/headless.rs:42
    let (width, height) = (size.x, size.y);
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 || !pixels_per_point.is_finite() || pixels_per_point <= 0.0 {
        return Err(RenderError::Size("viewport dimensions and scale must be finite and positive".into()));
    }
    let axis = |value: f32| {
        let pixels = (value * pixels_per_point).round();
        if !(1.0..=MAX_IMAGE_EDGE as f32).contains(&pixels) {
            return Err(RenderError::Size(format!("scaled viewport edges must be between 1 and {MAX_IMAGE_EDGE} pixels")));
        }
        Ok(pixels as usize)
    };
    let (w, h) = (axis(width)?, axis(height)?);
    if w.checked_mul(h).is_none_or(|pixels| pixels > MAX_IMAGE_PIXELS) {
        return Err(RenderError::Size(format!("scaled viewport exceeds {MAX_IMAGE_PIXELS} pixels")));
    }
    Ok([w, h])
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod render_tests;
