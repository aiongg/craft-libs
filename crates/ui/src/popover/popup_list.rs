//! `PopupList` (15, 17): a dropdown under a field, holding the caller's `List`.

use super::anchored::PopoverOutput;
use crate::contract::{Bind, BindMut, Common, Component, Container, ContainerProps, Cx, Mount, Overlay, Pad, Props as _, Role, Trigger};
use crate::runtime::env::Label;
use crate::runtime::overlay::CloseReason;
use crate::style::{ComponentStyle, StyleCx, SurfaceKind};
use craft_behavior::{Metrics, Sense};
use egui::Response;

/// The style of a popup list: the surface comes from the container.
#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct PopupListStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// A list popup under a field the app owns (a combo, a history picker). The body is the caller's `List`.
///
/// - The popup is at least as wide as its opener and capped to a share of the viewport.
/// - The opener keeps the keyboard focus; the list takes the arrow keys while the popup is open.
/// - Activating a row closes the popup with `CloseReason::Activated`; Escape and a press outside close it too.
/// - Open state: the caller's binding. The output's `closed` says why it closed in this pass.
///
/// Components that own their list (`Select`, completions) open it from their overlay hook instead.
#[derive(crate::contract::Component, crate::contract::Props)]
#[component(container)]
pub struct PopupList<'a> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    open: Bind<'a, bool>,
    closed: Option<CloseReason>,
}

impl<'a> PopupList<'a> {
    /// A popup list named `label`, open while `open` is true.
    pub fn new(label: impl Into<Label>, open: Bind<'a, bool>) -> Self {
        let mut overlay = Overlay::dropdown();
        overlay.dismiss = overlay.dismiss.on_activate(true);
        let props = ContainerProps::new().surface(SurfaceKind::Popup).pad(Pad::None).collection_body().mount(Mount::Layer(overlay));
        PopupList { common: Common::new(), props, open, closed: None }.label(label)
    }

    /// The field the popup hangs from. The popup closes when the opener is no longer shown.
    #[must_use]
    pub fn opened_by(mut self, opener: &Response) -> Self {
        let overlay = match std::mem::take(&mut self.props.mount) {
            Mount::Layer(overlay) => overlay,
            _ => Overlay::dropdown(),
        };
        self.props = self.props.mount(Mount::Layer(overlay.opened_by(opener, Trigger::Manual)));
        self
    }
}

impl Component for PopupList<'_> {
    const ROLE: Role = Role::LIST_BOX;
    type Style = PopupListStyle;
    type Output = PopoverOutput;

    fn style(_: &StyleCx<'_>, _: PopupListStyleKey) -> PopupListStyle {
        PopupListStyle::default()
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn finish(self, cx: Cx<'_, Self>) -> PopoverOutput {
        PopoverOutput { response: cx.response.clone(), closed: self.closed }
    }
}

impl Container for PopupList<'_> {
    fn open(&mut self) -> Option<BindMut<'_, bool>> {
        Some(self.open.reborrow())
    }

    fn closed(&mut self, why: CloseReason) {
        self.closed.get_or_insert(why);
    }
}
