//! The Liquify workspace (Filter › Liquify…), in Photoshop's layout.
//!
//! [`LiquifyWorkspace`] is a [`FullWindowDialog`] with the title strip "Liquify (layer, 66%)", the
//! tool strip at the start edge (the eleven Liquify tools in three groups, then Hand and Zoom), the
//! canvas, and one Standard [`FullWindowSide`] at the end edge: the Properties panel (Brush Tool
//! Options, Brush Reconstruct Options, Mask Options, View Options) with OK and Cancel at its
//! bottom. No options bar.
//!
//! The library holds the session as plain data: the strokes and whole-field operations in the order
//! the engine replays them ([`LiquifySession`]), with [`EditHistory`] undo. The app owns the
//! displacement field and its preview: it applies each [`LiquifyEvent`] to the field as it comes,
//! replays the list after undo, redo and Restore All, and shows the result through
//! [`LiquifySource`]. OK runs `filter.liquify` with [`LiquifySession::steps`]; Cancel changes
//! nothing.
//!
//! ```ignore
//! let out = app.liquify.show(ui, &PcLiquifySource { proxy: &app.liquify_proxy, doc: &doc });
//! for event in out.events {
//!     match event {
//!         LiquifyEvent::Stroke(e) => app.liquify_engine.stroke(e),
//!         LiquifyEvent::Global(op) => app.liquify_engine.global(op),
//!         LiquifyEvent::Replay(steps) => app.liquify_engine.replay(&steps),
//!         LiquifyEvent::BackdropWanted(of) => app.load_backdrop(of),
//!     }
//! }
//! match out.closed {
//!     Some(DialogRole::Accept) => app.run("filter.liquify", app.liquify.session().steps()),
//!     Some(DialogRole::Cancel) => app.close_liquify(),
//!     _ => {}
//! }
//! app.prefs.liquify = app.liquify.settings().clone();
//! ```
//!
//! Files: `session` (the steps, the settings), `tools` (the tools and the stroke
//! recorder), `commands` (the workspace keys), `panel` (the Properties panel),
//! `canvas` (the canvas tool, the backdrop, the mask and mesh, the brush resize), this file
//! (`show`, `step`).
//!
//! [`FullWindowDialog`]: craft_ui::dialog::FullWindowDialog
//! [`FullWindowSide`]: craft_ui::dialog::FullWindowSide
//! [`EditHistory`]: craft_behavior::EditHistory

mod canvas;
mod commands;
mod panel;
mod session;
pub(crate) mod stories;
mod tools;
mod words;

pub use canvas::{
    BackdropLayering, LassoLine, LassoLineStyle, LassoTone, LayeredImage, LiquifyCanvas, LiquifyCanvasTool, LiquifyMesh, LiquifyTiming,
};
pub use commands::LiquifyCommands;
pub use panel::{LiquifyPanel, LiquifyPanelOutput};
pub use session::{BackdropMode, BackdropUse, LiquifyGlobal, LiquifySession, LiquifySettings, LiquifyStep, LiquifyStroke, MeshSize, StrokeEvent};
pub use tools::{LiquifyBrush, LiquifyTool, StrokeRecorder};
pub use words::LiquifyWords;

use std::cell::RefCell;
use std::sync::Arc;

use egui::{Color32, Id, Response, Ui};

use craft_behavior::edit_history::Step;
use craft_behavior::kurbo::Size;
use craft_behavior::{CancelReason, Cursor, EditEvent, EditHistory, GestureId, Mixed, PixelSize};
use craft_ui::canvas::{NavigationOptions, PickStep, RasterSource, ToolHost, ToolPath, tool_pick};
use craft_ui::canvas_controls::brush::{BrushCursorOptions, TipFalloff};
use craft_ui::contract::{Bind, NoValue, Outcome, Pattern, Remember as _, Show as _, ShowContainer as _};
use craft_ui::dialog::{FullWindowCanvas, FullWindowDialog, FullWindowSide, FullWindowTitle, FullWindowView, FullWindowZoomKeys, RasterOverlay};
use craft_ui::layout_components::Row;
use craft_ui::primitives::Checker;
use craft_ui::rails::{LastUsed, ToolRail};
use craft_ui::runtime::commands::{CommandSet as _, Commands as _};
use craft_ui::runtime::env::Env;
use craft_ui::runtime::words::CommonWords;
use craft_ui::sections::{DialogRole, FooterButton};

