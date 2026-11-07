//! The frame plugin: the per-pass and per-frame steps, with the inputs the app sets and the outputs it drains.

use crate::runtime::commands::{self, CommandInput, Fired};
use crate::runtime::egui::input::{self as raw_input, HeldInput, RawInput};
use crate::runtime::egui::memory::{self as store, Service};
use crate::runtime::egui::{pass, scale, style};
use crate::runtime::env::{Env, RootEnv};
use crate::runtime::{access, cursor, drag, focus, ime, keys, layout_memory, memory, motion, overlay, pointer, toast, wheel, windows};
use craft_host::Host;
use craft_host::target::Os;
use craft_theme::{AppSizes, Appearance, Contrast, HitSizes, Mode, SUITE_DEFAULT, Size, TextSize, ThemeDef, builtins, resolve_sized};
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
    /// App-scope command state, the binding overrides and the gate, set from app state.
    pub commands: CommandInput,
    /// The app's `Host`, read through `EnvView::host`.
    pub host: Option<Arc<Host>>,
    /// Whether decision records are kept for the debug inspector.
    pub inspect: bool,
}

impl FrameInput {
    /// A frame with `root` and no host.
    pub fn new(root: RootEnv) -> Self {
        FrameInput { root, commands: CommandInput::new(), host: None, inspect: false }
    }

    /// Sets the command input.
    pub fn commands(mut self, commands: CommandInput) -> Self {
        self.commands = commands;
        self
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
    /// The app-scope commands fired since the last call, in order.
    pub fired: Vec<Fired>,
    /// Text for the live announcement region.
    pub announcements: Vec<String>,
    /// Whether the chrome-toggle key was pressed.
    pub toggle_chrome: bool,
}

/// The per-frame and per-context state of the frame loop.
#[derive(Default)]
struct FrameState {
    input: Option<FrameInput>,
    /// The pointer tolerances of the app's resolved tokens (`tokens.size.hit`) and the `sizes` they came from:
    /// the input hook writes the slop before any `Env` exists, and the press lifecycle reads the hit target.
    hit: Option<(Arc<AppSizes>, HitSizes)>,
    output: FrameOutput,
    /// Passes between their begin and end hooks: nested viewport passes count.
    passes_open: u32,
    /// How many root-viewport passes the begin hook has run for: egui counts a pass when it ends.
    root_passes_hooked: u64,
    prunes: HashMap<TypeId, fn(&Context, &[ViewportId])>,
    sweeps: HashMap<TypeId, fn(&Context)>,
    frame_layers: Option<fn(&Context)>,
}

/// Records of one runtime service, one per viewport. A viewport egui stops reporting has its record dropped
/// at once, by the frame loop's end hook.
pub(crate) struct PerViewport<T> {
    records: HashMap<ViewportId, T>,
    pruning: bool,
}

impl<T> Default for PerViewport<T> {
    fn default() -> Self {
        PerViewport { records: HashMap::new(), pruning: false }
    }
}

/// How a record that follows the passes of its viewport advanced to the running pass.
pub(crate) enum PassStep {
    /// The viewport's previous pass left this record.
    Next,
    /// A pass or more went by without touching it.
    Skipped,
}

/// A record that resets when its viewport's pass number changes.
#[derive(Default)]
pub(crate) struct Stamped<T> {
    pass: u64,
    value: T,
}

impl<T: Default + Send + 'static> PerViewport<T> {
    /// Runs `f` on the record of `viewport`, created on first use.
    pub(crate) fn with<R>(ctx: &Context, viewport: ViewportId, f: impl FnOnce(&mut T) -> R) -> R {
        let service = Service::<PerViewport<T>>::of(ctx);
        if !service.with(|records| std::mem::replace(&mut records.pruning, true)) {
            Self::register_prune(ctx);
        }
        service.with(|records| f(records.records.entry(viewport).or_default()))
    }

    fn register_prune(ctx: &Context) {
        fn prune<T: Default + Send + 'static>(ctx: &Context, live: &[ViewportId]) {
            Service::<PerViewport<T>>::of(ctx).with(|records| records.records.retain(|viewport, _| live.contains(viewport)));
        }
        register_prune(ctx, TypeId::of::<T>(), prune::<T>);
    }
}

impl<T: Default + Send + 'static> PerViewport<Stamped<T>> {
    /// Runs `f` on the record of `viewport` for its pass `pass_nr`; `reset` runs first when the pass changed.
    pub(crate) fn with_pass<R>(
        ctx: &Context,
        viewport: ViewportId,
        pass_nr: u64,
        reset: impl FnOnce(&mut T, PassStep),
        f: impl FnOnce(&mut T) -> R,
    ) -> R {
        Self::with(ctx, viewport, |record| {
            if record.pass != pass_nr {
                reset(&mut record.value, if record.pass + 1 == pass_nr { PassStep::Next } else { PassStep::Skipped });
                record.pass = pass_nr;
            }
            f(&mut record.value)
        })
    }
}

