//! Collections with columns, reordering and free layouts.

use super::kit::{Dismissed, ROW_H, Row};
use crate::contract::kit::PlainStyle;
use craft_behavior::{FreeLayout, ItemSource, Selection, SpanLayout};
use craft_ui::contract::{Collection, CollectionProps, CollectionResponse, Common, Component, ContainerProps, Cx, Role, SelectMode};
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
        Table { common: Common::new(), props: ContainerProps::new(), items }
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
pub struct Free {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    items: CollectionProps,
    rects: Vec<Rect>,
}

impl Free {
    pub fn new(rects: Vec<Rect>) -> Self {
        Free { common: Common::new(), props: ContainerProps::new(), items: CollectionProps::new().select(SelectMode::Multi), rects }
    }
}

impl Component for Free {
    const ROLE: Role = Role::LIST;
    type Style = PlainStyle;
    type Output = CollectionResponse<usize, Dismissed>;

    fn style(_: &StyleCx<'_>, _: FreeStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn finish(self, mut cx: Cx<'_, Self>) -> Self::Output {
        cx.collection_response()
    }
}

impl Collection for Free {
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
