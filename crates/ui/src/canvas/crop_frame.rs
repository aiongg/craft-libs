//! [`CropFrame`]: the Crop tool's frame, shield, composition guides and Classic-mode gestures.

use egui::{Color32, Context, CursorIcon, Id, IdSalt, Key, Mesh, Modifiers, Painter, Pos2, Response, Shape, Stroke, Ui, pos2};
use kurbo::{Point, Rect, Vec2};

use super::claim::PrimaryPress;
use super::quad::BoxPart;
use super::transform_box::{fixed, key_part, outset, part_key, pos};
use super::{CanvasView, Claim, CropGuide, Part, PartFocus, Readout, RulerUnits, crisp, resize_cursor, turn_cursor};
use crate::behavior::nudge;
use crate::edit::order::Turn;
use crate::edit::{CancelReason, EditEvent, EditPolicy, Gesture};
use crate::localized::Unit;
use crate::popup_list;
use crate::primitives::{FocusRing, Handle, HandleShape};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens};

mod geometry;
mod image_mode;
pub(crate) mod story;
mod straighten;

pub use geometry::CropBox;
use geometry::{Hit, direction, drawn, handle_part, hit, normalized_angle, part_handle, resized_turned, turn, turned_angle, within};

/// The AccessKit names of a frame and its parts, the host's translated text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CropNames {
    /// The group, e.g. "Crop frame".
    pub group: String,
    /// The whole frame as a keyboard part, e.g. "Whole frame".
    pub whole: String,
    /// The corners clockwise from the top left of the upright frame: "Top left corner" …
    pub corners: [String; 4],
    /// The edges clockwise from the top: "Top edge" …
    pub edges: [String; 4],
}

/// The labels of the readout beside the pointer during a gesture, the host's translated text
/// (PhotoCraft's "W:", "H:", "Angle:").
#[derive(Clone, Debug)]
pub struct CropLabels {
    pub w: String,
    pub h: String,
    pub angle: String,
    /// Around the angle: `{n}°`.
    pub degrees: Unit,
}

impl CropLabels {
    /// The labels with `{n}°` round the angle.
    pub fn new(w: impl Into<String>, h: impl Into<String>, angle: impl Into<String>) -> Self {
        Self { w: w.into(), h: h.into(), angle: angle.into(), degrees: Unit::new("{n}°") }
    }
}

/// A frame's look and reach, from [`CropFrame::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct CropStyle {
    /// The frame line: `crop.line` of `overlay.crop`, bare.
    pub frame: Stroke,
    /// A composition guide line: `crop.guide_line` of `overlay.crop_guide`, bare.
    pub guide: Stroke,
    /// Corner brackets and edge bars: `crop.bracket` of `overlay.crop`.
    pub bracket: Stroke,
    /// Under the brackets and bars: `overlay.halo`, `handle.halo` wider on each side; no width
    /// outside high contrast.
    pub halo: Stroke,
    /// Reach of the edges and corners in screen points: `crop.hit`.
    pub hit: f32,
    /// Corner bracket arm: `crop.bracket_arm`.
    pub bracket_arm: f32,
    /// Divisor of a side giving an edge bar's length: `crop.bar_share`.
    pub bar_share: f32,
    /// The Grid guide's target cell side: `crop.grid_step`.
    pub grid_step: f32,
    /// Most Grid guide lines per axis: `crop.grid_max`.
    pub grid_max: usize,
    /// The focus line round the whole frame: the focus ring's stroke.
    pub focus: Stroke,
    /// Under it: `overlay.halo`, `handle.halo` wider on each side.
    pub focus_halo: Stroke,
    /// From the frame line to the focus line: the focus ring's offset.
    pub focus_offset: f32,
    /// The Straighten line: `crop.straighten_line` of `overlay.stroke`.
    pub straighten: Stroke,
    /// Under the Straighten line: `crop.straighten_halo` of `overlay.halo`.
    pub straighten_halo: Stroke,
    /// The shortest Straighten line, in screen points: `crop.straighten_min`.
    pub straighten_min: f32,
}

/// What [`CropFrame::show`] reports.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct CropResponse {
    /// The frame's id, for [`CropFrame::focus`].
    pub id: Id,
    /// A drag (Begin on the first move, Update, Commit on release; Esc cancels it, and so does a
    /// release that leaves a side under one unit) or an arrow key's or X's Set.
    pub event: Option<EditEvent<CropBox>>,
    /// Apply the crop: Enter, a double-click inside the frame (not on a handle, not on the
    /// untouched default frame).
    pub apply: bool,
    /// Close the frame without applying it: Esc while no drag is on.
    pub discard: bool,
    /// The part the keys act on ([`BoxPart::Inside`]: the whole frame), picked by the pointer or
    /// the Page keys; `None` before either.
    pub picked: Option<BoxPart>,
    /// What a press at the pointer would take: a corner, an edge, [`BoxPart::Inside`] or
    /// [`BoxPart::Outside`] (turns the frame); `None` while the pointer is off the canvas or
    /// another overlay owns the press.
    pub hovered: Option<BoxPart>,
    /// Whether the frame owns the current press: the host skips its tool input.
    pub claimed: bool,
    /// Whether a drag is running: the host's input for [`GuideShow::shows`](super::GuideShow::shows)
    /// and the shield's editing opacity on the next frame.
    pub dragging: bool,
    /// The frame's angle while it is being turned.
    pub turning: Option<f64>,
    /// The size Enter would crop to while the frame is drawn or resized.
    pub sizing: Option<(f64, f64)>,
    /// The angle a released Straighten line gives ([`CropFrame::straighten`]): the frame angle
    /// that levels the line, or plumbs it when it is nearer vertical. The host applies it, with
    /// [`CropBox::fitted`], as one box step, and disarms Straighten. The drag itself sends no
    /// edit event.
    /// An Image-mode host calls [`CropFrame::sync_view`] after applying this, so the frame shows
    /// upright over the turned image.
    pub straightened: Option<f64>,
}

