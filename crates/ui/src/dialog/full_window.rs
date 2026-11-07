//! `FullWindowDialog` (16.5): a dialog over the whole window, on the panel surface, with the dialog focus trap.

use super::frame::{self, CloseHandler, FOOTER, FooterCx, FooterFn};
use crate::contract::{Bind, BindMut, BodyCx, Common, Component, Container, ContainerProps, Cx, HeaderRegion, Mount, Pad, Props as _, Role, SaveKey};
use crate::runtime::env::Label;
use crate::runtime::focus::{EnterRule, FocusPolicy, Initial};
use crate::runtime::overlay::CloseReason;
use crate::sections::HeaderAction;
use crate::style::{ComponentStyle, StyleCx, SurfaceKind};
use craft_behavior::{Metrics, Sense};
use egui::{Response, Ui};

/// The style of a full-window dialog: the surface comes from the container.
#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct FullWindowDialogStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// A dialog that fills the window: a title row, a body that takes the room between the header and the footer, and an
/// optional footer. The areas the body shows inside it are panels at rects.
///
/// - Open state: bound with [`opened`](FullWindowDialog::opened), else a session slot that starts open.
/// - Focus: the dialog trap, with nothing focused at first unless [`initial_focus`](FullWindowDialog::initial_focus)
///   says otherwise.
#[derive(crate::contract::Component, crate::contract::Props)]
#[component(container)]
pub struct FullWindowDialog<'a> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    /// Binds the open state to `open`.
    #[prop(name = "opened")]
    open: Option<Bind<'a, bool>>,
    footer: Option<FooterFn<'a>>,
    actions: Vec<HeaderAction>,
    /// Reports how the dialog closed. Called once for each close.
    #[prop(into)]
    on_close: Option<CloseHandler<'a>>,
}

impl<'a> FullWindowDialog<'a> {
    /// A dialog titled `title`, with no title actions.
    pub fn new(title: impl Into<Label>) -> Self {
        let props = ContainerProps::new()
            .mount(Mount::FullWindow)
            .surface(SurfaceKind::Panel)
            .pad(Pad::None)
            .header(HeaderRegion::new().rule(true))
            .focus(FocusPolicy::DIALOG.initial(Initial::Keep));
        FullWindowDialog { common: Common::new(), props, open: None, footer: None, actions: Vec::new(), on_close: None }.label(title)
    }

    /// The action parts at the end of the title row, in reading order; the close action closes the dialog.
    #[must_use]
    pub fn actions(mut self, actions: impl IntoIterator<Item = HeaderAction>) -> Self {
        self.actions = actions.into_iter().collect();
        self
    }

    /// Who gets Enter: the dialog's submit, or the body.
    #[must_use]
    pub fn enter(mut self, rule: EnterRule) -> Self {
        self.props.focus = FocusPolicy { enter: rule, ..self.props.focus };
        self
    }

    /// What takes the keyboard focus when the dialog opens.
    #[must_use]
    pub fn initial_focus(mut self, initial: Initial) -> Self {
        self.props.focus = self.props.focus.initial(initial);
        self
    }

    /// Whether the dialog holds one outer gesture over its body's fields, from open to close.
    #[must_use]
    pub fn outer_gesture(mut self, on: bool) -> Self {
        self.props = self.props.outer_edit(on);
        self
    }

    /// Saves the open state under `key`.
    #[must_use]
    pub fn remember(mut self, key: SaveKey) -> Self {
        self.props = self.props.save(key);
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

impl Component for FullWindowDialog<'_> {
    const ROLE: Role = Role::DIALOG;
    type Style = FullWindowDialogStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: FullWindowDialogStyleKey) -> FullWindowDialogStyle {
        FullWindowDialogStyle::default()
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

impl Container for FullWindowDialog<'_> {
    fn open(&mut self) -> Option<BindMut<'_, bool>> {
        self.open.as_mut().map(Bind::reborrow)
    }

    fn header(&mut self, ui: &mut Ui, cx: &mut BodyCx<'_>) {
        frame::header(ui, cx, &self.actions);
    }

    fn footer(&mut self, ui: &mut Ui, cx: &mut BodyCx<'_>) {
        frame::footer(ui, cx, &mut self.footer);
    }

    fn closed(&mut self, why: CloseReason) {
        if let Some(report) = self.on_close.as_mut() {
            report.report(why);
        }
    }
}
