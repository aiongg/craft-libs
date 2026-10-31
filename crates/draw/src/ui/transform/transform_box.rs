//! [`TransformBox`]: the box of Free Transform and of a Move tool's transform controls.

use craft_ui::testing::canvas_story::ViewArgs;
use egui::{AsIdSalt, Color32, Context, CursorIcon, Id, IdSalt, Key, Modifiers, Painter, Pos2, Response, Shape, Stroke, Ui, vec2};
use kurbo::{Point, Rect, Vec2};

use craft_ui::behavior::nudge;
use craft_ui::canvas::claim::PrimaryPress;
use craft_ui::canvas::quad::{self, BoxPart, BoxRules, BoxState, ScaleRule, TransformMode};
use craft_ui::canvas::{CanvasView, Claim, Part, PartFocus, Readout, RulerUnits, resize_cursor, turn_cursor};
use craft_ui::edit::order::Turn;
use craft_ui::edit::{EditEvent, EditPolicy, Gesture};
use craft_ui::localized::{Labels, Unit, Word, Words};
use craft_ui::popup_list;
use craft_ui::primitives::{FocusRing, Handle, HandleShape, HandleStyle};
use craft_ui::story::{Level, Story, StoryChoice, Surface};
use craft_ui::theme::{ToEgui, Tokens, TokensOf, UiTokens};
use craft_ui::typed_story;

/// Where a press outside the box rotates it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum RotateZone {
    /// Anywhere outside (PhotoCraft's Free Transform); the rotate pointer shows within
    /// `transform.rotate_reach` of a corner.
    #[default]
    Outside,
    /// Only `transform.band_inner` to `transform.band_outer` outside a corner (PhotoCraft's Move
    /// tool controls, DesignCraft, VectorCraft); the rest of the outside is the tool's.
    Band,
}

/// The AccessKit names of a box and its parts. `Default` is English; a host passes its own
/// translated text, or [`TransformNames::of`] for the words installed in the context.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransformNames {
    /// The group, e.g. "Free Transform".
    pub group: String,
    /// The whole box as a keyboard part, e.g. "Whole box".
    pub whole: String,
    /// The corners clockwise from the top left, as the document has them: "Top left corner" …
    pub corners: [String; 4],
    /// The edges clockwise from the top: "Top edge" …
    pub edges: [String; 4],
    /// The reference point.
    pub pivot: String,
}

/// One word per field of [`TransformNames`], in field order.
const NAME_WORDS: [Word; 11] = [
    Word::keyed("transform.group", "Free Transform"),
    Word::keyed("transform.whole", "Whole box"),
    Word::keyed("transform.topLeftCorner", "Top left corner"),
    Word::keyed("transform.topRightCorner", "Top right corner"),
    Word::keyed("transform.bottomRightCorner", "Bottom right corner"),
    Word::keyed("transform.bottomLeftCorner", "Bottom left corner"),
    Word::keyed("transform.topEdge", "Top edge"),
    Word::keyed("transform.rightEdge", "Right edge"),
    Word::keyed("transform.bottomEdge", "Bottom edge"),
    Word::keyed("transform.leftEdge", "Left edge"),
    Word::keyed("transform.pivot", "Reference point"),
];

impl Labels for TransformNames {
    const WORDS: &'static [Word] = &NAME_WORDS;

    fn english() -> Self {
        let blank = Self { group: String::new(), whole: String::new(), corners: Default::default(), edges: Default::default(), pivot: String::new() };
        blank.localized_in(&Words::english())
    }

    fn localized_in(self, words: &Words) -> Self {
        let text = |i: usize| words.get(NAME_WORDS[i]).into_owned();
        Self {
            group: text(0),
            whole: text(1),
            corners: std::array::from_fn(|i| text(2 + i)),
            edges: std::array::from_fn(|i| text(6 + i)),
            pivot: text(10),
        }
    }
}

impl Default for TransformNames {
    fn default() -> Self {
        Self::english()
    }
}

/// The labels and units of a box's readout during a drag (PhotoCraft's "Angle:", "W:", "H:",
/// "ΔX:", "ΔY:"). `Default` is English.
#[derive(Clone, Debug)]
pub struct TransformLabels {
    pub angle: String,
    pub w: String,
    pub h: String,
    pub dx: String,
    pub dy: String,
    /// Around a W or H scale: `{n}%`.
    pub percent: Unit,
    /// Around an angle: `{n}°`.
    pub degrees: Unit,
}

const LABEL_WORDS: [Word; 7] = [
    Word::keyed("transform.readoutAngle", "Angle:"),
    Word::keyed("transform.readoutWidth", "W:"),
    Word::keyed("transform.readoutHeight", "H:"),
    Word::keyed("transform.readoutDx", "ΔX:"),
    Word::keyed("transform.readoutDy", "ΔY:"),
    Word::in_context("unit", "{n}%"),
    Word::in_context("unit", "{n}°"),
];

impl Labels for TransformLabels {
    const WORDS: &'static [Word] = &LABEL_WORDS;

    fn english() -> Self {
        Self::new("", "", "", "", "").localized_in(&Words::english())
    }

    fn localized_in(self, words: &Words) -> Self {
        let text = |i: usize| words.get(LABEL_WORDS[i]).into_owned();
        Self { angle: text(0), w: text(1), h: text(2), dx: text(3), dy: text(4), percent: Unit::new(&text(5)), degrees: Unit::new(&text(6)) }
    }
}

impl Default for TransformLabels {
    fn default() -> Self {
        Self::english()
    }
}

impl TransformLabels {
    /// The labels with `{n}%` and `{n}°` round the numbers.
    pub fn new(angle: impl Into<String>, w: impl Into<String>, h: impl Into<String>, dx: impl Into<String>, dy: impl Into<String>) -> Self {
        Self { angle: angle.into(), w: w.into(), h: h.into(), dx: dx.into(), dy: dy.into(), percent: Unit::new("{n}%"), degrees: Unit::new("{n}°") }
    }
}

