//! [`Rulers`]: the strips along the top and left of a canvas, and the corner that holds the zero
//! point.

use crate::canvas::story::ViewArgs;
use crate::localized::Labels as _;
use egui::accesskit::{self, Role};
use egui::{
    Align2, AsIdSalt, Color32, Context, CursorIcon, FontId, Id, IdSalt, LayerId, Modifiers, Painter, Pos2, Rect as ScreenRect, Response, Sense,
    Stroke, Ui, WidgetInfo, pos2, vec2,
};
use kurbo::Point;
use std::ops::{Range, RangeInclusive};

use super::claim::PrimaryPress;
use super::guides::segment;
use super::units::tick_snap;
use super::{CanvasView, Guides, Orientation, RulerUnits, crisp, resize_cursor};
use crate::edit::{EditEvent, EditPolicy, Gesture};
use crate::localized::units::{LengthUnit, LengthUnitLabels};
use crate::menu::{ContextMenu, MenuEntry, MenuItem, OpenAt};
use crate::primitives::Divider;
use crate::story::{Level, Story, StoryChoice, Surface};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens};
use crate::typed_story;

/// A press that began on a ruler strip, kept until it ends.
#[derive(Clone, Copy)]
struct RulerPress {
    press: PrimaryPress,
    orientation: Orientation,
    /// The guides have the drag.
    handed: bool,
}

/// Hands the press that began on a ruler `strip` to the `guides` once it is decidedly a drag: moved
/// beyond a click, or held long. Decided from the pointer's own state, not egui's drag on the
/// strip, which never starts for a press released in the pass that moves it. A press that has
/// ended by then is handed over with its release.
fn hand_off(ctx: &Context, key: Id, guides: Id, layer: LayerId, strips: [(ScreenRect, Orientation); 2]) {
    let current = ctx.data(|d| d.get_temp::<RulerPress>(key)).filter(|s| s.press.holds(ctx));
    let fresh = || {
        let press = PrimaryPress::current(ctx)?;
        let (_, orientation) = strips.iter().find(|(strip, _)| strip.contains(press.origin))?;
        (ctx.layer_id_at(press.origin) == Some(layer)).then_some(RulerPress { press, orientation: *orientation, handed: false })
    };
    let Some(mut state) = current.or_else(fresh) else {
        ctx.data_mut(|d| d.remove::<RulerPress>(key));
        return;
    };
    if !state.handed && ctx.input(|i| !i.pointer.could_any_button_be_click()) {
        Guides::hand_off(ctx, guides, state.orientation, state.press);
        state.handed = true;
    }
    ctx.data_mut(|d| d.insert_temp(key, state));
}

/// A top-ruler label sits this far below the strip's top edge (PhotoCraft).
const LABEL_TOP: f32 = 1.0;
/// Ticks beyond this many in one ruler are not drawn: a custom table that asks for them has no
/// useful picture to give.
const MAX_TICKS: i64 = 20_000;
/// The zero point is written to this many decimals (PhotoCraft).
const ZERO_ROUND: f64 = 1000.0;

/// The part of the rulers a pointer is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RulerPart {
    /// The top ruler, which measures the document's x axis.
    Horizontal,
    /// The left ruler, which measures the document's y axis.
    Vertical,
    /// The square where the rulers meet.
    Corner,
}

/// What happened to the rulers in a pass.
#[derive(Clone, Debug, PartialEq)]
#[must_use = "apply the zero point events to the host's document"]
pub struct RulersResponse {
    /// The rulers' id: what the host's zero point commands pass to [`Rulers::reset_zero`] and
    /// [`Rulers::set_zero`].
    pub id: Id,
    /// A move of the zero point (document units): a drag's Begin, Update, Commit (`changed: false`
    /// when it was dropped back on the corner) or Cancel, or the Set of [`Rulers::set_zero`]. The
    /// host applies the Commit and the Set.
    pub origin: Option<EditEvent<Point>>,
    /// The corner was double-clicked, or the host's command asked for it ([`Rulers::reset_zero`]):
    /// the host puts the zero point back at the document's origin.
    pub reset_origin: bool,
    /// A ruler or the corner was clicked with the secondary button, for the host's units menu.
    /// With [`Rulers::units_menu`] the rulers open the menu themselves and this stays `None`.
    pub context: Option<RulerPart>,
    /// A unit picked from the rulers' own units menu ([`Rulers::units_menu`]): the host stores
    /// it as the ruler unit preference.
    pub unit_picked: Option<LengthUnit>,
}