/// Registers the end hook's prune of a service's per-viewport records, once per service type.
pub(crate) fn register_prune(ctx: &Context, key: TypeId, prune: fn(&Context, &[ViewportId])) {
    store::with::<FrameState, _>(ctx, |state| {
        state.prunes.entry(key).or_insert(prune);
    });
}

/// Registers a sweep the end hook runs for the running viewport's pass, once per service type.
pub(crate) fn register_end_sweep(ctx: &Context, key: TypeId, sweep: fn(&Context)) {
    store::with::<FrameState, _>(ctx, |state| {
        state.sweeps.entry(key).or_insert(sweep);
    });
}

/// Before the pass: from eframe's `App::raw_input_hook`, or before `run_ui` in tests. A call during a pass,
/// or after a pass whose plugin hooks did not run, is a debug assertion.
pub fn set_input(ctx: &Context, input: FrameInput) {
    let scale = input.root.scale;
    let commands = input.commands.clone();
    let root_passes = pass::pass_nr(ctx, ViewportId::ROOT);
    let hooks_ran = store::with::<FrameState, _>(ctx, |state| {
        debug_assert!(state.passes_open == 0, "set_input was called during a pass");
        let ran = state.root_passes_hooked == root_passes;
        if !state.hit.as_ref().is_some_and(|(sizes, _)| Arc::ptr_eq(sizes, &input.root.sizes)) {
            state.hit = Some((input.root.sizes.clone(), resolved_hit(&input.root.sizes)));
        }
        state.input = Some(input);
        ran
    });
    commands::set_input(ctx, &commands);
    windows::set_input(ctx);
    debug_assert!(hooks_ran, "the runtime plugin's hooks did not run for the previous pass: drive frames with `Context::run_ui` or `run_pass`");
    scale::set_interface_scale(ctx, scale.0);
}

/// The tolerances the app's `sizes` give the default density: the built-in sets and the overrides apply in
/// `craft_theme`'s resolve, whichever theme carries them.
fn resolved_hit(sizes: &AppSizes) -> HitSizes {
    let appearance =
        Appearance { mode: Mode::Light, contrast: Contrast::Standard, size: Size::default(), text_size: TextSize::default(), reduced_motion: false };
    resolve_sized(suite_default_theme(), appearance, None, sizes).tokens.size.hit
}

/// The built-in theme the tolerances resolve under.
fn suite_default_theme() -> &'static ThemeDef {
    let themes = builtins();
    themes.iter().find(|theme| theme.id.as_str() == SUITE_DEFAULT).unwrap_or(&themes[0])
}

/// The tolerances of the running frame: the app's, else the default density's.
fn frame_hit(ctx: &Context) -> HitSizes {
    store::with::<FrameState, _>(ctx, |state| state.hit.as_ref().map(|(_, hit)| *hit)).unwrap_or_else(|| resolved_hit(&AppSizes::default()))
}

/// Called by `install`: the passes the context ran before the plugin existed have no hooks to wait for.
pub(crate) fn seed_hooked_passes(ctx: &Context) {
    let root_passes = pass::pass_nr(ctx, ViewportId::ROOT);
    store::with::<FrameState, _>(ctx, |state| state.root_passes_hooked = root_passes);
}

/// Whether a pass is between its begin and end hooks.
pub(crate) fn in_pass(ctx: &Context) -> bool {
    store::with::<FrameState, _>(ctx, |state| state.passes_open > 0)
}

/// Everything accumulated since the last call, across passes and discarded passes.
pub fn take_output(ctx: &Context) -> FrameOutput {
    let mut output = store::with::<FrameState, _>(ctx, |state| std::mem::take(&mut state.output));
    output.fired = commands::take_fired(ctx);
    output
}

/// Sets the end hook's ghost-layer painter.
pub(crate) fn set_frame_layers(ctx: &Context, painter: Option<fn(&Context)>) {
    store::with::<FrameState, _>(ctx, |state| state.frame_layers = painter);
}

/// Whether decision records are kept for the debug inspector.
pub(crate) fn inspecting(ctx: &Context) -> bool {
    store::with::<FrameState, _>(ctx, |state| state.input.as_ref().is_some_and(|input| input.inspect))
}

/// Adds text to the next `FrameOutput`'s announcements.
pub(crate) fn announce(ctx: &Context, text: String) {
    store::with::<FrameState, _>(ctx, |state| state.output.announcements.push(text));
}