/// A box's look and reach, from [`TransformBox::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct TransformStyle {
    /// The box line: `transform.line` of `overlay.line`, bare.
    pub line: Stroke,
    /// The 8 handles: [`Handle::style`] `Square`.
    pub handle: HandleStyle,
    /// Distance from a handle or the reference point within which a press takes it:
    /// `transform.hit`.
    pub hit: f32,
    /// Distance from a corner within which the rotate pointer shows outside: `transform.rotate_reach`.
    pub rotate_reach: f32,
    /// The rotate band of [`RotateZone::Band`], from a corner: `transform.band_inner`,
    /// `transform.band_outer`.
    pub band_inner: f32,
    pub band_outer: f32,
    /// Farthest a press outside may move and still be a click: `transform.click_slop`.
    pub click_slop: f32,
    /// The reference point's circle: `transform.pivot_radius`.
    pub pivot_radius: f32,
    /// Each arm of its cross, from the centre: `transform.pivot_arm`.
    pub pivot_arm: f32,
    /// The reference point's lines: `transform.line` of `overlay.handle_fill`.
    pub pivot: Stroke,
    /// Under them: `overlay.halo`, `handle.halo` wider on each side.
    pub pivot_halo: Stroke,
    /// The focus line round the whole box: the focus ring's stroke.
    pub focus: Stroke,
    /// Under it: `overlay.halo`, `handle.halo` wider on each side.
    pub focus_halo: Stroke,
    /// From the box line to the focus line: the focus ring's offset.
    pub focus_offset: f32,
}

/// What [`TransformBox::show`] reports.
#[derive(Clone, Debug, PartialEq)]
pub struct TransformBoxResponse {
    /// The box's id, for [`TransformBox::focus`].
    pub id: Id,
    /// A drag (Begin on the first move, Update, Commit on release; Esc cancels it) or an arrow
    /// key's Set.
    pub event: Option<EditEvent<BoxState>>,
    /// Apply the box: Enter, a click outside it, a double-click inside it.
    pub apply: bool,
    /// Close the box without applying it: Esc while no drag is on.
    pub discard: bool,
    /// The part the keys act on ([`BoxPart::Inside`]: the whole box), picked by the pointer or the
    /// Page keys; `None` before either.
    pub picked: Option<BoxPart>,
    /// The part a press at the pointer would take; `None` where it is the tool's.
    pub hovered: Option<BoxPart>,
    /// Whether the box owns the current press: the host skips its tool input.
    pub claimed: bool,
}

/// The host's snapping of a dragged point ([`TransformBox::snap`]).
type Snap<'a> = Box<dyn Fn(BoxPart, Point, Modifiers) -> Point + 'a>;

/// A press the host-driven calls track, kept by the host from [`TransformBox::press`] to
/// [`TransformBox::release`]. [`show`](TransformBox::show) keeps the same value in the context's
/// memory under the box's id.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoxPress {
    part: BoxPart,
    /// The host's box at the press.
    at_press: BoxState,
    /// The box the drag works from: `at_press`, its reference point moved by a ⌥-press.
    start: BoxState,
    /// The press in document space.
    from: Point,
    /// The press in canvas space.
    origin: Pos2,
    began: bool,
    /// A ⌥-press away from the handles: a click there moves the reference point.
    alt_pivot: bool,
    /// The box the drag last produced.
    last: BoxState,
}

impl BoxPress {
    /// The part the press took.
    pub fn part(&self) -> BoxPart {
        self.part
    }

    /// Whether the drag has begun (a move past the click slop outside, any move elsewhere).
    pub fn began(&self) -> bool {
        self.began
    }

    /// The box the drag last produced; the host's box at the press until it begins.
    pub fn last(&self) -> &BoxState {
        &self.last
    }
}

/// What [`TransformBox::track`] reports for one pointer position.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoxTrack {
    /// The drag began with this position: the host takes its "before" state.
    pub begun: bool,
    /// The box the drag produces at the pointer, once it has begun.
    pub state: Option<BoxState>,
}

/// What [`TransformBox::release`] reports.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BoxRelease {
    /// The drag ended with this box (the last one [`BoxPress::last`] gave).
    Commit(BoxState),
    /// A click outside the box: apply it.
    Apply,
    /// A ⌥-click that moved the reference point: the box with it moved.
    Pivot(BoxState),
    /// Nothing happened.
    Nothing,
}

/// A drag [`TransformBox::show`] keeps between passes.
#[derive(Clone, Copy, Debug)]
struct Drag {
    core: BoxPress,
    press: PrimaryPress,
    /// Esc, an outside change or a missed pass ended the gesture; the rest of the press does
    /// nothing.
    cancelled: bool,
    /// The press waits for another control's typing run to end ([`Claim::turn`]); nothing of it
    /// has run.
    waits: bool,
}

/// The box of Free Transform: 8 handles and a reference point over the host's content, dragged by
/// the pointer through [`quad::drag`]'s rules, moved and picked from the keyboard.
///
/// ```
/// use craft_draw::ui::{TransformBox, TransformNames};
/// use craft_ui::canvas::{CanvasView, kurbo::{Point, Rect}, quad::BoxState};
/// # egui::__run_test_ui(|ui| {
/// let (rect, canvas) = ui.allocate_exact_size(egui::vec2(400.0, 300.0), egui::Sense::click_and_drag());
/// let view = CanvasView::new(rect, Point::new(200.0, 150.0), 1.0);
/// let state = BoxState::from_rect(Rect::new(100.0, 80.0, 300.0, 220.0));
/// let names = TransformNames {
///     group: "Free Transform".into(),
///     whole: "Whole box".into(),
///     corners: ["Top left corner", "Top right corner", "Bottom right corner", "Bottom left corner"].map(String::from),
///     edges: ["Top edge", "Right edge", "Bottom edge", "Left edge"].map(String::from),
///     pivot: "Reference point".into(),
/// };
/// let r = TransformBox::new("free-transform", &state, names).show(ui, &canvas, &view);
/// if !r.claimed {
///     // the tool's input
/// }
/// # });
/// ```
///
/// - The host owns the box (a session, PhotoCraft's model): it applies the events to its
///   [`BoxState`], passes it back each frame, commits on [`apply`](TransformBoxResponse::apply)
///   and cancels on [`discard`](TransformBoxResponse::discard).
/// - The host shows it after its guides (it draws over them) and before them in input order: the
///   box senses through the canvas response and takes its presses with [`Claim`] first.
/// - Pointer: the nearest handle or the reference point within `transform.hit` on screen, else
///   inside or outside. Parts the [rules](Self::rules) refuse don't take a press; a press outside
///   is taken in [`RotateZone::Outside`] even where rotating is refused, so a click there applies.
///   ⌥-press away from the handles moves the reference point there and drags it. A drag begins on
///   the first move (outside, past `transform.click_slop`).
/// - Keys, whenever no text field has focus and no popup is open: Enter applies; Esc cancels a
///   drag, ends a held press before its first move (the rest of the press does nothing) and
///   otherwise discards; the arrows move the whole box, or, while the box has keyboard focus, the
///   picked part as a plain drag of it would. With the focus (one Tab stop, [`PartFocus`]) Page
///   Down and Page Up pick the next and previous part: the whole box, the corners and edges
///   clockwise from the top left, the reference point.
/// - Nothing mirrors in a right-to-left interface: the box is the document's.
pub struct TransformBox<'a> {
    id_salt: IdSalt,
    state: &'a BoxState,
    names: TransformNames,
    rules: BoxRules,
    zone: RotateZone,
    inside_moves: bool,
    units: Option<(&'a RulerUnits, &'a RulerUnits)>,
    readout: Option<TransformLabels>,
    start: Option<(Rect, f64)>,
    snap: Option<Snap<'a>>,
    value: Option<String>,
    policy: EditPolicy,
    modifiers: Option<Modifiers>,
    pixel_grid: bool,
}