/// The rulers' look, from [`Rulers::style`].
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct RulerStyle {
    /// Thickness of a strip and side of the corner: `ruler.size`.
    pub size: f32,
    /// The labels' font: the UI face at `ruler.label`.
    pub font: FontId,
    /// Between the characters of the left ruler's labels: `ruler.label_pitch`.
    pub label_pitch: f32,
    /// Between a tick and its label: `ruler.label_pad`.
    pub label_pad: f32,
    /// Least distance between labelled ticks: `ruler.step_min`.
    pub step_min: f32,
    /// Length of the tick halfway between labelled ticks: `ruler.tick_mid`.
    pub tick_mid: f32,
    /// Length of the other minor ticks: `ruler.tick_minor`.
    pub tick_minor: f32,
    /// The strips and the corner: `surface.chrome`.
    pub fill: Color32,
    /// 1 point of `text.faint`.
    pub tick: Stroke,
    /// Labels: `text.muted`.
    pub label: Color32,
    /// The pointer's marker and the cross hairs of a zero point drag: 1 point of `text.normal`.
    pub marker: Stroke,
    /// The margin part of a strip, where the text part keeps `fill`: `ruler.margin_shade`.
    pub margin_shade: Color32,
}

crate::words! {
    /// The AccessKit names of the rulers. `Default` is English; a host passes its own translated
    /// text, or `RulerNames::of(ui)` for the words installed in the context.
    pub struct RulerNames {
        /// The top ruler.
        horizontal: "ruler.horizontal" = "Horizontal ruler";
        /// The left ruler.
        vertical: "ruler.vertical" = "Vertical ruler";
        /// The corner that holds the zero point.
        origin: "ruler.origin" = "Ruler origin";
    }
}

impl RulerNames {
    /// The accessible names of the two rulers and the origin corner.
    pub fn new(horizontal: impl Into<String>, vertical: impl Into<String>, origin: impl Into<String>) -> Self {
        Self { horizontal: horizontal.into(), vertical: vertical.into(), origin: origin.into() }
    }
}

/// One tick of [`Rulers::ticks`].
#[derive(Clone, Debug, PartialEq)]
pub struct RulerTick {
    /// Ruler units from the zero point.
    pub value: f64,
    /// Along the strip's depth: `ruler.size` for a labelled tick, `ruler.tick_mid` halfway between
    /// two, `ruler.tick_minor` for the rest.
    pub length: f32,
    /// A labelled tick's number, written by [`RulerUnits::tick_label`].
    pub label: Option<String>,
}

/// Which tick table [`Rulers::ticks`] follows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TickRule<'a> {
    /// The units' own table ([`RulerUnits::step`]): decimal steps and ten subdivisions, or the
    /// host's custom table.
    Metric,
    /// Inches: a label every inch (every 2, 5, 10 … inches when smaller), subdivided by the
    /// first of these counts, finest first, that keeps the ticks `ruler.step_min` / 10 points
    /// apart, the spacing of the metric rule's tenths ([`INCH_SIXTEENTHS`], [`INCH_EIGHTHS`]).
    Inches(&'a [u32]),
}

/// Word's inch ruler: sixteenths, down to halves.
pub const INCH_SIXTEENTHS: [u32; 4] = [16, 8, 4, 2];
/// DesignCraft's inch ruler: eighths, down to halves (`geom/src/units.rs:117`).
pub const INCH_EIGHTHS: [u32; 3] = [8, 4, 2];

/// The metric rule's subdivisions of a labelled step.
const METRIC_SUBDIVISIONS: f32 = 10.0;
/// Labelled inch steps in inches (DesignCraft's 1, 2, 5, 10 × base, `geom/src/units.rs:130-137`).
const INCH_STEPS: [f64; 9] = [1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0, 200.0, 500.0];
/// The coarsest inch step, when even 500 inches are closer than `step_min`.
const INCH_STEP_LAST: f64 = 1000.0;

/// The labelled inch step and its subdivisions at `points_per_inch` screen points per inch:
/// labels `step_min` points apart at least, ticks `step_min / 10` apart at least (the metric rule).
fn inch_step(points_per_inch: f64, step_min: f32, finest: &[u32]) -> (f64, u32) {
    let step = INCH_STEPS.into_iter().find(|step| step * points_per_inch >= f64::from(step_min)).unwrap_or(INCH_STEP_LAST);
    if step > 1.0 {
        return (step, 2);
    }
    let gap = f64::from(step_min / METRIC_SUBDIVISIONS);
    let subdivisions = finest.iter().copied().find(|n| points_per_inch / f64::from(*n) >= gap).unwrap_or(1);
    (step, subdivisions)
}

/// A zero point command of the host's, waiting for the rulers' next pass.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Command {
    Reset,
    Set(Point),
}

