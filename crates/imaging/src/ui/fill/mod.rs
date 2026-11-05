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
pub(crate) use labels::{LASSO_EDIT_LIMIT, POINT_LIMIT, STROKE_LIMIT};
pub use lasso::FillAreaEdit;
pub use overlay::{FillOverlay, SamplingPlane};
pub use panel::{FillPanel, FillPanelOutput};
pub use preview_pane::FillPreviewPane;
pub use session::{
    ColorAdaptation, FillCommand, FillLimit, FillLimits, FillOutputTo, FillReset, FillSession, FillSettings, RotationAdaptation, SamplingMode,
    SamplingStroke,
};
pub use tool::{FillCanvas, FillCanvasTool, FillTool, FillToolOptions};

use egui::{Color32, Context, Key, Modifiers, Rect};

use craft_theme::Srgba;
use craft_ui::canvas::kurbo::{self, Size};
use craft_ui::canvas::{Checkerboard, CursorOptions, HandTool, NavigationOptions, RasterSource, ToolHost, ZoomTool};
use craft_ui::dialog::full_window::{FullWindowCanvas, FullWindowSide, FullWindowTitle, FullWindowView, PreviewStatus, PreviewTicket, SideFooter};
use craft_ui::dialog::{ButtonRole, ColorRequest, DialogLabels, DialogState, FooterButton, FullWindowDialog, LocalHistory, Outcome, Step};
use craft_ui::edit::{CancelReason, EditEvent, GestureId};
use craft_ui::icons::Icon;
use craft_ui::localized::Labels as _;
use craft_ui::patterns::{LastUsed, ToolRail};

/// The key of the colour request for the sampling tint (`FillOutput::pick_color`,
/// [`ContentAwareFillWorkspace::set_color`]).
pub const OVERLAY_COLOR_KEY: &str = "fill.overlayColor";

/// The canvas's id salt, shared by the canvas and the tool switch that cancels its gesture.
const CANVAS: &str = "fill-view";

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
    /// A stroke or edit the limits refused, with the limit (worded under the panel,
    /// [`FillLimit::message`]).
    Refused(FillLimit),
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
    /// The Color swatch was pressed: the app opens its colour picker and answers with
    /// [`ContentAwareFillWorkspace::set_color`].
    pub pick_color: Option<ColorRequest>,
}

