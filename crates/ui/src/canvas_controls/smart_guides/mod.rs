//! `SmartGuides`: the alignment lines and distances shown while something moves, a canvas control that only paints
//! (20.6). It has no parts, so a press never lands on it.

#[cfg(feature = "testing")]
mod stories;

use crate::canvas::canvas_space;
use crate::contract::{Common, Component, Cx, MeasureCx, PaintUi, Role};
use crate::primitives::{PaintShape, Rule, StrokeWidth, TextRun};
use crate::style::{ComponentStyle, DataColor, RuleLook, StyleColor, StyleCx, TypeRole, UiTokens};
use craft_behavior::geometry::snap::SnapLine;
use craft_behavior::kurbo::{Point, Rect as DocRect};
use craft_behavior::{Axis, Metrics, Sense};
use craft_theme::{Rgba, contrast_ratio};
use egui::emath::GuiRounding as _;
use egui::{Color32, CornerRadius, Pos2, Response, Vec2};

/// How far a snap line without a span reaches along its line, in document units: past any view.
const WHOLE_VIEW: f64 = 1.0e6;

/// An alignment line of [`SmartGuides`], in document coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SmartLine {
    /// One end of the line.
    pub a: Point,
    /// The other end of the line.
    pub b: Point,
    /// An extension from an object's edge to a distance: dashed.
    pub extension: bool,
}

impl SmartLine {
    /// A solid line from `a` to `b`.
    pub const fn new(a: Point, b: Point) -> Self {
        Self { a, b, extension: false }
    }

    /// The same line as a dashed extension.
    #[must_use]
    pub const fn dashed(mut self) -> Self {
        self.extension = true;
        self
    }

    /// The line a snap drew: along its span, or across the whole view when it has none.
    pub fn from_snap(line: SnapLine) -> Self {
        let (from, to) = line.span.unwrap_or((-WHOLE_VIEW, WHOLE_VIEW));
        match line.along {
            Axis::Y => Self::new(Point::new(line.position, from), Point::new(line.position, to)),
            Axis::X => Self::new(Point::new(from, line.position), Point::new(to, line.position)),
        }
    }
}

/// A distance of [`SmartGuides`]: a line with end caps and a label, in document coordinates.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Measure {
    /// One end of the distance.
    pub a: Point,
    /// The other end of the distance.
    pub b: Point,
    /// The app's formatted distance (`24 px`).
    pub label: String,
}

impl Measure {
    /// The distance from `a` to `b`, labelled `label`.
    pub fn new(a: Point, b: Point, label: impl Into<String>) -> Self {
        Self { a, b, label: label.into() }
    }
}

/// The style of the smart guides.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct SmartGuidesStyle {
    #[style(metrics)]
    metrics: Metrics,
    /// Lines, distances and the labels' fill, unless the app gives a colour.
    smart: StyleColor,
    /// The moving object's bounds.
    bounds: StyleColor,
    /// Label text on a dark fill and on a light one; the one that contrasts more with the fill is used.
    ink_dark: StyleColor,
    ink_light: StyleColor,
    width: f32,
    dash: f32,
    /// The end caps of a distance.
    cap: f32,
    /// Around a distance's label.
    pad: Vec2,
    radius: CornerRadius,
}

/// The alignment lines and distances an app shows while something moves (Smart Guides and Alt distances).
///
/// Paint only: the app computes what aligns and how far apart things are. A [`SmartLine`] is a thin line in the smart
/// guide colour, an extension is dashed, a [`Measure`] has end caps and a label on a pill. [`color`](Self::color)
/// replaces the colour (an app's "Smart Guide Color" preference); the label text stays dark or light, whichever
/// contrasts more with it. [`bounds`](Self::bounds) outlines the moving object. Not in the accessibility tree: they
/// are hints during a gesture, whose result the moved object's own value reports.
#[derive(crate::contract::Component, crate::contract::Props)]
#[component(canvas_control, content_painter)]
pub struct SmartGuides<'a> {
    #[common]
    common: Common,
    lines: &'a [SmartLine],
    measures: &'a [Measure],
    /// The colour of the lines and distances, in place of the theme's smart guide colour.
    #[prop]
    color: Option<DataColor>,
    /// The document bounds of the object being moved, drawn as an outline under the lines.
    #[prop]
    bounds: Option<DocRect>,
}