/// Marks the chrome toggle in the next `FrameOutput`.
#[expect(dead_code, reason = "used by the pipeline (F3)")]
pub(crate) fn request_chrome_toggle(ctx: &Context) {
    store::with::<FrameState, _>(ctx, |state| state.output.toggle_chrome = true);
}

/// The held input, the runtime's request to end the viewport's egui drag and the clock of its running pass.
#[derive(Default)]
struct ViewportRecord {
    held: HeldInput,
    press_cancelled: bool,
    now: f64,
}

/// The running pass's time in seconds, read once by the begin hook.
pub(crate) fn clock(ctx: &Context) -> f64 {
    PerViewport::<ViewportRecord>::with(ctx, pass::current_viewport(ctx), |record| record.now)
}

/// Records that the runtime cancelled the captured press of the running viewport; the next begin hook ends
/// egui's drag so hover reaches other widgets again.
pub(crate) fn note_cancelled_press(ctx: &Context) {
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
    // H2: click options.
    raw_input::set_click_options(ctx, frame_hit(ctx).slop);
    // H3: the key stage strips what the runtime owns; egui decides on a repaint from the events left.
    ime::input_hook(ctx, &mut input.events, viewport);
    let stripped = keys::input_hook(ctx, input);
    // H3b: a press outside the top menu family goes no further than the menu's dismissal.
    let absorbed = overlay::absorb_presses(ctx, input);
    // H4: release after an OS resize.
    let released = windows::input_hook(ctx, input);
    if held_back || stripped || absorbed || released {
        raw_input::request_repaint_of(ctx, viewport);
    }
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
    // H5: read the clock once; the runtime cancelled a captured press since this viewport's last pass.
    let now = pass::now(&ctx);
    let cancelled = PerViewport::<ViewportRecord>::with(&ctx, viewport, |record| {
        record.now = now;
        std::mem::take(&mut record.press_cancelled)
    });
    if cancelled {
        raw_input::stop_dragging(&ctx);
    }
    // H5: this viewport's window info; a press H3 dropped under a modal raises the modal's window.
    windows::begin(&ctx);
    // H5: the pointer snapshot, before any component runs.
    let input = frame_input.unwrap_or_else(|| FrameInput::new(RootEnv::fallback(&ctx)));
    pointer::begin(&ctx, frame_hit(&ctx));
    drag::begin(&ctx);
    // H5: install the root `Env` and the theme styles' direction from `FrameInput.root`.
    let env = Env::from_root(&ctx, &input.root, input.host, scale::pixels_per_point(&ctx));
    style::set_theme_direction(&ctx, ui, env.direction.is_rtl());
    Env::set_root(&ctx, viewport, env.clone());
    drop(Env::install_for(ui, env));
    // H7: focus moves and the press classification, then the pass's intents, then the wheel split.
    focus::begin(&ctx, input.root.tab_all_controls || raw_input::os(&ctx) != Os::Mac);
    ime::begin(&ctx, focus::focused_rec(&ctx).filter(|rec| rec.stop.text).map(|rec| rec.stop.root));
    // Lead-1: an Escape over a captured press cancels the press and reaches no component.
    if keys::begin(&ctx, pointer::press_captured(&ctx) || drag::running(&ctx)) {
        pointer::cancel_press(&ctx, craft_behavior::PressCancel::Escape);
        drag::cancel(&ctx, craft_behavior::CancelReason::Escape);
    }
    wheel::begin(&ctx);
    // H7: shortcuts the owner allows, through the gate.
    commands::begin(&ctx);
    access::begin_live_pass(&ctx);
}

/// The end hook.
pub(crate) fn end_pass(ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    drag::end_pass(&ctx);
    cursor::end_pass(&ctx);
    commands::end(&ctx);
    access::end_live_pass(&ctx);
    let painter = store::with::<FrameState, _>(&ctx, |state| state.frame_layers);
    if let Some(paint_frame_layers) = painter {
        paint_frame_layers(&ctx);
    }
    toast::end_pass(&ctx);
    overlay::end_pass(&ctx);
    focus::end(&ctx);
    let sweeps: Vec<fn(&Context)> = store::with::<FrameState, _>(&ctx, |state| state.sweeps.values().copied().collect());
    for sweep in sweeps {
        sweep(&ctx);
    }
    // Evict shown memory by id and queue the evicted slots (7.2).
    memory::evict(&ctx);
    layout_memory::end_hook(&ctx);
    motion::end_pass(&ctx);
    windows::end(&ctx);
    ime::end_pass(&ctx);
    let live = pass::live_viewports(&ctx);
    let prunes: Vec<fn(&Context, &[ViewportId])> = store::with::<FrameState, _>(&ctx, |state| {
        state.passes_open = state.passes_open.saturating_sub(1);
        state.prunes.values().copied().collect()
    });
    for prune in prunes {
        prune(&ctx, &live);
    }
}
