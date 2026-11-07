//! `ApplyRow`: a bulk-edit row whose check box turns "apply this value to the selection" on, with the caller's field
//! as the body. It is a group box configuration: the same header and gate, with the header inline so the check and
//! the field share one line.

#[cfg(feature = "testing")]
mod stories;

use super::title_row::{HeaderStrength, TitleRow};
use crate::contract::{
    Bind, BindMut, BodyCx, Common, Component, Container, ContainerProps, Cx, Edit as _, EditResponse, HeaderRegion, Pad, Props as _, Role, Show as _,
    WordSetCx as _,
};
use crate::style::{ComponentStyle, StyleCx};
use craft_behavior::{Metrics, Sense};
use egui::Ui;

/// The style of an apply row: no frame and no padding of its own.
#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct ApplyRowStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// A bulk-edit row: a check box named by the row's label edits `apply`, and the body, the caller's field bound to a
/// `Mixed` value, is enabled only while `apply` is on. Opening a dialog or visiting a tab never turns mixed values into
/// the first item's value: `apply` starts off.
#[derive(crate::contract::Component)]
#[component(container)]
pub struct ApplyRow<'a> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    apply: Bind<'a, bool>,
}

impl<'a> ApplyRow<'a> {
    /// A row labelled `label` that edits `apply`.
    pub fn new(label: impl Into<crate::runtime::env::Label>, apply: Bind<'a, bool>) -> Self {
        let props = ContainerProps::new().pad(Pad::None).header(HeaderRegion::new().inline(true));
        ApplyRow { common: Common::new(), props, apply }.label(label)
    }
}

impl Component for ApplyRow<'_> {
    const ROLE: Role = Role::GROUP;
    type Style = ApplyRowStyle;
    type Output = EditResponse<bool>;

    fn style(_: &StyleCx<'_>, _: ApplyRowStyleKey) -> ApplyRowStyle {
        ApplyRowStyle::default()
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn bind(&mut self) -> Option<BindMut<'_, bool>> {
        Some(self.apply.reborrow())
    }

    fn finish(self, cx: Cx<'_, Self>) -> EditResponse<bool> {
        cx.edit_response()
    }
}

impl Container for ApplyRow<'_> {
    /// The apply check's shown copy goes to the check box; the container applies a change at step 15.
    fn header(&mut self, ui: &mut Ui, cx: &mut BodyCx<'_>) {
        let row = TitleRow::new(cx.label()).strength(HeaderStrength::Normal).fill(false);
        match cx.gate() {
            Some(gate) => row.gate(Bind::bound(gate)).show(ui),
            None => row.show(ui),
        };
    }
}
