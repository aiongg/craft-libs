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
use egui::{Context, Key, Modifiers, Rect};

use craft_ui::canvas::kurbo::Size;
use craft_ui::canvas::{Checkerboard, HandTool, NavigationOptions, RasterSource, ToolHost, ZoomTool};
use craft_ui::dialog::full_window::{FullWindowCanvas, FullWindowSide, FullWindowTitle, FullWindowView, PreviewTicket, SideFooter};
use craft_ui::dialog::{ButtonRole, DialogLabels, DialogState, FooterButton, FullWindowDialog, LocalHistory, Outcome, Step, take_step};
use craft_ui::edit::CancelReason;
use craft_ui::edit::EditEvent;
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
    /// The strokes changed outside `show` (the app's Edit menu stepped the history): the next pass
    /// asks for a new preview.
    stale: bool,
    last_used: LastUsed<Pick>,
    hand: HandTool,
    zoom: ZoomTool,
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
            stale: false,
            last_used: LastUsed::default(),
            hand: HandTool::default(),
            zoom: ZoomTool::new(Size::ZERO, NavigationOptions::default()),
        }
    }

    /// Shows the workspace for this pass: the title, the keys, [`VanishingPointBar`], the strip
    /// (the editor's three tools, `group_end`, Hand and Zoom), the narrow side column with
    /// `SideFooter::bottom` OK and Cancel, and the canvas with the source's image and the editor's
    /// pass (or Hand, Zoom). Then the history settles (busy while the editor drags).
    pub fn show(&mut self, ctx: &Context, scene: &dyn VanishingScene, source: &dyn VanishingPointSource) -> VanishingPointOutput {
        // pc: crates/ui-egui/src/dialogs/vp_ui.rs:1230 `options_bar`, :1045 `keys`, :1068
        // `escape`; distort_ui.rs:352-368 (the mode on the main canvas, replaced by the frame).
        let labels = VanishingPointLabels::of_ctx(ctx);
        let dialog_labels = DialogLabels::of_ctx(ctx);
        let style = VanishingPointStyle::of(&current(ctx));
        let title = FullWindowTitle::new(labels.name.as_str()).subject(self.subject.as_str()).zoom(self.view.zoom_percent()).text(ctx);
        let Self { dialog, session, history, options, subject: _, size, view, strip, editor, preview_ticket, stale, last_used, hand, zoom } = self;
        let doc = Size::new(f64::from(size[0]), f64::from(size[1]));
        // The user's navigation options are the app's; Photoshop's defaults until it passes them.
        let navigation = NavigationOptions::default();

        let mut events = Vec::new();
        let mut wanted = std::mem::take(stale);
        let response = FullWindowDialog::new("vanishing-point").title(title).show(ctx, dialog, session, |ui, session, fx| {
            // The history keys are read whatever has focus, so they work while a number field has
            // it, but only while this workspace is the top modal layer.
            if ctx.memory(|m| m.top_modal_layer()) == Some(ui.layer_id()) {
                if let Some(step) = take_step(ctx)
                    && history.step(session, step)
                {
                    editor.after_step(session);
                    wanted = true;
                }
                // Esc drops half-clicked corners before it cancels the workspace.
                if ctx.input(|i| i.key_pressed(Key::Escape) && i.modifiers.is_none()) && editor.drop_pending() {
                    ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
                }
            }

            let new_layer = options.new_layer;
            let bar =
                VanishingPointBar::new("vanishing-point-bar", editor.tool(), options, session.paste, source.paste().is_some(), &labels).show(ui);
            if let Some(paste) = bar.paste {
                session.paste = paste;
                events.push(VanishingPointEvent::Paste(EditEvent::set(paste, Modifiers::NONE)));
            }
            wanted |= options.new_layer != new_layer && !session.strokes.is_empty();

            let slots = Pick::slots(&labels);
            let current = Pick::of(*strip, editor);
            let rail = ToolRail::new("vanishing-point-tools").label(labels.tools.as_str()).pick_by_key(true).show(
                ui,
                &slots,
                &current,
                last_used,
                |_, _| {},
            );
            if let Some(pick) = rail.select.filter(|pick| *pick != current) {
                // A gesture in progress ends with the tool that began it.
                let id = FullWindowCanvas::canvas_id(ui, CANVAS);
                match *strip {
                    VanishingPointStripTool::Hand => ToolHost::cancel(ctx, id, hand, CancelReason::Removed),
                    VanishingPointStripTool::Zoom => ToolHost::cancel(ctx, id, zoom, CancelReason::Removed),
                    VanishingPointStripTool::Editor => {
                        ToolHost::cancel(ctx, id, &mut editor.pass(session, options, scene, source, &style, &labels), CancelReason::Removed)
                    }
                };
                pick.select(strip, editor);
            }

            let footer = SideFooter::bottom(vec![
                FooterButton::new(ButtonRole::Default, dialog_labels.ok.as_str()),
                FooterButton::new(ButtonRole::Cancel, dialog_labels.cancel.as_str()),
            ]);
            FullWindowSide::new("vanishing-point-side").footer(footer).show(ui, fx, |_| {});

            // While strokes exist the canvas shows the app's copy of the layer with them painted on.
            let image: &dyn RasterSource = match source.stamp_preview().filter(|_| !session.strokes.is_empty()) {
                Some(preview) => preview,
                None => source.image(),
            };
            let widget = FullWindowCanvas::new(CANVAS, view).source(image).checkerboard(Checkerboard::default()).navigation(navigation);
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
                VanishingPointStripTool::Editor => widget.tool(&mut editor.pass(session, options, scene, source, &style, &labels)).show(ui),
            };
            // The Esc that cancelled a gesture is not the dialog's Cancel.
            if shown.took_key.is_some_and(|key| key.key == Key::Escape) {
                ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
            }
            events.extend(editor.take_events());
        });
        // A plane or the paste changed for good: the app's strokes and paste are baked on the planes.
        wanted |= (!session.strokes.is_empty() || options.paste)
            && events.iter().any(|event| {
                matches!(
                    event,
                    VanishingPointEvent::Planes(EditEvent::Commit { .. } | EditEvent::Set { .. })
                        | VanishingPointEvent::Paste(EditEvent::Commit { .. } | EditEvent::Set { .. })
                )
            });
        history.settle(session, editor.busy() || ctx.input(|i| i.pointer.any_down()));
        let preview_wanted = wanted.then(|| {
            *preview_ticket += 1;
            PreviewTicket(*preview_ticket)
        });
        VanishingPointOutput { events, preview_wanted, outcome: response.outcome, rect: response.rect }
    }

    pub fn session(&self) -> &VanishingPointSession {
        &self.session
    }

    pub fn options(&self) -> &VanishingPointOptions {
        &self.options
    }

    /// Undo or redo from the app's Edit menu; returns whether it stepped.
    pub fn step(&mut self, step: Step) -> bool {
        // pc: crates/ui-egui/src/dialogs/vp_ui.rs:383 `step`.
        let stepped = self.history.step(&mut self.session, step);
        if stepped {
            self.editor.after_step(&self.session);
            self.stale = true;
        }
        stepped
    }
}
