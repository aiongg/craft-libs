//! `Dialog` (16.5): `DialogFrame`'s chrome on an egui layer: modal or modeless, movable by its header, with a
//! focus trap, placement and size remembered under a `SaveKey`.

use super::frame::{self, DialogOutput, FOOTER, FooterCx, FooterFn};
use crate::contract::overlay::{FloatPlace, OverlayFit, Place};
use crate::contract::{
    Bind, BindMut, BodyCx, Common, Component, Container, ContainerProps, Cx, DragBand, HeaderRegion, Mount, Overlay, Props as _, Role, SaveKey,
};
use crate::runtime::access::Access;
use crate::runtime::env::Label;
use crate::runtime::focus::Initial;
use crate::runtime::overlay::{Backdrop, CloseReason, Rank};
use crate::sections::{DialogRole, FooterButton};
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
    /// Lets the host refuse the close of a picked button: `allows` gets the index of every picked button but
    /// `Cancel` and the dialog's body context, so a host reads its kept state, and returns whether a button that
    /// closes the dialog may. It is not the place to act on a pick (see
    /// `ResetFooter::install`). On `false` the dialog stays open, with `DialogOutput::picked` still reporting the pick; the host shows why. A button marked `stays_open` never closes, whatever it returns.
    #[prop(boxed, name = "close_guard")]
    guard: Option<Box<dyn FnMut(usize, &mut BodyCx<'_>) -> bool + 'a>>,
    /// Sets the buttons' state each pass: `settle` gets the dialog's body context and the buttons when the footer is
    /// shown, after the body ran, so a button's `enabled` can follow what the body wrote in the same pass (through
    /// `BodyCx::memory`, say). Enter's routing is decided from the buttons given to `buttons`.
    #[prop(boxed, name = "settle_buttons")]
    settle: Option<Box<dyn FnMut(&mut BodyCx<'_>, &mut [FooterButton]) + 'a>>,
    /// Acts on a picked footer button in the footer step, before the close is decided: `act` gets the button's index
    /// and the dialog's body context. `ResetFooter::install` sets it.
    #[prop(boxed, name = "on_pick")]
    on_pick: Option<Box<dyn FnMut(usize, &mut BodyCx<'_>) + 'a>>,
    buttons: Vec<FooterButton>,
    /// Whether Enter closes the dialog as its OK.
    enter: bool,
    /// Whether the title row has a close action.
    #[prop]
    closable: bool,
    closed: Option<CloseReason>,
    picked: Option<usize>,
    alert: bool,
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
        Dialog {
            common: Common::new(),
            props,
            open: None,
            footer: None,
            guard: None,
            on_pick: None,
            settle: None,
            buttons: Vec::new(),
            enter: false,
            closable: true,
            closed: None,
            picked: None,
            alert: false,
        }
        .label(title)
    }

    /// Changes the layer the dialog shows in, for the settings without a setter of their own: the size a resizable
    /// dialog has until the user resizes it (`Overlay::size`) and the least it may be resized to (`Overlay::min_size`).
    #[must_use]
    pub fn overlay(mut self, change: impl FnOnce(Overlay) -> Overlay) -> Self {
        self.props = match std::mem::take(&mut self.props.mount) {
            Mount::Layer(overlay) => self.props.mount(Mount::Layer(change(overlay))),
            Mount::Viewport { fallback, spec } => self.props.mount(Mount::Viewport { fallback: change(fallback), spec }),
            _ => self.props.mount(Mount::Layer(change(Overlay::dialog().resizable(false)))),
        };
        self
    }

    /// Shows the dialog in its own OS window, with the layer it has so far as the fallback where windows are
    /// unavailable. Call it after the settings that shape that layer, or before: later settings shape the fallback.
    #[must_use]
    pub fn viewport(self) -> Self {
        let Mount::Layer(overlay) = self.props.mount.clone() else { return self };
        let mount = Mount::viewport(overlay);
        Dialog { props: self.props.mount(mount), ..self }
    }

    /// A tool window: no modal layer and no backdrop; its edits are the app's normal edits.
    #[must_use]
    pub fn modeless(self) -> Self {
        self.overlay(Overlay::modeless)
    }

    /// What is drawn behind a modal dialog.
    #[must_use]
    pub fn backdrop(self, backdrop: Backdrop) -> Self {
        self.overlay(|mut overlay| {
            if overlay.rank.is_modal() {
                overlay.rank = Rank::modal(backdrop);
            }
            overlay
        })
    }

    /// Whether edge grips resize the dialog, and the size the user leaves it at is saved.
    #[must_use]
    pub fn resizable(self, on: bool) -> Self {
        self.overlay(|overlay| if on { overlay.resizable(true).fit(OverlayFit::Resizable) } else { overlay.resizable(false) })
    }

    /// What takes the keyboard focus when the dialog opens.
    #[must_use]
    pub fn initial_focus(self, initial: Initial) -> Self {
        self.overlay(|overlay| {
            let focus = overlay.focus.initial(initial);
            overlay.focus(focus)
        })
    }

    /// Where the dialog first opens, as a screen position of its top-left corner; the middle of the window without
    /// it. A saved placement comes first.
    #[must_use]
    pub fn first_place(self, at: Pos2) -> Self {
        self.overlay(|overlay| overlay.place(Place::Float(FloatPlace::at(at))))
    }

    /// Saves the placement and size under `key`, so a reopened dialog sits where the user left it.
    #[must_use]
    pub fn remember(mut self, key: SaveKey) -> Self {
        self.props = self.props.save(key);
        self
    }

    /// Whether Enter, pressed with the focus inside and left untaken by the focused part, is the OK of the dialog.
    /// With an enabled `Accept` button in the footer, Enter activates that button, so a click and Enter take the same
    /// way (`close_guard` included) and `DialogOutput::picked` says so. Without one, Enter closes the dialog with
    /// `CloseReason::Enter`.
    #[must_use]
    pub fn close_on_enter(mut self, on: bool) -> Self {
        self.enter = on;
        self.route_enter()
    }

    /// Sets the overlay's own Enter dismissal for the dialogs whose footer has no default button to take it.
    fn route_enter(self) -> Self {
        let dismiss = self.enter && !self.buttons.iter().any(|button| button.role() == DialogRole::Accept && button.is_enabled());
        self.overlay(|mut overlay| {
            overlay.dismiss = overlay.dismiss.enter(dismiss);
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

    /// The dialog reports the alert-dialog role instead of the dialog role.
    #[must_use]
    pub(super) fn alert(mut self) -> Self {
        self.alert = true;
        self
    }

    /// The dialog buttons in the footer, in the platform's order for their roles. A click closes the dialog: `Cancel`
    /// as `CloseReason::Requested`, every other role as `CloseReason::Activated` unless the button `stays_open`;
    /// `DialogOutput::picked` says which.
    #[must_use]
    pub fn buttons(mut self, buttons: impl IntoIterator<Item = FooterButton>) -> Self {
        self.props = self.props.footer(FOOTER);
        self.buttons = buttons.into_iter().collect();
        self.route_enter()
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
    const PARTS: bool = false;
    const STOPS: bool = false;
    const ALT_ROLES: &'static [Role] = &[Role::ALERT_DIALOG];
    type Style = DialogStyle;
    type Output = DialogOutput;

    fn style(_: &StyleCx<'_>, _: DialogStyleKey) -> DialogStyle {
        DialogStyle::default()
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn access(&self, access: &mut Access, _: &Cx<'_, Self>) {
        if self.alert {
            access.role(Role::ALERT_DIALOG);
        }
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
        frame::footer(
            ui,
            cx,
            &mut self.footer,
            &self.buttons,
            self.enter,
            frame::FooterHost { picked: &mut self.picked, guard: &mut self.guard, on_pick: &mut self.on_pick, settle: &mut self.settle },
        );
    }

    fn closed(&mut self, why: CloseReason) {
        self.closed.get_or_insert(why);
    }
}
