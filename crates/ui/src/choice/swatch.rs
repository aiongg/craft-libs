//! `Swatch`: a colour well. It shows a colour from data, a mixed look for several colours, and reports a click. It
//! edits nothing: the host opens its picker on the click, or takes the `ColorRequest` the swatch files.

use super::radio::RadioKind;
use super::words::ChoiceWords;
use crate::contract::{
    Access, Common, Component, Cx, GroupHandle, MeasureCx, Outcome, PaintUi, Pointer as _, Props as _, Role, Styled as _, WordSetCx as _,
};
use crate::patterns::form_dialogs::ColorRequest;
use crate::primitives::{Checker, PaintShape, StrokeWidth};
use crate::style::{ComponentStyle, DataColor, Flags, FocusLook, StyleColor, StyleCx, UiTokens, control_metrics};
use craft_behavior::{CursorIcon, CursorPriority, Metrics, Mixed, Sense, Value};
use craft_theme::{ControlSize, Opacity, Rgba, Srgba, Stateful, States};
use egui::{Color32, CornerRadius, Rect, Response, Shape, Vec2, pos2};

#[cfg(feature = "testing")]
mod stories;

/// The largest corner radius of a well, whatever the control radius is.
const RADIUS_MAX: f32 = 4.0;
/// Opacity of the black or white ring that stands out from a well's colour.
const INNER_EDGE_ALPHA: u8 = 0x80;
/// The length of the dash on a mixed well, as a share of the well's width.
const MIXED_DASH_SHARE: f32 = 0.5;

/// The sizes of a swatch at a control size: what a composition that lays out wells reads.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SwatchMetrics {
    /// The side of a square well: the control height.
    pub side: f32,
    /// The side of a cell of a table of touching swatches.
    pub cell: f32,
    /// The corners of a well: `shape.radius.control`, capped.
    pub radius: CornerRadius,
}

impl SwatchMetrics {
    /// The metrics at `size` from `tokens`.
    pub fn of(tokens: &UiTokens, size: ControlSize) -> SwatchMetrics {
        SwatchMetrics {
            side: control_metrics(tokens, size).height,
            cell: tokens.size.icon.md,
            radius: CornerRadius::same(tokens.shape.radius.control.min(RADIUS_MAX).round() as u8),
        }
    }
}

/// The style of a swatch.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct SwatchStyle {
    #[style(metrics)]
    metrics: Metrics,
    #[style(focus)]
    focus: FocusLook,
    /// The side of the square well, or of the cell.
    side: f32,
    /// The well's corners; square in a cell.
    radius: CornerRadius,
    /// The ring on the well's outline, which separates the well from the panel.
    edge: Stateful<StyleColor>,
    /// The ring inside the outline of a selected or checked well; clear while it is off.
    ring: Stateful<StyleColor>,
    /// The well of a mixed colour.
    mixed_fill: StyleColor,
    /// The dash on a mixed well.
    mixed_mark: StyleColor,
    /// The checkerboard behind a translucent colour: the colours of its corners and its other cells.
    checker: [Rgba; 2],
    /// The mixed well's colour, which `inner_edge` contrasts with.
    mixed_rgba: Rgba,
}

impl SwatchStyle {
    /// A ring of black or white at partial alpha that stands out from `color`, or from the mixed well for `None`: for
    /// a composition that paints a well apart from its colours.
    pub fn inner_edge(&self, color: Option<Srgba>) -> DataColor {
        let color = color.map_or(self.mixed_rgba, Srgba::to_rgba);
        let shown = self.checker.map(|behind| over(color, behind));
        let ring = |base: Rgba| base.with_alpha(INNER_EDGE_ALPHA);
        let least = |base: Rgba| shown.iter().map(|&s| craft_theme::contrast_ratio(over(ring(base), s), s)).fold(f64::INFINITY, f64::min);
        let base = if least(Rgba::BLACK) >= least(Rgba::WHITE) { Rgba::BLACK } else { Rgba::WHITE };
        let [r, g, b, a] = ring(base).0;
        DataColor::new(Color32::from_rgba_unmultiplied(r, g, b, a))
    }
}

/// `fg` composited over `bg`, in sRGB channels; opaque when `bg` is.
fn over(fg: Rgba, bg: Rgba) -> Rgba {
    let a = f64::from(fg.a()) / 255.0;
    let channel = |i: usize| (f64::from(fg.0[i]) * a + f64::from(bg.0[i]) * (1.0 - a)).round() as u8;
    Rgba([channel(0), channel(1), channel(2), bg.a()])
}

