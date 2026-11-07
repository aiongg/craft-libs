//! `app::run`, the eframe entry point, and `native_options`: the root window opens where it was and the runtime is
//! installed on eframe's context. Behind the `app` feature because eframe is a heavy, platform-specific dependency.
//!
//! Apps with their own eframe or wgpu setup call `runtime::install` and `runtime::frame::set_input` themselves.

use crate::runtime::InstallSpec;
use crate::runtime::frame::{FrameInput, FrameOutput, set_input, take_output};
use craft_host::target::ShellInfo;
use craft_window::restore::{Defaults, MAIN, Store, place};
use craft_window::system::NoWindows;
use egui::{Context, Pos2, Ui, ViewportBuilder};
use std::sync::{Arc, Mutex};

/// What `run` needs of the app.
#[non_exhaustive]
pub struct AppSpec {
    /// The root window's title.
    pub title: String,
    /// The app id: the Wayland app id and the name eframe stores the app under.
    pub app_id: String,
    /// The platform the app runs on.
    pub shell_info: ShellInfo,
    /// The first window's size and the smallest window.
    pub defaults: Defaults,
    /// Where window placement is kept.
    pub placement: Arc<Mutex<Store>>,
}

impl AppSpec {
    /// A spec for `app_id` titled `title`, with the running build's shell and an in-memory placement store.
    pub fn new(app_id: impl Into<String>, title: impl Into<String>, defaults: Defaults) -> Self {
        AppSpec {
            title: title.into(),
            app_id: app_id.into(),
            shell_info: ShellInfo::default(),
            defaults,
            placement: Arc::new(Mutex::new(Store::in_memory())),
        }
    }

    /// Sets the placement store, shared by every window of the app.
    pub fn placement(mut self, store: Arc<Mutex<Store>>) -> Self {
        self.placement = store;
        self
    }

    /// Sets the shell info.
    pub fn shell_info(mut self, shell_info: ShellInfo) -> Self {
        self.shell_info = shell_info;
        self
    }
}

/// The app `run` drives.
pub trait ShellApp {
    /// What the runtime reads before each pass.
    fn frame_input(&mut self) -> FrameInput;

    /// Shows the root window.
    fn ui(&mut self, ui: &mut Ui);

    /// Receives what the runtime accumulated since the last frame: fired commands, announcements, the chrome toggle.
    fn output(&mut self, _output: FrameOutput) {}
}

/// eframe's `App` over a `ShellApp`: inputs go in before the pass and outputs come out at the start of the next
/// frame, never in `logic`.
struct Driver<A> {
    app: A,
}

impl<A: ShellApp> eframe::App for Driver<A> {
    fn raw_input_hook(&mut self, ctx: &Context, _: &mut crate::runtime::egui::input::RawInput) {
        let output = take_output(ctx);
        self.app.output(output);
        set_input(ctx, self.app.frame_input());
    }

    fn ui(&mut self, ui: &mut Ui, _: &mut eframe::Frame) {
        self.app.ui(ui);
    }
}

/// The native options for the root window: its saved placement, else the defaults.
pub fn native_options(spec: &AppSpec) -> eframe::NativeOptions {
    let saved = spec.placement.lock().unwrap_or_else(std::sync::PoisonError::into_inner).get(MAIN);
    // The monitors are not known before the window exists, so the saved geometry is kept as it is.
    let placed = place(saved.as_ref(), &[], None, &spec.defaults);
    let scale = placed.scale.max(f32::EPSILON);
    let mut viewport = ViewportBuilder::default()
        .with_title(spec.title.clone())
        .with_app_id(spec.app_id.clone())
        .with_inner_size(placed.normal.size)
        .with_min_inner_size(spec.defaults.min)
        .with_maximized(placed.maximized)
        .with_fullscreen(placed.fullscreen);
    if let Some([x, y]) = placed.position_px {
        viewport = viewport.with_position(Pos2::new(x as f32 / scale, y as f32 / scale));
    }
    eframe::NativeOptions { viewport, ..eframe::NativeOptions::default() }
}

/// Opens the root window and runs `make`'s app until it closes. The runtime is installed on eframe's context
/// before `make` runs; an install error (two style replacements for one style key) ends the run with it.
pub fn run<A: ShellApp + 'static>(spec: AppSpec, make: impl FnOnce(&Context) -> A + 'static) -> eframe::Result {
    let options = native_options(&spec);
    let title = spec.app_id.clone();
    eframe::run_native(
        &title,
        options,
        Box::new(move |cc| {
            let install = InstallSpec::new(spec.shell_info, Arc::new(NoWindows)).placement(spec.placement);
            crate::install(&cc.egui_ctx, install).map_err(|error| format!("the runtime could not be installed: {error}"))?;
            Ok(Box::new(Driver { app: make(&cc.egui_ctx) }))
        }),
    )
}
