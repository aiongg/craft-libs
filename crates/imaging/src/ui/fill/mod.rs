//! The Content-Aware Fill workspace (Edit › Content-Aware Fill…), in Photoshop's layout.
//!
//! [`ContentAwareFillWorkspace`] is a [`FullWindowDialog`] with the title strip "Content-Aware Fill
//! (layer, 66%)", the full-width options bar ([`FillBar`]), the tool strip (Sampling Brush, Lasso,
//! Hand, Zoom), the canvas with the sampling tint and the fill area's ants ([`FillOverlay`]), and
//! two Standard side columns from the end edge inwards: the Content-Aware Fill panel
//! ([`FillPanel`]) with the footer (Reset; OK, Apply, Cancel), and the Preview panel
//! ([`FillPreviewPane`]).
//!
//! PhotoCraft measured the layout and behaviour on Photoshop 25.4 (`caf_ui.rs:1-25`). The session
//! is plain data ([`FillSession`]: the sampling option, the brush strokes, the lasso edits) with
//! [`LocalHistory`] undo; the app's engine plans the fill (`content_aware_plan`), gives the
//! sampling and fill-area planes and the preview through [`FillSource`], and fills on Apply and
//! OK with [`FillCommand`]. Apply keeps the workspace open: the app fills, then calls
//! [`ContentAwareFillWorkspace::rebase`]. Cancel closes, keeping what Apply did.
//!
//! Files: `session`, `lasso` and `tool` (task WS-CA1); `overlay`, `panel`, `preview_pane`, `bar`
//! and this file's bodies (WS-CA2).
//!
//! [`FullWindowDialog`]: craft_ui::dialog::FullWindowDialog

mod bar;
mod labels;
mod lasso;
mod overlay;
mod panel;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod panel_tests;
mod preview_pane;
mod session;
#[cfg(test)]
mod session_tests;
pub(crate) mod stories;
mod tool;

pub use bar::{FillBar, FillBarOutput};
pub use labels::FillLabels;
pub use lasso::FillAreaEdit;
pub use overlay::{FillOverlay, SamplingPlane};
pub use panel::{FillPanel, FillPanelOutput};
pub use preview_pane::FillPreviewPane;
pub use session::{
    ColorAdaptation, FillCommand, FillLimits, FillOutputTo, FillReset, FillSession, FillSettings, RotationAdaptation, SamplingMode, SamplingStroke,
};
pub use tool::{FillCanvas, FillCanvasTool, FillTool, FillToolOptions};

use egui::{Context, Rect};

use craft_ui::canvas::RasterSource;
use craft_ui::canvas::kurbo;
use craft_ui::dialog::full_window::{FullWindowView, PreviewStatus, PreviewTicket};
use craft_ui::dialog::{DialogState, LocalHistory, Outcome, Step};
use craft_ui::edit::EditEvent;
use craft_ui::patterns::LastUsed;

/// What the app gives the workspace to show each pass.
pub trait FillSource {
    /// The document as the canvas shows it (the visible layers, so an Apply to a new layer shows).
    fn image(&self) -> &dyn RasterSource;

    /// The sampling area after the strokes so far (the stroke in progress included), for the tint.
    fn sampling(&self) -> Option<&SamplingPlane>;

    /// The fill area (the selection after the lasso edits), for the ants; never tinted.
    fn fill_area(&self) -> Option<&SamplingPlane>;

    /// The sampling window in document pixels: the Preview panel opens framed on it.
    fn sampling_window(&self) -> Option<kurbo::Rect>;

    /// The last ready preview of the fill, for the Preview panel.
    fn preview(&self) -> Option<&dyn RasterSource>;
}

/// What opens the workspace.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct FillOpen {
    pub size: [u32; 2],
    /// The layer's name: the title's subject.
    pub subject: String,
    pub settings: FillSettings,
    pub limits: FillLimits,
    /// The Sampling Brush's first size: a tenth of the sampling window's side.
    pub brush_size: f64,
}

impl FillOpen {
    pub fn new(size: [u32; 2], subject: impl Into<String>, limits: FillLimits) -> Self {
        Self { size, subject: subject.into(), settings: FillSettings::default(), limits, brush_size: 20.0 }
    }

    pub fn settings(mut self, settings: FillSettings) -> Self {
        self.settings = settings;
        self
    }

    pub fn brush_size(mut self, brush_size: f64) -> Self {
        self.brush_size = brush_size;
        self
    }
}

/// What the workspace reports, in order.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum FillEvent {
    /// A Sampling Brush stroke: `Begin`, `Update` with each new point (the app paints its sampling
    /// plane), `Commit` (recorded), or `Cancel`.
    Stroke(EditEvent<SamplingStroke>),
    /// A lasso polygon, Expand or Contract, recorded in the session: the app re-plans the fill
    /// area and the window. If the edit empties the fill area, the app calls
    /// [`ContentAwareFillWorkspace::refuse_fill_edit`].
    FillArea(FillAreaEdit),
    /// A Reset or a new sampling option changed the session: the app re-plans.
    Reset(FillReset),
    /// A stroke or edit the limits refused, with the message (also under the panel).
    Refused(String),
}

