//! [`PinWarp`]: Puppet Warp's pins on the canvas, placed, moved, turned and removed by pointer and
//! keyboard (task TR-T12).

use egui::{AsIdSalt, Context, CursorIcon, Id, IdSalt, Key, Modifiers, Painter, Pos2, Rect, Response, Shape, Ui, vec2};
use kurbo::Vec2;

use craft_ui::behavior::{ClaimedKey, KeyClaim, nudge};
use craft_ui::canvas::claim::PrimaryPress;
use craft_ui::canvas::{CanvasView, Claim, Part, PartFocus, RulerUnits};
use craft_ui::composites::curve_editor::{fill, take_exact};
use craft_ui::edit::order::Turn;
use craft_ui::edit::{EditPolicy, Gesture};
use craft_ui::localized::{Labels as _, LocaleNumbers};
use craft_ui::popup_list;
use craft_ui::primitives::{Handle, HandleShape, HandleStyle};
use craft_ui::theme::{Tokens, TokensOf as _, UiTokens};
use craft_ui::words;

use super::{Pin, PinEdit, PinMeshView, PinWarpSession};
use crate::ui::warp::{PinWarpStyle, paint_mesh};

words! {
    /// The AccessKit words of [`PinWarp`]. `Default` is English.
    // Every field is one self-evident word; its English text is the default.
    #[allow(missing_docs)]
    pub struct PinWarpLabels {
        group: "pinWarp.group" = "Puppet Warp";
        pin: "pinWarp.pin" = "Pin {n} of {count}";
        turn: "pinWarp.turn" = "turn {degrees}°";
        position: "pinWarp.position" = "{x}, {y}";
    }
}

/// An open Puppet Warp's pins over the warped picture (drawn by the transform preview under the
/// guides), and the mesh lines when Show Mesh is on.
///
/// Pointer (strict `<` on `transform.hit` as the box):
/// - A click on the mesh away from pins places a pin at the rest position under it
///   ([`PinMeshView::rest_of`]), selects it, and the drag that follows moves it: one step. A click
///   off the mesh does nothing.
/// - A press on a pin selects it and drags it; Shift-click adds it to the selection or takes it
///   out; dragging a selected pin moves every selected pin.
/// - ⌥-click on a pin deletes it (one step).
/// - ⌥-drag within `transform.pin_turn` of a selected pin, not on it, turns it: the pin's turn at
///   the press (its fixed rotation, or an Auto pin's turn in the deformed mesh,
///   [`PinMeshView::turn_at`]) changes by the angle the pointer sweeps about the pin, and the
///   result is its fixed rotation, which it keeps (Photoshop; VectorCraft).
/// - Esc during a drag puts the pins back (Cancel); with no drag it discards the session.
///
/// Keys (one Tab stop, `PartFocus`): Page Down and Up pick the next and previous pin (every pin,
/// then none); Home and End the first and last; arrows move the picked pin (⇧ ten, through
/// `behavior::nudge`); Delete or Backspace removes the selected pins; Insert places a pin at the
/// rest position under the pointer, or at the [mesh's centre](PinMeshView::centre) when the
/// pointer is outside, and nothing where the mesh has no rest position; ⌘A selects every pin; Enter applies.
///
/// AccessKit: a group named [`PinWarpLabels::group`]; each pin a graphics object "Pin 2 of 5"
/// with value "x, y" (and "turn N°" when fixed) and its selected state. The mesh lines are
/// decoration. Nothing mirrors in a right-to-left interface.
pub struct PinWarp<'a> {
    id_salt: IdSalt,
    session: &'a PinWarpSession,
    labels: Option<PinWarpLabels>,
    units: Option<(&'a RulerUnits, &'a RulerUnits)>,
    modifiers: Option<Modifiers>,
    policy: EditPolicy,
}

/// What [`PinWarp::show`] reported this pass.
#[derive(Clone, Debug, PartialEq)]
pub struct PinWarpResponse {
    /// The overlay's widget id.
    pub id: Id,
    /// The edits in the order they happened; the app applies each with
    /// [`PinWarpSession::apply`].
    pub edits: Vec<PinEdit>,
    /// Enter: the app commits ([`PinWarpSession::result`]).
    pub apply: bool,
    /// Esc with no drag running: the app discards the session.
    pub discard: bool,
    /// The overlay took this pass's press.
    pub claimed: bool,
    /// The pin under the pointer.
    pub hovered: Option<usize>,
}

