//! `BarCell`: a control in a bar with its caption before it.

use crate::contract::{Common, Component, Container, ContainerProps, Props as _, Role, SideLabel, Sizing};
use crate::runtime::access::Access;
use crate::runtime::env::Label;
use crate::style::{ComponentStyle, StyleCx};
use craft_behavior::{Metrics, Sense};
use egui::Response;

/// The style of a bar cell: no look of its own; the caption carries it.
#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct BarCellStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// One control with its caption before it, as wide as the two together. The caption is the label step's side label,
/// as in `Field`: it names the control, focuses it when clicked, and shares its line with it. The control is the
/// body's one child and returns its own output.
#[derive(crate::contract::Component)]
#[component(container)]
pub struct BarCell {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
}

impl BarCell {
    /// A cell whose control is named and captioned `caption`.
    pub fn new(caption: impl Into<Label>) -> Self {
        let props = ContainerProps::new().sizing(Sizing::Content).label(SideLabel::new());
        BarCell { common: Common::new(), props }.label(caption)
    }
}

impl Component for BarCell {
    const ROLE: Role = Role::GROUP;
    const PARTS: bool = false;
    const STOPS: bool = false;
    type Style = BarCellStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: BarCellStyleKey) -> BarCellStyle {
        BarCellStyle::default()
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn access(&self, _: &mut Access, _: &crate::contract::Cx<'_, Self>) {}

    fn finish(self, cx: crate::contract::Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

impl Container for BarCell {}
