//! `FindingsList`: a list of findings, each a row of a severity glyph, its text and an action part that asks to show
//! the finding. The list is the contract's collection; the row is its item component.

use super::words::ListWords;
use crate::contract::{
    Bind, BindMut, BodyCx, Collection, CollectionProps, CollectionResponse, Common, Component, ContainerProps, Cx, ItemOutput, MeasureCx, NoValue,
    Outcome, PaintUi, PartsCx, Pointer as _, Props as _, Role, SelectMode, Show as _, Styled as _, WordSetCx as _,
};
use crate::controls::disclosure::DisclosureGlyph;
use crate::primitives::{PaintShape, Span, StrokeWidth, TextRun};
use crate::style::{ComponentStyle, StyleColor, StyleCx, SurfaceKind, SurfaceLook};
use craft_behavior::{DisplayOrder, Flow, Insets, ItemSource, Metrics, PartKey, Selection, Sense, SpanLayout};
use craft_theme::{Stateful, States, variant::Tone};
use egui::{Pos2, Response, Ui, Vec2};
use std::borrow::Cow;

/// How serious a finding is. The glyph's shape differs per severity as well as its colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Severity {
    /// For information.
    Info,
    /// Worth fixing.
    Warning,
    /// Must be fixed.
    Error,
}

/// One finding: how serious it is and what it says.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Finding {
    /// How serious the finding is.
    pub severity: Severity,
    /// What it says.
    pub text: String,
}

impl Finding {
    /// A finding of `severity` that says `text`.
    pub fn new(severity: Severity, text: impl Into<String>) -> Self {
        Finding { severity, text: text.into() }
    }
}

/// The findings of a slice as an item source; the key of a finding is its position.
#[derive(Clone, Copy, Debug)]
pub struct Findings<'a>(&'a [Finding]);

impl<'a> Findings<'a> {
    /// The findings of `findings`.
    pub const fn new(findings: &'a [Finding]) -> Self {
        Findings(findings)
    }
}

impl DisplayOrder<usize> for Findings<'_> {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn key(&self, at: usize) -> usize {
        at
    }

    fn index_of(&self, key: &usize) -> Option<usize> {
        (*key < self.0.len()).then_some(*key)
    }

    fn text(&self, at: usize) -> Option<Cow<'_, str>> {
        self.0.get(at).map(|finding| Cow::Borrowed(finding.text.as_str()))
    }
}

impl ItemSource<usize> for Findings<'_> {}

/// What a finding row reports.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FindingEvent {
    /// The action part asked to show the finding.
    Show,
}

/// What a finding row returns: its response and the action taken.
#[derive(Debug)]
#[non_exhaustive]
pub struct FindingRowOutput {
    /// The row's response.
    pub response: Response,
    /// The action part's event.
    pub event: Option<FindingEvent>,
}

impl Outcome for FindingRowOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

impl ItemOutput<FindingEvent> for FindingRowOutput {
    fn into_event(self) -> Option<FindingEvent> {
        self.event
    }
}

const ACTION: PartKey = PartKey::new("action");

/// The style of a finding row.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct FindingRowStyle {
    #[style(surface)]
    surface: SurfaceLook,
    #[style(metrics)]
    metrics: Metrics,
    text: Stateful<StyleColor>,
    action: Stateful<StyleColor>,
    info: StyleColor,
    warning: StyleColor,
    error: StyleColor,
    icon: f32,
}

/// A finding as a list row: severity glyph, text, and an action part at the end.
#[derive(crate::contract::Component)]
pub struct FindingRow<'r> {
    #[common]
    common: Common,
    finding: &'r Finding,
    shown: bool,
}

impl<'r> FindingRow<'r> {
    /// A row of `finding`; the finding's text names the row.
    pub fn new(finding: &'r Finding) -> Self {
        FindingRow { common: Common::new(), finding, shown: false }.label(finding.text.clone())
    }
}

