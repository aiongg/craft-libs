//! `TitleRow` (16.4): the leaf that fills a container's header region. Its lead and actions are parts that wear
//! the faces of other components, so they have those components' look, content paint and AccessKit role.

use super::words::SectionWords;
use crate::contract::{
    Access, Bind, BindMut, Common, Component, Cx, Edit as _, EditResponse, Focus as _, MeasureCx, Outcome, PaintUi, PartsCx, Pointer as _,
    Props as _, Role, WordSetCx as _,
};
use crate::buttons::IconButton;
use crate::controls::check::Checkbox;
use crate::controls::disclosure::Disclosure;
use crate::primitives::{Span, TextRun};
use crate::runtime::env::Label;
use crate::style::{ComponentStyle, StyleCx, TypeRole};
use craft_behavior::{Flow, IconId, Metrics, NavShape, PartKey, PartRef, Roving, Sense};
use egui::{Rect, Response, Vec2};

/// How much weight a header carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum HeaderStrength {
    /// A control-height row with label text.
    Normal,
    /// A panel or dialog title.
    Strong,
    /// A section heading.
    Section,
}

/// What a title row's action parts ask the container to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum HeaderAction {
    /// Close the container: the container calls `cx.request_close()`.
    Close,
    /// Show the container's menu from its `overlay` hook.
    Menu,
    /// Float the container out of its dock.
    Float,
}

/// What `TitleRow` reports: the lead's value edits and the action taken.
#[derive(Debug)]
#[non_exhaustive]
pub struct TitleOutput {
    /// The lead's edits and the row's response.
    pub edit: EditResponse<bool>,
    /// The action part taken this pass.
    pub action: Option<HeaderAction>,
}

impl Outcome for TitleOutput {
    type Value = bool;

    fn response(&self) -> &Response {
        &self.edit.response
    }
}

enum Lead<'a> {
    None,
    Disclosure(Bind<'a, bool>),
    Check(Bind<'a, bool>),
}

const LEAD: PartKey = PartKey::new("lead");
const ACTION: PartKey = PartKey::new("action");

/// The style of a title row.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct TitleRowStyle {
    #[style(metrics)]
    metrics: Metrics,
    role: TypeRole,
}

/// A title with an optional subtitle, a lead (a disclosure bound to the container's open state, or a check box
/// bound to its gate) and action parts.
#[derive(crate::contract::Component, crate::contract::Props)]
pub struct TitleRow<'a> {
    #[common]
    common: Common,
    /// The header's weight.
    #[style_key]
    #[prop]
    strength: HeaderStrength,
    /// The heading level AccessKit reports.
    #[prop]
    level: usize,
    /// Text after the title.
    #[prop(into)]
    subtitle: Option<Label>,
    lead: Lead<'a>,
    actions: Vec<HeaderAction>,
    taken: Option<HeaderAction>,
}

impl<'a> TitleRow<'a> {
    /// A title row for `title`.
    pub fn new(title: impl Into<Label>) -> Self {
        let common = Common::new();
        TitleRow { common, strength: HeaderStrength::Normal, level: 2, subtitle: None, lead: Lead::None, actions: Vec::new(), taken: None }
            .label(title)
    }

    /// A check box lead bound to the container's gate.
    #[must_use]
    pub fn gate(mut self, gate: Bind<'a, bool>) -> Self {
        self.lead = Lead::Check(gate);
        self
    }

    /// A disclosure lead bound to the container's open state.
    #[must_use]
    pub fn disclosure(mut self, open: Bind<'a, bool>) -> Self {
        self.lead = Lead::Disclosure(open);
        self
    }

    /// The action parts at the end of the row, in reading order.
    #[must_use]
    pub fn actions(mut self, actions: impl IntoIterator<Item = HeaderAction>) -> Self {
        self.actions = actions.into_iter().collect();
        self
    }

    fn has_lead(&self) -> bool {
        !matches!(self.lead, Lead::None)
    }
}

