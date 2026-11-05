//! [`GripSet`]: per-object grips with cold, warm and hot states (CadCraft's grips).

use egui::{AsIdSalt, Context, Id, IdSalt, Key, Modifiers, Pos2, Rect, Response, Shape, Stroke, Ui, vec2};
use kurbo::{Point, Vec2};

use crate::behavior::nudge;
use crate::canvas::claim::PrimaryPress;
use crate::canvas::{CanvasView, Claim, Part, PartFocus, RulerUnits};
use crate::edit::order::Turn;
use crate::edit::{EditEvent, EditPolicy, Gesture};
use crate::localized::Labels;
use crate::popup_list;
use crate::theme::current;

use super::GripSetStyle;

/// Where a dynamic arrow's tip and tail sit along its direction, in halves of a side: the tail is
/// `ARROW_TAIL` behind the centre, the tip `ARROW_TIP` ahead (CadCraft's `1.6 gs` long, from
/// `-0.5` to `+0.8` of that length).
const ARROW_LENGTH: f32 = 1.6;
const ARROW_TAIL: f32 = 0.5;
const ARROW_TIP: f32 = 0.8;
/// An arrow's half width, in halves of a side.
const ARROW_HALF_WIDTH: f32 = 1.1;
/// A list triangle's half width, and its top and bottom below or above the centre, in halves of a
/// side and of that half width.
const TRIANGLE_HALF_WIDTH: f32 = 1.3;
const TRIANGLE_TOP: f32 = 0.6;
const TRIANGLE_BOTTOM: f32 = 0.9;
/// A direction shorter than this on screen points right.
const MIN_DIRECTION: f32 = 1e-6;

crate::words! {
    /// The AccessKit names of a [`GripSet`], English by default. `grip` takes `{n}` (1-based) and
    /// `{count}`.
    #[non_exhaustive]
    pub struct GripSetLabels {
        group: "gripSet.group" = "Grips";
        grip: "gripSet.grip" = "Grip {n} of {count}";
        more: "gripSet.more" = "More grips";
    }
}

/// How a grip is drawn and hit. Sizes come from `grip.size` (CadCraft's `gripsize` 5 is a half
/// side, so 10 pt across).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum GripShape {
    /// A square `grip.size` across: a vertex, an end point, a dynamic point grip.
    Square,
    /// A rectangle `grip.size` wide and `grip.midpoint_height` high: a polyline segment's
    /// midpoint.
    Midpoint,
    /// A triangle pointing along `direction` (document units; drawn as the view maps it): a
    /// dynamic linear grip, pointing the way its distance grows.
    Arrow { direction: Vec2 },
    /// A downward triangle: a dynamic list grip, which opens the host's list of values.
    Triangle,
}

/// The state a grip is drawn in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GripState {
    /// At rest: `overlay.grip`, or `overlay.grip_dyn` for a [dynamic](Grip::dynamic) grip.
    #[default]
    Cold,
    /// The pointer is within `grip.warm_pad` of its bounds (`grip.hit_pad` for an arrow or a
    /// triangle, CadCraft's dynamic grips): `overlay.grip_warm`.
    Warm,
    /// Picked for editing ([`Grip::hot`]): `overlay.grip_hot`.
    Hot,
}

/// One grip of a [`GripSet`].
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Grip {
    /// The host's key for the grip, stable across passes (an entity and a grip index).
    pub key: u64,
    /// Where it sits, in document units.
    pub at: Point,
    pub shape: GripShape,
    /// Whether the host has made it hot (CadCraft's ⇧-click list, the grip being edited).
    pub hot: bool,
    /// Whether it is a dynamic grip (CadCraft's arrows, triangles and dynamic points): its cold
    /// fill is `overlay.grip_dyn`.
    pub dynamic: bool,
}

impl Grip {
    /// A cold grip keyed `key` at `at`.
    pub fn new(key: u64, at: Point, shape: GripShape) -> Self {
        Self { key, at, shape, hot: false, dynamic: false }
    }

    /// Makes the grip a dynamic one.
    pub fn dynamic(mut self, dynamic: bool) -> Self {
        self.dynamic = dynamic;
        self
    }

    /// Makes the grip hot.
    pub fn hot(mut self, hot: bool) -> Self {
        self.hot = hot;
        self
    }
}

/// A click on a grip: the pointer's press and release without a drag, or Enter on the picked
/// grip.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct GripClick {
    pub key: u64,
    /// ⇧ was held (CadCraft: toggles the grip hot; a plain click starts the grip edit).
    pub shift: bool,
}

