//! The style rows of the details sheet: a collection of rows that hold their own controls. Each row names its
//! style, offers Install or Remove (or shows the job's progress) at its end, and shows the sample in the style's
//! face under them.

use craft_behavior::{DisplayOrder, Flow, Insets, ItemSource, Proposal, Selection, SpanLayout};
use craft_ui::buttons::Button;
use craft_ui::contract::{
    Collection, CollectionProps, CollectionResponse, Common, Component, ContainerProps, Cx, ItemOutput, MeasureCx, NoValue, Outcome, PaintUi,
    Props as _, Role, SelectMode, Sizing, clicked,
};
use craft_ui::display::Progress;
use craft_ui::primitives::TextRun;
use craft_ui::runtime::env::Label;
use craft_ui::style::{ComponentStyle, Metrics, Stateful, StyleCx, TextLook, TypeRole, Variant};
use egui::{Pos2, Rect, Response, Vec2};

use super::DetailsWords;
use crate::FaceInfo;
use crate::panel::faces::Owner;
use crate::panel::specimen::{FontSample, SampleFace};

/// The action band at a row's end is this many control heights wide.
const ACTION_IN_HEIGHTS: f32 = 4.0;

/// What a style row offers at its end.
#[derive(Clone, Debug)]
pub(super) enum Action {
    None,
    /// Install the file at this path of the index.
    Install(String),
    /// Remove this installed face.
    Remove(Box<FaceInfo>),
    /// A job runs for the row; the fraction when known.
    Running(Option<f32>),
}

/// One style of the sheet.
#[derive(Clone, Debug)]
pub(super) struct StyleLine {
    pub label: String,
    pub face: Option<(Owner, FaceInfo)>,
    pub action: Action,
}

/// The lines as a collection source, keyed by position.
pub(super) struct Lines<'a>(pub &'a [StyleLine]);

impl DisplayOrder<usize> for Lines<'_> {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn key(&self, at: usize) -> usize {
        at
    }

    fn index_of(&self, key: &usize) -> Option<usize> {
        (*key < self.0.len()).then_some(*key)
    }

    fn text(&self, at: usize) -> Option<std::borrow::Cow<'_, str>> {
        Some(std::borrow::Cow::Borrowed(self.0[at].label.as_str()))
    }
}

impl ItemSource<usize> for Lines<'_> {}

/// A row's Install or Remove button was pressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Pressed;

/// The style of the list: no look of its own.
#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub(super) struct StyleListStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// The style rows of one family.
#[derive(craft_ui::contract::Component)]
#[component(collection)]
pub(super) struct StyleList {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    items: CollectionProps,
}

impl StyleList {
    /// A list named `label`; it is as tall as its rows.
    pub(super) fn new(label: impl Into<Label>) -> Self {
        StyleList {
            common: Common::new(),
            props: ContainerProps::new().sizing(Sizing::Content),
            items: CollectionProps::new().select(SelectMode::Single),
        }
        .label(label)
    }
}

impl Component for StyleList {
    const ROLE: Role = Role::LIST;
    type Style = StyleListStyle;
    type Output = CollectionResponse<usize, Pressed>;

    fn style(_: &StyleCx<'_>, _: StyleListStyleKey) -> StyleListStyle {
        StyleListStyle::default()
    }

    fn finish(self, mut cx: Cx<'_, Self>) -> Self::Output {
        cx.collection_response()
    }
}

impl Collection for StyleList {
    type Key = usize;
    type ItemLayout = SpanLayout;
    type Selection = Selection<usize>;
    type Item<'r> = StyleRow<'r>;
    type ItemEvent = Pressed;

    fn items(&self) -> &CollectionProps {
        &self.items
    }

    fn item_layout(&self, source: &dyn ItemSource<usize>, width: f32, cx: &StyleCx<'_>) -> SpanLayout {
        SpanLayout::sized(source, width, cx.control().height)
    }
}

/// The style of a row.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub(super) struct StyleRowStyle {
    #[style(metrics)]
    metrics: Metrics,
    /// The style's name.
    #[style(text)]
    name: TextLook,
    /// The name line and the buttons' height.
    height: f32,
    pad: f32,
}

