//! `Panel` (16.5): an inline container on the panel surface that fills the room it is given. Its header is a
//! `TitleRow` with optional icon, subtitle and actions, its body scrolls vertically and an optional footer sits under
//! the body. The panel is one pane for F6.

use super::FooterBar;
use super::title_row::{HeaderAction, HeaderStrength, MenuRows, TitleRow};
use crate::contract::overlay::Popup;
use crate::contract::{
    Bind, BindMut, BodyCx, Common, Component, Container, ContainerProps, Cx, FooterRegion, HeaderRegion, NoValue, Outcome, Pad, Props as _, Role,
    Scroll, Show as _, ShowContainer as _, Sizing, WordSetCx as _,
};
use crate::primitives::IconSpec;
use crate::runtime::env::Label;
use crate::runtime::focus::FocusPolicy;
use crate::runtime::overlay::CloseReason;
use crate::style::{ComponentStyle, StyleCx, SurfaceKind};
use craft_behavior::{Metrics, Sense};
use egui::{Id, Response, Ui};

#[cfg(feature = "testing")]
mod stories;

/// The style of a panel: the surface and padding come from the container.
#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct PanelStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// What a panel reports: its response, the header action taken this pass and how it closed.
#[derive(Debug)]
#[non_exhaustive]
pub struct PanelOutput {
    /// The panel's response.
    pub response: Response,
    /// The header action part taken this pass. The panel runs Close and Menu itself; Float is the host's.
    pub action: Option<HeaderAction>,
    /// How the panel closed in this pass (`Requested` for the close action); `None` while it stays open.
    pub closed: Option<CloseReason>,
}

impl Outcome for PanelOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// The closure that fills the footer bar.
type FooterFn<'a> = Box<dyn FnMut(&mut Ui, &mut BodyCx<'_>) + 'a>;

/// A panel of a workspace: title row, scrolling body, optional footer.
///
/// - Header: the title, with the optional icon and subtitle, and the actions in `actions`. The Menu action opens the
///   rows given to `menu` under the title row; Close asks the panel to close, which hides its body until the host
///   stops showing it (bind the state with `opened` to keep it); Float is reported in `PanelOutput::action` for the
///   host. [`headerless`](Panel::headerless) leaves the header out.
/// - Body: scrolls vertically; the panel fills the width and height offered.
/// - Footer: the closure given to `footer` fills a footer bar under the body, separated by a rule.
#[derive(crate::contract::Component, crate::contract::Props)]
#[component(container)]
pub struct Panel<'a> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    /// Binds the open state to `open`.
    #[prop(name = "opened")]
    open: Option<Bind<'a, bool>>,
    /// Text after the title in the header.
    #[prop(into)]
    subtitle: Option<Label>,
    /// An icon before the title.
    #[prop]
    icon: Option<IconSpec<'a>>,
    /// The rows of the menu the Menu action opens.
    #[prop(func = "FnMut(&mut Ui, &mut BodyCx<'_>)")]
    menu: Option<MenuRows<'a>>,
    actions: Vec<HeaderAction>,
    footer: Option<FooterFn<'a>>,
    taken: Option<HeaderAction>,
    menu_opener: Option<Id>,
    closed: Option<CloseReason>,
}

impl<'a> Panel<'a> {
    /// A panel titled `title`, with a header, a vertical scroll and no footer.
    pub fn new(title: impl Into<Label>) -> Self {
        let props = ContainerProps::new()
            .surface(SurfaceKind::Panel)
            .pad(Pad::Regular)
            .sizing(Sizing::Fill)
            .header(HeaderRegion::new().rule(true))
            .scroll(Scroll::vertical())
            .focus(FocusPolicy::PANE);
        Panel {
            common: Common::new(),
            props,
            open: None,
            subtitle: None,
            icon: None,
            menu: None,
            actions: Vec::new(),
            footer: None,
            taken: None,
            menu_opener: None,
            closed: None,
        }
        .label(title)
    }

    /// The action parts at the end of the header, in reading order.
    #[must_use]
    pub fn actions(mut self, actions: impl IntoIterator<Item = HeaderAction>) -> Self {
        self.actions = actions.into_iter().collect();
        self
    }

    /// A panel with no header: the title names the panel for assistive technology only.
    #[must_use]
    pub fn headerless(mut self) -> Self {
        self.props.header = None;
        self
    }

    /// The footer's content: the closure fills a footer bar under the body.
    #[must_use]
    pub fn footer(mut self, content: impl FnMut(&mut Ui, &mut BodyCx<'_>) + 'a) -> Self {
        self.props = self.props.footer(FooterRegion::new().rule(true));
        self.footer = Some(Box::new(content));
        self
    }
}

impl Component for Panel<'_> {
    const ROLE: Role = Role::GROUP;
    const PARTS: bool = false;
    const STOPS: bool = false;
    type Style = PanelStyle;
    type Output = PanelOutput;

    fn style(_: &StyleCx<'_>, _: PanelStyleKey) -> PanelStyle {
        PanelStyle::default()
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn finish(self, mut cx: Cx<'_, Self>) -> PanelOutput {
        if let Some(opener) = self.menu_opener {
            Popup::toggle(opener, &mut cx);
        }
        PanelOutput { response: cx.response.clone(), action: self.taken, closed: self.closed }
    }
}

impl Container for Panel<'_> {
    fn open(&mut self) -> Option<BindMut<'_, bool>> {
        self.open.as_mut().map(Bind::reborrow)
    }

    fn header(&mut self, ui: &mut Ui, cx: &mut BodyCx<'_>) {
        let mut row = TitleRow::new(cx.label()).strength(HeaderStrength::Strong).actions(self.actions.iter().copied());
        if let Some(subtitle) = self.subtitle.clone() {
            row = row.subtitle(subtitle);
        }
        if let Some(icon) = self.icon.clone() {
            row = row.icon(icon);
        }
        let shown = row.show(ui);
        if shown.action == Some(HeaderAction::Close) {
            cx.request_close();
        }
        self.taken = shown.action.or(self.taken);
        if let Some(rows) = self.menu.as_mut() {
            self.menu_opener = rows.hang(ui, cx, &shown);
        }
    }

    fn footer(&mut self, ui: &mut Ui, _: &mut BodyCx<'_>) {
        if let Some(content) = self.footer.as_mut() {
            FooterBar::body().show(ui, |ui, bar| content(ui, bar));
        }
    }

    fn closed(&mut self, why: CloseReason) {
        self.closed.get_or_insert(why);
    }
}
