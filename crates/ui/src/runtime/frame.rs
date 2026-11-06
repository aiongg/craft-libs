//! The frame plugin: the per-pass and per-frame steps, with the inputs the app sets and the outputs it drains.

use crate::runtime::egui::input::{self as raw_input, HeldInput, RawInput};
use crate::runtime::egui::memory::{self as store, Service};
use crate::runtime::egui::{pass, scale, style};
use crate::runtime::env::{Env, RootEnv, prune_viewports};
use crate::runtime::memory;
use craft_host::Host;
use egui::{Context, Ui, ViewportId};
use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Arc;

/// What the app sets before each frame.
#[derive(Clone)]
#[non_exhaustive]
pub struct FrameInput {
    /// The app's root values.
    pub root: RootEnv,
    /// The app's `Host`, read through `EnvView::host`.
    pub host: Option<Arc<Host>>,
    /// Whether decision records are kept for the debug inspector.
    pub inspect: bool,
}

impl FrameInput {
    /// A frame with `root` and no host.
    pub fn new(root: RootEnv) -> Self {
        FrameInput { root, host: None, inspect: false }
    }

    /// Sets the host.
    pub fn host(mut self, host: Arc<Host>) -> Self {
        self.host = Some(host);
        self
    }

    /// Turns decision records on or off.
    pub const fn inspect(mut self, on: bool) -> Self {
        self.inspect = on;
        self
    }
}

/// What the runtime accumulated since the last `take_output`, across passes and discarded passes.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct FrameOutput {
    /// Text for the live announcement region.
    pub announcements: Vec<String>,
    /// Whether the chrome-toggle key was pressed.
    pub toggle_chrome: bool,
}

/// The per-frame and per-context state of the frame loop.
#[derive(Default)]
struct FrameState {
    input: Option<FrameInput>,
    output: FrameOutput,
    /// Passes between their begin and end hooks: nested viewport passes count.
    passes_open: u32,
    /// How many root-viewport passes the begin hook has run for: egui counts a pass when it ends.
    root_passes_hooked: u64,
    prunes: HashMap<TypeId, fn(&Context, &[ViewportId])>,
    frame_layers: Option<fn(&Context)>,
}

/// Records of one runtime service, one per viewport. A viewport egui stops reporting has its record dropped
/// at once, by the frame loop's end hook.
pub(crate) struct PerViewport<T>(HashMap<ViewportId, T>);

impl<T> Default for PerViewport<T> {
    fn default() -> Self {
        PerViewport(HashMap::new())
    }
}

impl<T: Default + Send + 'static> PerViewport<T> {
    /// Runs `f` on the record of `viewport`, created on first use.
    pub(crate) fn with<R>(ctx: &Context, viewport: ViewportId, f: impl FnOnce(&mut T) -> R) -> R {
        Self::register_prune(ctx);
        Service::<PerViewport<T>>::of(ctx).with(|records| f(records.0.entry(viewport).or_default()))
    }

    fn register_prune(ctx: &Context) {
        fn prune<T: Default + Send + 'static>(ctx: &Context, live: &[ViewportId]) {
            Service::<PerViewport<T>>::of(ctx).with(|records| records.0.retain(|viewport, _| live.contains(viewport)));
        }
        store::with::<FrameState, _>(ctx, |state| {
            state.prunes.entry(TypeId::of::<T>()).or_insert(prune::<T>);
        });
    }
}

/// Before the pass: from eframe's `App::raw_input_hook`, or before `run_ui` in tests. A call during a pass,
/// or after a pass whose plugin hooks did not run, is a debug assertion.
pub fn set_input(ctx: &Context, input: FrameInput) {
    let scale = input.root.scale;
    let hooks_ran = store::with::<FrameState, _>(ctx, |state| {
        debug_assert!(state.passes_open == 0, "set_input was called during a pass");
        let ran = state.root_passes_hooked == pass::pass_nr(ctx, ViewportId::ROOT);
        state.input = Some(input);
        ran
    });
    debug_assert!(hooks_ran, "the runtime plugin's hooks did not run for the previous pass: drive frames with `Context::run_ui` or `run_pass`");
    scale::set_interface_scale(ctx, scale.0);
}

