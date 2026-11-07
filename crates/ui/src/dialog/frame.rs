//! `DialogFrame` (16.5) and the chrome every dialog shell shares: the title row with its close action, the footer
//! region and the footer closure's context.

use super::host::DialogResult;
use crate::contract::{
    BodyCx, Common, Component, Container, ContainerProps, Cx, FooterRegion, HeaderRegion, Pad, Props as _, Role, Scroll, Show as _,
    ShowContainer as _, WordSetCx as _,
};
use crate::contract::{NoValue, Outcome};
use crate::runtime::env::Label;
use crate::runtime::overlay::CloseReason;
use crate::sections::{DialogRole, FooterBar, FooterButton, HeaderAction, HeaderStrength, TitleRow};
use crate::style::{ComponentStyle, StyleCx, SurfaceKind};
use craft_behavior::{Metrics, Sense};
use egui::{Response, Ui};

/// What a dialog shell returns: its response, how it closed in this pass, and the footer button that was picked.
#[derive(Debug)]
#[non_exhaustive]
pub struct DialogOutput {
    /// The dialog's response.
    pub response: Response,
    /// How the dialog closed in this pass: `Escape` is the cancel, `Enter` the OK of `close_on_enter`, `Activated` and
    /// `Requested` what a footer button asked for. `None` while it stays open. Reported once for each close.
    pub closed: Option<CloseReason>,
    /// The index, in the order given to `buttons`, of the footer button clicked in this pass.
    pub picked: Option<usize>,
}

impl Outcome for DialogOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

impl DialogOutput {
    /// The dialog's answer for a [`DialogHost`](super::DialogHost) entry: `Ok(answer())` when it closed as an OK
    /// (Enter or an accepting button), `Cancel` when it closed any other way, `None` while it is open.
    pub fn result<T>(&self, answer: impl FnOnce() -> T) -> Option<DialogResult<T>> {
        match self.closed? {
            CloseReason::Enter | CloseReason::Activated => Some(DialogResult::Ok(answer())),
            _ => Some(DialogResult::Cancel),
        }
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

/// Fills the footer region: the caller's content closure in a footer bar, and under it the dialog buttons, whose
/// roles decide the close reason (`Accept` and the other outcomes close as `Activated`, `Cancel` as `Requested`).
pub(super) fn footer(ui: &mut Ui, cx: &mut BodyCx<'_>, content: &mut Option<FooterFn<'_>>, buttons: &[FooterButton], picked: &mut Option<usize>) {
    let mut close = None;
    if let Some(content) = content.as_mut() {
        FooterBar::body().show(ui, |ui, bar| content(ui, &mut FooterCx { bar, close: &mut close }));
    }
    if !buttons.is_empty() {
        *picked = FooterBar::buttons(buttons.iter().cloned()).show_content(ui);
        if let Some(at) = *picked {
            close.get_or_insert(if buttons[at].role() == DialogRole::Cancel { CloseReason::Requested } else { CloseReason::Activated });
        }
    }
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
    const PARTS: bool = false;
    const STOPS: bool = false;
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
        footer(ui, cx, &mut self.footer, &[], &mut None);
    }
}
