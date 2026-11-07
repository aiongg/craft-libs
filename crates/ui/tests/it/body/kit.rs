//! A container for the body trials: its props come from the trial, and its header and footer fill a block of
//! the size given.

use crate::contract::kit::PlainStyle;
use craft_ui::contract::{BodyCx, Common, Component, Container, ContainerProps, Cx, Role};
use craft_ui::style::StyleCx;
use craft_ui::testing::layout::allocate;
use egui::{Rect, Response, Ui, Vec2};
use std::cell::RefCell;

/// The rects the regions of a `Slab` took.
#[derive(Default)]
#[non_exhaustive]
pub struct Regions {
    pub header: Option<Rect>,
    pub footer: Option<Rect>,
}

#[derive(craft_ui::contract::Component)]
#[component(container)]
pub struct Slab<'a> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    header: Vec2,
    footer: Vec2,
    regions: Option<&'a RefCell<Regions>>,
}

impl<'a> Slab<'a> {
    pub fn new(props: ContainerProps) -> Self {
        Slab { common: Common::new(), props, header: Vec2::new(100.0, 24.0), footer: Vec2::new(100.0, 16.0), regions: None }
    }

    pub fn regions(mut self, regions: &'a RefCell<Regions>) -> Self {
        self.regions = Some(regions);
        self
    }
}

impl Component for Slab<'_> {
    const ROLE: Role = Role::GROUP;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: SlabStyleKey) -> PlainStyle {
        PlainStyle::default()
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

impl Container for Slab<'_> {
    fn header(&mut self, ui: &mut Ui, _: &mut BodyCx<'_>) {
        let rect = allocate(ui, self.header);
        if let Some(regions) = self.regions {
            regions.borrow_mut().header = Some(rect);
        }
    }

    fn footer(&mut self, ui: &mut Ui, _: &mut BodyCx<'_>) {
        let rect = allocate(ui, self.footer);
        if let Some(regions) = self.regions {
            regions.borrow_mut().footer = Some(rect);
        }
    }
}
