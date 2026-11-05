//! [`BlurPinsOverlay`]: the pins on the main canvas.

use egui::epaint::PathShape;
use egui::{AsIdSalt, Context, CornerRadius, Id, IdSalt, Key, Modifiers, Painter, Pos2, Rect, Response, Shape, Stroke, StrokeKind, Ui, Vec2};

use craft_ui::behavior::{ClaimedKey, KeyClaim, nudge};
use craft_ui::canvas::claim::PrimaryPress;
use craft_ui::canvas::kurbo::Point;
use craft_ui::canvas::{CanvasView, Claim, Part, PartFocus, PartFocusResponse, Turn};
use craft_ui::composites::curve_editor::take_exact;
use craft_ui::edit::{EditEvent, EditPolicy, Gesture};
use craft_ui::localized::{Labels as _, Units};
use craft_ui::theme::{UiTokens, current};

use super::grab::{AXIS_HANDLES, feather_handles, iris_point, round_handle, tilt_point, turn_handle};
use super::model::{BlurKind, BlurPin, BlurPins};
use super::style::BlurPinsStyle;
use super::{BlurGrab, BlurPinsLabels, drag as drag_pins, grab as grab_at};

/// What [`BlurPinsOverlay::show`] reports.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct BlurPinsResponse {
    /// A pin added (`Set`), a drag (Begin, Update, Commit) or a pin removed (`Set`); the pins
    /// changed in place too.
    pub edit: Option<EditEvent<BlurPins>>,
    /// The overlay took the press: the canvas's own tool gets none.
    pub claimed: bool,
}

/// The Blur Gallery's pins over the main canvas while its filter dialog is open (lead call 48:
/// PhotoCraft's form, an overlay with a panel, no workspace).
///
/// - Pointer: a click on nothing adds a pin ([`BlurPins::add`]); a press grabs per
///   [`grab`](super::grab) and drags per [`drag`](super::drag); the cursor follows the grab
///   ([`BlurGrab::cursor`](super::BlurGrab::cursor)).
/// - Keyboard: the pins are parts of one `PartFocus` group ("Pin 3, Blur 15 px"); Page picks
///   (and selects), arrows move the pin one document pixel (Shift ten), Delete and Backspace
///   remove the selected pin unless a text field has focus.
/// - Drawing: each pin's dot (`blur_pins.dot` 5) and ring (`.ring` 24, the amount as an arc of
///   it); the selected pin's outline (Iris's superellipse, Spin's ellipse) with its handles
///   (`.handle` 4.5), the rounding square and the feather dots; Tilt-Shift's solid focus lines,
///   dashed transition lines and turn dots; all over the halo.
///
/// A click that only adds a pin or selects one reports a `Set`; a press that moves reports one
/// gesture from the pins before the press, so a pin added and dragged in one press is one step.
pub struct BlurPinsOverlay<'a> {
    id_salt: IdSalt,
    pins: &'a mut BlurPins,
    labels: Option<BlurPinsLabels>,
}

/// A press the overlay owns, kept between passes.
#[derive(Clone, Debug)]
struct Held {
    press: PrimaryPress,
    grab: BlurGrab,
    /// The pins before the press.
    base: BlurPins,
    /// The pins at the press, with the pin a click on nothing added or the pin a press selected.
    start: BlurPins,
    /// What a release without a drag sets: `start` when the press added or selected.
    pending: Option<BlurPins>,
    /// The press and the previous frame's pointer, in document pixels.
    from: Point,
    last: Point,
    /// The pins the drag last computed: the ring turns from them frame by frame.
    shown: BlurPins,
    began: bool,
    /// Esc or an outside change ended it: the rest of the press does nothing.
    cancelled: bool,
    /// The press waits for another control's typing run to end ([`Claim::turn`]).
    waits: bool,
}

fn held_key(id: Id) -> Id {
    id.with("craft_imaging::blur_pins::held")
}

/// Delete and Backspace are the overlay's while its group has focus.
const CLAIMED: [ClaimedKey; 2] = [ClaimedKey::exact(Modifiers::NONE, Key::Delete), ClaimedKey::exact(Modifiers::NONE, Key::Backspace)];

