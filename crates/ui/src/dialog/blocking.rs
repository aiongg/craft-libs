//! `BlockingOverlay` (16.5): a modal layer with no title, for work the window must not interrupt.

use crate::contract::{
    Bind, BindMut, Common, Component, Container, ContainerProps, Cx, Mount, Overlay, Pad, Pointer as _, Props as _, Role, SaveKey,
};
use crate::runtime::env::Label;
use crate::runtime::focus::{FocusPolicy, Initial};
use crate::runtime::overlay::{Backdrop, Rank};
use crate::style::{ComponentStyle, StyleCx, SurfaceKind};
use craft_behavior::{CancelRule, CursorIcon, CursorPriority, Metrics, Sense};
use egui::Response;

/// The style of a blocking overlay: the card's surface and padding come from the container.
#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct BlockingOverlayStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// A modal layer that takes the window's input while it is shown, with the wait cursor. Its label is the accessible
/// name. Without a card the layer has no look at all; with one, the body sits on the dialog surface, centred.
///
/// - Open state: bound with [`opened`](BlockingOverlay::opened), else a session slot that starts open. Show it
///   every pass the work lasts, or bind the state to the work.
/// - Cancel: Escape closes it when [`cancellable`](BlockingOverlay::cancellable) says so; the open state then
///   reads `false`.
#[derive(crate::contract::Component)]
#[component(container)]
pub struct BlockingOverlay<'a> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    open: Option<Bind<'a, bool>>,
    cursor: CursorIcon,
}

impl<'a> BlockingOverlay<'a> {
    /// An overlay named `name`, with the wait cursor, no backdrop, no card and no cancel.
    pub fn new(name: impl Into<Label>) -> Self {
        let mut overlay = Overlay::modal();
        overlay.rank = Rank::modal(Backdrop::None);
        overlay.focus = FocusPolicy::DIALOG.initial(Initial::Keep).cancel(CancelRule::Ignore);
        let props = ContainerProps::new().mount(Mount::Layer(overlay));
        BlockingOverlay { common: Common::new(), props, open: None, cursor: CursorIcon::Wait }.label(name)
    }

    fn with_overlay(mut self, change: impl FnOnce(&mut Overlay)) -> Self {
        if let Mount::Layer(overlay) = &mut self.props.mount {
            change(overlay);
        }
        self
    }

    /// Binds the open state to `open`.
    #[must_use]
    pub fn opened(mut self, open: Bind<'a, bool>) -> Self {
        self.open = Some(open);
        self
    }

    /// The cursor while the overlay is shown.
    #[must_use]
    pub fn cursor(mut self, cursor: CursorIcon) -> Self {
        self.cursor = cursor;
        self
    }

    /// Dims the window behind the overlay.
    #[must_use]
    pub fn scrim(self, on: bool) -> Self {
        self.with_overlay(|overlay| overlay.rank = Rank::modal(if on { Backdrop::Scrim } else { Backdrop::None }))
    }

    /// A card on the dialog surface around the body.
    #[must_use]
    pub fn card(mut self, on: bool) -> Self {
        self.props.surface = on.then_some(SurfaceKind::Dialog);
        self.props.pad = if on { Pad::Roomy } else { Pad::None };
        self
    }

    /// Whether Escape closes the overlay.
    #[must_use]
    pub fn cancellable(self, on: bool) -> Self {
        self.with_overlay(|overlay| overlay.focus = overlay.focus.cancel(if on { CancelRule::Close } else { CancelRule::Ignore }))
    }

    /// Saves the open state under `key`.
    #[must_use]
    pub fn remember(mut self, key: SaveKey) -> Self {
        self.props = self.props.save(key);
        self
    }
}

impl Component for BlockingOverlay<'_> {
    const ROLE: Role = Role::DIALOG;
    type Style = BlockingOverlayStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: BlockingOverlayStyleKey) -> BlockingOverlayStyle {
        BlockingOverlayStyle::default()
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        cx.cursor(self.cursor, CursorPriority::Hover);
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

impl Container for BlockingOverlay<'_> {
    fn open(&mut self) -> Option<BindMut<'_, bool>> {
        self.open.as_mut().map(Bind::reborrow)
    }
}
