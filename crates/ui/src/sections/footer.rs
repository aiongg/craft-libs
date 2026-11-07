//! `FooterBar` (16.4): the container that fills a container's footer region.

use crate::contract::{Common, Component, Container, ContainerProps, Cx, Role};
use crate::style::{ComponentStyle, StyleCx};
use craft_behavior::{Metrics, NavShape, Roving, Sense};
use egui::Response;

/// The style of a footer bar: no look of its own.
#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct FooterBarStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// A footer region's content: its body is the caller's closure, and its members are one roving group.
// F8-LATER: `FooterBar::actions(..)` and `FooterBar::buttons(..)` need `IconButton` and `Button`.
// F5-MERGE: F5c. The row layout with end alignment comes from `ContainerProps.layout` once the body step has it.
#[derive(crate::contract::Component)]
#[component(container)]
pub struct FooterBar {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
}

impl FooterBar {
    /// A footer bar whose content is its body.
    pub fn body() -> Self {
        FooterBar { common: Common::new(), props: ContainerProps::new().roving(Roving::new(NavShape::Flat)) }
    }
}

impl Component for FooterBar {
    const ROLE: Role = Role::GROUP;
    type Style = FooterBarStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: FooterBarStyleKey) -> FooterBarStyle {
        FooterBarStyle::default()
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

impl Container for FooterBar {}