impl Component for FindingRow<'_> {
    const ROLE: Role = Role::LIST_ITEM;
    type Style = FindingRowStyle;
    type Output = FindingRowOutput;

    fn style(cx: &StyleCx<'_>, _: FindingRowStyleKey) -> FindingRowStyle {
        let control = cx.control();
        let colors = &cx.tokens.color;
        let (state, text) = (&colors.state, &colors.text);
        let clear = cx.color(craft_theme::Rgba::TRANSPARENT);
        let fill = Stateful::new(clear).off(States::new(clear, cx.color(state.hover), cx.color(state.pressed), clear)).on(States::new(
            cx.color(state.selected),
            cx.color(state.selected_hover),
            cx.color(state.selected_hover),
            cx.color(state.selected),
        ));
        let on_selected = cx.color(text.on_selected);
        let ink = Stateful::new(cx.color(text.normal)).on(States::new(on_selected, on_selected, on_selected, on_selected));
        let accent = &colors.accent;
        let action = Stateful::new(cx.color(text.muted)).off(States::new(
            cx.color(text.muted),
            cx.color(accent.strong_hover),
            cx.color(accent.strong_pressed),
            cx.color(text.disabled),
        ));
        FindingRowStyle {
            surface: SurfaceLook::new(fill),
            metrics: Metrics::new().padding(Insets::horizontal(control.pad_x)).min_size(Vec2::new(0.0, control.height)).gap(control.gap),
            text: ink,
            action,
            info: cx.tone(Tone::Info),
            warning: cx.tone(Tone::Warn),
            error: cx.tone(Tone::Bad),
            icon: control.icon,
        }
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        let text = cx.layout(&TextRun::new(self.finding.text.as_str())).size();
        Vec2::new(text.x + cx.icon() * 2.0 + cx.gap() * 2.0, text.y.max(cx.icon()))
    }

    fn sense(&self) -> Sense {
        Sense::CLICK | Sense::FOCUS
    }

    fn parts(&mut self, rect: egui::Rect, cx: &mut PartsCx<'_, Self>) {
        let action = Flow::new(rect, cx.flow.is_rtl()).take_end(rect.height()).0;
        cx.part(ACTION, action).sense(Sense::CLICK).pointer_only("Enter on the row takes the same action");
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        self.shown = cx.part(ACTION).press.phase.is_click();
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let style = cx.style;
        let rtl = cx.env.direction().is_rtl();
        let (glyph, rest) = Flow::new(cx.content_rect, rtl).take_start(style.icon);
        let (_, rest) = rest.take_start(style.metrics.gap);
        let (action, _) = Flow::new(cx.rect, rtl).take_end(cx.rect.height());
        let text = Flow::new(rest.rect(), rtl).take_end(action.width()).1.rect();
        let draw = ui.draw();
        let (center, radius) = (glyph.center(), style.icon * GLYPH_SHARE);
        match self.finding.severity {
            Severity::Info => draw.stroke(&PaintShape::Circle(center, radius), StrokeWidth::Medium, style.info),
            Severity::Warning => {
                let corner = |angle: f32| -> Pos2 { center + Vec2::angled(angle) * radius };
                let points = [TRIANGLE_TOP, TRIANGLE_TOP + THIRD_TURN, TRIANGLE_TOP + 2.0 * THIRD_TURN].map(corner).to_vec();
                draw.fill(&PaintShape::Polygon(points), style.warning);
            }
            Severity::Error => draw.fill(&PaintShape::Circle(center, radius), style.error),
        }
        draw.text(text, &TextRun::new(self.finding.text.as_str()).color(cx.resolve(&style.text)).elide());
        let chevron = egui::Rect::from_center_size(action.center(), Vec2::splat(style.icon));
        let path = DisclosureGlyph::new(0.0, cx.env.direction()).path(chevron);
        draw.stroke(&PaintShape::Path(path), StrokeWidth::Medium, cx.resolve_part(ACTION, &style.action));
    }

    fn finish(self, cx: Cx<'_, Self>) -> FindingRowOutput {
        FindingRowOutput { response: cx.response.clone(), event: self.shown.then_some(FindingEvent::Show) }
    }
}

