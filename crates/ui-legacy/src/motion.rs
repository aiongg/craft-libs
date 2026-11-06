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

use craft_theme::{Easing, Motion};
use egui::{Context, Pos2, Rect, pos2};

/// A resting caret is on for this long, then off for as long: 530 ms (Word's, and the Windows
/// default).
pub const CARET_BLINK_SECS: f64 = 0.53;
/// Moves shorter than this (points) don't glide.
pub const MIN_MOVE: f32 = 0.5;
/// How far outside the view a glide may start or end (points); a move from or to further away
/// is drawn in place.
pub const OFFSCREEN_SLACK: f32 = 48.0;

/// A motion token's duration in seconds (0 with reduced motion).
pub fn motion_secs(motion: Motion) -> f64 {
    f64::from(motion.millis) / 1000.0
}

/// Whether glides may run in `ctx`: the theme's motion isn't reduced
/// (`tokens.motion.reduced`, the appearance's reduced motion) and egui's `animation_time` is
/// above zero (the headless harness sets it to zero, so screenshots never catch a caret in
/// flight).
// new: the suite-wide switch; WordCraft's `look` and GridCraft's `animate` read their own settings.
pub fn glides_allowed(ctx: &Context) -> bool {
    !crate::theme::current(ctx).motion.reduced && ctx.global_style().animation_time > 0.0
}

/// The glide curve of a motion token at progress `s`, clamped to 0–1 (a non-finite `s` is 1).
// wordcraft: crates/ui-egui/src/caret_glide.rs:282 ease (Word's curve; the token's `easing` stands in)
pub fn ease(easing: Easing, s: f32) -> f32 {
    easing.at(if s.is_finite() { s } else { 1.0 })
}

/// A resting caret's opacity `since` seconds after its blink restarted: 1 for the first
/// [`CARET_BLINK_SECS`], then off and on in turns of that length, each change faded over `fade`
/// seconds (no fade at 0). A non-finite `since` is 1.
// wordcraft: crates/ui-egui/src/caret_glide.rs:290 blink
pub fn blink(since: f64, fade: f64) -> f32 {
    if !since.is_finite() || since < CARET_BLINK_SECS {
        return 1.0;
    }
    let phase = since % (2.0 * CARET_BLINK_SECS);
    let (into, turning_on) = if phase < CARET_BLINK_SECS { (phase, true) } else { (phase - CARET_BLINK_SECS, false) };
    let done = if fade > 0.0 { (into / fade).clamp(0.0, 1.0) as f32 } else { 1.0 };
    if turning_on { done } else { 1.0 - done }
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
    /// The shape of the caret bar.
    pub bar: CaretBar,
    /// 0 (not drawn) to 1.
    pub opacity: f32,
    /// It is gliding or fading in: the host asks for a repaint.
    pub moving: bool,
}

/// A caret's motion between frames, after WordCraft's caret glide (Word for Windows' caret).
///
/// - A key or typing moves the caret: it glides from where it was heading to its new place in
///   the `motion.caret` token's time on its easing; another move before it arrives starts from the old glide's
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
    easing: Easing,
    target: Option<CaretBar>,
    opacity: f32,
    /// From, to, and when it set off.
    glide: Option<(CaretBar, CaretBar, f64)>,
    fade: Option<(f32, f64)>,
    cause: u64,
    view: u64,
    frame_nr: u64,
}

impl CaretGlide {
    /// A caret whose glides and fades take the token's time and glide on its easing (a time of 0:
    /// it jumps and blinks hard; at most 10 s, for looking at it slowed down). Hosts pass
    /// `tokens.motion.caret`.
    // wordcraft: crates/ui-egui/src/caret_glide.rs:140 with_secs
    pub fn with_motion(motion: Motion) -> Self {
        let secs = motion_secs(motion).clamp(0.0, 10.0);
        Self { secs, easing: motion.easing, target: None, opacity: 0.0, glide: None, fade: None, cause: 0, view: 0, frame_nr: 0 }
    }

