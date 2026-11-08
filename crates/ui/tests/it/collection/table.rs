//! Collections with columns, reordering and free layouts.

use super::kit::{Dismissed, ROW_H, Row};
use crate::contract::kit::PlainStyle;
use craft_behavior::{FreeLayout, ItemSource, ScrollPos, Selection, SpanLayout};
use craft_ui::contract::{Bind, BindMut, Collection, CollectionProps, CollectionResponse, Common, Component, ContainerProps, Cx, Role, SelectMode};
use craft_ui::style::StyleCx;
use egui::Rect;

/// A list of rows at `ROW_H` with the collection props the trial gives.
#[derive(craft_ui::contract::Component)]
#[component(collection)]
pub struct Table {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    items: CollectionProps,
}

impl Table {
    pub fn new(items: CollectionProps) -> Self {
        Table { common: Common::new(), props: ContainerProps::new().sizing(craft_ui::contract::Sizing::Fill), items }
    }
}

impl Component for Table {
    const ROLE: Role = Role::LIST;
    type Style = PlainStyle;
    type Output = CollectionResponse<usize, Dismissed>;

    fn style(_: &StyleCx<'_>, _: TableStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn finish(self, mut cx: Cx<'_, Self>) -> Self::Output {
        cx.collection_response()
    }
}

impl Collection for Table {
    type Key = usize;
    type ItemLayout = SpanLayout;
    type Selection = Selection<usize>;
    type Item<'r> = Row;
    type ItemEvent = Dismissed;

    fn items(&self) -> &CollectionProps {
        &self.items
    }

    fn item_layout(&self, source: &dyn ItemSource<usize>, width: f32, _: &StyleCx<'_>) -> SpanLayout {
        SpanLayout::new(source, width, ROW_H)
    }
}

/// Items at the rects the trial gives.
#[derive(craft_ui::contract::Component)]
#[component(collection)]
pub struct Free<'a> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    items: CollectionProps,
    rects: Vec<Rect>,
    scroll: Option<Bind<'a, ScrollPos>>,
}

impl<'a> Free<'a> {
    pub fn new(rects: Vec<Rect>) -> Self {
        Free {
            common: Common::new(),
            props: ContainerProps::new().sizing(craft_ui::contract::Sizing::Fill),
            items: CollectionProps::new().select(SelectMode::Multi),
            rects,
            scroll: None,
        }
    }

    /// The scroll position the app owns.
    pub fn scroll(mut self, scroll: Bind<'a, ScrollPos>) -> Self {
        self.scroll = Some(scroll);
        self
    }
}

impl Component for Free<'_> {
    const ROLE: Role = Role::LIST;
    type Style = PlainStyle;
    type Output = CollectionResponse<usize, Dismissed>;

    fn style(_: &StyleCx<'_>, _: FreeStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn scroll(&mut self) -> Option<BindMut<'_, ScrollPos>> {
        self.scroll.as_mut().map(Bind::reborrow)
    }

    fn finish(self, mut cx: Cx<'_, Self>) -> Self::Output {
        cx.collection_response()
    }
}

impl Collection for Free<'_> {
    type Key = usize;
    type ItemLayout = FreeLayout;
    type Selection = Selection<usize>;
    type Item<'r> = Row;
    type ItemEvent = Dismissed;

    fn items(&self) -> &CollectionProps {
        &self.items
    }

    fn item_layout(&self, _: &dyn ItemSource<usize>, _: f32, _: &StyleCx<'_>) -> FreeLayout {
        FreeLayout::new(self.rects.clone())
    }
}
