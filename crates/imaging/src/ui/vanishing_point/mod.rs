//! Vanishing Point (Filter › Vanishing Point…): the plane editor and its full-window workspace.
//!
//! The owner's answer (decisions, "Step 7 questions" 2): Vanishing Point opens in its own window,
//! as in Photoshop; PhotoCraft ran it on the main canvas. [`VanishingPointWorkspace`] is a
//! [`FullWindowDialog`] with the title strip "Vanishing Point (layer, 66%)", the full-width
//! options bar ([`VanishingPointBar`]), the tool strip (Edit Plane V, Create Plane C, Stamp S,
//! then Hand H and Zoom Z), the canvas, and a narrow side column with only OK and Cancel at its
//! bottom (Photoshop has no side panel and PhotoCraft's bar ends in ✓ and ⊘; the owner's answer
//! keeps the column, which also holds the preview's status).
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
    PasteImage, PlaneEdit, StampLimits, StampSettings, StampStroke, VanishingPointEditor, VanishingPointPass, VanishingPointSession,
    VanishingPointSettings, VanishingTool,
};
pub use labels::VanishingPointLabels;
pub use planes::{MAX_GRID_LINES, MAX_PLANES, PlaneCheck, PlaneEdge, TornPlane, VanishingPlane, VanishingScene, fit_torn, slide_edge};
pub use style::VanishingPointStyle;

use egui::load::SizedTexture;
use egui::{Context, Key, Modifiers, Rect};

use craft_ui::canvas::kurbo::Size;
use craft_ui::canvas::{Checkerboard, CursorOptions, HandTool, NavigationOptions, RasterSource, ToolHost, ZoomTool};
use craft_ui::dialog::full_window::{
    FullWindowCanvas, FullWindowLabels, FullWindowSide, FullWindowTitle, FullWindowView, PreviewStatus, PreviewTicket, SideFooter, SideWidth,
};
use craft_ui::dialog::{ButtonRole, DialogLabels, DialogState, FooterButton, FullWindowDialog, LocalHistory, Outcome, Step};
use craft_ui::edit::{self, CancelReason, EditEvent};
use craft_ui::icons::Icon;
use craft_ui::localized::Labels as _;
use craft_ui::patterns::{LastUsed, RailTool, ToolRail, ToolSlot};
use craft_ui::theme::current;

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
    pub settings: VanishingPointSettings,
    /// The engine's clone-stroke limits.
    pub limits: StampLimits,
    /// The user's navigation options (Photoshop's defaults).
    pub navigation: NavigationOptions,
    /// The user's transparency grid.
    pub checkerboard: Checkerboard,
    /// The user's Preferences › Cursors.
    pub cursor_options: CursorOptions,
}

impl VanishingPointOpen {
    pub fn new(size: [u32; 2], subject: impl Into<String>, limits: StampLimits) -> Self {
        Self {
            size,
            subject: subject.into(),
            session: VanishingPointSession::default(),
            settings: VanishingPointSettings::default(),
            limits,
            navigation: NavigationOptions::default(),
            checkerboard: Checkerboard::default(),
            cursor_options: CursorOptions::default(),
        }
    }

    pub fn settings(mut self, settings: VanishingPointSettings) -> Self {
        self.settings = settings;
        self
    }

    /// The user's navigation options, for the canvas, Hand and Zoom.
    pub fn navigation(mut self, options: NavigationOptions) -> Self {
        self.navigation = options;
        self
    }

    /// The user's transparency grid under the image.
    pub fn checkerboard(mut self, checkerboard: Checkerboard) -> Self {
        self.checkerboard = checkerboard;
        self
    }

    /// The user's Preferences › Cursors, for the editor's cursor.
    pub fn cursor_options(mut self, options: CursorOptions) -> Self {
        self.cursor_options = options;
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
    /// The bar changed a setting (the app stores the settings).
    pub settings_changed: bool,
    /// The message of a refused Stamp press, translated, for the app's status line.
    pub status: Option<String>,
    pub outcome: Option<Outcome>,
    pub rect: Rect,
}

/// The Vanishing Point workspace.
///
/// Keys: V, C, S, H, Z (`ToolRail::pick_by_key`); Delete and Backspace the selected plane; ⌘+, ⌘−,
/// ⌘0, ⌘1 the view; ⌘Z, ⇧⌘Z the history (a gesture in progress is cancelled first, with
/// `CancelReason::UndoDuringGesture`); Enter OK; Esc drops pending corners first, else Cancel. A
/// tool switch cancels the old tool's gesture with `CancelReason::Removed`.
pub struct VanishingPointWorkspace {
    dialog: DialogState<VanishingPointSession>,
    session: VanishingPointSession,
    history: LocalHistory<VanishingPointSession>,
    settings: VanishingPointSettings,
    subject: String,
    size: [u32; 2],
    // View state outside the history (risk 8).
    view: FullWindowView,
    strip: VanishingPointStripTool,
    editor: VanishingPointEditor,
    preview: PreviewStatus,
    /// The strokes changed outside `show` (the app's Edit menu stepped the history): the next pass
    /// asks for a new preview.
    stale: bool,
    last_used: LastUsed<Pick>,
    hand: HandTool,
    zoom: ZoomTool,
    navigation: NavigationOptions,
    checkerboard: Checkerboard,
    /// The words; the installed words while `None`.
    labels: Option<VanishingPointLabels>,
}

/// A button of the strip: the editor's three tools, Hand and Zoom.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pick {
    EditPlane,
    CreatePlane,
    Stamp,
    Hand,
    Zoom,
}