/// The rulers of a canvas: ticks and labels counted from a zero point, level whatever the view's
/// rotation, a pointer marker, the hand-off of guide drags and the zero point's corner.
///
/// ```
/// use craft_ui::canvas::{CanvasView, Guides, RulerNames, RulerUnits, Rulers, kurbo::Point};
/// use craft_ui::localized::LocaleNumbers;
/// # egui::__run_test_ui(|ui| {
/// let px = RulerUnits::pixels(LocaleNumbers::from_tag("en"));
/// let full = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(420.0, 320.0));
/// let tokens = <craft_ui::theme::Tokens as craft_ui::theme::TokensOf>::of(ui);
/// let content = Rulers::content_rect(&tokens, full);
/// let view = CanvasView::new(content, Point::new(200.0, 150.0), 1.0);
/// let names = RulerNames::new("Horizontal ruler", "Vertical ruler", "Ruler origin");
/// let rulers = Rulers::new("rulers", &px, &px, names).show(ui, full, &view, None);
/// # let _ = rulers;
/// # });
/// ```
///
/// - Lay the canvas out inside [`Rulers::content_rect`] of the area `full`, build the
///   [`CanvasView`] over it, show the guides, then the rulers with the guides'
///   [`id`](super::GuidesResponse::id).
/// - A drag on the top ruler starts a horizontal guide, one on the left ruler a vertical guide
///   ([`Guides::drag_from_ruler`]); the guides run it to the release.
/// - A drag out of the corner moves the zero point: cross hairs show where it lands, Shift snaps
///   to the ticks (otherwise the host's [`snap`](Self::snap)), dropped back on the corner it
///   stays, Esc cancels. Double-click reports [`reset_origin`](RulersResponse::reset_origin).
///   [`origin_locked`](Self::origin_locked) turns both off.
/// - The rulers and the corner are not Tab stops: their functions have keyboard paths in the
///   host's menus. The host's Reset Zero Point and Set Zero Point… commands call
///   [`Rulers::reset_zero`] and [`Rulers::set_zero`], which the rulers report like the corner's
///   gestures; every item of the host's ruler menu needs such a menu-bar or Preferences route,
///   since the menu opens from a pointer only. Each part is in the AccessKit tree.
pub struct Rulers<'a> {
    id_salt: IdSalt,
    units: (&'a RulerUnits, &'a RulerUnits),
    zero: Point,
    origin_locked: bool,
    interactive: bool,
    snap: Option<Snap<'a>>,
    names: RulerNames,
    policy: EditPolicy,
    units_menu: Option<(&'a [LengthUnit], LengthUnit)>,
    /// The margin parts of the top and left strips, document units along each axis.
    margins: (Vec<Range<f64>>, Vec<Range<f64>>),
}

/// The host's snapping of the zero point ([`Rulers::snap`]).
type Snap<'a> = Box<dyn Fn(Orientation, f64, Modifiers) -> f64 + 'a>;

