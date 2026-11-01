//! [`CropTool`]: the Crop tool on the canvas tool host. The app owns one `CropTool` and builds a
//! [`CropToolPass`] each pass from it and the pass's input ([`CropInput`]), so the snap closure can
//! borrow the app's snap data (lead call 11).

use egui::{Color32, CursorIcon, Key, Modifiers, Painter, Pos2};
use kurbo::{Affine, Point, Rect, Vec2};

use super::crop_frame::{CropBox, CropFrame, CropKeyAction, CropLabels, CropNames, CropPress, CropPressKind, CropReadout, CropRelease};
use super::options::{CropFill, CropSettings, CropTarget};
use super::session::{CropDocument, CropKept, CropSession, LeaveFrame};
use crate::ui::transform::transform_box::{key_part, part_key};
use craft_ui::canvas::quad::BoxPart;
use craft_ui::canvas::{
    CanvasTool, CanvasView, RulerUnits, ToolAction, ToolContext, ToolCursor, ToolKey, ToolParts, ToolPointer, resize_cursor, turn_cursor,
};
use craft_ui::edit::{CancelReason, EditEvent, GestureId};
use craft_ui::localized::Labels as _;
use craft_ui::theme::ToEgui as _;

/// The host's snapping of a dragged part or a Straighten line end, as
/// [`CropFrame::snap`](super::CropFrame::snap) takes it: the offset to add to the part's point.
type Snap<'a> = &'a dyn Fn(BoxPart, &CropBox, Modifiers) -> Vec2;

/// What the app crops with: the frame (its rect before the turn, and `angle`, the turn in degrees
/// clockwise on screen), the size to resample to, Delete Cropped Pixels and Fill. The app maps it
/// to its crop command; a commit it refuses goes back with [`CropTool::applied`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CropCommit {
    pub frame: CropBox,
    /// [`CropSettings::target`]: W × H × Resolution's size, `None` for no resample.
    pub target: Option<CropTarget>,
    pub delete_pixels: bool,
    pub fill: CropFill,
}

impl CropCommit {
    /// The commit of `frame` with `settings` on `document`.
    pub fn new(frame: CropBox, settings: &CropSettings, document: &CropDocument) -> Self {
        Self { frame, target: settings.target(document), delete_pixels: settings.delete_pixels, fill: settings.fill }
    }
}

/// What the Crop tool reports. One pass's events come in order and are handled in that pass.
#[derive(Clone, Debug, PartialEq)]
pub enum CropEvent {
    /// The frame changed: a drag (Begin on the first move, Update, Commit on release; Cancel on
    /// Esc or a frame under one unit) or a key, refit, undo step or Straighten (`Set`). Mirrored,
    /// not applied: the frame belongs to the tool's [`CropSession`]. Not a document step.
    Frame(EditEvent<CropBox>),
    /// Crop the document (Enter, a double click inside a drawn frame, ✓, Image › Crop, leaving the
    /// tool with a drawn frame): one document step. The app then calls [`CropTool::applied`].
    Apply(CropCommit),
    /// Esc with no drag, ⊘, or a frame of another document: the frame is gone.
    Discard,
    /// O, ⇧O, H, P or I changed the options: the app stores the new settings.
    Settings(CropSettings),
    /// A Straighten line set the frame's angle (degrees, clockwise on screen) and fitted the frame
    /// to the canvas, as one box step; Straighten is disarmed.
    Straightened(f64),
}

/// What a pass of the tool reports, from [`CropToolPass::finish`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CropOutput {
    pub events: Vec<CropEvent>,
    /// The frame after the pass: the default one or the drawn one; `None` without a document.
    /// The app's control channel mirrors it.
    pub frame: Option<CropBox>,
    /// The frame has been pressed since it was made: the app shows the layers' pixels past the
    /// canvas edges under the shield (Photoshop's crop preview).
    pub shows_beyond_canvas: bool,
    /// A drag of the frame runs: [`GuideShow::Auto`](super::GuideShow::Auto) shows the overlay
    /// and Auto Adjust Opacity lightens the shield.
    pub dragging: bool,
}

/// One pass's input: the options, the document, and how the frame reads and snaps.
pub struct CropInput<'a> {
    settings: &'a CropSettings,
    document: &'a CropDocument,
    units: Option<(&'a RulerUnits, &'a RulerUnits)>,
    snap: Option<Snap<'a>>,
    names: Option<CropNames>,
    readout: Option<CropLabels>,
    canvas_colour: Option<Color32>,
}

impl<'a> CropInput<'a> {
    /// The pass on `document` with `settings`.
    pub fn new(settings: &'a CropSettings, document: &'a CropDocument) -> Self {
        Self { settings, document, units: None, snap: None, names: None, readout: None, canvas_colour: None }
    }