/// The colour with the disabled opacity.
fn faded(color: StyleColor) -> StyleColor {
    StyleColor(color.egui().gamma_multiply(Opacity::DISABLED))
}

/// What a swatch reports.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct SwatchOutput {
    /// The swatch's response.
    pub response: Response,
    /// A primary click, Space, Enter or an AccessKit click reached the enabled swatch.
    pub clicked: bool,
    /// With `picker`, the colour request the click files; the caller hands it to its host.
    pub pick_color: Option<ColorRequest>,
}

impl Outcome for SwatchOutput {
    type Value = crate::contract::NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// A colour well that shows a colour from data and reports a click; it edits nothing.
///
/// The colour fills the well over a checkerboard where it has alpha, inside a thin ring that separates it from the
/// panel. A `Mixed` colour shows a neutral well with a dash and has no colour value. A click reports `clicked`; with
/// `picker` it also files a `ColorRequest` for the host's colour picker.
///
/// AccessKit reports a colour well named by the label, whose value is the colour as `#rrggbb` (`#rrggbbaa` when
/// translucent) and whose colour value is the colour; a mixed well has the value "Mixed" and no colour value.
/// `radio` makes the swatch a member of the enclosing `RadioGroup`: a radio button, checked when the group holds its
/// value, whose keys and focus are the group's. `cell` makes it a cell of a table of touching swatches, a list box
/// option. The selected ring is `selected` or a checked radio.
#[derive(crate::contract::Component, crate::contract::Props)]
#[component(content_painter, probe(Swatch<u8>))]
pub struct Swatch<T: Value + 'static = ()> {
    #[common]
    common: Common,
    color: Mixed<Srgba>,
    /// The value this swatch stands for in its `RadioGroup`.
    member: Option<T>,
    /// The key of the `ColorRequest` a click files.
    #[prop(into)]
    picker: Option<String>,
    /// A cell of a table of touching swatches: `size.icon.md` square with square corners.
    #[style_key]
    #[prop(flag)]
    cell: bool,
    /// Takes the width the layout offers, at the control height.
    #[prop(flag)]
    fill: bool,
    checked: bool,
    clicked: bool,
    request: Option<ColorRequest>,
}

impl Swatch<()> {
    /// A swatch of `color` named `label`.
    pub fn new(label: impl Into<crate::runtime::env::Label>, color: impl Into<Mixed<Srgba>>) -> Self {
        Swatch {
            common: Common::new(),
            color: color.into(),
            member: None,
            picker: None,
            cell: false,
            fill: false,
            checked: false,
            clicked: false,
            request: None,
        }
        .label(label)
    }
}

impl<T: Value + 'static> Swatch<T> {
    /// Makes the swatch the member standing for `value` of the enclosing `RadioGroup<U>`.
    pub fn radio<U: Value + 'static>(self, value: U) -> Swatch<U> {
        Swatch {
            common: self.common,
            color: self.color,
            member: Some(value),
            picker: self.picker,
            cell: self.cell,
            fill: self.fill,
            checked: false,
            clicked: false,
            request: None,
        }
    }
}