fn usable(pins: &BlurPins) -> bool {
    let b = pins.bounds;
    b[2] > b[0] && b[3] > b[1]
}

impl<'a> BlurPinsOverlay<'a> {
    /// Starts the overlay over `pins`, which it edits in place; `id_salt` keeps its state apart from other overlays.
    pub fn new(id_salt: impl AsIdSalt, pins: &'a mut BlurPins) -> Self {
        Self { id_salt: IdSalt::new(id_salt), pins, labels: None }
    }

    /// The words; default [`BlurPinsLabels::of`] the context's language.
    pub fn labels(mut self, labels: BlurPinsLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The marks' look and reach in `tokens`.
    pub fn style(tokens: &UiTokens) -> BlurPinsStyle {
        BlurPinsStyle::of(tokens)
    }

    /// Runs and draws the overlay over `canvas` (the main canvas's response) through `view`.
    pub fn show(self, ui: &mut Ui, canvas: &Response, view: &CanvasView) -> BlurPinsResponse {
        // pc: crates/ui-egui/src/dialogs/blur_pins.rs:533 `canvas`, :689 `draw`, :675 `draw_line`.
        let Self { id_salt, pins, labels } = self;
        let ctx = ui.ctx().clone();
        let id = ui.make_persistent_id(id_salt);
        let style = Self::style(&current(&ctx));
        let labels = labels.unwrap_or_else(|| BlurPinsLabels::of(ui));
        let units = Units::of(ui);
        let editable = usable(pins) && ui.is_enabled();
        // The pins are written in place from the gesture's value, so the gesture's own last report
        // (or the value it began from) stands for the pins while they show one of its values.
        let reported = Gesture::<BlurPins>::reported(ui, id).map(|(_, value)| value);
        let before = ctx.data(|d| d.get_temp::<Held>(held_key(id))).filter(|h| h.began && !h.cancelled);
        let host = match (&before, &reported) {
            (Some(h), Some(r)) if *pins == h.base || *pins == *r || *pins == h.shown => r.clone(),
            _ => pins.clone(),
        };
        let mut gesture = Gesture::load(ui, id, &host, EditPolicy::default()).focus_owners(&[]).enabled(editable);

        let held = press_of(&ctx, canvas, id, view, &style, gesture.shown(), editable);
        let held = held.and_then(|held| track(&ctx, view, held, &mut gesture));
        // A click that adds or selects shows at once, before its release sets it.
        let mut shown = gesture.shown().clone();
        if let Some(held) = held.as_ref().filter(|h| !h.began && !h.cancelled && !h.waits) {
            shown = held.start.clone();
        }

        if let Some(held) = &held {
            if !held.cancelled && !held.waits {
                ctx.set_cursor_icon(held.grab.cursor(shown.kind == BlurKind::TiltShift));
            }
        } else if editable
            && canvas.contains_pointer()
            && Claim::owner(&ctx, canvas.id).is_none()
            && let Some(at) = ctx.input(|i| i.pointer.latest_pos())
        {
            ctx.set_cursor_icon(grab_at(&shown, view, at, &style).cursor(shown.kind == BlurKind::TiltShift));
        }

        let parts = parts(view, &shown, &style, &labels, &units);
        let picked = shown.selected.map(|i| i as u64);
        let focus = PartFocus::new(id, labels.pins.as_str()).parts(&parts).picked(picked).focusable(editable).show(ui, canvas);
        if held.is_none() && editable {
            if let Some(Some(key)) = focus.pick
                && let Ok(index) = usize::try_from(key)
                && shown.selected != Some(index)
            {
                let mut next = shown.clone();
                next.selected = Some(index);
                gesture.set(next);
            }
            if focus.focused {
                KeyClaim::claim(&ctx, id, &CLAIMED);
            }
            keys(ui, &ctx, &focus, &shown, &mut gesture);
        }

        // The pins follow the gesture's value in place.
        if *gesture.shown() != *pins {
            *pins = gesture.shown().clone();
        }
        let edit = gesture.finish();
        let claimed = Claim::is_owner(&ctx, canvas.id, id);
        ctx.data_mut(|d| match &held {
            Some(held) => {
                d.insert_temp(held_key(id), held.clone());
            }
            None => d.remove::<Held>(held_key(id)),
        });

        let painter = ui.painter_at(view.rect);
        let ring = focus.keyboard.then(|| parts.iter().find(|p| Some(p.key) == picked).map(|p| p.rect)).flatten();
        paint(&painter, view, &shown, &style);
        if let Some(rect) = ring {
            PartFocus::ring(&painter, rect);
        }
        BlurPinsResponse { edit, claimed }
    }
}

/// The press the overlay holds in this pass: one taken earlier (a waiting one when its turn
/// comes), else a new press on the canvas.
fn press_of(ctx: &Context, canvas: &Response, id: Id, view: &CanvasView, style: &BlurPinsStyle, pins: &BlurPins, editable: bool) -> Option<Held> {
    let kept = ctx.data(|d| d.get_temp::<Held>(held_key(id))).filter(|h| h.press.holds(ctx) || (h.waits && Claim::is_owner(ctx, canvas.id, id)));
    match kept {
        Some(held) if held.waits => match Claim::turn(ctx, canvas.id, id) {
            Turn::Waits => Some(held),
            Turn::Runs => Some(start(view, style, pins, held.press)),
            Turn::Now => None,
        },
        Some(held) => Some(held),
        None if editable => {
            if !ctx.input(|i| i.pointer.primary_pressed()) {
                return None;
            }
            let press = PrimaryPress::current(ctx)?;
            // A press with ⌘ (Ctrl) is the host's.
            if ctx.input(|i| i.modifiers.command) || !(view.rect.contains(press.origin) && canvas.contains_pointer()) {
                return None;
            }
            if !Claim::take(ctx, canvas.id, id) {
                return None;
            }
            if Claim::turn(ctx, canvas.id, id) == Turn::Waits {
                return Some(Held { waits: true, ..start(view, style, pins, press) });
            }
            Some(start(view, style, pins, press))
        }
        None => None,
    }
}

/// What `press` grabs in `pins`: a click on nothing adds a pin (grabbed as a pin), a press on a
/// pin selects it.
fn start(view: &CanvasView, style: &BlurPinsStyle, pins: &BlurPins, press: PrimaryPress) -> Held {
    let from = view.to_doc(press.origin);
    let mut grab = grab_at(pins, view, press.origin, style);
    let mut at = pins.clone();
    let mut pending = None;
    match grab {
        BlurGrab::New => {
            if at.add(from) {
                grab = BlurGrab::Pin { pin: at.pins.len() - 1 };
                pending = Some(at.clone());
            }
        }
        BlurGrab::Pin { pin } if at.selected != Some(pin) => {
            at.selected = Some(pin);
            pending = Some(at.clone());
        }
        _ => {}
    }
    Held { press, grab, base: pins.clone(), start: at.clone(), pending, from, last: from, shown: at, began: false, cancelled: false, waits: false }
}

/// One pass of a press the overlay owns: the press that goes on.
fn track(ctx: &Context, view: &CanvasView, mut held: Held, gesture: &mut Gesture<BlurPins>) -> Option<Held> {
    if held.began && !held.cancelled && !gesture.is_active() {
        // Esc, an outside change or disabling ended the gesture.
        held.cancelled = true;
    }
    if !held.began && !held.cancelled && !held.waits && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
        held.cancelled = true;
    }
    if held.waits {
        return Some(held);
    }
    let (pointer, down) = ctx.input(|i| (i.pointer.latest_pos(), i.pointer.primary_down()));
    if held.cancelled {
        return down.then_some(held);
    }
    if let Some(pointer) = pointer {
        let slop = ctx.options(|o| o.input_options.max_click_dist);
        if !held.began && held.grab != BlurGrab::New && pointer.distance(held.press.origin) > slop {
            gesture.begin_with_value();
            held.began = gesture.is_active();
        }
        if held.began {
            let to = view.to_doc(pointer);
            let mut next;
            if matches!(held.grab, BlurGrab::Ring { .. }) {
                // The ring turns by the angle since the last frame, so several turns work.
                next = held.shown.clone();
                drag_pins(&mut next, held.grab, &held.shown, held.last, to);
            } else {
                next = held.start.clone();
                drag_pins(&mut next, held.grab, &held.start, held.from, to);
            }
            gesture.update(next.clone(), ctx.input(|i| i.modifiers));
            held.shown = next;
            held.last = to;
        }
    }
    if down {
        return Some(held);
    }
    if held.began {
        gesture.commit();
    } else if let Some(pending) = held.pending.take() {
        gesture.set(pending);
    }
    None
}