impl<'a> Rulers<'a> {
    /// Rulers measuring the x axis in `x` and the y axis in `y`, named `names` in the AccessKit
    /// tree.
    pub fn new(id_salt: impl AsIdSalt, x: &'a RulerUnits, y: &'a RulerUnits, names: RulerNames) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            units: (x, y),
            zero: Point::ZERO,
            origin_locked: false,
            interactive: true,
            snap: None,
            names,
            policy: EditPolicy::default(),
            units_menu: None,
            margins: (Vec::new(), Vec::new()),
        }
    }

    /// The margin parts of the strips, as document ranges along the x axis (the top strip) and the
    /// y axis (the left strip): painted in [`RulerStyle::margin_shade`] under the ticks, while the
    /// text part keeps the strip's fill. A page view's margins, a text frame's insets. A reversed
    /// range is read the right way round.
    pub fn margins(mut self, horizontal: impl IntoIterator<Item = Range<f64>>, vertical: impl IntoIterator<Item = Range<f64>>) -> Self {
        self.margins = (horizontal.into_iter().collect(), vertical.into_iter().collect());
        self
    }

    /// The document point the rulers count from.
    pub fn zero(mut self, zero: Point) -> Self {
        self.zero = zero;
        self
    }

    /// Whether the zero point is locked (DesignCraft's Lock Zero Point).
    pub fn origin_locked(mut self, locked: bool) -> Self {
        self.origin_locked = locked;
        self
    }

    /// Whether the rulers take the pointer; `false` paints them only (PhotoCraft's other tiles).
    pub fn interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }

    /// The host's snapping of the zero point when Shift is not held: the document coordinate the
    /// pointer is at on `orientation`'s axis (x for `Vertical`) to the one it lands on.
    pub fn snap(mut self, snap: impl Fn(Orientation, f64, Modifiers) -> f64 + 'a) -> Self {
        self.snap = Some(Box::new(snap));
        self
    }

    /// How the rulers report gestures.
    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Photoshop's ruler menu: a secondary click on a ruler or the corner opens a menu of
    /// `units` with `current` checked, and the pick comes back in
    /// [`RulersResponse::unit_picked`]. Names from [`LengthUnitLabels`](crate::localized::units::LengthUnitLabels) installed in the context.
    /// Without it the host gets [`RulersResponse::context`] and opens its own menu.
    pub fn units_menu(mut self, units: &'a [LengthUnit], current: LengthUnit) -> Self {
        self.units_menu = Some((units, current));
        self
    }

    /// The host's Reset Zero Point command for the rulers `rulers` ([`RulersResponse::id`]): their
    /// next pass reports [`reset_origin`](RulersResponse::reset_origin), as a double-click on the
    /// corner does. Nothing while the zero point is locked.
    pub fn reset_zero(ctx: &Context, rulers: Id) {
        ctx.data_mut(|d| d.insert_temp(command_key(rulers), Command::Reset));
        ctx.request_repaint();
    }

    /// The host's Set Zero Point… command: `point` is the document point the host's dialog took.
    /// The rulers' next pass reports it as a Set of [`origin`](RulersResponse::origin), after a
    /// zero point drag in progress ends. Nothing while the zero point is locked.
    pub fn set_zero(ctx: &Context, rulers: Id, point: Point) {
        ctx.data_mut(|d| d.insert_temp(command_key(rulers), Command::Set(point)));
        ctx.request_repaint();
    }

    /// The canvas's rect inside the rulers of the area `full`.
    pub fn content_rect(tokens: &UiTokens, full: ScreenRect) -> ScreenRect {
        let size = tokens.component.ruler.size;
        ScreenRect::from_min_max(full.min + vec2(size, size), full.max)
    }

    /// The ruler look in `tokens`.
    pub fn style(tokens: &UiTokens) -> RulerStyle {
        let (r, c) = (&tokens.component.ruler, &tokens.color);
        let family = tokens.families().family(craft_theme::FontSlot::Ui).clone();
        RulerStyle {
            size: r.size,
            font: FontId::new(r.label, family),
            label_pitch: r.label_pitch,
            label_pad: r.label_pad,
            step_min: r.step_min,
            tick_mid: r.tick_mid,
            tick_minor: r.tick_minor,
            fill: c.surface.chrome.to_egui(),
            tick: Stroke::new(1.0, c.text.faint.to_egui()),
            label: c.text.muted.to_egui(),
            marker: Stroke::new(1.0, c.text.normal.to_egui()),
            margin_shade: r.margin_shade.to_egui(),
        }
    }

    /// Draws the rulers in `full` for `view` (whose rect is [`content_rect`](Self::content_rect)
    /// of `full`). `guides` is the [`id`](super::GuidesResponse::id) of the guides that run the
    /// drags out of the rulers; `None` makes the rulers no guide source.
    pub fn show(self, ui: &mut Ui, full: ScreenRect, view: &CanvasView, guides: Option<Id>) -> RulersResponse {
        let ctx = ui.ctx().clone();
        let tokens = Tokens::of(ui);
        let style = Rulers::style(&tokens);
        let id = ui.make_persistent_id(self.id_salt);
        let size = style.size;
        let top = ScreenRect::from_min_max(pos2(full.left() + size, full.top()), pos2(full.right(), full.top() + size));
        let left = ScreenRect::from_min_max(pos2(full.left(), full.top() + size), pos2(full.left() + size, full.bottom()));
        let corner = ScreenRect::from_min_size(full.min, vec2(size, size));
        let mut out = RulersResponse { id, origin: None, reset_origin: false, context: None, unit_picked: None };

        let painter = ui.painter_at(full);
        for r in [top, left, corner] {
            painter.rect_filled(r, 0.0, style.fill);
        }
        let level = view.level(self.zero);
        for (spans, strip, vertical) in [(&self.margins.0, top, false), (&self.margins.1, left, true)] {
            let clipped = painter.with_clip_rect(strip);
            for span in spans {
                let at = |along: f64| {
                    let s = level.to_screen(if vertical { Point::new(self.zero.x, along) } else { Point::new(along, self.zero.y) });
                    if vertical { s.y } else { s.x }
                };
                let (a, b) = (at(span.start), at(span.end));
                let (lo, hi) = (a.min(b), a.max(b));
                let shade = if vertical {
                    ScreenRect::from_min_max(pos2(strip.left(), lo), pos2(strip.right(), hi))
                } else {
                    ScreenRect::from_min_max(pos2(lo, strip.top()), pos2(hi, strip.bottom()))
                };
                clipped.rect_filled(shade, 0.0, style.margin_shade);
            }
        }
        let half = 0.5 / painter.pixels_per_point();
        Divider::paint(&painter, pos2(top.left(), top.bottom() - half), pos2(top.right(), top.bottom() - half), &tokens);
        Divider::paint(&painter, pos2(left.right() - half, left.top()), pos2(left.right() - half, left.bottom()), &tokens);

        let (x, y) = self.units;
        self.axis(&painter.with_clip_rect(top), &style, &level, top, false, x, view.zoom_x());
        self.axis(&painter.with_clip_rect(left), &style, &level, left, true, y, view.zoom);

        if let Some(h) = ctx.pointer_hover_pos().filter(|h| full.contains(*h)) {
            let (p, l) = (painter.with_clip_rect(top), painter.with_clip_rect(left));
            crisp(&p, pos2(h.x, top.top()), pos2(h.x, top.bottom()), style.marker);
            crisp(&l, pos2(left.left(), h.y), pos2(left.right(), h.y), style.marker);
        }

        let sense = if self.interactive { Sense::click_and_drag() - Sense::FOCUSABLE } else { Sense::hover() };
        let parts = [(RulerPart::Horizontal, top, "top"), (RulerPart::Vertical, left, "left"), (RulerPart::Corner, corner, "corner")];
        let [top_response, left_response, corner_response] = parts.map(|(part, rect, name)| {
            let response = ui.interact(rect, id.with(name), sense);
            if response.secondary_clicked() {
                out.context = Some(part);
            }
            response
        });
        let enabled = ui.is_enabled();
        for (response, orientation, name, units) in [
            (&top_response, accesskit::Orientation::Horizontal, &self.names.horizontal, x),
            (&left_response, accesskit::Orientation::Vertical, &self.names.vertical, y),
        ] {
            response.widget_info(|| {
                let mut info = WidgetInfo::labeled(Role::Group, enabled, name);
                info.current_text_value = Some(units.unit_name());
                info
            });
            ctx.accesskit_node_builder(response.id, |node| node.set_orientation(orientation));
        }
        corner_response.widget_info(|| {
            let mut info = WidgetInfo::labeled(Role::GraphicsObject, enabled, &self.names.origin);
            info.current_text_value = Some(format!("{}, {}", x.format_value(self.zero.x, 0.0), y.format_value(self.zero.y, 0.0)));
            info
        });

        if self.interactive
            && let Some(guides) = guides
        {
            for (response, orientation) in [(&top_response, Orientation::Horizontal), (&left_response, Orientation::Vertical)] {
                if response.hovered() || response.dragged() {
                    ctx.set_cursor_icon(resize_cursor(view, orientation.across()));
                }
            }
            if ui.is_enabled() {
                hand_off(&ctx, id.with("press"), guides, ui.layer_id(), [(top, Orientation::Horizontal), (left, Orientation::Vertical)]);
            }
        }
        self.corner(ui, &style, view, &corner_response, id, &mut out);
        if let Some((units, current)) = self.units_menu {
            let pressed = [&top_response, &left_response, &corner_response].into_iter().find(|r| r.secondary_clicked()).cloned();
            out.context = None;
            out.unit_picked = units_menu(ui, id, pressed.as_ref(), units, current);
        }
        out
    }

    /// Ticks and labels of one ruler along `strip`, `level` giving the position of each tick.
    #[allow(clippy::too_many_arguments)]
    fn axis(&self, painter: &Painter, style: &RulerStyle, level: &CanvasView, strip: ScreenRect, vertical: bool, units: &RulerUnits, zoom: f64) {
        let zero = if vertical { self.zero.y } else { self.zero.x };
        if !(units.per_doc.is_finite() && units.per_doc > 0.0) {
            return;
        }
        // The ruler's value (units from the zero point) at a screen coordinate along it and back.
        let value_at = |pos: Pos2| {
            let d = level.to_doc(pos);
            ((if vertical { d.y } else { d.x }) - zero) * units.per_doc
        };
        let screen_at = |value: f64| {
            let along = zero + value / units.per_doc;
            let p = if vertical { Point::new(self.zero.x, along) } else { Point::new(along, self.zero.y) };
            let s = level.to_screen(p);
            if vertical { s.y } else { s.x }
        };
        let (a, b) = if vertical { (strip.left_top(), strip.left_bottom()) } else { (strip.left_top(), strip.right_top()) };
        let (v0, v1) = (value_at(a), value_at(b));
        let font = &style.font;
        for tick in Rulers::ticks(units, TickRule::Metric, style, zoom, v0.min(v1)..=v0.max(v1)) {
            let c = screen_at(tick.value);
            if vertical {
                crisp(painter, pos2(strip.right() - tick.length, c), pos2(strip.right(), c), style.tick);
            } else {
                crisp(painter, pos2(c, strip.bottom() - tick.length), pos2(c, strip.bottom()), style.tick);
            }
            let Some(label) = tick.label else { continue };
            if vertical {
                // One character per line, centred in the strip: a narrow "1" lines up with a "0".
                for (i, ch) in label.chars().enumerate() {
                    let at = pos2(strip.center().x, c + style.label_pad + i as f32 * style.label_pitch);
                    painter.text(at, Align2::CENTER_TOP, ch, font.clone(), style.label);
                }
            } else {
                painter.text(pos2(c + style.label_pad, strip.top() + LABEL_TOP), Align2::LEFT_TOP, label, font.clone(), style.label);
            }
        }
    }

    /// The ticks of one ruler over `values` (ruler units from the zero point): the one tick rule
    /// of every ruler, so a text ruler or a page view's ruler draws the same ticks as the canvas's.
    /// One labelled step beyond each end is included, since a label starts right of its tick.
    /// `zoom` is screen points per document unit along the axis, as [`RulerUnits::step`] takes it.
    /// Empty when the units are unusable or the span would need more than 20,000 ticks.
    pub fn ticks(units: &RulerUnits, rule: TickRule<'_>, style: &RulerStyle, zoom: f64, values: RangeInclusive<f64>) -> Vec<RulerTick> {
        let (step, subdivisions) = match rule {
            TickRule::Metric => units.step(zoom, style.step_min),
            TickRule::Inches(finest) => inch_step(zoom / units.per_doc, style.step_min, finest),
        };
        let minor = step / f64::from(subdivisions);
        if !(minor.is_finite() && minor > 0.0 && units.per_doc.is_finite() && units.per_doc > 0.0) {
            return Vec::new();
        }
        let (lo, hi) = (*values.start(), *values.end());
        let subdivisions = i64::from(subdivisions);
        let (first, last) = ((lo / minor).floor() as i64 - subdivisions, (hi / minor).ceil() as i64 + subdivisions);
        if !(last - first <= MAX_TICKS) {
            return Vec::new();
        }
        (first..=last)
            .map(|m| {
                let rem = m.rem_euclid(subdivisions);
                let (length, label) = if rem == 0 {
                    (style.size, Some(units.tick_label((m / subdivisions) as f64 * step, step)))
                } else if subdivisions % 2 == 0 && rem == subdivisions / 2 {
                    (style.tick_mid, None)
                } else {
                    (style.tick_minor, None)
                };
                RulerTick { value: m as f64 * minor, length, label }
            })
            .collect()
    }

    /// The zero point: dragged out of the corner, reset by its double-click, and the host's
    /// commands.
    fn corner(&self, ui: &Ui, style: &RulerStyle, view: &CanvasView, corner: &egui::Response, id: Id, out: &mut RulersResponse) {
        let ctx = ui.ctx();
        let locked = self.origin_locked || !ui.is_enabled();
        let pointer = self.interactive && !locked;
        if pointer && (corner.hovered() || corner.dragged()) {
            ctx.set_cursor_icon(CursorIcon::Crosshair);
        }
        if pointer && corner.double_clicked() {
            out.reset_origin = true;
        }
        let running_key = corner.id.with("running");
        let reported = Gesture::<Point>::reported(ui, corner.id).map(|(_, value)| value);
        let mut gesture = Gesture::load(ui, corner.id, &reported.unwrap_or(self.zero), self.policy).focus_owners(&[]).enabled(!locked);
        let mut running: bool = ctx.data(|d| d.get_temp(running_key)).unwrap_or(false);
        if running && !gesture.is_active() {
            // Esc or a change by the host ended it; the rest of the drag does nothing.
            running = false;
        }
        if corner.drag_started() && pointer {
            gesture.begin();
            running = gesture.is_active();
        }
        if running {
            let (pointer, down, mods) = ctx.input(|i| (i.pointer.latest_pos(), i.pointer.primary_down(), i.modifiers));
            let start = gesture.start().copied().unwrap_or(self.zero);
            let at = match pointer {
                Some(p) if !corner.rect.contains(p) => self.landing(view, p, mods, style.step_min),
                _ => start,
            };
            gesture.update(at, mods);
            if at != start {
                self.crosshairs(ui, style, view, at);
            }
            if !down {
                gesture.commit();
                running = false;
            }
        }
        // A command waits while a drag runs; the lock drops it.
        let command =
            (!running).then(|| ctx.data_mut(|d| d.get_temp::<Command>(command_key(id)).inspect(|_| d.remove::<Command>(command_key(id))))).flatten();
        if let Some(command) = command
            && !locked
        {
            match command {
                Command::Reset => out.reset_origin = true,
                Command::Set(point) => gesture.set(round_zero(point)),
            }
        }
        ctx.data_mut(|d| d.insert_temp(running_key, running));
        out.origin = gesture.finish();
    }

    /// Where the zero point lands for the pointer at screen position `pointer`: on the nearest
    /// tick of the ruler with Shift, else where the host's snap puts it.
    fn landing(&self, view: &CanvasView, pointer: Pos2, mods: Modifiers, step_min: f32) -> Point {
        let d = view.to_doc(pointer);
        let (x, y) = self.units;
        let (px, py) = if tick_snap(mods) {
            (x.snap_to_tick(d.x, self.zero.x, view.zoom_x(), step_min), y.snap_to_tick(d.y, self.zero.y, view.zoom, step_min))
        } else if let Some(snap) = &self.snap {
            (snap(Orientation::Vertical, d.x, mods), snap(Orientation::Horizontal, d.y, mods))
        } else {
            (d.x, d.y)
        };
        round_zero(Point::new(px, py))
    }

    /// Cross hairs along the document axes through `at`, drawn like guides so they turn with the
    /// view.
    fn crosshairs(&self, ui: &Ui, style: &RulerStyle, view: &CanvasView, at: Point) {
        let painter = ui.painter_at(view.rect);
        let visible = view.visible();
        for (orientation, position) in [(Orientation::Vertical, at.x), (Orientation::Horizontal, at.y)] {
            if let Some([a, b]) = segment(view, visible, orientation, position, None) {
                crisp(&painter, a, b, style.marker);
            }
        }
    }
}

