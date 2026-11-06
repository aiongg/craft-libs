//! [`SelectionFrame`]: the box round selected objects, with eight grips and a movable inside.

use egui::{AsIdSalt, Color32, Context, CursorIcon, Id, IdSalt, Key, Modifiers, Pos2, Response, Shape, Stroke, Ui, vec2};
use kurbo::{Point, Vec2};

use crate::behavior::nudge;
use crate::canvas::claim::PrimaryPress;
use crate::canvas::quad::{self, BoxPart, BoxRules, BoxState, bisector, outset};
use crate::canvas::{CanvasView, Claim, Part, PartFocus, RulerUnits, resize_cursor};
use crate::edit::order::Turn;
use crate::edit::{EditEvent, EditPolicy, Gesture};
use crate::localized::Labels;
use crate::popup_list;
use crate::primitives::{FocusRing, Handle, HandleShape};
use crate::theme::{ToEgui, current};

use super::SelectionFrameStyle;

crate::words! {
    /// The AccessKit names of a [`SelectionFrame`] and its parts, English by default (the corner
    /// and edge names match Free Transform's).
    #[non_exhaustive]
    pub struct SelectionFrameLabels {
        group: "selectionFrame.group" = "Selection frame";
        whole: "selectionFrame.whole" = "Whole frame";
        top_left: "selectionFrame.topLeftCorner" = "Top left corner";
        top_right: "selectionFrame.topRightCorner" = "Top right corner";
        bottom_right: "selectionFrame.bottomRightCorner" = "Bottom right corner";
        bottom_left: "selectionFrame.bottomLeftCorner" = "Bottom left corner";
        top: "selectionFrame.topEdge" = "Top edge";
        right: "selectionFrame.rightEdge" = "Right edge";
        bottom: "selectionFrame.bottomEdge" = "Bottom edge";
        left: "selectionFrame.leftEdge" = "Left edge";
    }
}

/// What [`SelectionFrame::show`] reports.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct SelectionFrameResponse {
    /// The frame's id, for [`SelectionFrame::focus`].
    pub id: Id,
    /// A drag (`Begin` on the first move past the click slop, `Update`, `Commit` on release; Esc
    /// cancels it) or one arrow press's `Set`. The value is the whole box.
    pub event: Option<EditEvent<BoxState>>,
    /// The part the keys act on ([`BoxPart::Inside`]: the whole frame), picked by the pointer or
    /// the Page keys; `None` before either.
    pub picked: Option<BoxPart>,
    /// The part a press at the pointer would take; `None` where the press is the tool's.
    pub hovered: Option<BoxPart>,
    /// Whether the frame owns the current press: the host skips its tool input.
    pub claimed: bool,
}

/// The selection frame of the object-editing apps: a 1 pt box round the selected objects in the
/// layer colour, eight square grips that resize it, and an inside that moves it.
///
/// ```no_run
/// use craft_ui::canvas::CanvasView;
/// use craft_ui::canvas::quad::BoxState;
/// use craft_ui::canvas::selection::SelectionFrame;
/// # fn demo(ui: &mut egui::Ui, canvas: &egui::Response, view: &CanvasView, bounds: &BoxState, color: egui::Color32) {
/// let out = SelectionFrame::new("frame", bounds).color(color).key_object(true).centre_mark(true).show(ui, canvas, view);
/// if let Some(event) = out.event {
///     // The host's gesture host previews Update, applies Commit and Set as one undo step each.
///     # let _ = event;
/// }
/// # }
/// ```
///
/// - Look: the box line `stroke.thin` in [`color`](Self::color) (the layer colour), else
///   `overlay.line`; with [`key_object`](Self::key_object) `stroke.thick`;
///   with [`centre_mark`](Self::centre_mark) a filled 3.5 pt square at
///   the centre; grips are [`Handle`](crate::primitives::Handle)s
///   ([`HandleShape::Square`](crate::primitives::HandleShape::Square), `size.handle.md`) filled
///   `overlay.handle_fill` and outlined in the frame's colour, each grown by
///   [`grow`](Self::grow) steps of 1 pt.
/// - Pointer, through the host's canvas response and [`Claim`](crate::canvas::Claim) first: a
///   press within half of `size.hit.target` (the side of the square a handle takes) of a grip's
///   centre, or of half its grown size if that is more, takes that corner or edge; a press inside takes the
///   inside; a press outside is not the frame's. A drag begins on the first move and follows
///   [`quad::drag`](crate::canvas::quad::drag) with the [rules](Self::rules) (resize only by
///   default: corners stretch freely and ⇧ keeps the proportions, ⌥ scales about the centre, an
///   edge stretches its one axis, the inside moves and ⇧ locks it to the axes). Parts the rules
///   refuse show no grip and take no press.
/// - Keys (one Tab stop through [`PartFocus`](crate::canvas::PartFocus)), whenever no text field
///   has focus and no popup is open: Page Down and Page Up pick the next and previous part (the
///   whole frame, then the corners and edges clockwise from the top left), Home and End the first
///   and last; the arrows move the picked part as a plain drag of it would, by
///   [`steps`](Self::steps) (⇧ the big step), one `Set` per press; Esc cancels a drag only.
/// - AccessKit: a group named [`SelectionFrameLabels::group`] valued with the bounds (in the
///   [`units`](Self::units)), parts named by the labels; the picked part is the active
///   descendant, with the focus ring round it when the keyboard moved last.
/// - A screen-reader step on a part during a drag of another waits until the drag ends (the edit
///   [`Gesture`](crate::edit::Gesture) host).
/// - Nothing mirrors in a right-to-left interface: the box is the document's.
pub struct SelectionFrame<'a> {
    id_salt: IdSalt,
    state: &'a BoxState,
    labels: Option<SelectionFrameLabels>,
    rules: Option<BoxRules>,
    color: Option<Color32>,
    key_object: bool,
    centre_mark: bool,
    grips: bool,
    grow: u8,
    units: Option<(&'a RulerUnits, &'a RulerUnits)>,
    value: Option<String>,
    steps: (f64, f64),
    modifiers: Option<Modifiers>,
    policy: EditPolicy,
}