impl<'a> TransformBox<'a> {
    /// The host's box `state`, named `names` in the AccessKit tree.
    pub fn new(id_salt: impl AsIdSalt, state: &'a BoxState, names: TransformNames) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            state,
            names,
            rules: BoxRules::default(),
            zone: RotateZone::default(),
            inside_moves: true,
            units: None,
            readout: None,
            start: None,
            snap: None,
            value: None,
            policy: EditPolicy::default(),
            modifiers: None,
            pixel_grid: false,
        }
    }

    /// The mode, scaling rule, aspect link, what the content allows and the rotate step;
    /// PhotoCraft's Free Transform by default.
    pub fn rules(mut self, rules: BoxRules) -> Self {
        self.rules = rules;
        self
    }

    /// The modifiers every test of this box reads (the press, the drag, the host's snap) in place
    /// of the input state: a touch host's sticky modifiers, or Ctrl turning snapping off.
    pub fn modifiers(mut self, modifiers: Modifiers) -> Self {
        self.modifiers = Some(modifiers);
        self
    }

    /// Rounds the corners of each dragged box to whole document units (PhotoCraft's Snap
    /// Transforms to Pixel Grid), so the host stores the emitted box unchanged. It rounds from the
    /// gesture's start box and the pointer on every move, so it does not drift. Only an upright
    /// rectangle (flips included) is rounded, and one that would collapse below a unit keeps its
    /// exact corners. Keys are not rounded.
    pub fn pixel_grid(mut self, on: bool) -> Self {
        self.pixel_grid = on;
        self
    }

    /// Where a press outside rotates; [`RotateZone::Outside`] by default.
    pub fn rotate_zone(mut self, zone: RotateZone) -> Self {
        self.zone = zone;
        self
    }

    /// Whether a press inside moves the box; true by default. False leaves it to the tool
    /// (PhotoCraft's Move tool controls).
    pub fn inside_moves(mut self, moves: bool) -> Self {
        self.inside_moves = moves;
        self
    }

    /// The ruler units of the document's x and y axes: the parts' AccessKit values and the
    /// readout's numbers. Without them the parts have no value and there is no readout.
    pub fn units(mut self, x: &'a RulerUnits, y: &'a RulerUnits) -> Self {
        self.units = Some((x, y));
        self
    }

    /// Shows the readout beside the pointer during a drag (PhotoCraft's Show Transformation
    /// Values): the angle while rotating, W and H while scaling a parallelogram (with
    /// [`start`](Self::start)), ΔX and ΔY while moving.
    pub fn readout(mut self, labels: TransformLabels) -> Self {
        self.readout = Some(labels);
        self
    }

    /// The box's rect before the transform and the V skew the host keeps (degrees), which the W
    /// and H readout reads the box against ([`BoxState::parts`]).
    pub fn start(mut self, rect: Rect, skew_v: f64) -> Self {
        self.start = Some((rect, skew_v));
        self
    }

    /// The host's snapping: where a drag of a part takes the pointer's document point, with the
    /// modifiers held (PhotoCraft's pixel grid, a distorted corner).
    pub fn snap(mut self, snap: impl Fn(BoxPart, Point, Modifiers) -> Point + 'a) -> Self {
        self.snap = Some(Box::new(snap));
        self
    }

    /// The group's value for a screen reader, e.g. "W 120 %, H 100 %, 15°".
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn style(tokens: &UiTokens) -> TransformStyle {
        let (t, o) = (&tokens.component.transform, &tokens.color.overlay);
        let halo = tokens.component.handle.halo;
        let ring = FocusRing::style(tokens);
        TransformStyle {
            line: Stroke::new(t.line, o.line.to_egui()),
            handle: Handle::style(tokens, HandleShape::Square),
            hit: t.hit,
            rotate_reach: t.rotate_reach,
            band_inner: t.band_inner,
            band_outer: t.band_outer,
            click_slop: t.click_slop,
            pivot_radius: t.pivot_radius,
            pivot_arm: t.pivot_arm,
            pivot: Stroke::new(t.line, o.handle_fill.to_egui()),
            pivot_halo: Stroke::new(t.line + 2.0 * halo, o.halo.to_egui()),
            focus: ring.stroke,
            focus_halo: Stroke::new(ring.stroke.width + 2.0 * halo, o.halo.to_egui()),
            focus_offset: ring.offset,
        }
    }

    /// Gives the box `id` ([`TransformBoxResponse::id`]) the keyboard, as the host does when it
    /// opens the box (PhotoCraft: ⌘T).
    pub fn focus(ctx: &Context, id: Id) {
        ctx.memory_mut(|m| m.request_focus(id));
    }

    /// Draws the box over `view` and senses through the host's `canvas` response.
    pub fn show(self, ui: &mut Ui, canvas: &Response, view: &CanvasView) -> TransformBoxResponse {
        let ctx = ui.ctx().clone();
        let tokens = Tokens::of(ui);
        let style = TransformBox::style(&tokens);
        let id = ui.make_persistent_id(self.id_salt);
        let mut out = TransformBoxResponse { id, event: None, apply: false, discard: false, picked: None, hovered: None, claimed: false };
        let mut picked: Option<BoxPart> = ctx.data(|d| d.get_temp(pick_key(id)));

        // A drag whose press ended while the box was not shown is dropped, and its gesture with it.
        let mut drag: Option<Drag> =
            ctx.data(|d| d.get_temp(drag_key(id))).filter(|d: &Drag| d.press.holds(&ctx) || (d.waits && Claim::is_owner(&ctx, canvas.id, id)));
        let waited = drag.is_some_and(|d| d.waits);
        if let Some(d) = &mut drag
            && d.waits
        {
            match Claim::turn(&ctx, canvas.id, id) {
                Turn::Waits => {}
                Turn::Runs => {
                    d.waits = false;
                    self.rebase(&mut d.core);
                }
                Turn::Now => drag = None,
            }
        }
        if drag.is_none() && !Claim::keeps_waiting(&ctx, canvas.id, id, waited) {
            drag = self.press_pass(ui, canvas, view, &style, id);
            if let Some(d) = drag
                && d.core.part != BoxPart::Outside
            {
                picked = Some(d.core.part);
            }
        }
        let reported = Gesture::<BoxState>::reported(ui, id).map(|(_, value)| value);
        let host = match drag {
            Some(d) if d.core.began && !d.cancelled && (*self.state == d.core.at_press || Some(*self.state) == reported) => {
                reported.unwrap_or(d.core.at_press)
            }
            _ => *self.state,
        };
        let mut gesture = Gesture::load(ui, id, &host, self.policy).focus_owners(&[]).enabled(ui.is_enabled());
        if let Some(d) = &mut drag
            && d.core.began
            && !d.cancelled
            && !gesture.is_active()
        {
            d.cancelled = true;
        }
        // Esc before the first move ends the press as it would end its drag.
        if let Some(d) = &mut drag
            && !d.core.began
            && !d.cancelled
            && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
        {
            d.cancelled = true;
        }
        let pointer = ctx.input(|i| i.pointer.latest_pos());
        if let Some(d) = &mut drag
            && !d.waits
        {
            self.track_pass(&ctx, canvas, view, &style, d, &mut gesture, &mut out);
        }
        if self.double_clicked_inside(&ctx, canvas, view, id, &style) {
            out.apply = true;
        }

        let shown = *gesture.shown();
        let screen = on_screen(view, &shown);
        let parts = self.parts(&screen, &style, &shown, picked);
        let mut focus = PartFocus::new(id, self.names.group.clone()).parts(&parts).picked(picked.map(part_key));
        if let Some(value) = &self.value {
            focus = focus.value(value.clone());
        }
        let focus = focus.show(ui, canvas);
        if let Some(pick) = focus.pick {
            picked = pick.and_then(key_part);
        }
        if drag.is_none_or(|d| d.cancelled) {
            // The pick is the keyboard's: without the focus the arrows move the whole box (PC).
            self.keys(ui, drag.is_none(), picked.filter(|_| focus.focused), &mut gesture, &mut out);
        }

        let dragging = drag.filter(|d| d.core.began && !d.cancelled);
        let other = Claim::owner(&ctx, canvas.id).is_some_and(|owner| owner != id);
        if let Some(d) = dragging
            && let Some(pointer) = pointer
        {
            self.cursor(&ctx, view, &screen, d.core.part, pointer, true);
            self.show_readout(&ctx, id, d.core.part, &shown, &d.core.at_press, pointer);
        } else if drag.is_none()
            && !other
            && let Some(pointer) = canvas.hover_pos()
        {
            let base = on_screen(view, self.state);
            out.hovered = self.takes(&base, pointer, &style, Modifiers::NONE);
            if let Some(part) = out.hovered {
                self.cursor(&ctx, view, &base, part, pointer, false);
            }
        }

        out.event = gesture.finish();
        out.claimed = Claim::is_owner(&ctx, canvas.id, id);
        out.picked = picked;
        let held = drag.filter(|d| d.waits || ctx.input(|i| i.pointer.primary_down()));
        ctx.data_mut(|data| {
            match held {
                Some(d) => {
                    data.insert_temp(drag_key(id), d);
                }
                None => {
                    data.remove::<Drag>(drag_key(id));
                }
            }
            match picked {
                Some(p) => {
                    data.insert_temp(pick_key(id), p);
                }
                None => {
                    data.remove::<BoxPart>(pick_key(id));
                }
            }
        });
        self.paint(ui, view, &screen, &style, picked, focus.keyboard);
        out
    }

    /// The modifiers the box reads: its [`modifiers`](Self::modifiers) if set, else `held`.
    fn mods(&self, held: Modifiers) -> Modifiers {
        self.modifiers.unwrap_or(held)
    }

    /// The part a press at canvas point `point` takes, with `modifiers` held (or the box's own,
    /// [`modifiers`](Self::modifiers)): the nearest handle or the reference point within
    /// `style.hit`, else inside or outside; `None` where a press is the tool's (a part the
    /// [rules](Self::rules) refuse, the inside when it does not move, the outside beyond the
    /// rotate zone). ⌥ over the inside or outside is the reference point. Canvas points are the
    /// screen points `view` maps the document to.
    pub fn hit(&self, view: &CanvasView, style: &TransformStyle, point: Pos2, modifiers: Modifiers) -> Option<BoxPart> {
        self.takes(&on_screen(view, self.state), point, style, self.mods(modifiers))
    }

    /// A press at canvas point `origin`: the press to track, or `None` where [`hit`](Self::hit)
    /// finds no part. The host's box at this call is the press's start.
    pub fn press(&self, view: &CanvasView, style: &TransformStyle, origin: Pos2, modifiers: Modifiers) -> Option<BoxPress> {
        let mods = self.mods(modifiers);
        let screen = on_screen(view, self.state);
        let part = self.takes(&screen, origin, style, mods)?;
        let from = view.to_doc(origin);
        let alt_pivot = part == BoxPart::Pivot && mods.alt && raw_part(&screen, origin, style.hit) != BoxPart::Pivot;
        let start = if alt_pivot { BoxState { pivot: from, ..*self.state } } else { *self.state };
        Some(BoxPress { part, at_press: *self.state, start, from, origin, began: false, alt_pivot, last: *self.state })
    }

    /// The pointer at canvas point `pointer` with `modifiers` held: begins the drag on the first
    /// move (outside, past `style.click_slop`) and gives the box it produces, snapped by the
    /// [`snap`](Self::snap) and rounded by [`pixel_grid`](Self::pixel_grid). The host applies the
    /// box as the update of its gesture.
    pub fn track(&self, view: &CanvasView, style: &TransformStyle, press: &mut BoxPress, pointer: Pos2, modifiers: Modifiers) -> BoxTrack {
        self.track_with(view, style, press, pointer, self.mods(modifiers), &mut || true)
    }

    /// [`track`](Self::track) where beginning the drag may be refused: `begin` answers whether it
    /// began.
    fn track_with(
        &self,
        view: &CanvasView,
        style: &TransformStyle,
        press: &mut BoxPress,
        pointer: Pos2,
        mods: Modifiers,
        begin: &mut dyn FnMut() -> bool,
    ) -> BoxTrack {
        let mut track = BoxTrack { begun: false, state: None };
        let moved = pointer.distance(press.origin);
        let threshold = if press.part == BoxPart::Outside { style.click_slop } else { 0.0 };
        if !press.began && moved > threshold && self.rules.allows(press.part) {
            press.began = begin();
            track.begun = press.began;
        }
        if press.began {
            let to = view.to_doc(pointer);
            let to = self.snap.as_ref().map_or(to, |snap| snap(press.part, to, mods));
            let mut dragged = quad::drag(&press.start, &press.last, press.part, press.from, to, mods, &self.rules);
            if self.pixel_grid {
                dragged = on_pixel_grid(dragged);
            }
            press.last = dragged;
            track.state = Some(dragged);
        }
        track
    }

    /// The button released at canvas point `pointer` (`None` if the pointer left the window):
    /// the end of the drag, or what a click does; `Nothing` when the pointer left the window
    /// without a drag. `double_clicked`: the release completes a double-click, which is not a
    /// click outside; a double-click inside applies the box through
    /// [`double_click`](Self::double_click), not through the release.
    pub fn release(&self, style: &TransformStyle, press: &BoxPress, pointer: Option<Pos2>, double_clicked: bool) -> BoxRelease {
        let near = pointer.is_some_and(|p| p.distance(press.origin) <= style.click_slop);
        if press.began {
            BoxRelease::Commit(press.last)
        } else if press.part == BoxPart::Outside && near && !double_clicked {
            BoxRelease::Apply
        } else if press.alt_pivot {
            BoxRelease::Pivot(press.start)
        } else {
            BoxRelease::Nothing
        }
    }

    /// A press on the canvas read from the input: the part it takes, claimed for the box.
    fn press_pass(&self, ui: &Ui, canvas: &Response, view: &CanvasView, style: &TransformStyle, id: Id) -> Option<Drag> {
        let ctx = ui.ctx();
        if !ui.is_enabled() || !ctx.input(|i| i.pointer.primary_pressed()) {
            return None;
        }
        let press = PrimaryPress::current(ctx)?;
        if !(view.rect.contains(press.origin) && canvas.contains_pointer()) {
            return None;
        }
        let mods = ctx.input(|i| i.modifiers);
        let core = self.press(view, style, press.origin, mods)?;
        if !Claim::take(ctx, canvas.id, id) {
            return None;
        }
        let waits = Claim::turn(ctx, canvas.id, id) == Turn::Waits;
        Some(Drag { core, press, cancelled: false, waits })
    }

    /// Takes the host's box as the press's start when its turn comes: a typing run that ended
    /// first may have changed it.
    fn rebase(&self, p: &mut BoxPress) {
        p.at_press = *self.state;
        p.last = *self.state;
        p.start = if p.alt_pivot { BoxState { pivot: p.from, ..*self.state } } else { *self.state };
    }

    /// The part a press at screen point `p` with `mods` takes from the box mapped to the screen;
    /// `None` where the press is the tool's.
    fn takes(&self, screen: &BoxState, p: Pos2, style: &TransformStyle, mods: Modifiers) -> Option<BoxPart> {
        let part = match raw_part(screen, p, style.hit) {
            BoxPart::Inside if !self.inside_moves => return None,
            BoxPart::Outside if self.zone == RotateZone::Band && !(self.rules.allows(BoxPart::Outside) && in_band(screen, p, style)) => {
                return None;
            }
            part => part,
        };
        match part {
            BoxPart::Inside | BoxPart::Outside if mods.alt => Some(BoxPart::Pivot),
            BoxPart::Outside => Some(part),
            part => self.rules.allows(part).then_some(part),
        }
    }

    /// One pass of a drag read from the input: begins it on the first move, updates it, ends it
    /// on the release.
    #[expect(clippy::too_many_arguments, reason = "one pass's state, passed once")]
    fn track_pass(
        &self,
        ctx: &Context,
        canvas: &Response,
        view: &CanvasView,
        style: &TransformStyle,
        d: &mut Drag,
        gesture: &mut Gesture<BoxState>,
        out: &mut TransformBoxResponse,
    ) {
        if d.cancelled {
            return;
        }
        let (pointer, down, input_mods) = ctx.input(|i| (i.pointer.latest_pos(), i.pointer.primary_down(), i.modifiers));
        let mods = self.mods(input_mods);
        if let Some(pointer) = pointer {
            d.core.last = *gesture.shown();
            let track = self.track_with(view, style, &mut d.core, pointer, mods, &mut || {
                gesture.begin();
                gesture.is_active()
            });
            if let Some(state) = track.state {
                gesture.update(state, mods);
            }
        }
        if down {
            return;
        }
        match self.release(style, &d.core, pointer, canvas.double_clicked()) {
            BoxRelease::Commit(_) => gesture.commit(),
            BoxRelease::Apply => out.apply = true,
            BoxRelease::Pivot(state) => gesture.set(state),
            BoxRelease::Nothing => {}
        }
    }

    /// Whether the canvas was double-clicked inside the box with no other overlay owning the press.
    fn double_clicked_inside(&self, ctx: &Context, canvas: &Response, view: &CanvasView, id: Id, style: &TransformStyle) -> bool {
        if !canvas.double_clicked() || Claim::owner(ctx, canvas.id).is_some_and(|owner| owner != id) {
            return false;
        }
        canvas.interact_pointer_pos().is_some_and(|p| self.double_click(view, style, p))
    }

    /// Whether a double-click at canvas point `point` is inside the box, away from its handles
    /// and the reference point, whatever [`inside_moves`](Self::inside_moves) and the modifiers
    /// make of a press there. The host applies the box on it, as [`show`](Self::show) does.
    pub fn double_click(&self, view: &CanvasView, style: &TransformStyle, point: Pos2) -> bool {
        raw_part(&on_screen(view, self.state), point, style.hit) == BoxPart::Inside
    }

    /// Enter, Esc and the arrows; `idle`: no press is held.
    fn keys(&self, ui: &Ui, idle: bool, picked: Option<BoxPart>, gesture: &mut Gesture<BoxState>, out: &mut TransformBoxResponse) {
        let ctx = ui.ctx();
        if !ui.is_enabled() || ctx.text_edit_focused() || popup_list::any_open(ctx) || !ctx.memory(|m| m.allows_interaction(ui.layer_id())) {
            return;
        }
        let (enter, escape) = ui.input_mut(|i| (i.consume_key(Modifiers::NONE, Key::Enter), i.consume_key(Modifiers::NONE, Key::Escape)));
        out.apply |= enter;
        out.discard |= escape && !enter;
        if !idle {
            return;
        }
        if let Some(by) = nudge::read(ui, nudge::STEP, nudge::BIG_STEP) {
            let part = picked.filter(|&p| self.rules.allows(p)).unwrap_or(BoxPart::Inside);
            gesture.set(self.nudged(part, by));
        }
    }

    /// The box after an arrow key moves `part` by `by`: as a plain drag of the part would.
    fn nudged(&self, part: BoxPart, by: Vec2) -> BoxState {
        let s = self.state;
        let at = match part {
            BoxPart::Corner(i) => s.quad[usize::from(i % 4)],
            BoxPart::Edge(i) => s.quad[usize::from(i % 4)].midpoint(s.quad[usize::from(i + 1) % 4]),
            BoxPart::Pivot => s.pivot,
            BoxPart::Inside | BoxPart::Outside => return s.moved(by),
        };
        quad::drag(s, s, part, at, at + by, Modifiers::NONE, &self.rules)
    }

    /// The keyboard parts of the box `shown` (mapped to the screen as `screen`): the whole box,
    /// the corners, the edges and the reference point, without those the rules refuse.
    fn parts(&self, screen: &BoxState, style: &TransformStyle, shown: &BoxState, picked: Option<BoxPart>) -> Vec<Part> {
        let reach = egui::Vec2::splat(2.0 * style.hit);
        let q = &shown.quad;
        let mut out = Vec::with_capacity(10);
        let bounds = egui::Rect::from_points(&screen.quad.map(pos));
        out.push(Part { key: part_key(BoxPart::Inside), name: self.names.whole.clone(), value: self.value.clone(), rect: bounds, selected: false });
        let handles = (0..4u8)
            .map(|i| (BoxPart::Corner(i), q[usize::from(i)], self.names.corners[usize::from(i)].clone()))
            .chain(
                (0..4u8).map(|i| (BoxPart::Edge(i), q[usize::from(i)].midpoint(q[usize::from(i + 1) % 4]), self.names.edges[usize::from(i)].clone())),
            )
            .chain([(BoxPart::Pivot, shown.pivot, self.names.pivot.clone())]);
        for (part, at, name) in handles {
            if !self.rules.allows(part) {
                continue;
            }
            let value = self.units.map(|(x, y)| format!("{}, {}", x.format_value(at.x, 0.0), y.format_value(at.y, 0.0)));
            let centre = pos(screen_point(screen, part).unwrap_or(Point::ZERO));
            out.push(Part { key: part_key(part), name, value, rect: egui::Rect::from_center_size(centre, reach), selected: false });
        }
        for p in &mut out {
            p.selected = picked.map(part_key) == Some(p.key);
        }
        out
    }

    /// The pointer for `part` at screen point `pointer`; `dragging`: during its drag.
    fn cursor(&self, ctx: &Context, view: &CanvasView, screen: &BoxState, part: BoxPart, pointer: Pos2, dragging: bool) {
        let style = TransformBox::style(&craft_ui::theme::current(ctx));
        if !self.rules.allows(part) {
            ctx.set_cursor_icon(CursorIcon::Default);
            return;
        }
        let rotating = match part {
            BoxPart::Outside => dragging || self.zone == RotateZone::Band || nearest_corner(screen, pointer).1 <= style.rotate_reach,
            BoxPart::Corner(_) | BoxPart::Edge(_) if self.rules.effective_mode() == TransformMode::Rotate => {
                dragging || nearest_corner(screen, pointer).1 <= style.rotate_reach
            }
            _ => false,
        };
        if rotating {
            turn_cursor(ctx, pointer, bisector(screen, nearest_corner(screen, pointer).0));
            return;
        }
        let q = &screen.quad;
        let mid = |k: usize| q[k % 4].midpoint(q[(k + 1) % 4]);
        let icon = match part {
            BoxPart::Corner(i) => {
                // Along the corner's bisector on screen, out of the box.
                let out = -bisector(screen, usize::from(i));
                resize_cursor(view, view.unmap_vec(out))
            }
            BoxPart::Edge(i) => {
                let i = usize::from(i);
                let across = mid(i) - mid(i + 2);
                resize_cursor(view, view.unmap_vec(egui::vec2(across.x as f32, across.y as f32)))
            }
            BoxPart::Pivot => CursorIcon::Crosshair,
            BoxPart::Inside => CursorIcon::Move,
            BoxPart::Outside => CursorIcon::Alias,
        };
        ctx.set_cursor_icon(icon);
    }

    /// The readout of a drag of `part`: the box `now` against the box at the press.
    fn show_readout(&self, ctx: &Context, id: Id, part: BoxPart, now: &BoxState, at_press: &BoxState, pointer: Pos2) {
        let (Some(labels), Some((ux, uy))) = (&self.readout, self.units) else { return };
        let numbers = &ux.numbers;
        let wrap = |unit: &Unit, v: f64| format!("{}{}{}", unit.prefix(), numbers.format(fixed(v, 1), 1..=1), unit.suffix());
        let rotating = part == BoxPart::Outside
            || (matches!(part, BoxPart::Corner(_) | BoxPart::Edge(_)) && self.rules.effective_mode() == TransformMode::Rotate);
        let rows: Vec<(String, String)> = if rotating {
            vec![(labels.angle.clone(), wrap(&labels.degrees, now.angle()))]
        } else {
            match part {
                BoxPart::Inside => {
                    let d = now.quad[0] - at_press.quad[0];
                    let len = |u: &RulerUnits, v: f64| {
                        let n = u.numbers.format(fixed(v * u.per_doc, u.decimals as i32), u.decimals..=u.decimals);
                        format!("{}{n}{}", u.unit.prefix(), u.unit.suffix())
                    };
                    vec![(labels.dx.clone(), len(ux, d.x)), (labels.dy.clone(), len(uy, d.y))]
                }
                BoxPart::Corner(_) | BoxPart::Edge(_) if now.is_parallelogram() => match self.start {
                    Some((rect, skew_v)) => {
                        let p = now.parts(rect, skew_v);
                        vec![(labels.w.clone(), wrap(&labels.percent, p.w * 100.0)), (labels.h.clone(), wrap(&labels.percent, p.h * 100.0))]
                    }
                    None => Vec::new(),
                },
                _ => Vec::new(),
            }
        };
        if rows.is_empty() {
            return;
        }
        rows.into_iter().fold(Readout::new(("craft_ui::canvas::TransformBox::readout", id)), |r, (l, v)| r.row(l, v)).show(ctx, pointer);
    }

    /// Draws the box line, the handles, the reference point and, after keyboard input, the focus
    /// round the picked part.
    fn paint(&self, ui: &Ui, view: &CanvasView, screen: &BoxState, style: &TransformStyle, picked: Option<BoxPart>, keyboard: bool) {
        let painter = ui.painter_at(view.rect);
        let corners = screen.quad.map(pos);
        painter.add(Shape::closed_line(corners.to_vec(), style.line));
        let ppp = ui.pixels_per_point();
        for i in 0..4u8 {
            for part in [BoxPart::Corner(i), BoxPart::Edge(i)] {
                if let Some(at) = screen_point(screen, part) {
                    painter.extend(Handle::shapes_with(&style.handle, HandleShape::Square, pos(at), ppp));
                }
            }
        }
        paint_pivot(&painter, pos(screen.pivot), style);
        if !keyboard {
            return;
        }
        match picked.filter(|&p| p != BoxPart::Inside && self.rules.allows(p)) {
            Some(BoxPart::Pivot) => {
                PartFocus::ring(&painter, egui::Rect::from_center_size(pos(screen.pivot), egui::Vec2::splat(2.0 * style.pivot_radius)))
            }
            Some(part) => {
                if let Some(at) = screen_point(screen, part) {
                    PartFocus::ring(&painter, egui::Rect::from_center_size(pos(at), egui::Vec2::splat(style.handle.size)));
                }
            }
            None => {
                let line = outset(&corners, style.line.width / 2.0 + style.focus_offset + style.focus.width / 2.0);
                painter.add(Shape::closed_line(line.to_vec(), style.focus_halo));
                painter.add(Shape::closed_line(line.to_vec(), style.focus));
            }
        }
    }
}

