//! [`PenDraft`]: the path being drawn with the Pen and the Curvature Pen, as PhotoCraft's tools
//! draw it; [`PathMode`]: which path overlay a tool and the held keys run; the pen's Path Options
//! ([`PathOptionsSettings`], [`PathOptionsPopover`]).

mod auto;
pub(crate) mod bezier;
mod curvature_mode;
#[cfg(test)]
mod curvature_tests;
mod mode;
#[cfg(test)]
mod mode_tests;
mod options;
pub(crate) mod story;

use egui::{AsIdSalt, Color32, Context, Id, IdSalt, Key, Modifiers, Response, Shape, Stroke, Ui, vec2};
use kurbo::Point;

use auto::AutoEdit;
pub use bezier::{
    AnchorsShown, BezierHit, BezierNames, BezierOverlay, BezierResponse, BezierStyle, BezierTool, EditPath, EditSubpath, HandleSide, HandlesShown,
    HitMark, Knot, KnotRef, MAX_ANCHORS, PathEdit, PathSelection, SegmentMark,
};
use bezier::{adjusted, constrain, outline};
use craft_ui::behavior::nudge;
use craft_ui::canvas::claim::PrimaryPress;
use craft_ui::canvas::{CanvasView, Claim, Part, PartFocus, RulerUnits, hit};
use craft_ui::edit::order::Turn;
use craft_ui::edit::{CancelReason, EditEvent, EditPolicy, Gesture};
use craft_ui::localized::words::format_in;
use craft_ui::localized::{Labels, Word, Words};
use craft_ui::primitives::{Handle, HandleShape, HandleStyle};
use craft_ui::theme::{Tokens, TokensOf, UiTokens};
pub use mode::{PathMode, PathTool, PenMode};
pub use options::{PATH_OPTIONS_COLOR_KEY, PathOptionsLabels, PathOptionsPopover, PathOptionsPopoverResponse, PathOptionsSettings};

/// The AccessKit names of the path being drawn. [`PenNames::english`] is English, and a host passes
/// its own translated text or [`PenNames::of`] for the words installed in the context.
pub struct PenNames<'a> {
    /// The group of anchors ("Path being drawn").
    pub group: String,
    /// The name of anchor `i` (from 1) of the draft's `n`: "Anchor 3 of 5".
    pub anchor: Box<dyn Fn(usize, usize) -> String + 'a>,
}

/// Group, then the anchor's name with `{i}` and `{n}`.
const WORDS: [Word; 2] = [Word::keyed("pen.group", "Path being drawn"), Word::keyed("pen.anchor", "Anchor {i} of {n}")];

impl Labels for PenNames<'static> {
    const WORDS: &'static [Word] = &WORDS;

    fn english() -> Self {
        Self::localized_in(Self::new("", |_, _| String::new()), &Words::english())
    }

    fn localized_in(self, words: &Words) -> Self {
        let name = |words: Option<&Words>, i: usize, n: usize| format_in(words, WORDS[1], &[("i", &i.to_string()), ("n", &n.to_string())]);
        let group = words.get(WORDS[0]);
        match words.translator() {
            Some(words) => Self::new(group, move |i, n| name(Some(&words), i, n)),
            None => Self::new(group, move |i, n| name(None, i, n)),
        }
    }
}

impl<'a> PenNames<'a> {
    /// Names from the group text and `anchor(i, n)`, the name of anchor `i` of `n`.
    pub fn new(group: impl Into<String>, anchor: impl Fn(usize, usize) -> String + 'a) -> Self {
        Self { group: group.into(), anchor: Box::new(anchor) }
    }
}