impl Pick {
    fn of(strip: VanishingPointStripTool, editor: &VanishingPointEditor) -> Pick {
        match (strip, editor.tool()) {
            (VanishingPointStripTool::Hand, _) => Pick::Hand,
            (VanishingPointStripTool::Zoom, _) => Pick::Zoom,
            (VanishingPointStripTool::Editor, VanishingTool::EditPlane) => Pick::EditPlane,
            (VanishingPointStripTool::Editor, VanishingTool::CreatePlane) => Pick::CreatePlane,
            (VanishingPointStripTool::Editor, VanishingTool::Stamp) => Pick::Stamp,
        }
    }

    /// The strip: Edit Plane (V), Create Plane (C), Stamp (S), a gap, Hand (H) and Zoom (Z).
    fn slots(labels: &VanishingPointLabels) -> Vec<ToolSlot<Pick>> {
        // pc: crates/ui-egui/src/dialogs/vp_ui.rs:1052-1059 (the letters), :1230 (the tool pills the
        // strip replaces). The icons are library `Icon`s until app icons land.
        let tools = [
            (Pick::EditPlane, Icon::Pointer, &labels.edit_plane, "V"),
            (Pick::CreatePlane, Icon::Grid, &labels.create_plane, "C"),
            (Pick::Stamp, Icon::Stamp, &labels.stamp, "S"),
            (Pick::Hand, Icon::Hand, &labels.hand, "H"),
            (Pick::Zoom, Icon::ZoomIn, &labels.zoom, "Z"),
        ];
        (0_u32..)
            .zip(tools)
            .map(|(id, (pick, icon, label, letter))| {
                ToolSlot::new(id, vec![RailTool::new(pick, icon, label.as_str()).shortcut(letter)]).group_end(pick == Pick::Stamp)
            })
            .collect()
    }

    fn select(self, strip: &mut VanishingPointStripTool, editor: &mut VanishingPointEditor) {
        match self {
            Pick::Hand => *strip = VanishingPointStripTool::Hand,
            Pick::Zoom => *strip = VanishingPointStripTool::Zoom,
            Pick::EditPlane | Pick::CreatePlane | Pick::Stamp => {
                *strip = VanishingPointStripTool::Editor;
                editor.set_tool(match self {
                    Pick::EditPlane => VanishingTool::EditPlane,
                    Pick::CreatePlane => VanishingTool::CreatePlane,
                    _ => VanishingTool::Stamp,
                });
            }
        }
    }
}

/// The canvas's id salt, shared by the canvas and the tool switch that cancels its gesture.
const CANVAS: &str = "vanishing-point-view";

impl VanishingPointWorkspace {
    pub fn new(open: VanishingPointOpen) -> Self {
        let mut editor = VanishingPointEditor::new(open.limits);
        editor.cursor_options(open.cursor_options);
        let size = Size::new(f64::from(open.size[0]), f64::from(open.size[1]));
        Self {
            dialog: DialogState::new(),
            history: LocalHistory::new(&open.session),
            session: open.session,
            settings: open.settings,
            subject: open.subject,
            size: open.size,
            view: FullWindowView::new(),
            strip: VanishingPointStripTool::Editor,
            editor,
            preview: PreviewStatus::new(),
            stale: false,
            last_used: LastUsed::default(),
            hand: HandTool::new(size, open.navigation),
            zoom: ZoomTool::new(size, open.navigation),
            navigation: open.navigation,
            checkerboard: open.checkerboard,
            labels: None,
        }
    }

    /// The words (default: the installed words).
    pub fn labels(mut self, labels: VanishingPointLabels) -> Self {
        self.editor.labels(labels.clone());
        self.labels = Some(labels);
        self
    }

