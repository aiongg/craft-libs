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
//! Files: `catalog` and `session`; `tree`, `stack`, `parameters` and this file's
//! bodies.
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
mod style;
mod tree;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod tree_tests;

pub use catalog::{FilterCatalog, FilterCategory, GalleryFilter, GalleryThumb};
pub use labels::FilterGalleryLabels;
pub use parameters::{EffectParameters, EffectParametersResponse};
pub use session::{EffectChange, EffectLayer, FilterGallerySession};
pub use stack::{EffectStack, EffectStackResponse};
pub use style::FilterGalleryStyle;
pub use tree::{FilterTree, FilterTreeResponse};

use egui::{Context, Rect};

use craft_theme::Srgba;
use craft_ui::canvas::kurbo::Size;
use craft_ui::canvas::textures::{ThumbSource, Thumbnails};
use craft_ui::canvas::{CanvasView, Checkerboard, HandTool, NavigationOptions, RasterSource, kurbo};
use craft_ui::dialog::full_window::{
    FullWindowCanvas, FullWindowLabels, FullWindowSide, FullWindowTitle, FullWindowView, FullWindowZoomBar, PreviewStatus, PreviewTicket, SideFooter,
};
use craft_ui::dialog::{
    ButtonRole, ColorRequest, DialogLabels, DialogState, FieldValue, FooterButton, FullWindowDialog, LocalHistory, Outcome, Step, translated,
};
use craft_ui::localized::Labels as _;
use craft_ui::primitives::Gap;
use craft_ui::theme::{Tokens, TokensOf as _};

/// What opens the workspace.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct FilterGalleryOpen {
    /// The layer's size in document pixels.
    pub size: [u32; 2],
    /// The layer's name: the title's subject.
    pub subject: String,
    /// The stack to start from: one layer of the last filter used, or a smart filter's recorded
    /// effects.
    pub session: FilterGallerySession,
    /// The user's navigation options (wheel, flick, animated zoom); Photoshop's by default.
    pub navigation: NavigationOptions,
    /// The user's transparency grid under the preview; none draws the picture on the canvas fill.
    pub checkerboard: Option<Checkerboard>,
}

impl FilterGalleryOpen {
    /// Opens on a layer of `size` document pixels named `subject`, starting from `session`; navigation and checkerboard take their defaults.
    pub fn new(size: [u32; 2], subject: impl Into<String>, session: FilterGallerySession) -> Self {
        Self { size, subject: subject.into(), session, navigation: NavigationOptions::default(), checkerboard: None }
    }

    /// The user's navigation options.
    pub fn navigation(mut self, navigation: NavigationOptions) -> Self {
        self.navigation = navigation;
        self
    }

    /// The user's transparency grid under the preview.
    pub fn checkerboard(mut self, checkerboard: Checkerboard) -> Self {
        self.checkerboard = Some(checkerboard);
        self
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
}

/// What [`FilterGalleryWorkspace::show`] reports.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct FilterGalleryOutput {
    /// What happened this pass, in order.
    pub events: Vec<FilterGalleryEvent>,
    /// The stack, the zoom or the visible part changed: the app renders
    /// [`FilterGalleryWorkspace::visible`] for this ticket.
    pub preview_wanted: Option<PreviewTicket>,
    /// OK or Cancel, once chosen.
    pub outcome: Option<Outcome>,
    /// The workspace's screen rectangle.
    pub rect: Rect,
    /// A color parameter's swatch pressed: the app opens its picker and answers into the selected
    /// effect's fields ([`FilterGalleryWorkspace::set_color`]).
    pub pick_color: Option<ColorRequest>,
}

/// The Filter Gallery workspace.
///
/// Keys: ⌘+, ⌘−, ⌘0, ⌘1 zoom the preview, never the canvas behind; ⌘Z, ⇧⌘Z the stack's history;
/// Enter OK, Esc Cancel.
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
    /// The Hand tool: the preview's only tool, kept for its drag between passes.
    hand: HandTool,
    /// The user's navigation options and transparency grid.
    navigation: NavigationOptions,
    checkerboard: Option<Checkerboard>,
    /// The selection the app has been told of (the open one, then each `Select` event).
    reported: usize,
    /// The canvas view of the last pass: what `visible` reads.
    shown: Option<CanvasView>,
    /// The stack changed outside `show` (an undo, a colour): the next pass asks for a preview.
    dirty: bool,
    /// Events from calls outside `show`, reported by the next pass.
    queued: Vec<FilterGalleryEvent>,
}

impl FilterGalleryWorkspace {
    /// A workspace on the stack and layer that `open` describes.
    pub fn new(open: FilterGalleryOpen) -> Self {
        let [width, height] = open.size;
        let selected = open.session.effects.len().saturating_sub(1);
        Self {
            dialog: DialogState::new(),
            history: LocalHistory::new(&open.session),
            selected,
            reported: selected,
            navigation: open.navigation,
            checkerboard: open.checkerboard,
            session: open.session,
            subject: open.subject,
            size: open.size,
            view: FullWindowView::new(),
            preview: PreviewStatus::new(),
            note: None,
            hand: HandTool::new(Size::new(f64::from(width), f64::from(height)), open.navigation),
            shown: None,
            dirty: true,
            queued: Vec::new(),
        }
    }

