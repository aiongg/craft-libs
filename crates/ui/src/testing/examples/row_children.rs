//! `LayerStack` and `LayerRow`: a layers panel whose rows hold their own controls (17.5).
//!
//! One tree collection. Each row shows its name, a visibility `Switch` and a blend-mode `Select` as child
//! components of the row. Their focus stops are members of the row's roving group: Tab passes the collection
//! once, Forward on a row enters the switch and then the select, Back returns, and Escape from a child returns to
//! its row. Groups open and close by Right and Left on the row.

use crate::choice::Switch;
use crate::contract::{
    Bind, Collection, CollectionEvent, CollectionProps, CollectionResponse, Common, Component, ContainerProps, Cx, Entry, ItemOutput, MeasureCx,
    Outcome, PaintUi, Props as _, Role, SelectMode, ShowCollection as _, Sizing,
};
use crate::pickers::Select;
use crate::runtime::env::Label;
use crate::style::{ComponentStyle, Ink, Stateful, StyleCx, TextLook, TypeRole};
use crate::testing::{Drive, StoryData, StoryOutput, show_as_only_item, story};
use craft_behavior::{DisplayOrder, Flow, Insets, ItemSource, Metrics, Selection, SpanLayout};
use egui::{Pos2, Rect, Response, Vec2};
use std::borrow::Cow;
use std::convert::Infallible;

/// The name band is this many control heights wide.
const NAME_IN_HEIGHTS: f32 = 5.0;
/// The blend-mode select is this many control heights wide.
const SELECT_IN_HEIGHTS: f32 = 5.0;
/// Each level of a group indents its rows by this many control heights.
const INDENT_IN_HEIGHTS: f32 = 1.0;

/// A layer or a group of layers.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Layer {
    /// The layer's name.
    pub name: String,
    /// The group levels around it.
    pub depth: usize,
    /// The layer holds the rows that follow it at a deeper depth.
    pub group: bool,
}

impl Layer {
    /// A layer named `name` at the top level.
    pub fn new(name: impl Into<String>) -> Self {
        Layer { name: name.into(), depth: 0, group: false }
    }

    /// The layer sits `depth` groups deep.
    #[must_use]
    pub fn depth(mut self, depth: usize) -> Self {
        self.depth = depth;
        self
    }

    /// The layer is a group.
    #[must_use]
    pub fn group(mut self) -> Self {
        self.group = true;
        self
    }
}

/// The layers as a collection source, keyed by position. `open` holds each row's open state.
#[non_exhaustive]
pub struct Layers<'a> {
    /// The layers in display order.
    pub layers: &'a [Layer],
    /// Whether each layer's children show; read for groups only.
    pub open: &'a [bool],
}

impl<'a> Layers<'a> {
    /// The source over `layers`, open as `open` says.
    pub const fn new(layers: &'a [Layer], open: &'a [bool]) -> Self {
        Layers { layers, open }
    }
}

impl DisplayOrder<usize> for Layers<'_> {
    fn len(&self) -> usize {
        self.layers.len()
    }

    fn key(&self, at: usize) -> usize {
        at
    }

    fn index_of(&self, key: &usize) -> Option<usize> {
        (*key < self.layers.len()).then_some(*key)
    }

    fn depth(&self, at: usize) -> usize {
        self.layers[at].depth
    }

    fn has_children(&self, at: usize) -> bool {
        self.layers[at].group
    }

    fn open(&self, at: usize) -> bool {
        self.open.get(at).copied().unwrap_or(false)
    }

    fn text(&self, at: usize) -> Option<Cow<'_, str>> {
        Some(Cow::Borrowed(self.layers[at].name.as_str()))
    }
}

impl ItemSource<usize> for Layers<'_> {}

/// The style of the stack: no look of its own.
#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct LayerStackStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// A tree of layers whose rows hold controls.
#[derive(crate::contract::Component)]
#[component(collection)]
pub struct LayerStack {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    items: CollectionProps,
}

impl LayerStack {
    /// A layer stack named `label`.
    pub fn new(label: impl Into<Label>) -> Self {
        LayerStack {
            common: Common::new(),
            props: ContainerProps::new().sizing(Sizing::Fill),
            items: CollectionProps::new().select(SelectMode::Single),
        }
        .label(label)
    }
}

impl Component for LayerStack {
    const ROLE: Role = Role::TREE;
    type Style = LayerStackStyle;
    type Output = CollectionResponse<usize, Infallible>;

    fn style(_: &StyleCx<'_>, _: LayerStackStyleKey) -> LayerStackStyle {
        LayerStackStyle::default()
    }

    fn finish(self, mut cx: Cx<'_, Self>) -> Self::Output {
        cx.collection_response()
    }
}

impl Collection for LayerStack {
    type Key = usize;
    type ItemLayout = SpanLayout;
    type Selection = Selection<usize>;
    type Item<'r> = LayerRow<'r>;
    type ItemEvent = Infallible;

    fn items(&self) -> &CollectionProps {
        &self.items
    }

    /// A row is one control high; the stack is a tree.
    fn item_layout(&self, source: &dyn ItemSource<usize>, width: f32, cx: &StyleCx<'_>) -> SpanLayout {
        SpanLayout::new(source, width, cx.control().height).tree()
    }
}