/// Everything accumulated since the last call, across passes and discarded passes.
pub fn take_output(ctx: &Context) -> FrameOutput {
    store::with::<FrameState, _>(ctx, |state| std::mem::take(&mut state.output))
}

/// Sets the end hook's ghost-layer painter.
pub(crate) fn set_frame_layers(ctx: &Context, painter: Option<fn(&Context)>) {
    store::with::<FrameState, _>(ctx, |state| state.frame_layers = painter);
}

/// Adds text to the next `FrameOutput`'s announcements.
#[doc(hidden)]
pub fn announce(ctx: &Context, text: String) {
    store::with::<FrameState, _>(ctx, |state| state.output.announcements.push(text));
}

/// Marks the chrome toggle in the next `FrameOutput`.
#[doc(hidden)]
pub fn request_chrome_toggle(ctx: &Context) {
    store::with::<FrameState, _>(ctx, |state| state.output.toggle_chrome = true);
}

/// The held input and the runtime's request to end the viewport's egui drag.
#[derive(Default)]
struct ViewportRecord {
    held: HeldInput,
    press_cancelled: bool,
}

/// Records that the runtime cancelled the captured press of the running viewport; the next begin hook ends
/// egui's drag so hover reaches other widgets again.
#[doc(hidden)]
pub fn note_cancelled_press(ctx: &Context) {
    let viewport = pass::current_viewport(ctx);
    PerViewport::<ViewportRecord>::with(ctx, viewport, |record| record.press_cancelled = true);
    ctx.request_repaint();
}

/// The input hook: egui runs it before it pushes the viewport, so every step reads only `RawInput` and
/// records keyed by `input.viewport_id`.
pub(crate) fn input_hook(ctx: &Context, input: &mut RawInput) {
    let viewport = input.viewport_id;
    // H0: one press per pass.
    let frame = pass::frame_nr(ctx, viewport);
    let held_back = PerViewport::<ViewportRecord>::with(ctx, viewport, |record| raw_input::hold_back_presses(input, frame, &mut record.held));
    if held_back {
        raw_input::request_repaint_of(ctx, viewport);
    }
    // H2: click options.
    raw_input::set_click_options(ctx);
}

/// The begin hook, where the viewport is current.
pub(crate) fn begin_pass(ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    let viewport = pass::current_viewport(&ctx);
    let root_pass = (viewport == ViewportId::ROOT).then(|| pass::pass_nr(&ctx, viewport));
    let frame_input = store::with::<FrameState, _>(&ctx, |state| {
        state.passes_open += 1;
        if let Some(nr) = root_pass {
            state.root_passes_hooked = nr + 1;
        }
        state.input.clone()
    });
    // H5: the runtime cancelled a captured press since this viewport's last pass.
    let cancelled = PerViewport::<ViewportRecord>::with(&ctx, viewport, |record| std::mem::take(&mut record.press_cancelled));
    if cancelled {
        raw_input::stop_dragging(&ctx);
    }
    // H5: install the root `Env` and the theme styles' direction from `FrameInput.root`.
    let input = frame_input.unwrap_or_else(|| FrameInput::new(RootEnv::default()));
    let env = Env::from_root(&input.root, input.host, scale::pixels_per_point(&ctx));
    style::set_theme_direction(&ctx, ui, env.direction.is_rtl());
    drop(Env::install_for(ui, env));
}

/// The end hook.
pub(crate) fn end_pass(ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    let painter = store::with::<FrameState, _>(&ctx, |state| state.frame_layers);
    if let Some(paint_frame_layers) = painter {
        paint_frame_layers(&ctx);
    }
    // Evict shown memory by id and queue the evicted slots (7.2).
    memory::evict(&ctx);
    let live = pass::live_viewports(&ctx);
    prune_viewports(&ctx, &live);
    let prunes: Vec<fn(&Context, &[ViewportId])> = store::with::<FrameState, _>(&ctx, |state| {
        state.passes_open = state.passes_open.saturating_sub(1);
        state.prunes.values().copied().collect()
    });
    for prune in prunes {
        prune(&ctx, &live);
    }
}