impl Component for TitleRow<'_> {
    const ROLE: Role = Role::HEADING;
    type Style = TitleRowStyle;
    type Output = TitleOutput;

    fn style(cx: &StyleCx<'_>, key: TitleRowStyleKey) -> TitleRowStyle {
        let (height, role) = match key.strength {
            HeaderStrength::Normal => (cx.control().height, TypeRole::Label),
            HeaderStrength::Strong => (cx.tokens.size.panel_header, TypeRole::Title),
            HeaderStrength::Section => (cx.tokens.size.panel_header, TypeRole::Heading),
        };
        TitleRowStyle { metrics: Metrics::new().min_size(Vec2::new(0.0, height)).gap(cx.control().gap), role }
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        let text = cx.layout(&title_run(self, cx.style.role, cx.resolved.label)).galley.size();
        let parts = usize::from(self.has_lead()) + self.actions.len();
        // The parts are square on the row's height, which the height token can raise above the text's.
        let side = text.y.max(cx.style.metrics.min_size.y);
        let natural = text.x + side * parts as f32 + cx.gap() * parts as f32;
        // The row fills the width it is offered, so its actions sit at the end edge.
        Vec2::new(natural.max(cx.room.unwrap_or(0.0)), text.y)
    }

    fn sense(&self) -> Sense {
        Sense::HOVER
    }

    /// Lead and actions are focusable parts of one roving group.
    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        let side = rect.height();
        let mut flow = Flow::new(rect, cx.flow.is_rtl());
        if self.has_lead() {
            let (lead, rest) = flow.take_start(side);
            cx.part(LEAD, lead).sense(Sense::CLICK | Sense::FOCUS).focusable();
            flow = rest;
        }
        // The last action sits at the end edge, so the group reads in the order given; stops register in that order.
        let mut rects = Vec::new();
        for _ in &self.actions {
            let (action, rest) = flow.take_end(side);
            rects.push(action);
            flow = rest;
        }
        for (at, action) in rects.into_iter().rev().enumerate() {
            cx.part(ACTION.at(at as u64), action).sense(Sense::CLICK | Sense::FOCUS).focusable();
        }
        cx.roving(Roving::new(NavShape::Flat));
    }

    fn bind(&mut self) -> Option<BindMut<'_, bool>> {
        match &mut self.lead {
            Lead::None => None,
            Lead::Disclosure(bind) | Lead::Check(bind) => Some(bind.reborrow()),
        }
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let by_key = cx.activation().filter(|activation| activation.by_key).and(cx.active_part());
        let hit = |cx: &Cx<'_, Self>, part: PartRef| cx.part(part).press.phase.is_click() || by_key == Some(part);
        if self.has_lead() && hit(cx, LEAD.into()) {
            let now = cx.value().copied().unwrap_or(false);
            cx.set(!now);
        }
        for (at, action) in self.actions.iter().enumerate() {
            if hit(cx, ACTION.at(at as u64)) {
                self.taken = Some(*action);
            }
        }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let side = cx.rect.height();
        let rtl = cx.env.direction().is_rtl();
        let words = cx.words::<SectionWords>();
        let mut flow = Flow::new(cx.rect, rtl);
        let value = cx.value().copied().unwrap_or(false);
        match &self.lead {
            Lead::None => {}
            Lead::Disclosure(_) => {
                ui.face(LEAD, Disclosure::new(Bind::given(value)).node(false).label(cx.resolved.label.to_owned()));
                flow = flow.take_start(side).1;
            }
            Lead::Check(_) => {
                ui.face(LEAD, Checkbox::new(Bind::given(value)).label(words.gate.clone()));
                flow = flow.take_start(side).1;
            }
        }
        for (at, action) in self.actions.iter().enumerate().rev() {
            let (icon, label) = match action {
                HeaderAction::Close => ("close", words.close.clone()),
                HeaderAction::Menu => ("more", words.menu.clone()),
                HeaderAction::Float => ("float", words.float.clone()),
            };
            ui.face(ACTION.at(at as u64), IconButton::new(IconId::new(icon)).label(label));
            flow = flow.take_end(side).1;
        }
        ui.draw().text(flow.rect(), &title_run(self, cx.style.role, cx.resolved.label));
    }

    fn access(&self, access: &mut Access, _: &Cx<'_, Self>) {
        access.level(self.level);
    }

    fn finish(self, cx: Cx<'_, Self>) -> TitleOutput {
        TitleOutput { edit: cx.edit_response(), action: self.taken }
    }
}

fn title_run<'t>(row: &'t TitleRow<'_>, role: TypeRole, title: &'t str) -> TextRun<'t> {
    let mut spans = vec![Span::new(title)];
    if let Some(subtitle) = &row.subtitle {
        spans.push(Span::new(format!(" {}", subtitle.as_str())).muted());
    }
    TextRun::spans(spans).role(role)
}
