//! [`Guides`]: ruler guides on a canvas, and [`SmartGuides`]: the alignment lines and distances a
//! host shows while something moves.

use std::ops::Range;

use egui::accesskit::Role;
use egui::emath::GuiRounding as _;
use egui::{
    Align2, AsIdSalt, Color32, Context, CornerRadius, EventFilter, Id, IdSalt, Key, Modifiers, Painter, Pos2, Response, Sense, Shape, Stroke, Ui,
    UiBuilder, Vec2 as ScreenVec, WidgetInfo, vec2,
};
use kurbo::{Point, Rect, Vec2};

use craft_theme::{Rgba, TypeRole, contrast_ratio};

use super::{CanvasView, Claim, Readout, RulerUnits, crisp_shape, hit, resize_cursor};
use crate::behavior::nudge;
use crate::edit::{CancelReason, EditEvent, EditPolicy, Gesture};
use crate::localized::LocaleNumbers;
use crate::primitives::FocusRing;
use crate::story::{Args, Level, Story};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens};

/// Within this many points of its press a guide's release is a click (PhotoCraft's
/// `finish_drag`).
const CLICK: f64 = 0.5;

/// Which way a guide runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Orientation {
    /// Across the document's x axis, at a y position.
    Horizontal,
    /// Along the document's y axis, at an x position.
    Vertical,
}

impl Orientation {
    pub fn flipped(self) -> Orientation {
        match self {
            Orientation::Horizontal => Orientation::Vertical,
            Orientation::Vertical => Orientation::Horizontal,
        }
    }

    /// The document direction a guide moves in: x for a vertical guide.
    pub fn across(self) -> Vec2 {
        match self {
            Orientation::Horizontal => Vec2::new(0.0, 1.0),
            Orientation::Vertical => Vec2::new(1.0, 0.0),
        }
    }

    /// The coordinate of `p` a guide is placed by: x for a vertical guide.
    pub fn position(self, p: Point) -> f64 {
        match self {
            Orientation::Horizontal => p.y,
            Orientation::Vertical => p.x,
        }
    }

    /// The coordinate of `p` along a guide.
    fn along(self, p: Point) -> f64 {
        self.flipped().position(p)
    }

    /// The point `along` units along a guide at `position`.
    fn point(self, position: f64, along: f64) -> Point {
        match self {
            Orientation::Horizontal => Point::new(along, position),
            Orientation::Vertical => Point::new(position, along),
        }
    }

    /// The `lo..hi` of `r` across a guide (on the axis it is placed by).
    fn range(self, r: Rect) -> (f64, f64) {
        match self {
            Orientation::Horizontal => (r.y0, r.y1),
            Orientation::Vertical => (r.x0, r.x1),
        }
    }

    /// Screen points per document unit across a guide.
    fn zoom(self, view: &CanvasView) -> f64 {
        match self {
            Orientation::Horizontal => view.zoom,
            Orientation::Vertical => view.zoom_x(),
        }
    }
}

/// How a guide's line is drawn (PhotoCraft's Guides preference; DesignCraft's liquid guides are
/// dashed).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GuideLine {
    #[default]
    Solid,
    Dashed,
    Dots,
}

/// A guide of the host's document.
#[derive(Clone, Debug, PartialEq)]
pub struct Guide {
    /// The host's name for the guide, unique among the guides shown together.
    pub key: u64,
    pub orientation: Orientation,
    /// Document coordinate across the guide: x for a vertical guide.
    pub position: f64,
    /// Where the guide runs along its length; `None` across the whole view (PhotoCraft).
    pub span: Option<Range<f64>>,
    /// The host's colour for this guide, in place of `overlay.guide` (PhotoCraft's New Guide
    /// colour and preferences). The selected guide is `overlay.guide_selected` whatever it is.
    pub color: Option<Color32>,
    pub line: GuideLine,
    /// A locked guide is drawn but takes no pointer or key.
    pub locked: bool,
}

impl Guide {
    /// A solid, unlocked guide across the view in the theme's colour.
    pub fn new(key: u64, orientation: Orientation, position: f64) -> Self {
        Self { key, orientation, position, span: None, color: None, line: GuideLine::Solid, locked: false }
    }

    pub fn place(&self) -> GuidePlace {
        GuidePlace { orientation: self.orientation, position: Some(self.position) }
    }
}

/// Where a guide is, the value of a guide's edit events: `position: None` is a removed guide.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GuidePlace {
    pub orientation: Orientation,
    pub position: Option<f64>,
}

/// What happened to the guides in a pass.
#[derive(Clone, Debug, PartialEq)]
#[must_use = "apply the guide events and selection to the host's guides"]
pub struct GuidesResponse {
    /// The guides' id: what a ruler passes to [`Guides::drag_from_ruler`].
    pub id: Id,
    /// An edit of a guide: its key (`None` for a guide being dragged out of a ruler, which the
    /// host adds on the commit) and the event. A drag reports Begin, Update and Commit
    /// (`position: None` when it was dropped on a ruler or outside the bounds), or Cancel; a nudge
    /// or Delete a Set.
    pub event: Option<(Option<u64>, EditEvent<GuidePlace>)>,
    /// A guide released where it was pressed: its key, the modifiers held (PhotoCraft turns a
    /// guide on Alt-click) and the document point.
    pub clicked: Option<(u64, Modifiers, Point)>,
    /// A guide double-clicked (PhotoCraft's guide dialog).
    pub double_clicked: Option<u64>,
    /// The selection the guides ask for: `Some(None)` deselects.
    pub select: Option<Option<u64>>,
    /// A guide clicked with the secondary button, for the host's guide menu.
    pub context: Option<u64>,
    /// The guides own the current press: the host's tool leaves it alone.
    pub claimed: bool,
}

