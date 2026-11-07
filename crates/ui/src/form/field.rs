//! `Field` (16.3): a container that shows one control with a side label, and a help line under it.

use crate::contract::{
    BodyCx, Common, Component, Container, ContainerProps, Cx, FieldLayout, FooterRegion, MeasureCx, PaintUi, Props as _, Role, Show as _, SideLabel,
    Sizing, Styled as _,
};
use crate::primitives::{TextRun, Wrap};
use crate::runtime::env::Label;
use crate::style::{ComponentStyle, StyleCx, TextLook, TypeRole};
use craft_behavior::{Metrics, Sense};
use craft_theme::{Stateful, States, variant::Tone};
use egui::{Response, Ui, Vec2};

/// The text under a field's control.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum Help<'a> {
    /// Muted text: a hint or a unit.
    Text(&'a str),
    /// A line in the colour of a tone.
    Status(Tone, &'a str),
}

/// The style of a field: no look of its own; the label step and the help line carry it.
#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct FieldRowStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// One control with a side label: beside it, above it, as a caption or after it (`Field::layout`). The label step
/// names the control by the label, runs the label's scrub and reset, and focuses the control when the label is
/// clicked. The control is the body's one child and returns its own output. `Field::help` adds a help line under
/// the control.
#[derive(crate::contract::Component)]
#[component(container)]
pub struct Field<'a> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    help: Option<Help<'a>>,
}

impl<'a> Field<'a> {
    /// A field labelled `label`, beside its control, filling the width offered.
    pub fn new(label: impl Into<Label>) -> Self {
        let props = ContainerProps::new().sizing(Sizing::Fill).label(SideLabel::new());
        Field { common: Common::new(), props, help: None }.label(label)
    }

    /// Where the label sits relative to the control.
    #[must_use]
    pub fn layout(mut self, layout: FieldLayout) -> Self {
        self.props.label = self.props.label.take().map(|side| side.layout(layout));
        self
    }

    /// How the label is shown and what it does: its icon, detail, scrub, whether it is visible and whether it names
    /// or describes the control. The text stays the field's label.
    #[must_use]
    pub fn side(mut self, side: SideLabel) -> Self {
        self.props.label = Some(side);
        self
    }

    /// Shows `help` under the control.
    #[must_use]
    pub fn help(mut self, help: Help<'a>) -> Self {
        self.props.footer = Some(FooterRegion::new());
        self.help = Some(help);
        self
    }
}

impl Component for Field<'_> {
    const ROLE: Role = Role::GROUP;
    type Style = FieldRowStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: FieldStyleKey) -> FieldRowStyle {
        FieldRowStyle::default()
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

impl Container for Field<'_> {
    fn footer(&mut self, ui: &mut Ui, _: &mut BodyCx<'_>) {
        match self.help {
            Some(Help::Text(text)) => HelpLine::new(text, Tone::Neutral).show(ui),
            Some(Help::Status(tone, text)) => HelpLine::new(text, tone).show(ui),
            None => return,
        };
    }
}

/// A line of caption text that wraps at the room's width, muted for the neutral tone and in the tone's colour
/// otherwise.
#[derive(crate::contract::Component)]
pub(super) struct HelpLine {
    #[common]
    common: Common,
    #[style_key]
    tone: Tone,
}

/// The style of a help line.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub(super) struct HelpLineStyle {
    #[style(metrics)]
    metrics: Metrics,
    /// The line's text.
    #[style(text)]
    text: TextLook,
}

impl HelpLine {
    /// `text`, which is also its accessible name, in `tone`.
    pub(super) fn new(text: impl Into<Label>, tone: Tone) -> Self {
        HelpLine { common: Common::new(), tone }.label(text)
    }
}

impl Component for HelpLine {
    const ROLE: Role = Role::LABEL;
    const INPUT: craft_behavior::Input = craft_behavior::Input::Display;
    const OVERLAYS: bool = false;
    type Style = HelpLineStyle;
    type Output = Response;

    fn style(cx: &StyleCx<'_>, key: HelpLineStyleKey) -> HelpLineStyle {
        let colors = &cx.tokens.color.text;
        let color = match key.tone {
            Tone::Neutral => cx.color(colors.muted),
            tone => cx.tone(tone),
        };
        let off = States::new(color, color, color, cx.color(colors.disabled));
        HelpLineStyle { metrics: Metrics::new(), text: TextLook::new(TypeRole::Caption, Stateful::new(color).off(off)) }
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        cx.layout(&TextRun::new(cx.resolved.label).role(cx.style.text.role).wrap(Wrap::Wrap)).size()
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let run = TextRun::new(cx.resolved.label).role(cx.style.text.role).wrap(Wrap::Wrap).color(cx.resolve(&cx.style.text.color));
        ui.draw().text(cx.content_rect, &run);
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}
