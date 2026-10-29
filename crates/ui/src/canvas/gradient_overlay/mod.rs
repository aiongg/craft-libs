//! [`GradientOverlay`]: a gradient drawn on the canvas as PhotoCraft's live Gradient tool draws
//! it, a line between a start and an end handle with the colour stops and midpoint diamonds on it.

mod keys;
#[cfg(test)]
mod order_tests;
mod parts;
mod pointer;
#[cfg(test)]
mod share_tests;
pub(crate) mod story;
#[cfg(test)]
mod tests;

use std::f32::consts::FRAC_PI_2;

use egui::{AsIdSalt, Color32, Context, CursorIcon, Id, IdSalt, Key, Modifiers, Painter, Pos2, Rect, Response, Shape, Stroke, Ui, vec2};
use kurbo::Point;

use super::gradient_line::{at_screen, midpoint_location};
use super::{CanvasView, Claim, GradientExtent, GradientLine, LineGradient, LinePart, PartFocus, RulerUnits};
use crate::behavior::{ClaimedKey, KeyClaim};
use crate::composites::{
    GradientEditor, GradientLabels, GradientStops, StopRef, StopRow, StopRows, StopRules, StopSampler as Sampler, default_sampler,
};
use crate::edit::{EditEvent, EditPolicy, Gesture};
use crate::localized::LocaleNumbers;
use crate::primitives::FocusRing;
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens};

/// The names a screen reader reads for the overlay's group and parts, translated by the host.
#[derive(Clone, Debug, PartialEq)]
pub struct GradientLineNames {
    /// The group: "Gradient".
    pub group: String,
    /// The line between the handles.
    pub line: String,
    /// The start handle: "Start point".
    pub start: String,
    /// The end handle: "End point".
    pub end: String,
}

/// A gradient line's look and reach, from [`GradientOverlay::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct GradientLineStyle {
    /// The line and the circle and square extents: `gradient_line.line` of `overlay.handle_fill`.
    pub line: Stroke,
    /// The outline each side of those: `gradient_line.halo` of `overlay.halo`.
    pub halo: Stroke,
    /// Dash and gap of the mirrored extent: `gradient_line.dash`, `gradient_line.gap`.
    pub dash: (f32, f32),
    /// Radius of an end handle's ring: `gradient_line.end_ring`.
    pub end_ring: f32,
    /// An end handle's ring: `gradient_line.end_width` of `overlay.handle_fill`.
    pub end: Stroke,
    /// The outline each side of the ring: `gradient_line.end_halo` of `overlay.halo`.
    pub end_halo: Stroke,
    /// Radius of a colour stop's disc: `gradient_line.stop`.
    pub stop: f32,
    /// The ring just outside the disc: `gradient_line.stop_ring` of `overlay.handle_fill`.
    pub stop_ring: Stroke,
    /// The 1 pt ring outside that: `handle.halo` of `overlay.halo`.
    pub stop_halo: Stroke,
    /// Radius of a midpoint's diamond: `gradient_line.diamond`.
    pub diamond: f32,
    /// A diamond's fill: `overlay.handle_fill`.
    pub diamond_fill: Color32,
    /// A diamond's outline: `handle.halo` of `overlay.halo`.
    pub diamond_halo: Stroke,
    /// The selected stop's ring and the selected diamond's fill: `overlay.handle_stroke`.
    pub selected: Color32,
    /// The fill under a stop's colour where the colour has opacity: `overlay.handle_fill`.
    pub backing: Color32,
    /// Distance from a part within which the pointer takes it: `gradient_line.hit`.
    pub hit: f32,
    /// Farthest a press may move and still be a click: `gradient_line.click_slop`.
    pub click_slop: f32,
    /// Distance from the line past which a dragged stop is torn off: `gradient_line.tear_off`.
    pub tear_off: f32,
    /// The lines beside the picked line and the ring round a picked part: the focus ring's stroke.
    pub focus: Stroke,
    /// Under each focus line, so it reads on any image: `overlay.halo`, `handle.halo` wider than
    /// the line on each side.
    pub focus_halo: Stroke,
    /// From the line's edge to the focus lines: the focus ring's offset.
    pub focus_offset: f32,
}