/// The host's snapping of a dragged point ([`CropFrame::snap`]).
type Snap<'a> = Box<dyn Fn(BoxPart, Point, Modifiers) -> Point + 'a>;

/// What a drag does to the frame.
#[derive(Clone, Copy, Debug)]
enum Kind {
    /// A new frame from `anchor`; `cur` is the opposite corner, `last` the pointer of the last
    /// pass (Space moves the frame by the pointer's step).
    Draw { anchor: Point, cur: Point, last: Point },
    /// Moves the frame on the page.
    Move,
    /// Drags the edge or corner (`hx`, `hy`), along the frame's own axes.
    Resize { hx: i8, hy: i8 },
    /// Turns the frame about its centre; `grab` is the pointer's direction from it at the press.
    Turn { grab: f64 },
}

impl Kind {
    /// The part the host's snap is told of.
    fn part(self) -> BoxPart {
        match self {
            Kind::Draw { .. } => BoxPart::Corner(2),
            Kind::Move => BoxPart::Inside,
            Kind::Resize { hx, hy } => handle_part(hx, hy),
            Kind::Turn { .. } => BoxPart::Outside,
        }
    }
}

/// A drag in progress, kept between passes.
#[derive(Clone, Copy, Debug)]
struct Drag {
    kind: Kind,
    /// The host's frame at the press.
    at_press: CropBox,
    /// The press in document space.
    from: Point,
    press: PrimaryPress,
    began: bool,
    /// Esc, an outside change or a missed pass ended the gesture; the rest of the press does
    /// nothing.
    cancelled: bool,
    /// The press waits for another control's typing run to end ([`Claim::turn`]); nothing of it
    /// has run.
    waits: bool,
    /// Image mode: the view at the press. Every pointer point of the drag is read in it, because
    /// the view moves and turns with the image while the drag runs; Esc puts it back.
    press_view: Option<CanvasView>,
}

/// The Crop tool's frame over the host's document: a shield outside it, the frame line, corner
/// brackets and edge bars, and composition guides, dragged by the pointer as Photoshop's Classic
/// mode has it, moved and picked from the keyboard.
///
/// ```
/// use craft_ui::canvas::{CanvasView, CropBox, CropFrame, CropNames, kurbo::{Point, Rect}};
/// # egui::__run_test_ui(|ui| {
/// let (rect, canvas) = ui.allocate_exact_size(egui::vec2(400.0, 300.0), egui::Sense::click_and_drag());
/// let view = CanvasView::new(rect, Point::new(200.0, 150.0), 1.0);
/// let state = CropBox::upright(Rect::new(100.0, 80.0, 300.0, 220.0));
/// let names = CropNames {
///     group: "Crop frame".into(),
///     whole: "Whole frame".into(),
///     corners: ["Top left corner", "Top right corner", "Bottom right corner", "Bottom left corner"].map(String::from),
///     edges: ["Top edge", "Right edge", "Bottom edge", "Left edge"].map(String::from),
/// };
/// let r = CropFrame::new("crop", &state, names).show(ui, &canvas, &view);
/// if !r.claimed {
///     // the tool's input
/// }
/// # });
/// ```
///
/// - The host owns the frame ([`CropBox`]): it applies the events to it, passes it back each
///   frame, crops on [`apply`](CropResponse::apply) and drops the frame on
///   [`discard`](CropResponse::discard). It resolves the shield fill
///   ([`CropShield::fill`](super::CropShield::fill)) and the guide to show
///   ([`GuideShow::shows`](super::GuideShow::shows)) and passes them in.
/// - Pointer: the frame takes every press on the canvas that no overlay shown before it took.
///   The nearest edge or corner within `crop.hit` on screen resizes along the frame's own axes
///   (⇧ keeps the proportions, ⌥ resizes about the centre); inside moves; outside turns the
///   frame about its centre (⇧ in `turn_step` steps). Inside the [default frame](Self::default_frame),
///   a drag draws a new frame (⇧ square, ⌥ from the centre, Space moves it while it is drawn).
///   A [ratio](Self::ratio) is held by every gesture. A drag begins on the first move; a press
///   released without moving changes nothing, and a drag that leaves a side under one document
///   unit is cancelled.
/// - Keys, whenever no text field has focus and no popup is open: Enter applies; Esc cancels a
///   drag, ends a held press before its first move and otherwise discards; the arrows move the
///   whole frame on the page, or, while the frame has keyboard focus, the picked part as a plain
///   drag of it would; X swaps portrait and landscape about the centre. With the focus (one Tab
///   stop, [`PartFocus`]) Page Down and Page Up pick the next and previous part: the whole
///   frame, then its corners and edges clockwise from the top left.
/// - Nothing mirrors in a right-to-left interface: the frame is the document's.
pub struct CropFrame<'a> {
    id_salt: IdSalt,
    state: &'a CropBox,
    names: CropNames,
    shield: Option<Color32>,
    guide: Option<(CropGuide, u8)>,
    ratio: Option<f64>,
    default_frame: bool,
    turn_step: f64,
    snap: Option<Snap<'a>>,
    readout: Option<CropLabels>,
    straighten: bool,
    bounds: Option<Rect>,
    units: Option<(&'a RulerUnits, &'a RulerUnits)>,
    value: Option<String>,
    policy: EditPolicy,
    modifiers: Option<Modifiers>,
    image: bool,
    auto_center: bool,
}