impl<'a> SelectionFrame<'a> {
    /// The frame of the host's box `state` (document units, corners clockwise from the top left).
    pub fn new(id_salt: impl AsIdSalt, state: &'a BoxState) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            state,
            labels: None,
            rules: None,
            color: None,
            key_object: false,
            centre_mark: false,
            grips: true,
            grow: 0,
            units: None,
            value: None,
            steps: (nudge::STEP, nudge::BIG_STEP),
            modifiers: None,
            policy: EditPolicy::default(),
        }
    }

    /// The AccessKit names; [`SelectionFrameLabels::of`](crate::localized::Labels::of) the
    /// installed words by default.
    pub fn labels(mut self, labels: SelectionFrameLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// How drags change the box. By default `BoxRules::resize_only()` (no rotate, skew or
    /// distort; [`ScaleRule::Free`](crate::canvas::quad::ScaleRule::Free)), the rule of
    /// VectorCraft's and DesignCraft's frames.
    pub fn rules(mut self, rules: BoxRules) -> Self {
        self.rules = Some(rules);
        self
    }

    /// The colour of the box line and the grip outlines: the selected object's layer colour.
    /// `overlay.line` by default.
    pub fn color(mut self, color: Color32) -> Self {
        self.color = Some(color);
        self
    }

    /// Draws the box with `stroke.thick`: the key object Align aligns to
    /// (DesignCraft 3 pt, VectorCraft 2.5 pt).
    pub fn key_object(mut self, key: bool) -> Self {
        self.key_object = key;
        self
    }

    /// Draws the centre mark (DesignCraft's).
    pub fn centre_mark(mut self, on: bool) -> Self {
        self.centre_mark = on;
        self
    }

    /// Whether the grips show and take presses; true by default. Off, the frame is a line only
    /// (DesignCraft with a placed graphic's content selected: the content's own frame has the
    /// grips; drawn as a second `SelectionFrame` in the content colour).
    pub fn grips(mut self, on: bool) -> Self {
        self.grips = on;
        self
    }

    /// Grows each grip by `steps` × 1 pt: VectorCraft's Selection & Anchor
    /// Display › Size, which the host passes as its size minus 3 (its default, 3, adds nothing).
    pub fn grow(mut self, steps: u8) -> Self {
        self.grow = steps;
        self
    }

    /// The ruler units of the document's x and y axes, for the AccessKit values of the group and
    /// the parts. Without them the values are left out.
    pub fn units(mut self, x: &'a RulerUnits, y: &'a RulerUnits) -> Self {
        self.units = Some((x, y));
        self
    }

    /// The group's value for a screen reader in place of the bounds, e.g. "3 objects".
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// The arrow keys' step and ⇧ step in document units; `nudge::STEP` and `nudge::BIG_STEP` by
    /// default (the host passes its own Keyboard Increment).
    pub fn steps(mut self, step: f64, big_step: f64) -> Self {
        self.steps = (step, big_step);
        self
    }

    /// The modifiers every test of the frame reads in place of the input state (a touch host's
    /// sticky modifiers).
    pub fn modifiers(mut self, modifiers: Modifiers) -> Self {
        self.modifiers = Some(modifiers);
        self
    }

    /// How the drag reports; realtime and cancellable by default.
    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Gives the frame `id` ([`SelectionFrameResponse::id`]) the keyboard.
    pub fn focus(ctx: &Context, id: Id) {
        // as `TransformBox::focus` (crates/draw/src/ui/transform/transform_box.rs:452).
        ctx.memory_mut(|m| m.request_focus(id));
    }

    /// Draws the frame over `view` and senses through the host's `canvas` response. The host
    /// calls it after the document and before its tool, so a press the frame takes never reaches
    /// the tool.
    pub fn show(self, ui: &mut Ui, canvas: &Response, view: &CanvasView) -> SelectionFrameResponse {
        // designcraft: crates/ui-egui/src/canvas/frames.rs:148-276 `draw_selection` (1 pt box in the
        // layer colour, key object 3 pt at :180, centre mark 3.5 pt at :248, eight 6.5 pt handles
        // filled white with a hair outline in the colour at :247-258; here `Handle` at 7 pt).
        // new: the press, drag, keys, parts and paint passes of `TransformBox::show`
        // (crates/draw/src/ui/transform/transform_box.rs:457-582, `press_pass` :667, `track_pass`
        // :713, `keys` :765, `parts` :796, `paint` :897) without rotation, the reference point, the
        // readout or apply and discard; the look from `SelectionFrame::style`.
        let ctx = ui.ctx().clone();
        let style = SelectionFrame::style(&current(&ctx));
        let rules = self.rules.unwrap_or_else(BoxRules::resize_only);
        let labels = self.labels.clone().unwrap_or_else(|| SelectionFrameLabels::of(ui));
        let id = ui.make_persistent_id(self.id_salt);
        let mut out = SelectionFrameResponse { id, event: None, picked: None, hovered: None, claimed: false };
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
                Turn::Runs => {
                    d.waits = false;
                    d.core.rebase(self.state);
                }
                Turn::Now => drag = None,
            }
        }
        if drag.is_none() && !Claim::keeps_waiting(&ctx, canvas.id, id, waited) {
            drag = self.press_pass(ui, canvas, view, &style, &rules, id);
            if let Some(d) = drag {
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
        if let Some(d) = &mut drag
            && !d.waits
            && !d.cancelled
        {
            self.track_pass(&ctx, view, &rules, d, &mut gesture);
        }

        let shown = *gesture.shown();
        let screen = on_screen(view, &shown);
        let parts = self.parts(&screen, &shown, &rules, &labels, &style);
        let mut focus = PartFocus::new(id, labels.group.clone()).parts(&parts).picked(picked.map(part_key));
        focus = focus.value(self.group_value(&shown));
        let focus = focus.show(ui, canvas);
        if let Some(pick) = focus.pick {
            picked = pick.and_then(key_part);
        }
        if drag.is_none_or(|d| d.cancelled) {
            // The arrows are the keyboard's: without the focus they stay with the host.
            self.keys(ui, drag.is_none() && focus.focused, picked, &rules, &mut gesture);
        }

        let dragging = drag.filter(|d| d.core.began && !d.cancelled);
        let other = Claim::owner(&ctx, canvas.id).is_some_and(|owner| owner != id);
        if let Some(d) = dragging {
            self.cursor(&ctx, view, &screen, d.core.part);
        } else if drag.is_none()
            && !other
            && let Some(pointer) = canvas.hover_pos()
        {
            let base = on_screen(view, self.state);
            out.hovered = self.takes(&base, pointer, &style, &rules);
            if let Some(part) = out.hovered {
                self.cursor(&ctx, view, &base, part);
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
        self.paint(ui, view, &screen, &style, &rules, picked, focus.keyboard);
        out
    }

    /// The modifiers the frame reads: its [`modifiers`](Self::modifiers) if set, else `held`.
    fn mods(&self, held: Modifiers) -> Modifiers {
        self.modifiers.unwrap_or(held)
    }

    /// The distance from a grip's centre within which a press takes it: half the square
    /// `size.hit.target` names, or half the grip's grown side if that is larger.
    fn reach(&self, style: &SelectionFrameStyle) -> f32 {
        style.handle.hit.max(style.handle.size + f32::from(self.grow) * style.grow_step) / 2.0
    }

    /// The part a press at screen point `p` takes from the box mapped to the screen; `None` where
    /// the press is the tool's (outside, a part the rules refuse).
    fn takes(&self, screen: &BoxState, p: Pos2, style: &SelectionFrameStyle, rules: &BoxRules) -> Option<BoxPart> {
        // Grips off: no tolerance, so only the inside is left. The frame has no reference point:
        // it is moved out of reach.
        let reach = if self.grips { f64::from(self.reach(style)) } else { 0.0 };
        let far = Point::new(f64::INFINITY, f64::INFINITY);
        let part = BoxState { pivot: far, ..*screen }.part_at(Point::new(f64::from(p.x), f64::from(p.y)), reach);
        match part {
            BoxPart::Outside | BoxPart::Pivot => None,
            part => rules.allows(part).then_some(part),
        }
    }

    /// A press on the canvas read from the input: the part it takes, claimed for the frame.
    fn press_pass(&self, ui: &Ui, canvas: &Response, view: &CanvasView, style: &SelectionFrameStyle, rules: &BoxRules, id: Id) -> Option<Drag> {
        let ctx = ui.ctx();
        if !ui.is_enabled() || !ctx.input(|i| i.pointer.primary_pressed()) {
            return None;
        }
        let press = PrimaryPress::current(ctx)?;
        if !(view.rect.contains(press.origin) && canvas.contains_pointer()) {
            return None;
        }
        let part = self.takes(&on_screen(view, self.state), press.origin, style, rules)?;
        if !Claim::take(ctx, canvas.id, id) {
            return None;
        }
        let from = view.to_doc(press.origin);
        let core = Press { part, at_press: *self.state, start: centred(self.state), from, origin: press.origin, began: false, last: *self.state };
        let waits = Claim::turn(ctx, canvas.id, id) == Turn::Waits;
        Some(Drag { core, press, cancelled: false, waits })
    }

    /// One pass of a drag read from the input: begins it on the first move, updates it, ends it
    /// on the release.
    fn track_pass(&self, ctx: &Context, view: &CanvasView, rules: &BoxRules, d: &mut Drag, gesture: &mut Gesture<BoxState>) {
        let (pointer, down, input_mods) = ctx.input(|i| (i.pointer.latest_pos(), i.pointer.primary_down(), i.modifiers));
        let mods = self.mods(input_mods);
        if let Some(pointer) = pointer {
            d.core.last = *gesture.shown();
            if !d.core.began && pointer != d.core.origin && rules.allows(d.core.part) {
                gesture.begin();
                d.core.began = gesture.is_active();
            }
            if d.core.began {
                let core = &d.core;
                let dragged = quad::drag(&core.start, &core.last, core.part, core.from, view.to_doc(pointer), mods, rules);
                d.core.last = dragged;
                gesture.update(dragged, mods);
            }
        }
        if !down && d.core.began {
            gesture.commit();
        }
    }

    /// The arrows, when `idle` (no press held, the frame focused).
    fn keys(&self, ui: &Ui, idle: bool, picked: Option<BoxPart>, rules: &BoxRules, gesture: &mut Gesture<BoxState>) {
        let ctx = ui.ctx();
        if !idle || !ui.is_enabled() || ctx.text_edit_focused() || popup_list::any_open(ctx) || !ctx.memory(|m| m.allows_interaction(ui.layer_id())) {
            return;
        }
        let (step, big) = self.steps;
        let Some(by) = nudge::read(ui, step, big) else { return };
        let part = picked.filter(|&p| rules.allows(p) && (self.grips || p == BoxPart::Inside)).unwrap_or(BoxPart::Inside);
        gesture.set(self.nudged(part, by, rules));
    }

    /// The box after an arrow key moves `part` by `by`: as a plain drag of the part would.
    fn nudged(&self, part: BoxPart, by: Vec2, rules: &BoxRules) -> BoxState {
        let s = centred(self.state);
        let at = match part {
            BoxPart::Corner(i) => s.quad[usize::from(i % 4)],
            BoxPart::Edge(i) => s.quad[usize::from(i % 4)].midpoint(s.quad[usize::from(i + 1) % 4]),
            BoxPart::Inside | BoxPart::Outside | BoxPart::Pivot => return self.state.moved(by),
        };
        quad::drag(&s, &s, part, at, at + by, Modifiers::NONE, rules)
    }

    /// The group's value: `value`, else the bounds in the units, else none.
    fn group_value(&self, shown: &BoxState) -> String {
        if let Some(value) = &self.value {
            return value.clone();
        }
        let Some((ux, uy)) = self.units else { return String::new() };
        let (min, max) = bounds(shown);
        [ux.format_value(min.x, 0.0), uy.format_value(min.y, 0.0), ux.format_value(max.x, min.x), uy.format_value(max.y, min.y)].join(", ")
    }

    /// The keyboard parts: the whole frame, then (with grips) the corners and the edges, without
    /// those the rules refuse.
    fn parts(&self, screen: &BoxState, shown: &BoxState, rules: &BoxRules, labels: &SelectionFrameLabels, style: &SelectionFrameStyle) -> Vec<Part> {
        let side = egui::Vec2::splat(self.reach(style) * 2.0);
        let q = &shown.quad;
        let mut out = Vec::with_capacity(9);
        let bounds = egui::Rect::from_points(&screen.quad.map(pos));
        out.push(Part {
            key: part_key(BoxPart::Inside),
            name: labels.whole.clone(),
            value: self.value.clone().or_else(|| Some(self.group_value(shown)).filter(|v| !v.is_empty())),
            rect: bounds,
            selected: false,
        });
        if self.grips {
            let corners = [&labels.top_left, &labels.top_right, &labels.bottom_right, &labels.bottom_left];
            let edges = [&labels.top, &labels.right, &labels.bottom, &labels.left];
            let handles = (0..4u8)
                .map(|i| (BoxPart::Corner(i), q[usize::from(i)], corners[usize::from(i)]))
                .chain((0..4u8).map(|i| (BoxPart::Edge(i), q[usize::from(i)].midpoint(q[usize::from(i + 1) % 4]), edges[usize::from(i)])));
            for (part, at, name) in handles {
                if !rules.allows(part) {
                    continue;
                }
                let value = self.units.map(|(x, y)| format!("{}, {}", x.format_value(at.x, 0.0), y.format_value(at.y, 0.0)));
                let centre = pos(screen_point(screen, part).unwrap_or(Point::ZERO));
                out.push(Part { key: part_key(part), name: name.clone(), value, rect: egui::Rect::from_center_size(centre, side), selected: false });
            }
        }
        out
    }

    /// The pointer for `part`.
    fn cursor(&self, ctx: &Context, view: &CanvasView, screen: &BoxState, part: BoxPart) {
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
                resize_cursor(view, view.unmap_vec(vec2(across.x as f32, across.y as f32)))
            }
            BoxPart::Inside => CursorIcon::Move,
            BoxPart::Outside | BoxPart::Pivot => CursorIcon::Default,
        };
        ctx.set_cursor_icon(icon);
    }

    /// Draws the box line, the centre mark, the grips and, after keyboard input, the focus round
    /// the picked part.
    #[expect(clippy::too_many_arguments, reason = "one pass's state, passed once")]
    fn paint(
        &self,
        ui: &Ui,
        view: &CanvasView,
        screen: &BoxState,
        style: &SelectionFrameStyle,
        rules: &BoxRules,
        picked: Option<BoxPart>,
        keyboard: bool,
    ) {
        let painter = ui.painter_at(view.rect);
        let color = self.color.unwrap_or(style.color);
        let width = if self.key_object { style.key_line } else { style.line };
        let corners = screen.quad.map(pos);
        painter.add(Shape::closed_line(corners.to_vec(), Stroke::new(width, color)));
        if self.centre_mark {
            let centre = pos(screen.quad[0].midpoint(screen.quad[2]));
            painter.rect_filled(egui::Rect::from_center_size(centre, egui::Vec2::splat(style.centre)), 0.0, color);
        }
        let mut handle = style.handle;
        handle.stroke.color = color;
        handle.size += f32::from(self.grow) * style.grow_step;
        let ppp = ui.pixels_per_point();
        if self.grips {
            for i in 0..4u8 {
                for part in [BoxPart::Corner(i), BoxPart::Edge(i)] {
                    if let Some(at) = screen_point(screen, part).filter(|_| rules.allows(part)) {
                        painter.extend(Handle::shapes_with(&handle, HandleShape::Square, pos(at), ppp));
                    }
                }
            }
        }
        if !keyboard {
            return;
        }
        match picked.filter(|&p| p != BoxPart::Inside && self.grips && rules.allows(p)) {
            Some(part) => {
                if let Some(at) = screen_point(screen, part) {
                    PartFocus::ring(&painter, egui::Rect::from_center_size(pos(at), egui::Vec2::splat(handle.size)));
                }
            }
            None => {
                let ring = FocusRing::style(&current(ui.ctx()));
                let halo = current(ui.ctx()).stroke.thin;
                let focus_halo = Stroke::new(ring.stroke.width + 2.0 * halo, current(ui.ctx()).color.overlay.halo.to_egui());
                let line = outset(&corners, width / 2.0 + ring.offset + ring.stroke.width / 2.0);
                painter.add(Shape::closed_line(line.to_vec(), focus_halo));
                painter.add(Shape::closed_line(line.to_vec(), ring.stroke));
            }
        }
    }
}