fn command_key(rulers: Id) -> Id {
    rulers.with("craft_ui::canvas::Rulers::command")
}

/// `p` written to the zero point's decimals.
fn round_zero(p: Point) -> Point {
    let round = |v: f64| (v * ZERO_ROUND).round() / ZERO_ROUND;
    Point::new(round(p.x), round(p.y))
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Rulers"];

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq, Eq)]
enum UnitsArg {
    #[choice(name = "Pixels")]
    Pixels,
    #[choice(name = "Millimetres")]
    Millimetres,
    #[choice(name = "Inches")]
    Inches,
}

#[derive(crate::story::StoryArgs, Debug, PartialEq)]
struct CanvasArgs {
    #[arg(flatten)]
    view: ViewArgs,
    units: UnitsArg,
    interactive: bool,
    locked: bool,
}

impl Default for CanvasArgs {
    fn default() -> Self {
        Self { view: ViewArgs::default(), units: UnitsArg::Pixels, interactive: true, locked: false }
    }
}

pub(crate) fn stories() -> Vec<Story> {
    vec![typed_story!("primitives/rulers/canvas", Level::Primitives, "Rulers", "Canvas", CanvasArgs::default(), show_canvas).on(Surface::Canvas)]
}

/// A ruler menu's commands in the story.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MenuCommand {
    Units(UnitsArg),
    ResetZero,
    SetZero,
}