/// The guides' look, from [`Guides::style`]; smart guides share it.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct GuideStyle {
    /// Line width: `guide.width`.
    pub width: f32,
    /// Distance from a guide within which the pointer takes it: `guide.hit`.
    pub hit: f32,
    /// A dashed guide's dashes and gaps: `guide.dash`, `guide.gap`.
    pub dash: f32,
    pub gap: f32,
    /// A dotted guide's dot spacing and radius: `guide.dot_gap`, `guide.dot_radius`.
    pub dot_gap: f32,
    pub dot_radius: f32,
    /// `overlay.guide`.
    pub color: Color32,
    /// The selected guide: `overlay.guide_selected`.
    pub selected: Color32,
    /// Smart guides and distances: `overlay.smart_guide`.
    pub smart: Color32,
    /// A distance's end caps: `guide.measure_cap`.
    pub measure_cap: f32,
    /// Around a distance's label: `guide.measure_pad_x`, `guide.measure_pad_y`.
    pub measure_pad: ScreenVec,
    /// A distance label's corners: `shape.radius.control`.
    pub measure_radius: CornerRadius,
    /// A distance label's type role: `caption`.
    pub measure_role: TypeRole,
    /// A distance label's text: black or white, whichever contrasts more with `smart`.
    pub measure_text: Color32,
    /// The lines beside the focused guide: the focus ring's stroke.
    pub focus: Stroke,
    /// From the guide's edge to the focus lines: the focus ring's offset.
    pub focus_offset: f32,
}

/// The host's snapping of a guide ([`Guides::snap`]).
type Snap<'a> = Box<dyn Fn(Orientation, f64, Modifiers) -> f64 + 'a>;

/// What a pass of [`Guides::show`] works with.
struct Pass<'p> {
    ctx: Context,
    id: Id,
    canvas: &'p Response,
    view: &'p CanvasView,
    style: GuideStyle,
    /// `ruler.step_min`, for the snap to the ticks.
    step_min: f32,
    /// The x and y units.
    units: (&'p RulerUnits, &'p RulerUnits),
    /// Each guide's drawn line, `None` out of view.
    segments: Vec<Option<[Pos2; 2]>>,
    /// The document bounds of the canvas rect.
    visible: Rect,
    /// The guides are in the Tab order.
    tab_stop: bool,
}

impl Pass<'_> {
    /// The units a guide of `orientation` is placed in: x for a vertical guide.
    fn units(&self, orientation: Orientation) -> &RulerUnits {
        if orientation == Orientation::Vertical { self.units.0 } else { self.units.1 }
    }
}

/// A guide drag in progress, kept between passes.
#[derive(Clone, Copy, Debug)]
struct Drag {
    /// The guide; `None` for one dragged out of a ruler.
    key: Option<u64>,
    /// The orientation the ruler makes, before Alt.
    base: Orientation,
    start: GuidePlace,
    /// Where the press went down.
    press: Pos2,
    /// Whether the gesture began: past the click distance, or out of a ruler.
    began: bool,
}

/// The ruler guides of a canvas: drawn, taken by the pointer, moved with the keyboard, and in the
/// AccessKit tree.
///
/// ```
/// use craft_ui::canvas::{CanvasView, Guide, Guides, Orientation, kurbo::Point};
/// # egui::__run_test_ui(|ui| {
/// let guides = vec![Guide::new(1, Orientation::Vertical, 120.0), Guide::new(2, Orientation::Horizontal, 80.0)];
/// let (rect, canvas) = ui.allocate_exact_size(egui::vec2(400.0, 300.0), egui::Sense::click_and_drag());
/// let view = CanvasView::new(rect, Point::new(200.0, 150.0), 1.0);
/// let response = Guides::new("guides", &guides).selected(Some(1)).interactive(true).show(ui, &canvas, &view);
/// if !response.claimed {
///     // the tool's input
/// }
/// # });
/// ```
///
/// - The host shows them before its tool input and skips the tool while
///   [`claimed`](GuidesResponse::claimed); objects that beat guides (DesignCraft) come first.
/// - A press within `guide.hit` of an unlocked guide takes it ([`Claim`]); past 0.5 points it
///   drags, and the release commits where it is, or removes the guide over a ruler or outside the
///   [bounds](Self::bounds). A release within 0.5 points is a click, which selects. A press
///   elsewhere on the canvas deselects. Esc cancels a drag.
/// - Shift snaps to the ruler's ticks, otherwise the host's [`snap`](Self::snap) places the guide.
/// - The guides are one Tab stop. Page Up and Page Down select the previous or next guide
///   (vertical guides left to right, then horizontal top to bottom), the arrow keys nudge the
///   selected one across its axis ([`nudge::read`]), Delete and Backspace remove it. With
///   keyboard focus two `line.focus` lines run beside the selected guide.
/// - Selection is the host's: it passes [`selected`](Self::selected) and applies
///   [`select`](GuidesResponse::select).
pub struct Guides<'a> {
    id_salt: IdSalt,
    guides: &'a [Guide],
    selected: Option<u64>,
    interactive: bool,
    bounds: Option<Rect>,
    snap: Option<Snap<'a>>,
    units: Option<(&'a RulerUnits, &'a RulerUnits)>,
    zero: Point,
    readout: Option<(String, String)>,
    label: String,
    names: (String, String),
    steps: (f64, f64),
    policy: EditPolicy,
}