/// What the pointer is over or doing, for the host's cursor ([`GradientOverlay::cursor`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LineCursor {
    /// Over a part a press would take.
    Hover(LinePart),
    /// Dragging a part (an Alt copy: the stop it copies).
    Drag(LinePart),
    /// Dragging a stop torn off the line: the release removes it.
    TornOff,
}

/// The host's snapping of a dragged end or of the line's start ([`GradientOverlay::snap`]).
type Snap<'a> = Box<dyn Fn(LinePart, Point, Modifiers) -> Point + 'a>;

/// The host's cursors ([`GradientOverlay::cursor`]).
type Cursor<'a> = Box<dyn Fn(LineCursor) -> CursorIcon + 'a>;

/// A gradient's line over a canvas: the line between its handles, the circle, square or mirrored
/// half its style adds, and the colour stops and midpoint diamonds on it, as PhotoCraft's Gradient
/// tool draws them. The host holds the gradient and passes the line and stops each frame.
///
/// ```
/// use craft_ui::canvas::{CanvasView, GradientLine, GradientLineNames, GradientOverlay, kurbo::Point};
/// use craft_ui::composites::{ColorStop, GradientStops};
/// use craft_theme::Srgba;
/// # egui::__run_test_ui(|ui| {
/// let (rect, canvas) = ui.allocate_exact_size(egui::vec2(400.0, 300.0), egui::Sense::click_and_drag());
/// let view = CanvasView::new(rect, Point::new(200.0, 150.0), 1.0);
/// let line = GradientLine { start: Point::new(120.0, 150.0), end: Point::new(280.0, 150.0) };
/// let stops = GradientStops {
///     color: vec![ColorStop::new(0.0, Srgba::BLACK), ColorStop::new(1.0, Srgba::WHITE)],
///     opacity: vec![],
/// };
/// let names = GradientLineNames { group: "Gradient".into(), line: "Gradient line".into(), start: "Start point".into(), end: "End point".into() };
/// GradientOverlay::new("gradient", &line, &stops, names).show(ui, &canvas, &view);
/// # });
/// ```
///
/// - Look: a 1.5 pt `overlay.handle_fill` line over an `overlay.halo` outline, the extent in the
///   line's style (the mirrored half dashed, no outline), rings at the ends, a disc in each stop's
///   colour with a ring round it, and a diamond between stops. Drawn in that order, so a stop at
///   an end sits inside its ring. Points go through the view, so a rotated, flipped or stretched
///   view draws the line where the document has it.
/// - Under [`StopRows::Split`] and [`StopRows::ColorOnly`] a stop's disc is opaque; under [`StopRows::Combined`] its colour
///   shows its opacity over a white backing.
/// - With [`reverse`](Self::reverse) location `t` is drawn at `1 − t` from the start.
/// - The selected stop or midpoint ([`select`](Self::select)) is drawn with its ring or fill in
///   `overlay.handle_stroke`, whether or not the overlay has focus.
/// - Nothing mirrors in a right-to-left interface: the line is the document's.
///
/// Pointer (PhotoCraft's live Gradient tool, VectorCraft where PhotoCraft has none):
///
/// - The overlay senses through the host's canvas response and takes a press on one of its parts
///   with [`Claim`](super::Claim), before the host's guides and tool. A press off every part, or
///   with ⌘ (Ctrl) held, is the host's.
/// - Parts within `gradient_line.hit` of the pointer, in this order: the end handle, the start
///   handle, the nearest stop, the nearest midpoint, the line.
/// - A drag begins past `gradient_line.click_slop`. An end follows the pointer (⇧: 45° steps from
///   [`angle_base`](Self::angle_base) about the other end); the line moves both ends; a stop moves
///   to the location under the pointer and is torn off (removed while its row is above
///   `rules.min`) more than `gradient_line.tear_off` from the line; a midpoint moves within its
///   segment. Alt drags a copy of a stop with [`alt_copies`](Self::alt_copies). Every update is
///   the whole edit of the gradient at the press, so a stop torn off and brought back is as it
///   was.
/// - A press on a stop or midpoint selects it; the selection follows a dragged stop. A click on the
///   line adds a colour stop there and selects it. A double-click on a stop, or on an end handle
///   for the stop at that end, asks for its colour ([`color_requested`](GradientOverlayResponse::color_requested)).
///   A secondary click reports the part as [`context`](GradientOverlayResponse::context).
/// - Esc during a drag cancels it; the rest of the press does nothing.
/// - A press while another control's typing run has the focus (the panel's Location field) waits
///   for the run's Commit and begins from the value it left. When the host ends the overlay's drag
///   for another control's change, the drag goes on in the same pass from the host's value, its stop at the pointer, in a
///   gesture of its own.
///
/// Keys (one Tab stop through [`PartFocus`]; the release of a press on a part gives the group
/// focus too):
///
/// - Page Down and Page Up pick the next and previous part, Home and End the first and last: the
///   line, the start, the stops and midpoints in location order, the end. Picking a stop or
///   midpoint selects it; a handle or the line keeps the selection. The picked end is drawn
///   selected.
/// - Left and Right move a picked stop or midpoint by 1 %, 10 % with Shift, Left always lowering
///   the location; Up and Down are the host's. The arrows move a picked end, or the line (with
///   nothing picked too), by 1 document unit, 10 with Shift. Delete and Backspace remove a stop,
///   Insert and Cmd/Ctrl+Enter add one halfway after the picked stop, and the stop that takes
///   the picked index or the new stop is picked and selected. Enter and Space on a colour stop
///   or an end set [`color_requested`](GradientOverlayResponse::color_requested). Esc cancels a
///   drag and is otherwise the host's. Every key needs exactly its modifiers. Each change is one
///   Set.
/// - After keyboard input a focus ring shows round the picked handle, stop or diamond, and two
///   focus lines beside the picked line.
/// - The group's AccessKit children are the parts, named and valued as the bar's markers are
///   ([`labels`](Self::labels), [`numbers`](Self::numbers)), the end handles valued through
///   [`units`](Self::units).
///
/// With [`selection_of`](Self::selection_of) the overlay and a gradient editor share one selected
/// stop. A selection another control makes becomes the picked part, and a stop moved past its
/// neighbour on the canvas moves the shared selection with it.
pub struct GradientOverlay<'a> {
    id_salt: IdSalt,
    line: &'a GradientLine,
    stops: &'a GradientStops,
    names: GradientLineNames,
    rules: StopRules,
    sampler: Option<&'a Sampler>,
    midpoints: bool,
    reverse: bool,
    extent: GradientExtent,
    editable: bool,
    alt_copies: bool,
    angle_base: f64,
    snap: Option<Snap<'a>>,
    cursor: Option<Cursor<'a>>,
    policy: EditPolicy,
    labels: GradientLabels,
    numbers: Option<&'a LocaleNumbers>,
    location_decimals: usize,
    units: Option<(&'a RulerUnits, &'a RulerUnits)>,
    shared: Option<Id>,
    value: Option<String>,
}

