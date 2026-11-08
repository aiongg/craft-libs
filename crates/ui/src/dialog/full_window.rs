//! `FullWindowDialog` (16.5): a dialog over the whole window, on the panel surface, with the dialog focus trap.

use super::frame::{self, DialogOutput, FOOTER, FooterCx, FooterFn};
use crate::contract::{Bind, BindMut, BodyCx, Common, Component, Container, ContainerProps, Cx, HeaderRegion, Mount, Pad, Props as _, Role, SaveKey};
use crate::runtime::env::Label;
use crate::runtime::focus::{EnterRule, FocusPolicy, Initial};
use crate::runtime::overlay::CloseReason;
use crate::sections::FooterButton;
use crate::style::{ComponentStyle, StyleCx, SurfaceKind};
use craft_behavior::{Metrics, Sense};
use egui::Ui;

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
    /// Lets the host act on a picked button and refuse its close: `allows` gets every picked button but `Cancel`
    /// and the dialog's body context, so a host reads and writes its kept state, and returns whether a button that
    /// closes the dialog may. On `false` the dialog stays open, with `DialogOutput::picked` still
    /// reporting the pick; the host shows why. A button marked `stays_open` never closes, whatever it returns.
    #[prop(boxed, name = "close_guard")]
    guard: Option<Box<dyn FnMut(&FooterButton, &mut BodyCx<'_>) -> bool + 'a>>,
    /// Sets the buttons' state each pass; see `Dialog::settle_buttons`.
    #[prop(boxed, name = "settle_buttons")]
    settle: Option<Box<dyn FnMut(&mut BodyCx<'_>, &mut [FooterButton]) + 'a>>,
    buttons: Vec<FooterButton>,
    /// Whether the title row has a close action.
    #[prop]
    closable: bool,
    closed: Option<CloseReason>,
    picked: Option<usize>,
}

impl<'a> FullWindowDialog<'a> {
    /// A dialog titled `title`, with no close action.
    pub fn new(title: impl Into<Label>) -> Self {
        let props = ContainerProps::new()
            .mount(Mount::FullWindow)
            .surface(SurfaceKind::Panel)
            .pad(Pad::None)
            .header(HeaderRegion::new().rule(true))
            .focus(FocusPolicy::DIALOG.initial(Initial::Keep));
        FullWindowDialog {
            common: Common::new(),
            props,
            open: None,
            footer: None,
            guard: None,
            settle: None,
            buttons: Vec::new(),
            closable: false,
            closed: None,
            picked: None,
        }
        .label(title)
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

    /// The dialog buttons in the footer, in the platform's order for their roles. A click closes the dialog: `Cancel`
    /// as `CloseReason::Requested`, every other role as `CloseReason::Activated` unless the button `stays_open`;
    /// `DialogOutput::picked` says which.
    #[must_use]
    pub fn buttons(mut self, buttons: impl IntoIterator<Item = FooterButton>) -> Self {
        self.props = self.props.footer(FOOTER);
        self.buttons = buttons.into_iter().collect();
        self
    }

    /// The footer's content: the closure fills a footer bar above the buttons, or alone.
    #[must_use]
    pub fn footer(mut self, content: impl FnMut(&mut Ui, &mut FooterCx<'_, '_>) + 'a) -> Self {
        self.props = self.props.footer(FOOTER);
        self.footer = Some(Box::new(content));
        self
    }
}

impl Component for FullWindowDialog<'_> {
    const ROLE: Role = Role::DIALOG;
    const PARTS: bool = false;
    const STOPS: bool = false;
    type Style = FullWindowDialogStyle;
    type Output = DialogOutput;

    fn style(_: &StyleCx<'_>, _: FullWindowDialogStyleKey) -> FullWindowDialogStyle {
        FullWindowDialogStyle::default()
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn finish(self, cx: Cx<'_, Self>) -> DialogOutput {
        DialogOutput { response: cx.response.clone(), closed: self.closed, picked: self.picked }
    }
}

impl Container for FullWindowDialog<'_> {
    fn open(&mut self) -> Option<BindMut<'_, bool>> {
        self.open.as_mut().map(Bind::reborrow)
    }

    fn header(&mut self, ui: &mut Ui, cx: &mut BodyCx<'_>) {
        frame::header(ui, cx, frame::close_action(self.closable));
    }

    fn footer(&mut self, ui: &mut Ui, cx: &mut BodyCx<'_>) {
        frame::footer(
            ui,
            cx,
            &mut self.footer,
            &self.buttons,
            false,
            frame::FooterHost { picked: &mut self.picked, guard: &mut self.guard, on_pick: &mut None, settle: &mut self.settle },
        );
    }

    fn closed(&mut self, why: CloseReason) {
        self.closed.get_or_insert(why);
    }
}
