//! `TitleRow` (16.4): the leaf that fills a container's header region. Its lead and actions are parts that wear
//! the faces of other components, so they have those components' look, content paint and AccessKit role.

use super::words::SectionWords;
use crate::buttons::IconButton;
use crate::choice::Checkbox;
use crate::contract::{
    Access, Bind, BindMut, Common, Component, Cx, Edit as _, EditResponse, Focus as _, MeasureCx, Outcome, PaintUi, PartsCx, Pointer as _,
    Props as _, Role, Styled as _, WordSetCx as _,
};
use crate::controls::disclosure::Disclosure;
use crate::primitives::{IconSpec, Span, TextRun};
use crate::runtime::env::Label;
use crate::style::{ComponentStyle, IconLook, Ink, StyleCx, TextLook, TypeRole};
use craft_behavior::{Flow, IconId, Metrics, NavShape, PartKey, PartRef, Roving, Sense};
use craft_theme::{Stateful, States};
use egui::{Rect, Response, Vec2};
use std::borrow::Cow;

#[cfg(feature = "testing")]
mod stories;

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
    /// A quiet heading: the caption role, faint, in capitals.
    Caption,
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

    #[cfg(feature = "testing")]
    fn story_facts(&self) -> crate::testing::StoryOutput {
        self.edit.story_facts()
    }
}

enum Lead<'a> {
    None,
    Disclosure(Bind<'a, bool>),
    Check(Bind<'a, bool>),
}

const LEAD: PartKey = PartKey::new("lead");
const ACTION: PartKey = PartKey::new("action");
/// The title band: a click on it toggles a disclosure lead, and it carries the tooltip of an elided title.
const TITLE: PartKey = PartKey::new("title");

/// The style of a title row.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct TitleRowStyle {
    #[style(metrics)]
    metrics: Metrics,
    /// The title's type role and colour.
    #[style(text)]
    text: TextLook,
    /// The icon before the title.
    #[style(icon)]
    icon: IconLook,
}

/// An optional icon, a title with an optional subtitle, a lead (a disclosure bound to the container's open state, or
/// a check box bound to its gate) and action parts. A row too narrow for the title and subtitle together drops the
/// subtitle before it elides the title; an elided title shows whole in a tooltip. A click on the title band toggles a
/// disclosure lead, as a click on the lead does.
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
    /// An icon before the title, painted in the style's icon colour unless the spec names another.
    #[prop]
    icon: Option<IconSpec<'a>>,
    /// Whether the row takes the whole width offered, so its actions sit at the end edge; off, it is as wide as its
    /// parts.
    #[prop]
    fill: bool,
    lead: Lead<'a>,
    actions: Vec<HeaderAction>,
    taken: Option<HeaderAction>,
    placed: Option<Placed>,
}

/// Where the icon, title and subtitle sit in the row, decided with the final width in `parts`.
#[derive(Clone, Copy)]
struct Placed {
    icon: Option<Rect>,
    title: Rect,
    subtitle: Option<Rect>,
}

impl<'a> TitleRow<'a> {
    /// A title row for `title`.
    pub fn new(title: impl Into<Label>) -> Self {
        let common = Common::new();
        TitleRow {
            common,
            strength: HeaderStrength::Normal,
            level: 2,
            subtitle: None,
            icon: None,
            fill: true,
            lead: Lead::None,
            actions: Vec::new(),
            taken: None,
            placed: None,
        }
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

    /// The title as it shows: a caption reads in capitals.
    fn title_text<'t>(&self, label: &'t str) -> Cow<'t, str> {
        if self.strength == HeaderStrength::Caption { Cow::Owned(label.to_uppercase()) } else { Cow::Borrowed(label) }
    }

    /// Divides the room left between the lead and the actions: the icon, then the title, then the subtitle when
    /// both texts fit whole.
    fn place(&self, mut band: Flow, cx: &PartsCx<'_, Self>) -> Placed {
        let side = band.rect().height();
        let mut icon = None;
        if self.icon.is_some() {
            let (slot, rest) = band.take_start(side);
            let (_, rest) = rest.take_start(cx.gap());
            icon = Some(slot);
            band = rest;
        }
        let role = cx.style.text.role;
        let subtitle = self.subtitle.as_ref().and_then(|subtitle| {
            let title = cx.layout_at(&TextRun::new(self.title_text(cx.resolved.label)).role(role), f32::INFINITY).galley.size().x;
            let width = cx.layout_at(&subtitle_run(subtitle.as_str(), role), f32::INFINITY).galley.size().x;
            (title + cx.gap() + width <= band.rect().width()).then_some(title)
        });
        match subtitle {
            Some(title) => {
                let (title, rest) = band.take_start(title);
                let (_, rest) = rest.take_start(cx.gap());
                Placed { icon, title, subtitle: Some(rest.rect()) }
            }
            None => Placed { icon, title: band.rect(), subtitle: None },
        }
    }
}