    /// The ruler units of the readout beside the pointer and of the parts' values, as
    /// [`CropFrame::units`](super::CropFrame::units).
    pub fn units(mut self, x: &'a RulerUnits, y: &'a RulerUnits) -> Self {
        self.units = Some((x, y));
        self
    }

    /// The host's snapping of a dragged part and of a Straighten line's ends (row 101: the snap
    /// targets and smart guides are the app's).
    pub fn snap(mut self, snap: Snap<'a>) -> Self {
        self.snap = Some(snap);
        self
    }

    /// The AccessKit names; default: [`CropNames`] of the installed words.
    pub fn names(mut self, names: CropNames) -> Self {
        self.names = Some(names);
        self
    }

    /// The readout's captions beside the pointer while the frame is drawn, resized or turned;
    /// default: [`CropLabels`] of the installed words.
    pub fn readout(mut self, labels: CropLabels) -> Self {
        self.readout = Some(labels);
        self
    }

    /// The canvas (pasteboard) colour the shield matches and Show Cropped Area hides behind;
    /// default: `surface.canvas`.
    pub fn canvas_colour(mut self, colour: Color32) -> Self {
        self.canvas_colour = Some(colour);
        self
    }
}

/// The Crop tool, kept by the app while the tool is picked and between picks.
///
/// ```no_run
/// use craft_draw::ui::crop::{CropDocKey, CropDocument, CropEvent, CropInput, CropSettings, CropTool};
/// use craft_ui::canvas::{CanvasId, CanvasView, ToolHost};
/// # fn demo(ui: &mut egui::Ui, view: &CanvasView, canvas: &egui::Response, id: CanvasId, settings: &mut CropSettings, crop: &mut CropTool) {
/// let document = CropDocument::new(CropDocKey(1), kurbo::Size::new(1200.0, 800.0)).resolution(300.0);
/// let mut pass = crop.pass(CropInput::new(settings, &document));
/// let host = ToolHost::new(id).document(document.size).show(ui, view, canvas, &mut pass);
/// let out = pass.finish();
/// for event in out.events {
///     match event {
///         CropEvent::Apply(commit) => { /* run the crop command, then */ crop.applied(true); }
///         CropEvent::Settings(next) => *settings = next,
///         _ => {}
///     }
/// }
/// # let _ = host;
/// # }
/// ```
///
/// - Pointer (Photoshop's rules, as [`CropFrame`](super::CropFrame) has them): with no frame or
///   inside the untouched default frame a drag draws a frame (⇧ square, ⌥ from the centre, Space
///   moves it while drawn); inside a frame a drag moves it; on an edge or corner it resizes along
///   the frame's own axes (⇧ keeps the ratio, ⌥ about the centre; a ratio from the options always
///   holds); outside it turns the frame about its centre (⇧ in 15° steps). W × H shows beside the
///   pointer while drawing or resizing, the angle while turning. A double click inside a drawn
///   frame (not on a handle) applies.
/// - Image mode (Classic Mode off, the default): the frame stays upright and the gestures move and
///   turn the image; the tool sets the turned view ([`ToolContext::set_view`]) and the app adopts
///   [`ToolHostResponse::view`](craft_ui::canvas::ToolHostResponse::view). Classic Mode: the
///   frame moves and turns over a fixed image. Auto Center Preview re-centres the view in Image
///   mode only.
/// - Keys while the canvas has the keyboard and no drag runs: Enter applies; Esc discards (during
///   a drag the host cancels only the drag); arrows nudge the frame 1 unit, ⇧ 10 (in Image mode
///   they move the image); X swaps portrait and landscape (and W with H in W × H × Resolution);
///   O cycles the overlay, ⇧O its orientation, H toggles Show Cropped Area (taken but inert in
///   Classic Mode), P toggles Classic Mode: these four only while a frame exists and no drag runs,
///   otherwise [`key`](CanvasTool::key) returns `false` and the app's tool shortcuts (Dodge, Hand,
///   Pen) get the letter. I (Front Image) acts whenever no drag runs.
/// - Straighten: [`toggle_straighten`](Self::toggle_straighten) (the bar's button) arms one line;
///   ⌘ (Ctrl) held is a temporary Straighten. The line needs `crop.straighten_min` screen points
///   and its ends go through the snap closure; the release levels the frame and fits it to the
///   canvas as one box step.
/// - Keyboard parts: the frame is one Tab stop through the host ([`CanvasTool::parts`]); Page Up
///   and Down pick the whole frame, a corner or an edge, and the arrows move the picked part.
/// - Box Undo and Redo: the app routes Edit › Undo and Redo to [`undo`](Self::undo) and
///   [`redo`](Self::redo) while [`CropSession::pending`]; Photoshop keeps Undo live during a crop.
#[derive(Clone, Debug, Default)]
pub struct CropTool {
    session: CropSession,
    press: Option<CropPress>,
    gesture: Option<GestureId>,
    /// The frame when the press began: the Commit's `changed`.
    before: Option<CropBox>,
    /// The part the keys act on; `None` is the whole frame.
    picked: Option<BoxPart>,
    /// The frame the Image-mode view was last set for: a frame changed outside a gesture (undo,
    /// refit, keys) turns the view on the next pass.
    synced: Option<CropBox>,
    /// The mode the last pass saw (Classic Mode on or off): a change brings the view to the new
    /// mode's.
    classic_seen: Option<bool>,
    /// The frame, box steps and view of the last Apply, until [`applied`](Self::applied).
    kept: Option<CropKept>,
}

