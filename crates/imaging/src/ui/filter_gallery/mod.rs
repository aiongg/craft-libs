//! The Filter Gallery workspace (Filter › Filter Gallery…), Photoshop's layout.
//!
//! [`FilterGalleryWorkspace`] is a [`FullWindowDialog`] with the title strip "Filter Gallery
//! (layer, 66%)": the preview canvas at the start with a [`FullWindowZoomBar`] under it, the
//! category folders in a Standard middle column ([`FilterTree`]), and the settings column at the
//! end edge with OK and Cancel at its top (`SideFooter::top`), the Filter select and the effect's
//! parameters ([`EffectParameters`]) and the effect-layer stack ([`EffectStack`]). No tool strip and
//! no options bar; a drag on the preview pans (the Hand tool is the canvas's only tool).
//!
//! The session is the stack ([`FilterGallerySession`]) with [`LocalHistory`] undo. The app's
//! catalog ([`FilterCatalog`]) names the filters and their parameters, its thumbnail source
//! renders the folders' pictures, and its job renders the preview of the visible part
//! ([`FilterGalleryWorkspace::visible`]) for each [`PreviewTicket`]. OK runs `filter.filterGallery`
//! with the effects; a smart filter opened again passes its recorded effects.
//!
//! Files: `catalog` and `session` (task WS-FG1); `tree`, `stack`, `settings` and this file's
//! bodies (WS-FG2).
//!
//! [`FullWindowDialog`]: craft_ui::dialog::FullWindowDialog
//! [`FullWindowZoomBar`]: craft_ui::dialog::full_window::FullWindowZoomBar

mod catalog;
mod labels;
mod parameters;
mod session;
#[cfg(test)]
mod session_tests;
mod stack;
pub(crate) mod stories;
mod tree;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod tree_tests;

pub use catalog::{FilterCatalog, FilterCategory, GalleryFilter, GalleryThumb};
pub use labels::FilterGalleryLabels;
pub use parameters::{EffectParameters, EffectParametersResponse};
pub use session::{EffectChange, EffectLayer, FilterGallerySession};
pub use stack::{EffectStack, EffectStackResponse};
pub use tree::{FilterTree, FilterTreeResponse};

use egui::{Context, Rect};

use craft_ui::canvas::RasterSource;
use craft_ui::canvas::kurbo;
use craft_ui::canvas::textures::Thumbnails;
use craft_ui::dialog::full_window::{FullWindowView, PreviewStatus, PreviewTicket};
use craft_ui::dialog::{ColorRequest, DialogState, LocalHistory, Outcome, Step};

/// What opens the workspace.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct FilterGalleryOpen {
    pub size: [u32; 2],
    /// The layer's name: the title's subject.
    pub subject: String,
    /// The stack to start from: one layer of the last filter used, or a smart filter's recorded
    /// effects.
    pub session: FilterGallerySession,
}

impl FilterGalleryOpen {
    pub fn new(size: [u32; 2], subject: impl Into<String>, session: FilterGallerySession) -> Self {
        Self { size, subject: subject.into(), session }
    }
}

/// What the workspace reports, in order.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum FilterGalleryEvent {
    /// The stack changed (recorded): the app may note it; the preview ticket follows.
    Stack(EffectChange),
    /// Another effect layer selected.
    Select(usize),
    /// A colour parameter's swatch pressed: the app answers into that effect's fields
    /// ([`FilterGalleryWorkspace::set_color`]).
    PickColor(ColorRequest),
}

/// What [`FilterGalleryWorkspace::show`] reports.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct FilterGalleryOutput {
    pub events: Vec<FilterGalleryEvent>,
    /// The stack, the zoom or the visible part changed: the app renders
    /// [`FilterGalleryWorkspace::visible`] for this ticket.
    pub preview_wanted: Option<PreviewTicket>,
    pub outcome: Option<Outcome>,
    pub rect: Rect,
}