/// The Content-Aware Fill workspace.
///
/// Keys (none while a text field edits or a popup is open, except ⌘Z): B, L, H, Z (`ToolRail::pick_by_key`); `[` and
/// `]` the brush size ([`FillToolOptions::step_size`]); ⌘H hides or shows the tint; ⌘+, ⌘−, ⌘0, ⌘1 the
/// canvas's view; ⌘Z, ⇧⌘Z the session's history; Enter OK; Esc Cancel (a stroke or lasso in
/// progress first).
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
    /// The user's navigation options and transparency grid, the app's.
    navigation: NavigationOptions,
    checkerboard: Checkerboard,
    /// ⌘H: the tint hidden without changing Show Sampling Area.
    tint_hidden: bool,
    preview: PreviewStatus,
    error: Option<FillIssue>,
    /// The gesture of a setting being dragged (the Opacity slider): the workspace is busy while
    /// its control is drawn.
    settings_gesture: Option<GestureId>,
    /// [`set_color`](Self::set_color) changed the color since the last pass: the next
    /// [`FillOutput::settings_changed`] says so.
    color_owed: bool,
    // The library's Hand and Zoom tools, kept between passes for their gestures.
    hand: HandTool,
    zoom: ZoomTool,
    /// The session or a fill setting changed and no preview was asked for yet (a drag or a stroke
    /// holds it back).
    stale: bool,
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
            navigation: NavigationOptions::default(),
            checkerboard: Checkerboard::default(),
            tint_hidden: false,
            preview: PreviewStatus::new(),
            error: None,
            settings_gesture: None,
            color_owed: false,
            hand: HandTool::new(Size::new(f64::from(open.size[0]), f64::from(open.size[1])), NavigationOptions::default()),
            zoom: ZoomTool::new(Size::new(f64::from(open.size[0]), f64::from(open.size[1])), NavigationOptions::default()),
            stale: true,
        }
    }

    /// The user's navigation options (wheel, flick, animated zoom) for the canvas and the Preview
    /// panel; Photoshop's by default.
    pub fn navigation(&mut self, navigation: NavigationOptions) {
        self.navigation = navigation;
    }

    /// The user's transparency grid under the image and the preview.
    pub fn checkerboard(&mut self, checkerboard: Checkerboard) {
        self.checkerboard = checkerboard;
    }

    /// The user's cursor preferences (the Sampling Brush's outline).
    pub fn cursor_options(&mut self, cursor_options: CursorOptions) {
        self.canvas.cursor_options(cursor_options);
    }

    /// Shows the workspace for this pass: the title, the keys, [`FillBar`], the strip, the two
    /// side columns ([`FillPanel`] with `SideFooter::bottom` OK, Apply, Cancel and Reset at
    /// `leading`; then [`FillPreviewPane`]), and the canvas with the source's image,
    /// [`FillOverlay`] and [`FillCanvas`]'s tool (or Hand, Zoom). Commits go into the session
    /// within the limits; a sampling option or Reset from the panel goes through
    /// [`FillSession::set_sampling`] and [`FillSession::reset`] (Custom makes the brush add, the
    /// others subtract). Then the history settles and a change asks for a preview ticket. An
    /// Apply or OK the app reports failing ([`Self::apply_failed`]) keeps the workspace open.
    pub fn show(&mut self, ctx: &Context, source: &dyn FillSource) -> FillOutput {
        // pc: crates/ui-egui/src/dialogs/caf_ui/view.rs:265 `show`; caf_ui/controls.rs:151 `keys`.
        let labels = FillLabels::of_ctx(ctx);
        let dialog_labels = DialogLabels::of_ctx(ctx);
        let title = FullWindowTitle::new(labels.name.as_str()).subject(self.subject.as_str()).zoom(self.view.zoom_percent()).text(ctx);
        let Self {
            dialog,
            session,
            history,
            settings,
            limits,
            subject: _,
            size,
            view,
            preview_view,
            tool,
            last_used,
            options,
            canvas,
            tint_hidden,
            preview,
            error,
            hand,
            zoom,
            stale,
            navigation,
            checkerboard,
            settings_gesture,
            color_owed,
        } = self;
        let doc = Size::new(f64::from(size[0]), f64::from(size[1]));
        let (navigation, checkerboard) = (*navigation, *checkerboard);
        (hand.size, hand.prefs, zoom.size, zoom.prefs) = (doc, navigation, doc, navigation);
        let before = settings.clone();

        let mut events = Vec::new();
        let mut session_changed = false;
        let mut panel_edit = None;
        let mut pick_color = None;
        let response = FullWindowDialog::new("fill").title(title).show(ctx, dialog, session, |ui, session, fx| {
            // The history keys are read before the body, whatever has focus, so ⌘Z works while a
            // number field has it, but only while this workspace is the top modal layer.
            if let Some(step) = fx.take_step(ui) {
                if canvas.busy() {
                    cancel_gesture(ui, canvas, *tool, options, session, limits, hand, zoom, CancelReason::UndoDuringGesture);
                    for event in canvas.take_events() {
                        absorb(session, limits, error, &mut events, &mut session_changed, event);
                    }
                }
                if history.step(session, step) {
                    synced(session, settings, options, error);
                    session_changed = true;
                }
            }
            if fx.keys_free(ui) {
                if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::H)) {
                    *tint_hidden = !*tint_hidden;
                }
                if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::OpenBracket)) {
                    options.step_size(false);
                }
                if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::CloseBracket)) {
                    options.step_size(true);
                }
            }

            let bar = FillBar::new("fill-bar", *tool, options).show(ui);
            if let Some(edit) = bar.edit {
                session_changed |= add_edit(session, edit, limits, error, &mut events);
            }
            if bar.reset_fill_area && session.reset(FillReset::FillArea, settings) {
                session_changed = true;
                *error = None;
                events.push(FillEvent::Reset(FillReset::FillArea));
            }

            let slots = FillTool::slots(&labels);
            let rail = ToolRail::new("fill-tools").label(labels.tools.as_str()).pick_by_key(true).show(ui, &slots, &*tool, last_used, |_, _| {});
            if let Some(picked) = rail.select.filter(|picked| picked != tool) {
                if canvas.busy() {
                    cancel_gesture(ui, canvas, *tool, options, session, limits, hand, zoom, CancelReason::Removed);
                    for event in canvas.take_events() {
                        absorb(session, limits, error, &mut events, &mut session_changed, event);
                    }
                }
                *tool = picked;
            }

            let mut reset_all = false;
            let footer = SideFooter::bottom(vec![
                FooterButton::new(ButtonRole::Default, dialog_labels.ok.as_str()),
                FooterButton::new(ButtonRole::Apply, dialog_labels.apply.as_str()),
                FooterButton::new(ButtonRole::Cancel, dialog_labels.cancel.as_str()),
            ])
            .leading(|ui| {
                reset_all = ui.add(craft_ui::controls::IconButton::new(Icon::Undo, dialog_labels.reset.as_str()).id_salt("reset-all")).clicked();
            });
            // The preview's failure shows under the Preview panel, once.
            let shown_error = error.as_ref().map(|issue| issue.text(ctx));
            let mut panel = None;
            FullWindowSide::new("fill-props").header(labels.name.as_str()).footer(footer).show(ui, fx, |ui| {
                panel = Some(FillPanel::new("fill-panel", settings, session.sampling).error(shown_error.as_deref()).show(ui));
            });
            if let Some(panel) = panel {
                panel_edit = panel.edit;
                if let Some(mode) = panel.sampling
                    && session.set_sampling(mode)
                {
                    synced(session, settings, options, error);
                    session_changed = true;
                    events.push(FillEvent::Reset(FillReset::Sampling));
                }
                if let Some(what) = panel.reset
                    && (session.reset(what, settings) || what == FillReset::Fill)
                {
                    synced(session, settings, options, error);
                    session_changed |= what == FillReset::Sampling;
                    events.push(FillEvent::Reset(what));
                }
                pick_color = panel.pick_color;
            }
            if reset_all {
                // The footer's Reset sets the session and every setting back, whether or not the
                // session was already at its defaults.
                let changed = session.reset(FillReset::All, settings);
                synced(session, settings, options, error);
                session_changed |= changed;
                events.push(FillEvent::Reset(FillReset::All));
            }

            FullWindowSide::new("fill-preview").header(labels.preview.as_str()).scrolls(false).show(ui, fx, |ui| {
                let image: &dyn RasterSource = source.preview().unwrap_or_else(|| source.image());
                FillPreviewPane::new("fill-preview-pane", preview_view, preview)
                    .source(Some(image))
                    .frame(source.sampling_window())
                    .navigation(navigation)
                    .checkerboard(checkerboard)
                    .show(ui);
            });

            let [r, g, b] = settings.color;
            let alpha = (settings.opacity.clamp(0.0, 100.0) / 100.0 * 255.0).round() as u8;
            let overlay = FillOverlay::new("fill-overlay", source.sampling(), source.fill_area())
                .color(Color32::from_rgba_unmultiplied(r, g, b, alpha))
                .excluded(settings.excluded)
                .tint(settings.show_overlay && !*tint_hidden)
                .document(*size);
            let overlay_id = overlay.id(ui);
            let ctx_for_marks = ctx.clone();
            let widget = FullWindowCanvas::new(CANVAS, view)
                .source(source.image())
                .checkerboard(checkerboard)
                .navigation(navigation)
                .painter(move |painter, view| overlay.paint(&ctx_for_marks, painter, view, overlay_id));
            let shown = match *tool {
                FillTool::Hand => widget.tool(hand).show(ui),
                FillTool::Zoom => widget.tool(zoom).show(ui),
                _ => widget.tool(&mut canvas.tool(*tool, options, session, limits)).show(ui),
            };
            // The Esc that cancelled a stroke or lasso is not the dialog's Cancel.
            if shown.took_key.is_some_and(|key| key.key == Key::Escape) {
                ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
            }
            for event in canvas.take_events() {
                absorb(session, limits, error, &mut events, &mut session_changed, event);
            }
        });
        let busy = canvas.busy() || settings_busy(ctx, settings_gesture, panel_edit.as_ref());
        history.settle(session, busy);
        *stale |= session_changed || fill_params(settings) != fill_params(&before);
        let preview_wanted = (*stale && !busy && response.outcome.is_none()).then(|| {
            *stale = false;
            preview.request()
        });
        FillOutput {
            events,
            preview_wanted,
            outcome: response.outcome,
            rect: response.rect,
            settings_changed: *settings != before || std::mem::take(color_owed),
            pick_color,
        }
    }

    /// The app's colour picker answered a [`FillOutput::pick_color`] request. A key this workspace
    /// did not ask for is ignored.
    pub fn set_color(&mut self, key: &str, color: Srgba) {
        if key == OVERLAY_COLOR_KEY {
            let [r, g, b, _] = Color32::from(color).to_array();
            if self.settings.color != [r, g, b] {
                self.settings.color = [r, g, b];
                self.color_owed = true;
            }
        }
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
    pub fn rebase(&mut self) {
        // pc: crates/ui-egui/src/dialogs/caf_ui/lifecycle.rs:80 `apply` (refresh), caf_ui/preview.rs:135
        // `rebase`.
        self.history = LocalHistory::new(&self.session);
        self.error = None;
        self.stale = true;
    }

    /// The app's Apply or OK failed: the message shows under the panel and the workspace stays.
    pub fn apply_failed(&mut self, message: String) {
        self.error = Some(FillIssue::Message(message));
    }

    /// The app found that the last fill-area edit empties the fill area: the edit is taken back
    /// (no redo of it) and `message` shows under the panel.
    pub fn refuse_fill_edit(&mut self, message: String) {
        // pc: crates/ui-egui/src/dialogs/caf_ui/lifecycle.rs:211-219.
        if !self.history.drop_last(&mut self.session) {
            self.session.fill_edits.pop();
        }
        synced(&self.session, &mut self.settings, &mut self.options, &mut self.error);
        self.error = Some(FillIssue::Message(message));
        self.stale = true;
    }

    pub fn preview_ready(&mut self, ticket: PreviewTicket) {
        self.preview.ready(ticket);
    }

    pub fn preview_failed(&mut self, ticket: PreviewTicket, message: String) {
        self.preview.failed(ticket, message);
    }

    /// Undo or redo from the app's Edit menu; returns whether it stepped.
    pub fn step(&mut self, step: Step) -> bool {
        // pc: crates/ui-egui/src/dialogs/caf_ui/controls.rs:151-159 (⌘Z, ⇧⌘Z).
        if self.canvas.busy() {
            return false;
        }
        let stepped = self.history.step(&mut self.session, step);
        if stepped {
            synced(&self.session, &mut self.settings, &mut self.options, &mut self.error);
            self.stale = true;
        }
        stepped
    }
}

