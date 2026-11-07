//! `Dialog` (16.5): `DialogFrame`'s chrome on an egui layer: modal or modeless, movable by its header, with a
//! focus trap, placement and size remembered under a `SaveKey`.

use super::frame::{self, DialogOutput, FOOTER, FooterCx, FooterFn};
use crate::contract::overlay::{FloatPlace, OverlayFit, Place};
use crate::contract::{
    Bind, BindMut, BodyCx, Common, Component, Container, ContainerProps, Cx, DragBand, HeaderRegion, Mount, Overlay, Props as _, Role, SaveKey,
};
use crate::runtime::env::Label;
use crate::runtime::focus::Initial;
use crate::runtime::overlay::{Backdrop, CloseReason, Rank};
use crate::sections::FooterButton;
use crate::style::{ComponentStyle, StyleCx};
use craft_behavior::{Metrics, Sense};
use egui::{Pos2, Ui};

/// The style of a dialog: the surface and padding come from the container.
#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct DialogStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// A dialog on its own layer. It opens in the middle of the window, moves by its title band, scrolls its body when the
/// window is smaller and traps the keyboard focus while it is open.
///
/// - Open state: bound with [`opened`](Dialog::opened) to the app's value or the host's entry, else kept in a
///   session slot that starts open. The close action, Escape and `BodyCx::request_close` write `false`.
/// - Placement and size: saved across runs under [`remember`](Dialog::remember); a resizable dialog also saves the
///   size the user left it at.
/// - A modal dialog blocks input below it in every window; a press outside does nothing.
#[derive(crate::contract::Component, crate::contract::Props)]
#[component(container)]
pub struct Dialog<'a> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    /// Binds the open state to `open`.
    #[prop(name = "opened")]
    open: Option<Bind<'a, bool>>,
    footer: Option<FooterFn<'a>>,
    buttons: Vec<FooterButton>,
    /// Whether the title row has a close action.
    #[prop]
    closable: bool,
    closed: Option<CloseReason>,
    picked: Option<usize>,
}

impl<'a> Dialog<'a> {
    /// A modal dialog titled `title`, with a close action; it moves by its title band and is not resizable until
    /// `resizable` says so.
    pub fn new(title: impl Into<Label>) -> Self {
        let overlay = Overlay::dialog().resizable(false);
        Dialog::on(title, overlay, HeaderRegion::new().band(DragBand::Move).rule(true))
    }

    /// A prompt titled `title`: a dialog on the modal preset, centred with its content capped, fixed in place and
    /// not remembered.
    pub fn prompt(title: impl Into<Label>) -> Self {
        Dialog::on(title, Overlay::modal(), HeaderRegion::new().rule(true))
    }

    fn on(title: impl Into<Label>, overlay: Overlay, header: HeaderRegion) -> Self {
        let props = frame::chrome_props().mount(Mount::Layer(overlay)).header(header);
        Dialog { common: Common::new(), props, open: None, footer: None, buttons: Vec::new(), closable: true, closed: None, picked: None }
            .label(title)
    }

    fn with_overlay(mut self, change: impl FnOnce(Overlay) -> Overlay) -> Self {
        let overlay = match std::mem::take(&mut self.props.mount) {
            Mount::Layer(overlay) => overlay,
            _ => Overlay::dialog().resizable(false),
        };
        self.props = self.props.mount(Mount::Layer(change(overlay)));
        self
    }

    /// A tool window: no modal layer and no backdrop; its edits are the app's normal edits.
    #[must_use]
    pub fn modeless(self) -> Self {
        self.with_overlay(Overlay::modeless)
    }

    /// What is drawn behind a modal dialog.
    #[must_use]
    pub fn backdrop(self, backdrop: Backdrop) -> Self {
        self.with_overlay(|mut overlay| {
            if overlay.rank.is_modal() {
                overlay.rank = Rank::modal(backdrop);
            }
            overlay
        })
    }

    /// Whether edge grips resize the dialog, and the size the user leaves it at is saved.
    #[must_use]
    pub fn resizable(self, on: bool) -> Self {
        self.with_overlay(|overlay| if on { overlay.resizable(true).fit(OverlayFit::Resizable) } else { overlay.resizable(false) })
    }

    /// What takes the keyboard focus when the dialog opens.
    #[must_use]
    pub fn initial_focus(self, initial: Initial) -> Self {
        self.with_overlay(|overlay| {
            let focus = overlay.focus.initial(initial);
            overlay.focus(focus)
        })
    }

    /// Where the dialog first opens, as a screen position of its top-left corner; the middle of the window without
    /// it. A saved placement comes first.
    #[must_use]
    pub fn first_place(self, at: Pos2) -> Self {
        self.with_overlay(|overlay| overlay.place(Place::Float(FloatPlace::at(at))))
    }

    /// Saves the placement and size under `key`, so a reopened dialog sits where the user left it.
    #[must_use]
    pub fn remember(mut self, key: SaveKey) -> Self {
        self.props = self.props.save(key);
        self
    }

    /// Whether Enter, pressed with the focus inside and left untaken by the focused part, closes the dialog with
    /// `CloseReason::Enter` (the OK of the dialog).
    #[must_use]
    pub fn close_on_enter(self, on: bool) -> Self {
        self.with_overlay(|mut overlay| {
            overlay.dismiss = overlay.dismiss.enter(on);
            overlay
        })
    }

    /// Whether the dialog holds one outer gesture over its body's fields, from open to close.
    #[must_use]
    pub fn outer_gesture(mut self, on: bool) -> Self {
        self.props = self.props.outer_edit(on);
        self
    }

    /// The larger frame margin.
    #[must_use]
    pub fn roomy(mut self, on: bool) -> Self {
        self.props = self.props.pad(if on { crate::contract::Pad::Roomy } else { crate::contract::Pad::Regular });
        self
    }

    /// The dialog buttons in the footer, in the platform's order for their roles. A click closes the dialog: `Cancel`
    /// as `CloseReason::Requested`, every other role as `CloseReason::Activated`; `DialogOutput::picked` says which.
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

impl Component for Dialog<'_> {
    const ROLE: Role = Role::DIALOG;
    type Style = DialogStyle;
    type Output = DialogOutput;

    fn style(_: &StyleCx<'_>, _: DialogStyleKey) -> DialogStyle {
        DialogStyle::default()
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn finish(self, cx: Cx<'_, Self>) -> DialogOutput {
        DialogOutput { response: cx.response.clone(), closed: self.closed, picked: self.picked }
    }
}

impl Container for Dialog<'_> {
    fn open(&mut self) -> Option<BindMut<'_, bool>> {
        self.open.as_mut().map(Bind::reborrow)
    }

    fn header(&mut self, ui: &mut Ui, cx: &mut BodyCx<'_>) {
        frame::header(ui, cx, frame::close_action(self.closable));
    }

    fn footer(&mut self, ui: &mut Ui, cx: &mut BodyCx<'_>) {
        frame::footer(ui, cx, &mut self.footer, &self.buttons, &mut self.picked);
    }

    fn closed(&mut self, why: CloseReason) {
        self.closed.get_or_insert(why);
    }
}
