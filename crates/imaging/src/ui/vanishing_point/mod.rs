//! Vanishing Point (Filter › Vanishing Point…): the plane editor and its full-window workspace.
//!
//! The owner's answer (decisions, "Step 7 questions" 2): Vanishing Point opens in its own window,
//! as in Photoshop; PhotoCraft ran it on the main canvas. [`VanishingPointWorkspace`] is a
//! [`FullWindowDialog`] with the title strip "Vanishing Point (layer, 66%)", the full-width
//! options bar ([`VanishingPointBar`]), the tool strip (Edit Plane V, Create Plane C, Stamp S,
//! then Hand H and Zoom Z), the canvas, and a narrow side column with only OK and Cancel at its
//! bottom (Photoshop has no side panel; the column carries the footer as in the other editors).
//!
//! [`VanishingPointEditor`] is the plane editor without a frame (lead call 46): the workspace runs
//! it in its canvas, and an app may run it on any canvas. The session
//! ([`VanishingPointSession`]: planes, torn planes, paste, clone source and strokes) has
//! [`LocalHistory`] undo. The app's [`VanishingScene`] holds the perspective maths, its
//! [`VanishingPointSource`] paints the stamp preview, and OK runs `filter.vanishingPoint` with the
//! session.
//!
//! Files: `planes` and `editor` (task WS-VP1); `draw`, `bar` and this file's `show` and `step` (WS-VP2);
//! `style` (TK4).
//!
//! [`FullWindowDialog`]: craft_ui::dialog::FullWindowDialog

mod bar;
mod draw;
mod editor;
#[cfg(test)]
mod editor_tests;
mod labels;
mod planes;
pub(crate) mod stories;
mod style;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod workspace_tests;

pub use bar::{VanishingPointBar, VanishingPointBarOutput};
pub use draw::paint_editor;
pub use editor::{
    PasteImage, PlaneEdit, StampOptions, StampStroke, VanishingPointEditor, VanishingPointOptions, VanishingPointPass, VanishingPointSession,
    VanishingTool,
};
pub use labels::VanishingPointLabels;
pub use planes::{MAX_GRID_LINES, MAX_PLANES, PlaneCheck, PlaneEdge, TornPlane, VanishingPlane, VanishingScene, fit_torn, slide_edge};
pub use style::VanishingPointStyle;

use egui::load::SizedTexture;
use egui::{Context, Rect};

use craft_ui::canvas::RasterSource;
use craft_ui::dialog::full_window::{FullWindowView, PreviewTicket};
use craft_ui::dialog::{DialogState, LocalHistory, Outcome, Step};
use craft_ui::edit::EditEvent;

/// The workspace's strip: the editor's tools, Hand and Zoom.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum VanishingPointStripTool {
    #[default]
    Editor,
    Hand,
    Zoom,
}

/// What the app gives the editor to show each pass.
pub trait VanishingPointSource {
    /// The layer as it is.
    fn image(&self) -> &dyn RasterSource;

    /// The clone strokes painted onto a copy of the layer (the app paints them in batches, lead
    /// call 47); `None` while there are none.
    fn stamp_preview(&self) -> Option<&dyn RasterSource>;

    /// The clipboard's image as a texture the app holds, for Paste; `None` without one.
    fn paste(&self) -> Option<SizedTexture>;
}

/// What opens the workspace.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct VanishingPointOpen {
    pub size: [u32; 2],
    pub subject: String,
    pub session: VanishingPointSession,
    pub options: VanishingPointOptions,
}

impl VanishingPointOpen {
    pub fn new(size: [u32; 2], subject: impl Into<String>) -> Self {
        Self { size, subject: subject.into(), session: VanishingPointSession::default(), options: VanishingPointOptions::default() }
    }

    pub fn options(mut self, options: VanishingPointOptions) -> Self {
        self.options = options;
        self
    }
}

/// What the editor reports, in order.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum VanishingPointEvent {
    /// The planes changed: a node or edge drag (Begin, Update, Commit), a plane made, torn off or
    /// deleted (`Set`).
    Planes(EditEvent<PlaneEdit>),
    /// The pasted image moved on its plane, or its Width changed.
    Paste(EditEvent<PasteImage>),
    /// A clone stroke: the app paints its preview from the points as they come.
    Stamp(EditEvent<StampStroke>),
}

/// What [`VanishingPointWorkspace::show`] reports.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct VanishingPointOutput {
    pub events: Vec<VanishingPointEvent>,
    /// The strokes changed other than by painting on (undo, a deleted plane): the app repaints the
    /// stamp preview from scratch for this ticket.
    pub preview_wanted: Option<PreviewTicket>,
    pub outcome: Option<Outcome>,
    pub rect: Rect,
}

/// The Vanishing Point workspace.
///
/// Keys: V, C, S, H, Z (`ToolRail::pick_by_key`); Delete and Backspace the selected plane; ⌘+, ⌘−,
/// ⌘0, ⌘1 the view; ⌘Z, ⇧⌘Z the history; Enter OK; Esc drops pending corners first, else Cancel.
#[expect(dead_code, reason = "skeleton: `show` (task WS-VP2) reads the fields")]
pub struct VanishingPointWorkspace {
    dialog: DialogState<VanishingPointSession>,
    session: VanishingPointSession,
    history: LocalHistory<VanishingPointSession>,
    options: VanishingPointOptions,
    subject: String,
    size: [u32; 2],
    // View state outside the history (risk 8).
    view: FullWindowView,
    strip: VanishingPointStripTool,
    editor: VanishingPointEditor,
    preview_ticket: u64,
}

impl VanishingPointWorkspace {
    pub fn new(open: VanishingPointOpen) -> Self {
        Self {
            dialog: DialogState::new(),
            history: LocalHistory::new(&open.session),
            session: open.session,
            options: open.options,
            subject: open.subject,
            size: open.size,
            view: FullWindowView::new(),
            strip: VanishingPointStripTool::Editor,
            editor: VanishingPointEditor::new(),
            preview_ticket: 0,
        }
    }

    /// Shows the workspace for this pass: the title, the keys, [`VanishingPointBar`], the strip
    /// (the editor's three tools, `group_end`, Hand and Zoom), the narrow side column with
    /// `SideFooter::bottom` OK and Cancel, and the canvas with the source's image and the editor's
    /// pass (or Hand, Zoom). Then the history settles (busy while the editor drags).
    #[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-VP2")]
    pub fn show(&mut self, ctx: &Context, scene: &dyn VanishingScene, source: &dyn VanishingPointSource) -> VanishingPointOutput {
        // pc: crates/ui-egui/src/dialogs/vp_ui.rs:1230 `options_bar`, :1045 `keys`, :1068
        // `escape`; distort_ui.rs:352-368 (the mode on the main canvas, replaced by the frame).
        todo!()
    }

    pub fn session(&self) -> &VanishingPointSession {
        &self.session
    }

    pub fn options(&self) -> &VanishingPointOptions {
        &self.options
    }

    /// Undo or redo from the app's Edit menu; returns whether it stepped.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-VP2")]
    pub fn step(&mut self, step: Step) -> bool {
        // pc: crates/ui-egui/src/dialogs/vp_ui.rs:383 `step`.
        todo!()
    }
}