impl CropTool {
    pub fn new() -> Self {
        Self::default()
    }

    /// The pending frame and its history.
    pub fn session(&self) -> &CropSession {
        &self.session
    }

    /// This pass's tool for the host, borrowing `input`'s data.
    pub fn pass<'a>(&'a mut self, input: CropInput<'a>) -> CropToolPass<'a> {
        CropToolPass { tool: self, input, output: CropOutput::default() }
    }

    /// The bar's Straighten button is pressed (armed for one line).
    pub fn straighten_armed(&self) -> bool {
        self.session.straighten().armed()
    }

    /// The bar's Straighten button: arms one line, or disarms it.
    pub fn toggle_straighten(&mut self) {
        self.session.straighten_mut().toggle();
    }

    /// Edit › Undo while a crop is pending: one box step back. The next pass turns an Image-mode
    /// view to match (the frame stays put on screen, #7088). `None` when there is nothing to undo.
    pub fn undo(&mut self) -> Option<CropBox> {
        // pc: crates/ui-egui/src/tools/crop_ui.rs:510 `step`.
        let frame = self.session.undo();
        self.stepped(frame)
    }

    /// Edit › Redo while a crop is pending.
    pub fn redo(&mut self) -> Option<CropBox> {
        // pc: crates/ui-egui/src/tools/crop_ui.rs:510 `step`.
        let frame = self.session.redo();
        self.stepped(frame)
    }

    fn stepped(&mut self, frame: Option<CropBox>) -> Option<CropBox> {
        if frame.is_some() {
            self.synced = None;
        }
        frame
    }

    /// The bar changed the ratio or W × H × Resolution ([`CropBarEvent::Ratio`](super::CropBarEvent)):
    /// the frame refits to [`CropSettings::ratio_value`] about its centre, from the frame the
    /// field edit began with while `editing` (#7752), as one box step. Returns the frame's `Set`
    /// for the host's mirror, or `None` when nothing changed.
    pub fn refit(&mut self, settings: &CropSettings, document: &CropDocument, editing: bool) -> Option<CropEvent> {
        // pc: crates/ui-egui/src/tools/crop_ui.rs:243 `apply_ratio`, :254 `apply_ratio_from`, :228
        // `fitted_ratio`, :267 `fit_frame`.
        // The run ends even when nothing refits.
        let base = self.session.ratio_base(editing);
        if self.press.is_some() {
            return None;
        }
        let (current, base) = (self.session.frame()?, base?);
        let rect = fitted_ratio(base.rect, settings.ratio_value(document)?)?;
        let next = CropBox { rect, angle: base.angle };
        // W × H × Resolution can change the output size even when the frame already fits, so it
        // makes the frame a pending crop.
        let sized = settings.target(document).is_some();
        if next == current && (editing || !sized) {
            return None;
        }
        // #6750: fitting the untouched default frame to a ratio alone is not an edit.
        let keep_default = !editing && !sized && self.session.is_default();
        self.session.set(next, keep_default, false);
        Some(CropEvent::Frame(EditEvent::set(next, Modifiers::NONE)))
    }

    /// ✓ in the bar or Image › Crop: [`CropEvent::Apply`] for a drawn frame (`view`: an Image-mode
    /// host's turned view, kept for a refused commit), `None` for the default frame or no frame.
    pub fn apply(&mut self, settings: &CropSettings, document: &CropDocument, view: Option<CanvasView>) -> Option<CropEvent> {
        // pc: crates/ui-egui/src/panels/options_bar.rs:539 (✓ calls `canvas::commit_crop`).
        if !self.session.pending() {
            return None;
        }
        let frame = self.session.frame()?;
        self.kept = Some(self.session.keep(view));
        Some(CropEvent::Apply(CropCommit::new(frame, settings, document)))
    }

    /// ⊘ in the bar: [`CropEvent::Discard`] and the session cleared (the next pass makes a new
    /// default frame); `None` without a frame.
    pub fn discard(&mut self) -> Option<CropEvent> {
        // pc: crates/ui-egui/src/tools/crop_ui.rs:424 `cancel`.
        self.session.frame()?;
        self.reset();
        Some(CropEvent::Discard)
    }