impl<'a> GradientOverlay<'a> {
    /// The host's gradient `line` and `stops`, named `names` in the AccessKit tree.
    pub fn new(id_salt: impl AsIdSalt, line: &'a GradientLine, stops: &'a GradientStops, names: GradientLineNames) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            line,
            stops,
            names,
            rules: StopRules::default(),
            sampler: None,
            midpoints: true,
            reverse: false,
            extent: GradientExtent::None,
            editable: true,
            alt_copies: false,
            angle_base: 0.0,
            snap: None,
            cursor: None,
            policy: EditPolicy::default(),
            labels: GradientLabels::default(),
            numbers: None,
            location_decimals: 0,
            units: None,
            shared: None,
            value: None,
        }
    }

    /// The rules the host gives its gradient editor: the row layout, the fewest and most stops and
    /// the midpoint range.
    pub fn rules(mut self, rules: StopRules) -> Self {
        self.rules = rules;
        self
    }

    /// Colours a stop added on the line; [`default_sampler`] of the rules' rows by default.
    pub fn sampler(mut self, sampler: &'a Sampler) -> Self {
        self.sampler = Some(sampler);
        self
    }

    /// Whether an Alt-drag on a stop drags a copy of it (VectorCraft); off by default, where Alt is
    /// the host's (PhotoCraft's eyedropper).
    pub fn alt_copies(mut self, copies: bool) -> Self {
        self.alt_copies = copies;
        self
    }

    /// The angle in degrees, along the document's axes, that ⇧'s 45° steps of a dragged end start
    /// from (VectorCraft's Constrain Angle); 0 by default.
    pub fn angle_base(mut self, degrees: f64) -> Self {
        self.angle_base = degrees;
        self
    }

    /// The host's snapping of a dragged point (anchors, smart guides): an end handle's new place,
    /// or for [`LinePart::Line`] the start's new place in a line drag. Given after ⇧'s 45° step.
    pub fn snap(mut self, snap: impl Fn(LinePart, Point, Modifiers) -> Point + 'a) -> Self {
        self.snap = Some(Box::new(snap));
        self
    }

    /// The host's pointer over and during a drag of each part; none by default (PhotoCraft).
    pub fn cursor(mut self, cursor: impl Fn(LineCursor) -> CursorIcon + 'a) -> Self {
        self.cursor = Some(Box::new(cursor));
        self
    }

    /// The drag's update rate and cost; [`EditPolicy::default`] by default.
    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// The words of the stops' and midpoints' accessible names and values, the host's
    /// [`GradientLabels`] for its gradient editor, so a stop reads the same in the panel and on the
    /// canvas.
    pub fn labels(mut self, labels: GradientLabels) -> Self {
        self.labels = labels;
        self
    }

    /// The numbers' format of the stops' accessible values; English by default. Give the editor's.
    pub fn numbers(mut self, numbers: &'a LocaleNumbers) -> Self {
        self.numbers = Some(numbers);
        self
    }

    /// Decimals of a stop's location in its accessible value; 0 by default, as the editor's.
    pub fn location_decimals(mut self, decimals: usize) -> Self {
        self.location_decimals = decimals;
        self
    }

    /// The ruler units of the document's x and y axes: the end handles' accessible values. They
    /// have none without.
    pub fn units(mut self, x: &'a RulerUnits, y: &'a RulerUnits) -> Self {
        self.units = Some((x, y));
        self
    }

    /// Shares the selected stop with the [`GradientEditor`] whose id is `editor`
    /// ([`GradientEditor::id_in`]): the overlay reads and writes the editor's selection, so a stop
    /// picked on the canvas is the one under the panel's bar and the other way round. Without it
    /// the overlay keeps a selection of its own ([`selected`](Self::selected)).
    pub fn selection_of(mut self, editor: Id) -> Self {
        self.shared = Some(editor);
        self
    }

    /// The group's and the line's accessible value; "{n} stops" ([`GradientLabels::bar_stops`])
    /// by default.
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Whether midpoint diamonds show; true by default.
    pub fn midpoints(mut self, midpoints: bool) -> Self {
        self.midpoints = midpoints;
        self
    }

    /// Whether the gradient runs from the end to the start.
    pub fn reverse(mut self, reverse: bool) -> Self {
        self.reverse = reverse;
        self
    }

    /// The shape the gradient's style adds round the line; decoration only.
    pub fn extent(mut self, extent: GradientExtent) -> Self {
        self.extent = extent;
        self
    }

    /// Whether the overlay takes presses and shows the selection; true by default. False draws the
    /// line and its parts only (the line a tool is drawing, a dialog over the gradient).
    pub fn editable(mut self, editable: bool) -> Self {
        self.editable = editable;
        self
    }

    /// The overlay `id`'s selected stop or midpoint, when it keeps its own selection (no
    /// [`selection_of`](Self::selection_of)).
    pub fn selected(ctx: &Context, id: Id) -> Option<StopRef> {
        ctx.data(|d| d.get_temp(id.with(SELECTED)))
    }

    /// Selects a stop or midpoint of the overlay `id`, when it keeps its own selection.
    pub fn select(ctx: &Context, id: Id, selected: Option<StopRef>) {
        ctx.data_mut(|d| match selected {
            Some(s) => {
                d.insert_temp(id.with(SELECTED), s);
            }
            None => {
                d.remove::<StopRef>(id.with(SELECTED));
            }
        });
    }

    pub fn style(tokens: &UiTokens) -> GradientLineStyle {
        let (g, o, h) = (&tokens.component.gradient_line, &tokens.color.overlay, &tokens.component.handle);
        let (fill, halo) = (o.handle_fill.to_egui(), o.halo.to_egui());
        let ring = FocusRing::style(tokens);
        GradientLineStyle {
            line: Stroke::new(g.line, fill),
            halo: Stroke::new(g.halo, halo),
            dash: (g.dash, g.gap),
            end_ring: g.end_ring,
            end: Stroke::new(g.end_width, fill),
            end_halo: Stroke::new(g.end_halo, halo),
            stop: g.stop,
            stop_ring: Stroke::new(g.stop_ring, fill),
            stop_halo: Stroke::new(h.halo, halo),
            diamond: g.diamond,
            diamond_fill: fill,
            diamond_halo: Stroke::new(h.halo, halo),
            selected: o.handle_stroke.to_egui(),
            backing: fill,
            hit: g.hit,
            click_slop: g.click_slop,
            tear_off: g.tear_off,
            focus: ring.stroke,
            focus_halo: Stroke::new(ring.stroke.width + 2.0 * h.halo, halo),
            focus_offset: ring.offset,
        }
    }

    /// The selected stop or midpoint: the shared editor's, or the overlay's own.
    fn selection(&self, ctx: &Context, id: Id) -> Option<StopRef> {
        match self.shared {
            Some(editor) => GradientEditor::selected(ctx, editor),
            None => Self::selected(ctx, id),
        }
    }

    fn store_selection(&self, ctx: &Context, id: Id, selected: Option<StopRef>) {
        match self.shared {
            Some(editor) => GradientEditor::select(ctx, editor, selected),
            None => Self::select(ctx, id, selected),
        }
    }

    /// Draws the gradient over `view` and senses through the host's `canvas` response.
    pub fn show(self, ui: &mut Ui, canvas: &Response, view: &CanvasView) -> GradientOverlayResponse {
        let ctx = ui.ctx().clone();
        let id = ui.make_persistent_id(self.id_salt);
        let style = GradientOverlay::style(&Tokens::of(ui));
        let editable = self.editable && ui.is_enabled();
        let default = default_sampler(self.rules.rows);
        let sampler: &Sampler = self.sampler.unwrap_or(&default);
        let pass = pointer::Pass { ctx: &ctx, id, canvas, view, style: &style, sampler };
        let mut out = GradientOverlayResponse { id, event: None, color_requested: None, picked: None, hovered: None, context: None, claimed: false };
        let at_load = pointer::Selection { selected: self.selection(&ctx, id), picked: ctx.data(|d| d.get_temp(pointer::pick_key(id))) };
        let mut sel = at_load;
        let value = LineGradient { line: *self.line, stops: self.stops.clone() };
        // A selection another control made (the panel's bar) is the one the keys act on.
        let seen = ctx.data(|d| d.get_temp::<Option<StopRef>>(pointer::seen_key(id))).unwrap_or(at_load.selected);
        if editable
            && at_load.selected != seen
            && let Some(part) = self.part_selected(&value, at_load.selected)
        {
            sel.picked = Some(part);
        }

        let press_held = ctx.data(|d| d.get_temp::<pointer::Drag>(pointer::drag_key(id))).is_some();
        let drag = self.press_of(&pass, &value, editable, &mut sel);
        let reported = Gesture::<LineGradient>::reported(ui, id).map(|(_, value)| value);
        // The drag's own last report stands for the host's value while the host shows it or still
        // shows the value at the press, so applying updates live is no outside change.
        let host = match (&drag, &reported) {
            (Some(d), Some(r)) if d.began && !d.cancelled && (value == d.base || value == *r) => r.clone(),
            _ => value,
        };
        let mut gesture = Gesture::load(ui, id, &host, self.policy).focus_owners(&[]).enabled(editable);
        let drag = drag.and_then(|d| self.track(&pass, d, &mut gesture, reported.as_ref(), &mut sel));
        if editable && drag.is_none() {
            self.clicks(&pass, gesture.shown(), &mut sel, &mut out);
        }
        let dragging = drag.as_ref().filter(|d| d.began && !d.cancelled).map(|d| d.part);
        if editable && drag.is_none() {
            out.hovered = self.hover(&pass, gesture.shown());
        }
        // The release of a press the overlay held gives the group the keyboard, so the keys act on
        // what the pointer took, even where a drag has moved the part from under the press.
        if editable && press_held && drag.is_none() {
            ctx.memory_mut(|m| m.request_focus(id));
        }

        let english;
        let numbers = match self.numbers {
            Some(numbers) => numbers,
            None => {
                english = LocaleNumbers::from_tag("en");
                &english
            }
        };
        let parts = self.parts(&pass, gesture.shown(), numbers, sel.selected.filter(|_| editable));
        let mut focus = PartFocus::new(id, &self.names.group).parts(&parts.list).picked(sel.picked.map(parts::part_key)).focusable(editable);
        focus = focus.value(self.group_value(gesture.shown(), numbers));
        let focus = focus.show(ui, canvas);
        match focus.pick {
            Some(Some(key)) => {
                sel.picked = parts::key_part(key);
                if let Some(selected) = sel.picked.and_then(pointer::selection_of) {
                    sel.selected = Some(selected);
                }
            }
            Some(None) => sel.picked = None,
            None => {}
        }
        if focus.focused {
            KeyClaim::claim(&ctx, id, &CLAIMED);
            if editable && drag.is_none() {
                self.keys(ui, &pass, &mut gesture, &mut sel, &mut out);
            }
        }

        let shown = gesture.shown().clone();
        out.event = gesture.finish();
        out.claimed = Claim::is_owner(&ctx, canvas.id, id);
        out.picked = sel.picked;
        ctx.data_mut(|d| {
            match &drag {
                Some(drag) => {
                    d.insert_temp(pointer::drag_key(id), drag.clone());
                }
                None => d.remove::<pointer::Drag>(pointer::drag_key(id)),
            }
            match sel.picked {
                Some(part) => {
                    d.insert_temp(pointer::pick_key(id), part);
                }
                None => d.remove::<LinePart>(pointer::pick_key(id)),
            }
            d.insert_temp(pointer::seen_key(id), sel.selected);
        });
        if sel.selected != at_load.selected {
            self.store_selection(&ctx, id, sel.selected);
        }

        let selected = if editable { sel.selected } else { None };
        let marked = dragging.or(sel.picked.filter(|p| editable && matches!(p, LinePart::Start | LinePart::End)));
        let painter = ui.painter_at(view.rect);
        self.paint(&painter, &style, view, &shown, selected, marked);
        // With no part picked the keys act on the line, so it carries the focus marks.
        if focus.keyboard {
            self.paint_focus(&painter, &style, view, &shown, sel.picked.unwrap_or(LinePart::Line));
        }
        out
    }

    /// Draws `value`; `marked` is the end a drag holds or the keys picked, drawn selected.
    fn paint(
        &self,
        painter: &Painter,
        style: &GradientLineStyle,
        view: &CanvasView,
        value: &LineGradient,
        selected: Option<StopRef>,
        marked: Option<LinePart>,
    ) {
        let (start, end) = (view.to_screen(value.line.start), view.to_screen(value.line.end));
        let stops = &value.stops;
        self.paint_extent(painter, style, start, end);
        painter.extend(line_shapes(style, start, end));
        if self.midpoints {
            for i in 0..stops.color.len() {
                if let Some(t) = midpoint_location(stops, i) {
                    let on = selected == Some(StopRef::Midpoint(StopRow::Color, i));
                    painter.add(diamond(style, at_screen(start, end, t, self.reverse), on));
                }
            }
        }
        for (p, part) in [(start, LinePart::Start), (end, LinePart::End)] {
            let ring = if marked == Some(part) { Stroke::new(style.end.width, style.selected) } else { style.end };
            // The ring is centred on `end_ring`; a circle strokes outside its radius.
            let inner = style.end_ring - style.end.width / 2.0;
            painter.add(Shape::circle_stroke(p, inner - style.end_halo.width, style.end_halo_stroke()));
            painter.add(Shape::circle_stroke(p, inner, ring));
        }
        for (i, stop) in stops.color.iter().enumerate() {
            let at = at_screen(start, end, stop.location, self.reverse);
            let on = selected == Some(StopRef::Color(i));
            painter.extend(self.stop_shapes(style, at, Color32::from(stop.color), on));
        }
    }

    fn paint_extent(&self, painter: &Painter, style: &GradientLineStyle, start: Pos2, end: Pos2) {
        let d = end - start;
        match self.extent {
            GradientExtent::None => {}
            GradientExtent::Circle => {
                // A circle strokes outside its radius: the line's centre is the extent's edge.
                let radius = d.length() - style.line.width / 2.0;
                let halo = Stroke::new(style.line.width + 2.0 * style.halo.width, style.halo.color);
                painter.add(Shape::circle_stroke(start, radius - style.halo.width, halo));
                painter.add(Shape::circle_stroke(start, radius, style.line));
            }
            GradientExtent::Square => {
                let turned = vec2(-d.y, d.x);
                let corners = vec![start + d, start + turned, start - d, start - turned];
                painter.extend(outlined(style, |stroke| vec![Shape::closed_line(corners.clone(), stroke)]));
            }
            GradientExtent::Mirror => {
                let far = start - d;
                painter.extend(Shape::dashed_line(&[start, far], style.line, style.dash.0, style.dash.1));
            }
        }
    }

    /// A colour stop: the disc, the ring just outside it, the halo ring outside that.
    fn stop_shapes(&self, style: &GradientLineStyle, at: Pos2, color: Color32, selected: bool) -> Vec<Shape> {
        let mut out = Vec::with_capacity(4);
        let (r, ring) = (style.stop, style.stop_ring.width);
        out.push(Shape::circle_stroke(at, r + ring, style.stop_halo));
        match self.rules.rows {
            StopRows::Split | StopRows::ColorOnly => out.push(Shape::circle_filled(at, r, Color32::from_rgb(color.r(), color.g(), color.b()))),
            StopRows::Combined => {
                out.push(Shape::circle_filled(at, r, style.backing));
                out.push(Shape::circle_filled(at, r, color));
            }
        }
        let ring_color = if selected { style.selected } else { style.stop_ring.color };
        out.push(Shape::circle_stroke(at, r, Stroke::new(ring, ring_color)));
        out
    }
}