/// Draws the reference point at screen point `c`: a circle and a cross over their halo, as a
/// [`TransformBox`] draws its own. For a host showing a reference point outside a box; `style` is
/// [`TransformBox::style`].
pub fn paint_pivot(painter: &Painter, c: Pos2, style: &TransformStyle) {
    let (r, arm) = (style.pivot_radius, style.pivot_arm);
    for stroke in [style.pivot_halo, style.pivot] {
        painter.circle_stroke(c, r, stroke);
        painter.line_segment([c - vec2(arm, 0.0), c + vec2(arm, 0.0)], stroke);
        painter.line_segment([c - vec2(0.0, arm), c + vec2(0.0, arm)], stroke);
    }
}

fn drag_key(id: Id) -> Id {
    id.with("craft_ui::canvas::TransformBox::drag")
}

fn pick_key(id: Id) -> Id {
    id.with("craft_ui::canvas::TransformBox::picked")
}

/// The [`Part`] key of a box part.
pub(crate) fn part_key(part: BoxPart) -> u64 {
    match part {
        BoxPart::Inside | BoxPart::Outside => 0,
        BoxPart::Corner(i) => 1 + u64::from(i % 4),
        BoxPart::Edge(i) => 5 + u64::from(i % 4),
        BoxPart::Pivot => 9,
    }
}

