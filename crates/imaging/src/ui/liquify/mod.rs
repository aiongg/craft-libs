//! The Liquify workspace (Filter › Liquify…), in Photoshop's layout.
//!
//! [`LiquifyWorkspace`] is a [`FullWindowDialog`] with the title strip "Liquify (layer, 66%)", the
//! tool strip at the start edge (the eleven Liquify tools in three groups, then Hand and Zoom), the
//! canvas, and one Standard [`FullWindowSide`] at the end edge: the Properties panel (Brush Tool
//! Options, Brush Reconstruct Options, Mask Options, View Options) with OK and Cancel at its
//! bottom. No options bar.
//!
//! The library holds the session as plain data: the strokes and whole-field operations in the order
//! the engine replays them ([`LiquifySession`]), with [`LocalHistory`] undo. The app owns the
//! displacement field and its preview: it applies each [`LiquifyEvent`] to the field as it comes,
//! replays the list after undo, redo and Restore All, and shows the result through
//! [`LiquifySource`]. OK runs `filter.liquify` with [`LiquifySession::steps`]; Cancel changes
//! nothing.
//!
//! ```ignore
//! let out = app.liquify.show(ctx, &PcLiquifySource { proxy: &app.liquify_proxy, doc: &doc });
//! for event in out.events {
//!     match event {
//!         LiquifyEvent::Stroke(e) => app.liquify_engine.stroke(e),
//!         LiquifyEvent::Global(op) => app.liquify_engine.global(op),
//!         LiquifyEvent::Replay(steps) => app.liquify_engine.replay(&steps),
//!         LiquifyEvent::BackdropWanted(of) => app.load_backdrop(of),
//!     }
//! }
//! match out.outcome {
//!     Some(Outcome::Ok) => app.run("filter.liquify", app.liquify.session().steps()),
//!     Some(Outcome::Cancel) => app.close_liquify(),
//!     _ => {}
//! }
//! app.prefs.liquify = app.liquify.settings().clone();
//! ```
//!
//! Files: `session` (the steps, the settings; task WS-LQ1), `tools` (the tools and the stroke
//! recorder; WS-LQ1), `keys` (the workspace keys; WS-LQ1), `panel` (the Properties panel; WS-LQ2),
//! `canvas` (the canvas tool, the backdrop, the mask and mesh, the brush resize; WS-LQ2), this file
//! (`show`, `step`; WS-LQ2).
//!
//! [`FullWindowDialog`]: craft_ui::dialog::FullWindowDialog
//! [`FullWindowSide`]: craft_ui::dialog::full_window::FullWindowSide

mod canvas;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod canvas_tests;
mod keys;
mod labels;
mod panel;
mod session;
#[cfg(test)]
mod session_tests;
pub(crate) mod stories;
mod style;
mod tools;

pub use canvas::{BackdropLayering, LiquifyCanvas, LiquifyCanvasTool, LiquifyMesh, LiquifyTiming};
pub use keys::LiquifyKeys;
pub use labels::LiquifyLabels;
pub use panel::{LiquifyPanel, LiquifyPanelOutput};
pub use session::{BackdropMode, BackdropUse, LiquifyGlobal, LiquifySession, LiquifySettings, LiquifyStep, LiquifyStroke, MeshSize};
pub use style::LiquifyStyle;
pub use tools::{LiquifyBrush, LiquifyTool, StrokeRecorder};

use egui::{Context, Key, Modifiers, Rect};

use craft_ui::canvas::kurbo::Size;
use craft_ui::canvas::{CanvasId, CanvasView, Checkerboard, CursorOptions, NavigationOptions, RasterSource, ToolHost};
use craft_ui::dialog::full_window::{FullWindowCanvas, FullWindowSide, FullWindowTitle, FullWindowView, RasterOverlay, SideFooter};
use craft_ui::dialog::{ButtonRole, DialogLabels, DialogState, FooterButton, FullWindowDialog, LocalHistory, Outcome, Step};
use craft_ui::edit::{CancelReason, EditEvent, GestureId, is_live};
use craft_ui::localized::Labels as _;
use craft_ui::patterns::{LastUsed, ToolRail};
use craft_ui::theme::current;

use canvas::LayeredImage;
use panel::backdrop_id;