    /// Advances to this pass and says how to draw the caret. The host passes
    /// [`glides_allowed`] through the token's time, building the glide with a `millis` of 0 when it is
    /// false. Call it after the pass's events are applied (see the type's doc).
    // wordcraft: crates/ui-egui/src/caret_glide.rs:158 frame
    pub fn frame(&mut self, f: CaretFrame) -> CaretLook {
        let continuous = f.frame_nr.wrapping_sub(self.frame_nr) <= 1;
        let was = self.target.filter(|_| continuous);
        let was_opacity = if was.is_some() { self.opacity } else { 0.0 };
        let edited = f.cause != self.cause;
        let same_view = f.view == self.view;
        self.cause = f.cause;
        self.view = f.view;
        self.frame_nr = f.frame_nr;
        if !(f.shown && is_finite(f.target) && f.now.is_finite()) {
            self.reset();
            return CaretLook { bar: f.target, opacity: 0.0, moving: false };
        }
        if self.secs <= 0.0 {
            self.glide = None;
            self.fade = None;
            let opacity = if blink(f.since_blink, 0.0) > 0.5 { 1.0 } else { 0.0 };
            return self.draw(f.target, f.target, opacity);
        }
        if !same_view {
            self.glide = None;
        }
        // Where the caret was heading: its place last frame, or the end of the glide under way.
        let heading = self.glide.map(|g| g.1).or(was);
        let moved = heading.is_none_or(|h| distance(h, f.target) >= MIN_MOVE);
        if edited && moved && (f.keys || f.pointer) {
            // A key or a click turns the caret on, from however visible it was.
            self.fade = Some((was_opacity, f.now));
            self.glide = match heading {
                // Keys glide, from where the caret was heading (not from where it has got to).
                Some(from) if f.keys && !f.pointer && same_view => {
                    let seen = f.viewport.expand(OFFSCREEN_SLACK);
                    let on_screen = |b: CaretBar| seen.intersects(Rect::from_two_pos(b[0], b[1]));
                    (on_screen(from) && on_screen(f.target)).then_some((from, f.target, f.now))
                }
                _ => None,
            };
        } else if edited && moved {
            // The host moved it: it is simply there.
            self.glide = None;
            self.fade = None;
        }
        // The target shifted under a glide (pixel snapping while scrolling): carry the glide along.
        if let Some(g) = &mut self.glide
            && g.1 != f.target
        {
            let d = f.target[0] - g.1[0];
            g.0 = [g.0[0] + d, g.0[1] + d];
            g.1 = f.target;
        }
        let bar = match self.glide {
            Some((from, to, started)) => match progress(f.now, started, self.secs) {
                Some(s) => lerp(from, to, ease(self.easing, s)),
                None => {
                    self.glide = None;
                    f.target
                }
            },
            None => f.target,
        };
        let opacity = match self.fade {
            Some((from, started)) => match progress(f.now, started, self.secs) {
                Some(s) => from + (1.0 - from) * s,
                None => {
                    self.fade = None;
                    1.0
                }
            },
            // A gliding caret stays lit; a resting one blinks.
            None if self.glide.is_some() => 1.0,
            None => blink(f.since_blink, self.secs),
        };
        self.draw(f.target, bar, opacity)
    }

    fn draw(&mut self, target: CaretBar, bar: CaretBar, opacity: f32) -> CaretLook {
        let opacity = if opacity.is_finite() { opacity.clamp(0.0, 1.0) } else { 1.0 };
        self.target = Some(target);
        self.opacity = opacity;
        CaretLook { bar, opacity, moving: self.glide.is_some() || self.fade.is_some() }
    }

    /// The caret is on its way somewhere.
    pub fn is_gliding(&self) -> bool {
        self.glide.is_some()
    }

