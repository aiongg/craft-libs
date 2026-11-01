//! [`CropSession`]: the Crop tool's pending frame and everything around it. The tool always shows
//! a frame (the selection's bounds or the canvas, marked default), keeps the box's own Undo and
//! Redo while a crop is pending, decides what leaving the tool does, and gives the frame back when
//! the app refuses a commit. Plain data: no egui state, no document access.

use kurbo::{Rect, Size};

use super::crop_frame::CropBox;
use craft_ui::canvas::CanvasView;
use craft_ui::dialog::{LocalHistory, Step};

/// The app's identity of a document (its id, never reused while the app runs). A frame belongs
/// to the document it was made for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CropDocKey(pub u64);

/// What the Crop tool needs to know of the active document each pass.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CropDocument {
    pub key: CropDocKey,
    /// The canvas size in document units (pixels).
    pub size: Size,
    /// The document's resolution in px/in: 72 by default.
    pub resolution: f64,
    /// Points per inch for points and picas: 72 (PostScript, the default) or 72.27.
    pub points_per_inch: f64,
    /// The selection's bounds clipped to the canvas, when there is a selection.
    pub selection: Option<Rect>,
}

impl CropDocument {
    /// Document `key` of `size`, at 72 px/in, 72 points per inch, no selection.
    pub fn new(key: CropDocKey, size: Size) -> Self {
        Self { key, size, resolution: 72.0, points_per_inch: 72.0, selection: None }
    }

    /// The resolution in px/in.
    pub fn resolution(mut self, ppi: f64) -> Self {
        self.resolution = ppi;
        self
    }

    /// Points per inch (Preferences › Units & Rulers › Point/Pica Size).
    pub fn points_per_inch(mut self, points_per_inch: f64) -> Self {
        self.points_per_inch = points_per_inch;
        self
    }

    /// The selection's bounds, clipped to the canvas by the tool.
    pub fn selection(mut self, bounds: Option<Rect>) -> Self {
        self.selection = bounds;
        self
    }

    /// The canvas: `(0, 0)` to [`size`](Self::size).
    pub fn canvas(&self) -> Rect {
        self.size.to_rect()
    }
}

/// One state of the box's own history: the frame and whether it is the untouched default.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CropStep {
    pub frame: Option<CropBox>,
    pub default_frame: bool,
}

/// Straighten: armed by the bar's button for one line, or held with ⌘ (Ctrl) for as long as the
/// key is down.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StraightenState {
    armed: bool,
    held: bool,
}

impl StraightenState {
    /// The bar's button: arms Straighten for one line, or disarms it.
    pub fn toggle(&mut self) {
        self.armed = !self.armed;
    }

    /// One line was drawn, Esc was pressed or another tool was picked.
    pub fn disarm(&mut self) {
        self.armed = false;
    }

    /// ⌘ (Ctrl) is held over the canvas this pass: a temporary Straighten.
    pub fn set_held(&mut self, held: bool) {
        self.held = held;
    }

    /// The bar's button is on (its pressed look).
    pub fn armed(&self) -> bool {
        self.armed
    }

    /// The next press draws a Straighten line.
    pub fn on(&self) -> bool {
        self.armed || self.held
    }
}

/// What leaving the Crop tool does to the frame ([`CropSession::leave`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LeaveFrame {
    /// No frame, or the session was already cleared.
    Nothing,
    /// The untouched default frame was dropped.
    Dropped,
    /// A drawn frame: the app crops with it as Enter would, then calls
    /// [`CropSession::clear`] (or [`restore`](CropSession::restore) when the crop is refused, and
    /// then clear, so the next tool does not retry it).
    Apply(CropBox),
    /// A gesture on the frame still runs, or the pen's eraser end stands in for the tool: the frame
    /// stays pending.
    Kept,
}

/// The pending frame, its box history and the view, kept while the app's crop command runs, so a
/// refused commit gives them back ([`CropSession::restore`]).
#[derive(Clone, Debug)]
pub struct CropKept {
    step: CropStep,
    history: Option<LocalHistory<CropStep>>,
    view: Option<CanvasView>,
    frame_for: Option<(CropDocKey, Size)>,
    editing: bool,
}

