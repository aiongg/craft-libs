//! `FullWindowSide`: a side column of a full-window dialog, with its title row, scrolling body and footer buttons.

use crate::contract::{
    BodyCx, Common, Component, Container, ContainerProps, Cx, FooterRegion, HeaderRegion, NoValue, Outcome, Pad, Props as _, Role, RowAlign, Scroll,
    Show as _, ShowContainer as _, Sizing, WordSetCx as _,
};
use crate::layout_components::Row;
use crate::runtime::focus::FocusPolicy;
use crate::sections::{FooterBar, FooterButton, HeaderStrength, TitleRow};
use crate::style::{ComponentStyle, StyleCx, SurfaceKind};
use craft_behavior::{Metrics, Sense};
use egui::{Response, Ui, Vec2};

/// How wide a side column is. The widths are `size.float.width`, the width a panel opens at, except a narrow column,
/// which is as wide as its content.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SideWidth {
    /// The standard width: Liquify, Content-Aware Fill, Filter Gallery, Wide Angle.
    #[default]
    Standard,
    /// The wide width: Select and Mask's Properties.
    Wide,
    /// A column that carries only the footer buttons (Vanishing Point): as wide as they are.
    Narrow,
}

/// The dialog's footer buttons in a side column: at its bottom (Liquify, Select and Mask, Content-Aware Fill, Wide
/// Angle) or its top (Filter Gallery), in the platform's order for their roles, with an optional leading slot (a
/// Reset icon button) before them.
#[must_use = "pass the footer to `FullWindowSide::footer`"]
pub struct SideFooter<'a> {
    buttons: Vec<FooterButton>,
    top: bool,
    leading: Option<Box<dyn FnMut(&mut Ui) + 'a>>,
}

impl<'a> SideFooter<'a> {
    /// The buttons at the bottom of the column, under its body.
    pub fn bottom(buttons: impl IntoIterator<Item = FooterButton>) -> Self {
        SideFooter { buttons: buttons.into_iter().collect(), top: false, leading: None }
    }

    /// The buttons at the top of the column, under its title row (Filter Gallery).
    pub fn top(buttons: impl IntoIterator<Item = FooterButton>) -> Self {
        SideFooter { buttons: buttons.into_iter().collect(), top: true, leading: None }
    }

    /// Shows `add` at the reading start of the footer row, before the buttons: Select and Mask's and Content-Aware
    /// Fill's Reset.
    pub fn leading(mut self, add: impl FnMut(&mut Ui) + 'a) -> Self {
        self.leading = Some(Box::new(add));
        self
    }

    /// Shows the row and returns the index of the button picked this pass.
    fn show(&mut self, ui: &mut Ui) -> Option<usize> {
        let align = if self.leading.is_some() { RowAlign::Start } else { RowAlign::End };
        let (buttons, leading) = (&self.buttons, &mut self.leading);
        Row::new()
            .fill()
            .align(align)
            .show(ui, |ui, _| {
                if let Some(add) = leading.as_mut() {
                    add(ui);
                }
                FooterBar::buttons(buttons.iter().cloned()).show_footer(ui).picked
            })
            .inner
            .flatten()
    }
}

/// The style of a side column: its width.
#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct FullWindowSideStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// What a side column reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct FullWindowSideOutput {
    /// The column's response.
    pub response: Response,
    /// The index of the footer button picked this pass, in the order given to the footer.
    pub picked: Option<usize>,
}

impl Outcome for FullWindowSideOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// A side column of a full-window dialog: Liquify's and Select and Mask's Properties, Content-Aware Fill's Preview
/// and settings panels, Filter Gallery's settings.
///
/// - **Look.** The panel surface; the title (the label) in a title row at the top, none with
///   [`headerless`](Self::headerless); the body padded by the panel tier.
/// - **Body.** Scrolls vertically when taller than the room; [`unscrolled`](Self::unscrolled) leaves the scroll out
///   for a pane that bounds itself.
/// - **Footer.** The dialog's buttons at the bottom of the column or under the title row, as [`SideFooter`] says.
///   Only one column of a dialog has the footer; the pick is reported in `FullWindowSideOutput::picked`.
/// - **Keyboard and AccessKit.** A group named by the label; Tab goes top to bottom, the footer buttons last (first
///   for the top footer).
/// - **Width.** [`SideWidth`]; the column's height is the room it is shown in.
#[derive(crate::contract::Component, crate::contract::Props)]
#[component(container)]
pub struct FullWindowSide<'a> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    /// How wide the column is.
    #[style_key]
    #[prop]
    width: SideWidth,
    footer: Option<SideFooter<'a>>,
    /// Whether the title row shows.
    titled: bool,
    picked: Option<usize>,
}

impl<'a> FullWindowSide<'a> {
    /// A column named and titled `title`, of the standard width, with a scrolling body and no footer.
    pub fn new(title: impl Into<crate::runtime::env::Label>) -> Self {
        let props = ContainerProps::new()
            .surface(SurfaceKind::Panel)
            .pad(Pad::Regular)
            .sizing(Sizing::Content)
            .header(HeaderRegion::new().rule(true))
            .scroll(Scroll::vertical())
            .focus(FocusPolicy::PANE);
        FullWindowSide { common: Common::new(), props, width: SideWidth::Standard, footer: None, titled: true, picked: None }.label(title)
    }

    /// A column with no title row: the label names the column for assistive technology only.
    #[must_use]
    pub fn headerless(mut self) -> Self {
        self.titled = false;
        self.props.header = self.footer.as_ref().filter(|footer| footer.top).map(|_| HeaderRegion::new());
        self
    }

    /// A column whose body does not scroll: a pane that bounds itself.
    #[must_use]
    pub fn unscrolled(mut self) -> Self {
        self.props.scroll = None;
        self
    }

    /// The dialog's footer buttons in this column.
    #[must_use]
    pub fn footer(mut self, footer: SideFooter<'a>) -> Self {
        if footer.top {
            self.props.header.get_or_insert_with(|| HeaderRegion::new().rule(self.titled));
        } else {
            self.props = self.props.footer(FooterRegion::new().rule(true));
        }
        self.footer = Some(footer);
        self
    }
}

impl Component for FullWindowSide<'_> {
    const ROLE: Role = Role::GROUP;
    const PARTS: bool = false;
    const STOPS: bool = false;
    type Style = FullWindowSideStyle;
    type Output = FullWindowSideOutput;

    fn style(cx: &StyleCx<'_>, key: FullWindowSideStyleKey) -> FullWindowSideStyle {
        let width = match key.width {
            SideWidth::Narrow => 0.0,
            _ => cx.tokens.size.float.width,
        };
        FullWindowSideStyle { metrics: Metrics::new().min_size(Vec2::new(width, 0.0)) }
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn finish(self, cx: Cx<'_, Self>) -> FullWindowSideOutput {
        FullWindowSideOutput { response: cx.response.clone(), picked: self.picked }
    }
}

impl Container for FullWindowSide<'_> {
    fn header(&mut self, ui: &mut Ui, cx: &mut BodyCx<'_>) {
        if self.titled {
            TitleRow::new(cx.label()).strength(HeaderStrength::Strong).show(ui);
        }
        if let Some(footer) = self.footer.as_mut().filter(|footer| footer.top) {
            self.picked = footer.show(ui).or(self.picked);
        }
    }

    fn footer(&mut self, ui: &mut Ui, _: &mut BodyCx<'_>) {
        if let Some(footer) = self.footer.as_mut().filter(|footer| !footer.top) {
            self.picked = footer.show(ui).or(self.picked);
        }
    }
}