impl<'a> PinWarp<'a> {
    /// The pins of `session`.
    pub fn new(id_salt: impl AsIdSalt, session: &'a PinWarpSession) -> Self {
        Self { id_salt: IdSalt::new(id_salt), session, labels: None, units: None, modifiers: None, policy: EditPolicy::default() }
    }

    /// Sets the words; the installed language by default.
    pub fn labels(mut self, labels: PinWarpLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The ruler units of the pins' AccessKit values.
    pub fn units(mut self, x: &'a RulerUnits, y: &'a RulerUnits) -> Self {
        self.units = Some((x, y));
        self
    }

    /// The modifiers every test reads in place of the input's.
    pub fn modifiers(mut self, modifiers: Modifiers) -> Self {
        self.modifiers = Some(modifiers);
        self
    }

    /// Sets how the overlay reports gestures (live, committed, cancelled).
    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// The look and reach in `tokens`.
    pub fn style(tokens: &craft_ui::theme::UiTokens) -> PinWarpStyle {
        PinWarpStyle::of(tokens)
    }

    /// Draws the mesh lines (when shown), the pins and the picked pin's focus ring over `view`, and
    /// senses through the host's `canvas` response. `mesh` is the solver's mesh this pass.
    pub fn show(self, ui: &mut Ui, canvas: &Response, view: &CanvasView, mesh: PinMeshView<'_>) -> PinWarpResponse {
        // pc: crates/ui-egui/src/tools/puppet_ui.rs:293 (`pointer`), :365 (`draw`), :154
        // (`pin_at`); crates/ui-egui/src/dialogs/distort_ui.rs:409 (`tolerance`, 8, replaced by
        // `transform.hit` 12).
        let ctx = ui.ctx().clone();
        let tokens = Tokens::of(ui);
        let style = PinWarp::style(&tokens);
        let id = ui.make_persistent_id(self.id_salt);
        let labels = self.labels.clone().unwrap_or_else(|| PinWarpLabels::of(ui));
        let enabled = ui.is_enabled();
        let mods = self.modifiers.unwrap_or_else(|| ctx.input(|i| i.modifiers));
        let pins = self.session.pins();
        let mut out = PinWarpResponse { id, edits: Vec::new(), apply: false, discard: false, claimed: false, hovered: None };
        let mut selected: Vec<usize> = self.session.selected().to_vec();
        let (mut before, mut after) = (Vec::new(), Vec::new());

        // A drag whose press ended while the overlay was not shown is dropped, and its gesture with it.
        let mut drag: Option<Drag> =
            ctx.data(|d| d.get_temp(drag_key(id))).filter(|d: &Drag| d.press.holds(&ctx) || (d.waits && Claim::is_owner(&ctx, canvas.id, id)));
        let waited = drag.as_ref().is_some_and(|d| d.waits);
        if let Some(d) = &mut drag
            && d.waits
        {
            match Claim::turn(&ctx, canvas.id, id) {
                Turn::Waits => {}
                Turn::Runs => d.waits = false,
                Turn::Now => drag = None,
            }
        }
        if enabled && drag.is_none() && !Claim::keeps_waiting(&ctx, canvas.id, id, waited) {
            let pressed = self.press_pass(&ctx, canvas, view, &style, id, mods, &selected, mesh);
            if let Some(Pressed { drag: started, select }) = pressed {
                if let Some(s) = select {
                    select_edit(&mut selected, &mut before, s);
                }
                drag = Some(started);
            }
        }

        let reported = Gesture::<Vec<Pin>>::reported(ui, id).map(|(_, value)| value);
        let host: Vec<Pin> = match &drag {
            Some(d) if d.began && !d.cancelled && (pins == d.start || pins == d.base || reported.as_deref() == Some(pins)) => {
                reported.clone().unwrap_or_else(|| d.base.clone())
            }
            _ => pins.to_vec(),
        };
        let mut gesture = Gesture::<Vec<Pin>>::load(ui, id, &host, self.policy).focus_owners(&[]).enabled(enabled);
        if let Some(d) = &mut drag
            && d.began
            && !d.cancelled
            && !gesture.is_active()
        {
            d.cancelled = true;
        }
        // Esc before the first move ends the press as it would end its drag.
        if let Some(d) = &mut drag
            && !d.began
            && !d.cancelled
            && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
        {
            d.cancelled = true;
        }
        if let Some(d) = &mut drag
            && !d.waits
        {
            self.track_pass(&ctx, view, mesh, d, pins, mods, &mut gesture, &mut selected, &mut after);
        }

        let shown: Vec<Pin> = gesture.shown().clone();
        let parts = self.parts(ui, view, &labels, &shown, &selected);
        let picked = selected.first().copied().filter(|&i| i < shown.len());
        let focus =
            PartFocus::new(id, labels.group.clone()).parts(&parts).picked(picked.map(|i| i as u64)).focusable(enabled).wrap(true).show(ui, canvas);
        match focus.pick {
            Some(Some(key)) => select_edit(&mut selected, &mut before, vec![key as usize]),
            Some(None) => select_edit(&mut selected, &mut before, Vec::new()),
            None => {}
        }
        if focus.focused {
            KeyClaim::claim(&ctx, id, &CLAIMED);
        }
        let idle = drag.as_ref().is_none_or(|d| d.cancelled);
        self.keys(ui, canvas, view, mesh, idle, focus.focused, &mut gesture, &shown, &mut selected, (&mut before, &mut after), &mut out);

        let dragging = drag.as_ref().filter(|d| d.began && !d.cancelled);
        let other = Claim::owner(&ctx, canvas.id).is_some_and(|owner| owner != id);
        if let Some(d) = dragging {
            ctx.set_cursor_icon(if matches!(d.kind, Kind::Turn { .. }) { CursorIcon::Alias } else { CursorIcon::Move });
        } else if drag.is_none()
            && !other
            && let Some(pointer) = canvas.hover_pos()
        {
            out.hovered = pin_at(view, pins, pointer, style.hit);
            if out.hovered.is_some() {
                ctx.set_cursor_icon(if mods.alt { CursorIcon::NotAllowed } else { CursorIcon::Move });
            }
        }

        out.edits = before;
        if let Some(event) = gesture.finish() {
            out.edits.push(PinEdit::Pins(event));
        }
        out.edits.extend(after);
        out.claimed = Claim::is_owner(&ctx, canvas.id, id);
        let held = drag.filter(|d| d.waits || ctx.input(|i| i.pointer.primary_down()));
        ctx.data_mut(|data| match held {
            Some(d) => {
                data.insert_temp(drag_key(id), d);
            }
            None => {
                data.remove::<Drag>(drag_key(id));
            }
        });

        let painter = ui.painter_at(view.rect);
        if self.session.show_mesh() {
            let edges = mesh.edges();
            let at = |i: u32| mesh.deformed.get(i as usize).map(|&p| view.to_screen(p));
            paint_mesh(&painter, edges.into_iter().filter_map(|[a, b]| Some([at(a)?, at(b)?])), &style.mesh);
        }
        paint_pins(&painter, &tokens, &style, view, &shown, &selected, picked.filter(|_| focus.keyboard));
        out
    }

    /// A press at `origin` on a pin, near a selected pin with ⌥, or on the mesh: the drag to track
    /// and the selection it makes. `None` where the press is the tool's.
    #[expect(clippy::too_many_arguments, reason = "one pass's state, passed once")]
    fn press_pass(
        &self,
        ctx: &Context,
        canvas: &Response,
        view: &CanvasView,
        style: &PinWarpStyle,
        id: Id,
        mods: Modifiers,
        selected: &[usize],
        mesh: PinMeshView<'_>,
    ) -> Option<Pressed> {
        if !ctx.input(|i| i.pointer.primary_pressed()) {
            return None;
        }
        let press = PrimaryPress::current(ctx)?;
        if !(view.rect.contains(press.origin) && canvas.contains_pointer()) {
            return None;
        }
        let pins = self.session.pins();
        let mut select = None;
        let kind = if let Some(i) = pin_at(view, pins, press.origin, style.hit) {
            if mods.alt {
                Kind::Delete(i)
            } else if mods.shift {
                let mut now = selected.to_vec();
                let kind = match now.iter().position(|&k| k == i) {
                    Some(at) => {
                        now.remove(at);
                        Kind::Click
                    }
                    None => {
                        now.push(i);
                        Kind::Move(now.clone())
                    }
                };
                select = Some(now);
                kind
            } else if selected.contains(&i) {
                Kind::Move(selected.to_vec())
            } else {
                select = Some(vec![i]);
                Kind::Move(vec![i])
            }
        } else if mods.alt {
            let i = nearest_selected(view, pins, selected, press.origin, style.turn_radius)?;
            let pin = pins[i];
            let base = pin.angle.or_else(|| mesh.turn_at(pin.rest))?;
            let from = angle_of(view.to_doc(press.origin) - pin.at)?;
            Kind::Turn { pin: i, base, from }
        } else {
            mesh.rest_of(view.to_doc(press.origin))?;
            Kind::Place
        };
        if !Claim::take(ctx, canvas.id, id) {
            return None;
        }
        let waits = Claim::turn(ctx, canvas.id, id) == Turn::Waits;
        Some(Pressed {
            drag: Drag { press, kind, origin: press.origin, began: false, cancelled: false, waits, start: Vec::new(), base: Vec::new() },
            select,
        })
    }

    /// One pass of a drag read from the input: begins it on the first move (a placed pin at once),
    /// updates it, ends it on the release.
    #[expect(clippy::too_many_arguments, reason = "one pass's state, passed once")]
    fn track_pass(
        &self,
        ctx: &Context,
        view: &CanvasView,
        mesh: PinMeshView<'_>,
        d: &mut Drag,
        pins: &[Pin],
        mods: Modifiers,
        gesture: &mut Gesture<Vec<Pin>>,
        selected: &mut Vec<usize>,
        after: &mut Vec<PinEdit>,
    ) {
        if d.cancelled {
            return;
        }
        let (pointer, down) = ctx.input(|i| (i.pointer.latest_pos(), i.pointer.primary_down()));
        match &d.kind {
            Kind::Click => return,
            Kind::Delete(i) => {
                if !d.began {
                    d.began = true;
                    if *i < pins.len() {
                        let mut rest = pins.to_vec();
                        rest.remove(*i);
                        gesture.set(rest);
                    }
                }
                return;
            }
            Kind::Place if !d.began => {
                let Some(rest) = mesh.rest_of(view.to_doc(d.origin)) else {
                    d.cancelled = true;
                    return;
                };
                d.start = pins.to_vec();
                d.base = pins.to_vec();
                d.base.push(Pin { rest, at: view.to_doc(d.origin), angle: None, depth: 0 });
                gesture.begin_with_value();
                if !gesture.is_active() {
                    d.cancelled = true;
                    return;
                }
                d.began = true;
                after.push(PinEdit::Select(vec![d.base.len() - 1]));
                *selected = vec![d.base.len() - 1];
            }
            _ if !d.began => {
                if pointer.is_none_or(|p| p == d.origin) {
                    return;
                }
                d.start = pins.to_vec();
                d.base = pins.to_vec();
                gesture.begin_with_value();
                if !gesture.is_active() {
                    d.cancelled = true;
                    return;
                }
                d.began = true;
            }
            _ => {}
        }
        if let Some(pointer) = pointer {
            gesture.update(d.value_at(view, pointer), mods);
        }
        if !down {
            gesture.commit();
        }
    }

    /// The keys: Enter and Esc always; with the group focused and no press held, the arrows move the
    /// picked pin, Delete and Backspace remove the selected pins, Insert places a pin and ⌘A
    /// selects every pin.
    #[expect(clippy::too_many_arguments, reason = "one pass's state, passed once")]
    fn keys(
        &self,
        ui: &Ui,
        canvas: &Response,
        view: &CanvasView,
        mesh: PinMeshView<'_>,
        idle: bool,
        focused: bool,
        gesture: &mut Gesture<Vec<Pin>>,
        shown: &[Pin],
        selected: &mut Vec<usize>,
        (before, after): (&mut Vec<PinEdit>, &mut Vec<PinEdit>),
        out: &mut PinWarpResponse,
    ) {
        let ctx = ui.ctx();
        if !ui.is_enabled() || ctx.text_edit_focused() || popup_list::any_open(ctx) || !ctx.memory(|m| m.allows_interaction(ui.layer_id())) {
            return;
        }
        let (enter, escape) = ui.input_mut(|i| (i.consume_key(Modifiers::NONE, Key::Enter), i.consume_key(Modifiers::NONE, Key::Escape)));
        out.apply |= enter;
        out.discard |= escape && !enter;
        if !idle || !focused {
            return;
        }
        if take_exact(ui, Modifiers::COMMAND, Key::A) > 0 {
            select_edit(selected, before, (0..shown.len()).collect());
        }
        let picked = selected.first().copied().filter(|&i| i < shown.len());
        if let (Some(by), Some(i)) = (nudge::read(ui, nudge::STEP, nudge::BIG_STEP), picked) {
            let mut moved = shown.to_vec();
            moved[i].at += by;
            gesture.set(moved);
        }
        let delete = take_exact(ui, Modifiers::NONE, Key::Delete) + take_exact(ui, Modifiers::NONE, Key::Backspace) > 0;
        if delete && selected.iter().any(|&i| i < shown.len()) {
            let rest: Vec<Pin> = shown.iter().enumerate().filter(|(i, _)| !selected.contains(i)).map(|(_, p)| *p).collect();
            gesture.set(rest);
        }
        if take_exact(ui, Modifiers::NONE, Key::Insert) > 0 {
            let under = canvas.hover_pos().map(|p| view.to_doc(p));
            let placed = under.and_then(|at| mesh.rest_of(at).map(|rest| Pin { rest, at, angle: None, depth: 0 })).or_else(|| {
                let at = mesh.centre()?;
                Some(Pin { rest: mesh.rest_of(at)?, at, angle: None, depth: 0 })
            });
            if let Some(pin) = placed {
                let mut grown = shown.to_vec();
                grown.push(pin);
                after.push(PinEdit::Select(vec![grown.len() - 1]));
                gesture.set(grown);
            }
        }
    }

    /// The pins as keyboard parts and AccessKit objects.
    fn parts(&self, ui: &Ui, view: &CanvasView, labels: &PinWarpLabels, pins: &[Pin], selected: &[usize]) -> Vec<Part> {
        let pixels;
        let (ux, uy) = match self.units {
            Some(units) => units,
            None => {
                pixels = RulerUnits::pixels((*LocaleNumbers::of(ui)).clone());
                (&pixels, &pixels)
            }
        };
        let hit = PinWarp::style(&Tokens::of(ui)).hit;
        pins.iter()
            .enumerate()
            .map(|(i, pin)| {
                let name = fill(&labels.pin, &[("n", &(i + 1).to_string()), ("count", &pins.len().to_string())]);
                let mut value = fill(&labels.position, &[("x", &ux.format_value(pin.at.x, 0.0)), ("y", &uy.format_value(pin.at.y, 0.0))]);
                if let Some(angle) = pin.angle {
                    let degrees = ux.numbers.format(angle, 0..=1);
                    value = format!("{value}, {}", fill(&labels.turn, &[("degrees", &degrees)]));
                }
                let at = view.to_screen(pin.at);
                Part {
                    key: i as u64,
                    name,
                    value: Some(value),
                    rect: Rect::from_center_size(at, vec2(2.0 * hit, 2.0 * hit)),
                    selected: selected.contains(&i),
                }
            })
            .collect()
    }
}

/// The keys a focused overlay keeps from the app's shortcuts.
const CLAIMED: [ClaimedKey; 4] = [
    ClaimedKey::exact(Modifiers::NONE, Key::Delete),
    ClaimedKey::exact(Modifiers::NONE, Key::Backspace),
    ClaimedKey::exact(Modifiers::NONE, Key::Insert),
    ClaimedKey::exact(Modifiers::COMMAND, Key::A),
];

/// What a press does.
#[derive(Clone, Debug)]
enum Kind {
    /// Moves these pins by the pointer's movement.
    Move(Vec<usize>),
    /// Places a pin under the press and moves it.
    Place,
    /// Turns this pin about its position: the pin's turn when pressed (its fixed angle, else the
    /// deformed mesh's turn under its rest point) and the pointer's angle about the pin then, in
    /// degrees.
    Turn { pin: usize, base: f64, from: f64 },
    /// Deletes this pin.
    Delete(usize),
    /// Only changed the selection.
    Click,
}

/// A press the overlay owns, kept between passes.
#[derive(Clone, Debug)]
struct Drag {
    press: PrimaryPress,
    kind: Kind,
    /// The press, in screen points.
    origin: Pos2,
    began: bool,
    /// Esc, an outside change or a missed pass ended the gesture; the rest of the press does
    /// nothing.
    cancelled: bool,
    /// The press waits for another control's typing run to end ([`Claim::turn`]).
    waits: bool,
    /// The host's pins when the gesture began.
    start: Vec<Pin>,
    /// The pins the movement is added to: `start`, with the placed pin for a placement.
    base: Vec<Pin>,
}

impl Drag {
    /// The pins with the pointer at screen point `pointer`.
    fn value_at(&self, view: &CanvasView, pointer: Pos2) -> Vec<Pin> {
        let mut pins = self.base.clone();
        match &self.kind {
            Kind::Move(moved) => {
                let by = view.to_doc(pointer) - view.to_doc(self.origin);
                for &i in moved {
                    if let Some(pin) = pins.get_mut(i) {
                        pin.at += by;
                    }
                }
            }
            Kind::Place => {
                let by = view.to_doc(pointer) - view.to_doc(self.origin);
                if let Some(pin) = pins.last_mut() {
                    pin.at += by;
                }
            }
            Kind::Turn { pin, base, from } => {
                if let Some(pin) = pins.get_mut(*pin)
                    && let Some(now) = angle_of(view.to_doc(pointer) - pin.at)
                {
                    pin.angle = Some(wrap_degrees(base + (now - from)));
                }
            }
            Kind::Delete(_) | Kind::Click => {}
        }
        pins
    }
}

/// A new press and the selection it makes.
struct Pressed {
    drag: Drag,
    select: Option<Vec<usize>>,
}

/// The angle of `v` in degrees, `None` for the zero vector.
fn angle_of(v: Vec2) -> Option<f64> {
    (v.hypot() > 0.0).then(|| v.y.atan2(v.x).to_degrees())
}

/// `degrees` in (−180, 180].
fn wrap_degrees(degrees: f64) -> f64 {
    let wrapped = (degrees + 180.0).rem_euclid(360.0) - 180.0;
    if wrapped == -180.0 { 180.0 } else { wrapped }
}

fn drag_key(id: Id) -> Id {
    id.with("craft_imaging::warp::pin::drag")
}

/// Reports a selection and takes it as the current one.
fn select_edit(selected: &mut Vec<usize>, edits: &mut Vec<PinEdit>, now: Vec<usize>) {
    if *selected != now {
        edits.push(PinEdit::Select(now.clone()));
        *selected = now;
    }
}

/// The pin nearest screen point `p` and strictly within `reach` of it.
fn pin_at(view: &CanvasView, pins: &[Pin], p: Pos2, reach: f32) -> Option<usize> {
    pins.iter()
        .enumerate()
        .map(|(i, pin)| (i, view.to_screen(pin.at).distance(p)))
        .filter(|&(_, d)| d < reach)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| i)
}