use canvas::MaskAtTexel;
use panel::backdrop_id;

use crate::ui::warp::MeshLines;

/// Photoshop's default Mask Color for Liquify's freeze mask: red at half opacity. The mask source is white with the
/// frozen amount as alpha, so the tint carries the colour. It is data the user's Mask Color setting replaces, not a
/// theme token.
const FREEZE_MASK_TINT: Color32 = Color32::from_rgba_premultiplied(0x80, 0x00, 0x00, 0x80);

/// What the app gives the workspace to show each pass: images and names only. The field, the
/// proxy and the backdrop render stay the app's (PhotoCraft's `photocraft_algo::liquify`).
pub trait LiquifySource {
    /// The liquified layer as the field shows it now (the app's proxy, or its full-resolution crop
    /// above 100 %), in document pixels.
    fn picture(&self) -> &dyn RasterSource;

    /// The freeze mask, white with the frozen amount as alpha, and its texel size in document
    /// pixels. The library tints it with the freeze while Show Mask is on.
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
    pub checkerboard: Checker,
    /// The user's cursor options (the brush outline); Photoshop's by default.
    pub cursor_options: Option<BrushCursorOptions>,
}

impl LiquifyOpen {
    /// Opens on a layer of `size` document pixels named `subject`; every other option takes its default.
    pub fn new(size: [u32; 2], subject: impl Into<String>) -> Self {
        Self { size, subject: subject.into(), ..Self::default() }
    }

    /// The remembered settings.
    #[must_use]
    pub fn settings(mut self, settings: LiquifySettings) -> Self {
        self.settings = settings;
        self
    }

    /// The steps of a smart filter opened again (PhotoCraft's `open_smart_filter`).
    #[must_use]
    pub fn steps(mut self, steps: Vec<LiquifyStep>) -> Self {
        self.steps = steps;
        self
    }

    /// The user's navigation options (wheel, flick, animated zoom), for the canvas and the Hand
    /// and Zoom tools.
    #[must_use]
    pub fn navigation(mut self, navigation: NavigationOptions) -> Self {
        self.navigation = navigation;
        self
    }

    /// The user's transparency grid under the image.
    #[must_use]
    pub fn checkerboard(mut self, checkerboard: Checker) -> Self {
        self.checkerboard = checkerboard;
        self
    }

