#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! Motion of text carets and selection outlines: [`CaretGlide`] slides a caret to its new place
//! and fades its blink, [`OutlineGlide`] slides a selection outline to its new cells. Both are
//! time-based, so they look the same at any frame rate, and both stand still when the appearance
//! asks for reduced motion or egui's animations are off ([`glides_allowed`]).
//!
//! Only painting is delayed: hit testing and editing use the real place at once.
//!
//! ```ignore
//! // Once a pass, after the editor applied this pass's events (a canvas tool's
//! // `CanvasTool::each_pass`, or after `TextEditor::finish_pass`), for the document's caret:
//! let look = caret.frame(CaretFrame { target, cause: hash((doc.revision, sel)), view: hash(zoom), shown, viewport, now, frame_nr, keys, pointer, since_blink });
//! paint_caret(look.bar, look.opacity);
//! if look.moving { ctx.request_repaint() }
//! ```

use egui::{Context, Pos2, Rect};

/// How long a caret's glide, and each fade of its blink, takes: 100 ms (WordCraft, measured from
/// Word for Windows 16.0 at 144 Hz).
pub const CARET_GLIDE_SECS: f64 = 0.10;
/// A resting caret is on for this long, then off for as long: 530 ms (Word's, and the Windows
/// default).
pub const CARET_BLINK_SECS: f64 = 0.53;
/// How long an outline's glide takes: 120 ms (GridCraft's estimate of Excel's cell cursor).
pub const OUTLINE_GLIDE_SECS: f64 = 0.12;
/// Moves shorter than this (points) don't glide.
pub const MIN_MOVE: f32 = 0.5;
/// How far outside the view a glide may start or end (points); a move from or to further away
/// is drawn in place.
pub const OFFSCREEN_SLACK: f32 = 48.0;

/// Whether glides may run in `ctx`: the theme's motion isn't reduced
/// (`tokens.motion.reduced`, the appearance's reduced motion) and egui's `animation_time` is
/// above zero (the headless harness sets it to zero, so screenshots never catch a caret in
/// flight).
// new: the suite-wide switch; WordCraft's `look` and GridCraft's `animate` read their own settings.
pub fn glides_allowed(ctx: &Context) -> bool {
    todo!()
}

/// Word's glide curve, `cubic-bezier(0, 0, 0, 1)`, in closed form: `3 s^(2/3) − 2 s` for `s`
/// clamped to 0–1 (a non-finite `s` is 1).
// wordcraft: crates/ui-egui/src/caret_glide.rs:282 ease
pub fn ease(s: f32) -> f32 {
    todo!()
}

/// A resting caret's opacity `since` seconds after its blink restarted: 1 for the first
/// [`CARET_BLINK_SECS`], then off and on in turns of that length, each change faded over `fade`
/// seconds (no fade at 0). A non-finite `since` is 1.
// wordcraft: crates/ui-egui/src/caret_glide.rs:290 blink
pub fn blink(since: f64, fade: f64) -> f32 {
    todo!()
}

/// A caret's bar: its two ends, in the space the host gives them in (WordCraft: content space,
/// so scrolling under a moving caret doesn't bend its path).
pub type CaretBar = [Pos2; 2];

/// What the host knows about its caret in one frame.
#[derive(Clone, Copy, Debug)]
pub struct CaretFrame {
    /// Where the caret belongs.
    pub target: CaretBar,
    /// A fingerprint of what put it there (the document's revision and the selection): the caret
    /// glides only when it changes.
    pub cause: u64,
    /// A fingerprint of how the document is shown (zoom, page arrangement, which document): a
    /// change ends any glide.
    pub view: u64,
    /// The caret is drawn this frame (focused, nothing selected).
    pub shown: bool,
    /// The part of the target's space on screen.
    pub viewport: Rect,
    /// egui's clock, seconds.
    pub now: f64,
    /// egui's frame counter: a caret that skipped a frame doesn't glide from its stale place.
    pub frame_nr: u64,
    /// A key was pressed or text arrived this frame.
    pub keys: bool,
    /// A mouse button is down, or was pressed or released, this frame.
    pub pointer: bool,
    /// Seconds since the blink restarted (the last edit or click).
    pub since_blink: f64,
}

/// How to draw the caret this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CaretLook {
    pub bar: CaretBar,
    /// 0 (not drawn) to 1.
    pub opacity: f32,
    /// It is gliding or fading in: the host asks for a repaint.
    pub moving: bool,
}