/// Delete and Backspace remove the selected pin (unless a text field has the focus); with the
/// group focused the arrows move it one document pixel (Shift ten).
fn keys(ui: &Ui, ctx: &Context, focus: &PartFocusResponse, shown: &BlurPins, gesture: &mut Gesture<BlurPins>) {
    let typing = !focus.focused && ctx.memory(|m| m.focused()).is_some();
    if typing {
        return;
    }
    let mut next = shown.clone();
    if take_exact(ui, Modifiers::NONE, Key::Delete) + take_exact(ui, Modifiers::NONE, Key::Backspace) > 0 {
        next.remove_selected();
    } else if focus.focused
        && nudge::reads(ui)
        && let Some(by) = nudge::read(ui, nudge::STEP, nudge::BIG_STEP)
        && let Some(index) = shown.selected
        && let Some(pin) = shown.pins.get(index)
    {
        let at = shown.to_document(pin);
        let (x, y) = shown.from_document(Point::new(at.x + by.x, at.y + by.y));
        next.pins[index].x = x;
        next.pins[index].y = y;
    }
    if next != *shown {
        gesture.set(next);
    }
}

/// The pins as keyboard parts, in order: "Pin 3, Blur 15 px" (Spin: "Blur Angle 90°").
fn parts(view: &CanvasView, pins: &BlurPins, style: &BlurPinsStyle, labels: &BlurPinsLabels, units: &Units) -> Vec<Part> {
    let reach = Vec2::splat(2.0 * style.grab);
    pins.pins
        .iter()
        .enumerate()
        .map(|(index, pin)| {
            let (word, amount) = if pins.kind == BlurKind::Spin {
                (&labels.pin_part_angle, units.degrees(pin.amount.round()))
            } else {
                (&labels.pin_part, units.pixels(pin.amount.round()))
            };
            let n = units.numbers().format(index as f64 + 1.0, 0..=0);
            Part {
                key: index as u64,
                name: craft_i18n::fmt(word, &[("n", n.as_str()), ("amount", amount.as_str())]),
                value: None,
                rect: Rect::from_center_size(view.to_screen(pins.to_document(pin)), reach),
                selected: pins.selected == Some(index),
            }
        })
        .collect()
}