use crate::ui::warp::{MeshStyle, paint_mesh};

/// What the app gives the workspace to show each pass: images and names only. The field, the
/// proxy and the backdrop render stay the app's (PhotoCraft's `photocraft_algo::liquify`).
pub trait LiquifySource {
    /// The liquified layer as the field shows it now (the app's proxy, or its full-resolution crop
    /// above 100 %), in document pixels.
    fn image(&self) -> &dyn RasterSource;

    /// The freeze mask, white with the frozen amount as alpha, and its texel size in document
    /// pixels. The library tints it with `liquify.freeze` while Show Mask is on.
    fn freeze_mask(&self) -> Option<(&dyn RasterSource, u32)>;

    /// The displaced mesh for Show Mesh at `size`: the field's nodes moved by the field. `None`
    /// while the app has no mesh to show.
    fn mesh(&self, size: MeshSize) -> Option<LiquifyMesh>;

    /// The backdrop for the current Show Backdrop › Use, once the app has it (it was asked with
    /// [`LiquifyEvent::BackdropWanted`]); `None` until then.
    fn backdrop(&self) -> Option<&dyn RasterSource>;

    /// Show Backdrop › Use's layers, bottom to top, as `(layer id, name)`. "All Layers" is the
    /// library's own first entry.
    fn layers(&self) -> Vec<(u64, String)>;
}

/// What opens the workspace.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct LiquifyOpen {
    /// The document's size in pixels.
    pub size: [u32; 2],
    /// The layer's name: the title's subject.
    pub subject: String,
    /// The remembered settings (PhotoCraft: `prefs.dialogs["filter.liquify"]`).
    pub settings: LiquifySettings,
    /// The recorded steps of a smart filter opened again; empty for a new Liquify.
    pub steps: Vec<LiquifyStep>,
    /// The user's navigation options (wheel, flick, animated zoom); Photoshop's by default.
    pub navigation: NavigationOptions,
    /// The user's transparency grid under the image.
    pub checkerboard: Checkerboard,
    /// The user's cursor options (the brush outline).
    pub cursor_options: CursorOptions,
}

impl LiquifyOpen {
    /// Opens on a layer of `size` document pixels named `subject`; every other option takes its default.
    pub fn new(size: [u32; 2], subject: impl Into<String>) -> Self {
        Self { size, subject: subject.into(), ..Self::default() }
    }

    /// The remembered settings.
    pub fn settings(mut self, settings: LiquifySettings) -> Self {
        self.settings = settings;
        self
    }

    /// The steps of a smart filter opened again (PhotoCraft's `open_smart_filter`).
    pub fn steps(mut self, steps: Vec<LiquifyStep>) -> Self {
        self.steps = steps;
        self
    }

    /// The user's navigation options (wheel, flick, animated zoom), for the canvas and the Hand
    /// and Zoom tools.
    pub fn navigation(mut self, navigation: NavigationOptions) -> Self {
        self.navigation = navigation;
        self
    }

    /// The user's transparency grid under the image.
    pub fn checkerboard(mut self, checkerboard: Checkerboard) -> Self {
        self.checkerboard = checkerboard;
        self
    }

    /// The user's cursor options for the brush outline.
    pub fn cursor_options(mut self, options: CursorOptions) -> Self {
        self.cursor_options = options;
        self
    }
}

/// What the workspace reports to the app, in the order it happened this pass.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum LiquifyEvent {
    /// A brush stroke or a Freeze Lasso polygon. `Begin` carries the stroke with its first point,
    /// `Update` the stroke's tool and brush with only the points added since the last event (the
    /// app applies them to its field), `Commit` the whole stroke, now recorded in the session
    /// (the app applies the points it has not seen: at least the release point); `Cancel` (Esc, a
    /// lost pointer, a tool switch or ⌘Z during a stroke) is followed by a
    /// [`Replay`](Self::Replay), since the app's field already took the dabs. A lasso reports only
    /// `Commit`.
    Stroke(EditEvent<LiquifyStroke>),
    /// Reconstruct, Mask All, None, Invert All (the panel's buttons, ⌘I, ⌘D): recorded in the
    /// session, then reported for the app to apply.
    Global(LiquifyGlobal),
    /// The field must be rebuilt from these steps: after undo, redo, Restore All and a cancelled
    /// stroke.
    Replay(Vec<LiquifyStep>),
    /// Show Backdrop turned on, or its Use changed: the app loads that backdrop (`None`: All
    /// Layers) and shows it through [`LiquifySource::backdrop`].
    BackdropWanted(Option<u64>),
}

