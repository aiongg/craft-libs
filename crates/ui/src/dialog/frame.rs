//! `DialogFrame` (16.5) and the chrome every dialog shell shares: the title row with its close action, the footer
//! region and the footer closure's context.

use crate::contract::{
    BodyCx, Common, Component, Container, ContainerProps, Cx, FooterRegion, HeaderRegion, Pad, Props as _, Role, Scroll, Show as _,
    ShowContainer as _, WordSetCx as _,
};
use crate::runtime::env::Label;
use crate::runtime::overlay::CloseReason;
use crate::sections::{FooterBar, HeaderAction, HeaderStrength, TitleRow};
use crate::style::{ComponentStyle, StyleCx, SurfaceKind};
use craft_behavior::{Metrics, Sense};
use egui::{Response, Ui};

/// What a dialog calls when it closes: a closure over the reason.
pub struct CloseHandler<'a>(Box<dyn FnMut(CloseReason) + 'a>);

impl<'a, F: FnMut(CloseReason) + 'a> From<F> for CloseHandler<'a> {
    fn from(report: F) -> Self {
        CloseHandler(Box::new(report))
    }
}

impl CloseHandler<'_> {
    pub(super) fn report(&mut self, why: CloseReason) {
        (self.0)(why);
    }
}

/// The heading level AccessKit reports for a dialog's title.
const TITLE_LEVEL: usize = 1;

/// What a footer closure reaches: the footer bar's body context, and a request to close the dialog.
pub struct FooterCx<'a, 'b> {
    bar: &'a mut BodyCx<'b>,
    close: &'a mut Option<CloseReason>,
}

impl<'b> FooterCx<'_, 'b> {
    /// The footer bar's body context.
    pub fn bar(&mut self) -> &mut BodyCx<'b> {
        self.bar
    }

    /// Asks the dialog to close, as `CloseReason::Requested`: step 15 closes it.
    pub fn request_close(&mut self) {
        self.request_close_as(CloseReason::Requested);
    }

    /// Asks the dialog to close and tells it why: `Activated` for the OK button, `Requested` for Cancel. The first
    /// request of a pass keeps its reason.
    pub fn request_close_as(&mut self, why: CloseReason) {
        self.close.get_or_insert(why);
    }
}

/// The footer closure of a dialog shell.
pub(super) type FooterFn<'a> = Box<dyn FnMut(&mut Ui, &mut FooterCx<'_, '_>) + 'a>;

/// Fills the header region: the title and, for a closable shell, the close action.
pub(super) fn header(ui: &mut Ui, cx: &mut BodyCx<'_>, actions: &[HeaderAction]) {
    let shown = TitleRow::new(cx.label()).strength(HeaderStrength::Strong).level(TITLE_LEVEL).actions(actions.iter().copied()).show(ui);
    if shown.action == Some(HeaderAction::Close) {
        cx.request_close();
    }
}

/// Fills the footer region with the caller's closure inside a footer bar.
pub(super) fn footer(ui: &mut Ui, cx: &mut BodyCx<'_>, content: &mut Option<FooterFn<'_>>) {
    let Some(content) = content.as_mut() else { return };
    let mut close = None;
    FooterBar::body().show(ui, |ui, bar| content(ui, &mut FooterCx { bar, close: &mut close }));
    if let Some(why) = close {
        cx.request_close_as(why);
    }
}

/// The header actions of a shell: the close action when it has one.
pub(super) fn close_action(closable: bool) -> &'static [HeaderAction] {
    if closable { &[HeaderAction::Close] } else { &[] }
}

/// The props every dialog shell starts from: the dialog surface, a header with a rule under it and a vertical scroll.
pub(super) fn chrome_props() -> ContainerProps {
    ContainerProps::new().surface(SurfaceKind::Dialog).pad(Pad::Regular).header(HeaderRegion::new().rule(true)).scroll(Scroll::vertical())
}

/// The footer region of a shell that has footer content.
pub(super) const FOOTER: FooterRegion = FooterRegion::new().rule(true);

/// The style of a dialog frame: the surface and padding come from the container.
#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct DialogFrameStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// The look of a dialog in the flow of its parent: the dialog surface, a title row, the body and a footer. It has no
/// layer, no focus scope and no placement; `Dialog` is the same chrome on an egui layer.
#[derive(crate::contract::Component, crate::contract::Props)]
#[component(container)]
pub struct DialogFrame<'a> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    footer: Option<FooterFn<'a>>,
    /// Whether the title row has a close action; the container closes itself when it is taken.
    #[prop]
    closable: bool,
}

impl<'a> DialogFrame<'a> {
    /// A frame titled `title`.
    pub fn new(title: impl Into<Label>) -> Self {
        DialogFrame { common: Common::new(), props: chrome_props(), footer: None, closable: false }.label(title)
    }

    /// The larger frame margin.
    #[must_use]
    pub fn roomy(mut self, on: bool) -> Self {
        self.props = self.props.pad(if on { Pad::Roomy } else { Pad::Regular });
        self
    }

    /// The footer's content: the closure fills a footer bar under the body.
    #[must_use]
    pub fn footer(mut self, content: impl FnMut(&mut Ui, &mut FooterCx<'_, '_>) + 'a) -> Self {
        self.props = self.props.footer(FOOTER);
        self.footer = Some(Box::new(content));
        self
    }
}

impl Component for DialogFrame<'_> {
    const ROLE: Role = Role::GROUP;
    type Style = DialogFrameStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: DialogFrameStyleKey) -> DialogFrameStyle {
        DialogFrameStyle::default()
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

impl Container for DialogFrame<'_> {
    fn header(&mut self, ui: &mut Ui, cx: &mut BodyCx<'_>) {
        header(ui, cx, close_action(self.closable));
    }

    fn footer(&mut self, ui: &mut Ui, cx: &mut BodyCx<'_>) {
        footer(ui, cx, &mut self.footer);
    }
}