    /// The app picked another tool (not a held temporary one): drops an untouched default frame
    /// and applies a drawn one (`keeps_pending`: the pen's eraser end stands in for the tool; a
    /// running drag also keeps the frame). Disarms Straighten.
    pub fn leave(&mut self, settings: &CropSettings, document: &CropDocument, keeps_pending: bool) -> Option<CropEvent> {
        // pc: crates/ui-egui/src/tools/crop_ui.rs:350 `frame` (leave half), :408 `commit_on_leave`.
        match self.session.leave(keeps_pending) {
            LeaveFrame::Kept => None,
            LeaveFrame::Nothing => {
                self.reset_gesture();
                None
            }
            LeaveFrame::Dropped => {
                self.reset();
                None
            }
            LeaveFrame::Apply(frame) => {
                self.reset_gesture();
                self.kept = Some(self.session.keep(None));
                Some(CropEvent::Apply(CropCommit::new(frame, settings, document)))
            }
        }
    }

    /// The app ran its crop command for the last [`CropEvent::Apply`]: `true` clears the session,
    /// `false` (refused) gives the frame, its box steps and the view back; the view is returned
    /// for the host to adopt.
    pub fn applied(&mut self, accepted: bool) -> Option<CanvasView> {
        // pc: crates/ui-egui/src/tools/crop_ui.rs:457 `Kept`, :469 `restore`.
        let kept = self.kept.take();
        if accepted {
            self.reset();
            return None;
        }
        let view = self.session.restore(kept?);
        // The restored view comes with the frame: nothing to turn.
        self.synced = self.session.frame();
        view
    }

    /// The gesture and the picked part end.
    fn reset_gesture(&mut self) {
        self.press = None;
        self.gesture = None;
        self.before = None;
        self.picked = None;
    }

    /// The session, the gesture and the view's sync are dropped; the next pass makes a new default
    /// frame.
    fn reset(&mut self) {
        self.session.clear();
        self.reset_gesture();
        self.synced = None;
    }
}

/// The Crop tool for one pass of the host ([`CropTool::pass`]).
pub struct CropToolPass<'a> {
    tool: &'a mut CropTool,
    input: CropInput<'a>,
    output: CropOutput,
}

impl CropToolPass<'_> {
    /// What the pass reported: its events in order, the frame, and the crop preview's state.
    pub fn finish(mut self) -> CropOutput {
        // new: the output with `frame`, `shows_beyond_canvas` and `dragging` read from the session
        // and the press.
        self.output.frame = self.tool.session.frame();
        self.output.shows_beyond_canvas = self.tool.session.editing();
        self.output.dragging = self.tool.dragging();
        self.output
    }

    /// The frame for `state` with this pass's options, as every call of the tool drives it.
    fn frame<'f>(input: &'f CropInput<'_>, tool: &CropTool, state: &'f CropBox, modifiers: Modifiers, cx: &ToolContext<'_>) -> CropFrame<'f> {
        let ctx = &cx.canvas.ctx;
        let settings = input.settings;
        let names = input.names.clone().unwrap_or_else(|| CropNames::of_ctx(ctx));
        let mut frame = CropFrame::new("craft_draw::CropTool", state, names)
            .ratio(settings.ratio_value(input.document))
            .default_frame(tool.session.is_default())
            .straighten(tool.session.straighten().on())
            .modifiers(modifiers)
            .image_mode(!settings.shield.classic_mode)
            .auto_center(settings.shield.auto_center_preview)
            .readout(input.readout.clone().unwrap_or_else(|| CropLabels::of_ctx(ctx)));
        if let Some((x, y)) = input.units {
            frame = frame.units(x, y);
        }
        if let Some(snap) = input.snap {
            frame = frame.snap(snap);
        }
        frame
    }

    fn classic(&self) -> bool {
        self.input.settings.shield.classic_mode
    }

    /// The view an Image-mode Apply keeps for a refused commit.
    fn kept_view(&self, cx: &ToolContext<'_>) -> Option<CanvasView> {
        (!self.classic()).then_some(*cx.view)
    }

    fn push(&mut self, event: CropEvent) {
        self.output.events.push(event);
    }

    /// Hands the host the view a call moved, when it differs from the host's.
    fn adopt(view: CanvasView, cx: &mut ToolContext<'_>) {
        if view != *cx.view {
            cx.set_view(view);
        }
    }

    /// The pointer at `pointer` moves the running press: Begin on the first move, Update after;
    /// in Image mode the view the track moved goes to the host.
    fn track(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        let (Some(mut press), Some(frame)) = (self.tool.press, self.tool.session.frame()) else { return };
        let space = cx.canvas.ctx.input(|i| i.key_down(Key::Space));
        let mut view = *cx.view;
        let track =
            Self::frame(&self.input, self.tool, &frame, pointer.modifiers, cx).track(&mut view, &mut press, pointer.screen, pointer.modifiers, space);
        self.tool.press = Some(press);
        if press.kind() == CropPressKind::Straighten {
            return;
        }
        let id = self.tool.gesture.unwrap_or_else(GestureId::next);
        let event = match (track.begun, track.state) {
            (true, Some(next)) => Some(EditEvent::begin_with(id, next)),
            (true, None) => Some(EditEvent::begin(id)),
            (false, Some(next)) => Some(EditEvent::update(id, next, pointer.modifiers)),
            (false, None) => None,
        };
        if let Some(event) = event {
            self.push(CropEvent::Frame(event));
        }
        if let Some(next) = track.state {
            self.tool.session.set(next, false, true);
            self.tool.synced = Some(next);
        }
        if !self.classic() {
            Self::adopt(view, cx);
        }
    }
}