/// What the session's sampling option sets in the settings and the bar after undo, redo or a
/// reset: the option is remembered and Custom makes the brush add. A stale error goes.
fn synced(session: &FillSession, settings: &mut FillSettings, options: &mut FillToolOptions, error: &mut Option<FillIssue>) {
    settings.sampling = session.sampling;
    options.brush_add = session.sampling.brush_adds();
    *error = None;
}

/// The settings that change the fill: a change asks for a new preview.
fn fill_params(settings: &FillSettings) -> (bool, ColorAdaptation, RotationAdaptation, bool, bool) {
    (settings.sample_all_layers, settings.color_adaptation, settings.rotation, settings.scale, settings.mirror)
}

/// Records a fill-area edit within the limits, or sets the reason as the error and reports it.
fn add_edit(session: &mut FillSession, edit: FillAreaEdit, limits: &FillLimits, error: &mut Option<FillIssue>, events: &mut Vec<FillEvent>) -> bool {
    match session.add_fill_edit(edit.clone(), limits) {
        Ok(()) => {
            *error = None;
            events.push(FillEvent::FillArea(edit));
            true
        }
        Err(limit) => {
            *error = Some(FillIssue::Limit(limit));
            events.push(FillEvent::Refused(limit));
            false
        }
    }
}

/// Puts a canvas event in `out`, recording a committed stroke and a lasso edit in the session.
fn absorb(
    session: &mut FillSession,
    limits: &FillLimits,
    error: &mut Option<FillIssue>,
    out: &mut Vec<FillEvent>,
    changed: &mut bool,
    event: FillEvent,
) {
    match event {
        FillEvent::FillArea(edit) => *changed |= add_edit(session, edit, limits, error, out),
        event => {
            match &event {
                FillEvent::Stroke(EditEvent::Commit { value, .. }) => {
                    session.add_stroke(value.clone());
                    *error = None;
                    *changed = true;
                }
                FillEvent::Refused(limit) => *error = Some(FillIssue::Limit(*limit)),
                _ => {}
            }
            out.push(event);
        }
    }
}

