//! The Select and Mask workspace (Select › Select and Mask…), in Photoshop's layout.
//!
//! [`SelectMaskWorkspace`] is a [`FullWindowDialog`] with the title strip "Select and Mask
//! (document, 66%)" (PhotoCraft's document tab is dropped, lead call 3), the full-width options bar
//! ([`SelectMaskBar`]), the tool strip at the start edge (W, R, B, L, H, Z with the shared W and L
//! slots), the canvas, and a Wide [`FullWindowSide`] at the end edge headed "Properties"
//! ([`SelectMaskPanel`]) with Reset at the footer's leading end and OK and Cancel.
//!
//! The library holds the session as typed data ([`SelectMaskSession`]: edits, Refine Edge Brush
//! strokes, refinement values) with [`LocalHistory`] undo, draws the tools and the panels, and
//! asks for previews with a [`PreviewTicket`]. The app's engine computes the view (the edits and
//! refinement on a reduced copy, composited by the [`MaskViewRecipe`] the workspace gives) and
//! shows it through [`SelectMaskSource`]; OK runs `select.refineEdge` with
//! [`SelectMaskSession::command`]. Cancel changes nothing. The app opens the workspace only with a
//! selection or a layer mask (lead call 26), and closes it when the document or layer changes
//! under it.
//!
//! ```ignore
//! let out = w.show(ctx, &PcSelectMaskSource { view: app.sm_preview.view(), image: app.image(), ants: app.sm_preview.ants() });
//! if let Some(t) = out.preview_wanted { app.sm_job = SmJob::spawn(t, w.session().clone(), w.recipe(), w.settings().high_quality) }
//! if let Some(r) = app.sm_job.poll() { match r.result { Ok(v) => { app.sm_preview.set(v); w.preview_ready(r.ticket); } Err(e) => w.preview_failed(r.ticket, e) } }
//! match out.outcome {
//!     Some(Outcome::Ok) => app.run("select.refineEdge", w.command()),
//!     Some(Outcome::Cancel) => app.close_select_mask(),
//!     _ => {}
//! }
//! if w.settings().remember { prefs.select_mask = w.settings().clone() }
//! ```
//!
//! Files: `session` and `edits` (task WS-SM1); `tools` and `bar` (WS-SM2); `panel` and this file's
//! `show` and `step` (WS-SM3).
//!
//! [`FullWindowDialog`]: craft_ui::dialog::FullWindowDialog
//! [`FullWindowSide`]: craft_ui::dialog::full_window::FullWindowSide

mod bar;
mod edits;
mod labels;
mod panel;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod panel_tests;
mod session;
#[cfg(test)]
mod session_tests;
pub(crate) mod stories;
mod style;
mod tools;
#[cfg(test)]
mod tools_tests;

pub use bar::{SelectMaskBar, SelectMaskBarOutput};
pub use edits::{EditMode, ObjectTarget, RefineStroke, SelectMaskEdit, round_point};
pub use labels::SelectMaskLabels;
pub(crate) use labels::{LIMIT_EDIT_POINTS, LIMIT_EDITS, LIMIT_STROKE_POINTS, LIMIT_STROKES};
pub use panel::{OVERLAY_COLOR_KEY, SelectMaskPanel, SelectMaskPanelOutput};
pub use session::{LimitRefused, OutputTo, RefineSettings, SelectMaskCommand, SelectMaskLimits, SelectMaskSession, SelectMaskSettings};
pub use style::SelectMaskStyle;
pub use tools::{BrushSizes, SelectMaskCanvas, SelectMaskCanvasTool, SelectMaskTool, SelectMaskToolOptions};

use egui::{Context, Event, Key, Modifiers, Rect, Ui};