impl<'a> Guides<'a> {
    pub fn new(id_salt: impl AsIdSalt, guides: &'a [Guide]) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            guides,
            selected: None,
            interactive: false,
            bounds: None,
            snap: None,
            units: None,
            zero: Point::ZERO,
            readout: None,
            label: "Guides".to_owned(),
            names: ("Vertical guide".to_owned(), "Horizontal guide".to_owned()),
            steps: (nudge::STEP, nudge::BIG_STEP),
            policy: EditPolicy::default(),
        }
    }

    /// The host's selected guide.
    pub fn selected(mut self, key: Option<u64>) -> Self {
        self.selected = key;
        self
    }

    /// Whether the pointer and the keyboard take guides (PhotoCraft: the Move tool, or ⌘ with a
    /// selection tool; DesignCraft: the selection tools). Off, the guides only draw.
    pub fn interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }

    /// Where guides may live, in document units (PhotoCraft: the document); a guide released
    /// outside is removed. Without bounds, the canvas rect.
    pub fn bounds(mut self, bounds: Rect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    /// The host's snapping (View › Snap To): an orientation, the pointer's position and the
    /// modifiers to the position to use. Shift snaps to the ruler's ticks instead.
    pub fn snap(mut self, snap: impl Fn(Orientation, f64, Modifiers) -> f64 + 'a) -> Self {
        self.snap = Some(Box::new(snap));
        self
    }

    /// The ruler units of the x and y axes, for the readout, the AccessKit values and the snap to
    /// ticks; document pixels without.
    pub fn units(mut self, x: &'a RulerUnits, y: &'a RulerUnits) -> Self {
        self.units = Some((x, y));
        self
    }

    /// The document point the rulers count from.
    pub fn zero(mut self, zero: Point) -> Self {
        self.zero = zero;
        self
    }

    /// Shows the position of a dragged guide beside the pointer, labelled `x` for a vertical
    /// guide and `y` for a horizontal one ("X:", "Y:", translated).
    pub fn readout(mut self, x: impl Into<String>, y: impl Into<String>) -> Self {
        self.readout = Some((x.into(), y.into()));
        self
    }

    /// The AccessKit name of the guides' group, translated.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// The AccessKit names of a vertical and a horizontal guide, translated.
    pub fn guide_names(mut self, vertical: impl Into<String>, horizontal: impl Into<String>) -> Self {
        self.names = (vertical.into(), horizontal.into());
        self
    }

    /// The arrow keys' nudge, in document units, plain and with Shift (1 and 10).
    pub fn steps(mut self, step: f64, big_step: f64) -> Self {
        self.steps = (step, big_step);
        self
    }

    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn style(tokens: &UiTokens) -> GuideStyle {
        let (g, o) = (&tokens.component.guide, &tokens.color.overlay);
        let ring = FocusRing::style(tokens);
        let smart: Rgba = o.smart_guide;
        let text = if contrast_ratio(Rgba::BLACK, smart) >= contrast_ratio(Rgba::WHITE, smart) { Color32::BLACK } else { Color32::WHITE };
        GuideStyle {
            width: g.width,
            hit: g.hit,
            dash: g.dash,
            gap: g.gap,
            dot_gap: g.dot_gap,
            dot_radius: g.dot_radius,
            color: o.guide.to_egui(),
            selected: o.guide_selected.to_egui(),
            smart: smart.to_egui(),
            measure_cap: g.measure_cap,
            measure_pad: vec2(g.measure_pad_x, g.measure_pad_y),
            measure_radius: tokens.radius.control,
            measure_role: TypeRole::Caption,
            measure_text: text,
            focus: ring.stroke,
            focus_offset: ring.offset,
        }
    }

    /// Starts dragging a new guide of `orientation` out of a ruler: the top ruler makes a
    /// horizontal guide, the left one a vertical guide. The ruler calls it when a drag starts on
    /// it, with the [`id`](GuidesResponse::id) of the guides it feeds; the guides run the drag
    /// from their next pass until the release, wherever the pointer goes. Alt held turns the guide
    /// the other way (PhotoCraft #5698). Released over the canvas and inside the bounds the drag
    /// commits with no key and the host adds the guide; elsewhere it is cancelled.
    pub fn drag_from_ruler(ctx: &Context, guides: Id, orientation: Orientation) {
        let press = ctx.input(|i| i.pointer.press_origin().or_else(|| i.pointer.latest_pos())).unwrap_or(Pos2::ZERO);
        let start = GuidePlace { orientation, position: None };
        ctx.data_mut(|d| d.insert_temp(drag_key(guides), Drag { key: None, base: orientation, start, press, began: false }));
    }

    /// Draws the guides over `view` and senses through the host's `canvas` response.
    pub fn show(self, ui: &mut Ui, canvas: &Response, view: &CanvasView) -> GuidesResponse {
        let ctx = ui.ctx().clone();
        let tokens = Tokens::of(ui);
        let id = ui.make_persistent_id(self.id_salt);
        let default_units;
        let units = match self.units {
            Some(units) => units,
            None => {
                default_units = RulerUnits::pixels(LocaleNumbers::from_tag("en").ungrouped());
                (&default_units, &default_units)
            }
        };
        let order = self.keyboard_order();
        let visible = view.visible();
        let pass = Pass {
            ctx: ctx.clone(),
            id,
            canvas,
            view,
            style: Guides::style(&tokens),
            step_min: tokens.component.ruler.step_min,
            units,
            segments: self.guides.iter().map(|g| segment(view, visible, g.orientation, g.position, g.span.as_ref())).collect(),
            visible,
            tab_stop: self.interactive && !order.is_empty(),
        };
        let group = ui.interact(view.rect, id, if pass.tab_stop { Sense::FOCUSABLE } else { Sense::hover() });
        group.widget_info(|| WidgetInfo::labeled(Role::Group, ui.is_enabled(), &self.label));
        let focused = pass.tab_stop && group.has_focus();
        let mut out = GuidesResponse { id, event: None, clicked: None, double_clicked: None, select: None, context: None, claimed: false };

        let mut drag: Option<Drag> = ctx.data(|d| d.get_temp(drag_key(id)));
        let reported = Gesture::<GuidePlace>::reported(ui, id).map(|(_, value)| value);
        let target = match drag {
            Some(d) => d.key,
            None => self.selected,
        };
        let mut gesture = Gesture::load(ui, id, &self.host_value(drag, reported), self.policy).focus_owners(&[]).enabled(ui.is_enabled());
        if drag.is_some_and(|d| d.began) && !gesture.is_active() {
            // Esc, a change by the host or `cancel_all` ended it; the press stays claimed.
            drag = None;
        }
        if drag.is_none() {
            drag = self.press(ui, &pass, focused, &mut out);
        }
        let mut live = None;
        if let Some(d) = drag {
            (drag, live) = self.track(ui, &pass, d, &mut gesture, &mut out);
        }
        if self.interactive && drag.is_none() {
            self.pointer(&pass, &mut out);
        }
        if focused && drag.is_none() {
            self.keys(ui, &order, &mut gesture, &mut out, view);
        }
        if pass.tab_stop && group.gained_focus() && !self.selected.is_some_and(|k| order.contains(&k)) && out.select.is_none() {
            out.select = Some(order.first().copied());
        }
        if focused {
            let filter = EventFilter { horizontal_arrows: true, vertical_arrows: true, escape: gesture.is_active(), ..EventFilter::default() };
            ui.memory_mut(|m| m.set_focus_lock_filter(id, filter));
        }

        out.event = gesture.finish().map(|event| (target, event));
        out.claimed = drag.is_some_and(|d| d.key.is_none()) || Claim::is_owner(&ctx, canvas.id, id);
        ctx.data_mut(|data| {
            if let Some(d) = drag {
                data.insert_temp(drag_key(id), d);
            } else {
                data.remove::<Drag>(drag_key(id));
            }
        });
        let selected = out.select.unwrap_or(self.selected);
        let dragged = live.map(|place| (drag.and_then(|d| d.key), place));
        self.paint(ui, &pass, selected, dragged, focused);
        self.nodes(ui, &pass, selected);
        out
    }

    /// The gesture's host value: during a drag the dragged guide as the gesture last reported it
    /// (the host may show the updates or not), otherwise the selected guide, for a key's Set.
    fn host_value(&self, drag: Option<Drag>, reported: Option<GuidePlace>) -> GuidePlace {
        match drag {
            Some(Drag { key: None, start, .. }) => reported.unwrap_or(start),
            Some(Drag { key: Some(key), start, began, .. }) => match self.find(key) {
                Some(g) if began && (g.place() == start || Some(g.place()) == reported) => reported.unwrap_or(start),
                Some(g) => g.place(),
                None => GuidePlace { position: None, ..start },
            },
            None => self.selected_guide().map_or(GuidePlace { orientation: Orientation::Vertical, position: None }, Guide::place),
        }
    }

    /// A press on the canvas: takes the guide under it, or deselects.
    fn press(&self, ui: &Ui, pass: &Pass<'_>, focused: bool, out: &mut GuidesResponse) -> Option<Drag> {
        let press = pass.ctx.input(|i| i.pointer.primary_pressed().then(|| i.pointer.press_origin()).flatten())?;
        if !(pass.view.rect.contains(press) && pass.canvas.contains_pointer()) {
            return None;
        }
        if let Some(i) = self.guide_at(&pass.segments, press, pass.style.hit).filter(|_| self.interactive)
            && Claim::take(&pass.ctx, pass.canvas.id, pass.id)
        {
            let g = &self.guides[i];
            return Some(Drag { key: Some(g.key), base: g.orientation, start: g.place(), press, began: false });
        }
        if self.selected.is_some() {
            out.select = Some(None);
        }
        if focused {
            ui.memory_mut(|m| m.surrender_focus(pass.id));
        }
        None
    }

    /// One pass of a drag: the drag that goes on, if it does, and the dragged guide's place once
    /// the gesture began.
    fn track(
        &self,
        ui: &Ui,
        pass: &Pass<'_>,
        mut d: Drag,
        gesture: &mut Gesture<GuidePlace>,
        out: &mut GuidesResponse,
    ) -> (Option<Drag>, Option<GuidePlace>) {
        let view = pass.view;
        if d.key.is_none() && !d.began {
            gesture.begin();
            d.began = true;
        }
        let (pointer, down, mods) = pass.ctx.input(|i| (i.pointer.latest_pos(), i.pointer.primary_down(), i.modifiers));
        let orientation = match d.key {
            None if mods.alt => d.base.flipped(),
            None => d.base,
            Some(_) => d.start.orientation,
        };
        let (mut outside, mut moved, mut at) = (true, true, None);
        if let Some(pointer) = pointer {
            let raw = view.to_doc(pointer);
            let position = self.snapped(orientation, orientation.position(raw), mods, pass);
            outside = !view.rect.contains(pointer) || !self.inside_bounds(orientation, position);
            let across = (orientation.position(raw) - orientation.position(view.to_doc(d.press))).abs() * orientation.zoom(view);
            moved = outside || across >= CLICK;
            if !d.began && moved {
                gesture.begin();
                d.began = true;
            }
            if d.began {
                gesture.update(GuidePlace { orientation, position: Some(position) }, mods);
            }
            at = Some((pointer, raw, position));
        }
        let live = d.began.then(|| *gesture.shown());
        if d.began {
            if let Some((pointer, _, position)) = at
                && let Some((x, y)) = &self.readout
            {
                let label = if orientation == Orientation::Vertical { x } else { y };
                let value = pass.units(orientation).format_value(position, orientation.position(self.zero));
                Readout::new(("craft_ui::canvas::Guides::readout", pass.id)).row(label.clone(), value).show(&pass.ctx, pointer);
            }
            pass.ctx.set_cursor_icon(resize_cursor(view, orientation.across()));
        }
        if down {
            return (Some(d), live);
        }
        match d.key {
            Some(key) if !moved => {
                if d.began {
                    gesture.update(d.start, mods);
                    gesture.commit();
                }
                let point = at.map_or_else(|| view.to_doc(d.press), |(_, raw, _)| raw);
                out.clicked = Some((key, mods, point));
                out.select = Some(Some(key));
            }
            Some(key) => {
                if outside {
                    gesture.update(GuidePlace { orientation, position: None }, mods);
                } else {
                    out.select = Some(Some(key));
                }
                gesture.commit();
            }
            None if outside => gesture.cancel(CancelReason::Dismissed),
            None => gesture.commit(),
        }
        // The keys then act on the guide the pointer took (PhotoCraft's Delete after a click).
        // Taken at the release: egui drops the focus of a widget clicked around.
        if d.key.is_some() && !outside && pass.tab_stop {
            ui.memory_mut(|m| m.request_focus(pass.id));
        }
        (None, live)
    }

    /// The hover cursor, double and secondary clicks on a guide.
    fn pointer(&self, pass: &Pass<'_>, out: &mut GuidesResponse) {
        let (canvas, hit) = (pass.canvas, pass.style.hit);
        let other = Claim::owner(&pass.ctx, canvas.id).is_some_and(|owner| owner != pass.id);
        if !other
            && let Some(pointer) = canvas.hover_pos()
            && let Some(i) = self.guide_at(&pass.segments, pointer, hit)
        {
            pass.ctx.set_cursor_icon(resize_cursor(pass.view, self.guides[i].orientation.across()));
        }
        let at = |pos: Option<Pos2>| pos.and_then(|p| self.guide_at(&pass.segments, p, hit)).map(|i| self.guides[i].key);
        if canvas.secondary_clicked() {
            out.context = at(canvas.interact_pointer_pos());
        }
        if canvas.double_clicked() {
            out.double_clicked = at(canvas.interact_pointer_pos());
        }
    }

    /// Draws the guides, a dragged one at its live place (`dragged`: its key, `None` out of a
    /// ruler) instead of where it was, and the focus lines.
    fn paint(&self, ui: &Ui, pass: &Pass<'_>, selected: Option<u64>, dragged: Option<(Option<u64>, GuidePlace)>, focused: bool) {
        let style = &pass.style;
        let painter = ui.painter_at(pass.view.rect);
        let mut focus_segment = None;
        for (g, seg) in self.guides.iter().zip(&pass.segments) {
            if dragged.is_some_and(|(key, _)| key == Some(g.key)) {
                continue;
            }
            if let Some([a, b]) = *seg {
                paint_line(&painter, a, b, self.color(g, selected, style), g.line, style);
                if Some(g.key) == selected {
                    focus_segment = Some([a, b]);
                }
            }
        }
        if let Some((key, GuidePlace { orientation, position: Some(position) })) = dragged {
            let guide = key.and_then(|key| self.find(key));
            if let Some([a, b]) = segment(pass.view, pass.visible, orientation, position, guide.and_then(|g| g.span.as_ref())) {
                let (color, line) = guide.map_or((style.color, GuideLine::Solid), |g| (self.color(g, selected, style), g.line));
                paint_line(&painter, a, b, color, line, style);
                if guide.is_some_and(|g| Some(g.key) == selected) {
                    focus_segment = Some([a, b]);
                }
            }
        }
        if focused
            && FocusRing::visible(&pass.ctx)
            && let Some([a, b]) = focus_segment
        {
            let along = (b - a).normalized();
            let normal = vec2(-along.y, along.x) * (style.width / 2.0 + style.focus_offset + style.focus.width / 2.0);
            let ppp = painter.pixels_per_point();
            painter.add(crisp_shape(a + normal, b + normal, style.focus, ppp));
            painter.add(crisp_shape(a - normal, b - normal, style.focus, ppp));
        }
    }

    /// AccessKit: a group of guide objects; the selected one is its active descendant.
    fn nodes(&self, ui: &mut Ui, pass: &Pass<'_>, selected: Option<u64>) {
        let (id, view) = (pass.id, pass.view);
        let nodes = ui.new_child(UiBuilder::new().max_rect(view.rect).accessibility_parent(id));
        for (g, seg) in self.guides.iter().zip(&pass.segments) {
            let rect = match *seg {
                Some([a, b]) => egui::Rect::from_two_pos(a, b).expand(pass.style.hit),
                None => {
                    egui::Rect::from_center_size(view.to_screen(g.orientation.point(g.position, g.orientation.along(view.center))), ScreenVec::ZERO)
                }
            };
            let response = nodes.interact(rect, guide_id(id, g.key), Sense::hover());
            let name = if g.orientation == Orientation::Vertical { &self.names.0 } else { &self.names.1 };
            response.widget_info(|| {
                let mut info = WidgetInfo::labeled(Role::GraphicsObject, nodes.is_enabled(), name);
                info.current_text_value = Some(pass.units(g.orientation).format_value(g.position, g.orientation.position(self.zero)));
                info
            });
            pass.ctx.accesskit_node_builder(response.id, |node| node.set_selected(selected == Some(g.key)));
        }
        let active = selected.filter(|&k| self.find(k).is_some());
        pass.ctx.accesskit_node_builder(id, |node| match active {
            Some(key) => node.set_active_descendant(guide_id(id, key).accesskit_id()),
            None => node.clear_active_descendant(),
        });
    }

    /// Page Up, Page Down, Delete and the arrow keys, while the guides have focus.
    fn keys(&self, ui: &Ui, order: &[u64], gesture: &mut Gesture<GuidePlace>, out: &mut GuidesResponse, view: &CanvasView) {
        let current = self.selected.and_then(|k| order.iter().position(|&o| o == k));
        let (up, down) = ui.input_mut(|i| (i.consume_key(Modifiers::NONE, Key::PageUp), i.consume_key(Modifiers::NONE, Key::PageDown)));
        let last = order.len().saturating_sub(1);
        let step = match (up, down, current) {
            (true, false, Some(c)) => Some(c.saturating_sub(1)),
            (true, false, None) => Some(last),
            (false, true, Some(c)) => Some((c + 1).min(last)),
            (false, true, None) => Some(0),
            _ => None,
        };
        if let Some(i) = step
            && let Some(&key) = order.get(i)
        {
            out.select = Some(Some(key));
        }
        let Some(guide) = current.and_then(|c| self.find(order[c])) else { return };
        let delete = ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Delete) || i.consume_key(Modifiers::NONE, Key::Backspace));
        if delete {
            gesture.set(GuidePlace { orientation: guide.orientation, position: None });
            return;
        }
        if let Some(moved) = nudge::read(ui, self.steps.0, self.steps.1) {
            let by = guide.orientation.position(moved.to_point());
            if by != 0.0 {
                let (lo, hi) = guide.orientation.range(self.bounds.unwrap_or_else(|| view.visible()));
                let position = (guide.position + by).clamp(lo.min(hi), hi.max(lo));
                gesture.set(GuidePlace { orientation: guide.orientation, position: Some(position) });
            }
        }
    }

    fn find(&self, key: u64) -> Option<&Guide> {
        self.guides.iter().find(|g| g.key == key)
    }

    fn selected_guide(&self) -> Option<&Guide> {
        self.selected.and_then(|k| self.find(k)).filter(|g| !g.locked)
    }

    /// The unlocked guides in keyboard order: vertical guides left to right, then horizontal ones
    /// top to bottom.
    fn keyboard_order(&self) -> Vec<u64> {
        let mut unlocked: Vec<&Guide> = self.guides.iter().filter(|g| !g.locked).collect();
        unlocked.sort_by(|a, b| {
            let rank = |g: &Guide| u8::from(g.orientation == Orientation::Horizontal);
            rank(a).cmp(&rank(b)).then(a.position.total_cmp(&b.position))
        });
        unlocked.into_iter().map(|g| g.key).collect()
    }

    /// The unlocked guide nearest `pointer` within `hit` points of its drawn line; of equal
    /// distances the one drawn last.
    fn guide_at(&self, segments: &[Option<[Pos2; 2]>], pointer: Pos2, hit: f32) -> Option<usize> {
        let mut best: Option<(usize, f32)> = None;
        for (i, (g, seg)) in self.guides.iter().zip(segments).enumerate() {
            let Some([a, b]) = *seg else { continue };
            let d = hit::segment(pointer, a, b);
            if !g.locked && d <= hit && best.is_none_or(|(_, bd)| d <= bd) {
                best = Some((i, d));
            }
        }
        best.map(|(i, _)| i)
    }

    /// Whether a guide at `position` is inside the bounds.
    fn inside_bounds(&self, orientation: Orientation, position: f64) -> bool {
        self.bounds.is_none_or(|b| {
            let (lo, hi) = orientation.range(b);
            (lo..=hi).contains(&position)
        })
    }

    /// `position` snapped: to the ruler's minor ticks with Shift, otherwise by the host.
    fn snapped(&self, orientation: Orientation, position: f64, mods: Modifiers, pass: &Pass<'_>) -> f64 {
        if mods.shift_only() {
            let zoom = orientation.zoom(pass.view);
            pass.units(orientation).snap_to_tick(position, orientation.position(self.zero), zoom, pass.step_min)
        } else if let Some(snap) = &self.snap {
            snap(orientation, position, mods)
        } else {
            position
        }
    }

    fn color(&self, g: &Guide, selected: Option<u64>, style: &GuideStyle) -> Color32 {
        if selected == Some(g.key) { style.selected } else { g.color.unwrap_or(style.color) }
    }
}