    /// The user's cursor options for the brush outline.
    #[must_use]
    pub fn cursor_options(mut self, options: BrushCursorOptions) -> Self {
        self.cursor_options = Some(options);
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
    /// lost pointer, a tool switch or an undo during a stroke) is followed by a
    /// [`Replay`](Self::Replay), since the app's field already took the dabs. A lasso reports only
    /// `Commit`.
    Stroke(StrokeEvent),
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
    /// OK (`Accept`) or Cancel this pass.
    pub closed: Option<DialogRole>,
    /// The dialog's response: its rect is the workspace's.
    pub response: Response,
    /// The settings changed this pass (compared before and after, whatever changed them: the
    /// panel, the keys, the strip or a brush resize); the app may persist them.
    pub settings_changed: bool,
    /// The cursor the canvas tool asked for this pass: the brush outline over the canvas, none off it.
    pub cursor: Option<Cursor>,
    /// The timing of the latest brush stroke, for the app's diagnostics (PhotoCraft's stats line
    /// is not drawn; lead call 14); `None` before the first stroke.
    pub timing: Option<LiquifyTiming>,
}

impl Outcome for LiquifyOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// What the dialog's body keeps between passes: the view, the strip's memory and the tool in progress. None of it is
/// the app's, and none of it is in the history.
#[derive(Clone, Default)]
struct LiquifyView {
    view: FullWindowView,
    last_used: LastUsed<LiquifyTool>,
    canvas: LiquifyCanvas,
    /// The canvas the tools run under, to cancel their gestures outside a pass.
    canvas_id: Option<Id>,
}

/// The Liquify workspace: the session and its history and the settings, as one value the app keeps while Liquify
/// is open. The view, the strip's memory and the stroke in progress live in the dialog's body.
///
/// Keys ([`LiquifyCommands`], on the dialog's scope): W R E C S B O F D L pick the Liquify tools and H, Z the Hand
/// and Zoom tools; `[` and `]` scale the brush, ⌘H shows or hides the mask, ⌘I inverts the mask, ⌘D clears it. The
/// app forwards the zoom commands to [`zoom`](Self::zoom) and ⌘Z and ⇧⌘Z (`EditCommands::Undo`, `Redo`) to
/// [`step`](Self::step); Enter is OK and Esc Cancel (Esc during a stroke cancels the stroke only). A tool switch or
/// an undo during a stroke cancels it first (`CancelReason::Removed`, `CancelReason::UndoDuringGesture`); the app's
/// field is rebuilt by the [`LiquifyEvent::Replay`] that follows.
///
/// The side column's header is "Properties", as Photoshop's Liquify panel is headed (lead call 89: the one editor
/// with a header besides Select and Mask and Content-Aware Fill).
#[derive(Clone, Pattern)]
pub struct LiquifyWorkspace {
    session: LiquifySession,
    history: EditHistory<LiquifySession>,
    settings: LiquifySettings,
    subject: String,
    size: PixelSize,
    navigation: NavigationOptions,
    checkerboard: Checker,
    cursor_options: BrushCursorOptions,
    /// The backdrop the app was last asked for, while Show Backdrop is on.
    asked: Option<Option<u64>>,
    /// Events that happened between passes ([`step`](Self::step)), reported by the next `show`.
    queued: Vec<LiquifyEvent>,
    /// A step the app asked for while a stroke was open; the next `show` cancels the stroke, then steps.
    requested: Vec<Step>,
    /// Zoom commands the app forwarded; the next `show` applies them to the view.
    zooms: Vec<FullWindowZoomKeys>,
    /// A stroke, a lasso or a resize was open at the end of the last pass.
    busy: bool,
    /// The zoom on screen at the end of the last pass, for the title.
    zoom_percent: f64,
    /// The timing of the latest stroke, as of the last pass.
    timing: Option<LiquifyTiming>,
    /// The cursor the canvas tool asked for in the last pass.
    cursor: Option<Cursor>,
    /// The id the next history entry takes.
    gesture: GestureId,
    /// This pass's events.
    events: Vec<LiquifyEvent>,
}

impl LiquifyWorkspace {
    /// A workspace over `open`, fitted on its first pass.
    pub fn new(open: LiquifyOpen) -> Self {
        let session = LiquifySession::from_steps(open.steps);
        let [width, height] = open.size;
        Self {
            history: EditHistory::new(&session),
            session,
            settings: open.settings,
            subject: open.subject,
            size: PixelSize::new(width, height),
            navigation: open.navigation,
            checkerboard: open.checkerboard,
            cursor_options: open.cursor_options.unwrap_or_else(canvas::default_cursor_options),
            asked: None,
            queued: Vec::new(),
            requested: Vec::new(),
            zooms: Vec::new(),
            busy: false,
            zoom_percent: 100.0,
            timing: None,
            cursor: None,
            gesture: GestureId::default(),
            events: Vec::new(),
        }
    }

    /// The user's navigation options (wheel, flick, animated zoom); Photoshop's by default.
    pub fn navigation(&mut self, navigation: NavigationOptions) {
        self.navigation = navigation;
    }

    /// The user's transparency grid under the image.
    pub fn checkerboard(&mut self, checkerboard: Checker) {
        self.checkerboard = checkerboard;
    }

    /// The user's cursor options for the brush outline.
    pub fn cursor_options(&mut self, options: BrushCursorOptions) {
        self.cursor_options = options;
    }