/// What [`ContentAwareFillWorkspace::show`] reports.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct FillOutput {
    pub events: Vec<FillEvent>,
    /// The session or a fill setting changed (or undo stepped): the app starts a preview job.
    pub preview_wanted: Option<PreviewTicket>,
    /// OK, Apply (stays open) or Cancel.
    pub outcome: Option<Outcome>,
    pub rect: Rect,
    pub settings_changed: bool,
}

/// The Content-Aware Fill workspace.
///
/// Keys (none while a text field edits, except ⌘Z): B, L, H, Z (`ToolRail::pick_by_key`); `[` and
/// `]` the brush size ([`FillToolOptions::step_size`]); ⌘H hides or shows the tint; ⌘+, ⌘−, ⌘0, ⌘1 the
/// canvas's view; ⌘Z, ⇧⌘Z the session's history; Enter OK; Esc Cancel (a stroke or lasso in
/// progress first).
#[expect(dead_code, reason = "skeleton: `show` (task WS-CA2) reads the fields")]
pub struct ContentAwareFillWorkspace {
    dialog: DialogState<FillSession>,
    session: FillSession,
    history: LocalHistory<FillSession>,
    settings: FillSettings,
    limits: FillLimits,
    subject: String,
    size: [u32; 2],
    // View state outside the history (risk 8).
    view: FullWindowView,
    preview_view: FullWindowView,
    tool: FillTool,
    last_used: LastUsed<FillTool>,
    options: FillToolOptions,
    canvas: FillCanvas,
    /// ⌘H: the tint hidden without changing Show Sampling Area.
    tint_hidden: bool,
    preview: PreviewStatus,
    error: Option<String>,
}

impl ContentAwareFillWorkspace {
    /// A workspace over `open`; its first pass asks for a preview.
    pub fn new(open: FillOpen) -> Self {
        let session = FillSession { sampling: open.settings.sampling, ..FillSession::default() };
        Self {
            dialog: DialogState::new(),
            history: LocalHistory::new(&session),
            options: FillToolOptions {
                brush_add: open.settings.sampling.brush_adds(),
                brush_size: open.brush_size,
                lasso: craft_ui::canvas::SelectionOperation::New,
                grow_by: 3.0,
            },
            session,
            settings: open.settings,
            limits: open.limits,
            subject: open.subject,
            size: open.size,
            view: FullWindowView::new(),
            preview_view: FullWindowView::new(),
            tool: FillTool::SamplingBrush,
            last_used: LastUsed::default(),
            canvas: FillCanvas::new(),
            tint_hidden: false,
            preview: PreviewStatus::new(),
            error: None,
        }
    }

    /// Shows the workspace for this pass: the title, the keys, [`FillBar`], the strip, the two
    /// side columns ([`FillPanel`] with `SideFooter::bottom` OK, Apply, Cancel and Reset at
    /// `leading`; then [`FillPreviewPane`]), and the canvas with the source's image,
    /// [`FillOverlay`] and [`FillCanvas`]'s tool (or Hand, Zoom). Commits go into the session
    /// within the limits; a sampling option or Reset from the panel goes through
    /// [`FillSession::set_sampling`] and [`FillSession::reset`] (Custom makes the brush add, the
    /// others subtract). Then the history settles and a change asks for a preview ticket. An
    /// Apply or OK the app reports failing ([`Self::apply_failed`]) keeps the workspace open.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-CA2")]
    pub fn show(&mut self, ctx: &Context, source: &dyn FillSource) -> FillOutput {
        // pc: crates/ui-egui/src/dialogs/caf_ui/view.rs:265 `show`; caf_ui/controls.rs:151 `keys`.
        todo!()
    }

    pub fn session(&self) -> &FillSession {
        &self.session
    }

    pub fn settings(&self) -> &FillSettings {
        &self.settings
    }

    /// Apply's and OK's parameters.
    pub fn command(&self) -> FillCommand {
        FillCommand::new(&self.session, &self.settings)
    }

    /// The app applied the fill: the session's history clears (Apply is one document step), the
    /// strokes and edits stay, and a preview is asked for over the new document.
    #[expect(clippy::todo, reason = "skeleton: task WS-CA2")]
    pub fn rebase(&mut self) {
        // pc: crates/ui-egui/src/dialogs/caf_ui/lifecycle.rs:80 `apply` (refresh), caf_ui/preview.rs:135
        // `rebase`.
        todo!()
    }

    /// The app's Apply or OK failed: the message shows under the panel and the workspace stays.
    pub fn apply_failed(&mut self, message: String) {
        self.error = Some(message);
    }

    /// The app found that the last fill-area edit empties the fill area: the edit is taken back
    /// (no redo of it) and `message` shows under the panel.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-CA2")]
    pub fn refuse_fill_edit(&mut self, message: String) {
        // pc: crates/ui-egui/src/dialogs/caf_ui/lifecycle.rs:211-219.
        todo!()
    }

    pub fn preview_ready(&mut self, ticket: PreviewTicket) {
        self.preview.ready(ticket);
    }

    pub fn preview_failed(&mut self, ticket: PreviewTicket, message: String) {
        self.preview.failed(ticket, message);
    }

    /// Undo or redo from the app's Edit menu; returns whether it stepped.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-CA2")]
    pub fn step(&mut self, step: Step) -> bool {
        // pc: crates/ui-egui/src/dialogs/caf_ui/controls.rs:151-159 (⌘Z, ⇧⌘Z).
        todo!()
    }
}