use craft_theme::Srgba;
use craft_ui::canvas::kurbo::Size;
use craft_ui::canvas::{CanvasTool, Checkerboard, CursorOptions, HandTool, NavigationOptions, RasterSource, ToolHost, ZoomTool};
use craft_ui::controls::IconButton;
use craft_ui::dialog::full_window::{
    FullWindowCanvas, FullWindowLabels, FullWindowSide, FullWindowTitle, FullWindowView, PreviewStatus, PreviewTicket, SideFooter, SideWidth,
};
use craft_ui::dialog::{ButtonRole, ColorRequest, DialogLabels, DialogState, FooterButton, FullWindowDialog, LocalHistory, Outcome, Step};
use craft_ui::edit::{CancelReason, EditEvent, GestureId};
use craft_ui::icons::Icon;
use craft_ui::localized::Labels as _;
use craft_ui::patterns::{LastUsed, ToolRail};

use crate::trace::MaskSource;
use crate::ui::mask_view::{MaskViewChange, MaskViewKeys, MaskViewRecipe, MaskViewState};
use crate::ui::selection::{OutlineSource, SelectionOverlay};

/// What the app gives the workspace to show each pass.
pub trait SelectMaskSource {
    /// The view as the engine composited it for the last ready preview (the recipe of
    /// [`SelectMaskWorkspace::recipe`]), in document pixels.
    fn view(&self) -> &dyn RasterSource;

    /// The document's image as it is, in document pixels, with no view applied: the workspace
    /// draws it while X is held ([`MaskViewRecipe::hidden`]), without waiting for a preview.
    fn image(&self) -> &dyn RasterSource;

    /// The refined selection for Marching Ants and its reduction factor (document pixels per mask
    /// pixel; 1 with High Quality Preview), drawn by `SelectionOverlay` with `reduction(k)`.
    /// `None` while there is none; PhotoCraft caps the ants at 200 000 edges and 4 000 000
    /// pixels.
    fn ants(&self) -> Option<(&dyn MaskSource, u32)>;
}

/// What opens the workspace.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct SelectMaskOpen {
    /// The document's size in pixels.
    pub size: [u32; 2],
    /// The document's name: the title's subject.
    pub subject: String,
    /// The remembered settings when Remember Settings was on, else the defaults.
    pub settings: SelectMaskSettings,
    /// The limits on edits, strokes and points.
    pub limits: SelectMaskLimits,
    /// The targeted layer mask is refined (`"target": "mask"`), not the selection.
    pub target_mask: bool,
    /// The user's wheel, flick and animated-zoom options.
    pub navigation: NavigationOptions,
    /// The user's transparency grid under the image.
    pub checkerboard: Checkerboard,
    /// Preferences › Cursors, which shape the brush outline.
    pub cursor_options: CursorOptions,
}

impl SelectMaskOpen {
    /// Opens on a document of `size` pixels named `subject`; settings and options take their defaults.
    pub fn new(size: [u32; 2], subject: impl Into<String>, limits: SelectMaskLimits) -> Self {
        Self {
            size,
            subject: subject.into(),
            settings: SelectMaskSettings::default(),
            limits,
            target_mask: false,
            navigation: NavigationOptions::default(),
            checkerboard: Checkerboard::default(),
            cursor_options: CursorOptions::default(),
        }
    }

    /// Sets the starting settings.
    pub fn settings(mut self, settings: SelectMaskSettings) -> Self {
        self.settings = settings;
        self
    }

    /// Sets whether the layer mask is refined instead of the selection.
    pub fn target_mask(mut self, target_mask: bool) -> Self {
        self.target_mask = target_mask;
        self
    }

    /// The user's navigation options; Photoshop's by default.
    pub fn navigation(mut self, navigation: NavigationOptions) -> Self {
        self.navigation = navigation;
        self
    }

    /// The user's transparency grid.
    pub fn checkerboard(mut self, checkerboard: Checkerboard) -> Self {
        self.checkerboard = checkerboard;
        self
    }

    /// The user's Preferences › Cursors.
    pub fn cursor_options(mut self, cursor_options: CursorOptions) -> Self {
        self.cursor_options = cursor_options;
        self
    }
}