/// The Crop tool's pending frame. The tool ([`CropTool`](super::CropTool)) owns one and drives
/// it; an app that hosts the frame itself (VectorCraft's Crop Image, LightCraft) may own one too.
///
/// - The default frame: [`ensure_frame`](Self::ensure_frame) frames the selection's bounds, else
///   the canvas, marked default, whenever there is no frame. A drag inside the untouched default
///   frame draws a new one; applying it does nothing. Another document, or the same one resized,
///   gets its own default frame.
/// - Box Undo and Redo: a [`LocalHistory`] of [`CropStep`]s. Each committed gesture, key step,
///   ratio refit and Straighten is one step ([`set`](Self::set) with `busy` false settles it);
///   the app routes Edit › Undo and Redo here while a crop is [`pending`](Self::pending).
///   The view is not in the steps: Undo never moves it.
/// - Leaving the tool ([`leave`](Self::leave)) drops an untouched default frame and applies a
///   drawn one.
#[derive(Clone, Debug, Default)]
pub struct CropSession {
    frame: Option<CropBox>,
    default_frame: bool,
    /// The document and size the frame was made for.
    frame_for: Option<(CropDocKey, Size)>,
    /// Pressed since it was made: the canvas shows what lies past its edges.
    editing: bool,
    history: Option<LocalHistory<CropStep>>,
    straighten: StraightenState,
    /// The frame a ratio-field edit began with (#7752).
    ratio_base: Option<CropBox>,
    /// The latest settled step: what [`revert`](Self::revert) goes back to while a drag runs
    /// ([`LocalHistory`] does not expose it).
    settled: CropStep,
}

impl CropSession {
    fn current(&self) -> CropStep {
        CropStep { frame: self.frame, default_frame: self.default_frame }
    }

    fn adopt(&mut self, step: CropStep) {
        self.frame = step.frame;
        self.default_frame = step.default_frame;
    }

    pub fn new() -> Self {
        Self::default()
    }

    /// The frame, default or drawn.
    pub fn frame(&self) -> Option<CropBox> {
        self.frame
    }

    /// The frame is the untouched default one.
    pub fn is_default(&self) -> bool {
        self.default_frame
    }

    /// A crop is pending: a frame that was drawn, moved, resized, turned or keyed, not the
    /// untouched default. Photoshop greys most menu commands out then; Edit › Undo steps the box.
    pub fn pending(&self) -> bool {
        self.frame.is_some() && !self.default_frame
    }

    /// The frame was pressed since it was made: the app shows the layers' pixels past the canvas
    /// edges under the shield.
    pub fn editing(&self) -> bool {
        self.editing && self.frame.is_some()
    }

    /// Drops the frame, its history and Straighten when they belong to another document than
    /// `key` (as Photoshop: switching documents discards the pending crop). Returns true when a
    /// drawn frame was dropped, so the tool reports [`CropEvent::Discard`](super::CropEvent).
    pub fn for_document(&mut self, key: CropDocKey) -> bool {
        // pc: crates/ui-egui/src/tools/crop_ui.rs:590 `cancel_stale`.
        let stale = self.frame_for.is_some_and(|(held, _)| held != key);
        if !stale {
            return false;
        }
        let drawn = self.pending();
        self.clear();
        drawn
    }

    /// Makes the default frame when there is none and no gesture runs (`dragging` false): the
    /// selection's bounds clipped to the canvas when they are not empty, else the canvas; an empty
    /// canvas gets none. A default frame made for another size is replaced. A new default frame
    /// starts a new box history, upright. Returns true when it made one.
    pub fn ensure_frame(&mut self, document: &CropDocument, dragging: bool) -> bool {
        // pc: crates/ui-egui/src/tools/crop_ui.rs:345 `ensure_frame`, :350 `frame` (the Crop-tool
        // half; the leave half is `leave`).
        let made_for = Some((document.key, document.size));
        if self.default_frame && self.frame_for != made_for && !dragging {
            self.frame = None;
            self.default_frame = false;
        }
        if self.frame.is_some() || dragging {
            return false;
        }
        let canvas = document.canvas();
        let usable = |r: &Rect| r.width() > 0.0 && r.height() > 0.0;
        if !usable(&canvas) {
            return false;
        }
        let rect = document.selection.map(|s| s.intersect(canvas)).filter(usable).unwrap_or(canvas);
        self.frame = Some(CropBox::upright(rect));
        self.default_frame = true;
        self.frame_for = made_for;
        self.editing = false;
        self.settled = self.current();
        self.history = Some(LocalHistory::new(&self.settled));
        true
    }

    /// The first press on the frame since it was made: [`editing`](Self::editing) from now on.
    /// Returns true when that is new.
    pub fn press(&mut self) -> bool {
        // pc: crates/ui-egui/src/tools/crop_ui.rs:608 `press`.
        let new = self.frame.is_some() && !self.editing;
        if new {
            self.editing = true;
        }
        new
    }

    /// A gesture, key or refit set the frame: it is no longer the default (`keep_default` true for
    /// a gesture that changed nothing visible, such as a click outside the default frame without
    /// a turn). With `busy` false the change since the last step becomes one box step; a drag sets
    /// `busy` until its release.
    pub fn set(&mut self, frame: CropBox, keep_default: bool, busy: bool) {
        // pc: crates/ui-egui/src/tools/crop_ui.rs:862 `release` (`changed`), :879 `push_press_step`,
        // :675 `keyed`; dialog/history.rs `LocalHistory::settle`.
        if self.history.is_none() {
            self.history = Some(LocalHistory::new(&self.settled));
        }
        self.frame = Some(frame);
        if !keep_default {
            self.default_frame = false;
        }
        if !self.default_frame {
            self.editing = true;
        }
        let now = self.current();
        if let Some(history) = &mut self.history {
            history.settle(&now, busy);
        }
        if !busy {
            self.settled = now;
        }
    }