/// What a row returns.
#[non_exhaustive]
pub(super) struct RowOut {
    pub response: Response,
    pressed: bool,
}

impl Outcome for RowOut {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

impl ItemOutput<Pressed> for RowOut {
    fn into_event(self) -> Option<Pressed> {
        self.pressed.then_some(Pressed)
    }
}

/// One style: its name, its action and its sample.
#[derive(craft_ui::contract::Component)]
pub(super) struct StyleRow<'r> {
    #[common]
    common: Common,
    line: &'r StyleLine,
    family: &'r str,
    sample: &'r str,
    points: f32,
    face: SampleFace<'r>,
    words: &'r DetailsWords,
    pressed: bool,
}

impl<'r> StyleRow<'r> {
    /// The row of `line`, a style of `family`, with the sample `sample` at `points` in `face`.
    pub(super) fn new(line: &'r StyleLine, family: &'r str, sample: &'r str, points: f32, face: SampleFace<'r>, words: &'r DetailsWords) -> Self {
        StyleRow { common: Common::new(), line, family, sample, points, face, words, pressed: false }.label(line.label.clone())
    }

    fn sample_leaf(&self) -> FontSample<'r> {
        FontSample::new(format!("{} {}: {}", self.family, self.line.label, self.sample), self.sample, self.face, self.points)
    }
}

impl Component for StyleRow<'_> {
    const ROLE: Role = Role::LIST_ITEM;
    type Style = StyleRowStyle;
    type Output = RowOut;

    fn style(cx: &StyleCx<'_>, _: StyleRowStyleKey) -> StyleRowStyle {
        let control = cx.control();
        StyleRowStyle {
            metrics: Metrics::new(),
            name: TextLook::new(TypeRole::Label, Stateful::new(cx.color(cx.tokens.color.text.muted))),
            height: control.height,
            pad: control.pad_x,
        }
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        let style = cx.style;
        let name = cx.layout(&TextRun::new(self.line.label.as_str())).size().x + 2.0 * style.pad;
        let room = cx.room.unwrap_or(name + ACTION_IN_HEIGHTS * style.height);
        let sample = cx.child("sample", &mut self.sample_leaf(), Proposal::Width(room));
        Vec2::new(room, style.height + sample.y)
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let style = cx.style;
        let rtl = cx.env.direction().is_rtl();
        let head = Rect::from_min_size(cx.rect.min, Vec2::new(cx.rect.width(), style.height));
        let (action, rest) = Flow::new(head, rtl).take_end(ACTION_IN_HEIGHTS * style.height);
        let name = TextRun::new(self.line.label.as_str()).role(style.name.role).color(cx.resolve(&style.name.color)).elide();
        ui.draw().text(rest.inset(Insets::horizontal(style.pad)), &name);
        let words = self.words;
        let style_name = [("style", self.line.label.as_str())];
        match &self.line.action {
            Action::None => {}
            Action::Install(_) => {
                let button = Button::new(words.install.clone()).description(words.install_style.fill(&style_name)).variant(Variant::SECONDARY);
                self.pressed = clicked(&ui.child("action", action, button));
            }
            Action::Remove(_) => {
                let button = Button::new(words.remove.clone()).description(words.remove_style.fill(&style_name)).variant(Variant::DANGER);
                self.pressed = clicked(&ui.child("action", action, button));
            }
            Action::Running(Some(fraction)) => {
                ui.child("action", action, Progress::new(*fraction).label(words.installing.clone()));
            }
            Action::Running(None) => {
                ui.child("action", action, Progress::indeterminate().label(words.installing.clone()));
            }
        }
        let band = Rect::from_min_max(Pos2::new(cx.rect.left(), cx.rect.top() + style.height), cx.rect.max);
        ui.child("sample", band, self.sample_leaf());
    }

    fn finish(self, cx: Cx<'_, Self>) -> RowOut {
        RowOut { response: cx.response.clone(), pressed: self.pressed }
    }
}