pub(crate) fn key_part(key: u64) -> Option<BoxPart> {
    match key {
        0 => Some(BoxPart::Inside),
        1..=4 => Some(BoxPart::Corner((key - 1) as u8)),
        5..=8 => Some(BoxPart::Edge((key - 5) as u8)),
        9 => Some(BoxPart::Pivot),
        _ => None,
    }
}

/// The box as `view` draws it, in screen points.
fn on_screen(view: &CanvasView, s: &BoxState) -> BoxState {
    let a = view.affine();
    BoxState { quad: s.quad.map(|p| a * p), pivot: a * s.pivot }
}

pub(crate) fn pos(p: Point) -> Pos2 {
    Pos2::new(p.x as f32, p.y as f32)
}

/// The handle's or reference point's screen position; `None` for the inside and the outside.
fn screen_point(screen: &BoxState, part: BoxPart) -> Option<Point> {
    let q = &screen.quad;
    match part {
        BoxPart::Corner(i) => Some(q[usize::from(i % 4)]),
        BoxPart::Edge(i) => Some(q[usize::from(i % 4)].midpoint(q[usize::from(i + 1) % 4])),
        BoxPart::Pivot => Some(screen.pivot),
        BoxPart::Inside | BoxPart::Outside => None,
    }
}

/// The part under screen point `p`: the nearest handle or the reference point within `hit`, else
/// inside or outside.
fn raw_part(screen: &BoxState, p: Pos2, hit: f32) -> BoxPart {
    screen.part_at(Point::new(f64::from(p.x), f64::from(p.y)), f64::from(hit))
}