/// The press of a drag: the part taken, the box at the press and the box the drag works from.
#[derive(Clone, Copy, Debug)]
struct Press {
    part: BoxPart,
    at_press: BoxState,
    /// `at_press` with the reference point at its centre, so ⌥ scales about the centre.
    start: BoxState,
    /// The press in document space.
    from: Point,
    /// The press in canvas space.
    origin: Pos2,
    began: bool,
    /// The box the drag last produced.
    last: BoxState,
}

impl Press {
    /// Takes the host's box as the press's start when its turn comes: a typing run that ended
    /// first may have changed it.
    fn rebase(&mut self, state: &BoxState) {
        self.at_press = *state;
        self.last = *state;
        self.start = centred(state);
    }
}

/// A drag [`SelectionFrame::show`] keeps between passes.
#[derive(Clone, Copy, Debug)]
struct Drag {
    core: Press,
    press: PrimaryPress,
    /// Esc, an outside change or a missed pass ended the gesture; the rest of the press does
    /// nothing.
    cancelled: bool,
    /// The press waits for another control's typing run to end ([`Claim::turn`]); nothing of it
    /// has run.
    waits: bool,
}

fn drag_key(id: Id) -> Id {
    id.with("craft_ui::canvas::SelectionFrame::drag")
}

fn pick_key(id: Id) -> Id {
    id.with("craft_ui::canvas::SelectionFrame::picked")
}