    /// A drag ended without a frame (too small, Esc): the frame and its default mark go back to
    /// the last step.
    pub fn revert(&mut self) {
        // pc: crates/ui-egui/src/tools/crop_ui.rs:889 `restore_frame`.
        let back = self.settled;
        self.adopt(back);
    }

    /// Edit › Undo while a crop is pending: the frame one box step back, or `None` when there is
    /// nothing to undo.
    pub fn undo(&mut self) -> Option<CropBox> {
        // pc: crates/ui-egui/src/tools/crop_ui.rs:510 `step` (the view half is the tool's).
        self.step(Step::Undo)
    }

    /// Edit › Redo while a crop is pending: as [`undo`](Self::undo), forward.
    pub fn redo(&mut self) -> Option<CropBox> {
        // pc: crates/ui-egui/src/tools/crop_ui.rs:510 `step`.
        self.step(Step::Redo)
    }

    fn step(&mut self, step: Step) -> Option<CropBox> {
        let mut now = self.current();
        if !self.history.as_mut()?.step(&mut now, step) {
            return None;
        }
        self.adopt(now);
        self.settled = now;
        now.frame
    }

    /// Whether Edit › Undo and Redo have a box step to go to (their menu items' enabled state).
    pub fn can_undo(&self) -> bool {
        // pc: crates/ui-egui/src/tools/crop_ui.rs:483 `is_enabled`.
        self.history.as_ref().is_some_and(LocalHistory::can_undo)
    }

    pub fn can_redo(&self) -> bool {
        // pc: crates/ui-egui/src/tools/crop_ui.rs:483 `is_enabled`.
        self.history.as_ref().is_some_and(LocalHistory::can_redo)
    }

    /// Another tool was picked. `keeps_pending`: a gesture on the frame still runs, or the pen's
    /// eraser end stands in for the Crop tool; the held temporary tools (Space, ⌘) never count as
    /// leaving and never call this. Disarms Straighten.
    pub fn leave(&mut self, keeps_pending: bool) -> LeaveFrame {
        // pc: crates/ui-egui/src/tools/crop_ui.rs:350 `frame` (the leave half), :401 `keeps_pending`,
        // :408 `commit_on_leave`.
        self.straighten.disarm();
        let Some(frame) = self.frame else { return LeaveFrame::Nothing };
        if keeps_pending {
            return LeaveFrame::Kept;
        }
        self.editing = false;
        if self.default_frame {
            self.clear();
            return LeaveFrame::Dropped;
        }
        LeaveFrame::Apply(frame)
    }

    /// Drops the frame, its history, the default mark, the editing state and Straighten: Esc, ⊘,
    /// an applied crop. With the tool still picked, the next
    /// [`ensure_frame`](Self::ensure_frame) makes a new default frame.
    pub fn clear(&mut self) {
        // pc: crates/ui-egui/src/tools/crop_ui.rs:424 `cancel`, :437 `clear_history`.
        *self = Self::default();
    }

    /// Keeps the frame, the box history and `view` (an Image-mode host's turned view) before the
    /// app runs its crop command.
    pub fn keep(&self, view: Option<CanvasView>) -> CropKept {
        // pc: crates/ui-egui/src/tools/crop_ui.rs:464 `keep`.
        CropKept { step: self.current(), history: self.history.clone(), view, frame_for: self.frame_for, editing: self.editing }
    }

    /// The crop command refused the frame: the frame, its default mark and its box history come
    /// back, and the view kept with them for the host to adopt.
    pub fn restore(&mut self, kept: CropKept) -> Option<CanvasView> {
        // pc: crates/ui-egui/src/tools/crop_ui.rs:469 `restore`.
        self.adopt(kept.step);
        self.settled = kept.step;
        self.history = kept.history;
        self.frame_for = kept.frame_for;
        self.editing = kept.editing;
        kept.view
    }

    /// The frame a ratio-field edit fits from: the frame when the edit began (`editing` true on
    /// the first change of the run), kept until the run ends (`editing` false), so 1 : 3 on the way
    /// to 16 : 3 does not shrink the frame first (#7752). `None` without a frame.
    pub fn ratio_base(&mut self, editing: bool) -> Option<CropBox> {
        // pc: crates/ui-egui/src/panels/options_bar.rs:489-500 (the `crop-ratio-edit-base` memory).
        if editing {
            if self.ratio_base.is_none() {
                self.ratio_base = self.frame;
            }
            self.ratio_base
        } else {
            self.ratio_base = None;
            self.frame
        }
    }

    /// Straighten's state.
    pub fn straighten(&self) -> &StraightenState {
        &self.straighten
    }

    pub fn straighten_mut(&mut self) -> &mut StraightenState {
        &mut self.straighten
    }
}