impl Component for TitleRow<'_> {
    const ROLE: Role = Role::HEADING;
    type Style = TitleRowStyle;
    type Output = TitleOutput;

    fn style(cx: &StyleCx<'_>, key: TitleRowStyleKey) -> TitleRowStyle {
        let text = &cx.tokens.color.text;
        let (height, role, shade) = match key.strength {
            HeaderStrength::Normal => (cx.control().height, TypeRole::Label, text.normal),
            HeaderStrength::Strong => (cx.tokens.size.panel_header, TypeRole::Title, text.normal),
            HeaderStrength::Section => (cx.tokens.size.panel_header, TypeRole::Heading, text.normal),
            HeaderStrength::Caption => (cx.control().height, TypeRole::Caption, text.faint),
        };
        let (shade, ink, disabled) = (cx.color(shade), cx.ink(Ink::OnPanel), cx.color(text.disabled));
        TitleRowStyle {
            metrics: Metrics::new().min_size(Vec2::new(0.0, height)).gap(cx.control().gap),
            text: TextLook::new(role, Stateful::new(shade).off(States::new(shade, shade, shade, disabled))),
            icon: IconLook::new(cx.control().icon, Stateful::new(ink).off(States::new(ink, ink, ink, disabled))),
        }
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        let role = cx.style.text.role;
        let title = cx.layout(&TextRun::new(self.title_text(cx.resolved.label)).role(role)).galley.size();
        let subtitle = self.subtitle.as_ref().map(|subtitle| cx.layout(&subtitle_run(subtitle.as_str(), role)).galley.size().x);
        let parts = usize::from(self.has_lead()) + usize::from(self.icon.is_some()) + self.actions.len();
        // The parts are square on the row's height, which the height token can raise above the text's.
        let side = title.y.max(cx.style.metrics.min_size.y);
        let text = title.x + subtitle.map_or(0.0, |width| width + cx.gap());
        let natural = text + side * parts as f32 + cx.gap() * parts as f32;
        Vec2::new(if self.fill { natural.max(cx.room.unwrap_or(0.0)) } else { natural }, title.y)
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
        let band = flow.rect();
        let placed = self.place(flow, cx);
        let title = TextRun::new(self.title_text(cx.resolved.label)).role(cx.style.text.role).elide();
        let elided = cx.layout_at(&title, placed.title.width()).elided;
        let sense = if matches!(self.lead, Lead::Disclosure(_)) { Sense::CLICK } else { Sense::HOVER };
        let tooltip = cx.resolved.label.to_owned();
        let part = cx.part(TITLE, band).sense(sense);
        if elided {
            part.tooltip(tooltip);
        }
        self.placed = Some(placed);
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
        let row_toggles = matches!(self.lead, Lead::Disclosure(_)) && cx.part(TITLE).press.phase.is_click();
        if self.has_lead() && (hit(cx, LEAD.into()) || row_toggles) {
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
        let words = cx.words::<SectionWords>();
        let value = cx.value().copied().unwrap_or(false);
        match &self.lead {
            Lead::None => {}
            Lead::Disclosure(_) => ui.face(LEAD, Disclosure::new(Bind::given(value)).node(false).label(cx.resolved.label.to_owned())),
            Lead::Check(_) => ui.face(LEAD, Checkbox::new(Bind::given(value)).label(words.gate.clone())),
        }
        for (at, action) in self.actions.iter().enumerate() {
            let (icon, label) = match action {
                HeaderAction::Close => ("close", words.close.clone()),
                HeaderAction::Menu => ("more", words.menu.clone()),
                HeaderAction::Float => ("float", words.float.clone()),
            };
            ui.face(ACTION.at(at as u64), IconButton::new(IconId::new(icon)).label(label));
        }
        let Some(placed) = self.placed else { return };
        let role = cx.style.text.role;
        if let (Some(rect), Some(icon)) = (placed.icon, &mut self.icon) {
            if icon.color.is_none() {
                icon.color = Some(cx.resolve(&cx.style.icon.color));
            }
            ui.draw().icon(rect, icon);
        }
        let mut title = TextRun::new(self.title_text(cx.resolved.label)).role(role).elide();
        if self.strength == HeaderStrength::Caption {
            title = title.color(cx.resolve(&cx.style.text.color));
        }
        ui.draw().text(placed.title, &title);
        if let (Some(rect), Some(subtitle)) = (placed.subtitle, &self.subtitle) {
            ui.draw().text(rect, &subtitle_run(subtitle.as_str(), role));
        }
    }

    fn access(&self, access: &mut Access, _: &Cx<'_, Self>) {
        access.level(self.level);
    }

    fn finish(self, cx: Cx<'_, Self>) -> TitleOutput {
        TitleOutput { edit: cx.edit_response(), action: self.taken }
    }
}

fn subtitle_run(subtitle: &str, role: TypeRole) -> TextRun<'_> {
    TextRun::spans(vec![Span::new(subtitle).muted()]).role(role)
}