impl<'a> CropFrame<'a> {
    /// The host's frame `state`, named `names` in the AccessKit tree.
    pub fn new(id_salt: impl egui::AsIdSalt, state: &'a CropBox, names: CropNames) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            state,
            names,
            shield: None,
            guide: None,
            ratio: None,
            default_frame: false,
            turn_step: 15.0,
            snap: None,
            readout: None,
            straighten: false,
            bounds: None,
            units: None,
            value: None,
            policy: EditPolicy::default(),
            modifiers: None,
            image: false,
            auto_center: false,
        }
    }

    /// The fill outside the frame ([`CropShield::fill`](super::CropShield::fill)); none draws no
    /// shield.
    pub fn shield(mut self, fill: Option<Color32>) -> Self {
        self.shield = fill;
        self
    }

    /// The composition guide and its orientation to draw now; the host applies the show rule
    /// with [`GuideShow::shows`](super::GuideShow::shows).
    pub fn guide(mut self, guide: Option<(CropGuide, u8)>) -> Self {
        self.guide = guide;
        self
    }

    /// The width over height every gesture holds (the options bar's preset).
    pub fn ratio(mut self, ratio: Option<f64>) -> Self {
        self.ratio = ratio.filter(|k| k.is_finite() && *k > 0.0);
        self
    }

    /// The frame is the untouched default (the canvas or the selection's bounds): a drag inside
    /// it draws a new frame, and a double-click inside does not apply.
    pub fn default_frame(mut self, default: bool) -> Self {
        self.default_frame = default;
        self
    }

    /// The step of a ⇧ turn, in degrees; 15 by default.
    pub fn turn_step(mut self, degrees: f64) -> Self {
        self.turn_step = degrees;
        self
    }

    /// The host's snapping: where a drag takes the pointer's document point, with the modifiers
    /// held, told which part it drags (a draw drags the bottom right corner; a move, the whole
    /// frame). In Image mode the point is the document point under the pointer in the view as it
    /// is now (the view moves with the image), as in Classic mode; a move is told the point of the
    /// frame that was grabbed, where the frame would have it.
    pub fn snap(mut self, snap: impl Fn(BoxPart, Point, Modifiers) -> Point + 'a) -> Self {
        self.snap = Some(Box::new(snap));
        self
    }

    /// Shows the readout beside the pointer during a drag (Show Transformation Values): W and H
    /// while drawing or resizing, the angle while turning. Needs [`units`](Self::units) for its
    /// number format.
    pub fn readout(mut self, labels: CropLabels) -> Self {
        self.readout = Some(labels);
        self
    }

    /// Straighten is armed (the options bar's button for one line, or ⌘ held; the host reads the
    /// modifiers and resets the one-shot): a drag on the canvas draws a line instead of a frame
    /// gesture, and its release gives [`CropResponse::straightened`] when the line is at least
    /// `crop.straighten_min` screen points long. The pointer is a crosshair; with a
    /// [readout](Self::readout) the line's angle shows beside it.
    pub fn straighten(mut self, armed: bool) -> Self {
        self.straighten = armed;
        self
    }

    /// The area the frame may not leave; none by default (PhotoCraft lets it leave the canvas).
    /// An unturned frame is cut to it; a turned frame stops where its bounds would leave.
    pub fn bounds(mut self, bounds: Rect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    /// The ruler units of the document's x and y axes: the parts' AccessKit values and the
    /// readout's numbers.
    pub fn units(mut self, x: &'a RulerUnits, y: &'a RulerUnits) -> Self {
        self.units = Some((x, y));
        self
    }

    /// The group's value for a screen reader; by default "W × H, X, Y, angle°" through the units.
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// The modifiers every test of this frame reads (the press, the drag, the host's snap) in
    /// place of the input state.
    pub fn modifiers(mut self, modifiers: Modifiers) -> Self {
        self.modifiers = Some(modifiers);
        self
    }

    /// Auto Center Preview (Image mode, [`show_image`](Self::show_image)): a frame being resized
    /// keeps its centre where it is on screen while the handle follows the pointer, and a drawn
    /// frame is moved to the middle of the canvas on release.
    pub fn auto_center(mut self, auto_center: bool) -> Self {
        self.auto_center = auto_center;
        self
    }

    pub fn style(tokens: &UiTokens) -> CropStyle {
        let (c, o) = (&tokens.component.crop, &tokens.color.overlay);
        let halo = Handle::style(tokens, HandleShape::Square).halo;
        let ring = FocusRing::style(tokens);
        let handle_halo = tokens.component.handle.halo;
        CropStyle {
            frame: Stroke::new(c.line, o.crop.to_egui()),
            guide: Stroke::new(c.guide_line, o.crop_guide.to_egui()),
            bracket: Stroke::new(c.bracket, o.crop.to_egui()),
            halo: Stroke::new(if halo.width > 0.0 { c.bracket + 2.0 * halo.width } else { 0.0 }, halo.color),
            hit: c.hit,
            bracket_arm: c.bracket_arm,
            bar_share: c.bar_share,
            grid_step: c.grid_step,
            grid_max: c.grid_max.max(0.0) as usize,
            focus: ring.stroke,
            focus_halo: Stroke::new(ring.stroke.width + 2.0 * handle_halo, o.halo.to_egui()),
            focus_offset: ring.offset,
            straighten: Stroke::new(c.straighten_line, o.stroke.to_egui()),
            straighten_halo: Stroke::new(c.straighten_halo, o.halo.to_egui()),
            straighten_min: c.straighten_min,
        }
    }

    /// Gives the frame `id` ([`CropResponse::id`]) the keyboard.
    pub fn focus(ctx: &Context, id: Id) {
        ctx.memory_mut(|m| m.request_focus(id));
    }

    /// Draws the frame over `view` and senses through the host's `canvas` response. Classic mode:
    /// the gestures move and turn the frame over the fixed image.
    pub fn show(self, ui: &mut Ui, canvas: &Response, view: &CanvasView) -> CropResponse {
        let mut view = *view;
        self.run(ui, canvas, &mut view)
    }

    fn run(self, ui: &mut Ui, canvas: &Response, view: &mut CanvasView) -> CropResponse {
        let ctx = ui.ctx().clone();
        let tokens = Tokens::of(ui);
        let style = CropFrame::style(&tokens);
        let id = ui.make_persistent_id(self.id_salt);
        let mut out = CropResponse {
            id,
            event: None,
            apply: false,
            discard: false,
            picked: None,
            hovered: None,
            claimed: false,
            dragging: false,
            turning: None,
            sizing: None,
            straightened: None,
        };
        if !self.state.is_finite() {
            return out;
        }
        let mut picked: Option<BoxPart> = ctx.data(|d| d.get_temp(pick_key(id)));

        // A drag whose press ended while the frame was not shown is dropped, and its gesture with it.
        let mut drag: Option<Drag> =
            ctx.data(|d| d.get_temp(drag_key(id))).filter(|d: &Drag| d.press.holds(&ctx) || (d.waits && Claim::is_owner(&ctx, canvas.id, id)));
        let waited = drag.is_some_and(|d| d.waits);
        if let Some(d) = &mut drag
            && d.waits
        {
            match Claim::turn(&ctx, canvas.id, id) {
                Turn::Waits => {}
                // A typing run that ended first may have changed the frame: the press works from it.
                Turn::Runs => {
                    d.waits = false;
                    d.at_press = *self.state;
                    d.press_view = self.image.then_some(*view);
                }
                Turn::Now => drag = None,
            }
        }
        let line = self.straighten_pass(ui, canvas, view, &style, id, drag.is_some());
        out.straightened = line.straightened;
        if drag.is_none() && !line.claimed && !self.straighten && !Claim::keeps_waiting(&ctx, canvas.id, id, waited) {
            drag = self.press(ui, canvas, view, &style, id);
            if let Some(d) = drag {
                picked = match d.kind {
                    Kind::Resize { hx, hy } => Some(handle_part(hx, hy)),
                    Kind::Move => Some(BoxPart::Inside),
                    Kind::Draw { .. } | Kind::Turn { .. } => picked,
                };
            }
        }
        let reported = Gesture::<CropBox>::reported(ui, id).map(|(_, value)| value);
        let host = match drag {
            Some(d) if d.began && !d.cancelled && (*self.state == d.at_press || Some(*self.state) == reported) => reported.unwrap_or(d.at_press),
            _ => *self.state,
        };
        let mut gesture = Gesture::load(ui, id, &host, self.policy).focus_owners(&[]).enabled(ui.is_enabled());
        if let Some(d) = &mut drag
            && d.began
            && !d.cancelled
            && !gesture.is_active()
        {
            d.cancelled = true;
            self.restore_view(d, view);
        }
        // Esc before the first move ends the press as it would end its drag.
        if let Some(d) = &mut drag
            && !d.began
            && !d.cancelled
            && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
        {
            d.cancelled = true;
        }
        let pointer = ctx.input(|i| i.pointer.latest_pos());
        if let Some(d) = &mut drag
            && !d.waits
        {
            self.track(&ctx, view, d, &mut gesture);
        }
        if self.double_clicked_inside(&ctx, canvas, view, id, style.hit) {
            out.apply = true;
        }

        let shown = *gesture.shown();
        let screen = on_screen(view, &shown);
        let parts = self.parts(screen, &style, &shown, picked);
        let mut focus = PartFocus::new(id, self.names.group.clone()).parts(&parts).picked(picked.map(part_key));
        let value = self.value.clone().or_else(|| self.group_value(&shown));
        if let Some(value) = value {
            focus = focus.value(value);
        }
        let focus = focus.show(ui, canvas);
        if let Some(pick) = focus.pick {
            picked = pick.and_then(key_part);
        }
        if drag.is_none_or(|d| d.cancelled) {
            // The pick is the keyboard's: without the focus the arrows move the whole frame (PC).
            let enter_ok = focus.focused || ctx.memory(|m| m.focused()).is_none_or(|f| f == canvas.id);
            self.keys(ui, view, drag.is_none() && !line.claimed, enter_ok, picked.filter(|_| focus.focused), &mut gesture, &mut out);
        }

        let dragging = drag.filter(|d| d.began && !d.cancelled);
        let other = Claim::owner(&ctx, canvas.id).is_some_and(|owner| owner != id);
        if let Some(d) = dragging
            && let Some(pointer) = pointer
        {
            self.cursor(&ctx, view, &shown, d.kind, pointer);
            self.show_readout(&ctx, id, d.kind, &shown, pointer);
            out.dragging = true;
            match d.kind {
                Kind::Turn { .. } => out.turning = Some(shown.angle),
                Kind::Draw { .. } | Kind::Resize { .. } => out.sizing = shown.crop_size(),
                Kind::Move => {}
            }
        } else if self.straighten && drag.is_none() && !other && canvas.hover_pos().is_some() {
            ctx.set_cursor_icon(CursorIcon::Crosshair);
        } else if drag.is_none()
            && !other
            && let Some(pointer) = canvas.hover_pos()
        {
            let over = self.over(view, pointer, &style);
            out.hovered = Some(match over {
                Hit::Handle(hx, hy) => handle_part(hx, hy),
                Hit::Inside => BoxPart::Inside,
                Hit::Outside => BoxPart::Outside,
            });
            self.hover_cursor(&ctx, view, over, pointer);
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
        self.paint(ui, view, screen, &style, picked, focus.keyboard);
        out
    }

    /// A press on the canvas: what it does to the frame, claimed for it.
    fn press(&self, ui: &Ui, canvas: &Response, view: &CanvasView, style: &CropStyle, id: Id) -> Option<Drag> {
        let ctx = ui.ctx();
        if !ui.is_enabled() || !ctx.input(|i| i.pointer.primary_pressed()) {
            return None;
        }
        let press = PrimaryPress::current(ctx)?;
        if !(view.rect.contains(press.origin) && canvas.contains_pointer()) {
            return None;
        }
        let from = view.to_doc(press.origin);
        let kind = match self.over(view, press.origin, style) {
            Hit::Handle(hx, hy) => Kind::Resize { hx, hy },
            Hit::Inside if !self.default_frame => Kind::Move,
            Hit::Outside => Kind::Turn { grab: direction(self.state.center(), from) },
            Hit::Inside => Kind::Draw { anchor: from, cur: from, last: from },
        };
        if !Claim::take(ctx, canvas.id, id) {
            return None;
        }
        let waits = Claim::turn(ctx, canvas.id, id) == Turn::Waits;
        Some(Drag { kind, at_press: *self.state, from, press, began: false, cancelled: false, waits, press_view: self.image.then_some(*view) })
    }

    /// What screen point `p` is over, in the frame's own axes.
    fn over(&self, view: &CanvasView, p: Pos2, style: &CropStyle) -> Hit {
        hit(self.state, view.to_doc(p), view.doc_len(style.hit))
    }

    /// One pass of a drag: begins it on the first move, updates it, ends it on the release.
    fn track(&self, ctx: &Context, view: &mut CanvasView, d: &mut Drag, gesture: &mut Gesture<CropBox>) {
        if d.cancelled {
            return;
        }
        let (pointer, down, input_mods, space) =
            ctx.input(|i| (i.pointer.latest_pos(), i.pointer.primary_down(), i.modifiers, i.key_down(Key::Space)));
        let mods = self.modifiers.unwrap_or(input_mods);
        if let Some(pointer) = pointer {
            if !d.began && pointer != d.press.origin {
                gesture.begin();
                d.began = gesture.is_active();
            }
            if d.began {
                let to = self.drag_point(d, view, pointer, mods);
                if let Some(next) = self.dragged(d, to, mods, space, *gesture.shown()) {
                    gesture.update(next, mods);
                    self.follow(d, view, &next);
                }
            }
        }
        if down || !d.began {
            return;
        }
        let shown = *gesture.shown();
        if shown.is_finite() && shown.size_ok() {
            gesture.commit();
            self.centre_drawn(d, view, &shown);
        } else {
            gesture.cancel(CancelReason::Dismissed);
            self.restore_view(d, view);
        }
    }

    /// The drag's document point for `pointer`, after the host's snap. In Image mode the point is in
    /// the view of the press, but the host is told the point in the current view (which moves with
    /// the image), and a Move is snapped as the frame's grab point, which the drag carries.
    fn drag_point(&self, d: &Drag, view: &CanvasView, pointer: Pos2, mods: Modifiers) -> Point {
        let Some(press) = d.press_view.as_ref() else {
            let to = view.to_doc(pointer);
            return self.snap.as_ref().map_or(to, |snap| snap(d.kind.part(), to, mods));
        };
        let to = press.to_doc(pointer);
        let Some(snap) = self.snap.as_ref() else { return to };
        if matches!(d.kind, Kind::Move) {
            let grab = d.from + (d.from - to);
            let snapped = snap(BoxPart::Inside, grab, mods);
            return if snapped == grab { to } else { d.from + (d.from - snapped) };
        }
        let now = view.to_doc(pointer);
        let snapped = snap(d.kind.part(), now, mods);
        if snapped == now { to } else { press.to_doc(view.to_screen(snapped)) }
    }

    /// The frame a drag gives with the pointer at document point `to`, kept inside the bounds;
    /// `None` when the frame is as it was (a draw not yet sized, or no room).
    fn dragged(&self, d: &mut Drag, to: Point, mods: Modifiers, space: bool, last_shown: CropBox) -> Option<CropBox> {
        let at = d.at_press;
        let (next, resizing, held) = match &mut d.kind {
            Kind::Draw { anchor, cur, last } => {
                if space {
                    // Space: the frame being drawn follows the pointer, keeping its size.
                    let step = to - *last;
                    *anchor += step;
                    *cur += step;
                } else {
                    *cur = to;
                }
                *last = to;
                if *anchor == *cur {
                    return None;
                }
                (CropBox::upright(drawn(*anchor, *cur, self.ratio, mods)), true, self.ratio.is_some() || mods.shift)
            }
            Kind::Move if self.image => (at.moved(d.from - to), false, false),
            Kind::Move => (at.moved(to - d.from), false, false),
            Kind::Resize { hx, hy } => {
                let r = at.rect.abs();
                let keep = (r.width() > 0.0 && r.height() > 0.0).then(|| r.width() / r.height());
                let ratio = self.ratio.or(if mods.shift { keep } else { None });
                let next = if self.image && self.auto_center {
                    image_mode::centred_resize(&at, *hx, *hy, to - d.from, ratio, mods.alt)?
                } else {
                    resized_turned(&at, *hx, *hy, to - d.from, ratio, mods.alt)
                };
                (next, true, ratio.is_some())
            }
            Kind::Turn { grab } => {
                let step = mods.shift.then_some(self.turn_step);
                let angle = if self.image {
                    image_mode::image_turned_angle(at.center(), *grab, at.angle, to, step)
                } else {
                    turned_angle(at.center(), *grab, at.angle, to, step)
                };
                (CropBox { rect: at.rect, angle }, true, false)
            }
        };
        if !next.is_finite() {
            return None;
        }
        match self.bounds {
            Some(bounds) => within(next, bounds, resizing, held).or(Some(last_shown)),
            None => Some(next),
        }
    }

    /// Whether the canvas was double-clicked inside the frame, off its handles, with no other
    /// overlay owning the press.
    fn double_clicked_inside(&self, ctx: &Context, canvas: &Response, view: &CanvasView, id: Id, hit_reach: f32) -> bool {
        if self.default_frame || !canvas.double_clicked() || Claim::owner(ctx, canvas.id).is_some_and(|owner| owner != id) {
            return false;
        }
        canvas.interact_pointer_pos().is_some_and(|p| hit(self.state, view.to_doc(p), view.doc_len(hit_reach)) == Hit::Inside)
    }

    /// Enter, Esc, the arrows and X; `idle`: no press is held; `enter_ok`: no other widget
    /// (a Tab-focused button) holds the keyboard, so Enter is the frame's.
    #[allow(clippy::too_many_arguments)]
    fn keys(
        &self,
        ui: &Ui,
        view: &mut CanvasView,
        idle: bool,
        enter_ok: bool,
        picked: Option<BoxPart>,
        gesture: &mut Gesture<CropBox>,
        out: &mut CropResponse,
    ) {
        let ctx = ui.ctx();
        if !ui.is_enabled() || ctx.text_edit_focused() || popup_list::any_open(ctx) || !ctx.memory(|m| m.allows_interaction(ui.layer_id())) {
            return;
        }
        let (enter, escape) = ui.input_mut(|i| (enter_ok && i.consume_key(Modifiers::NONE, Key::Enter), i.consume_key(Modifiers::NONE, Key::Escape)));
        out.apply |= enter;
        out.discard |= escape && !enter;
        if !idle {
            return;
        }
        let mut image_nudge = false;
        let mut swapped = false;
        let next = if let Some(by) = nudge::read(ui, nudge::STEP, nudge::BIG_STEP) {
            match picked.unwrap_or(BoxPart::Inside) {
                // The image moves under the frame: the frame goes the other way.
                BoxPart::Inside if self.image => {
                    image_nudge = true;
                    Some(self.state.moved(-image_mode::screen_step(view, by)))
                }
                part => Some(self.nudged(part, by)),
            }
        } else if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::X)) {
            swapped = true;
            Some(self.state.swapped())
        } else {
            None
        };
        if let Some(next) = next.filter(CropBox::is_finite) {
            let next = match self.bounds {
                Some(bounds) => within(next, bounds, next.rect.size() != self.state.rect.size(), self.ratio.is_some()).unwrap_or(*self.state),
                None => next,
            };
            if image_nudge {
                // The view follows the frame, so the frame stays put on screen.
                view.center += next.center() - self.state.center();
            }
            if swapped && self.image && self.auto_center {
                *view = Self::view_for(&next, view);
                view.center = next.center();
            }
            gesture.set(next);
        }
    }

    /// The frame after an arrow key moves `part` by `by`: as a plain drag of the part would.
    fn nudged(&self, part: BoxPart, by: Vec2) -> CropBox {
        match part_handle(part) {
            Some((hx, hy)) => resized_turned(self.state, hx, hy, by, self.ratio, false),
            None => self.state.moved(by),
        }
    }

    /// The group's default value: "W × H, X, Y, angle°" through the units.
    fn group_value(&self, shown: &CropBox) -> Option<String> {
        let (x, y) = self.units?;
        let r = shown.rect.abs();
        let degrees = self.readout.as_ref().map_or_else(|| Unit::new("{n}°"), |l| l.degrees.clone());
        let angle = format!("{}{}{}", degrees.prefix(), x.numbers.format(fixed(shown.angle, 1), 1..=1), degrees.suffix());
        Some(format!(
            "{} × {}, {}, {}, {angle}",
            x.format_value(r.width(), 0.0),
            y.format_value(r.height(), 0.0),
            x.format_value(r.x0, 0.0),
            y.format_value(r.y0, 0.0),
        ))
    }

    /// The keyboard parts of the frame `shown` (mapped to the screen as `screen`): the whole
    /// frame, the corners and the edges.
    fn parts(&self, q: [Point; 4], style: &CropStyle, shown: &CropBox, picked: Option<BoxPart>) -> Vec<Part> {
        let reach = egui::Vec2::splat(2.0 * style.hit);
        let doc = shown.corners();
        let mid = |a: Point, b: Point| a.midpoint(b);
        let mut out = Vec::with_capacity(9);
        let bounds = egui::Rect::from_points(&q.map(pos));
        out.push(Part {
            key: part_key(BoxPart::Inside),
            name: self.names.whole.clone(),
            value: self.group_value(shown),
            rect: bounds,
            selected: false,
        });
        let handles = (0..4u8)
            .map(|i| (BoxPart::Corner(i), q[usize::from(i)], doc[usize::from(i)], self.names.corners[usize::from(i)].clone()))
            .chain((0..4u8).map(|i| {
                let (a, b) = (usize::from(i), usize::from(i + 1) % 4);
                (BoxPart::Edge(i), mid(q[a], q[b]), mid(doc[a], doc[b]), self.names.edges[usize::from(i)].clone())
            }));
        for (part, on_screen, at, name) in handles {
            let value = self.units.map(|(x, y)| format!("{}, {}", x.format_value(at.x, 0.0), y.format_value(at.y, 0.0)));
            out.push(Part { key: part_key(part), name, value, rect: egui::Rect::from_center_size(pos(on_screen), reach), selected: false });
        }
        for p in &mut out {
            p.selected = picked.map(part_key) == Some(p.key);
        }
        out
    }

    /// The pointer while hovering over `over`.
    fn hover_cursor(&self, ctx: &Context, view: &CanvasView, over: Hit, pointer: Pos2) {
        match over {
            Hit::Handle(hx, hy) => ctx.set_cursor_icon(self.resize_icon(view, hx, hy)),
            Hit::Inside if self.default_frame => ctx.set_cursor_icon(CursorIcon::Crosshair),
            Hit::Inside => ctx.set_cursor_icon(CursorIcon::Move),
            Hit::Outside => self.turn_pointer(ctx, view, self.state, pointer),
        }
    }

    /// The pointer during a drag of `kind`.
    fn cursor(&self, ctx: &Context, view: &CanvasView, shown: &CropBox, kind: Kind, pointer: Pos2) {
        match kind {
            Kind::Draw { .. } => ctx.set_cursor_icon(CursorIcon::Crosshair),
            Kind::Move => ctx.set_cursor_icon(CursorIcon::Move),
            Kind::Resize { hx, hy } => ctx.set_cursor_icon(self.resize_icon_of(view, shown, hx, hy)),
            Kind::Turn { .. } => self.turn_pointer(ctx, view, shown, pointer),
        }
    }

    /// The resize arrow for handle (`hx`, `hy`): its outward direction turned with the frame, as
    /// the view shows it.
    fn resize_icon(&self, view: &CanvasView, hx: i8, hy: i8) -> CursorIcon {
        self.resize_icon_of(view, self.state, hx, hy)
    }

    fn resize_icon_of(&self, view: &CanvasView, frame: &CropBox, hx: i8, hy: i8) -> CursorIcon {
        let d = turn(Point::new(f64::from(hx), f64::from(hy)), Point::ZERO, frame.angle);
        resize_cursor(view, Vec2::new(d.x, d.y))
    }

    /// The turn pointer at screen point `pointer`.
    fn turn_pointer(&self, ctx: &Context, view: &CanvasView, frame: &CropBox, pointer: Pos2) {
        turn_cursor(ctx, pointer, turn_toward(view, frame, pointer));
    }

    /// The readout of a drag of `kind`: the frame's size as Enter would crop it, or its angle.
    fn show_readout(&self, ctx: &Context, id: Id, kind: Kind, now: &CropBox, pointer: Pos2) {
        let (Some(labels), Some((ux, _))) = (&self.readout, self.units) else { return };
        let numbers = &ux.numbers;
        let rows: Vec<(String, String)> = match kind {
            Kind::Turn { .. } => {
                let turn = if self.image { image_mode::image_turn(now.angle) } else { normalized_angle(now.angle) };
                let angle = format!("{}{}{}", labels.degrees.prefix(), numbers.format(fixed(turn, 1), 1..=1), labels.degrees.suffix());
                vec![(labels.angle.clone(), angle)]
            }
            Kind::Draw { .. } | Kind::Resize { .. } => match now.crop_size() {
                Some((w, h)) => {
                    let whole = |v: f64| format!("{}{}{}", ux.unit.prefix(), numbers.format(v, 0..=0), ux.unit.suffix());
                    vec![(labels.w.clone(), whole(w)), (labels.h.clone(), whole(h))]
                }
                None => Vec::new(),
            },
            Kind::Move => Vec::new(),
        };
        if rows.is_empty() {
            return;
        }
        rows.into_iter().fold(Readout::new(("craft_ui::canvas::CropFrame::readout", id)), |r, (l, v)| r.row(l, v)).show(ctx, pointer);
    }

    /// Draws the shield, the frame line, the guide, the corner brackets and edge bars, and, after
    /// keyboard input, the focus round the picked part.
    fn paint(&self, ui: &Ui, view: &CanvasView, screen: [Point; 4], style: &CropStyle, picked: Option<BoxPart>, keyboard: bool) {
        let painter = ui.painter_at(view.rect);
        let q = screen.map(pos);
        let (u, v) = (q[1] - q[0], q[3] - q[0]);
        let (lu, lv) = (u.length(), v.length());
        if !(lu.is_finite() && lv.is_finite()) || lu < 1e-3 || lv < 1e-3 {
            return;
        }
        let at = |a: f32, b: f32| q[0] + u * a + v * b;
        if let Some(dim) = self.shield {
            paint_shield(&painter, q, dim);
        }
        paint_frame_line(&painter, q, style.frame);
        if let Some((guide, orientation)) = self.guide {
            for line in guide.guides_with(orientation, lu, lv, style.grid_step, style.grid_max) {
                painter.add(Shape::line(line.iter().map(|p| at(p[0], p[1])).collect(), style.guide));
            }
        }
        let l = style.bracket_arm.min(lu / 3.0).min(lv / 3.0);
        let (du, dv) = (u / lu, v / lv);
        let mut bars: Vec<[Pos2; 2]> = Vec::with_capacity(16);
        for (c, su, sv) in [(q[0], 1.0, 1.0), (q[1], -1.0, 1.0), (q[3], 1.0, -1.0), (q[2], -1.0, -1.0)] {
            bars.push([c, c + du * (l * su)]);
            bars.push([c, c + dv * (l * sv)]);
        }
        let (lx, ly) = (l.min(lu / style.bar_share) / 2.0, l.min(lv / style.bar_share) / 2.0);
        for (c, e) in [(at(0.5, 0.0), du * lx), (at(0.5, 1.0), du * lx), (at(0.0, 0.5), dv * ly), (at(1.0, 0.5), dv * ly)] {
            bars.push([c - e, c + e]);
        }
        for stroke in [style.halo, style.bracket] {
            if stroke.width > 0.0 {
                for bar in &bars {
                    painter.line_segment(*bar, stroke);
                }
            }
        }
        if !keyboard {
            return;
        }
        match picked {
            Some(part @ (BoxPart::Corner(_) | BoxPart::Edge(_))) => {
                let (centre, size) = match part {
                    BoxPart::Corner(i) => {
                        // The bracket's own square: its arms run `l` along both edges.
                        let (c, su, sv) = [(q[0], 1.0, 1.0), (q[1], -1.0, 1.0), (q[2], -1.0, -1.0), (q[3], 1.0, -1.0)][usize::from(i % 4)];
                        (
                            c + (du * su + dv * sv) * (l / 2.0),
                            egui::Rect::from_points(&[c, c + du * (l * su) + dv * (l * sv)]).size().max_elem() + style.bracket.width,
                        )
                    }
                    BoxPart::Edge(i) => {
                        let (centre, half) = [(at(0.5, 0.0), lx), (at(1.0, 0.5), ly), (at(0.5, 1.0), lx), (at(0.0, 0.5), ly)][usize::from(i % 4)];
                        (centre, 2.0 * half + style.bracket.width)
                    }
                    _ => return,
                };
                PartFocus::ring(&painter, egui::Rect::from_center_size(centre, egui::Vec2::splat(size)));
            }
            _ => {
                let line = outset(&q, style.frame.width / 2.0 + style.focus_offset + style.focus.width / 2.0);
                painter.add(Shape::closed_line(line.to_vec(), style.focus_halo));
                painter.add(Shape::closed_line(line.to_vec(), style.focus));
            }
        }
    }
}