/// A halo-edged line from `a` to `b`, dashed when `dash` is set.
fn line(painter: &Painter, style: &BlurPinsStyle, w: &BlurPinsStyle, a: Pos2, b: Pos2, dash: bool) {
    let (under, over) = (Stroke::new(w.line_halo, style.halo), Stroke::new(w.line_width, style.line));
    if dash {
        painter.extend(Shape::dashed_line(&[a, b], under, w.dash, w.dash_gap));
        painter.extend(Shape::dashed_line(&[a, b], over, w.dash, w.dash_gap));
    } else {
        painter.line_segment([a, b], under);
        painter.line_segment([a, b], over);
    }
}

/// The pins over the image: a dot per pin (filled when selected) and, round the selected one, its
/// ring with the amount as an arc from the top, and its outline and handles or Tilt-Shift's lines.
fn paint(painter: &Painter, view: &CanvasView, pins: &BlurPins, style: &BlurPinsStyle) {
    if !usable(pins) {
        return;
    }
    let w = style;
    for (index, pin) in pins.pins.iter().enumerate() {
        let centre = view.to_screen(pins.to_document(pin));
        if pins.selected == Some(index) {
            selected(painter, view, pins, pin, centre, style, w);
        } else {
            painter.circle_stroke(centre, style.dot, Stroke::new(w.line_halo, style.halo));
            painter.circle_stroke(centre, style.dot, Stroke::new(w.line_width, style.line));
        }
    }
}