fn drag_key(guides: Id) -> Id {
    guides.with("craft_ui::canvas::Guides::drag")
}

fn guide_id(guides: Id, key: u64) -> Id {
    guides.with(("guide", key))
}

/// The screen ends of a guide at `position` across what the view shows (`visible`, the document
/// bounds of the canvas rect), limited to `span`: a rotated view turns the guide with the
/// document (PhotoCraft #3277). `None` when the span is out of view.
fn segment(view: &CanvasView, visible: Rect, orientation: Orientation, position: f64, span: Option<&Range<f64>>) -> Option<[Pos2; 2]> {
    let (lo, hi) = orientation.flipped().range(visible);
    let (lo, hi) = match span {
        Some(s) => (lo.max(s.start.min(s.end)), hi.min(s.end.max(s.start))),
        None => (lo, hi),
    };
    (lo <= hi).then(|| [view.to_screen(orientation.point(position, lo)), view.to_screen(orientation.point(position, hi))])
}

/// One guide line from `a` to `b` in `color`: solid and dashed lines crisp on the pixel grid.
fn paint_line(painter: &Painter, a: Pos2, b: Pos2, color: Color32, line: GuideLine, style: &GuideStyle) {
    let stroke = Stroke::new(style.width, color);
    let ppp = painter.pixels_per_point();
    match line {
        GuideLine::Solid => {
            painter.add(crisp_shape(a, b, stroke, ppp));
        }
        GuideLine::Dashed => painter.extend(dashes(a, b, style.dash, style.gap).map(|(s, e)| crisp_shape(s, e, stroke, ppp))),
        GuideLine::Dots => painter.extend(Shape::dotted_line(&[a, b], color, style.dot_gap, style.dot_radius)),
    }
}