/// What the workspace reports, in order.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum SelectMaskEvent {
    /// A tool edit: `Begin`, `Update` with the edit so far (only with Real-Time Refinement on, so
    /// the preview can show the stroke as it is drawn), then `Commit` with the edit, already
    /// recorded in the session, or `Cancel` (refused by the limits, Esc, a lost pointer). Select
    /// Subject, Clear Selection and Invert are `Set`.
    Edit(EditEvent<SelectMaskEdit>),
    /// A Refine Edge Brush stroke, the same way.
    Refine(EditEvent<RefineStroke>),
}

/// What [`SelectMaskWorkspace::show`] reports.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct SelectMaskOutput {
    /// What happened this pass, in order.
    pub events: Vec<SelectMaskEvent>,
    /// The session, the view or a preview option changed this pass (or undo stepped): the app
    /// starts a preview job for this ticket.
    pub preview_wanted: Option<PreviewTicket>,
    /// OK or Cancel, once chosen.
    pub outcome: Option<Outcome>,
    /// The workspace's screen rectangle.
    pub rect: Rect,
    /// A setting changed this pass.
    pub settings_changed: bool,
    /// The Overlay colour swatch was pressed: open the app's colour picker with this request
    /// (keyed [`OVERLAY_COLOR_KEY`]) and answer with [`SelectMaskWorkspace::set_color`].
    pub pick_color: Option<ColorRequest>,
}

/// The Select and Mask workspace.
///
/// Keys (none while a text field edits, except ⌘Z): W, R, B, L, H, Z and Shift+W, Shift+L
/// (`ToolRail::pick_by_key`); `[` and `]` the brush size ([`BrushSizes::step`]); O M V A T K Y, F, J,
/// P and X held ([`MaskViewKeys`]); Backspace and Delete the
/// polygon's last vertex; ⌘+, ⌘−, ⌘0, ⌘1 the view; ⌘Z, ⇧⌘Z the session's history; Enter closes a
/// polygon first, else OK; Esc drops a polygon first, else Cancel (the dialog answers them; the workspace consumes the press it used).
pub struct SelectMaskWorkspace {
    dialog: DialogState<SelectMaskSession>,
    session: SelectMaskSession,
    history: LocalHistory<SelectMaskSession>,
    settings: SelectMaskSettings,
    limits: SelectMaskLimits,
    target_mask: bool,
    subject: String,
    size: [u32; 2],
    // View state outside the history (risk 8).
    view: FullWindowView,
    mask_view: MaskViewState,
    tool: SelectMaskTool,
    last_used: LastUsed<SelectMaskTool>,
    options: SelectMaskToolOptions,
    canvas: SelectMaskCanvas,
    preview: PreviewStatus,
    /// The limit that refused an edit, until the next accepted edit; worded each pass.
    error: Option<LimitRefused>,
    /// The refinement slider gesture the panel last reported open; it ends with the control's
    /// own gesture ([`craft_ui::edit::is_live`]).
    panel_gesture: Option<GestureId>,
    /// The Hand and Zoom strip tools (they keep their drag between passes).
    hand: HandTool,
    zoom: ZoomTool,
    navigation: NavigationOptions,
    checkerboard: Checkerboard,
    /// A preview is owed: the first pass, an undo or redo from the Edit menu, a colour answer.
    owed: bool,
}

impl SelectMaskWorkspace {
    /// A workspace over `open`; its first pass asks for a preview.
    pub fn new(open: SelectMaskOpen) -> Self {
        let session = SelectMaskSession { refine: open.settings.refine.clone(), ..SelectMaskSession::default() };
        let mut canvas = SelectMaskCanvas::new();
        canvas.cursor_options(open.cursor_options);
        Self {
            dialog: DialogState::new(),
            history: LocalHistory::new(&session),
            session,
            mask_view: open.settings.view_state(),
            settings: open.settings,
            limits: open.limits,
            target_mask: open.target_mask,
            subject: open.subject,
            size: open.size,
            view: FullWindowView::new(),
            tool: SelectMaskTool::Quick,
            last_used: LastUsed::default(),
            options: SelectMaskToolOptions::default(),
            canvas,
            preview: PreviewStatus::new(),
            error: None,
            panel_gesture: None,
            hand: HandTool::default(),
            zoom: ZoomTool::new(Size::new(f64::from(open.size[0]), f64::from(open.size[1])), open.navigation),
            navigation: open.navigation,
            checkerboard: open.checkerboard,
            owed: true,
        }
    }