/// The [`Part`] key of a box part.
fn part_key(part: BoxPart) -> u64 {
    match part {
        BoxPart::Inside | BoxPart::Outside | BoxPart::Pivot => 0,
        BoxPart::Corner(i) => 1 + u64::from(i % 4),
        BoxPart::Edge(i) => 5 + u64::from(i % 4),
    }
}

fn key_part(key: u64) -> Option<BoxPart> {
    match key {
        0 => Some(BoxPart::Inside),
        1..=4 => Some(BoxPart::Corner((key - 1) as u8)),
        5..=8 => Some(BoxPart::Edge((key - 5) as u8)),
        _ => None,
    }
}

/// `state` with the reference point at the centre of its quad.
fn centred(state: &BoxState) -> BoxState {
    BoxState { pivot: state.quad[0].midpoint(state.quad[2]), ..*state }
}

/// The box's corner extremes in document units.
fn bounds(state: &BoxState) -> (Point, Point) {
    let fold = |f: fn(f64, f64) -> f64, init: f64, get: fn(&Point) -> f64| state.quad.iter().map(get).fold(init, f);
    (
        Point::new(fold(f64::min, f64::INFINITY, |p| p.x), fold(f64::min, f64::INFINITY, |p| p.y)),
        Point::new(fold(f64::max, f64::NEG_INFINITY, |p| p.x), fold(f64::max, f64::NEG_INFINITY, |p| p.y)),
    )
}

/// The box as `view` draws it, in screen points.
fn on_screen(view: &CanvasView, s: &BoxState) -> BoxState {
    let a = view.affine();
    BoxState { quad: s.quad.map(|p| a * p), pivot: a * s.pivot }
}

fn pos(p: Point) -> Pos2 {
    Pos2::new(p.x as f32, p.y as f32)
}

/// The handle's screen position; `None` for the inside and the outside.
fn screen_point(screen: &BoxState, part: BoxPart) -> Option<Point> {
    let q = &screen.quad;
    match part {
        BoxPart::Corner(i) => Some(q[usize::from(i % 4)]),
        BoxPart::Edge(i) => Some(q[usize::from(i % 4)].midpoint(q[usize::from(i + 1) % 4])),
        BoxPart::Inside | BoxPart::Outside | BoxPart::Pivot => None,
    }
}
