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

pub(crate) use labels::CONSTRAINTS;
pub use labels::WideAngleLabels;
pub use panel::{WideAnglePanel, WideAnglePanelOutput};
pub use session::{ConstraintOrientation, WideAngleConstraint, WideAngleConstraintEdit, WideAngleGeometry, WideAngleModel, WideAngleSession};
pub use style::WideAngleStyle;
pub use tool::{ConstraintTool, ConstraintToolPass};

use egui::{Context, Key, Modifiers, Rect};

use craft_ui::canvas::kurbo::Size;
use craft_ui::canvas::{HandTool, NavigationOptions, RasterSource, ZoomTool};
use craft_ui::dialog::full_window::{
    FullWindowCanvas, FullWindowLabels, FullWindowSide, FullWindowTitle, FullWindowView, PreviewStatus, PreviewTicket, SideFooter,
};
use craft_ui::dialog::{ButtonRole, DialogLabels, DialogState, FooterButton, FullWindowDialog, LocalHistory, Outcome, Step, take_step};
use craft_ui::edit::EditEvent;
use craft_ui::icons::Icon;
use craft_ui::localized::Labels as _;
use craft_ui::menu::{ContextMenu, MenuEntry, MenuItem, OpenAt};
use craft_ui::patterns::{LastUsed, RailTool, ToolRail, ToolSlot};
use craft_ui::theme::current;

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
    hand: HandTool,
    zoom: ZoomTool,
    /// Preview: the corrected image (on) or the source (off); the source shows while a drag runs.
    preview_on: bool,
    preview: PreviewStatus,
    /// The session the latest preview ticket was asked for.
    asked: WideAngleSession,
}

/// A command of the constraint menu.
#[derive(Clone, Copy, Debug, PartialEq)]
enum MenuCommand {
    Orient(ConstraintOrientation),
    Delete,
}

/// Applies `edit` to `session` and reports it when it changed something.
fn record(session: &mut WideAngleSession, events: &mut Vec<WideAngleEvent>, edit: WideAngleConstraintEdit) {
    if session.apply(edit.clone()) {
        events.push(WideAngleEvent::Constraints(EditEvent::set(edit, Modifiers::NONE)));
    }
}

impl WideAngleWorkspace {
    pub fn new(open: WideAngleOpen) -> Self {
        Self {
            dialog: DialogState::new(),
            history: LocalHistory::new(&open.session),
            asked: open.session.clone(),
            session: open.session,
            subject: open.subject,
            size: open.size,
            view: FullWindowView::new(),
            tool: WideAngleTool::Constraint,
            last_used: LastUsed::default(),
            constraint: ConstraintTool::new(),
            hand: HandTool::new(Size::ZERO, NavigationOptions::default()),
            zoom: ZoomTool::new(Size::ZERO, NavigationOptions::default()),
            preview_on: true,
            preview: PreviewStatus::new(),
        }
    }