/// The pen draft's look, from [`PenDraft::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct PenStyle {
    /// The path so far: its width, in `overlay.line`.
    pub draft_line: f32,
    /// The path so far: its colour.
    pub color: Color32,
    /// The rubber band's width: dashed, in `overlay.line`.
    pub rubber_width: f32,
    /// Dash length of the rubber band.
    pub rubber_dash: f32,
    /// Gap length of the rubber band.
    pub rubber_gap: f32,
    /// An anchor, as [`BezierOverlay`]'s.
    pub anchor: HandleStyle,
    /// The last anchor, filled.
    pub selected_anchor: HandleStyle,
    /// A direction handle's end and its line's width.
    pub control: HandleStyle,
    /// Width of the line from an anchor to its direction handle.
    pub control_line: f32,
    /// The first anchor while a press there closes the path: filled, `close_anchor` wide.
    pub close_anchor: HandleStyle,
    /// The close cue's ring beside the pointer: its radius.
    pub close_ring: f32,
    /// Offset of the close cue's ring from the pointer, across and down.
    pub close_offset: f32,
    /// Distance within which a press takes an anchor, a handle or an open path's end.
    pub hit: f32,
    /// Farthest a press may move and still be a click.
    pub click_slop: f32,
    /// Largest distance between a curve and the lines it is drawn with.
    pub flatten: f32,
}

/// What a pass of [`PenDraft::show`] reports.
#[derive(Clone, Debug, PartialEq)]
#[must_use = "keep the draft the events carry, end it when finished and continue the resumed path"]
pub struct PenResponse {
    /// The overlay's id.
    pub id: Id,
    /// The draft's edits, each value the whole draft: Begin at its first change (a draft the host
    /// hands in, a resumed path's, begins at once), Update on each change, Commit when the pen
    /// ends a changed draft (`closed` is set on a close). A draft that ends with fewer than two
    /// anchors (one left at Enter or Esc, the only one removed) is no path: Cancel while the host
    /// holds none of it, else a Commit of the empty draft, which removes the path the host holds
    /// (a resumed path, or the anchors kept when the host ended the gesture). A draft begun and
    /// ended in one frame is a Set. After the host ends the gesture for another control's change,
    /// the draft's next change begins a new gesture, and a draft ended with no change since
    /// reports nothing: [`finished`](Self::finished) says when the draft is over.
    pub event: Option<EditEvent<EditSubpath>>,
    /// The pen ended the draft in this pass (Enter, Esc, closing it, removing its only anchor):
    /// the host stops handing it in. It comes with the draft's Commit or Cancel, or alone when
    /// the host already holds every anchor (it ended the draft's gesture for another control's
    /// change and no anchor changed since).
    pub finished: bool,
    /// A press on the end point of an open path while no draft is open: the host makes that
    /// path's subpath the draft with the knot of that end last (reversing it for its first knot).
    pub resumed: Option<(u64, KnotRef)>,
    /// The overlay owns the current press: the host's tool leaves it alone.
    pub claimed: bool,
    /// A press at the pointer would close the path (over the first anchor of a draft that can
    /// close: 2 or more anchors with the Pen, 3 or more in Curvature mode): the host shows its
    /// close-path cursor (PhotoCraft `pen.rs:27-34`).
    pub closes: bool,
}

/// The looks of one Pen pass.
struct Looks {
    pen: PenStyle,
    bezier: BezierStyle,
}

/// What a press drags.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    /// A new last anchor: a drag pulls out its handles, symmetric.
    Pull,
    /// A press on the first anchor: the path closes on release; a drag pulls out that anchor's
    /// handles.
    Close,
    /// A press on the last anchor, whose out handle it retracts: a drag sets that handle alone.
    Retract,
    /// ⌥ on one of the last anchor's handles: a drag moves it alone.
    Grab(HandleSide),
}

/// A press the overlay owns.
#[derive(Clone, Debug)]
struct Drag {
    press: PrimaryPress,
    kind: Kind,
    /// Past the click slop.
    moved: bool,
    /// The in handle as the last pass left it: ⌥ keeps it there while the out handle follows.
    in_ctrl: Point,
    /// The press waits for another control's typing run to end ([`Claim::turn`]), with the
    /// modifiers at the press; what it takes and does is decided when its turn comes.
    waits: Option<Modifiers>,
}