impl CanvasTool for CropToolPass<'_> {
    /// Drops a frame of another document ([`CropEvent::Discard`]), makes the default frame,
    /// reads ⌘ for a temporary Straighten, and in Image mode sets the view for a frame changed
    /// outside a gesture ([`CropFrame::sync_view`](super::CropFrame::sync_view)).
    fn each_pass(&mut self, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/crop_ui.rs:345 `ensure_frame`, :590 `cancel_stale`;
        // crop_straighten.rs:38 `set_held`; crop_mode.rs:93 `sync_frame`.
        let document = self.input.document;
        if self.tool.session.for_document(document.key) {
            self.tool.reset();
            self.push(CropEvent::Discard);
        }
        self.tool.session.straighten_mut().set_held(cx.modifiers.command);
        let dragging = self.tool.press.is_some();
        let made = self.tool.session.ensure_frame(document, dragging);
        if made {
            self.tool.picked = None;
            self.tool.synced = None;
        }
        if dragging {
            return;
        }
        let classic = self.classic();
        let mode_changed = self.tool.classic_seen != Some(classic);
        if mode_changed {
            self.tool.synced = None;
            self.tool.classic_seen = Some(classic);
        }
        let Some(frame) = self.tool.session.frame() else { return };
        if classic {
            // The camera is upright and the frame turns on it (never back to a Rotate View turn).
            if made || mode_changed {
                Self::adopt(CanvasView { rotation: 0.0, ..*cx.view }, cx);
            }
        } else if self.tool.synced != Some(frame) {
            Self::adopt(CropFrame::view_for(&frame, cx.view), cx);
            self.tool.synced = Some(frame);
        }
    }

    /// The frame's hover cursor ([`CropFrame::hit`](super::CropFrame::hit)): resize arrows along
    /// the frame's axes on an edge or corner, Move inside a drawn frame, the crosshair inside the
    /// default frame and while Straighten is on, the turn cursor outside.
    fn cursor(&self, cx: &ToolContext<'_>) -> ToolCursor {
        // pc: crates/ui-egui/src/tools/crop_ui.rs:990 `cursor`, :1020 `turn_cursor_dir`.
        let arrow = ToolCursor::System(CursorIcon::Default);
        let crosshair = ToolCursor::System(CursorIcon::Crosshair);
        let (Some(frame), Some(pointer)) = (self.tool.session.frame().filter(CropBox::is_finite), cx.pointer) else { return arrow };
        let kind = self.tool.press.map(|p| p.kind());
        let part = match kind {
            Some(CropPressKind::Straighten | CropPressKind::Draw) => return crosshair,
            Some(CropPressKind::Move) => BoxPart::Inside,
            Some(CropPressKind::Resize(part)) => part,
            Some(CropPressKind::Turn) => BoxPart::Outside,
            None => {
                let style = CropFrame::style(cx.tokens);
                match Self::frame(&self.input, self.tool, &frame, Modifiers::NONE, cx).hit(cx.view, &style, pointer.screen) {
                    Some(part) => part,
                    // Straighten is on: every press draws its line.
                    None => return crosshair,
                }
            }
        };
        match part {
            BoxPart::Corner(_) | BoxPart::Edge(_) => {
                let (hx, hy) = handle_of(part);
                let (s, c) = frame.angle.to_radians().sin_cos();
                let (x, y) = (f64::from(hx), f64::from(hy));
                ToolCursor::System(resize_cursor(cx.view, Vec2::new(x * c - y * s, x * s + y * c)))
            }
            BoxPart::Outside => {
                turn_cursor(&cx.canvas.ctx, pointer.screen, turn_toward(cx.view, &frame, pointer.screen));
                ToolCursor::Hidden
            }
            // Inside the untouched default frame a drag draws a new one (#4461).
            BoxPart::Inside if kind.is_none() && self.tool.session.is_default() => crosshair,
            BoxPart::Inside => ToolCursor::System(CursorIcon::Move),
            BoxPart::Pivot => arrow,
        }
    }

    /// A press on the canvas: a Straighten line while Straighten is on, else
    /// [`CropFrame::press`](super::CropFrame::press) on the session's frame (unframed: draws).
    /// Picks the part it grabbed; marks the session editing. A second click of a run inside a
    /// drawn frame ([`ToolPointer::clicks`] 2) applies.
    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        // pc: crates/ui-egui/src/tools/crop_ui.rs:817 `begin_drag`, :634 `commits_at`; :794 `straighten`.
        let Some(frame) = self.tool.session.frame().filter(CropBox::is_finite) else { return ToolAction::Ignored };
        let style = CropFrame::style(cx.tokens);
        let (double_click, press) = {
            let host = Self::frame(&self.input, self.tool, &frame, pointer.modifiers, cx);
            (pointer.clicks == 2 && host.double_click(cx.view, &style, pointer.screen), host.press(cx.view, &style, pointer.screen))
        };
        if double_click {
            let view = self.kept_view(cx);
            if let Some(event) = self.tool.apply(self.input.settings, self.input.document, view) {
                self.push(event);
            }
            return ToolAction::Done;
        }
        let Some(press) = press else { return ToolAction::Ignored };
        self.tool.session.press();
        self.tool.picked = match press.kind() {
            CropPressKind::Resize(part) => Some(part),
            CropPressKind::Move => Some(BoxPart::Inside),
            CropPressKind::Draw | CropPressKind::Turn | CropPressKind::Straighten => self.tool.picked,
        };
        self.tool.press = Some(press);
        self.tool.gesture = Some(GestureId::next());
        self.tool.before = Some(frame);
        ToolAction::Claimed
    }

    /// [`CropFrame::track`](super::CropFrame::track) with Space from the modifiers; Begin on the
    /// first move, Update after; in Image mode the view the track moved goes to `set_view`.
    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/crop_ui.rs:909 `update`, :919 `follow_view_drag`, :949
        // `dragged_frame`.
        self.track(pointer, cx);
    }

    /// [`CropFrame::release`](super::CropFrame::release): Commit sets the session's frame (one box
    /// step), Cancel reverts it, Straighten levels and fits it ([`CropEvent::Straightened`]).
    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/crop_ui.rs:862 `release`, :900 `finish_frame`;
        // crop_straighten.rs:70 `pointer` (Up), :133 `apply`.
        // The host sends no move for the release point: the drag ends where the pointer is.
        self.track(pointer, cx);
        let Some(press) = self.tool.press.take() else { return };
        let id = self.tool.gesture.take().unwrap_or_else(GestureId::next);
        let before = self.tool.before.take();
        let Some(frame) = self.tool.session.frame() else { return };
        let style = CropFrame::style(cx.tokens);
        let mut view = *cx.view;
        let released = Self::frame(&self.input, self.tool, &frame, pointer.modifiers, cx).release(&mut view, &style, &press, Some(pointer.screen));
        let image = !self.classic();
        match released {
            CropRelease::Commit(next) => {
                let changed = before != Some(next);
                self.tool.session.set(next, !changed, false);
                self.tool.synced = Some(next);
                self.push(CropEvent::Frame(EditEvent::commit(id, next, changed, pointer.modifiers)));
            }
            CropRelease::Cancel => {
                self.tool.session.revert();
                self.tool.synced = self.tool.session.frame();
                self.push(CropEvent::Frame(EditEvent::cancel(id, CancelReason::Dismissed)));
            }
            CropRelease::Straighten(angle) => {
                // One line per click of the button.
                self.tool.session.straighten_mut().disarm();
                if let Some(next) = frame.fitted(self.input.document.size, angle) {
                    self.tool.session.set(next, false, false);
                    if image {
                        view = CropFrame::view_for(&next, cx.view);
                    }
                    self.tool.synced = Some(next);
                    self.push(CropEvent::Frame(EditEvent::set(next, pointer.modifiers)));
                    self.push(CropEvent::Straightened(angle));
                }
            }
            CropRelease::Nothing => {
                if press.kind() == CropPressKind::Straighten {
                    self.tool.session.straighten_mut().disarm();
                }
            }
        }
        if image {
            Self::adopt(view, cx);
        }
    }

    /// Esc or a lost pointer during a drag: [`CropFrame::cancel`](super::CropFrame::cancel) puts an
    /// Image-mode view back; the frame reverts and the event is Cancel.
    fn cancel(&mut self, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/crop_ui.rs:889 `restore_frame`.
        let Some(press) = self.tool.press.take() else { return };
        let id = self.tool.gesture.take().unwrap_or_else(GestureId::next);
        self.tool.before = None;
        if press.kind() == CropPressKind::Straighten {
            return;
        }
        if let Some(frame) = self.tool.session.frame() {
            let mut view = *cx.view;
            Self::frame(&self.input, self.tool, &frame, Modifiers::NONE, cx).cancel(&mut view, &press);
            if !self.classic() {
                Self::adopt(view, cx);
            }
        }
        if press.began() {
            self.tool.session.revert();
            self.push(CropEvent::Frame(EditEvent::cancel(id, CancelReason::Escape)));
        }
        self.tool.synced = self.tool.session.frame();
    }

    /// Enter, Esc, arrows and X through [`CropFrame::key`](super::CropFrame::key); O, ⇧O, H, P and I
    /// as the type's doc says ([`CropEvent::Settings`]). Returns `false` for a key the tool leaves
    /// to the app's shortcuts.
    fn key(&mut self, key: &ToolKey, cx: &mut ToolContext<'_>) -> bool {
        // pc: crates/ui-egui/src/tools/crop_ui.rs:743 `keys`; crop_overlay.rs:268 `keys`;
        // crop_shield.rs:126 `keys`; crop_size.rs:273 `keys`.
        if !key.pressed || self.tool.press.is_some() {
            return false;
        }
        let (settings, document) = (self.input.settings, self.input.document);
        let frame = self.tool.session.frame().filter(CropBox::is_finite);
        // Shift counts: ⇧O is not O.
        let plain = key.modifiers.matches_exact(Modifiers::NONE);
        let mut next = settings.clone();
        match key.key {
            Key::I if plain => next.front_image(document),
            Key::O if frame.is_some() && plain => next.cycle_guide(),
            Key::O if frame.is_some() && key.modifiers.matches_exact(Modifiers::SHIFT) => next.cycle_orientation(),
            // Show Cropped Area is taken but inert in Classic Mode, where it acts as on.
            Key::H if frame.is_some() && plain => next.shield.show_cropped_area ^= !next.shield.classic_mode,
            Key::P if frame.is_some() && plain => next.shield.classic_mode = !next.shield.classic_mode,
            _ => return self.frame_key(key, frame, cx),
        }
        if next != *settings {
            self.push(CropEvent::Settings(next));
        }
        true
    }

    /// The shield, frame, guide (by [`GuideShow`](super::GuideShow)) and brackets
    /// ([`CropFrame::paint`](super::CropFrame::paint)), and during a drag the gesture's line and
    /// readout ([`CropFrame::paint_gesture`](super::CropFrame::paint_gesture)).
    fn paint(&self, painter: &Painter, cx: &ToolContext<'_>) {
        // pc: crates/ui-egui/src/tools/crop_straighten.rs:152 `draw`; the frame's look is the
        // library's (`CropFrame::paint`).
        let Some(frame) = self.tool.session.frame().filter(CropBox::is_finite) else { return };
        let style = CropFrame::style(cx.tokens);
        let settings = self.input.settings;
        let dragging = self.tool.dragging();
        let canvas = self.input.canvas_colour.unwrap_or_else(|| cx.tokens.color.surface.canvas.to_egui());
        let guide = settings.guide_show.shows(dragging).then_some((settings.guide, settings.guide_orientation));
        let host = Self::frame(&self.input, self.tool, &frame, Modifiers::NONE, cx).shield(settings.shield.fill(canvas, dragging)).guide(guide);
        // The host draws the focus ring round the picked part.
        host.paint(painter, cx.view, &style, &frame, None);
        if let (Some(press), Some(pointer)) = (self.tool.press, cx.pointer) {
            let labels = self.input.readout.clone().unwrap_or_else(|| CropLabels::of_ctx(&cx.canvas.ctx));
            let readout = self.input.units.map(|(x, y)| CropReadout { labels: &labels, x, y });
            host.paint_gesture(painter, &style, &press, pointer.screen, readout);
        }
    }

    /// [`CropFrame::parts`](super::CropFrame::parts) of the session's frame; `None` without one.
    fn parts(&self, cx: &ToolContext<'_>) -> Option<ToolParts> {
        // new: lead call 33.
        let frame = self.tool.session.frame().filter(CropBox::is_finite)?;
        let style = CropFrame::style(cx.tokens);
        Some(Self::frame(&self.input, self.tool, &frame, Modifiers::NONE, cx).parts(cx.view, &style, &frame, self.tool.picked))
    }

    fn picked(&self) -> Option<u64> {
        // new: the picked `BoxPart` through `transform_box::part_key`.
        self.tool.picked.map(part_key)
    }

    fn pick(&mut self, key: Option<u64>, _cx: &mut ToolContext<'_>) {
        // new: `transform_box::key_part`.
        self.tool.picked = key.and_then(key_part);
    }
}

