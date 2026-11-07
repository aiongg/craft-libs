//! `Bar`: the container of a toolbar, an options bar or a status bar.

use super::words::BarWords;
use crate::buttons::IconButton;
use crate::contract::{
    Access, BodyCx, BodyLayout, Common, Component, Container, ContainerProps, Cx, Gap, HeaderRegion, NoValue, Outcome, Pad, Props as _, Role,
    Show as _, Sizing, WordSetCx as _,
};
use crate::runtime::access::Name;
use crate::runtime::env::Label;
use crate::style::{ComponentStyle, StyleCx, SurfaceKind};
use craft_behavior::{IconId, Metrics, NavShape, Roving, Sense};
use craft_theme::ControlSize;
use egui::{Response, Ui};
use std::borrow::Cow;
use std::num::NonZeroU8;

/// What a bar is for; it picks the bar's role, spacing and name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BarKind {
    /// A strip of tools or commands.
    Tool,
    /// The options of the active tool.
    Options,
    /// Status messages and readouts.
    Status,
}

/// The style of a bar: the gap between the members of its body.
#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct BarStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// What a bar reports: its response and the commit and cancel buttons clicked this pass.
#[derive(Debug)]
#[non_exhaustive]
pub struct BarOutput {
    /// The bar's response.
    pub response: Response,
    /// The commit button was clicked.
    pub committed: bool,
    /// The cancel button was clicked.
    pub cancelled: bool,
}

impl Outcome for BarOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// A bar over a chrome surface whose body holds the members in reading order at the small control size. They are
/// one roving group. The bar is as tall as its tallest member plus the control padding. A bar of one row lays its
/// body out as a row; a bar of several rows, or a vertical one, as a stack whose rows the caller makes with `Row`.
/// A bar with `commit_cancel` ends with a cancel and a commit button.
#[derive(crate::contract::Component)]
#[component(container)]
pub struct Bar {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    #[style_key]
    kind: BarKind,
    rows: NonZeroU8,
    vertical: bool,
    commit_cancel: Option<(Label, Label)>,
    committed: bool,
    cancelled: bool,
}

impl Bar {
    /// A bar named `label` of `kind`.
    pub fn new(label: impl Into<Label>, kind: BarKind) -> Self {
        let gap = match kind {
            BarKind::Tool => Gap::Tight,
            BarKind::Options => Gap::Row,
            BarKind::Status => Gap::Section,
        };
        let props = ContainerProps::new()
            .surface(SurfaceKind::Chrome)
            .pad(Pad::Control)
            .sizing(Sizing::Fill)
            .body_gap(gap)
            .child_size(ControlSize::Sm)
            .roving(Roving::new(NavShape::Flat))
            .layout(BodyLayout::row());
        Bar { common: Common::new(), props, kind, rows: NonZeroU8::MIN, vertical: false, commit_cancel: None, committed: false, cancelled: false }
            .label(label)
    }

    /// Sets the number of rows the body holds.
    #[must_use]
    pub fn rows(mut self, rows: NonZeroU8) -> Self {
        self.rows = rows;
        self.lay_out()
    }

    /// Lays the body out along the vertical axis.
    #[must_use]
    pub fn vertical(mut self) -> Self {
        self.vertical = true;
        self.lay_out()
    }

    /// Ends the bar with a cancel and a commit button named `cancel` and `commit`; their tooltips add Esc and Enter,
    /// which the caller's session handles as keys.
    #[must_use]
    pub fn commit_cancel(mut self, commit: impl Into<Label>, cancel: impl Into<Label>) -> Self {
        self.commit_cancel = Some((commit.into(), cancel.into()));
        self.props = self.props.header(HeaderRegion::new().inline(true).trailing(true));
        self
    }

    fn lay_out(mut self) -> Self {
        let layout = if self.vertical || self.rows.get() > 1 { BodyLayout::stack() } else { BodyLayout::row() };
        self.props = self.props.layout(layout).sizing(if self.vertical { Sizing::Content } else { Sizing::Fill });
        self
    }
}

impl Component for Bar {
    const ROLE: Role = Role::TOOLBAR;
    const PARTS: bool = false;
    const STOPS: bool = false;
    const ALT_ROLES: &'static [Role] = &[Role::GROUP, Role::STATUS];
    const OVERLAYS: bool = false;
    type Style = BarStyle;
    type Output = BarOutput;

    fn style(cx: &StyleCx<'_>, key: BarStyleKey) -> BarStyle {
        let gap = match key.kind {
            BarKind::Tool => cx.tokens.space.gap.tight,
            BarKind::Options => cx.tokens.space.gap.row,
            BarKind::Status => cx.tokens.space.gap.section,
        };
        BarStyle { metrics: Metrics::new().gap(gap) }
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    /// An options bar is a group and a status bar a status region; a bar the caller did not label is named by the
    /// words of the family.
    fn access(&self, access: &mut Access, cx: &Cx<'_, Self>) {
        match self.kind {
            BarKind::Tool => {}
            BarKind::Options => {
                access.role(Role::GROUP);
            }
            BarKind::Status => {
                access.role(Role::STATUS);
            }
        }
        if cx.resolved.label.is_empty() {
            let words = cx.words::<BarWords>();
            let name = match self.kind {
                BarKind::Tool => &words.tool,
                BarKind::Options => &words.options,
                BarKind::Status => &words.status,
            };
            access.name(Name::Text(name.as_str().to_owned()));
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> BarOutput {
        BarOutput { response: cx.response.clone(), committed: self.committed, cancelled: self.cancelled }
    }
}

impl Container for Bar {
    /// The cancel button, then the commit button at the reading end.
    fn controls(&mut self, ui: &mut Ui, cx: &mut BodyCx<'_>) {
        let Some((commit, cancel)) = &self.commit_cancel else { return };
        let words = cx.env.words::<BarWords>();
        let tip = |label: &Label, key: &Label| Label::from_cow(Cow::Owned(format!("{} ({})", cx.env.resolve(label).as_str(), key.as_str())));
        let cancel_tip = tip(cancel, &words.escape_key);
        let commit_tip = tip(commit, &words.enter_key);
        self.cancelled |= crate::contract::clicked(&IconButton::new(IconId::new("ban")).label(cancel.clone()).tooltip(cancel_tip).show(ui));
        self.committed |= crate::contract::clicked(&IconButton::new(IconId::new("check")).label(commit.clone()).tooltip(commit_tip).show(ui));
    }
}