impl<'a> SmartGuides<'a> {
    /// Shows `lines` and `measures`; both are in document coordinates.
    pub fn new(lines: &'a [SmartLine], measures: &'a [Measure]) -> Self {
        SmartGuides { common: Common::new(), lines, measures, color: None, bounds: None }
    }
}

fn contrast(fill: Color32, ink: StyleColor) -> f64 {
    let rgba = |color: Color32| Rgba(color.to_array());
    contrast_ratio(rgba(ink.egui()), rgba(fill))
}

impl Component for SmartGuides<'_> {
    const ROLE: Role = Role::None;
    const PARTS: bool = false;
    const STOPS: bool = false;
    const INPUT: craft_behavior::Input = craft_behavior::Input::Display;
    const SHOWS_LABEL: bool = false;
    const OVERLAYS: bool = false;
    type Style = SmartGuidesStyle;
    type Output = Response;

    fn style(cx: &StyleCx<'_>, _: SmartGuidesStyleKey) -> SmartGuidesStyle {
        let tokens = cx.tokens;
        SmartGuidesStyle {
            metrics: Metrics::new(),
            smart: cx.color(tokens.color.data.magenta),
            bounds: cx.color(tokens.color.accent.solid),
            ink_dark: cx.color(tokens.color.text.on_strong),
            ink_light: cx.color(tokens.color.text.normal),
            width: tokens.stroke.thin,
            dash: tokens.stroke.dash_gap,
            cap: tokens.space.gap.tight,
            pad: Vec2::new(tokens.space.pad.control, tokens.space.gap.tight),
            radius: UiTokens::corner(tokens.shape.radius.control),
        }
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::ZERO
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let Some(space) = canvas_space(&cx.env) else { return };
        let view = &space.view;
        let style = &cx.style;
        let fill = self.color.map_or(style.smart.egui(), |color| color.0);
        let painter = ui.content_painter(cx);
        let ppp = painter.pixels_per_point();
        let line = RuleLook { color: fill, width: style.width };
        if let Some(bounds) = self.bounds {
            let corners = view.quad(bounds).to_vec();
            ui.draw().stroke(&PaintShape::Polygon(corners), StrokeWidth::Thin, style.bounds);
        }
        for smart in self.lines {
            let (a, b) = (view.to_screen(smart.a), view.to_screen(smart.b));
            if smart.extension {
                painter.dashed(&[a, b], style.width, style.dash, style.dash, 0.0, fill);
            } else {
                painter.painter.extend(Rule::shapes(&line, a, b, ppp));
            }
        }
        let ink = if contrast(fill, style.ink_dark) >= contrast(fill, style.ink_light) { style.ink_dark } else { style.ink_light };
        let draw = ui.draw();
        for measure in self.measures {
            let (a, b) = (view.to_screen(measure.a), view.to_screen(measure.b));
            painter.painter.extend(Rule::shapes(&line, a, b, ppp));
            let along = (b - a).normalized();
            let cap = Vec2::new(-along.y, along.x) * (style.cap / 2.0);
            for end in [a, b] {
                painter.painter.extend(Rule::shapes(&line, end - cap, end + cap, ppp));
            }
            let run = TextRun::new(measure.label.as_str()).role(TypeRole::Caption).color(ink);
            let layout = draw.layout(&run, f32::INFINITY);
            let pill = egui::Rect::from_center_size(a.lerp(b, 0.5), layout.size() + 2.0 * style.pad).round_to_pixels(ppp);
            painter.painter.rect_filled(pill, style.radius, fill);
            let at: Pos2 = pill.center() - layout.size() / 2.0;
            draw.galley(at, &layout);
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}