impl CropToolPass<'_> {
    /// Enter, Esc, an arrow or X through the frame; `frame` is the session's.
    fn frame_key(&mut self, key: &ToolKey, frame: Option<CropBox>, cx: &mut ToolContext<'_>) -> bool {
        let Some(frame) = frame else { return false };
        // Without the canvas's focus the parts group has the keyboard: the arrows move the picked part.
        let picked = self.tool.picked.filter(|_| !cx.canvas.has_focus());
        let mut view = *cx.view;
        let action = Self::frame(&self.input, self.tool, &frame, key.modifiers, cx).key(key, &mut view, picked);
        let (settings, document) = (self.input.settings, self.input.document);
        match action {
            None => false,
            Some(CropKeyAction::Apply) => {
                let view = self.kept_view(cx);
                if let Some(event) = self.tool.apply(settings, document, view) {
                    self.push(event);
                }
                true
            }
            Some(CropKeyAction::Discard) => {
                if let Some(event) = self.tool.discard() {
                    self.push(event);
                }
                true
            }
            Some(CropKeyAction::Set(mut next)) => {
                if key.key == Key::X {
                    // The ratio swaps with the frame, as in Photoshop (W with H in W × H × Resolution).
                    let mut swapped = settings.clone();
                    if swapped.swap(document) && swapped != *settings {
                        if let Some(rect) = swapped.ratio_value(document).and_then(|k| fitted_ratio(next.rect, k)) {
                            next.rect = rect;
                        }
                        self.push(CropEvent::Settings(swapped));
                    }
                }
                self.tool.session.set(next, false, false);
                self.tool.synced = Some(next);
                self.push(CropEvent::Frame(EditEvent::set(next, key.modifiers)));
                if !self.classic() {
                    Self::adopt(view, cx);
                }
                true
            }
        }
    }
}

