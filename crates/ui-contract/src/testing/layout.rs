//! The layout services the trials check: placeholder children that take a given size, and regions and allocation
//! in a `Ui`.

use crate::contract::{Common, Component, Cx, MeasureCx, Response, Role, Ui, UiBuilder};
use crate::style::{ComponentStyle, StyleCx};
use craft_behavior::{Metrics, Sense};
use craft_ui_rt::{Rect, Vec2};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A placeholder child of a fixed size: a leaf that measures `size`, senses nothing and paints nothing.
#[derive(Component)]
pub struct Block {
    #[common]
    common: Common,
    size: Vec2,
    #[key(skip)]
    measures: Option<Arc<AtomicUsize>>,
}

impl Block {
    /// A child that takes `size`.
    pub const fn new(size: Vec2) -> Self {
        Block { common: Common::new(), size, measures: None }
    }

    /// Counts each call of its measure in `measures`.
    pub fn counted(mut self, measures: &Arc<AtomicUsize>) -> Self {
        self.measures = Some(measures.clone());
        self
    }
}

/// The style of a `Block`: no padding.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
pub struct BlockStyle {
    #[style(metrics)]
    metrics: Metrics,
}

impl Component for Block {
    const ROLE: Role = Role::None;
    const PARTS: bool = false;
    const STOPS: bool = false;
    type Style = BlockStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: BlockStyleKey) -> BlockStyle {
        BlockStyle { metrics: Metrics::new() }
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        if let Some(measures) = &self.measures {
            measures.fetch_add(1, Ordering::Relaxed);
        }
        self.size
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

/// Runs `body` in a child `Ui` whose region is `rect`.
pub fn in_region<R>(ui: &mut Ui, rect: Rect, body: impl FnOnce(&mut Ui) -> R) -> R {
    ui.scope_builder(UiBuilder::new().max_rect(rect), body)
}

/// Takes `size` in the layout of `ui` and returns its rect.
pub fn allocate(ui: &mut Ui, size: Vec2) -> Rect {
    ui.allocate(size)
}