/// Rulers, guides and a marquee on the story document: drag guides out of the rulers (Alt turns
/// them), drag the corner to move the zero point (Shift snaps, Esc cancels, double-click resets),
/// drag a marquee on the canvas; right-click a ruler for its unit and the zero point commands an
/// app also has in its menu bar.
fn show_canvas(ui: &mut Ui, args: &mut CanvasArgs) {
    use super::guides::GuideHost;
    use super::{Guide, GuideNames, MarqueeDrag, MarqueeShape};
    use kurbo::Rect;

    let tokens = Tokens::of(ui);
    let document_name = crate::story::StoryLabels::of(ui).document;
    let canvas = super::story::document_in(ui, args.view, &document_name, tokens.component.ruler.size);
    let (response, view, full) = (&canvas.response, &canvas.view, canvas.full);

    let key = ui.scope_id().with("rulers-story");
    let mut host: GuideHost = ui
        .data(|d| d.get_temp(key.with("guides")))
        .unwrap_or_else(|| GuideHost::new(vec![Guide::new(1, Orientation::Vertical, 160.0), Guide::new(2, Orientation::Horizontal, 120.0)]));
    let mut zero: Point = ui.data(|d| d.get_temp(key.with("zero"))).unwrap_or(Point::ZERO);
    let mut selection: Option<Rect> = ui.data(|d| d.get_temp(key.with("selection")));

    let numbers = crate::localized::LocaleNumbers::from_tag("en").ungrouped();
    let units = match args.units {
        UnitsArg::Millimetres => RulerUnits {
            per_doc: 25.4 / 96.0,
            ticks: super::Ticks::Decimal { whole: false },
            numbers,
            unit: crate::localized::Unit::new("{n} mm"),
            decimals: 2,
        },
        UnitsArg::Inches => RulerUnits {
            per_doc: 1.0 / 96.0,
            ticks: super::Ticks::Decimal { whole: false },
            numbers,
            unit: crate::localized::Unit::new("{n} in"),
            decimals: 3,
        },
        UnitsArg::Pixels => RulerUnits::pixels(numbers),
    };

    let guides = Guides::new("rulers-story-guides", &host.guides, &units, &units, GuideNames::of(ui))
        .selected(host.selected)
        .interactive(args.interactive)
        .bounds(super::story::DOC)
        .zero(zero)
        .readout("X:", "Y:")
        .show(ui, response, view);
    let marquee = MarqueeDrag::new("rulers-story-marquee").shape(MarqueeShape::Rect).show(ui, response, view);
    if marquee.clicked {
        selection = None;
    }
    if let Some(EditEvent::Commit { value, changed: true, .. }) = &marquee.event {
        selection = Some(value.rect());
    }
    if let Some(rect) = marquee.rect.or(selection) {
        MarqueeShape::Rect.marquee(rect, "Selection").bounds_value(&units.numbers, &units.unit).show(ui, response, view);
    }
    host.apply(&guides);

    let locked = args.locked;
    let rulers = Rulers::new("rulers-story", &units, &units, RulerNames::of(ui)).zero(zero).origin_locked(locked).interactive(args.interactive).show(
        ui,
        full,
        view,
        Some(guides.id),
    );
    let open_menu = rulers.context.map(|_| OpenAt::Pointer(ui.input(|i| i.pointer.interact_pos()).unwrap_or(full.min)));
    let mut entries: Vec<MenuEntry<MenuCommand>> = (0..UnitsArg::OPTIONS.len())
        .map(UnitsArg::from_index)
        .map(|units| MenuItem::new(units.name(), MenuCommand::Units(units)).radio(units == args.units).into())
        .collect();
    entries.push(MenuEntry::Separator);
    entries.push(MenuItem::new("Reset Zero Point", MenuCommand::ResetZero).enabled(!locked).into());
    entries.push(MenuItem::new("Set Zero Point…", MenuCommand::SetZero).enabled(!locked).into());
    let dialog_key = key.with("set-zero");
    match ContextMenu::new(&entries).open_at(open_menu).own_gestures(false).show(ui, response).chosen {
        Some(MenuCommand::Units(units)) => args.units = units,
        Some(MenuCommand::ResetZero) => Rulers::reset_zero(ui.ctx(), rulers.id),
        Some(MenuCommand::SetZero) => ui.data_mut(|d| {
            d.insert_temp(dialog_key, zero);
        }),
        None => {}
    }
    set_zero_dialog(ui, dialog_key, &units, rulers.id);
    if rulers.reset_origin {
        zero = Point::ZERO;
    }
    if let Some(EditEvent::Commit { value, .. } | EditEvent::Set { value, .. }) = rulers.origin {
        zero = value;
    }

    ui.data_mut(|d| {
        d.insert_temp(key.with("guides"), host);
        d.insert_temp(key.with("zero"), zero);
        match selection {
            Some(r) => {
                d.insert_temp(key.with("selection"), r);
            }
            None => {
                d.remove::<Rect>(key.with("selection"));
            }
        }
    });
}