impl CropTool {
    /// A drag of the frame runs (not a Straighten line).
    fn dragging(&self) -> bool {
        self.press.is_some_and(|p| p.began() && p.kind() != CropPressKind::Straighten)
    }
}

/// Largest centred rectangle of `ratio` (width over height) contained in `rect`; never adds
/// area. `None` for a rect or a ratio that is not a positive finite number.
fn fitted_ratio(rect: Rect, ratio: f64) -> Option<Rect> {
    // pc: crates/ui-egui/src/tools/crop_ui.rs:228 `fitted_ratio`.
    let r = rect.abs();
    let (w, h) = (r.width(), r.height());
    let finite = [r.x0, r.y0, r.x1, r.y1, ratio].iter().all(|v| v.is_finite());
    if !finite || w <= 0.0 || h <= 0.0 || ratio <= 0.0 {
        return None;
    }
    if ((w / h) / ratio - 1.0).abs() < 1e-12 {
        return Some(r);
    }
    let c = r.center();
    let (w, h) = if w / h > ratio { (h * ratio, h) } else { (w, w / ratio) };
    let fitted = Rect::new(c.x - w / 2.0, c.y - h / 2.0, c.x + w / 2.0, c.y + h / 2.0);
    (fitted.x1 > fitted.x0 && fitted.y1 > fitted.y0 && [fitted.x0, fitted.y0, fitted.x1, fitted.y1].iter().all(|v| v.is_finite())).then_some(fitted)
}