/// A grip moved by a drag or an arrow key: the value of the set's [`EditEvent`]s.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct GripDrag {
    pub key: u64,
    /// Where the grip was at the press, in document units.
    pub from: Point,
    /// Where the pointer (or the arrow keys) put it.
    pub to: Point,
}

/// What [`GripSet::show`] reports.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct GripSetResponse {
    /// The set's id.
    pub id: Id,
    /// A click on a grip.
    pub clicked: Option<GripClick>,
    /// A drag of a grip (`Begin` on the first move past the click slop, `Update`, `Commit`; Esc
    /// cancels) or one arrow press's `Set`.
    pub drag: Option<EditEvent<GripDrag>>,
    /// The grip under the pointer.
    pub hovered: Option<u64>,
    /// The grip the keys act on, picked by the pointer or the Page keys.
    pub picked: Option<u64>,
    /// Whether the set owns the current press: the host skips its tool input.
    pub claimed: bool,
}

/// The grips of the selected objects (CadCraft's): squares, midpoint rectangles, arrows and
/// triangles that are cold, warm under the pointer and hot when the host says so. The set draws
/// and hit-tests them and reports clicks and drags; what a grip does (stretch, move, a menu) is
/// the host's.
///
/// - Look: filled `overlay.grip`, `overlay.grip_warm` or `overlay.grip_hot` by [`GripState`],
///   outlined 1 pt (`grip.stroke`) in `overlay.grip_stroke`. Every grip is drawn.
/// - Pointer, through the host's canvas response and [`Claim`](crate::canvas::Claim) first: the
///   hit area is a grip's bounds expanded by `grip.hit_pad`; a press there takes the grip; a
///   release without a drag is a [`GripClick`] with ⇧ as held; a drag after the press reports
///   [`GripDrag`] events. Warm follows the pointer within the bounds expanded by `grip.warm_pad`
///   (`grip.hit_pad` for arrows and triangles).
/// - Keys (one Tab stop through [`PartFocus`](crate::canvas::PartFocus)): Page Down and Page Up
///   pick the next and previous grip, Home and End the first and last; the arrows move the picked
///   grip by [`steps`](Self::steps), one `Set` per press; Enter is a click, ⇧+Enter a ⇧-click.
/// - AccessKit: a group named [`GripSetLabels::group`]; the first [`ACCESS_LIMIT`](Self::ACCESS_LIMIT)
///   grips are parts named "Grip 3 of 12" with their position as the value (in the
///   [`units`](Self::units)) and `selected` when hot; past the limit the group's value adds
///   [`GripSetLabels::more`] and the rest are drawn but not reachable from the keyboard.
/// - Nothing mirrors in a right-to-left interface.
pub struct GripSet<'a> {
    id_salt: IdSalt,
    grips: &'a [Grip],
    labels: Option<GripSetLabels>,
    units: Option<(&'a RulerUnits, &'a RulerUnits)>,
    steps: (f64, f64),
    modifiers: Option<Modifiers>,
    policy: EditPolicy,
}

impl<'a> GripSet<'a> {
    /// The grips the AccessKit tree and the keyboard reach: CadCraft draws grips for at most 200
    /// objects.
    pub const ACCESS_LIMIT: usize = 200;