/// The story's Set Zero Point… dialog while its point is in `key`: X and Y in document pixels,
/// OK hands the point to the rulers.
fn set_zero_dialog(ui: &mut Ui, key: Id, units: &RulerUnits, rulers: Id) {
    use crate::controls::{Button, NumberField};

    let Some(mut point) = ui.data(|d| d.get_temp::<Point>(key)) else { return };
    let mut done = None;
    let modal = egui::Modal::new(key.with("modal")).show(ui.ctx(), |ui| {
        ui.label("Set Zero Point");
        for (label, value) in [("X", &mut point.x), ("Y", &mut point.y)] {
            let field = NumberField::new(label, label, *value, &units.numbers).show(ui);
            if let Some(EditEvent::Commit { value: v, .. } | EditEvent::Set { value: v, .. }) = field.event {
                *value = v;
            }
            if field.submitted {
                done = Some(true);
            }
        }
        ui.horizontal(|ui| {
            if ui.add(Button::new("OK")).clicked() {
                done = Some(true);
            }
            if ui.add(Button::new("Cancel")).clicked() {
                done = Some(false);
            }
        });
    });
    if modal.should_close() && done.is_none() {
        done = Some(false);
    }
    match done {
        Some(ok) => {
            if ok {
                Rulers::set_zero(ui.ctx(), rulers, point);
            }
            ui.data_mut(|d| d.remove::<Point>(key));
        }
        None => ui.data_mut(|d| {
            d.insert_temp(key, point);
        }),
    }
}