/// A caret's motion between frames, after WordCraft's caret glide (Word for Windows' caret).
///
/// - A key or typing moves the caret: it glides from where it was heading to its new place in
///   [`CARET_GLIDE_SECS`] on [`ease`]; another move before it arrives starts from the old glide's
///   end. The time is the same for one character and a whole line.
/// - A click moves it: no glide. A move by the host (a script, the control channel): no glide and
///   no fade.
/// - A key or a click turns it on, fading from however visible it was; a resting caret blinks
///   with fades ([`blink`]).
/// - Never glides: a view change, a caret not drawn the frame before, a move shorter than
///   [`MIN_MOVE`], a move starting or ending more than [`OFFSCREEN_SLACK`] off the viewport.
/// - The target shifting under a glide (pixel snapping while scrolling) carries the glide along.
/// - With [`glides_allowed`] false (or `secs` 0) it jumps and blinks on and off.
///
/// The host calls [`frame`](Self::frame) once a pass after its editor has handled the pass's
/// events, with that pass's `keys` and `pointer` (read from the pass's input) and the caret's
/// new target, so the move and its cause arrive together; nothing is carried to the next pass.
/// WordCraft read the keys a frame late because its canvas handled them at the end of a frame;
/// with `CanvasTool::each_pass` and `TextEditor::finish_pass` the editor's events are applied
/// before the caret is placed.
#[derive(Clone, Debug)]
pub struct CaretGlide {
    secs: f64,
    target: Option<CaretBar>,
    drawn: Option<CaretBar>,
    opacity: f32,
    glide: Option<(CaretBar, CaretBar, f64)>,
    fade: Option<(f32, f64)>,
    cause: u64,
    view: u64,
    frame_nr: u64,
}

impl Default for CaretGlide {
    fn default() -> Self {
        Self::with_secs(CARET_GLIDE_SECS)
    }
}

impl CaretGlide {
    /// A caret whose glides and fades take `secs` (0 or less: it jumps and blinks hard; a
    /// non-finite value is the default; at most 10 s, for looking at it slowed down).
    // wordcraft: crates/ui-egui/src/caret_glide.rs:140 with_secs
    pub fn with_secs(secs: f64) -> Self {
        let secs = if secs.is_finite() { secs.clamp(0.0, 10.0) } else { CARET_GLIDE_SECS };
        Self { secs, target: None, drawn: None, opacity: 0.0, glide: None, fade: None, cause: 0, view: 0, frame_nr: 0 }
    }

    /// Advances to this pass and says how to draw the caret. The host passes
    /// [`glides_allowed`] through `secs` by building the glide with `with_secs(0.0)` when it is
    /// false. Call it after the pass's events are applied (see the type's doc).
    // wordcraft: crates/ui-egui/src/caret_glide.rs:158 frame
    pub fn frame(&mut self, frame: CaretFrame) -> CaretLook {
        todo!()
    }

    /// The caret is on its way somewhere.
    pub fn is_gliding(&self) -> bool {
        self.glide.is_some()
    }

    /// Forgets the caret: the next [`frame`](Self::frame) puts it at its target, with no glide.
    pub fn reset(&mut self) {
        *self = Self::with_secs(self.secs);
    }
}

/// What the host knows about its selection outline in one frame.
#[derive(Clone, Copy, Debug)]
pub struct OutlineFrame {
    /// A fingerprint of the place shown (workbook and sheet, document and page): the outline never
    /// glides between two.
    pub place: u64,
    /// A fingerprint of the selection (its range): the outline glides when it changes.
    pub selection: u64,
    /// Where the selection's cells are on screen.
    pub target: Rect,
    /// The view on screen.
    pub view: Rect,
    /// egui's clock, seconds.
    pub now: f64,
    /// egui's frame counter.
    pub frame_nr: u64,
    /// Glides may run this frame ([`glides_allowed`], and the host's own switch); otherwise the
    /// outline sits on its cells.
    pub animate: bool,
}

/// A selection outline's motion between frames, after GridCraft's selection glide (Excel's cell
/// cursor).
///
/// - The selection changed (keys, a click, a drag, any command): the outline, and whatever the
///   host draws with it (fill, fill handle), glides from where it is drawn now to the new cells
///   in [`OUTLINE_GLIDE_SECS`], each edge on its own, so extending a selection grows it edge by
///   edge and a held arrow key reads as one motion.
/// - The same cells moved on screen (scrolling, zoom, a resized column): the outline stays on
///   its cells. The glide is kept as each edge's distance from its target, decaying to zero, so
///   the outline is always "the cells' rect plus what is left of the offset".
/// - Another place, an outline not drawn the frame before, a move from or to more than
///   [`OFFSCREEN_SLACK`] off the view, `animate` false: it appears in place.
#[derive(Clone, Debug, Default)]
pub struct OutlineGlide {
    place: u64,
    selection: u64,
    drawn: Option<Rect>,
    offset: [f32; 4],
    started: f64,
    frame_nr: u64,
}

impl OutlineGlide {
    /// Advances to this frame; returns where to draw the outline (screen points).
    // gridcraft: crates/ui-egui/src/selection_glide.rs:77 frame
    pub fn frame(&mut self, frame: OutlineFrame) -> Rect {
        todo!()
    }

    /// The outline is on its way: keep repainting.
    // gridcraft: crates/ui-egui/src/selection_glide.rs:101 is_gliding
    pub fn is_gliding(&self) -> bool {
        todo!()
    }
}