    /// Forgets where the caret was drawn: the next [`frame`](Self::frame) puts it at its target, with no glide.
    pub fn reset(&mut self) {
        // What the host showed (the cause, the view, the frame) stays: the caret reappearing is
        // not an edit.
        self.target = None;
        self.opacity = 0.0;
        self.glide = None;
        self.fade = None;
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
///   in the `motion.outline` token's time and easing, each edge on its own, so extending a selection grows it edge by
///   edge and a held arrow key reads as one motion.
/// - The same cells moved on screen (scrolling, zoom, a resized column): the outline stays on
///   its cells. The glide is kept as each edge's distance from its target, decaying to zero, so
///   the outline is always "the cells' rect plus what is left of the offset".
/// - Another place, an outline not drawn the frame before, a move from or to more than
///   [`OFFSCREEN_SLACK`] off the view, `animate` false: it appears in place.
#[derive(Clone, Debug)]
pub struct OutlineGlide {
    secs: f64,
    easing: Easing,
    place: u64,
    selection: u64,
    drawn: Option<Rect>,
    offset: [f32; 4],
    started: f64,
    gliding: bool,
    frame_nr: u64,
}

impl OutlineGlide {
    /// An outline whose glides take the token's time on its easing (a time of 0: it jumps; at
    /// most 10 s). Hosts pass `tokens.motion.outline`.
    pub fn with_motion(motion: Motion) -> Self {
        let secs = motion_secs(motion).clamp(0.0, 10.0);
        Self { secs, easing: motion.easing, place: 0, selection: 0, drawn: None, offset: [0.0; 4], started: 0.0, gliding: false, frame_nr: 0 }
    }

    /// Advances to this frame; returns where to draw the outline (screen points).
    // gridcraft: crates/ui-egui/src/selection_glide.rs:77 frame
    pub fn frame(&mut self, f: OutlineFrame) -> Rect {
        let fresh = f.frame_nr.wrapping_sub(self.frame_nr) <= 1;
        self.frame_nr = f.frame_nr;
        if self.place != f.place || self.selection != f.selection {
            // A new destination: set off from where the outline is drawn now (mid-glide or at rest).
            self.offset = match self.drawn {
                Some(drawn) if self.place == f.place && fresh => offset_between(drawn, f.target, f.view),
                _ => [0.0; 4],
            };
            self.started = f.now;
            self.place = f.place;
            self.selection = f.selection;
        }
        if !f.animate {
            self.offset = [0.0; 4];
        }
        // Arrived when the time is up (or the clock went backwards).
        let progress = if self.secs > 0.0 { (f.now - self.started) / self.secs } else { 1.0 };
        self.gliding = self.offset != [0.0; 4] && (0.0..1.0).contains(&progress);
        let left = if self.gliding { 1.0 - ease(self.easing, progress as f32) } else { 0.0 };
        let [l, t, r, b] = self.offset.map(|o| o * left);
        let drawn = Rect::from_min_max(pos2(f.target.min.x + l, f.target.min.y + t), pos2(f.target.max.x + r, f.target.max.y + b));
        self.drawn = Some(drawn);
        drawn
    }

    /// The outline is on its way: keep repainting.
    // gridcraft: crates/ui-egui/src/selection_glide.rs:101 is_gliding
    pub fn is_gliding(&self) -> bool {
        self.gliding
    }
}

/// How far along an animation that started at `started` and lasts `secs` is at `now`: 0 to 1, or
/// `None` once it is over (or the clock went backwards).
fn progress(now: f64, started: f64, secs: f64) -> Option<f32> {
    let s = (now - started) / secs;
    (0.0..1.0).contains(&s).then_some(s as f32)
}

fn lerp(a: CaretBar, b: CaretBar, t: f32) -> CaretBar {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
}

/// How far the bar's farthest end moves.
fn distance(a: CaretBar, b: CaretBar) -> f32 {
    (b[0] - a[0]).length().max((b[1] - a[1]).length())
}

fn is_finite(b: CaretBar) -> bool {
    b[0].x.is_finite() && b[0].y.is_finite() && b[1].x.is_finite() && b[1].y.is_finite()
}

/// Each edge's distance from `to` back to `from`. No glide (all zero) when either end is off the
/// view or the numbers are not sane. An edge sets off at most one view away, so the far edge of a
/// whole column or row slides in from the border of the view instead of popping in at the end.
fn offset_between(from: Rect, to: Rect, view: Rect) -> [f32; 4] {
    let seen = view.expand(OFFSCREEN_SLACK);
    if !(seen.intersects(from) && seen.intersects(to)) {
        return [0.0; 4];
    }
    let limit = |v: f32| if v.is_finite() { v.abs() } else { 0.0 };
    let (lx, ly) = (limit(view.width()), limit(view.height()));
    let offset = [
        (from.min.x - to.min.x).clamp(-lx, lx),
        (from.min.y - to.min.y).clamp(-ly, ly),
        (from.max.x - to.max.x).clamp(-lx, lx),
        (from.max.y - to.max.y).clamp(-ly, ly),
    ];
    if offset.iter().all(|o| o.is_finite()) { offset } else { [0.0; 4] }
}