    /// Shows the workspace for this pass: the title, the strip, the side column ([`WideAnglePanel`],
    /// `SideFooter::bottom` OK and Cancel), the canvas (the corrected preview while Preview is on
    /// and no drag runs, else the source) with [`ConstraintTool`] (or Hand, Zoom), and the
    /// constraint menu while `menu_for` is set. Edits go through [`WideAngleSession::apply`]; the
    /// history settles (busy while dragging); a change asks for a ticket.
    ///
    /// The canvas is in the source's pixels and the constraints in document pixels (`open.size`);
    /// the tool converts between them, so the source may be a proxy of the layer.
    pub fn show(&mut self, ctx: &Context, geometry: &dyn WideAngleGeometry, source: &dyn WideAngleSource) -> WideAngleOutput {
        // pc: crates/ui-egui/src/dialogs/wide_angle_ui.rs:256 `show` (title :272, the image and
        // constraints, the panel), :232 `constraint_menu`.
        let Self { dialog, session, history, subject, size, view, tool, last_used, constraint, hand, zoom, preview_on, preview, asked } = self;
        let labels = WideAngleLabels::of_ctx(ctx);
        let dialog_labels = DialogLabels::of_ctx(ctx);
        let window_labels = FullWindowLabels::of_ctx(ctx);
        let style = WideAngleStyle::of(&current(ctx));
        let title = FullWindowTitle::new(labels.name.as_str()).subject(subject.as_str()).zoom(view.zoom_percent()).text(ctx);
        let image = source.source();
        let corrected = source.corrected().filter(|_| *preview_on && !constraint.dragging());
        let shown: &dyn RasterSource = corrected.unwrap_or(image);
        let (canvas_size, document) = (shown.size(), *size);
        let navigation_size = Size::new(f64::from(canvas_size[0]), f64::from(canvas_size[1]));
        hand.size = navigation_size;
        zoom.size = navigation_size;

        let mut events = Vec::new();
        let mut busy = false;
        let response = FullWindowDialog::new("wide-angle").title(title).show(ctx, dialog, session, |ui, session, fx| {
            let ctx = ui.ctx().clone();
            // The history, while this dialog is the top modal layer (a number field keeps ⌘Z too).
            if ctx.memory(|m| m.top_modal_layer()) == Some(ui.layer_id())
                && let Some(step) = take_step(&ctx)
                && history.step(session, step)
            {
                constraint.drop_drag();
                constraint.sync(session.constraints.len());
            }

            let slots = [
                (WideAngleTool::Constraint, Icon::Slash, labels.constraint_tool.as_str(), "A"),
                (WideAngleTool::Hand, Icon::Hand, labels.hand.as_str(), "H"),
                (WideAngleTool::Zoom, Icon::ZoomIn, labels.zoom.as_str(), "Z"),
            ]
            .into_iter()
            .zip(0..)
            .map(|((id, icon, label, key), slot)| ToolSlot::new(slot, vec![RailTool::new(id, icon, label).shortcut(key)]))
            .collect::<Vec<_>>();
            let rail = ToolRail::new("wide-angle-tools").label(labels.tools.as_str()).pick_by_key(true).show(ui, &slots, tool, last_used, |_, _| {});
            if let Some(next) = rail.select
                && next != *tool
            {
                constraint.drop_drag();
                *tool = next;
            }

            let buttons = vec![
                FooterButton::new(ButtonRole::Default, dialog_labels.ok.as_str()),
                FooterButton::new(ButtonRole::Cancel, dialog_labels.cancel.as_str()),
            ];
            FullWindowSide::new("wide-angle-side").header(labels.settings.as_str()).footer(SideFooter::bottom(buttons)).show(ui, fx, |ui| {
                let panel = WideAnglePanel::new("wide-angle-panel", session, preview_on, &labels).show(ui);
                busy |= panel.dragging;
                if let Some(edit) = panel.edit {
                    record(session, &mut events, edit);
                }
                preview.show(ui, &window_labels);
            });

            let mut pass = constraint.pass(session, geometry, &style, &labels).image(canvas_size, document).corrected(corrected.is_some());
            let canvas = FullWindowCanvas::new("wide-angle-view", view).source(shown);
            let canvas = match *tool {
                WideAngleTool::Constraint => canvas.tool(&mut pass),
                WideAngleTool::Hand => canvas.tool(hand),
                WideAngleTool::Zoom => canvas.tool(zoom),
            };
            let out = canvas.show(ui);
            // Esc ended a drag: the dialog does not also cancel.
            if out.took_key.is_some_and(|key| key.key == Key::Escape) {
                ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
            }

            if let Some(index) = constraint.menu_for()
                && let Some(current) = session.constraints.get(index).map(|c| c.orientation)
            {
                let entries: Vec<MenuEntry<MenuCommand>> = [
                    (ConstraintOrientation::Free, &labels.free),
                    (ConstraintOrientation::Horizontal, &labels.horizontal),
                    (ConstraintOrientation::Vertical, &labels.vertical),
                ]
                .into_iter()
                .map(|(orientation, label)| MenuItem::new(label.as_str(), MenuCommand::Orient(orientation)).radio(orientation == current).into())
                .chain([MenuEntry::Separator, MenuItem::new(labels.delete.as_str(), MenuCommand::Delete).into()])
                .collect();
                let request = constraint.take_menu_at().map(OpenAt::Pointer);
                let asked_now = request.is_some();
                let menu = ContextMenu::new(&entries).own_gestures(false).open_at(request).show(ui, &out.response);
                match menu.chosen {
                    Some(MenuCommand::Orient(orientation)) => record(session, &mut events, WideAngleConstraintEdit::Orient { index, orientation }),
                    Some(MenuCommand::Delete) => record(session, &mut events, WideAngleConstraintEdit::Delete(index)),
                    None => {}
                }
                if menu.chosen.is_some() || (!menu.open && !asked_now) {
                    constraint.close_menu();
                }
            }

            for edit in constraint.take_edits() {
                record(session, &mut events, edit);
            }
            constraint.sync(session.constraints.len());
            busy |= constraint.dragging();
        });

        history.settle(session, busy);
        let preview_wanted = (response.opened || *session != *asked).then(|| {
            *asked = session.clone();
            preview.request()
        });
        WideAngleOutput { events, preview_wanted, outcome: response.outcome, rect: response.rect }
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
    pub fn step(&mut self, step: Step) -> bool {
        // pc: crates/ui-egui/src/dialogs/wide_angle_ui.rs:566 (the test: ⌘Z steps the dialog, not
        // the document).
        let stepped = self.history.step(&mut self.session, step);
        if stepped {
            self.constraint.drop_drag();
            self.constraint.sync(self.session.constraints.len());
        }
        stepped
    }
}