/// The handle (`hx`, `hy`) of a corner or edge part, -1 for left and top, 1 for right and bottom.
fn handle_of(part: BoxPart) -> (i8, i8) {
    match part {
        BoxPart::Corner(i) => [(-1, -1), (1, -1), (1, 1), (-1, 1)][usize::from(i % 4)],
        BoxPart::Edge(i) => [(0, -1), (1, 0), (0, 1), (-1, 0)][usize::from(i % 4)],
        _ => (0, 0),
    }
}

/// The screen direction the turn cursor's arc bends towards at screen point `pointer`: the frame
/// corner of the pointer's quadrant, along the diagonal of the frame's own axes.
fn turn_toward(view: &CanvasView, frame: &CropBox, pointer: Pos2) -> egui::Vec2 {
    // pc: crates/ui-egui/src/tools/crop_ui.rs:1020 `turn_cursor_dir`.
    let c = frame.center();
    let turn = |p: Point, about: Point, degrees: f64| Affine::rotate_about(degrees.to_radians(), about) * p;
    let local = turn(view.to_doc(pointer), c, -frame.angle);
    let (sx, sy) = (if local.x < c.x { 1.0 } else { -1.0 }, if local.y < c.y { 1.0 } else { -1.0 });
    let diagonal = std::f64::consts::FRAC_1_SQRT_2;
    let toward = turn(Point::new(sx * diagonal, sy * diagonal), Point::ZERO, frame.angle);
    view.map_vec(Vec2::new(toward.x, toward.y))
}