/// The glyph's radius as a share of the icon side.
const GLYPH_SHARE: f32 = 0.4;
/// The angle of the warning triangle's apex: straight up in screen coordinates.
const TRIANGLE_TOP: f32 = -std::f32::consts::FRAC_PI_2;
/// A third of a turn.
const THIRD_TURN: f32 = std::f32::consts::TAU / 3.0;

/// The style of the list: it has no look of its own beyond its container's surface.
#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct FindingsListStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// The list of findings: one selected finding, Enter activates, and an empty list says so.
#[derive(crate::contract::Component, crate::contract::Props)]
#[component(collection)]
pub struct FindingsList<'a> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    items: CollectionProps,
    /// The selection the caller owns.
    #[prop]
    selection: Option<Bind<'a, Selection<usize>>>,
}

impl<'a> FindingsList<'a> {
    /// A findings list named `label`.
    pub fn new(label: impl Into<crate::runtime::env::Label>) -> Self {
        FindingsList {
            common: Common::new(),
            props: ContainerProps::new().surface(SurfaceKind::Sunken),
            items: CollectionProps::new().select(SelectMode::Single),
            selection: None,
        }
        .label(label)
    }
}

impl Component for FindingsList<'_> {
    const ROLE: Role = Role::LIST;
    type Style = FindingsListStyle;
    type Output = CollectionResponse<usize, FindingEvent>;

    fn style(_: &StyleCx<'_>, _: FindingsListStyleKey) -> FindingsListStyle {
        FindingsListStyle::default()
    }

    fn finish(self, mut cx: Cx<'_, Self>) -> Self::Output {
        cx.collection_response()
    }
}

impl Collection for FindingsList<'_> {
    type Key = usize;
    type ItemLayout = SpanLayout;
    type Selection = Selection<usize>;
    type Item<'r> = FindingRow<'r>;
    type ItemEvent = FindingEvent;

    fn items(&self) -> &CollectionProps {
        &self.items
    }

    fn selection(&mut self) -> Option<BindMut<'_, Selection<usize>>> {
        self.selection.as_mut().map(Bind::reborrow)
    }

    fn item_layout(&self, source: &dyn ItemSource<usize>, width: f32, cx: &StyleCx<'_>) -> SpanLayout {
        SpanLayout::new(source, width, cx.control().height)
    }

    fn empty(&mut self, ui: &mut Ui, cx: &mut BodyCx<'_>) {
        let words = cx.words::<ListWords>();
        EmptyNote::new().label(words.no_findings.clone()).show_at(ui, cx.rect);
    }
}

/// The style of the empty note: the control's padding.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
struct EmptyNoteStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// The text of an empty list.
// F8-MERGE: Notice (display family) replaces this note.
#[derive(crate::contract::Component)]
#[component(exempt(case = "stories", reason = "The empty state of `FindingsList`, shown by its empty story."))]
struct EmptyNote {
    #[common]
    common: Common,
}

impl EmptyNote {
    fn new() -> Self {
        EmptyNote { common: Common::new() }
    }
}

impl Component for EmptyNote {
    const ROLE: Role = Role::LABEL;
    type Style = EmptyNoteStyle;
    type Output = Response;

    fn style(cx: &StyleCx<'_>, _: EmptyNoteStyleKey) -> EmptyNoteStyle {
        EmptyNoteStyle { metrics: Metrics::new().padding(Insets::horizontal(cx.control().pad_x)).min_size(Vec2::new(0.0, cx.control().height)) }
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        cx.layout(&TextRun::new(cx.label().as_str())).size()
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        ui.draw().text(cx.content_rect, &TextRun::spans(vec![Span::new(cx.resolved.label).muted()]));
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}