/// What [`LiquifyWorkspace::show`] reports.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct LiquifyOutput {
    /// What happened this pass, in order.
    pub events: Vec<LiquifyEvent>,
    /// OK or Cancel this pass.
    pub outcome: Option<Outcome>,
    /// The workspace's rect.
    pub rect: Rect,
    /// The settings changed this pass (compared before and after, whatever changed them: the
    /// panel, the keys, the strip or a brush resize); the app may persist them.
    pub settings_changed: bool,
    /// The timing of the latest brush stroke, for the app's diagnostics (PhotoCraft's stats line
    /// is not drawn; lead call 14); `None` before the first stroke.
    pub timing: Option<LiquifyTiming>,
}

/// The Liquify workspace: the dialog, the session and its history, the settings, the view and the
/// canvas tool, as one value the app keeps while Liquify is open.
///
/// Keys (single keys only while `FullWindowFx::keys_free`; ⌘Z whatever has focus): W R E C S B O F D L pick the Liquify tools and
/// H, Z the Hand and Zoom tools ([`ToolRail::pick_by_key`](craft_ui::patterns::ToolRail::pick_by_key));
/// `[` and `]` scale the brush, ⌘H shows or hides the mask, ⌘I inverts the mask, ⌘D clears it
/// ([`LiquifyKeys`]); ⌘+, ⌘−, ⌘0, ⌘1 zoom the workspace's view; ⌘Z and ⇧⌘Z step the session's
/// history, also while a number field has focus; Enter is OK and Esc Cancel (Esc during a stroke
/// cancels the stroke only). A tool switch or ⌘Z during a stroke cancels it first
/// (`CancelReason::Removed`, `CancelReason::UndoDuringGesture`); the app's field is rebuilt by the
/// [`LiquifyEvent::Replay`] that follows.
///
/// The side column's header is "Properties", as Photoshop's Liquify panel is headed (lead call 89:
/// the one editor with a header besides Select and Mask and Content-Aware Fill).
pub struct LiquifyWorkspace {
    dialog: DialogState<LiquifySession>,
    session: LiquifySession,
    history: LocalHistory<LiquifySession>,
    settings: LiquifySettings,
    subject: String,
    size: [u32; 2],
    // View state, never in the history (risk 8): the view, the strip's memory, the canvas tool with
    // its stroke and resize in progress.
    view: FullWindowView,
    last_used: LastUsed<LiquifyTool>,
    canvas: LiquifyCanvas,
    /// The backdrop the app was last asked for, while Show Backdrop is on.
    asked: Option<Option<u64>>,
    /// The panel's gesture while a slider runs: the history waits while its control is drawn.
    gesture: Option<GestureId>,
    /// Events that happened between passes ([`step`](Self::step)), reported by the next `show`.
    queued: Vec<LiquifyEvent>,
    /// A step the app asked for while a stroke was open; the next `show` cancels the stroke, then
    /// steps.
    requested: Vec<Step>,
    navigation: NavigationOptions,
    checkerboard: Checkerboard,
}

impl LiquifyWorkspace {
    /// A workspace over `open`, fitted on its first pass.
    pub fn new(open: LiquifyOpen) -> Self {
        let session = LiquifySession::from_steps(open.steps);
        Self {
            dialog: DialogState::new(),
            history: LocalHistory::new(&session),
            session,
            canvas: {
                let mut canvas = LiquifyCanvas::new();
                canvas.cursor_options(open.cursor_options);
                canvas
            },
            settings: open.settings,
            subject: open.subject,
            size: open.size,
            view: FullWindowView::new(),
            last_used: LastUsed::default(),
            asked: None,
            gesture: None,
            queued: Vec::new(),
            requested: Vec::new(),
            navigation: open.navigation,
            checkerboard: open.checkerboard,
        }
    }

    /// The user's navigation options (wheel, flick, animated zoom); Photoshop's by default.
    pub fn navigation(&mut self, navigation: NavigationOptions) {
        self.navigation = navigation;
    }