#[cfg(test)]
#[path = "rulers_tests.rs"]
mod tests;

/// The rows of a units menu: every unit of `units` named by the installed
/// [`LengthUnitLabels`](crate::localized::units::LengthUnitLabels), `current` marked. The command
/// of a row is the unit's index in `units`.
// pc: crates/ui-egui/src/canvas/rulers.rs:73 `unit_menu_items`.
pub fn unit_entries(ui: &Ui, units: &[LengthUnit], current: LengthUnit) -> Vec<MenuEntry<usize>> {
    let labels = LengthUnitLabels::of(ui);
    units.iter().enumerate().map(|(i, &unit)| MenuItem::new(unit.label(&labels), i).radio(unit == current).into()).collect()
}

/// The rulers' units menu: opened at the pointer by `pressed` (a secondary click on a part this
/// pass), kept open across passes under `id`, `current` checked. Returns the unit picked.
// pc: crates/ui-egui/src/canvas/rulers.rs:79 `unit_menu`, :73 `unit_menu_items`.
fn units_menu(ui: &mut Ui, id: Id, pressed: Option<&Response>, units: &[LengthUnit], current: LengthUnit) -> Option<LengthUnit> {
    // The menu belongs to one widget whichever ruler part opened it, so a second click on
    // another part moves the open menu instead of opening another.
    let anchor = ui.interact(ScreenRect::from_min_size(ui.max_rect().min, vec2(0.0, 0.0)), id.with("craft_ui::rulers::units_menu"), Sense::hover());
    let at = pressed.map(|r| OpenAt::Pointer(r.interact_pointer_pos().or_else(|| ui.input(|i| i.pointer.interact_pos())).unwrap_or(r.rect.center())));
    let entries = unit_entries(ui, units, current);
    let chosen = ContextMenu::new(&entries).open_at(at).own_gestures(false).show(ui, &anchor).chosen;
    chosen.and_then(|i| units.get(i)).copied()
}