/// The selected pin nearest screen point `p` and strictly within `reach` of it.
fn nearest_selected(view: &CanvasView, pins: &[Pin], selected: &[usize], p: Pos2, reach: f32) -> Option<usize> {
    selected
        .iter()
        .filter_map(|&i| pins.get(i).map(|pin| (i, view.to_screen(pin.at).distance(p))))
        .filter(|&(_, d)| d < reach)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| i)
}

/// The pins in depth order (a higher depth on top), each a round handle in the pin look; the
/// selected ones with their core and, when turned, their ring and line; the picked one with the
/// focus ring.
fn paint_pins(painter: &Painter, tokens: &UiTokens, style: &PinWarpStyle, view: &CanvasView, pins: &[Pin], selected: &[usize], ring: Option<usize>) {
    let mut look: HandleStyle = Handle::style(tokens, HandleShape::Round);
    look.size = style.pin;
    look.fill = style.fill;
    look.stroke = style.ring;
    let ppp = painter.ctx().pixels_per_point();
    let mut order: Vec<usize> = (0..pins.len()).collect();
    order.sort_by_key(|&i| pins[i].depth);
    for i in order {
        let pin = &pins[i];
        let at = view.to_screen(pin.at);
        painter.extend(Handle::shapes_with(&look, HandleShape::Round, at, ppp));
        if selected.contains(&i) {
            painter.add(Shape::circle_filled(at, style.core / 2.0, style.core_color));
            if let Some(angle) = pin.angle {
                let dir = view.map_vec(Vec2::from_angle(angle.to_radians())).normalized();
                painter.add(Shape::circle_stroke(at, style.turn_radius, style.turn));
                painter.add(Shape::line_segment([at, at + dir * style.turn_radius], style.turn));
            }
        }
    }
    if let Some(i) = ring.filter(|&i| i < pins.len()) {
        PartFocus::ring(painter, Rect::from_center_size(view.to_screen(pins[i].at), vec2(style.pin, style.pin)));
    }
}
