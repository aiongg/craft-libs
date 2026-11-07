//! `Dialog` (16.5): `DialogFrame`'s chrome on an egui layer: modal or modeless, movable by its header, with a
//! focus trap, placement and size remembered under a `SaveKey`.

use super::frame::{self, FOOTER, FooterCx, FooterFn};
use crate::contract::overlay::{FloatPlace, OverlayFit, Place};
use crate::contract::{
    BindMut, BodyCx, Bind, Common, Component, Container, ContainerProps, Cx, DragBand, HeaderRegion, Mount, Overlay, Props as _, Role, SaveKey,
};
use crate::runtime::env::Label;
use crate::runtime::focus::Initial;
use crate::runtime::overlay::{Backdrop, CloseReason, Rank};
use crate::style::{ComponentStyle, StyleCx};
use craft_behavior::{Metrics, Sense};
use egui::{Pos2, Response, Ui};

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
#[derive(crate::contract::Component)]
#[component(container)]
pub struct Dialog<'a> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    open: Option<Bind<'a, bool>>,
    footer: Option<FooterFn<'a>>,
    closable: bool,
    on_close: Option<Box<dyn FnMut(CloseReason) + 'a>>,
}

impl<'a> Dialog<'a> {
    /// A modal dialog titled `title`, with a close action; it is not resizable until `resizable` says so.
    pub fn new(title: impl Into<Label>) -> Self {
        let overlay = Overlay::dialog().resizable(false);
        let props = frame::chrome_props().mount(Mount::Layer(overlay)).header(HeaderRegion::new().band(DragBand::Move).rule(true));
        Dialog { common: Common::new(), props, open: None, footer: None, closable: true, on_close: None }.label(title)
    }

    fn with_overlay(mut self, change: impl FnOnce(Overlay) -> Overlay) -> Self {
        let overlay = match std::mem::take(&mut self.props.mount) {
            Mount::Layer(overlay) => overlay,
            _ => Overlay::dialog().resizable(false),
        };
        self.props = self.props.mount(Mount::Layer(change(overlay)));
        self
    }

    /// Binds the open state to `open`.
    #[must_use]
    pub fn opened(mut self, open: Bind<'a, bool>) -> Self {
        self.open = Some(open);
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

    /// Reports how the dialog closed: `Escape` is the cancel, `Enter` the OK of `close_on_enter`, `Activated` and
    /// `Requested` what a footer button asked for with `FooterCx::request_close_as`. Called once for each close.
    #[must_use]
    pub fn on_close(mut self, report: impl FnMut(CloseReason) + 'a) -> Self {
        self.on_close = Some(Box::new(report));
        self
    }

    /// Whether the title row has a close action.
    #[must_use]
    pub fn closable(mut self, on: bool) -> Self {
        self.closable = on;
        self
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

    /// The footer's content: the closure fills a footer bar under the body.
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
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: DialogStyleKey) -> DialogStyle {
        DialogStyle::default()
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
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
        frame::footer(ui, cx, &mut self.footer);
    }

    fn closed(&mut self, why: CloseReason) {
        if let Some(report) = self.on_close.as_mut() {
            report(why);
        }
    }
}