    /// Shows the workspace for this pass.
    ///
    /// A [`FullWindowDialog`] titled by [`FullWindowTitle`] (the subject and the zoom), in the scope of
    /// [`LiquifyCommands`], with the Properties column ([`LiquifyPanel`], OK and Cancel at its bottom) in the dialog's
    /// side region. The body, in order: the events [`step`](Self::step) queued and the zoom commands
    /// [`zoom`](Self::zoom) took; the commands; the tool strip (a `ToolRail` over [`LiquifyTool::slots`]; a pick
    /// cancels the old tool's gesture with `Removed`); the canvas ([`FullWindowCanvas`]) with the source's image under
    /// [`BackdropLayering`], the freeze mask as a `RasterOverlay` tinted red while Show Mask is on, the mesh while
    /// Show Mesh is on, the lasso and the resize preview as canvas controls, and [`LiquifyCanvasTool`] as its tool.
    /// Undo and redo report [`LiquifyEvent::Replay`] in the same pass.
    pub fn show(&mut self, ui: &mut Ui, source: &dyn LiquifySource) -> LiquifyOutput {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:950 `show` (title, strip, panel, footer,
        // preview, brush circle); :730 `history_keys`; :740 `keys`.
        let env = Env::of(ui);
        let words = env.words::<LiquifyWords>();
        let common = env.words::<CommonWords>();
        let layers = source.layers();
        let title = FullWindowTitle::new(words.name.clone()).subject(self.subject.as_str()).zoom(self.zoom_percent).label(&env);
        let settings_before = self.settings.clone();
        let mut asked_steps = std::mem::take(&mut self.requested);
        let mut zooms = std::mem::take(&mut self.zooms);
        self.events = std::mem::take(&mut self.queued);
        let document = Size::new(f64::from(self.size.width), f64::from(self.size.height));
        let slots = LiquifyTool::slots();
        let this = RefCell::new(&mut *self);

        let shown = FullWindowDialog::new(title)
            .command_scope(LiquifyCommands::SCOPE)
            .buttons([FooterButton::new(common.ok.clone(), DialogRole::Accept), FooterButton::new(common.cancel.clone(), DialogRole::Cancel)])
            .sides(|ui, _| {
                let mut guard = this.borrow_mut();
                let work: &mut LiquifyWorkspace = &mut **guard;
                let panel = FullWindowSide::new(words.properties.clone())
                    .show(ui, |ui, _| LiquifyPanel::new(Bind::bound(&mut work.settings)).layers(&layers).show(ui))
                    .inner;
                if let Some(op) = panel.global {
                    work.record_global(None, op);
                }
                if panel.restore_all {
                    let before = work.session.clone();
                    work.session.restore_all();
                    work.record_history(before);
                    work.events.push(LiquifyEvent::Replay(Vec::new()));
                }
                if let Some(of) = panel.backdrop_wanted {
                    work.events.push(LiquifyEvent::BackdropWanted(of));
                    work.asked = Some(of);
                }
            })
            .show(ui, |ui, cx| {
                let ctx = ui.ctx().clone();
                let mut guard = this.borrow_mut();
                let work: &mut LiquifyWorkspace = &mut **guard;
                let mut state = std::mem::take(cx.keep::<LiquifyView>());
                state.canvas.navigate(document, work.navigation);
                for key in zooms.drain(..) {
                    state.view.apply(key);
                }

                // An open stroke is cancelled first; the replay of the step supersedes the one the cancel reports.
                for step in asked_steps.drain(..) {
                    work.cancel_gesture(&ctx, &mut state, CancelReason::UndoDuringGesture);
                    if work.history.step(&mut work.session, step) {
                        if matches!(work.events.last(), Some(LiquifyEvent::Replay(_))) {
                            work.events.pop();
                        }
                        work.events.push(LiquifyEvent::Replay(work.session.steps().to_vec()));
                    }
                }

                while let Some((command, fired)) = cx.take_command::<LiquifyCommands>() {
                    if let Some(tool) = command.tool() {
                        let slot = slots.iter().find(|slot| slot.tools.iter().any(|entry| entry.id == tool));
                        let tools: Vec<_> = slot.into_iter().flat_map(|slot| slot.tools.iter()).collect();
                        let picked = slot.and_then(|slot| {
                            tool_pick(&tools, slot.id, &work.settings.tool, &state.last_used, PickStep::of(&fired), |_| true)
                                .map(|entry| (slot.id, entry.id))
                        });
                        if let Some((slot_id, tool)) = picked {
                            work.pick(&ctx, &mut state, tool);
                            state.last_used.set(slot_id, tool);
                        }
                        continue;
                    }
                    match command {
                        LiquifyCommands::Smaller => work.settings.brush = work.settings.brush.smaller(),
                        LiquifyCommands::Larger => work.settings.brush = work.settings.brush.larger(),
                        LiquifyCommands::ShowMask => work.settings.show_mask = !work.settings.show_mask,
                        LiquifyCommands::InvertMask => work.record_global(Some(&mut state), LiquifyGlobal::InvertAll),
                        LiquifyCommands::ClearMask => work.record_global(Some(&mut state), LiquifyGlobal::MaskNone),
                        _ => {}
                    }
                }

                if !work.settings.show_backdrop {
                    work.asked = None;
                } else if work.asked.is_none() {
                    // Show Backdrop was already on when the workspace opened.
                    let of = backdrop_id(work.settings.backdrop);
                    work.events.push(LiquifyEvent::BackdropWanted(of));
                    work.asked = Some(of);
                }

                let backdrop = source.backdrop().filter(|_| work.settings.show_backdrop);
                let layering = BackdropLayering::of(backdrop.is_some(), work.settings.backdrop_mode, work.settings.backdrop_opacity);
                let layered = backdrop.filter(|_| !layering.is_plain()).map(|backdrop| LayeredImage::new(source.picture(), backdrop, layering));
                let image: &dyn RasterSource = match &layered {
                    Some(layered) => layered,
                    None => source.picture(),
                };
                let mask =
                    source.freeze_mask().filter(|_| work.settings.show_mask).map(|(mask, texel)| MaskAtTexel { mask, texel, document: image.size() });
                let edges = source.mesh(work.settings.mesh_size).filter(|_| work.settings.show_mesh).map(|mesh| mesh.edges());
                let lasso = state.canvas.lasso_line();
                let falloff = state.canvas.resize().dimensions().map(|dims| TipFalloff::from_hardness(dims.diameter / 2.0, dims.level));
                let mut preview = falloff.as_ref().and_then(|falloff| state.canvas.resize().preview(falloff));

                let shown = Row::new()
                    .fill()
                    .show(ui, |ui, _| {
                        let rail = ToolRail::new(words.tools.clone(), &slots, &work.settings.tool, &state.last_used).show_rail(ui, |_, _| {});
                        if let Some(next) = rail.last_used {
                            state.last_used = next;
                        }
                        if let Some(tool) = rail.select {
                            work.pick(&ctx, &mut state, tool);
                        }
                        let mut canvas = FullWindowCanvas::new(ToolPath::Freehand, &state.view, image)
                            .checkerboard(work.checkerboard)
                            .navigation(work.navigation)
                            .controls(|ui, rect| {
                                if let Some(edges) = &edges {
                                    MeshLines::new(edges).show_at(ui, rect);
                                }
                                if let Some((points, tone)) = &lasso {
                                    LassoLine::new(points).tone(*tone).show_at(ui, rect);
                                }
                                if let Some(preview) = preview.take() {
                                    preview.show_at(ui, rect);
                                }
                            });
                        if let Some(mask) = &mask {
                            canvas = canvas.overlay(RasterOverlay::new(mask).tint(FREEZE_MASK_TINT));
                        }
                        let LiquifyView { canvas: tools, .. } = &mut state;
                        match work.settings.tool {
                            LiquifyTool::Hand => canvas.tool(&mut tools.hand).show(ui),
                            LiquifyTool::Zoom => canvas.tool(&mut tools.zoom).show(ui),
                            _ => {
                                let mut tool = tools.tool(&mut work.settings, &work.cursor_options);
                                canvas.tool(&mut tool).show(ui)
                            }
                        }
                    })
                    .inner;
                state.canvas_id = Some(shown.response.id);
                work.cursor = shown.tool.cursor.clone();
                state.view = shown.view.unwrap_or(shown.shown);
                work.zoom_percent = state.view.zoom_percent();

                work.absorb_canvas_events(&mut state);
                work.busy = state.canvas.busy();
                work.timing = state.canvas.timing();
                *cx.keep::<LiquifyView>() = state;
            });
        let this = this.into_inner();
        let timing = this.timing;
        let picked = shown.output.picked.and_then(|at| [DialogRole::Accept, DialogRole::Cancel].get(at).copied());
        let closed = picked.or(shown.output.closed.map(|_| DialogRole::Cancel));
        LiquifyOutput {
            events: std::mem::take(&mut this.events),
            closed,
            response: shown.output.response,
            settings_changed: this.settings != settings_before,
            cursor: this.cursor.take(),
            timing,
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
    pub fn step(&mut self, step: Step) -> bool {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:340 `undo`, :351 `redo`.
        if self.busy {
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

    /// Applies a zoom command to the workspace's view on the next [`show`](Self::show): the app forwards the fired
    /// `FullWindowZoomKeys`. Returns whether the command will be applied.
    pub fn zoom(&mut self, key: FullWindowZoomKeys) -> bool {
        self.zooms.push(key);
        true
    }
}

impl LiquifyWorkspace {
    /// The id of the next history entry.
    fn next_gesture(&mut self) -> GestureId {
        self.gesture = self.gesture.next();
        self.gesture
    }

    /// Records the session as one history step when it differs from `before`.
    fn record_history(&mut self, before: LiquifySession) {
        if before == self.session {
            return;
        }
        let id = self.next_gesture();
        let name: Arc<str> = Arc::from(HISTORY_NAME);
        self.history.record(&EditEvent::set(id, Mixed::Same(before), self.session.clone(), name, None));
    }

    /// Puts `event` in this pass's events, recording a committed stroke in the session first (the lasso's closing
    /// point included) and following a cancelled one with the replay that rebuilds the app's field.
    fn absorb(&mut self, event: LiquifyEvent) {
        match event {
            LiquifyEvent::Stroke(StrokeEvent::Commit(stroke)) => {
                let before = self.session.clone();
                self.session.record(stroke.clone());
                let recorded = match self.session.steps().last() {
                    Some(LiquifyStep::Stroke(recorded)) if self.session.len() > before.len() => recorded.clone(),
                    _ => stroke,
                };
                self.record_history(before);
                self.events.push(LiquifyEvent::Stroke(StrokeEvent::Commit(recorded)));
            }
            LiquifyEvent::Stroke(cancel @ StrokeEvent::Cancel(_)) => {
                self.events.push(LiquifyEvent::Stroke(cancel));
                self.events.push(LiquifyEvent::Replay(self.session.steps().to_vec()));
            }
            other => self.events.push(other),
        }
    }

    /// Takes the events the canvas tool collected.
    fn absorb_canvas_events(&mut self, state: &mut LiquifyView) {
        for event in state.canvas.take_events() {
            self.absorb(event);
        }
    }

    /// Records a whole-field operation after the stroke in progress, if `state` has one.
    fn record_global(&mut self, state: Option<&mut LiquifyView>, op: LiquifyGlobal) {
        if let Some(state) = state
            && let Some(event) = state.canvas.end_stroke()
        {
            self.absorb(event);
        }
        let before = self.session.clone();
        self.session.record_global(op);
        self.record_history(before);
        self.events.push(LiquifyEvent::Global(op));
    }

    /// Cancels the gesture the current tool holds (a Liquify tool's stroke, lasso or resize, or the Hand's or Zoom's
    /// drag) through the host, and the tool's own state if the host held none.
    fn cancel_gesture(&mut self, ctx: &egui::Context, state: &mut LiquifyView, reason: CancelReason) {
        let cursor = self.cursor_options;
        let LiquifyView { canvas, canvas_id, .. } = state;
        if let Some(id) = *canvas_id {
            match self.settings.tool {
                LiquifyTool::Hand => ToolHost::cancel(ctx, id, &mut canvas.hand, reason),
                LiquifyTool::Zoom => ToolHost::cancel(ctx, id, &mut canvas.zoom, reason),
                _ => ToolHost::cancel(ctx, id, &mut canvas.tool(&mut self.settings, &cursor), reason),
            };
        }
        if canvas.busy() {
            canvas.tool(&mut self.settings, &cursor).abandon(reason);
        }
        self.absorb_canvas_events(state);
    }

    /// Switches to `tool`, cancelling the old tool's gesture first.
    fn pick(&mut self, ctx: &egui::Context, state: &mut LiquifyView, tool: LiquifyTool) {
        if tool != self.settings.tool {
            self.cancel_gesture(ctx, state, CancelReason::Removed);
            self.settings.tool = tool;
        }
    }
}

/// The name the history gives its steps.
const HISTORY_NAME: &str = "Liquify";