/// The style of a layer row.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct LayerRowStyle {
    #[style(metrics)]
    metrics: Metrics,
    /// The layer name.
    #[style(text)]
    name: TextLook,
    /// The row and control height.
    height: f32,
    pad: f32,
}

/// What a layer row returns.
#[non_exhaustive]
pub struct LayerOut {
    /// The row's response.
    pub response: Response,
}

impl Outcome for LayerOut {
    type Value = crate::contract::NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

impl ItemOutput<Infallible> for LayerOut {
    fn into_event(self) -> Option<Infallible> {
        None
    }
}

/// One layer: its name, a visibility switch and a blend-mode select, both child components of the row.
#[derive(crate::contract::Component)]
pub struct LayerRow<'r> {
    #[common]
    common: Common,
    layer: &'r Layer,
    modes: &'r [Entry<u8>],
}

impl<'r> LayerRow<'r> {
    /// A row of `layer` whose select chooses among `modes`.
    pub fn new(layer: &'r Layer, modes: &'r [Entry<u8>]) -> Self {
        LayerRow { common: Common::new(), layer, modes }.label(layer.name.clone())
    }
}

/// The part of `row` that starts `at` from its leading edge and runs `length`, over the row's full height.
fn band(flow: &Flow, row: Rect, at: f32, length: f32) -> Rect {
    let span = flow.span(at, length);
    Rect::from_min_max(Pos2::new(span.left(), row.top()), Pos2::new(span.right(), row.bottom()))
}

impl Component for LayerRow<'_> {
    const ROLE: Role = Role::TREE_ITEM;
    type Style = LayerRowStyle;
    type Output = LayerOut;

    fn style(cx: &StyleCx<'_>, _: LayerRowStyleKey) -> LayerRowStyle {
        LayerRowStyle {
            metrics: Metrics::new(),
            name: TextLook::new(TypeRole::Label, Stateful::new(cx.ink(Ink::OnPanel))),
            height: cx.control().height,
            pad: cx.control().pad_x,
        }
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        let height = cx.style.height;
        let indent = self.layer.depth as f32 * INDENT_IN_HEIGHTS;
        Vec2::new((indent + NAME_IN_HEIGHTS + 1.0 + SELECT_IN_HEIGHTS) * height, height)
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let rtl = cx.env.direction().is_rtl();
        let height = cx.style.height;
        let row = cx.rect;
        let flow = Flow::new(row, rtl);
        let start = self.layer.depth as f32 * INDENT_IN_HEIGHTS * height;
        let name = band(&flow, row, start, NAME_IN_HEIGHTS * height);
        ui.draw()
            .text(Flow::new(name, rtl).inset(Insets::horizontal(cx.style.pad)), &crate::primitives::TextRun::new(self.layer.name.as_str()).elide());
        let switch = band(&flow, row, start + NAME_IN_HEIGHTS * height, height);
        ui.child("visible", switch, Switch::new(Bind::kept()).label("Visible"));
        let select = band(&flow, row, start + (NAME_IN_HEIGHTS + 1.0) * height, SELECT_IN_HEIGHTS * height);
        ui.child("blend", select, Select::new("Blend mode", Bind::kept(), self.modes));
    }

    fn finish(self, cx: Cx<'_, Self>) -> LayerOut {
        LayerOut { response: cx.response.clone() }
    }
}

fn layers() -> Vec<Layer> {
    vec![Layer::new("Group 1").group(), Layer::new("Highlights").depth(1), Layer::new("Shadows").depth(1), Layer::new("Background")]
}

fn modes() -> [Entry<u8>; 3] {
    [Entry::new(0, "Normal"), Entry::new(1, "Multiply"), Entry::new(2, "Screen")]
}

fn stack(ui: &mut egui::Ui, data: &mut StoryData, layers: &[Layer]) -> StoryOutput {
    let open = data.value("open", vec![true; layers.len()]).clone();
    let source = Layers::new(layers, &open);
    let modes = modes();
    let out = LayerStack::new("Layers").show_items(ui, &source, &mut |key, _| LayerRow::new(&layers[*key], &modes));
    for event in &out.events {
        if let CollectionEvent::Open(change) = event {
            data.value("open", vec![true; layers.len()])[change.key] = change.open;
        }
    }
    out.into()
}

#[story(LayerStack, covers = [Empty], drive = Drive::Never)]
fn layer_stack_empty(ui: &mut egui::Ui, data: &mut StoryData) -> StoryOutput {
    stack(ui, data, &[])
}

#[story(LayerStack, covers = [One], drive = Drive::Never)]
fn layer_stack_one(ui: &mut egui::Ui, data: &mut StoryData) -> StoryOutput {
    stack(ui, data, &layers()[3..])
}

#[story(LayerStack, covers = [Default, Many], drive = Drive::Never)]
fn layer_stack_many(ui: &mut egui::Ui, data: &mut StoryData) -> StoryOutput {
    stack(ui, data, &layers())
}

#[story(LayerRow<'static>, covers = [Default], drive = Drive::Never)]
fn layer_row(ui: &mut egui::Ui, _: &mut StoryData) -> StoryOutput {
    let (layer, modes) = (Layer::new("Background"), modes());
    show_as_only_item(ui, LayerRow::new(&layer, &modes)).into()
}