    /// The set of `grips`, in keyboard order.
    pub fn new(id_salt: impl AsIdSalt, grips: &'a [Grip]) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            grips,
            labels: None,
            units: None,
            steps: (nudge::STEP, nudge::BIG_STEP),
            modifiers: None,
            policy: EditPolicy::default(),
        }
    }

    /// The AccessKit names; the installed words by default.
    pub fn labels(mut self, labels: GripSetLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The ruler units of the document's x and y axes, for the grips' AccessKit values.
    pub fn units(mut self, x: &'a RulerUnits, y: &'a RulerUnits) -> Self {
        self.units = Some((x, y));
        self
    }

    /// The arrow keys' step and ⇧ step in document units; `nudge::STEP` and `nudge::BIG_STEP` by
    /// default.
    pub fn steps(mut self, step: f64, big_step: f64) -> Self {
        self.steps = (step, big_step);
        self
    }

    /// The modifiers every test reads in place of the input state (a touch host's sticky
    /// modifiers).
    pub fn modifiers(mut self, modifiers: Modifiers) -> Self {
        self.modifiers = Some(modifiers);
        self
    }

    /// How the drag reports; realtime and cancellable by default.
    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Draws the grips over `view` and senses through the host's `canvas` response.
    pub fn show(self, ui: &mut Ui, canvas: &Response, view: &CanvasView) -> GripSetResponse {
        // cadcraft: crates/ui-egui/src/canvas.rs:1053-1076 (cold, warm within 2, hot fill; 1 pt
        // #103080 outline); crates/ui-egui/src/canvas/grips.rs:71-75 `grip_rect` (2 × gripsize
        // square, midpoint half as high), :78-101 `grip_at` (hit = rect expanded by 3, first 200
        // objects); crates/ui-egui/src/dyngrips.rs:47-66 `outline` (arrow 1.6 × 1.1 gripsize along
        // its screen direction, list triangle 1.3 gripsize), :69-72 `hit`.
        // cadcraft: crates/ui-egui/src/canvas/grips.rs:5-27 `click_grip` (⇧ toggles hot).
        // new: keys and parts through `PartFocus`, as `SelectionFrame`; the look from
        // `GripSet::style`.
        let ctx = ui.ctx().clone();
        let style = GripSet::style(&current(&ctx));
        let id = ui.make_persistent_id(self.id_salt);
        let labels = self.labels.clone().unwrap_or_else(|| GripSetLabels::of(ui));
        let mut out = GripSetResponse { id, clicked: None, drag: None, hovered: None, picked: None, claimed: false };
        let at_of = |key: u64| self.grips.iter().find(|g| g.key == key).map(|g| g.at);
        let mut picked: Option<u64> = ctx.data(|d| d.get_temp(pick_key(id))).filter(|&k| at_of(k).is_some());

        // A press whose button went up while the set was not shown is dropped, and its gesture
        // with it, as is one whose grip is gone.
        let mut press: Option<Press> = ctx
            .data(|d| d.get_temp(press_key(id)))
            .filter(|p: &Press| (p.press.holds(&ctx) || (p.waits && Claim::is_owner(&ctx, canvas.id, id))) && at_of(p.key).is_some());
        let waited = press.is_some_and(|p| p.waits);
        if let Some(p) = &mut press
            && p.waits
        {
            match Claim::turn(&ctx, canvas.id, id) {
                Turn::Waits => {}
                Turn::Runs => {
                    p.waits = false;
                    p.from = at_of(p.key).unwrap_or(p.from);
                }
                Turn::Now => press = None,
            }
        }
        if press.is_none() && !Claim::keeps_waiting(&ctx, canvas.id, id, waited) {
            press = self.press_pass(ui, canvas, view, &style, id);
            if let Some(p) = press {
                picked = Some(p.key);
            }
        }

        // The grip the gesture is about: the pressed one during a press, else the picked one.
        let subject = press.map(|p| p.key).or(picked);
        let host = match (subject, press) {
            (Some(key), Some(p)) => GripDrag { key, from: p.from, to: at_of(key).unwrap_or(p.from) },
            (Some(key), None) => {
                let at = at_of(key).unwrap_or(Point::ZERO);
                GripDrag { key, from: at, to: at }
            }
            (None, _) => GripDrag { key: 0, from: Point::ZERO, to: Point::ZERO },
        };
        let mut gesture = Gesture::<GripDrag>::load(ui, id, &host, self.policy).focus_owners(&[]).enabled(ui.is_enabled());
        if let Some(p) = &mut press
            && p.began
            && !p.cancelled
            && !gesture.is_active()
        {
            p.cancelled = true;
        }
        // Esc before the first move ends the press as it would end its drag.
        if let Some(p) = &mut press
            && !p.began
            && !p.cancelled
            && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
        {
            p.cancelled = true;
        }
        if let Some(p) = &mut press
            && !p.waits
        {
            self.track_pass(&ctx, view, p, &mut gesture, &mut out);
        }

        // Where the grips are drawn: the dragged one where the gesture shows it.
        let shown = *gesture.shown();
        let dragging = press.filter(|p| p.began && !p.cancelled);
        let centres: Vec<Pos2> = self
            .grips
            .iter()
            .map(|g| view.to_screen(if dragging.is_some_and(|p| p.key == g.key) && shown.key == g.key { shown.to } else { g.at }))
            .collect();
        let bounds = |i: usize| self.bounds(&style, self.grips[i].shape, centres[i], view);

        let parts: Vec<Part> = self
            .grips
            .iter()
            .enumerate()
            .take(Self::ACCESS_LIMIT)
            .map(|(i, g)| {
                let value = self.units.map(|(x, y)| format!("{}, {}", x.format_value(g.at.x, 0.0), y.format_value(g.at.y, 0.0)));
                let name = labels.grip.replace("{n}", &(i + 1).to_string()).replace("{count}", &self.grips.len().to_string());
                Part { key: g.key, name, value, rect: bounds(i).expand(style.hit_pad), selected: g.hot }
            })
            .collect();
        let mut focus = PartFocus::new(id, labels.group.clone()).parts(&parts).picked(picked);
        if self.grips.len() > Self::ACCESS_LIMIT {
            focus = focus.value(labels.more.clone());
        }
        let focus = focus.show(ui, canvas);
        if let Some(pick) = focus.pick {
            picked = pick;
        }
        if press.is_none_or(|p| p.cancelled) {
            // The pick is the keyboard's: without the focus the keys have no grip.
            self.keys(ui, press.is_none(), picked.filter(|_| focus.focused), &at_of, &mut gesture, &mut out);
        }

        let other = Claim::owner(&ctx, canvas.id).is_some_and(|owner| owner != id);
        let hover = if press.is_none() && !other { canvas.hover_pos() } else { None };
        if let Some(pointer) = hover {
            out.hovered = (0..self.grips.len()).find(|&i| bounds(i).expand(style.hit_pad).contains(pointer)).map(|i| self.grips[i].key);
        }

        out.drag = gesture.finish();
        out.claimed = Claim::is_owner(&ctx, canvas.id, id);
        out.picked = picked;
        let held = press.filter(|p| p.waits || ctx.input(|i| i.pointer.primary_down()));
        ctx.data_mut(|data| {
            match held {
                Some(p) => {
                    data.insert_temp(press_key(id), p);
                }
                None => {
                    data.remove::<Press>(press_key(id));
                }
            }
            match picked {
                Some(k) => {
                    data.insert_temp(pick_key(id), k);
                }
                None => {
                    data.remove::<u64>(pick_key(id));
                }
            }
        });

        let painter = ui.painter_at(view.rect);
        let stroke = Stroke::new(style.stroke, style.outline);
        for (i, g) in self.grips.iter().enumerate() {
            if !view.rect.contains(centres[i]) {
                continue;
            }
            let state = Self::state(g, bounds(i), hover, &style);
            let fill = match state {
                GripState::Cold if g.dynamic => style.dynamic,
                GripState::Cold => style.cold,
                GripState::Warm => style.warm,
                GripState::Hot => style.hot,
            };
            painter.add(Shape::convex_polygon(Self::outline(g.shape, centres[i], style.size, style.midpoint_height, view), fill, stroke));
        }
        if focus.keyboard
            && let Some(i) = picked.and_then(|k| self.grips.iter().position(|g| g.key == k))
        {
            PartFocus::ring(&painter, bounds(i));
        }
        out
    }

    /// The screen outline of a grip of `shape` centred at `centre`, `size` across, for drawing and
    /// for the hit test (its bounds).
    pub(crate) fn outline(shape: GripShape, centre: Pos2, size: f32, midpoint_height: f32, view: &CanvasView) -> Vec<Pos2> {
        // cadcraft: crates/ui-egui/src/dyngrips.rs:47-66 `outline`; crates/ui-egui/src/canvas/grips.rs:71-75.
        // `size` is twice CadCraft's `gripsize`, so a half side is `size / 2`.
        let half = size / 2.0;
        match shape {
            GripShape::Square => corners(Rect::from_center_size(centre, vec2(size, size))),
            GripShape::Midpoint => corners(Rect::from_center_size(centre, vec2(size, midpoint_height))),
            GripShape::Arrow { direction } => {
                // The arrow points the way the distance grows, on screen.
                let ahead = view.map_vec(direction);
                let u = if ahead.length() > MIN_DIRECTION { ahead / ahead.length() } else { vec2(1.0, 0.0) };
                let n = vec2(-u.y, u.x);
                let (len, wing) = (half * ARROW_LENGTH, half * ARROW_HALF_WIDTH);
                vec![centre - u * (len * ARROW_TAIL) + n * wing, centre + u * (len * ARROW_TIP), centre - u * (len * ARROW_TAIL) - n * wing]
            }
            GripShape::Triangle => {
                let w = half * TRIANGLE_HALF_WIDTH;
                vec![centre + vec2(-w, -w * TRIANGLE_TOP), centre + vec2(w, -w * TRIANGLE_TOP), centre + vec2(0.0, w * TRIANGLE_BOTTOM)]
            }
        }
    }

    /// The bounds of a grip of `shape` at screen point `centre`.
    fn bounds(&self, style: &GripSetStyle, shape: GripShape, centre: Pos2, view: &CanvasView) -> Rect {
        Rect::from_points(&Self::outline(shape, centre, style.size, style.midpoint_height, view))
    }

    /// The state `grip` is drawn in, with `bounds` its screen bounds and `hover` the pointer: hot
    /// when the host says so, else warm when the pointer is within the warm pad of the bounds.
    /// CadCraft's dynamic grips (arrows, triangles) warm over their hit area, the 3 pt pad.
    pub(crate) fn state(grip: &Grip, bounds: Rect, hover: Option<Pos2>, style: &GripSetStyle) -> GripState {
        let pad = match grip.shape {
            GripShape::Arrow { .. } | GripShape::Triangle => style.hit_pad,
            GripShape::Square | GripShape::Midpoint => style.warm_pad,
        };
        if grip.hot {
            GripState::Hot
        } else if hover.is_some_and(|p| bounds.expand(pad).contains(p)) {
            GripState::Warm
        } else {
            GripState::Cold
        }
    }

    /// A press on the canvas read from the input: the grip it takes, claimed for the set.
    fn press_pass(&self, ui: &Ui, canvas: &Response, view: &CanvasView, style: &GripSetStyle, id: Id) -> Option<Press> {
        let ctx = ui.ctx();
        if !ui.is_enabled() || !ctx.input(|i| i.pointer.primary_pressed()) {
            return None;
        }
        let press = PrimaryPress::current(ctx)?;
        if !(view.rect.contains(press.origin) && canvas.contains_pointer()) {
            return None;
        }
        let grip =
            self.grips.iter().find(|g| self.bounds(style, g.shape, view.to_screen(g.at), view).expand(style.hit_pad).contains(press.origin))?;
        if !Claim::take(ctx, canvas.id, id) {
            return None;
        }
        let waits = Claim::turn(ctx, canvas.id, id) == Turn::Waits;
        Some(Press { key: grip.key, from: grip.at, origin: press.origin, press, began: false, cancelled: false, waits })
    }

    /// One pass of a press read from the input: begins the drag on the first move past the click
    /// slop, updates it, and on the release ends it or reports the click.
    fn track_pass(&self, ctx: &Context, view: &CanvasView, p: &mut Press, gesture: &mut Gesture<GripDrag>, out: &mut GripSetResponse) {
        if p.cancelled {
            return;
        }
        let (pointer, down, held) = ctx.input(|i| (i.pointer.latest_pos(), i.pointer.primary_down(), i.modifiers));
        let slop = ctx.options(|o| o.input_options.max_click_dist);
        if let Some(pointer) = pointer {
            if !p.began && pointer.distance(p.origin) > slop {
                gesture.begin();
                p.began = gesture.is_active();
            }
            if p.began {
                gesture.update(GripDrag { key: p.key, from: p.from, to: view.to_doc(pointer) }, held);
            }
        }
        if down {
            return;
        }
        if p.began {
            gesture.commit();
        } else if pointer.is_some_and(|q| q.distance(p.origin) <= slop) {
            out.clicked = Some(GripClick { key: p.key, shift: self.modifiers.unwrap_or(held).shift });
        }
    }

    /// Enter, ⇧+Enter and the arrows on the grip `picked`; `idle`: no press is held.
    fn keys(
        &self,
        ui: &Ui,
        idle: bool,
        picked: Option<u64>,
        at_of: &dyn Fn(u64) -> Option<Point>,
        gesture: &mut Gesture<GripDrag>,
        out: &mut GripSetResponse,
    ) {
        let ctx = ui.ctx();
        if !idle || !ui.is_enabled() || ctx.text_edit_focused() || popup_list::any_open(ctx) || !ctx.memory(|m| m.allows_interaction(ui.layer_id())) {
            return;
        }
        let Some(key) = picked else { return };
        let Some(at) = at_of(key) else { return };
        let (shifted, plain) = ui.input_mut(|i| (i.consume_key(Modifiers::SHIFT, Key::Enter), i.consume_key(Modifiers::NONE, Key::Enter)));
        if shifted || plain {
            out.clicked = Some(GripClick { key, shift: shifted });
            return;
        }
        if let Some(by) = nudge::read(ui, self.steps.0, self.steps.1) {
            gesture.set(GripDrag { key, from: at, to: at + by });
        }
    }
}

/// A press the set owns: the grip it took and how far it has gone.
#[derive(Clone, Copy, Debug)]
struct Press {
    key: u64,
    /// Where the grip was at the press, in document units.
    from: Point,
    origin: Pos2,
    press: PrimaryPress,
    began: bool,
    cancelled: bool,
    /// The drag waits for its turn behind a typing run that ended first.
    waits: bool,
}

fn corners(r: Rect) -> Vec<Pos2> {
    vec![r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()]
}

fn press_key(id: Id) -> Id {
    id.with("craft_ui::canvas::GripSet::press")
}

fn pick_key(id: Id) -> Id {
    id.with("craft_ui::canvas::GripSet::picked")
}