/// The corner nearest `p` and its distance.
fn nearest_corner(screen: &BoxState, p: Pos2) -> (usize, f32) {
    let p = Point::new(f64::from(p.x), f64::from(p.y));
    (0..4).map(|i| (i, screen.quad[i].distance(p) as f32)).min_by(|a, b| a.1.total_cmp(&b.1)).unwrap_or((0, f32::INFINITY))
}

/// Whether `p` is in the rotate band: outside the box, `band_inner` to `band_outer` from a corner.
fn in_band(screen: &BoxState, p: Pos2, style: &TransformStyle) -> bool {
    let d = nearest_corner(screen, p).1;
    !screen.contains(Point::new(f64::from(p.x), f64::from(p.y))) && (style.band_inner..=style.band_outer).contains(&d)
}

/// Corner `i`'s bisector on screen, into the box; zero for a collapsed corner.
fn bisector(screen: &BoxState, i: usize) -> egui::Vec2 {
    let q = &screen.quad;
    let unit = |to: Point| {
        let d = to - q[i];
        let l = d.hypot();
        if l.is_finite() && l > 1e-9 { d / l } else { Vec2::ZERO }
    };
    let s = unit(q[(i + 1) % 4]) + unit(q[(i + 3) % 4]);
    let l = s.hypot();
    if l.is_finite() && l > 1e-6 { egui::vec2((s.x / l) as f32, (s.y / l) as f32) } else { egui::Vec2::ZERO }
}