    /// Shows the workspace for this pass: the title (the selected effect's filter name as the
    /// subject's prefix, PhotoCraft's "{effect} (layer, 66%)"), the side columns from the end edge
    /// inwards (settings with the top footer, then folders), the zoom bar (progress while the
    /// preview renders, else the note), and the canvas over `preview` (the app's render of the
    /// visible part) with the `HandTool`. Changes go through [`FilterGallerySession::apply`], then
    /// the history settles (busy during a slider drag) and a change, a zoom or a pan asks for a
    /// ticket.
    ///
    /// `thumb_source` renders the pictures `thumbs` asks for (one per filter, the filter applied
    /// to a small crop of the layer).
    pub fn show(
        &mut self,
        ctx: &Context,
        catalog: &dyn FilterCatalog,
        thumbs: &Thumbnails<GalleryThumb>,
        thumb_source: &dyn ThumbSource<GalleryThumb>,
        preview: &dyn RasterSource,
    ) -> FilterGalleryOutput {
        // pc: crates/ui-egui/src/dialogs/gallery_ui.rs:636 `show` (title :645-655, zoom bar
        // :673-700), :483 `keys`.
        let labels = FilterGalleryLabels::of_ctx(ctx);
        let full = FullWindowLabels::of_ctx(ctx);
        let dialog_labels = DialogLabels::of_ctx(ctx);
        // The catalog's labels are English; the app's I18n translates them like `Param` labels.
        let categories: Vec<FilterCategory> =
            catalog.categories().into_iter().map(|category| FilterCategory { label: translated(ctx, &category.label), ..category }).collect();
        let filter_label = |key: &str| catalog.filter(key).map_or_else(|| key.to_owned(), |filter| translated(ctx, &filter.label));
        let mut events = std::mem::take(&mut self.queued);
        let mut changed = std::mem::take(&mut self.dirty);
        let visible_before = self.visible();
        self.selected = self.selected.min(self.session.effects.len().saturating_sub(1));

        let name = self.session.effects.get(self.selected).map_or_else(|| labels.name.clone(), |effect| filter_label(&effect.filter));
        let title = FullWindowTitle::new(name).subject(self.subject.as_str()).zoom(self.view.zoom_percent()).labels(full.clone()).text(ctx);
        let footer = vec![
            FooterButton::new(ButtonRole::Default, dialog_labels.ok.as_str()),
            FooterButton::new(ButtonRole::Cancel, dialog_labels.cancel.as_str()),
        ];

        let mut busy = false;
        let mut dialog_changes: Vec<EffectChange> = Vec::new();
        let mut color_request = None;
        let mut tree_pick = None;
        self.hand.prefs = self.navigation;
        let (selected, view, status, note, hand, shown, history) =
            (&mut self.selected, &mut self.view, &self.preview, &self.note, &mut self.hand, &mut self.shown, &mut self.history);
        let (navigation, checkerboard) = (self.navigation, self.checkerboard);
        let response = FullWindowDialog::new("filter-gallery").title(title).labels(dialog_labels.clone()).show(
            ctx,
            &mut self.dialog,
            &mut self.session,
            |ui, session, fx| {
                // Undo and Redo first, so a number field never sees ⌘Z: the stack has one history.
                if let Some(step) = fx.take_step(ui)
                    && step_stack(history, session, selected, step)
                {
                    changed = true;
                }
                // The settings column sits at the end edge, the folders inwards of it.
                let mut stack_pick = None;
                FullWindowSide::new("settings").name(labels.settings.as_str()).footer(SideFooter::top(footer)).show(ui, fx, |ui| {
                    let current = *selected;
                    if let Some(effect) = session.effects.get(current) {
                        let filter = catalog.filter(&effect.filter);
                        let parameters = EffectParameters::new("parameters", current, effect, filter.as_ref(), &categories, &filter_label)
                            .labels(labels.clone())
                            .show(ui);
                        busy = parameters.dragging;
                        color_request = parameters.rows.and_then(|rows| rows.color);
                        dialog_changes.extend(parameters.change);
                    }
                    ui.add_space(Gap::Row.points(&Tokens::of(ui)));
                    let stack = EffectStack::new("stack", &session.effects, current, &filter_label).labels(labels.clone()).show(ui);
                    stack_pick = stack.selected;
                    dialog_changes.extend(stack.change);
                    status.show(ui, &full);
                });
                for change in std::mem::take(&mut dialog_changes) {
                    if session.apply(change.clone(), catalog, selected) {
                        events.push(FilterGalleryEvent::Stack(change));
                        changed = true;
                    }
                }
                if let Some(pick) = stack_pick {
                    *selected = pick.min(session.effects.len().saturating_sub(1));
                }

                let current = session.effects.get(*selected).map_or("", |effect| effect.filter.as_str());
                FullWindowSide::new("filters").name(labels.filters.as_str()).show(ui, fx, |ui| {
                    let tree = FilterTree::new("tree", &categories, &filter_label, current, thumbs, thumb_source).labels(labels.clone()).show(ui);
                    tree_pick = tree.picked;
                });
                if let Some(filter) = tree_pick.take() {
                    let change = EffectChange::SetFilter { index: *selected, filter };
                    if session.apply(change.clone(), catalog, selected) {
                        events.push(FilterGalleryEvent::Stack(change));
                        changed = true;
                    }
                }

                let mut bar = FullWindowZoomBar::new("zoom", view).progress(status.progress());
                if let Some(note) = note.as_deref() {
                    bar = bar.note(note);
                }
                bar.show(ui);
                let [width, height] = preview.size();
                hand.size = Size::new(f64::from(width), f64::from(height));
                let mut canvas = FullWindowCanvas::new("preview", view).source(preview).tool(hand).navigation(navigation).labels(full.clone());
                if let Some(checkerboard) = checkerboard {
                    canvas = canvas.checkerboard(checkerboard);
                }
                let canvas = canvas.show(ui);
                *shown = Some(canvas.view);
            },
        );
        self.history.settle(&self.session, busy);
        changed |= std::mem::take(&mut self.dirty);
        events.append(&mut self.queued);
        if self.selected != self.reported {
            self.reported = self.selected;
            events.push(FilterGalleryEvent::Select(self.selected));
        }
        let preview_wanted = (changed || self.visible() != visible_before).then(|| self.preview.request());
        FilterGalleryOutput { events, preview_wanted, outcome: response.outcome, rect: response.rect, pick_color: color_request }
    }