    /// The user's navigation options (wheel, flick, animated zoom); Photoshop's by default.
    pub fn navigation(&mut self, navigation: NavigationOptions) -> &mut Self {
        self.navigation = navigation;
        self
    }

    /// The user's transparency grid under the image.
    pub fn checkerboard(&mut self, checkerboard: Checkerboard) -> &mut Self {
        self.checkerboard = checkerboard;
        self
    }

    /// The user's Preferences › Cursors, which shape the brush outline.
    pub fn cursor_options(&mut self, cursor_options: CursorOptions) -> &mut Self {
        self.canvas.cursor_options(cursor_options);
        self
    }

    /// The app's colour picker answered a [`SelectMaskOutput::pick_color`] request. A key this
    /// workspace did not ask for is ignored.
    pub fn set_color(&mut self, key: &str, color: Srgba) {
        if key == OVERLAY_COLOR_KEY && self.mask_view.overlay_color != color {
            self.mask_view.overlay_color = color;
            self.settings.take_view(&self.mask_view);
            self.owed = true;
        }
    }

    /// Shows the workspace for this pass: the title, the keys, [`SelectMaskBar`], the strip, the
    /// side column ([`SelectMaskPanel`], footer `SideFooter::bottom` with OK and Cancel and Reset
    /// at `leading`), the canvas (the source's view over the checkerboard, the ants through
    /// `SelectionOverlay` while the recipe has them, [`SelectMaskCanvas`]'s tool or the Hand and
    /// Zoom tools). Commits from the canvas go through [`SelectMaskSession::add_edit`] and
    /// [`SelectMaskSession::add_stroke`]; a refusal sets the error and reports `Cancel`. Then the
    /// history settles (busy while the canvas or a slider drags), the settings take the view and
    /// refinement values, and a change of the session, the view or a preview option requests a
    /// ticket from the [`PreviewStatus`].
    pub fn show(&mut self, ctx: &Context, source: &dyn SelectMaskSource) -> SelectMaskOutput {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:1812 `show`, :1375 `keys`, :870
        // `enter`, :881 `escape`.
        let labels = SelectMaskLabels::of_ctx(ctx);
        let words = DialogLabels::of_ctx(ctx);
        let frame_labels = FullWindowLabels::of_ctx(ctx);
        let title = FullWindowTitle::new(labels.name.as_str()).subject(self.subject.as_str()).zoom(self.view.zoom_percent()).text(ctx);
        let settings_before = self.settings.clone();
        let document = Size::new(f64::from(self.size[0]), f64::from(self.size[1]));
        for (size, prefs) in [(&mut self.hand.size, &mut self.hand.prefs), (&mut self.zoom.size, &mut self.zoom.prefs)] {
            *size = document;
            *prefs = self.navigation;
        }
        let Self {
            dialog,
            session,
            history,
            settings,
            limits,
            view,
            mask_view,
            tool,
            last_used,
            options,
            canvas,
            preview,
            error,
            panel_gesture,
            hand,
            zoom,
            navigation,
            checkerboard,
            owed,
            ..
        } = self;
        let (navigation, checkerboard) = (*navigation, *checkerboard);

        let mut run = Run::default();
        let mut reset = false;
        let response = FullWindowDialog::new("select-mask").title(title).labels(words.clone()).show(ctx, dialog, session, |ui, session, fx| {
            let ctx = ui.ctx().clone();
            if let Some(step) = fx.take_step(ui) {
                // A stroke under way ends first, so its Commit never lands on the restored session.
                cancel_gesture(ui, *tool, canvas, options, settings.real_time, (&mut *hand, &mut *zoom), view, CancelReason::UndoDuringGesture);
                canvas.drop_polygon();
                if history.step(session, step) {
                    *error = None;
                    run.session = true;
                }
            }
            if fx.keys_free(ui) {
                if tool.brushes() {
                    for (key, larger) in [(Key::OpenBracket, false), (Key::CloseBracket, true)] {
                        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, key)) {
                            options.sizes.step(*tool, larger);
                        }
                    }
                }
                if canvas.polygon_open() {
                    if take_enter(&ctx) {
                        canvas.close_polygon();
                    } else if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
                        canvas.drop_polygon();
                    }
                }
            }
            // Holding X shows the document's image at once: it changes no preview.
            run.view |= MaskViewKeys::read(ui, mask_view).iter().any(|change| !matches!(change, MaskViewChange::HideView(_)));