/// The closed polygon `corners` moved `by` points outward, each corner along its mitre (kept
/// within four times `by` at a sharp corner).
pub(crate) fn outset(corners: &[Pos2; 4], by: f32) -> [Pos2; 4] {
    let area: f32 = (0..4).map(|i| corners[i].x * corners[(i + 1) % 4].y - corners[(i + 1) % 4].x * corners[i].y).sum();
    let side = if area >= 0.0 { 1.0 } else { -1.0 };
    let normal = |a: Pos2, b: Pos2| {
        let d = (b - a).normalized();
        vec2(d.y, -d.x) * side
    };
    std::array::from_fn(|i| {
        let (prev, here, next) = (corners[(i + 3) % 4], corners[i], corners[(i + 1) % 4]);
        let (n1, n2) = (normal(prev, here), normal(here, next));
        let sum = n1 + n2;
        let k = 1.0 + n1.dot(n2);
        let mitre = if k > 1e-3 && sum.is_finite() { sum * (by / k) } else { n1 * by };
        let mitre = if mitre.length() > 4.0 * by { mitre.normalized() * 4.0 * by } else { mitre };
        here + mitre
    })
}

/// `v` rounded to `places` decimals, with no negative zero (a hair below zero never reads "-0").
pub(crate) fn fixed(v: f64, places: i32) -> f64 {
    let k = 10f64.powi(places);
    let v = (v * k).round() / k;
    if v == 0.0 { 0.0 } else { v }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Transform box"];

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum ModeArg {
    #[choice(name = "Free")]
    Free,
    #[choice(name = "Rotate")]
    Rotate,
    #[choice(name = "Skew")]
    Skew,
    #[choice(name = "Distort")]
    Distort,
    #[choice(name = "Perspective")]
    Perspective,
}

impl ModeArg {
    fn mode(self) -> TransformMode {
        match self {
            ModeArg::Free => TransformMode::Free,
            ModeArg::Rotate => TransformMode::Rotate,
            ModeArg::Skew => TransformMode::Skew,
            ModeArg::Distort => TransformMode::Distort,
            ModeArg::Perspective => TransformMode::Perspective,
        }
    }
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum ScaleArg {
    #[choice(name = "Proportional")]
    Proportional,
    #[choice(name = "Free")]
    Free,
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum ZoneArg {
    #[choice(name = "Outside")]
    Outside,
    #[choice(name = "Band")]
    Band,
}

#[derive(craft_ui::story::StoryArgs, Debug, PartialEq)]
struct FreeTransformArgs {
    #[arg(flatten)]
    view: ViewArgs,
    mode: ModeArg,
    scale: ScaleArg,
    zone: ZoneArg,
    readout: bool,
}

impl Default for FreeTransformArgs {
    fn default() -> Self {
        Self { view: ViewArgs::default(), mode: ModeArg::Free, scale: ScaleArg::Proportional, zone: ZoneArg::Outside, readout: true }
    }
}

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "primitives/transform-box/free-transform",
            Level::Primitives,
            "Transform box",
            "Free transform",
            FreeTransformArgs::default(),
            show_transform_box
        )
        .on(Surface::Canvas),
    ]
}