/// The Filter Gallery workspace.
///
/// Keys: ⌘+, ⌘−, ⌘0, ⌘1 zoom the preview, never the canvas behind; ⌘Z, ⇧⌘Z the stack's history;
/// Enter OK, Esc Cancel.
#[expect(dead_code, reason = "skeleton: `show` (task WS-FG2) reads the fields")]
pub struct FilterGalleryWorkspace {
    dialog: DialogState<FilterGallerySession>,
    session: FilterGallerySession,
    history: LocalHistory<FilterGallerySession>,
    subject: String,
    size: [u32; 2],
    // View state outside the history (risk 8).
    selected: usize,
    view: FullWindowView,
    preview: PreviewStatus,
    /// "Preview N ms" in the zoom bar while no render runs (the app's timing).
    note: Option<String>,
}

impl FilterGalleryWorkspace {
    pub fn new(open: FilterGalleryOpen) -> Self {
        Self {
            dialog: DialogState::new(),
            history: LocalHistory::new(&open.session),
            selected: open.session.effects.len().saturating_sub(1),
            session: open.session,
            subject: open.subject,
            size: open.size,
            view: FullWindowView::new(),
            preview: PreviewStatus::new(),
            note: None,
        }
    }

    /// Shows the workspace for this pass: the title (the selected effect's filter name as the
    /// subject's prefix, PhotoCraft's "{effect} (layer, 66%)"), the side columns from the end edge
    /// inwards (settings with the top footer, then folders), the zoom bar (progress while the
    /// preview renders, else the note), and the canvas over `preview` (the app's render of the
    /// visible part) with the `HandTool`. Changes go through [`FilterGallerySession::apply`], then
    /// the history settles (busy during a slider drag) and a change, a zoom or a pan asks for a
    /// ticket.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-FG2")]
    pub fn show(
        &mut self,
        ctx: &Context,
        catalog: &dyn FilterCatalog,
        thumbs: &Thumbnails<GalleryThumb>,
        preview: &dyn RasterSource,
    ) -> FilterGalleryOutput {
        // pc: crates/ui-egui/src/dialogs/gallery_ui.rs:636 `show` (title :645-655, zoom bar
        // :673-700), :483 `keys`.
        todo!()
    }

    pub fn session(&self) -> &FilterGallerySession {
        &self.session
    }

    /// The part of the document the preview shows and the zoom (screen pixels per document
    /// pixel): what the app's job renders, plus the filters' reach.
    #[expect(clippy::todo, reason = "skeleton: task WS-FG2")]
    pub fn visible(&self) -> (kurbo::Rect, f64) {
        // pc: crates/ui-egui/src/dialogs/gallery_ui.rs:496 `view`.
        todo!()
    }

    /// The colour the app picked for a [`FilterGalleryEvent::PickColor`] request: written into the
    /// request's key of the selected effect (one undo step).
    #[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-FG2")]
    pub fn set_color(&mut self, request: &ColorRequest, rgb: [f32; 3]) {
        // new: `ParamRows` reports colour requests; the gallery's host answers them.
        todo!()
    }

    pub fn preview_ready(&mut self, ticket: PreviewTicket) {
        self.preview.ready(ticket);
    }

    /// The render for `ticket` reports how far it is (0..=1).
    pub fn preview_progress(&mut self, ticket: PreviewTicket, fraction: f32) {
        self.preview.report_progress(ticket, fraction);
    }

    pub fn preview_failed(&mut self, ticket: PreviewTicket, message: String) {
        self.preview.failed(ticket, message);
    }

    /// The zoom bar's note while no render runs ("Preview 120 ms"; the app's words).
    pub fn set_note(&mut self, note: Option<String>) {
        self.note = note;
    }

    /// Undo or redo from the app's Edit menu; returns whether it stepped (the selection is clamped).
    #[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-FG2")]
    pub fn step(&mut self, step: Step) -> bool {
        // pc: crates/ui-egui/src/dialogs/gallery_ui.rs:158 `step`.
        todo!()
    }
}