/// Cancels the gesture of `tool` on the canvas with `reason`: `Removed` for a tool switch,
/// `UndoDuringGesture` before an undo step.
#[expect(clippy::too_many_arguments, reason = "the canvas and everything its per-pass tool borrows")]
fn cancel_gesture(
    ui: &egui::Ui,
    canvas: &mut FillCanvas,
    tool: FillTool,
    options: &mut FillToolOptions,
    session: &FillSession,
    limits: &FillLimits,
    hand: &mut HandTool,
    zoom: &mut ZoomTool,
    reason: CancelReason,
) {
    let (ctx, id) = (ui.ctx(), FullWindowCanvas::canvas_id(ui, CANVAS));
    match tool {
        FillTool::Hand => ToolHost::cancel(ctx, id, hand, reason),
        FillTool::Zoom => ToolHost::cancel(ctx, id, zoom, reason),
        _ => ToolHost::cancel(ctx, id, &mut canvas.tool(tool, options, session, limits), reason),
    };
}

/// What shows under the panel: the limit a stroke or edit ran into, or the app's message (a failed
/// Apply, an edit that emptied the fill area).
#[derive(Clone, Debug, PartialEq)]
enum FillIssue {
    Limit(FillLimit),
    Message(String),
}

impl FillIssue {
    fn text(&self, ctx: &Context) -> String {
        match self {
            FillIssue::Limit(limit) => limit.message(ctx),
            FillIssue::Message(message) => message.clone(),
        }
    }
}

/// Whether a setting is being dragged after this pass's `edit`: a `Begin` or `Update` is, and a
/// gesture whose control is still drawn is, until its `Commit` or `Cancel`.
fn settings_busy(ctx: &Context, gesture: &mut Option<GestureId>, edit: Option<&EditEvent<FillSettings>>) -> bool {
    match edit {
        Some(EditEvent::Begin { id, .. } | EditEvent::Update { id, .. }) => {
            *gesture = Some(*id);
            return true;
        }
        Some(EditEvent::Commit { .. } | EditEvent::Cancel { .. }) => *gesture = None,
        _ => {}
    }
    if gesture.is_some_and(|id| !craft_ui::edit::is_live(ctx, id)) {
        *gesture = None;
    }
    gesture.is_some()
}