    /// The marks' look from the theme's tokens.
    pub fn style(tokens: &craft_ui::theme::UiTokens) -> VanishingPointStyle {
        VanishingPointStyle::of(tokens)
    }

    /// Selects plane `index` (none when it is out of range or `None`), for an app or a story that
    /// opens on a chosen plane.
    pub fn selected_plane(&mut self, index: Option<usize>) -> &mut Self {
        self.editor.select(index, &self.session);
        self
    }

    /// Shows the workspace for this pass: the title, the keys, [`VanishingPointBar`], the strip
    /// (the editor's three tools, `group_end`, Hand and Zoom), the narrow side column with
    /// `SideFooter::bottom` OK and Cancel and the preview's status, and the canvas with the
    /// source's image and the editor's pass (or Hand, Zoom). Then the history settles (busy while
    /// the editor drags or a bar field is being edited).
    pub fn show(&mut self, ctx: &Context, scene: &dyn VanishingScene, source: &dyn VanishingPointSource) -> VanishingPointOutput {
        // pc: crates/ui-egui/src/dialogs/vp_ui.rs:1230 `options_bar`, :1045 `keys`, :1068
        // `escape`; distort_ui.rs:352-368 (the mode on the main canvas, replaced by the frame).
        let Self {
            dialog,
            session,
            history,
            settings,
            subject,
            size,
            view,
            strip,
            editor,
            preview,
            stale,
            last_used,
            hand,
            zoom,
            navigation,
            checkerboard,
            labels: custom,
        } = self;
        let installed;
        let labels = match custom.as_ref() {
            Some(labels) => labels,
            None => {
                installed = VanishingPointLabels::of_ctx(ctx);
                &installed
            }
        };
        let dialog_labels = DialogLabels::of_ctx(ctx);
        let window_labels = FullWindowLabels::of_ctx(ctx);
        let style = VanishingPointEditor::style(&current(ctx));
        let title = FullWindowTitle::new(labels.name.as_str()).subject(subject.as_str()).zoom(view.zoom_percent()).text(ctx);
        let doc = Size::new(f64::from(size[0]), f64::from(size[1]));
        let navigation = *navigation;
        let checkerboard = *checkerboard;
        let before = *settings;

        let mut events = Vec::new();
        let mut wanted = std::mem::take(stale);
        let mut bar_live = false;
        let response = FullWindowDialog::new("vanishing-point").title(title).show(ctx, dialog, session, |ui, session, fx| {
            // The history keys are read whatever has focus, so they work while a number field has
            // it, but only while this workspace is the top modal layer. A gesture in progress ends
            // first, so the step finds the session as the history has it.
            if let Some(step) = fx.take_step(ui) {
                let canvas = FullWindowCanvas::canvas_id(ui, CANVAS);
                cancel_active(ctx, canvas, *strip, hand, zoom, editor, session, settings, scene, source, &style, CancelReason::UndoDuringGesture);
                if history.step(session, step) {
                    editor.after_step(session);
                    wanted = true;
                }
            }
            // Esc drops half-clicked corners before it cancels the workspace.
            if fx.keys_free(ui) && ctx.input(|i| i.key_pressed(Key::Escape) && i.modifiers.is_none()) && editor.drop_pending() {
                ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
            }

            let new_layer = settings.new_layer;
            let mut bar = VanishingPointBar::new("vanishing-point-bar", editor.tool(), settings, session.paste, source.paste().is_some())
                .has_strokes(!session.strokes.is_empty());
            if let Some(labels) = custom.as_ref() {
                bar = bar.labels(labels.clone());
            }
            let bar = bar.show(ui);
            bar_live = bar.edit.as_ref().and_then(EditEvent::id).is_some_and(|id| edit::is_live(ctx, id));
            if let Some(paste) = bar.paste {
                session.paste = paste;
                events.push(VanishingPointEvent::Paste(EditEvent::set(paste, Modifiers::NONE)));
            }
            wanted |= settings.new_layer != new_layer && !session.strokes.is_empty();

            let slots = Pick::slots(labels);
            let current = Pick::of(*strip, editor);
            let rail = ToolRail::new("vanishing-point-tools").label(labels.tools.as_str()).pick_by_key(fx.keys_free(ui)).show(
                ui,
                &slots,
                &current,
                last_used,
                |_, _| {},
            );
            if let Some(pick) = rail.select.filter(|pick| *pick != current) {
                // A gesture in progress ends with the tool that began it.
                let canvas = FullWindowCanvas::canvas_id(ui, CANVAS);
                cancel_active(ctx, canvas, *strip, hand, zoom, editor, session, settings, scene, source, &style, CancelReason::Removed);
                pick.select(strip, editor);
            }

            let footer = SideFooter::bottom(vec![
                FooterButton::new(ButtonRole::Default, dialog_labels.ok.as_str()),
                FooterButton::new(ButtonRole::Cancel, dialog_labels.cancel.as_str()),
            ]);
            FullWindowSide::new("vanishing-point-side").width(SideWidth::Narrow).footer(footer).show(ui, fx, |ui| {
                preview.show(ui, &window_labels);
            });

            // While strokes exist the canvas shows the app's copy of the layer with them painted on.
            let image: &dyn RasterSource = match source.stamp_preview().filter(|_| !session.strokes.is_empty()) {
                Some(preview) => preview,
                None => source.image(),
            };
            let widget = FullWindowCanvas::new(CANVAS, view).source(image).checkerboard(checkerboard).navigation(navigation);
            let shown = match *strip {
                VanishingPointStripTool::Hand => {
                    hand.size = doc;
                    hand.prefs = navigation;
                    widget.tool(hand).show(ui)
                }
                VanishingPointStripTool::Zoom => {
                    zoom.size = doc;
                    zoom.prefs = navigation;
                    widget.tool(zoom).show(ui)
                }
                VanishingPointStripTool::Editor => widget.tool(&mut editor.pass(session, settings, scene, source, &style)).show(ui),
            };
            // The Esc that cancelled a gesture is not the dialog's Cancel.
            if shown.took_key.is_some_and(|key| key.key == Key::Escape) {
                ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
            }
            events.extend(editor.take_events());
        });
        // A plane or the paste changed for good: the app's strokes and paste are baked on the planes.
        wanted |= (!session.strokes.is_empty() || settings.paste)
            && events.iter().any(|event| {
                matches!(
                    event,
                    VanishingPointEvent::Planes(EditEvent::Commit { .. } | EditEvent::Set { .. })
                        | VanishingPointEvent::Paste(EditEvent::Commit { .. } | EditEvent::Set { .. })
                )
            });
        history.settle(session, editor.busy() || bar_live || ctx.input(|i| i.pointer.any_down()));
        let preview_wanted = wanted.then(|| preview.request());
        VanishingPointOutput {
            events,
            preview_wanted,
            settings_changed: *settings != before,
            status: editor.take_status(),
            outcome: response.outcome,
            rect: response.rect,
        }
    }