/// What [`GradientOverlay::show`] reports.
#[derive(Clone, Debug, PartialEq)]
pub struct GradientOverlayResponse {
    /// The overlay's id, also its AccessKit group's.
    pub id: Id,
    /// A drag (Begin past the click slop, Update, Commit on release; Esc cancels it) or a stop
    /// added by a click on the line (Set).
    pub event: Option<EditEvent<LineGradient>>,
    /// A double-click asked for the colour of colour stop `i`; the rect is its marker on screen,
    /// for a picker opened beside it.
    pub color_requested: Option<(usize, Rect)>,
    /// The part the keys act on, picked by the last press.
    pub picked: Option<LinePart>,
    /// The part a press at the pointer would take; `None` where it is the host's.
    pub hovered: Option<LinePart>,
    /// A secondary click on a part, for the host's context menu.
    pub context: Option<LinePart>,
    /// Whether the overlay owns the current press: the host skips its guides and tool input.
    pub claimed: bool,
}

impl GradientLineStyle {
    /// The halo ring under an end handle's ring: wide enough to show `end_halo` each side of it.
    fn end_halo_stroke(&self) -> Stroke {
        Stroke::new(self.end.width + 2.0 * self.end_halo.width, self.end_halo.color)
    }
}

/// The keys a focused overlay keeps from the app's shortcuts.
const CLAIMED: [ClaimedKey; 4] = [
    ClaimedKey::exact(Modifiers::NONE, Key::Delete),
    ClaimedKey::exact(Modifiers::NONE, Key::Backspace),
    ClaimedKey::exact(Modifiers::NONE, Key::Insert),
    ClaimedKey::exact(Modifiers::COMMAND, Key::Enter),
];

const SELECTED: &str = "craft_ui::canvas::GradientOverlay::selected";

/// Strokes in the line's style: the halo under, then the line.
fn outlined(style: &GradientLineStyle, shape: impl Fn(Stroke) -> Vec<Shape>) -> Vec<Shape> {
    let halo = Stroke::new(style.line.width + 2.0 * style.halo.width, style.halo.color);
    let mut out = shape(halo);
    out.extend(shape(style.line));
    out
}

fn line_shapes(style: &GradientLineStyle, start: Pos2, end: Pos2) -> Vec<Shape> {
    outlined(style, |stroke| vec![Shape::line_segment([start, end], stroke)])
}

/// A midpoint diamond centred on `at`.
fn diamond(style: &GradientLineStyle, at: Pos2, selected: bool) -> Shape {
    let r = style.diamond;
    let points = (0..4).map(|k| at + vec2((k as f32 * FRAC_PI_2).cos(), (k as f32 * FRAC_PI_2).sin()) * r).collect();
    let fill = if selected { style.selected } else { style.diamond_fill };
    Shape::convex_polygon(points, fill, style.diamond_halo)
}