/// The path being drawn with the Pen: the host's open draft, its gestures and its keys. The
/// overlay draws, senses and reports the whole draft as an [`EditSubpath`]; the host keeps it
/// from the events and ends it with the commit.
///
/// ```
/// use craft_draw::ui::{EditSubpath, PenDraft, PenNames};
/// use craft_ui::canvas::{CanvasView, kurbo::Point};
/// # egui::__run_test_ui(|ui| {
/// let (rect, canvas) = ui.allocate_exact_size(egui::vec2(400.0, 300.0), egui::Sense::click_and_drag());
/// let view = CanvasView::new(rect, Point::new(200.0, 150.0), 1.0);
/// let draft: Option<EditSubpath> = None;
/// let names = PenNames::new("Path being drawn", |i, n| format!("Anchor {i} of {n}"));
/// let r = PenDraft::new("pen", draft.as_ref(), names).show(ui, &canvas, &view);
/// # let _ = r;
/// # });
/// ```
///
/// - Pointer, with the pen tool: a click adds a corner anchor (⇧ on the nearest 45° ray from the
///   last one); a drag pulls out symmetric handles (⇧ at 45°); ⌥ during the drag breaks the pair,
///   so only the out handle follows. A press on the last anchor retracts its out handle and a drag
///   sets it alone; ⌥ on one of its handles grabs that handle alone. A press on the first anchor
///   closes the path on release, a drag there pulling out that anchor's handles. With no draft, a
///   press on an open path's end point ([`ends`](Self::ends)) reports
///   [`resumed`](PenResponse::resumed).
/// - Keys while a draft is open and no text field has focus: Enter and Esc end it open and keep
///   it (a draft of one anchor is discarded, as PhotoCraft files no path under two anchors);
///   Backspace and Delete remove the last anchor, the only one discarding the draft; the host's
///   Undo does the same through [`remove_last`](Self::remove_last). Each ending is
///   [`finished`](PenResponse::finished). A press that is still down finishes first.
/// - Look: the path so far, a dashed rubber band from the last anchor to where a click would land,
///   the last anchor's handles, the anchors with the last one filled, and over the first anchor the
///   close cue.
/// - Not a Tab stop: there is no keyboard drawing. The anchors are AccessKit objects in a group
///   named by the host, the last one selected.
/// - The edit gesture is the draft, its policy not cancellable: Esc ends the path and keeps it.
///   An interrupted draft commits ([`Gesture::commit_on_interrupt`]): anchors are the user's
///   work, and switching tools (the host stops showing the overlay) finishes the path. Keyboard
///   focus plays no part: a click in an options-bar field or Tab leaves the draft open. When the
///   host ends the gesture because another control begins a gesture or sets a value (a stroke
///   width in the options bar), the anchors so far are one undo step and the draft goes on: its
///   next change begins a new gesture ([`Gesture`]'s rule for gestures the host ended). Ending
///   the draft is tool state, not an edit: the host ends it on [`finished`](PenResponse::finished),
///   since a draft ended with no change after such a step has no event to report.
///   [`GestureHost`](craft_ui::edit::GestureHost) judges an interruption from
///   [`ending`](craft_ui::edit::ending).
/// - Order: the host shows the pen after its handles and guides, as it is a tool, and after the
///   [`BezierOverlay`] whose tool [`PathMode::resolve`] picked (it claims only presses on parts).
/// - [`PenMode::Curvature`] ([`mode`](Self::mode)): a click adds a smooth anchor the curve flows
///   through, which follows the pointer while the button is down (past `click_slop`); a
///   press on the first anchor of 3 or more closes at once; a press on another anchor grabs and
///   drags it; the second click of a double-click on an anchor toggles it between smooth and
///   corner. Every change reflows the handles ([`EditSubpath::reflow`]). ⇧ and ⌥ do nothing; the
///   keys and the edit gesture are the Pen's; there is no resume. Hover: with two or more
///   anchors a dashed preview of the curve the next click makes, with one a dashed line; always
///   drawn, whatever the rubber band setting.
/// - [`sensing`](Self::sensing) false: drawn, keys kept, no press taken (⌘ held).
///   [`auto_add_delete`](Self::auto_add_delete) (Pen mode): a press on a segment of the draft
///   inserts an anchor, on an anchor that is not the first or the last deletes it, each an
///   Update; off while ⇧, ⌥ or ⌘ is held and while a handle is pulled. The hover shows the
///   [`HitMark`] of what a press would do. [`path_options`](Self::path_options): the colour and
///   thickness of the draft line and whether the Pen's rubber band shows (off by default, as
///   Photoshop).
pub struct PenDraft<'a> {
    id_salt: IdSalt,
    draft: Option<&'a EditSubpath>,
    names: PenNames<'a>,
    ends: &'a [(u64, KnotRef, Point)],
    units: Option<(&'a RulerUnits, &'a RulerUnits)>,
    zero: Point,
    look: Option<PenStyle>,
    mode: PenMode,
    sensing: bool,
    remove_last: bool,
    auto_add_delete: bool,
    path_options: Option<&'a PathOptionsSettings>,
}