fn selected(painter: &Painter, view: &CanvasView, pins: &BlurPins, pin: &BlurPin, centre: Pos2, style: &BlurPinsStyle, w: &BlurPinsStyle) {
    let edge = Stroke::new(w.handle_outline, style.halo);
    let screen = |p: Point| view.to_screen(p);
    if let Some(shape) = pin.iris {
        // Where the blur reaches its full amount (Iris) or where the spin ends.
        let m = shape.exponent();
        let outline: Vec<Pos2> = (0..=128)
            .map(|n| {
                let a = std::f64::consts::TAU * f64::from(n) / 128.0;
                let (sin, cos) = a.sin_cos();
                let r = (cos.abs().powf(m) + sin.abs().powf(m)).powf(-1.0 / m);
                screen(iris_point(pins, pin, shape, r * cos, r * sin))
            })
            .collect();
        painter.add(PathShape::line(outline.clone(), Stroke::new(w.line_halo, style.halo)));
        painter.add(PathShape::line(outline, Stroke::new(w.line_width, style.line)));
        for (u, v) in AXIS_HANDLES {
            painter.circle(screen(iris_point(pins, pin, shape, u, v)), style.handle, style.line, edge);
        }
        if pins.kind == BlurKind::Iris {
            let (u, v) = round_handle(shape);
            let square = Rect::from_center_size(screen(iris_point(pins, pin, shape, u, v)), Vec2::splat(2.0 * style.handle));
            painter.rect(square, CornerRadius::from(style.round_corner), style.line, edge, StrokeKind::Middle);
            for (u, v) in feather_handles(shape) {
                painter.circle(screen(iris_point(pins, pin, shape, u, v)), style.handle + style.feather_extra, style.line, edge);
            }
        }
    }
    if let Some(band) = pin.tilt {
        // The sharp band's edges (solid) and where the blur is full (dashed), across the image.
        let side = pins.short_side();
        let b = pins.bounds;
        let clip =
            painter.with_clip_rect(Rect::from_two_pos(screen(Point::new(b[0], b[1])), screen(Point::new(b[2], b[3]))).intersect(painter.clip_rect()));
        let reach = (b[2] - b[0]).hypot(b[3] - b[1]);
        for (offset, dash) in [(band.focus, false), (band.focus + band.transition, true)] {
            for sign in [1.0, -1.0] {
                let o = sign * offset * side;
                line(&clip, style, w, screen(tilt_point(pins, pin, band, o, -reach)), screen(tilt_point(pins, pin, band, o, reach)), dash);
            }
        }
        for sign in [1.0, -1.0] {
            painter.circle(screen(turn_handle(pins, pin, band, view, style, sign)), style.handle + style.turn_extra, style.line, edge);
        }
    }
    painter.circle_stroke(centre, style.ring, Stroke::new(w.line_halo, style.halo));
    painter.circle_stroke(centre, style.ring, Stroke::new(w.line_width, style.line));
    let fraction = (pin.amount / pins.kind.max()).clamp(0.0, 1.0) as f32;
    if fraction > 0.0 {
        let steps = (fraction * 64.0).ceil().max(1.0) as usize;
        let arc: Vec<Pos2> = (0..=steps)
            .map(|n| {
                let a = -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * fraction * n as f32 / steps as f32;
                centre + Vec2::angled(a) * style.ring
            })
            .collect();
        painter.add(PathShape::line(arc.clone(), Stroke::new(w.arc_halo, style.halo)));
        painter.add(PathShape::line(arc, Stroke::new(w.arc, style.amount)));
    }
    painter.circle(centre, style.dot, style.line, edge);
}