/// The dashes of a dashed line from `a` to `b`.
fn dashes(a: Pos2, b: Pos2, dash: f32, gap: f32) -> impl Iterator<Item = (Pos2, Pos2)> {
    let length = a.distance(b);
    let dir = if length > 0.0 { (b - a) / length } else { ScreenVec::ZERO };
    let period = (dash + gap).max(0.5);
    let count = if dash > 0.0 { (length / period).ceil() as usize } else { 0 };
    (0..count).map(move |i| {
        let start = i as f32 * period;
        (a + dir * start, a + dir * (start + dash).min(length))
    })
}

/// An alignment line of [`SmartGuides`], in document coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SmartLine {
    pub a: Point,
    pub b: Point,
    /// An extension from an object's edge to a distance: dashed.
    pub extension: bool,
}

/// A distance of [`SmartGuides`]: a line with end caps and a label, in document coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct Measure {
    pub a: Point,
    pub b: Point,
    /// The host's formatted distance (`24 px`).
    pub label: String,
}

/// The alignment lines and distances a host shows while something moves (PhotoCraft's smart
/// guides and Alt distances): `SmartGuides::new(&lines, &measures).show(ui, &view)`.
///
/// Drawing only: the host computes what aligns and how far apart things are. Lines are 1 point
/// of `overlay.smart_guide`, crisp; extensions dashed; distances get `guide.measure_cap` end
/// caps and a label pill. Not in the AccessKit tree: they are hints during a gesture, whose
/// result the moved object's own value reports.
pub struct SmartGuides<'a> {
    lines: &'a [SmartLine],
    measures: &'a [Measure],
}

