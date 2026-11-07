//! `Section` (16.5): an inline container whose header is a `TitleRow` with a disclosure bound to the container's
//! open state. The body shows while the section is open.

use super::group_box::SECTION_LEVEL;
use super::title_row::{HeaderAction, HeaderStrength, TitleRow};
use crate::contract::{
    Bind, BindMut, BodyCx, Collapse, Common, Component, Container, ContainerProps, Cx, DragBand, HeaderRegion, NoValue, Outcome, Props as _, Role,
    SaveKey, Show as _, WordSetCx as _,
};
use crate::runtime::env::Label;
use crate::style::{ComponentStyle, StyleCx};
use craft_behavior::{Metrics, Sense};
use egui::{Response, Ui};

/// The style of a section: no surface or padding of its own; the header and body carry the look.
#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct SectionStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// What a section reports: its response and the header action taken this pass. The open state is the caller's
/// binding, or the saved slot.
#[derive(Debug)]
#[non_exhaustive]
pub struct SectionOutput {
    /// The section's response.
    pub response: Response,
    /// The header action part taken this pass.
    pub action: Option<HeaderAction>,
}

impl Outcome for SectionOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// An inline section of a panel: a section-strength title row over a body, separated by a rule. The open state is
/// the caller's binding, or a slot saved under a key, or fixed open.
#[derive(crate::contract::Component)]
#[component(container)]
pub struct Section<'a> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    open: Option<Bind<'a, bool>>,
    collapsible: bool,
    actions: Vec<HeaderAction>,
    taken: Option<HeaderAction>,
}

impl<'a> Section<'a> {
    /// A section titled `title` whose open state is `open`.
    pub fn new(title: impl Into<Label>, open: Bind<'a, bool>) -> Self {
        Section::build(title, Some(open), true, ContainerProps::new().collapse(Collapse::new()))
    }

    /// A section that keeps its open state in the slot `key`, saved across runs. It starts open.
    pub fn saved(title: impl Into<Label>, key: SaveKey) -> Self {
        Section::build(title, None, true, ContainerProps::new().collapse(Collapse::new()).save(key))
    }

    /// A section that is always open: the title alone, with no disclosure.
    pub fn fixed(title: impl Into<Label>) -> Self {
        Section::build(title, None, false, ContainerProps::new())
    }

    fn build(title: impl Into<Label>, open: Option<Bind<'a, bool>>, collapsible: bool, props: ContainerProps) -> Self {
        let props = props.header(HeaderRegion::new().rule(true));
        Section { common: Common::new(), props, open, collapsible, actions: Vec::new(), taken: None }.label(title)
    }

    /// Whether dragging the header reorders the section among its siblings.
    #[must_use]
    pub fn reorder(mut self, on: bool) -> Self {
        let band = if on { DragBand::Reorder } else { DragBand::None };
        self.props = self.props.header(HeaderRegion::new().rule(true).band(band));
        self
    }

    /// The action parts at the end of the header, in reading order.
    #[must_use]
    pub fn actions(mut self, actions: impl IntoIterator<Item = HeaderAction>) -> Self {
        self.actions = actions.into_iter().collect();
        self
    }
}

impl Component for Section<'_> {
    const ROLE: Role = Role::GROUP;
    type Style = SectionStyle;
    type Output = SectionOutput;

    fn style(_: &StyleCx<'_>, _: SectionStyleKey) -> SectionStyle {
        SectionStyle::default()
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn finish(self, cx: Cx<'_, Self>) -> SectionOutput {
        SectionOutput { response: cx.response.clone(), action: self.taken }
    }
}

impl Container for Section<'_> {
    fn open(&mut self) -> Option<BindMut<'_, bool>> {
        self.open.as_mut().map(Bind::reborrow)
    }

    fn header(&mut self, ui: &mut Ui, cx: &mut BodyCx<'_>) {
        let row = TitleRow::new(cx.label()).strength(HeaderStrength::Section).level(SECTION_LEVEL).actions(self.actions.iter().copied());
        let row = if self.collapsible { row.disclosure(Bind::bound(cx.open())) } else { row };
        if let Some(action) = row.show(ui).action {
            self.taken = Some(action);
        }
    }
}