            let bar = SelectMaskBar::new("select-mask-bar", *tool, options).show(ui);
            run.sample = bar.sample_all_layers;
            if bar.select_subject {
                let edit = SelectMaskEdit::Subject { sample_all_layers: options.sample_all_layers };
                run.absorb_edit(session, limits, error, edit);
            }

            let slots = SelectMaskTool::slots(&labels);
            let rail = ToolRail::new("select-mask-tools").label(labels.tools.as_str()).pick_by_key(true).show(ui, &slots, tool, last_used, |_, _| {});
            if let Some(next) = rail.select
                && next != *tool
            {
                cancel_gesture(ui, *tool, canvas, options, settings.real_time, (&mut *hand, &mut *zoom), view, CancelReason::Removed);
                canvas.drop_polygon();
                *tool = next;
            }

            let footer = SideFooter::bottom(vec![
                FooterButton::new(ButtonRole::Default, words.ok.as_str()),
                FooterButton::new(ButtonRole::Cancel, words.cancel.as_str()),
            ])
            .leading(|ui| reset = ui.add(IconButton::new(Icon::Undo, words.reset.as_str())).clicked());
            FullWindowSide::new("select-mask-properties").width(SideWidth::Wide).header(labels.properties.as_str()).footer(footer).show(
                ui,
                fx,
                |ui| {
                    let message = error.map(|why| why.message(ui.ctx()));
                    let refine_before = session.refine.clone();
                    let panel =
                        SelectMaskPanel::new("select-mask-panel", settings, &mut session.refine, mask_view).error(message.as_deref()).show(ui);
                    run.session |= session.refine != refine_before;
                    run.view |= panel.view_changed;
                    if let Some(event) = &panel.edit {
                        *panel_gesture = match event {
                            EditEvent::Begin { id, .. } | EditEvent::Update { id, .. } => Some(*id),
                            _ => None,
                        };
                    }
                    run.dragging = panel_gesture.is_some_and(|id| craft_ui::edit::is_live(ui.ctx(), id));
                    run.pick_color = panel.pick_color;
                    run.panel_edit = panel.selection_edit;
                    preview.show(ui, &frame_labels);
                },
            );
            if let Some(edit) = run.panel_edit.take() {
                run.absorb_edit(session, limits, error, edit);
            }