impl<T: Value + 'static> Component for Swatch<T> {
    const ROLE: Role = Role::COLOR_WELL;
    const PARTS: bool = false;
    const OVERLAYS: bool = false;
    const ALT_ROLES: &'static [Role] = &[Role::RADIO_BUTTON, Role::LIST_BOX_OPTION];
    type Style = SwatchStyle;
    type Output = SwatchOutput;

    fn style(cx: &StyleCx<'_>, key: SwatchStyleKey<T>) -> SwatchStyle {
        let colors = &cx.tokens.color;
        let metrics = SwatchMetrics::of(cx.tokens, cx.size);
        let clear = cx.color(Rgba::TRANSPARENT);
        let rest = cx.color(colors.line.control_edge);
        let accent = cx.color(colors.accent.solid);
        SwatchStyle {
            metrics: Metrics::new(),
            focus: cx.focus_look(),
            side: if key.cell { metrics.cell } else { metrics.side },
            radius: if key.cell { CornerRadius::ZERO } else { metrics.radius },
            edge: Stateful::new(rest).off(States::new(rest, rest, rest, faded(rest))),
            ring: Stateful::new(clear).on(States::new(accent, accent, accent, faded(accent))),
            mixed_fill: cx.color(colors.surface.sunken),
            mixed_mark: cx.color(colors.text.muted),
            checker: [colors.overlay.checker_light, colors.overlay.checker_dark],
            mixed_rgba: colors.surface.sunken,
        }
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        let side = cx.style.side;
        let width = if self.fill { cx.room.filter(|room| room.is_finite()).map_or(side, |room| room.max(side)) } else { side };
        Vec2::new(width, side)
    }

    fn fills(&self) -> Option<craft_behavior::Axis> {
        self.fill.then_some(craft_behavior::Axis::X)
    }

    fn sense(&self) -> Sense {
        Sense::CLICK | Sense::FOCUS
    }

    /// Registers with the enclosing group when it is a radio; reports the click and files the colour request.
    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        if let Some(value) = &self.member {
            let group = cx.env.get::<GroupHandle<RadioKind<T>>>();
            group.register(value.clone(), cx.id, cx.resolved.enabled);
            cx.join_roving();
            self.checked = group.is(value);
        }
        self.clicked = cx.resolved.enabled && cx.activation().is_some();
        if self.clicked
            && let Some(key) = &self.picker
        {
            let opens_at = self.color.same().copied().unwrap_or(Srgba::BLACK);
            self.request = Some(ColorRequest::new(key.clone(), opens_at).title(cx.resolved.label_value.clone()));
        }
        if cx.resolved.enabled && cx.hit() {
            cx.cursor(CursorIcon::PointingHand, CursorPriority::Part);
        }
    }

    fn flags(&self) -> Flags {
        let mut flags = Flags::empty();
        flags.set(Flags::CHECKED, self.checked);
        flags
    }

    fn access(&self, access: &mut Access, cx: &Cx<'_, Self>) {
        if self.member.is_some() {
            access.role(Role::RADIO_BUTTON);
        } else if self.cell {
            access.role(Role::LIST_BOX_OPTION);
        }
        match self.color.same() {
            Some(color) => {
                access.value(color.to_hex());
                access.color(*color);
            }
            None => {
                access.value(cx.words::<ChoiceWords>().swatch_mixed.as_str());
            }
        }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let style = cx.style;
        let well = cx.rect;
        let shape = PaintShape::Rect(well, style.radius);
        match self.color.same() {
            None => {
                let draw = ui.draw();
                draw.fill(&shape, style.mixed_fill);
                let (center, half) = (well.center(), well.width() * MIXED_DASH_SHARE / 2.0);
                draw.stroke(
                    &PaintShape::Path(vec![pos2(center.x - half, center.y), pos2(center.x + half, center.y)]),
                    StrokeWidth::Medium,
                    style.mixed_mark,
                );
            }
            Some(color) => {
                if color.a < 1.0 {
                    let shapes = ui.with_style(cx.resolved.variant, |style| Checker::new().shapes(style.tokens, well, well));
                    ui.content_painter(cx).painter.extend(rounded(shapes, well, style.radius));
                }
                ui.draw().data_fill(&shape, DataColor::new(Color32::from(*color)));
            }
        }
        let draw = ui.draw();
        draw.stroke(&shape, StrokeWidth::Thin, cx.resolve(&style.edge));
        draw.stroke(&shape, StrokeWidth::Thick, cx.resolve(&style.ring));
    }

    fn finish(self, cx: Cx<'_, Self>) -> SwatchOutput {
        SwatchOutput { response: cx.response.clone(), clicked: self.clicked, pick_color: self.request }
    }
}

/// `shapes` of a checkerboard over `well` with the corners of `radius` where a cell meets a corner of the well.
fn rounded(mut shapes: Vec<Shape>, well: Rect, radius: CornerRadius) -> Vec<Shape> {
    for shape in &mut shapes {
        if let Shape::Rect(cell) = shape {
            let round = |touches: bool, r: u8| if touches { r } else { 0 };
            cell.corner_radius = CornerRadius {
                nw: round(cell.rect.min == well.min, radius.nw),
                ne: round(cell.rect.right_top() == well.right_top(), radius.ne),
                sw: round(cell.rect.left_bottom() == well.left_bottom(), radius.sw),
                se: round(cell.rect.max == well.max, radius.se),
            };
        }
    }
    shapes
}