impl<'a> SmartGuides<'a> {
    pub fn new(lines: &'a [SmartLine], measures: &'a [Measure]) -> Self {
        Self { lines, measures }
    }

    pub fn show(self, ui: &Ui, view: &CanvasView) {
        let tokens = Tokens::of(ui);
        let style = Guides::style(&tokens);
        let painter = ui.painter_at(view.rect);
        let ppp = painter.pixels_per_point();
        let stroke = Stroke::new(style.width, style.smart);
        for line in self.lines {
            let (a, b) = (view.to_screen(line.a), view.to_screen(line.b));
            if line.extension {
                // PhotoCraft's extensions are dashed 3/3.
                painter.extend(dashes(a, b, style.gap, style.gap).map(|(s, e)| crisp_shape(s, e, stroke, ppp)));
            } else {
                painter.add(crisp_shape(a, b, stroke, ppp));
            }
        }
        let font = tokens.font(style.measure_role);
        for measure in self.measures {
            let (a, b) = (view.to_screen(measure.a), view.to_screen(measure.b));
            painter.add(crisp_shape(a, b, stroke, ppp));
            let along = (b - a).normalized();
            let cap = vec2(-along.y, along.x) * (style.measure_cap / 2.0);
            for end in [a, b] {
                painter.add(crisp_shape(end - cap, end + cap, stroke, ppp));
            }
            let galley = painter.layout_no_wrap(measure.label.clone(), font.clone(), style.measure_text);
            let pill = egui::Rect::from_center_size(a.lerp(b, 0.5), galley.size() + 2.0 * style.measure_pad);
            let pill = pill.round_to_pixels(ppp);
            painter.rect_filled(pill, style.measure_radius, style.smart);
            painter.galley(Align2::CENTER_CENTER.anchor_size(pill.center(), galley.size()).min, galley, style.measure_text);
        }
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Guides", "Smart guides"];

pub(crate) fn stories() -> Vec<Story> {
    vec![
        Story {
            id: "primitives/guides/guides",
            level: Level::Primitives,
            component: "Guides",
            name: "Guides",
            args: || {
                super::story::args()
                    .with_choice("line", &["Solid", "Dashed", "Dots"], 0)
                    .with_bool("interactive", true)
                    .with_bool("locked", false)
                    .with_bool("readout", true)
            },
            show: show_guides,
        },
        Story {
            id: "primitives/smart-guides/move",
            level: Level::Primitives,
            component: "Smart guides",
            name: "Move",
            args: super::story::args,
            show: show_smart_guides,
        },
    ]
}

/// A host's guides and selection, as the stories and tests keep them.
#[derive(Clone, Debug)]
pub(crate) struct GuideHost {
    pub guides: Vec<Guide>,
    pub selected: Option<u64>,
    /// The guides when a drag began, to restore on a cancel.
    pub before: Option<Vec<Guide>>,
    pub next_key: u64,
}

impl GuideHost {
    pub(crate) fn new(guides: Vec<Guide>) -> Self {
        let next_key = guides.iter().map(|g| g.key + 1).max().unwrap_or(1);
        Self { guides, selected: None, before: None, next_key }
    }

    /// Applies a pass's events and selection, showing a drag's updates as they come.
    pub(crate) fn apply(&mut self, response: &GuidesResponse) {
        if let Some(select) = response.select {
            self.selected = select;
        }
        let Some((key, event)) = &response.event else { return };
        match (event, key) {
            (EditEvent::Begin { .. }, _) => self.before = Some(self.guides.clone()),
            (EditEvent::Cancel { .. }, _) => {
                if let Some(before) = self.before.take() {
                    self.guides = before;
                }
            }
            (EditEvent::Update { value, .. }, Some(key)) => self.place(*key, *value),
            (EditEvent::Commit { value, .. } | EditEvent::Set { value, .. }, Some(key)) => {
                self.before = None;
                self.place(*key, *value);
            }
            (EditEvent::Commit { value, .. }, None) => {
                self.before = None;
                if let Some(position) = value.position {
                    self.guides.push(Guide::new(self.next_key, value.orientation, position));
                    self.next_key += 1;
                }
            }
            _ => {}
        }
    }

    fn place(&mut self, key: u64, place: GuidePlace) {
        match place.position {
            Some(position) => {
                if let Some(g) = self.guides.iter_mut().find(|g| g.key == key) {
                    g.orientation = place.orientation;
                    g.position = position;
                }
            }
            None => {
                self.guides.retain(|g| g.key != key);
                if self.selected == Some(key) {
                    self.selected = None;
                }
            }
        }
    }
}

/// Guides on the story document: drag them, drag new ones out of the strip above (Alt turns
/// them), Shift snaps to the ticks; Tab to them, Page Up and Page Down, the arrows and Delete.
fn show_guides(ui: &mut Ui, args: &mut Args) {
    let key = ui.scope_id().with("guides-story");
    let mut host: GuideHost = ui.data(|d| d.get_temp(key)).unwrap_or_else(|| {
        GuideHost::new(vec![
            Guide::new(1, Orientation::Vertical, 160.0),
            Guide::new(2, Orientation::Vertical, 480.0),
            Guide { color: Some(Color32::from_rgb(255, 120, 0)), ..Guide::new(3, Orientation::Horizontal, 120.0) },
            Guide { span: Some(0.0..320.0), ..Guide::new(4, Orientation::Horizontal, 280.0) },
        ])
    });
    let line = match args.choice("line") {
        "Dashed" => GuideLine::Dashed,
        "Dots" => GuideLine::Dots,
        _ => GuideLine::Solid,
    };
    for g in &mut host.guides {
        g.line = line;
        g.locked = args.bool("locked") && g.key == 2;
    }
    let tokens = Tokens::of(ui);
    let (strip, _) = ui.allocate_exact_size(vec2(ui.available_width().clamp(240.0, 720.0), tokens.component.ruler.size), Sense::hover());
    let ruler = ui.interact(strip, key.with("ruler"), Sense::drag());
    ui.painter().rect_filled(strip, 0.0, tokens.color.surface.panel.to_egui());
    let canvas = super::story::document(ui, args, "Document");
    let mut guides =
        Guides::new("story-guides", &host.guides).selected(host.selected).interactive(args.bool("interactive")).bounds(super::story::DOC);
    if args.bool("readout") {
        guides = guides.readout("X:", "Y:");
    }
    let response = guides.show(ui, &canvas.response, &canvas.view);
    if ruler.drag_started() {
        Guides::drag_from_ruler(ui.ctx(), response.id, Orientation::Horizontal);
    }
    host.apply(&response);
    ui.data_mut(|d| d.insert_temp(key, host));
}

/// Two boxes on the story document: the moving one's top aligned with the other's, 80 units to its
/// right.
fn show_smart_guides(ui: &mut Ui, args: &mut Args) {
    let canvas = super::story::document(ui, args, "Document");
    let view = &canvas.view;
    let painter = ui.painter_at(view.rect);
    let still = Rect::new(120.0, 80.0, 280.0, 200.0);
    let moving = Rect::new(360.0, 80.0, 480.0, 160.0);
    for r in [still, moving] {
        painter.add(Shape::closed_line(view.quad(r).to_vec(), Stroke::new(1.0, Color32::from_gray(60))));
    }
    let lines = [
        SmartLine { a: Point::new(still.x0, still.y0), b: Point::new(moving.x1, moving.y0), extension: false },
        SmartLine { a: Point::new(still.x1, still.y1), b: Point::new(still.x1, 240.0), extension: true },
        SmartLine { a: Point::new(moving.x0, moving.y1), b: Point::new(moving.x0, 240.0), extension: true },
    ];
    let measures = [Measure { a: Point::new(still.x1, 240.0), b: Point::new(moving.x0, 240.0), label: "80 px".to_owned() }];
    SmartGuides::new(&lines, &measures).show(ui, view);
}

#[cfg(test)]
#[path = "guides_tests.rs"]
mod tests;
