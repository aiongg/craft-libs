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

pub use canvas::{BackdropLayering, LiquifyCanvas, LiquifyCanvasTool, LiquifyMesh};
pub use keys::LiquifyKeys;
pub use labels::LiquifyLabels;
pub use panel::{LiquifyPanel, LiquifyPanelOutput};
pub use session::{BackdropMode, BackdropUse, LiquifyGlobal, LiquifySession, LiquifySettings, LiquifyStep, LiquifyStroke, MeshSize};
pub use style::LiquifyStyle;
pub use tools::{LiquifyBrush, LiquifyTool, StrokeRecorder};

use egui::{Context, Rect};

use craft_ui::canvas::RasterSource;
use craft_ui::dialog::full_window::FullWindowView;
use craft_ui::dialog::{DialogState, LocalHistory, Outcome, Step};
use craft_ui::edit::EditEvent;
use craft_ui::patterns::LastUsed;

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
}

impl LiquifyOpen {
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
}

/// What the workspace reports to the app, in the order it happened this pass.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum LiquifyEvent {
    /// A brush stroke or a Freeze Lasso polygon. `Begin` carries the stroke with its first point,
    /// `Update` the stroke with every point so far (the app applies the points it has not seen
    /// to its field), `Commit` the whole stroke, now recorded in the session; `Cancel` (Esc or a
    /// lost pointer during a stroke) is followed by a [`Replay`](Self::Replay), since the app's
    /// field already took the dabs. A lasso reports only `Commit`.
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
    pub events: Vec<LiquifyEvent>,
    /// OK or Cancel this pass.
    pub outcome: Option<Outcome>,
    /// The workspace's rect.
    pub rect: Rect,
    /// The settings changed this pass; the app may persist them.
    pub settings_changed: bool,
}

/// The Liquify workspace: the dialog, the session and its history, the settings, the view and the
/// canvas tool, as one value the app keeps while Liquify is open.
///
/// Keys (none while a text field edits, except ⌘Z): W R E C S B O F D L pick the Liquify tools and
/// H, Z the Hand and Zoom tools ([`ToolRail::pick_by_key`](craft_ui::patterns::ToolRail::pick_by_key));
/// `[` and `]` scale the brush, ⌘H shows or hides the mask, ⌘I inverts the mask, ⌘D clears it
/// ([`LiquifyKeys`]); ⌘+, ⌘−, ⌘0, ⌘1 zoom the workspace's view; ⌘Z and ⇧⌘Z step the session's
/// history, also while a number field has focus; Enter is OK and Esc Cancel (Esc during a stroke
/// cancels the stroke only).
#[expect(dead_code, reason = "skeleton: `show` (task WS-LQ2) reads the fields")]
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
}

impl LiquifyWorkspace {
    /// A workspace over `open`, fitted on its first pass.
    pub fn new(open: LiquifyOpen) -> Self {
        let session = LiquifySession::from_steps(open.steps);
        Self {
            dialog: DialogState::new(),
            history: LocalHistory::new(&session),
            session,
            canvas: LiquifyCanvas::new(),
            settings: open.settings,
            subject: open.subject,
            size: open.size,
            view: FullWindowView::new(),
            last_used: LastUsed::default(),
        }
    }

    /// Shows the workspace for this pass.
    ///
    /// In order: the title ([`FullWindowTitle`](craft_ui::dialog::full_window::FullWindowTitle)
    /// with the subject and the zoom); [`LiquifyKeys`] and the history keys (`take_step`, read
    /// whatever has focus, so ⌘Z works while a number field has focus); the tool strip (a
    /// `ToolRail` with `pick_by_key(true)` over [`LiquifyTool::slots`]); the side column
    /// ([`LiquifyPanel`], footer `SideFooter::bottom` with OK and Cancel); the canvas
    /// ([`FullWindowCanvas`](craft_ui::dialog::full_window::FullWindowCanvas)) with the source's
    /// image under [`BackdropLayering`], the freeze mask as a `RasterOverlay` tinted
    /// `liquify.freeze` while Show Mask is on, the mesh through `warp::paint_mesh` while Show Mesh
    /// is on, and [`LiquifyCanvas`] as its tool. Then `LocalHistory::settle(&session, busy)` with
    /// `busy` while a stroke or a slider drag runs. Undo and redo report
    /// [`LiquifyEvent::Replay`] in the same pass.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-LQ2")]
    pub fn show(&mut self, ctx: &Context, source: &dyn LiquifySource) -> LiquifyOutput {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:950 `show` (title, strip, panel, footer,
        // preview, brush circle); :730 `history_keys`; :740 `keys`.
        todo!()
    }

    /// What OK applies.
    pub fn session(&self) -> &LiquifySession {
        &self.session
    }

    /// What the app persists (always: PhotoCraft remembers Liquify's settings on every close).
    pub fn settings(&self) -> &LiquifySettings {
        &self.settings
    }

    /// Undo or redo from the app's Edit menu: steps the history and returns the
    /// [`LiquifyEvent::Replay`] to apply, or `None` with nothing to step. A stroke in progress ends
    /// first. The workspace reads ⌘Z itself while it is the top modal layer.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-LQ2")]
    pub fn step(&mut self, step: Step) -> Option<LiquifyEvent> {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:340 `undo`, :351 `redo` (a stroke still
        // open ends first).
        todo!()
    }
}