    /// The effect-layer stack as it is now.
    pub fn session(&self) -> &FilterGallerySession {
        &self.session
    }

    /// The part of the document the preview shows and the zoom (screen pixels per document
    /// pixel): what the app's job renders, plus the filters' reach.
    pub fn visible(&self) -> (kurbo::Rect, f64) {
        // pc: crates/ui-egui/src/dialogs/gallery_ui.rs:496 `view`.
        let image = kurbo::Rect::new(0.0, 0.0, f64::from(self.size[0]), f64::from(self.size[1]));
        let part = self.shown.map_or(image, |view| view.visible().intersect(image));
        (part.expand(), self.view.zoom())
    }

    /// The color the app picked for a [`FilterGalleryOutput::pick_color`] request: written into
    /// the request's `key` of the selected effect (one undo step).
    pub fn set_color(&mut self, key: &str, color: Srgba) {
        // new: `ParamRows` reports colour requests; the gallery's host answers them.
        let index = self.selected.min(self.session.effects.len().saturating_sub(1));
        let Some(effect) = self.session.effects.get_mut(index) else { return };
        let mut fields = effect.fields.clone();
        fields.set(key, FieldValue::Color(color.to_rgb_array()));
        if fields == effect.fields {
            return;
        }
        effect.fields = fields.clone();
        self.history.settle(&self.session, false);
        self.queued.push(FilterGalleryEvent::Stack(EffectChange::Fields { index, fields }));
        self.dirty = true;
    }

    /// The preview job `ticket` finished: the preview shows the new result.
    pub fn preview_ready(&mut self, ticket: PreviewTicket) {
        self.preview.ready(ticket);
    }

    /// The render for `ticket` reports how far it is (0..=1).
    pub fn preview_progress(&mut self, ticket: PreviewTicket, fraction: f32) {
        self.preview.report_progress(ticket, fraction);
    }

    /// The preview job `ticket` failed: `message` shows in the preview pane.
    pub fn preview_failed(&mut self, ticket: PreviewTicket, message: String) {
        self.preview.failed(ticket, message);
    }

    /// The zoom bar's note while no render runs ("Preview 120 ms"; the app's words).
    pub fn set_note(&mut self, note: Option<String>) {
        self.note = note;
    }

    /// Undo or redo from the app's Edit menu; returns whether it stepped. The next pass clamps the
    /// selection and reports it as a `Select` event when it moved.
    pub fn step(&mut self, step: Step) -> bool {
        // pc: crates/ui-egui/src/dialogs/gallery_ui.rs:158 `step`.
        if !step_stack(&mut self.history, &mut self.session, &mut self.selected, step) {
            return false;
        }
        self.dirty = true;
        true
    }
}

/// Steps the history and keeps `selected` on a layer that exists.
fn step_stack(history: &mut LocalHistory<FilterGallerySession>, session: &mut FilterGallerySession, selected: &mut usize, step: Step) -> bool {
    if !history.step(session, step) {
        return false;
    }
    *selected = (*selected).min(session.effects.len().saturating_sub(1));
    true
}