/// The screen direction the turn pointer's arc bends towards at screen point `pointer`: the frame
/// corner of the pointer's quadrant, along the diagonal of the frame's own axes. That gives four
/// pointers, one per corner, turned with the frame.
fn turn_toward(view: &CanvasView, frame: &CropBox, pointer: Pos2) -> egui::Vec2 {
    let (c, p) = (frame.center(), view.to_doc(pointer));
    let local = turn(p, c, -frame.angle);
    let (sx, sy) = (if local.x < c.x { 1.0 } else { -1.0 }, if local.y < c.y { 1.0 } else { -1.0 });
    let diagonal = std::f64::consts::FRAC_1_SQRT_2;
    let toward = turn(Point::new(sx * diagonal, sy * diagonal), Point::ZERO, frame.angle);
    view.map_vec(Vec2::new(toward.x, toward.y))
}

/// Whether the screen quad `q` has horizontal and vertical sides (an upright or quarter-turned
/// view of an unturned frame).
fn axis_aligned(q: [Pos2; 4]) -> bool {
    (0..4).all(|i| {
        let d = q[(i + 1) % 4] - q[i];
        d.x.abs() < 0.01 || d.y.abs() < 0.01
    })
}

/// The shield: the area round the frame `q`, filled with `dim`. An axis-aligned frame leaves four
/// rects to the clip; a turned one four trapezoids out to the frame grown past the whole view.
fn paint_shield(painter: &Painter, q: [Pos2; 4], dim: Color32) {
    let clip = painter.clip_rect();
    if axis_aligned(q) {
        let r = egui::Rect::from_points(&q);
        for band in [
            egui::Rect::from_min_max(clip.min, pos2(clip.max.x, r.min.y)),
            egui::Rect::from_min_max(pos2(clip.min.x, r.max.y), clip.max),
            egui::Rect::from_min_max(pos2(clip.min.x, r.min.y), pos2(r.min.x, r.max.y)),
            egui::Rect::from_min_max(pos2(r.max.x, r.min.y), pos2(clip.max.x, r.max.y)),
        ] {
            if band.is_positive() {
                painter.rect_filled(band, 0.0, dim);
            }
        }
        return;
    }
    let (u, v) = (q[1] - q[0], q[3] - q[0]);
    let at = |a: f32, b: f32| q[0] + u * a + v * b;
    let reach = clip.size().length() + (clip.center() - at(0.5, 0.5)).length();
    let (eu, ev) = (reach / u.length(), reach / v.length());
    let outer = [at(-eu, -ev), at(1.0 + eu, -ev), at(1.0 + eu, 1.0 + ev), at(-eu, 1.0 + ev)];
    let mut mesh = Mesh::default();
    for p in q.iter().chain(outer.iter()) {
        mesh.colored_vertex(*p, dim);
    }
    for i in 0..4u32 {
        let j = (i + 1) % 4;
        mesh.add_triangle(i, j, 4 + j);
        mesh.add_triangle(i, 4 + j, 4 + i);
    }
    painter.add(Shape::mesh(mesh));
}

