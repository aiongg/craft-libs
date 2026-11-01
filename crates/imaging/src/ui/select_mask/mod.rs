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
//! let out = w.show(ctx, &PcSelectMaskSource { view: app.sm_preview.view(), ants: app.sm_preview.ants() });
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
mod tools;
#[cfg(test)]
mod tools_tests;

pub use bar::{SelectMaskBar, SelectMaskBarOutput};
pub use edits::{EditMode, ObjectTarget, RefineStroke, SelectMaskEdit, round_point};
pub use labels::SelectMaskLabels;
pub use panel::{SelectMaskPanel, SelectMaskPanelOutput};
pub use session::{OutputTo, RefineSettings, SelectMaskCommand, SelectMaskLimits, SelectMaskSession, SelectMaskSettings};
pub use tools::{BrushSizes, SelectMaskCanvas, SelectMaskCanvasTool, SelectMaskTool, SelectMaskToolOptions};

use egui::{Context, Rect};

use craft_ui::canvas::RasterSource;
use craft_ui::dialog::full_window::{FullWindowView, PreviewStatus, PreviewTicket};
use craft_ui::dialog::{DialogState, LocalHistory, Outcome, Step};
use craft_ui::edit::EditEvent;
use craft_ui::patterns::LastUsed;

use crate::trace::MaskSource;
use crate::ui::mask_view::{MaskViewRecipe, MaskViewState};

/// What the app gives the workspace to show each pass.
pub trait SelectMaskSource {
    /// The view as the engine composited it for the last ready preview (the recipe of
    /// [`SelectMaskWorkspace::recipe`]), in document pixels.
    fn view(&self) -> &dyn RasterSource;

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
    pub size: [u32; 2],
    /// The document's name: the title's subject.
    pub subject: String,
    /// The remembered settings when Remember Settings was on, else the defaults.
    pub settings: SelectMaskSettings,
    pub limits: SelectMaskLimits,
    /// The targeted layer mask is refined (`"target": "mask"`), not the selection.
    pub target_mask: bool,
}

impl SelectMaskOpen {
    pub fn new(size: [u32; 2], subject: impl Into<String>, limits: SelectMaskLimits) -> Self {
        Self { size, subject: subject.into(), settings: SelectMaskSettings::default(), limits, target_mask: false }
    }

    pub fn settings(mut self, settings: SelectMaskSettings) -> Self {
        self.settings = settings;
        self
    }

    pub fn target_mask(mut self, target_mask: bool) -> Self {
        self.target_mask = target_mask;
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
    pub events: Vec<SelectMaskEvent>,
    /// The session, the view or a preview option changed this pass (or undo stepped): the app
    /// starts a preview job for this ticket.
    pub preview_wanted: Option<PreviewTicket>,
    pub outcome: Option<Outcome>,
    pub rect: Rect,
    pub settings_changed: bool,
}

/// The Select and Mask workspace.
///
/// Keys (none while a text field edits, except ⌘Z): W, R, B, L, H, Z and Shift+W, Shift+L
/// (`ToolRail::pick_by_key`); `[` and `]` the brush size ([`BrushSizes::step`]); O M V A T K Y, F, J,
/// P and X held ([`MaskViewKeys`](crate::ui::mask_view::MaskViewKeys)); Backspace and Delete the
/// polygon's last vertex; ⌘+, ⌘−, ⌘0, ⌘1 the view; ⌘Z, ⇧⌘Z the session's history; Enter closes a
/// polygon first, else OK; Esc drops a polygon first, else Cancel (`EnterRule::Body`).
#[expect(dead_code, reason = "skeleton: `show` (task WS-SM3) reads the fields")]
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
    /// The limit message, until the next accepted edit.
    error: Option<String>,
}

impl SelectMaskWorkspace {
    /// A workspace over `open`; its first pass asks for a preview.
    pub fn new(open: SelectMaskOpen) -> Self {
        let session = SelectMaskSession { refine: open.settings.refine.clone(), ..SelectMaskSession::default() };
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
            canvas: SelectMaskCanvas::new(),
            preview: PreviewStatus::new(),
            error: None,
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
    #[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-SM3")]
    pub fn show(&mut self, ctx: &Context, source: &dyn SelectMaskSource) -> SelectMaskOutput {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:1812 `show`, :1375 `keys`, :870
        // `enter`, :881 `escape`.
        todo!()
    }

    pub fn session(&self) -> &SelectMaskSession {
        &self.session
    }

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
        self.preview.ready(ticket);
    }

    /// The preview job for `ticket` failed; the message shows under the panel.
    pub fn preview_failed(&mut self, ticket: PreviewTicket, message: String) {
        self.preview.failed(ticket, message);
    }

    /// Undo or redo from the app's Edit menu. Returns whether it stepped; a preview is then asked
    /// for on the next pass's output.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-SM3")]
    pub fn step(&mut self, step: Step) -> bool {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:644 `undo`, :672 `redo`.
        todo!()
    }
}