            let hand_zoom_active: Option<&mut dyn CanvasTool> = match *tool {
                SelectMaskTool::Hand => Some(hand),
                SelectMaskTool::Zoom => Some(zoom),
                _ => None,
            };
            let mut brush = canvas.tool(*tool, options, settings.real_time);
            let active: &mut dyn CanvasTool = match hand_zoom_active {
                Some(navigation_tool) => navigation_tool,
                None => &mut brush,
            };
            let recipe = mask_view.recipe();
            let shown = FullWindowCanvas::new(CANVAS, view)
                .source(if recipe.hidden { source.image() } else { source.view() })
                .checkerboard(checkerboard)
                .navigation(navigation)
                .tool(active)
                .show(ui);
            if shown.took_key.is_some_and(|k| k.key == Key::Escape) {
                // The Esc that cancelled a stroke is not the dialog's Cancel.
                ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
            }
            if recipe.ants
                && !recipe.hidden
                && let Some((mask, reduction)) = source.ants()
            {
                SelectionOverlay::new("select-mask-ants", OutlineSource::Mask(mask)).reduction(reduction).show(ui, &shown.response, &shown.view);
            }
            run.absorb_canvas(canvas, session, limits, error, settings.real_time);
        });

        if matches!(response.outcome, Some(Outcome::Ok)) && canvas.close_polygon() {
            run.absorb_canvas(canvas, session, limits, error, settings.real_time);
        }
        if reset {
            canvas.drop_polygon();
            session.reset();
            *error = None;
            run.session = true;
        }
        history.settle(session, canvas.busy() || run.dragging);
        settings.take_view(mask_view);
        settings.refine = session.refine.clone();
        let wanted = std::mem::take(owed) | run.session | run.view | run.stream | run.refused | run.sample;
        SelectMaskOutput {
            events: run.events,
            preview_wanted: wanted.then(|| preview.request()),
            outcome: response.outcome,
            rect: response.rect,
            settings_changed: *settings != settings_before,
            pick_color: run.pick_color,
        }
    }

    /// The edits and strokes recorded so far.
    pub fn session(&self) -> &SelectMaskSession {
        &self.session
    }

    /// The current settings.
    pub fn settings(&self) -> &SelectMaskSettings {
        &self.settings
    }

    /// OK's parameters: [`SelectMaskSession::command`] with the bar's Sample All Layers and the
    /// mask target.
    pub fn command(&self) -> SelectMaskCommand {
        self.session.command(self.options.sample_all_layers, self.target_mask)
    }

    /// What the engine composites for the current view (`MaskViewState::recipe`).
    pub fn recipe(&self) -> MaskViewRecipe {
        self.mask_view.recipe()
    }

    /// The preview job for `ticket` finished; an older ticket is ignored.
    pub fn preview_ready(&mut self, ticket: PreviewTicket) {
        if self.preview.ready(ticket) {
            self.canvas.drop_trail();
        }
    }

    /// The preview job for `ticket` failed; the message shows under the panel.
    pub fn preview_failed(&mut self, ticket: PreviewTicket, message: String) {
        if self.preview.failed(ticket, message) {
            self.canvas.drop_trail();
        }
    }

    /// Undo or redo from the app's Edit menu. Returns whether it stepped; a preview is then asked
    /// for on the next pass's output.
    pub fn step(&mut self, step: Step) -> bool {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:644 `undo`, :672 `redo`.
        self.canvas.drop_polygon();
        let moved = self.history.step(&mut self.session, step);
        if moved {
            self.error = None;
            self.settings.refine = self.session.refine.clone();
            self.owed = true;
        }
        moved
    }
}

/// The canvas's id salt.
const CANVAS: &str = "select-mask-canvas";

/// Ends the gesture the active `tool` holds on the canvas, without a commit: the tool reports
/// `Cancel` with `reason` through `canvas`'s events, and the Hand's view is put back.
#[expect(clippy::too_many_arguments, reason = "the tool, its state and the view are separate fields of the workspace")]
fn cancel_gesture(
    ui: &Ui,
    tool: SelectMaskTool,
    canvas: &mut SelectMaskCanvas,
    options: &mut SelectMaskToolOptions,
    real_time: bool,
    (hand, zoom): (&mut HandTool, &mut ZoomTool),
    view: &mut FullWindowView,
    reason: CancelReason,
) {
    let (ctx, id) = (ui.ctx(), FullWindowCanvas::canvas_id(ui, CANVAS));
    let back = match tool {
        SelectMaskTool::Hand => ToolHost::cancel(ctx, id, hand, reason),
        SelectMaskTool::Zoom => ToolHost::cancel(ctx, id, zoom, reason),
        _ => ToolHost::cancel(ctx, id, &mut canvas.tool(tool, options, real_time), reason),
    };
    if let Some(back) = back {
        view.take(&back, ctx.pixels_per_point());
    }
}