impl<'a> PenDraft<'a> {
    /// The host's open `draft`, named `names` in the AccessKit tree.
    pub fn new(id_salt: impl AsIdSalt, draft: Option<&'a EditSubpath>, names: PenNames<'a>) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            draft,
            names,
            ends: &[],
            units: None,
            zero: Point::ZERO,
            look: None,
            mode: PenMode::default(),
            sensing: true,
            remove_last: false,
            auto_add_delete: false,
            path_options: None,
        }
    }

    /// The Pen or the Curvature Pen; [`PenMode::Pen`] by default.
    pub fn mode(mut self, mode: PenMode) -> Self {
        self.mode = mode;
        self
    }

    /// Whether the draft takes presses; true by default. False draws the draft and keeps its keys
    /// (Enter, Esc, Backspace, Delete, the host's undo) while another overlay takes the pointer:
    /// [`PathMode::draft_senses`] (⌘ making the pointer Direct Selection, gap row 39).
    pub fn sensing(mut self, sensing: bool) -> Self {
        self.sensing = sensing;
        self
    }

    /// The Pen's Auto Add/Delete on the open draft (PhotoCraft's `pen_auto_add_delete`, on by
    /// default there; the host passes its option). False by default. No effect in Curvature mode.
    pub fn auto_add_delete(mut self, on: bool) -> Self {
        self.auto_add_delete = on;
        self
    }

    /// The user's Path Options: the draft line's colour and thickness, and the Pen's rubber band.
    /// Without them: the tokens' look and no rubber band.
    pub fn path_options(mut self, options: &'a PathOptionsSettings) -> Self {
        self.path_options = Some(options);
        self
    }

    /// The end points of the open paths a press continues: the path's key, the end's knot and its
    /// document point.
    pub fn ends(mut self, ends: &'a [(u64, KnotRef, Point)]) -> Self {
        self.ends = ends;
        self
    }

    /// The ruler units of the axes, for the AccessKit values of the anchors; none without.
    pub fn units(mut self, x: &'a RulerUnits, y: &'a RulerUnits) -> Self {
        self.units = Some((x, y));
        self
    }

    /// The document point the rulers count from, for the AccessKit values.
    pub fn zero(mut self, zero: Point) -> Self {
        self.zero = zero;
        self
    }

    /// A look of the host's in place of [`style`](Self::style)'s.
    pub fn look(mut self, style: PenStyle) -> Self {
        self.look = Some(style);
        self
    }

    /// The pen draft's look in `tokens`.
    pub fn style(tokens: &UiTokens) -> PenStyle {
        let (stroke, size) = (&tokens.stroke, &tokens.size);
        let b = BezierStyle::new(tokens);
        PenStyle {
            draft_line: stroke.medium,
            color: b.color,
            rubber_width: stroke.thin,
            rubber_dash: stroke.dash,
            rubber_gap: stroke.dash_gap,
            anchor: b.anchor,
            selected_anchor: b.selected_anchor,
            control: b.control,
            control_line: stroke.thin,
            close_anchor: adjusted(b.selected_anchor, |h| h.size = size.handle.md),
            close_ring: b.close_ring,
            close_offset: b.close_offset,
            hit: b.hit,
            click_slop: b.click_slop,
            flatten: b.flatten,
        }
    }

    /// The look of this pass: the given or token look with the Path Options' colour and
    /// thickness.
    fn styled(&self, ui: &Ui) -> PenStyle {
        let mut style = self.look.unwrap_or_else(|| PenDraft::style(&Tokens::of(ui)));
        if let Some(options) = self.path_options {
            if let Some([r, g, b]) = options.color {
                style.color = Color32::from_rgb(r, g, b);
            }
            if let Some(thickness) = options.thickness {
                style.draft_line = thickness;
            }
        }
        style
    }

    /// [`PenResponse::closes`]: sensing, no button down and the pointer over the canvas.
    fn closes_now(&self, ui: &Ui, canvas: &Response, view: &CanvasView, draft: &EditSubpath, style: &PenStyle) -> bool {
        let (pointer, down) = ui.input(|i| (i.pointer.latest_pos(), i.pointer.primary_down()));
        self.sensing && !down && pointer.is_some_and(|p| view.rect.contains(p) && canvas.contains_pointer() && self.closes_at(draft, p, view, style))
    }

    /// Removes the draft's last anchor in this pass when `remove` is true: the host's Undo while a
    /// draft is open (PhotoCraft ⌘Z). The only anchor discards the draft. The host cancels a
    /// running press with `CancelReason::UndoDuringGesture` before it routes Undo, because the
    /// pass ignores the request while a press is held.
    pub fn remove_last(mut self, remove: bool) -> Self {
        self.remove_last = remove;
        self
    }

    /// Draws the draft over `view` and senses through the host's `canvas` response.
    pub fn show(mut self, ui: &mut Ui, canvas: &Response, view: &CanvasView) -> PenResponse {
        let style = self.styled(ui);
        self.look = Some(style);
        if self.mode == PenMode::Curvature {
            return self.show_curvature(ui, canvas, view);
        }
        let looks = Looks { pen: style, bezier: BezierOverlay::style(&Tokens::of(ui)) };
        let ctx = ui.ctx().clone();
        let id = ui.make_persistent_id(self.id_salt);
        let editing = ui.is_enabled();
        let host = self.draft.cloned().unwrap_or_default();
        let mut gesture = Gesture::load(ui, id, &host, EditPolicy::default().cancellable(false)).commit_on_interrupt(true).enabled(editing);
        // A draft the host hands in (a resumed path) is the gesture's start.
        if !gesture.is_active() && !host.knots.is_empty() {
            gesture.begin();
        }
        let mut out = PenResponse { id, event: None, finished: false, resumed: None, claimed: false, closes: false };

        let mut drag: Option<Drag> = ctx
            .data(|d| d.get_temp(drag_key(id)))
            .filter(|d: &Drag| d.press.holds(&ctx) || (d.waits.is_some() && Claim::is_owner(&ctx, canvas.id, id)));
        let waited = drag.as_ref().is_some_and(|d| d.waits.is_some());
        if let Some(d) = &drag
            && let Some(mods) = d.waits
        {
            drag = match Claim::turn(&ctx, canvas.id, id) {
                Turn::Waits => drag,
                Turn::Runs => self.start(view, &looks, &mut gesture, &mut out, d.press, mods),
                Turn::Now => None,
            };
        }
        if drag.is_none() && !Claim::keeps_waiting(&ctx, canvas.id, id, waited) && editing && self.sensing {
            drag = self.press(&ctx, canvas, view, &looks, &mut gesture, &mut out);
        }
        if let Some(d) = drag.take() {
            drag = if d.waits.is_some() { Some(d) } else { self.track(&ctx, view, &style, d, &mut gesture, &mut out) };
        }
        if editing && drag.is_none() {
            out.finished |= self.keys(ui, self.remove_last, &mut gesture);
        }

        let draft = gesture.shown().clone();
        out.closes = self.closes_now(ui, canvas, view, &draft, &style);
        let parts = self.parts(&draft, view, &style);
        PartFocus::new(id, &self.names.group).parts(&parts).focusable(false).picked(parts.last().map(|p| p.key)).show(ui, canvas);

        out.event = gesture.finish();
        out.claimed = Claim::is_owner(&ctx, canvas.id, id);
        ctx.data_mut(|d| match &drag {
            Some(drag) => {
                d.insert_temp(drag_key(id), drag.clone());
            }
            None => d.remove::<Drag>(drag_key(id)),
        });
        self.paint(ui, canvas, view, &looks, &draft, drag.as_ref());
        out
    }

    /// A press on the canvas: the part of the draft under it, or a new anchor.
    fn press(
        &self,
        ctx: &Context,
        canvas: &Response,
        view: &CanvasView,
        looks: &Looks,
        gesture: &mut Gesture<EditSubpath>,
        out: &mut PenResponse,
    ) -> Option<Drag> {
        if !ctx.input(|i| i.pointer.primary_pressed()) {
            return None;
        }
        let press = PrimaryPress::current(ctx)?;
        if !(view.rect.contains(press.origin) && canvas.contains_pointer()) {
            return None;
        }
        if !Claim::take(ctx, canvas.id, out.id) {
            return None;
        }
        let mods = ctx.input(|i| i.modifiers);
        if Claim::turn(ctx, canvas.id, out.id) == Turn::Waits {
            return Some(Drag { press, kind: Kind::Pull, moved: false, in_ctrl: Point::ZERO, waits: Some(mods) });
        }
        self.start(view, looks, gesture, out, press, mods)
    }

    /// The press `press` made with `mods`, against the draft now: begins what it takes, or
    /// resumes the path whose end it hit.
    fn start(
        &self,
        view: &CanvasView,
        looks: &Looks,
        gesture: &mut Gesture<EditSubpath>,
        out: &mut PenResponse,
        press: PrimaryPress,
        mods: Modifiers,
    ) -> Option<Drag> {
        let Looks { pen: style, bezier } = looks;
        let mut draft = gesture.shown().clone();
        // Auto Add/Delete takes the press whole: one Update, no drag, nothing added.
        if let Some(edit) = self.auto_edit(&draft, press.origin, mods, false, view, bezier) {
            let changed = match edit {
                AutoEdit::Insert { segment, t } => draft.insert_at(segment, t).is_some(),
                AutoEdit::Delete { knot } => draft.remove_knot(knot),
            };
            if changed {
                gesture.begin();
                gesture.update(draft, mods);
            }
            return None;
        }
        let near = |p: Point| hit::point(press.origin, view.to_screen(p)) <= style.hit;
        let n = draft.knots.len();
        let handle = (n > 0 && mods.alt)
            .then(|| {
                let last = draft.knots[n - 1];
                [HandleSide::Out, HandleSide::In].into_iter().find(|&side| !last.retracted(side) && near(last.handle(side)))
            })
            .flatten();
        let kind = if let Some(side) = handle {
            Kind::Grab(side)
        } else if n >= 2 && !draft.closed && near(draft.knots[0].anchor) {
            Kind::Close
        } else if n >= 1 && near(draft.knots[n - 1].anchor) {
            Kind::Retract
        } else if n == 0
            && let Some(&(path, knot, _)) = self.ends.iter().find(|(_, _, p)| near(*p))
        {
            out.resumed = Some((path, knot));
            return None;
        } else {
            Kind::Pull
        };
        let at = match kind {
            Kind::Close => 0,
            _ => n.saturating_sub(1),
        };
        let in_ctrl = match kind {
            Kind::Pull => {
                let mut p = view.to_doc(press.origin);
                if mods.shift && n > 0 {
                    p = draft.knots[n - 1].anchor + constrain(p - draft.knots[n - 1].anchor);
                }
                draft.knots.push(Knot::corner(p));
                p
            }
            Kind::Retract => {
                let k = &mut draft.knots[at];
                k.out_ctrl = k.anchor;
                k.smooth = false;
                k.in_ctrl
            }
            Kind::Close | Kind::Grab(_) => draft.knots[at].in_ctrl,
        };
        gesture.begin();
        gesture.update(draft, mods);
        Some(Drag { press, kind, moved: false, in_ctrl, waits: None })
    }

    /// One pass of a press the overlay owns: the press that goes on, if it does.
    fn track(
        &self,
        ctx: &Context,
        view: &CanvasView,
        style: &PenStyle,
        mut d: Drag,
        gesture: &mut Gesture<EditSubpath>,
        out: &mut PenResponse,
    ) -> Option<Drag> {
        let (pointer, down, mods) = ctx.input(|i| (i.pointer.latest_pos(), i.pointer.primary_down(), i.modifiers));
        if let Some(pointer) = pointer {
            d.moved |= pointer.distance(d.press.origin) > style.click_slop;
            let mut draft = gesture.shown().clone();
            let at = if d.kind == Kind::Close { 0 } else { draft.knots.len().saturating_sub(1) };
            if d.moved && at < draft.knots.len() {
                pull(&mut draft.knots[at], d.kind, view.to_doc(pointer), mods, &mut d.in_ctrl);
                gesture.update(draft, mods);
            }
        }
        if down {
            return Some(d);
        }
        if d.kind == Kind::Close {
            let mut draft = gesture.shown().clone();
            draft.closed = true;
            gesture.update(draft, mods);
            gesture.commit();
            out.finished = true;
        }
        None
    }

    /// Enter, Esc, Backspace and Delete, and the host's undo, while a draft is open: whether they
    /// ended it.
    fn keys(&self, ui: &Ui, undo: bool, gesture: &mut Gesture<EditSubpath>) -> bool {
        let draft = gesture.shown().clone();
        if draft.knots.is_empty() || !nudge::reads(ui) {
            return false;
        }
        let (end, remove) = ui.input_mut(|i| {
            let end = i.consume_key(Modifiers::NONE, Key::Enter) | i.consume_key(Modifiers::NONE, Key::Escape);
            let remove = i.consume_key(Modifiers::NONE, Key::Backspace) | i.consume_key(Modifiers::NONE, Key::Delete);
            (end, remove)
        });
        let mods = ui.input(|i| i.modifiers);
        if remove || undo {
            if draft.knots.len() <= 1 {
                discard(gesture, CancelReason::UndoDuringGesture, mods);
                return true;
            }
            let mut draft = draft;
            draft.knots.pop();
            gesture.update(draft, mods);
        } else if end {
            if draft.knots.len() < 2 {
                discard(gesture, CancelReason::Escape, mods);
            } else {
                gesture.update(draft, mods);
                gesture.commit();
            }
            return true;
        }
        false
    }

    /// The anchors' AccessKit nodes, the last one selected.
    fn parts(&self, draft: &EditSubpath, view: &CanvasView, style: &PenStyle) -> Vec<Part> {
        let n = draft.knots.len();
        draft
            .knots
            .iter()
            .enumerate()
            .map(|(i, k)| {
                let at = view.to_screen(k.anchor);
                let value =
                    self.units.map(|(x, y)| format!("{}, {}", x.format_value(k.anchor.x, self.zero.x), y.format_value(k.anchor.y, self.zero.y)));
                Part {
                    key: i as u64,
                    name: (self.names.anchor)(i + 1, n),
                    value,
                    rect: egui::Rect::from_center_size(at, vec2(2.0 * style.hit, 2.0 * style.hit)),
                    selected: i + 1 == n,
                }
            })
            .collect()
    }

    fn paint(&self, ui: &Ui, canvas: &Response, view: &CanvasView, looks: &Looks, draft: &EditSubpath, drag: Option<&Drag>) {
        let Looks { pen: style, bezier } = looks;
        let n = draft.knots.len();
        if n == 0 {
            return;
        }
        let painter = ui.painter_at(view.rect);
        let ppp = ui.pixels_per_point();
        let to_screen = view.affine();
        let pointer = ui.input(|i| i.pointer.latest_pos()).filter(|p| view.rect.contains(*p) && canvas.contains_pointer());
        let pressing = drag.is_some();
        // The path so far, the first segment bending with a handle pulled out in a close drag.
        outline(&painter, to_screen * draft.bez_path(), f64::from(style.flatten), Stroke::new(style.draft_line, style.color));
        let last = draft.knots[n - 1];
        let pointer = pointer.filter(|_| self.sensing);
        let mods = ui.input(|i| i.modifiers);
        // Where a press edits the draft, the hover marks that edit and no rubber band runs.
        let auto = pointer.filter(|_| !pressing).and_then(|p| self.auto_edit(draft, p, mods, false, view, bezier));
        if let Some(edit) = auto {
            let found = match edit {
                AutoEdit::Insert { segment, t } => BezierHit::Segment { path: 0, segment: KnotRef::new(0, segment), t },
                AutoEdit::Delete { knot } => BezierHit::Anchor { path: 0, knot: KnotRef::new(0, knot) },
            };
            let path = EditPath { key: 0, subpaths: vec![draft.clone()], color: None };
            HitMark::new(&path, found).paint(&painter, view, bezier);
        }
        if let Some(pointer) = pointer
            && !pressing
            && auto.is_none()
            && self.path_options.is_some_and(|o| o.rubber_band)
        {
            let mut end = view.to_doc(pointer);
            if mods.shift {
                end = last.anchor + constrain(end - last.anchor);
            }
            let line = [view.to_screen(last.anchor), view.to_screen(end)];
            painter.extend(Shape::dashed_line(&line, Stroke::new(style.rubber_width, style.color), style.rubber_dash, style.rubber_gap));
        }
        let stroke = Stroke::new(style.control_line, style.control.fill);
        for side in [HandleSide::In, HandleSide::Out] {
            if !last.retracted(side) {
                painter.line_segment([view.to_screen(last.anchor), view.to_screen(last.handle(side))], stroke);
            }
        }
        for (i, k) in draft.knots.iter().enumerate() {
            let look = if i + 1 == n { &style.selected_anchor } else { &style.anchor };
            painter.extend(Handle::shapes_with(look, HandleShape::Square, view.to_screen(k.anchor), ppp));
        }
        for side in [HandleSide::In, HandleSide::Out] {
            if !last.retracted(side) {
                painter.extend(Handle::shapes_with(&style.control, HandleShape::Rotate, view.to_screen(last.handle(side)), ppp));
            }
        }
        if let Some(pointer) = pointer
            && n >= 2
            && !draft.closed
            && hit::point(pointer, view.to_screen(draft.knots[0].anchor)) <= style.hit
        {
            let first = view.to_screen(draft.knots[0].anchor);
            painter.extend(Handle::shapes_with(&style.close_anchor, HandleShape::Square, first, ppp));
            let ring = pointer + vec2(style.close_offset, style.close_offset);
            painter.circle_stroke(ring, style.close_ring, Stroke::new(style.draft_line, style.color));
        }
    }
}