/// The English names the tests use.
#[cfg(test)]
pub(crate) fn english_names() -> TransformNames {
    TransformNames::english()
}

/// `state` with its corners on whole units, when it is an upright rectangle of at least one unit
/// each way; otherwise as it is. The reference point stays where it is.
fn on_pixel_grid(state: BoxState) -> BoxState {
    let q = state.quad;
    let near = |a: f64, b: f64| (a - b).abs() < 1e-6;
    let upright = near(q[0].y, q[1].y) && near(q[2].y, q[3].y) && near(q[0].x, q[3].x) && near(q[1].x, q[2].x);
    let rounded = q.map(|p| Point::new(p.x.round(), p.y.round()));
    let ok = rounded.iter().all(|p| p.x.is_finite() && p.y.is_finite())
        && (rounded[1].x - rounded[0].x).abs() >= 1.0
        && (rounded[3].y - rounded[0].y).abs() >= 1.0;
    if upright && ok { BoxState { quad: rounded, ..state } } else { state }
}

/// A host's open box, as the stories and tests keep it: updates shown as they come.
#[derive(Clone, Copy, Debug)]
pub(crate) struct BoxHost {
    pub state: BoxState,
    /// The box when a drag began, to restore on a cancel.
    pub before: Option<BoxState>,
}

impl BoxHost {
    pub(crate) fn new(state: BoxState) -> Self {
        Self { state, before: None }
    }

    pub(crate) fn apply(&mut self, response: &TransformBoxResponse) {
        match &response.event {
            Some(EditEvent::Begin { .. }) => self.before = Some(self.state),
            Some(EditEvent::Update { value, .. }) => self.state = *value,
            Some(EditEvent::Commit { value, .. } | EditEvent::Set { value, .. }) => {
                self.before = None;
                self.state = *value;
            }
            Some(EditEvent::Cancel { .. }) => {
                if let Some(before) = self.before.take() {
                    self.state = before;
                }
            }
            Some(_) | None => {}
        }
    }
}

/// The story's picture before any transform.
const PICTURE: Rect = Rect::new(200.0, 120.0, 440.0, 280.0);

/// A picture under a Free Transform box: Enter, a click outside or a double-click inside applies
/// it (the picture stays and a new box opens on a fresh one), Esc puts it back.
fn show_transform_box(ui: &mut Ui, args: &mut FreeTransformArgs) {
    let key = ui.scope_id().with("transform-box-story");
    let (mut host, mut applied): (BoxHost, Vec<[Point; 4]>) =
        ui.data(|d| d.get_temp(key)).unwrap_or_else(|| (BoxHost::new(BoxState::from_rect(PICTURE)), Vec::new()));
    let mode = args.mode.mode();
    let scale = if args.scale == ScaleArg::Free { ScaleRule::Free } else { ScaleRule::Proportional };
    let zone = if args.zone == ZoneArg::Band { RotateZone::Band } else { RotateZone::Outside };
    let rules = BoxRules { mode, scale, rotate_step: if scale == ScaleRule::Free { 45.0 } else { 15.0 }, ..BoxRules::default() };
    let canvas = craft_ui::testing::canvas_story::document(ui, args.view, "Document");
    let view = &canvas.view;
    let painter = ui.painter_at(view.rect);
    let picture = |quad: &[Point; 4], alpha: u8| {
        let fill = Color32::from_rgba_unmultiplied(70, 130, 180, alpha);
        Shape::convex_polygon(quad.iter().map(|&p| view.to_screen(p)).collect(), fill, Stroke::new(1.0, Color32::from_gray(40)))
    };
    for quad in &applied {
        painter.add(picture(quad, 140));
    }
    painter.add(picture(&host.state.quad, 220));
    let px = RulerUnits::pixels(craft_ui::localized::LocaleNumbers::from_tag("en").ungrouped());
    let mut b = TransformBox::new("story-transform-box", &host.state, TransformNames::of(ui))
        .rules(rules)
        .rotate_zone(zone)
        .units(&px, &px)
        .start(PICTURE, 0.0);
    if args.readout {
        b = b.readout(TransformLabels::new("Angle:", "W:", "H:", "ΔX:", "ΔY:"));
    }
    let r = b.show(ui, &canvas.response, view);
    host.apply(&r);
    if r.apply {
        applied.push(host.state.quad);
        host = BoxHost::new(BoxState::from_rect(PICTURE));
    } else if r.discard {
        host = BoxHost::new(BoxState::from_rect(PICTURE));
    }
    ui.data_mut(|d| d.insert_temp(key, (host, applied)));
}

#[cfg(test)]
#[path = "transform_box_tests.rs"]
mod tests;
