//! `Section` (16.5): an inline container whose header is a `TitleRow` with a disclosure bound to the container's
//! open state, and an optional trailing region for the app's controls. The body shows while the section is open.

use super::group_box::SECTION_LEVEL;
use super::title_row::{HeaderAction, HeaderStrength, TitleRow};
use crate::contract::{
    Bind, BindMut, BodyCx, Collapse, Common, Component, Container, ContainerProps, Cx, DragBand, HeaderRegion, NoValue, Outcome, Props as _, Role,
    SaveKey, Show as _, Sizing, WordSetCx as _,
};
use crate::runtime::env::Label;
use crate::style::{ComponentStyle, StyleCx};
use craft_behavior::{Metrics, Sense};
use egui::{Response, Ui};

#[cfg(feature = "testing")]
mod stories;

/// The style of a section: no surface or padding of its own; the header and body carry the look.
#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct SectionStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// What a section reports: its response and the header action taken this pass. The open state is the caller's
/// binding, or the container's own memory (saved across runs under `remember`).
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

/// The closure that fills a section's trailing header region.
type Controls<'a> = Box<dyn FnOnce(&mut Ui, &mut BodyCx<'_>) + 'a>;

/// An inline section of a panel that fills the width offered: a section-strength title row over a body, separated by
/// a rule. The open state is the container's own (it starts open), the caller's binding, or remembered across runs
/// under a key; a fixed section is always open and shows the title alone.
#[derive(crate::contract::Component, crate::contract::Props)]
#[component(container)]
pub struct Section<'a> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    /// Binds the open state to `open`.
    #[prop]
    open: Option<Bind<'a, bool>>,
    collapsible: bool,
    /// The header's weight: `Section` by default, `Caption` for a quiet heading.
    #[prop]
    strength: HeaderStrength,
    actions: Vec<HeaderAction>,
    controls: Option<Controls<'a>>,
    taken: Option<HeaderAction>,
}

impl<'a> Section<'a> {
    /// A collapsible section titled `title`. It starts open.
    pub fn new(title: impl Into<Label>) -> Self {
        Section::build(title, true, ContainerProps::new().collapse(Collapse::new()).sizing(Sizing::Fill))
    }

    /// A section that is always open: the title alone, with no disclosure.
    pub fn fixed(title: impl Into<Label>) -> Self {
        Section::build(title, false, ContainerProps::new().sizing(Sizing::Fill))
    }

    fn build(title: impl Into<Label>, collapsible: bool, props: ContainerProps) -> Self {
        let props = props.header(HeaderRegion::new().rule(true));
        Section {
            common: Common::new(),
            props,
            open: None,
            collapsible,
            strength: HeaderStrength::Section,
            actions: Vec::new(),
            controls: None,
            taken: None,
        }
        .label(title)
    }

    /// Whether the section starts open (the default). The state lives in the container unless `open` binds it.
    #[must_use]
    pub fn initially_open(mut self, on: bool) -> Self {
        if let Some(collapse) = self.props.collapse {
            self.props = self.props.collapse(collapse.initially_open(on));
        }
        self
    }

    /// Keeps the open state across runs in the slot `key`.
    #[must_use]
    pub fn remember(mut self, key: SaveKey) -> Self {
        self.props = self.props.save(key);
        self
    }

    /// Whether dragging the header reorders the section among its siblings.
    #[must_use]
    pub fn reorder(mut self, on: bool) -> Self {
        let band = if on { DragBand::Reorder } else { DragBand::None };
        self.update_header(|header| header.band(band));
        self
    }

    /// Fills the trailing region of the header with the app's controls (a Segmented, a Select, an icon button, count
    /// text). They sit at the end edge, after the title row and its fixed actions, and take their own clicks.
    #[must_use]
    pub fn controls(mut self, controls: impl FnOnce(&mut Ui, &mut BodyCx<'_>) + 'a) -> Self {
        self.update_header(|header| header.trailing(true));
        self.controls = Some(Box::new(controls));
        self
    }

    fn update_header(&mut self, change: impl FnOnce(HeaderRegion) -> HeaderRegion) {
        let header = change(self.props.header.unwrap_or_default());
        self.props.header = Some(header);
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
    const PARTS: bool = false;
    const STOPS: bool = false;
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

    fn controls(&mut self, ui: &mut Ui, cx: &mut BodyCx<'_>) {
        if let Some(controls) = self.controls.take() {
            controls(ui, cx);
        }
    }

    fn header(&mut self, ui: &mut Ui, cx: &mut BodyCx<'_>) {
        let row = TitleRow::new(cx.label()).strength(self.strength).level(SECTION_LEVEL).actions(self.actions.iter().copied());
        let row = if self.collapsible { row.disclosure(Bind::bound(cx.open())) } else { row };
        if let Some(action) = row.show(ui).action {
            self.taken = Some(action);
        }
    }
}