    pub fn session(&self) -> &VanishingPointSession {
        &self.session
    }

    pub fn settings(&self) -> &VanishingPointSettings {
        &self.settings
    }

    /// The app's preview for `ticket` is in: the status clears when it is the latest request, and
    /// a result for an older ticket is dropped.
    pub fn preview_ready(&mut self, ticket: PreviewTicket) {
        self.preview.ready(ticket);
    }

    /// The app's preview for `ticket` failed with `message` (translated): shown in the side
    /// column when it is the latest request.
    pub fn preview_failed(&mut self, ticket: PreviewTicket, message: String) {
        self.preview.failed(ticket, message);
    }

    /// Undo or redo from the app's Edit menu; returns whether it stepped. A drag in progress
    /// refuses the step: the workspace's own ⌘Z cancels the gesture first, an app menu waits for
    /// the release.
    pub fn step(&mut self, step: Step) -> bool {
        // pc: crates/ui-egui/src/dialogs/vp_ui.rs:383 `step`.
        if self.editor.busy() {
            return false;
        }
        let stepped = self.history.step(&mut self.session, step);
        if stepped {
            self.editor.after_step(&self.session);
            self.stale = true;
        }
        stepped
    }
}

/// Ends the gesture of the tool the strip has picked, with `reason`: the host reports the tool's
/// `Cancel` (the editor puts back what the press found).
#[expect(clippy::too_many_arguments, reason = "the editor's pass takes the whole workspace state")]
fn cancel_active(
    ctx: &Context,
    canvas: craft_ui::canvas::CanvasId,
    strip: VanishingPointStripTool,
    hand: &mut HandTool,
    zoom: &mut ZoomTool,
    editor: &mut VanishingPointEditor,
    session: &mut VanishingPointSession,
    settings: &VanishingPointSettings,
    scene: &dyn VanishingScene,
    source: &dyn VanishingPointSource,
    style: &VanishingPointStyle,
    reason: CancelReason,
) {
    match strip {
        VanishingPointStripTool::Hand => ToolHost::cancel(ctx, canvas, hand, reason),
        VanishingPointStripTool::Zoom => ToolHost::cancel(ctx, canvas, zoom, reason),
        VanishingPointStripTool::Editor => ToolHost::cancel(ctx, canvas, &mut editor.pass(session, settings, scene, source, style), reason),
    };
}