    /// The user's transparency grid under the image.
    pub fn checkerboard(&mut self, checkerboard: Checkerboard) {
        self.checkerboard = checkerboard;
    }

    /// The user's cursor options for the brush outline.
    pub fn cursor_options(&mut self, options: CursorOptions) {
        self.canvas.cursor_options(options);
    }

    /// Shows the workspace for this pass.
    ///
    /// In order: the events [`step`](Self::step) queued; the title
    /// ([`FullWindowTitle`] with the subject and
    /// the zoom); the history keys (`FullWindowFx::take_step`, read before the body whatever has
    /// focus, so ⌘Z works while a number field has focus; a stroke or other gesture is cancelled
    /// with `UndoDuringGesture` first); [`LiquifyKeys`] while `FullWindowFx::keys_free`; the tool
    /// strip (a `ToolRail` with `pick_by_key(true)` over [`LiquifyTool::slots`]; a pick cancels
    /// the old tool's gesture with `Removed`); the side column ([`LiquifyPanel`], footer
    /// `SideFooter::bottom` with OK and Cancel); the canvas
    /// ([`FullWindowCanvas`]) with the source's
    /// image under [`BackdropLayering`], the freeze mask as a `RasterOverlay` tinted
    /// `liquify.freeze` while Show Mask is on, the mesh through `warp::paint_mesh` while Show Mesh
    /// is on, and [`LiquifyCanvas`] as its tool. Then `LocalHistory::settle(&session, busy)` with
    /// `busy` while a stroke runs or the panel's gesture is live. Undo and redo report
    /// [`LiquifyEvent::Replay`] in the same pass.
    pub fn show(&mut self, ctx: &Context, source: &dyn LiquifySource) -> LiquifyOutput {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:950 `show` (title, strip, panel, footer,
        // preview, brush circle); :730 `history_keys`; :740 `keys`.
        let labels = LiquifyLabels::of_ctx(ctx);
        let dialog_labels = DialogLabels::of_ctx(ctx);
        let tokens = current(ctx);
        let style = Self::style(&tokens);
        let mesh_style = MeshStyle::of(&tokens);
        let layers = source.layers();
        let title = FullWindowTitle::new(labels.name.as_str()).subject(self.subject.as_str()).zoom(self.view.zoom_percent()).text(ctx);
        let Self {
            dialog,
            session,
            history,
            settings,
            subject: _,
            size,
            view,
            last_used,
            canvas,
            asked,
            gesture,
            queued,
            requested,
            navigation,
            checkerboard,
        } = self;
        let (navigation, checkerboard) = (*navigation, *checkerboard);
        let doc = Size::new(f64::from(size[0]), f64::from(size[1]));
        canvas.navigate(doc, navigation);
        let settings_before = settings.clone();
        let asked_steps = std::mem::take(requested);

        let mut events = std::mem::take(queued);
        let mut panel_busy = false;
        let response = FullWindowDialog::new("liquify").title(title).show(ctx, dialog, session, |ui, session, fx| {
            // The id the canvas runs its `ToolHost` under, taken in the scope that shows it.
            let canvas_id = FullWindowCanvas::canvas_id(ui, CANVAS);
            // The history keys are read before the body, whatever has focus, but only while this
            // workspace is the top modal layer; the app's own requests ([`Self::step`]) go first.
            for step in asked_steps.into_iter().chain(fx.take_step(ui)) {
                // An open stroke is cancelled first; the replay of the step supersedes the one the
                // cancel reports.
                let pending = cancel_gesture(ctx, canvas_id, canvas, settings, &style, CancelReason::UndoDuringGesture);
                adopt(view, ui, pending);
                for event in canvas.take_events() {
                    absorb(session, event, &mut events);
                }
                if history.step(session, step) {
                    if matches!(events.last(), Some(LiquifyEvent::Replay(_))) {
                        events.pop();
                    }
                    events.push(LiquifyEvent::Replay(session.steps().to_vec()));
                }
            }
            if fx.keys_free(ui) {
                let (_, globals) = LiquifyKeys::read(ctx, settings);
                for op in globals {
                    record_global(canvas, session, op, &mut events);
                }
            }

            let slots = LiquifyTool::slots(&labels);
            let rail =
                ToolRail::new("liquify-tools").label(labels.tools.as_str()).pick_by_key(true).show(ui, &slots, &settings.tool, last_used, |_, _| {});
            if let Some(tool) = rail.select.filter(|tool| *tool != settings.tool) {
                let pending = cancel_gesture(ctx, canvas_id, canvas, settings, &style, CancelReason::Removed);
                adopt(view, ui, pending);
                for event in canvas.take_events() {
                    absorb(session, event, &mut events);
                }
                settings.tool = tool;
            }

            let footer = SideFooter::bottom(vec![
                FooterButton::new(ButtonRole::Default, dialog_labels.ok.as_str()),
                FooterButton::new(ButtonRole::Cancel, dialog_labels.cancel.as_str()),
            ]);
            let mut panel = None;
            FullWindowSide::new("liquify-props").header(labels.properties.as_str()).footer(footer).show(ui, fx, |ui| {
                panel = Some(LiquifyPanel::new("liquify-panel", settings).layers(&layers).show(ui));
            });
            if let Some(panel) = panel {
                match &panel.edit {
                    Some(EditEvent::Begin { id, .. } | EditEvent::Update { id, .. }) => *gesture = Some(*id),
                    Some(EditEvent::Commit { .. } | EditEvent::Cancel { .. }) => *gesture = None,
                    _ => {}
                }
                if let Some(op) = panel.global {
                    record_global(canvas, session, op, &mut events);
                }
                if panel.restore_all {
                    finish_stroke(canvas, session, &mut events);
                    session.restore_all();
                    events.push(LiquifyEvent::Replay(Vec::new()));
                }
                if let Some(of) = panel.backdrop_wanted {
                    events.push(LiquifyEvent::BackdropWanted(of));
                    *asked = Some(of);
                }
            }
            // A slider whose control is no longer drawn has ended its gesture.
            if gesture.is_some_and(|id| !is_live(ctx, id)) {
                *gesture = None;
            }
            panel_busy = gesture.is_some();
            if !settings.show_backdrop {
                *asked = None;
            } else if asked.is_none() {
                // Show Backdrop was already on when the workspace opened.
                let of = backdrop_id(settings.backdrop);
                events.push(LiquifyEvent::BackdropWanted(of));
                *asked = Some(of);
            }

            let backdrop = source.backdrop().filter(|_| settings.show_backdrop);
            let layering = BackdropLayering::of(backdrop.is_some(), settings.backdrop_mode, settings.backdrop_opacity);
            let layered = backdrop.filter(|_| !layering.is_plain()).map(|backdrop| LayeredImage { image: source.image(), backdrop, layering });
            let image: &dyn RasterSource = match &layered {
                Some(layered) => layered,
                None => source.image(),
            };
            let mut widget = FullWindowCanvas::new(CANVAS, view).source(image).checkerboard(checkerboard).navigation(navigation);
            if let Some((mask, texel)) = source.freeze_mask().filter(|_| settings.show_mask) {
                // The mask is white with the frozen amount as alpha: the tint carries the color.
                let [r, g, b, a] = style.freeze.to_srgba_unmultiplied();
                let tint = egui::Color32::from_rgba_premultiplied(r, g, b, (f32::from(a) * style.mask_opacity).round() as u8);
                widget = widget.overlay(RasterOverlay::new(mask, texel).tint(tint));
            }
            let mesh = source.mesh(settings.mesh_size).filter(|_| settings.show_mesh);
            widget = widget.painter(move |painter, view| {
                if let Some(mesh) = &mesh {
                    paint_mesh(painter, mesh.edges(view), &mesh_style);
                }
            });
            let shown = match settings.tool {
                LiquifyTool::Hand => widget.tool(canvas.hand()).show(ui),
                LiquifyTool::Zoom => widget.tool(canvas.zoom()).show(ui),
                _ => widget.tool(&mut canvas.tool(settings, &style)).show(ui),
            };
            // The Esc that cancelled a stroke is not the dialog's Cancel.
            if shown.took_key.is_some_and(|key| key.key == Key::Escape) {
                ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
            }
            for event in canvas.take_events() {
                absorb(session, event, &mut events);
            }
        });
        history.settle(session, canvas.busy() || panel_busy);
        LiquifyOutput {
            events,
            outcome: response.outcome,
            rect: response.rect,
            settings_changed: *settings != settings_before,
            timing: canvas.timing(),
        }
    }

    /// What OK applies.
    pub fn session(&self) -> &LiquifySession {
        &self.session
    }

    /// What the app persists (always: PhotoCraft remembers Liquify's settings on every close).
    pub fn settings(&self) -> &LiquifySettings {
        &self.settings
    }

    /// Undo or redo from the app's Edit menu. Returns whether a step will happen; its
    /// [`LiquifyEvent::Replay`] comes with the next [`show`](Self::show)'s events. With a stroke
    /// open the step waits for that pass, which cancels the stroke (`UndoDuringGesture`) first.
    /// The workspace reads ⌘Z itself while it is the top modal layer.
    pub fn step(&mut self, step: Step) -> bool {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:340 `undo`, :351 `redo`.
        if self.canvas.busy() {
            self.requested.push(step);
            return match step {
                Step::Undo => self.history.can_undo(),
                Step::Redo => self.history.can_redo(),
            };
        }
        let stepped = self.history.step(&mut self.session, step);
        if stepped {
            self.queued.push(LiquifyEvent::Replay(self.session.steps().to_vec()));
        }
        stepped
    }
}

/// The canvas's id salt, shared by the canvas and the tool switch that cancels its gesture.
const CANVAS: &str = "liquify-view";

/// Puts `event` in `out`, recording a committed stroke in the session first (the lasso's closing
/// point included) and following a cancelled one with the replay that rebuilds the app's field.
fn absorb(session: &mut LiquifySession, event: LiquifyEvent, out: &mut Vec<LiquifyEvent>) {
    match event {
        LiquifyEvent::Stroke(EditEvent::Commit { id, value, modifiers, .. }) => {
            let before = session.len();
            session.record(value.clone());
            let recorded = match session.steps().last() {
                Some(LiquifyStep::Stroke(stroke)) if session.len() > before => stroke.clone(),
                _ => value,
            };
            out.push(LiquifyEvent::Stroke(EditEvent::commit(id, recorded, true, modifiers)));
        }
        LiquifyEvent::Stroke(cancel @ EditEvent::Cancel { .. }) => {
            out.push(LiquifyEvent::Stroke(cancel));
            out.push(LiquifyEvent::Replay(session.steps().to_vec()));
        }
        other => out.push(other),
    }
}

/// Cancels the gesture the current tool holds (a Liquify tool's stroke, lasso or resize, or the
/// Hand's or Zoom's drag) through the host, and the tool's own state if the host held none. Returns
/// the view the Hand puts back, for the workspace's view to adopt.
fn cancel_gesture(
    ctx: &Context,
    id: CanvasId,
    canvas: &mut LiquifyCanvas,
    settings: &mut LiquifySettings,
    style: &LiquifyStyle,
    reason: CancelReason,
) -> Option<CanvasView> {
    let pending = match settings.tool {
        LiquifyTool::Hand => ToolHost::cancel(ctx, id, canvas.hand(), reason),
        LiquifyTool::Zoom => ToolHost::cancel(ctx, id, canvas.zoom(), reason),
        _ => ToolHost::cancel(ctx, id, &mut canvas.tool(settings, style), reason),
    };
    if canvas.busy() {
        canvas.tool(settings, style).abandon(reason);
    }
    pending
}

/// Takes the view a cancelled tool put back.
fn adopt(view: &mut FullWindowView, ui: &egui::Ui, pending: Option<CanvasView>) {
    if let Some(pending) = pending {
        view.take(&pending, ui.pixels_per_point());
    }
}

/// Ends the stroke in progress, if any, and records it.
fn finish_stroke(canvas: &mut LiquifyCanvas, session: &mut LiquifySession, out: &mut Vec<LiquifyEvent>) {
    if let Some(event) = canvas.end_stroke() {
        absorb(session, event, out);
    }
}

/// Records a whole-field operation after the stroke in progress.
fn record_global(canvas: &mut LiquifyCanvas, session: &mut LiquifySession, op: LiquifyGlobal, out: &mut Vec<LiquifyEvent>) {
    finish_stroke(canvas, session, out);
    session.record_global(op);
    out.push(LiquifyEvent::Global(op));
}