/// What one pass of [`SelectMaskWorkspace::show`] collected.
#[derive(Default)]
struct Run {
    events: Vec<SelectMaskEvent>,
    /// The session changed (an edit, a refinement value, a step).
    session: bool,
    /// The view or a preview option changed.
    view: bool,
    /// A stroke grew with Real-Time Refinement on.
    stream: bool,
    /// The limits refused an edit.
    refused: bool,
    /// A refinement slider's gesture is open.
    dragging: bool,
    /// Sample All Layers changed.
    sample: bool,
    pick_color: Option<ColorRequest>,
    panel_edit: Option<SelectMaskEdit>,
}

impl Run {
    /// Records a button's edit (Select Subject, Clear Selection, Invert) as a `Set`.
    fn absorb_edit(&mut self, session: &mut SelectMaskSession, limits: &SelectMaskLimits, error: &mut Option<LimitRefused>, edit: SelectMaskEdit) {
        match session.add_edit(edit.clone(), limits) {
            Ok(()) => {
                *error = None;
                self.session = true;
                self.events.push(SelectMaskEvent::Edit(EditEvent::set(edit, Modifiers::NONE)));
            }
            Err(why) => {
                *error = Some(why);
                self.refused = true;
            }
        }
    }

    /// Takes the canvas's events: a commit is recorded in the session within the limits, or
    /// refused (the message shows under the panel, and the event becomes a `Cancel`).
    fn absorb_canvas(
        &mut self,
        canvas: &mut SelectMaskCanvas,
        session: &mut SelectMaskSession,
        limits: &SelectMaskLimits,
        error: &mut Option<LimitRefused>,
        real_time: bool,
    ) {
        for event in canvas.take_events() {
            let event = match event {
                SelectMaskEvent::Edit(EditEvent::Commit { id, value, changed, modifiers, .. }) => {
                    let recorded = !changed || session.add_edit(value.clone(), limits).map_err(|why| *error = Some(why)).is_ok();
                    SelectMaskEvent::Edit(self.settle(recorded, error, EditEvent::commit(id, value, changed, modifiers), id))
                }
                SelectMaskEvent::Refine(EditEvent::Commit { id, value, changed, modifiers, .. }) => {
                    let recorded = !changed || session.add_stroke(value.clone(), limits).map_err(|why| *error = Some(why)).is_ok();
                    SelectMaskEvent::Refine(self.settle(recorded, error, EditEvent::commit(id, value, changed, modifiers), id))
                }
                SelectMaskEvent::Edit(update @ EditEvent::Update { .. }) => {
                    self.stream |= real_time;
                    SelectMaskEvent::Edit(update)
                }
                SelectMaskEvent::Refine(update @ EditEvent::Update { .. }) => {
                    self.stream |= real_time;
                    SelectMaskEvent::Refine(update)
                }
                other => other,
            };
            self.events.push(event);
        }
    }

    /// `commit` when it was recorded, else the refusal as a `Cancel`.
    fn settle<T>(&mut self, recorded: bool, error: &mut Option<LimitRefused>, commit: EditEvent<T>, id: craft_ui::edit::GestureId) -> EditEvent<T> {
        if recorded {
            if matches!(commit, EditEvent::Commit { changed: true, .. }) {
                *error = None;
                self.session = true;
            }
            commit
        } else {
            self.refused = true;
            EditEvent::cancel(id, CancelReason::External)
        }
    }
}

/// A plain Enter press this pass, removed from the input so the dialog does not answer OK.
fn take_enter(ctx: &Context) -> bool {
    let is_enter = |e: &Event| matches!(e, Event::Key { key: Key::Enter, pressed: true, repeat: false, modifiers, .. } if !modifiers.command && !modifiers.ctrl && !modifiers.alt);
    let pressed = ctx.input(|i| i.events.iter().any(is_enter));
    if pressed {
        ctx.input_mut(|i| i.events.retain(|e| !is_enter(e)));
    }
    pressed
}
