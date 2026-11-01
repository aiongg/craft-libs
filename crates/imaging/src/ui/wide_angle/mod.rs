//! The Adaptive Wide Angle workspace (Filter › Adaptive Wide Angle…), in Photoshop's layout.
//!
//! [`WideAngleWorkspace`] is a [`FullWindowDialog`] with the title strip "Adaptive Wide Angle
//! (layer, 66%)", the tool strip (Constraint A, Hand H, Zoom Z; lead call 43: Photoshop's set
//! without Polygon Constraint and Move), the canvas, and a Standard side column with
//! [`WideAnglePanel`] and OK and Cancel at its bottom (PhotoCraft's full-width footer
//! is replaced, lead call 11).
//!
//! The session ([`WideAngleSession`]) is `filter.adaptiveWideAngle`'s parameters with
//! [`LocalHistory`] undo. The app's [`WideAngleGeometry`] curves the constraints by its lens model,
//! its job solves and warps the corrected preview for each [`PreviewTicket`], and
//! [`WideAngleSource`] shows the source and the preview. OK runs the command with the session.
//!
//! Files: everything here is task WS-WA1's, except `style` (TK4).
//!
//! [`FullWindowDialog`]: craft_ui::dialog::FullWindowDialog

mod labels;
mod panel;
mod session;
pub(crate) mod stories;
mod style;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod tests;
mod tool;

pub use labels::WideAngleLabels;
pub use panel::{WideAnglePanel, WideAnglePanelOutput};
pub use session::{ConstraintOrientation, WideAngleConstraint, WideAngleConstraintEdit, WideAngleGeometry, WideAngleModel, WideAngleSession};
pub use style::WideAngleStyle;
pub use tool::{ConstraintTool, ConstraintToolPass};

use egui::{Context, Rect};

use craft_ui::canvas::RasterSource;
use craft_ui::dialog::full_window::{FullWindowView, PreviewStatus, PreviewTicket};
use craft_ui::dialog::{DialogState, LocalHistory, Outcome, Step};
use craft_ui::edit::EditEvent;
use craft_ui::patterns::LastUsed;

/// The tools of the strip.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum WideAngleTool {
    /// A.
    #[default]
    Constraint,
    /// H.
    Hand,
    /// Z.
    Zoom,
}

/// What the app gives the workspace to show each pass.
pub trait WideAngleSource {
    /// The layer as it is (a proxy of at most 800 px in PhotoCraft).
    fn source(&self) -> &dyn RasterSource;

    /// The last ready corrected preview; `None` until one is ready.
    fn corrected(&self) -> Option<&dyn RasterSource>;
}

/// What opens the workspace.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct WideAngleOpen {
    pub size: [u32; 2],
    pub subject: String,
    /// The settings to start from (the last used, or the command's parameters from the control
    /// channel).
    pub session: WideAngleSession,
}

impl WideAngleOpen {
    pub fn new(size: [u32; 2], subject: impl Into<String>) -> Self {
        Self { size, subject: subject.into(), session: WideAngleSession::default() }
    }

    pub fn session(mut self, session: WideAngleSession) -> Self {
        self.session = session;
        self
    }
}

/// What the workspace reports.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum WideAngleEvent {
    /// A constraint added (`Set`), oriented, deleted, or moved by the keyboard (`Set`), recorded
    /// in the session.
    Constraints(EditEvent<WideAngleConstraintEdit>),
}

/// What [`WideAngleWorkspace::show`] reports.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct WideAngleOutput {
    pub events: Vec<WideAngleEvent>,
    /// The session changed (or undo stepped): the app solves and warps for this ticket.
    pub preview_wanted: Option<PreviewTicket>,
    pub outcome: Option<Outcome>,
    pub rect: Rect,
}

/// The Adaptive Wide Angle workspace.
///
/// Keys: A, H, Z (`ToolRail::pick_by_key`); Delete and Backspace the picked constraint; ⌘+, ⌘−,
/// ⌘0, ⌘1 the view; ⌘Z, ⇧⌘Z the history; Enter OK (PhotoCraft had none); Esc Cancel (a drag in
/// progress first).
#[expect(dead_code, reason = "skeleton: `show` (task WS-WA1) reads the fields")]
pub struct WideAngleWorkspace {
    dialog: DialogState<WideAngleSession>,
    session: WideAngleSession,
    history: LocalHistory<WideAngleSession>,
    subject: String,
    size: [u32; 2],
    // View state outside the history (risk 8).
    view: FullWindowView,
    tool: WideAngleTool,
    last_used: LastUsed<WideAngleTool>,
    constraint: ConstraintTool,
    /// Preview: the corrected image (on) or the source (off); the source shows while a drag runs.
    preview_on: bool,
    preview: PreviewStatus,
}

impl WideAngleWorkspace {
    pub fn new(open: WideAngleOpen) -> Self {
        Self {
            dialog: DialogState::new(),
            history: LocalHistory::new(&open.session),
            session: open.session,
            subject: open.subject,
            size: open.size,
            view: FullWindowView::new(),
            tool: WideAngleTool::Constraint,
            last_used: LastUsed::default(),
            constraint: ConstraintTool::new(),
            preview_on: true,
            preview: PreviewStatus::new(),
        }
    }

    /// Shows the workspace for this pass: the title, the strip, the side column ([`WideAnglePanel`],
    /// `SideFooter::bottom` OK and Cancel), the canvas (the corrected preview while Preview is on
    /// and no drag runs, else the source) with [`ConstraintTool`] (or Hand, Zoom), and the
    /// constraint menu while `menu_for` is set. Edits go through [`WideAngleSession::apply`]; the
    /// history settles (busy while dragging); a change asks for a ticket.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-WA1")]
    pub fn show(&mut self, ctx: &Context, geometry: &dyn WideAngleGeometry, source: &dyn WideAngleSource) -> WideAngleOutput {
        // pc: crates/ui-egui/src/dialogs/wide_angle_ui.rs:256 `show` (title :272, the image and
        // constraints, the panel), :232 `constraint_menu`.
        todo!()
    }

    pub fn session(&self) -> &WideAngleSession {
        &self.session
    }

    pub fn preview_ready(&mut self, ticket: PreviewTicket) {
        self.preview.ready(ticket);
    }

    pub fn preview_failed(&mut self, ticket: PreviewTicket, message: String) {
        self.preview.failed(ticket, message);
    }

    /// Undo or redo from the app's Edit menu; returns whether it stepped.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-WA1")]
    pub fn step(&mut self, step: Step) -> bool {
        // pc: crates/ui-egui/src/dialogs/wide_angle_ui.rs:566 (the test: ⌘Z steps the dialog, not
        // the document).
        todo!()
    }
}