/// A pulled knot: its handles follow `to`. A pull and a close drag put the out handle there and
/// the in handle opposite it, unless ⌥ keeps the in handle where it was; a retracted anchor's
/// handle and a grabbed handle move alone. ⇧ turns the handle to the nearest 45° ray.
fn pull(knot: &mut Knot, kind: Kind, to: Point, mods: Modifiers, in_ctrl: &mut Point) {
    let mut v = to - knot.anchor;
    if mods.shift {
        v = constrain(v);
    }
    match kind {
        Kind::Pull | Kind::Close => {
            knot.out_ctrl = knot.anchor + v;
            knot.in_ctrl = if mods.alt { *in_ctrl } else { knot.anchor - v };
            *in_ctrl = knot.in_ctrl;
            knot.smooth = !mods.alt;
        }
        Kind::Retract => {
            knot.out_ctrl = knot.anchor + v;
            knot.smooth = false;
        }
        Kind::Grab(HandleSide::Out) => {
            knot.out_ctrl = knot.anchor + v;
            knot.smooth = false;
        }
        Kind::Grab(HandleSide::In) => {
            knot.in_ctrl = knot.anchor + v;
            knot.smooth = false;
        }
    }
}

fn drag_key(id: Id) -> Id {
    id.with("craft_ui::canvas::PenDraft::drag")
}

/// Ends a draft too short to be a path (PhotoCraft files none under two anchors). The gesture's
/// start is what the host holds of the draft: when that is empty a cancel leaves nothing behind;
/// when it holds anchors (a resumed path, or anchors kept when the host ended the gesture) the
/// empty draft is committed, so the host removes them.
fn discard(gesture: &mut Gesture<EditSubpath>, reason: CancelReason, mods: Modifiers) {
    if gesture.start().is_some_and(|start| !start.knots.is_empty()) {
        gesture.update(EditSubpath::default(), mods);
        gesture.commit();
    } else {
        gesture.cancel(reason);
    }
}

#[cfg(test)]
mod tests;