/// The frame line: whole device pixels when the frame has horizontal and vertical sides, else an
/// antialiased closed line.
fn paint_frame_line(painter: &Painter, q: [Pos2; 4], stroke: Stroke) {
    if !axis_aligned(q) {
        painter.add(Shape::closed_line(q.to_vec(), stroke));
        return;
    }
    let r = egui::Rect::from_points(&q);
    let w = stroke.width;
    // The right and bottom lines lie a line width past their positions, so the sides are extended
    // to meet them.
    crisp(painter, r.left_top(), pos2(r.max.x + w, r.min.y), stroke);
    crisp(painter, r.left_bottom(), pos2(r.max.x + w, r.max.y), stroke);
    crisp(painter, r.left_top(), pos2(r.min.x, r.max.y + w), stroke);
    crisp(painter, r.right_top(), pos2(r.max.x, r.max.y + w), stroke);
}

/// The corners of the frame `b` as `view` draws them, top left of the upright frame first.
fn on_screen(view: &CanvasView, b: &CropBox) -> [Point; 4] {
    b.corners().map(|p| {
        let s = view.to_screen(p);
        Point::new(f64::from(s.x), f64::from(s.y))
    })
}

fn drag_key(id: Id) -> Id {
    id.with("craft_ui::canvas::CropFrame::drag")
}

fn pick_key(id: Id) -> Id {
    id.with("craft_ui::canvas::CropFrame::picked")
}

#[cfg(test)]
#[path = "crop_frame_tests.rs"]
mod tests;
