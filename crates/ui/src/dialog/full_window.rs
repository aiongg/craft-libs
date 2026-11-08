//! `FullWindowDialog` (16.5): a dialog over the whole window, on the panel surface, with the dialog focus trap.

use super::frame::{self, DialogOutput, FOOTER, FooterCx, FooterFn, FooterHooks};
use super::full_window_parts::SideFooter;
use crate::canvas::ViewCommands;
use crate::contract::{
    Bind, BindMut, BodyCx, Common, Component, Container, ContainerProps, Cx, EndRegion, FooterPlace, FooterRegion, HeaderRegion, Mount, Pad,
    Props as _, Role, SaveKey,
};
use crate::runtime::commands::CommandSet as _;
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
/// - Side columns: [`sides`](FullWindowDialog::sides) fills a region at the end edge, as high as the dialog under the
///   title row and as wide as its content; the body (the canvas) takes the rest of the width. With side columns the
///   footer sits among them, at their bottom unless [`side_footer`](FullWindowDialog::side_footer) says otherwise.
/// - Buttons: the dialog's own, wherever the footer sits: Cancel and Escape close it as `CloseReason::Requested`,
///   Enter activates the first enabled `Accept` button while the dialog's Enter rule is `EnterRule::Submit`, and
///   `close_guard` and `settle_buttons` see every pick.
/// - Open state: bound with [`opened`](FullWindowDialog::opened), else a session slot that starts open.
/// - View commands: the dialog is an instance of the [`ViewCommands`] scope, so the zoom chords act on the view the
///   dialog's zoom bar is bound to, not on a canvas behind it.
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
    /// Acts on a picked footer button in the footer step, before the close is decided: `act` gets the button and the
    /// dialog's body context. `ResetFooter::install` sets it.
    #[prop(boxed, name = "on_pick")]
    on_pick: Option<Box<dyn FnMut(&FooterButton, &mut BodyCx<'_>) + 'a>>,
    buttons: Vec<FooterButton>,
    sides: Option<SidesFn<'a>>,
    side_footer: SideFooter<'a>,
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
            .focus(FocusPolicy::DIALOG.initial(Initial::Keep))
            .command_scope(ViewCommands::SCOPE);
        FullWindowDialog {
            common: Common::new(),
            props,
            open: None,
            footer: None,
            guard: None,
            settle: None,
            on_pick: None,
            buttons: Vec::new(),
            sides: None,
            side_footer: SideFooter::bottom(),
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
        let region = self.footer_region();
        self.props = self.props.footer(region);
        self.buttons = buttons.into_iter().collect();
        self
    }

    /// The footer's content: the closure fills a footer bar above the buttons, or alone.
    #[must_use]
    pub fn footer(mut self, content: impl FnMut(&mut Ui, &mut FooterCx<'_, '_>) + 'a) -> Self {
        let region = self.footer_region();
        self.props = self.props.footer(region);
        self.footer = Some(Box::new(content));
        self
    }

    /// The side columns: `content` fills the region at the end edge, as high as the dialog under the title row and
    /// as wide as what it shows, before the body takes the rest of the width. It shows the columns
    /// ([`FullWindowSide`](crate::dialog::FullWindowSide)) side by side in reading order, and the dialog's footer
    /// sits among them.
    #[must_use]
    pub fn sides(mut self, content: impl FnMut(&mut Ui, &mut BodyCx<'_>) + 'a) -> Self {
        self.props = self.props.end(EndRegion::new().full_height(true));
        self.sides = Some(Box::new(content));
        self.place_footer()
    }

    /// Where the footer sits among the side columns, and its leading slot; a dialog without side columns ignores it.
    #[must_use]
    pub fn side_footer(mut self, footer: SideFooter<'a>) -> Self {
        self.side_footer = footer;
        self.place_footer()
    }

    /// The footer region: among the side columns when there are any, else across the dialog's foot.
    fn footer_region(&self) -> FooterRegion {
        FOOTER.place(if self.sides.is_some() { self.side_footer.place() } else { FooterPlace::Across })
    }

    fn place_footer(mut self) -> Self {
        if self.props.footer.is_some() {
            let region = self.footer_region();
            self.props = self.props.footer(region);
        }
        self
    }
}

/// Fills the side columns' region.
type SidesFn<'a> = Box<dyn FnMut(&mut Ui, &mut BodyCx<'_>) + 'a>;

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

    fn end(&mut self, ui: &mut Ui, cx: &mut BodyCx<'_>) {
        if let Some(sides) = self.sides.as_mut() {
            sides(ui, cx);
        }
    }

    fn footer(&mut self, ui: &mut Ui, cx: &mut BodyCx<'_>) {
        let enter = self.props.focus.enter == EnterRule::Submit;
        let host = frame::FooterHost { picked: &mut self.picked, guard: &mut self.guard, on_pick: &mut self.on_pick, settle: &mut self.settle };
        let (content, buttons) = (&mut self.footer, &self.buttons);
        if self.sides.is_some() {
            self.side_footer.show(ui, |ui| frame::footer(ui, cx, content, buttons, enter, host));
        } else {
            frame::footer(ui, cx, content, buttons, enter, host);
        }
    }

    fn closed(&mut self, why: CloseReason) {
        self.closed.get_or_insert(why);
    }
}

impl<'a> FooterHooks<'a> for FullWindowDialog<'a> {
    fn buttons(self, buttons: impl IntoIterator<Item = FooterButton>) -> Self {
        FullWindowDialog::buttons(self, buttons)
    }

    fn settle_buttons<T: FnMut(&mut BodyCx<'_>, &mut [FooterButton]) + 'a>(self, settle: T) -> Self {
        FullWindowDialog::settle_buttons(self, settle)
    }

    fn on_pick<T: FnMut(&FooterButton, &mut BodyCx<'_>) + 'a>(self, act: T) -> Self {
        FullWindowDialog::on_pick(self, act)
    }

    fn close_guard<T: FnMut(&FooterButton, &mut BodyCx<'_>) -> bool + 'a>(self, allows: T) -> Self {
        FullWindowDialog::close_guard(self, allows)
    }
}
