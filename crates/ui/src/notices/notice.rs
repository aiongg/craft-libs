//! `Notice` (20.3): a message in a tone with an icon, an optional title, actions and a dismiss button. One leaf
//! covers the inline notice, the toast card and the empty state; the layout is a style key. Actions and the
//! dismiss button are parts that wear the faces of `Link`, `Button` and `IconButton`.

use super::words::NoticeWords;
use crate::buttons::{Button, IconButton, Link};
use crate::contract::{
    Access, Accessible as _, Common, Component, Cx, Focus as _, MeasureCx, Name, NoValue, Outcome, PaintUi, PartsCx, Pointer as _, Politeness,
    Props as _, Remember as _, Role, Styled as _, WordSetCx as _,
};
use crate::primitives::{IconSpec, TextRun, Wrap};
use crate::runtime::commands::{self, CommandRef, FireSource};
use crate::runtime::env::{EnvView, Label};
use crate::style::{ComponentStyle, IconLook, Insets, StyleColor, StyleCx, SurfaceKind, SurfaceLook, TextLook, TypeRole, UiTokens};
use craft_behavior::{Flow, IconId, Metrics, NavShape, PartKey, PartRef, Roving, Sense};
use craft_theme::{Rgba, Stateful, States, variant::Tone};
use egui::{Align, Pos2, Rect, Response, Vec2};

/// Where a notice sits and how much room it takes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum NoticeLayout {
    /// The icon at the reading start and the text beside it, across the width offered, with no surface.
    #[default]
    Inline,
    /// The inline layout on a floating surface, as wide as its text up to a token width: a toast.
    Card,
    /// Icon, title, message and actions in a column centred in the room: an empty state.
    Centred,
}

/// The icon of a notice.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub enum NoticeIcon<'a> {
    /// The tone's icon; the neutral tone has none.
    #[default]
    Tone,
    /// No icon.
    None,
    /// This icon, in the tone's colour unless the spec names another.
    Custom(IconSpec<'a>),
}

/// An action of a notice: a label, or a command that supplies it and runs when the action is taken.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct NoticeAction {
    label: Label,
    command: Option<CommandRef>,
}

impl NoticeAction {
    /// An action named `label`; the notice reports it as `NoticeOutput.action`.
    pub fn new(label: impl Into<Label>) -> Self {
        NoticeAction { label: label.into(), command: None }
    }

    /// An action that shows `command`'s label and fires it when taken; the notice reports it as well.
    pub fn command(command: impl Into<CommandRef>) -> Self {
        NoticeAction { label: Label::Empty, command: Some(command.into()) }
    }
}

/// What a notice reports: its response, the action taken this pass and whether dismiss was pressed.
#[derive(Debug)]
#[non_exhaustive]
pub struct NoticeOutput {
    /// The notice's response; its hover tells a toast host to hold the timer.
    pub response: Response,
    /// The index of the action taken this pass.
    pub action: Option<usize>,
    /// The dismiss button was pressed this pass.
    pub dismissed: bool,
}

impl Outcome for NoticeOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

const ACTION: PartKey = PartKey::new("action");
const DISMISS: PartKey = PartKey::new("dismiss");
/// The title's band; the centred layout names it as a heading.
const TITLE: PartKey = PartKey::new("title");
/// The heading level of the centred title: the empty state's top-level heading.
const TITLE_LEVEL: usize = 1;
const DISMISS_ICON: &str = "close";

/// Whether the first show of a notice has been announced.
#[derive(Clone, Default)]
struct Announced(bool);

/// The style of a notice.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct NoticeStyle {
    /// A floating surface in the card layout, none otherwise.
    #[style(surface)]
    surface: SurfaceLook,
    #[style(metrics)]
    metrics: Metrics,
    /// The title's role and colour.
    #[style(text)]
    title: TextLook,
    /// The message's role and colour.
    #[style(text)]
    message: TextLook,
    /// The icon's side and colour.
    #[style(icon)]
    icon: IconLook,
    /// The width a card wraps its text at when the room is open.
    card_width: f32,
}

/// A message in a tone. The tone picks the icon and the colours; the neutral tone reads muted and has no icon. The
/// message is the label; a title above it is optional. Actions read as links (as buttons in the centred layout) and
/// are reported in the output; with `dismiss` a close button is the last part. With `announce` set, the first show is
/// announced once.
#[derive(crate::contract::Component, crate::contract::Props)]
pub struct Notice<'a> {
    #[common]
    common: Common,
    /// The tone: the icon and the colours.
    #[style_key]
    tone: Tone,
    /// The layout.
    #[style_key]
    #[prop]
    layout: NoticeLayout,
    /// A title above the message.
    #[prop(into)]
    title: Option<Label>,
    /// The icon.
    #[prop]
    icon: NoticeIcon<'a>,
    /// Shows a dismiss button.
    #[prop(flag)]
    dismiss: bool,
    /// Announces the notice once when it first shows.
    #[prop]
    announce: Option<Politeness>,
    actions: Vec<NoticeAction>,
    placed: Option<Placed>,
    taken: Option<usize>,
    dismissed: bool,
}

/// Where each element sits, decided with the final size in `parts`.
#[derive(Clone, Default)]
struct Placed {
    icon: Option<Rect>,
    title: Option<Rect>,
    message: Option<Rect>,
    actions: Vec<Rect>,
    dismiss: Option<Rect>,
}

/// Sizes of the elements for one width.
struct Plan {
    icon: Option<f32>,
    title: Option<Vec2>,
    message: Vec2,
    actions: Vec<Vec2>,
    dismiss: Option<Vec2>,
    /// The width the elements need at this width: the natural width when it is unbounded.
    needs: f32,
    height: f32,
}

/// The texts of a notice as resolved for this pass.
struct Texts<'t> {
    title: Option<String>,
    message: &'t str,
}

impl<'a> Notice<'a> {
    /// A notice of `tone` saying `message`.
    pub fn new(tone: Tone, message: impl Into<Label>) -> Self {
        Notice {
            common: Common::new(),
            tone,
            layout: NoticeLayout::Inline,
            title: None,
            icon: NoticeIcon::Tone,
            dismiss: false,
            announce: None,
            actions: Vec::new(),
            placed: None,
            taken: None,
            dismissed: false,
        }
        .label(message)
    }

    /// Adds an action after the message, in reading order.
    #[must_use]
    pub fn action(mut self, action: NoticeAction) -> Self {
        self.actions.push(action);
        self
    }

    fn texts<'t>(&self, env: &EnvView<'_>, message: &'t str) -> Texts<'t> {
        Texts { title: self.title.as_ref().map(|title| env.resolve(title).as_str().to_owned()), message }
    }

    fn icon_spec(&self) -> Option<IconSpec<'a>> {
        match &self.icon {
            NoticeIcon::Tone => tone_icon(self.tone).map(|id| IconSpec::svg(IconId::new(id))),
            NoticeIcon::Custom(spec) => Some(spec.clone()),
            _ => None,
        }
    }

    fn action_labels(&self, ctx: &egui::Context) -> Vec<Label> {
        self.actions
            .iter()
            .map(|action| match &action.command {
                Some(command) => Label::from(commands::resolve(ctx, command.clone()).label),
                None => action.label.clone(),
            })
            .collect()
    }

    fn row(&self) -> bool {
        self.layout != NoticeLayout::Centred
    }

    fn face_size(&self, cx: &MeasureCx<'_, Self>, label: Label) -> Vec2 {
        if self.row() { cx.face(Link::new(label).role(cx.style.message.role)) } else { cx.face(Button::new(label)) }
    }

    /// Sizes every element for `width`, which is infinite to find the natural width.
    fn plan(&self, cx: &MeasureCx<'_, Self>, texts: &Texts<'_>, width: f32) -> Plan {
        let gap = cx.gap();
        let words = cx.words::<NoticeWords>();
        let row = self.row();
        let icon = self.icon_spec().map(|_| cx.style.icon.side);
        let dismiss = (self.dismiss && row).then(|| cx.face(IconButton::new(IconId::new(DISMISS_ICON)).label(words.dismiss.clone())));
        let actions: Vec<Vec2> = self.action_labels(cx.ui.ctx()).into_iter().map(|label| self.face_size(cx, label)).collect();
        let icon_width = if row { icon.map_or(0.0, |side| side + gap) } else { 0.0 };
        let dismiss_width = dismiss.map_or(0.0, |size| size.x + gap);
        let column = if width.is_finite() { (width - icon_width - dismiss_width).max(0.0) } else { f32::INFINITY };
        let run = |text: &str, role: TypeRole| cx.layout_at(&TextRun::new(text.to_owned()).role(role).wrap(Wrap::Wrap), column).size();
        let title = texts.title.as_deref().map(|title| run(title, cx.style.title.role));
        let message = run(texts.message, cx.style.message.role);
        let actions_width = actions.iter().map(|size| size.x).sum::<f32>() + gap * actions.len().saturating_sub(1) as f32;
        let actions_height = actions.iter().map(|size| size.y).fold(0.0, f32::max);
        let text_width = title.map_or(0.0, |size| size.x).max(message.x).max(actions_width);
        let text_height = title.map_or(0.0, |size| size.y) + message.y;
        let with_actions = text_height + if actions.is_empty() { 0.0 } else { gap + actions_height };
        let (needs, height) = if row {
            (icon_width + text_width + dismiss_width, with_actions.max(icon.unwrap_or(0.0)).max(dismiss.map_or(0.0, |size| size.y)))
        } else {
            let parts = [icon, title.map(|size| size.y), Some(message.y), (!actions.is_empty()).then_some(actions_height)];
            let shown: Vec<f32> = parts.into_iter().flatten().collect();
            (text_width.max(icon.unwrap_or(0.0)), shown.iter().sum::<f32>() + gap * shown.len().saturating_sub(1) as f32)
        };
        Plan { icon, title, message, actions, dismiss, needs, height }
    }
}

impl Component for Notice<'_> {
    const ROLE: Role = Role::STATUS;
    const ALT_ROLES: &'static [Role] = &[Role::ALERT, Role::LABEL];
    const OVERLAYS: bool = false;
    type Style = NoticeStyle;
    type Output = NoticeOutput;

    fn style(cx: &StyleCx<'_>, key: NoticeStyleKey) -> NoticeStyle {
        let text = &cx.tokens.color.text;
        let disabled = cx.color(text.disabled);
        let stateful = |color: StyleColor| Stateful::new(color).off(States::new(color, color, color, disabled));
        let tone = if key.tone == Tone::Neutral { cx.color(text.muted) } else { cx.tone(key.tone) };
        let (normal, muted) = (cx.color(text.normal), cx.color(text.muted));
        let control = cx.control();
        let (title, message, surface, padding) = match key.layout {
            NoticeLayout::Inline => (
                TextLook::new(TypeRole::BodyStrong, stateful(tone)),
                TextLook::new(TypeRole::Caption, stateful(tone)),
                SurfaceLook::new(Stateful::new(cx.color(Rgba::TRANSPARENT))),
                Insets::uniform(0.0),
            ),
            NoticeLayout::Card => {
                let fill = StyleColor(cx.surface(SurfaceKind::Floating).fill);
                (
                    TextLook::new(TypeRole::BodyStrong, stateful(normal)),
                    TextLook::new(TypeRole::Body, stateful(muted)),
                    SurfaceLook::new(Stateful::new(fill)).kind(SurfaceKind::Floating).radius(UiTokens::corner(cx.tokens.shape.radius.popup)),
                    Insets::uniform(cx.tokens.space.pad.dialog),
                )
            }
            NoticeLayout::Centred => (
                TextLook::new(TypeRole::Title, stateful(normal)),
                TextLook::new(TypeRole::Body, stateful(muted)),
                SurfaceLook::new(Stateful::new(cx.color(Rgba::TRANSPARENT))),
                Insets::horizontal(control.pad_x),
            ),
        };
        NoticeStyle {
            surface,
            metrics: Metrics::new().padding(padding).gap(control.gap),
            title,
            message,
            icon: IconLook::new(control.icon, stateful(tone)),
            card_width: cx.tokens.size.length.field,
        }
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        let texts = self.texts(&cx.env, cx.resolved.label);
        let natural = self.plan(cx, &texts, f32::INFINITY).needs;
        let width = match (self.layout, cx.room) {
            (NoticeLayout::Card, Some(room)) => natural.min(cx.style.card_width).min(room),
            (NoticeLayout::Card, None) => natural.min(cx.style.card_width),
            (_, Some(room)) => room,
            (_, None) => natural,
        };
        Vec2::new(width, self.plan(cx, &texts, width).height)
    }

    fn sense(&self) -> Sense {
        Sense::HOVER
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        let rtl = cx.flow.is_rtl();
        let gap = cx.gap();
        let texts = self.texts(&cx.env, cx.resolved.label);
        let plan = self.plan(cx, &texts, rect.width());
        let mut placed = Placed::default();
        if self.row() {
            let mut flow = Flow::new(rect, rtl);
            if let Some(side) = plan.icon {
                let (slot, rest) = flow.take_start(side);
                placed.icon = Some(Rect::from_min_size(slot.min, Vec2::splat(side)));
                flow = rest.take_start(gap).1;
            }
            if let Some(size) = plan.dismiss {
                let (slot, rest) = flow.take_end(size.x);
                placed.dismiss = Some(Rect::from_min_size(slot.min, size));
                flow = rest.take_end(gap).1;
            }
            let column = flow.rect();
            let mut y = column.top();
            if let Some(size) = plan.title {
                placed.title = Some(Rect::from_min_size(Pos2::new(column.left(), y), Vec2::new(column.width(), size.y)));
                y += size.y;
            }
            placed.message = Some(Rect::from_min_size(Pos2::new(column.left(), y), Vec2::new(column.width(), plan.message.y)));
            y += plan.message.y + gap;
            let mut line = Flow::new(Rect::from_min_size(Pos2::new(column.left(), y), Vec2::new(column.width(), (rect.bottom() - y).max(0.0))), rtl);
            for size in &plan.actions {
                let (slot, rest) = line.take_start(size.x);
                placed.actions.push(Rect::from_min_size(slot.min, *size));
                line = rest.take_start(gap).1;
            }
        } else {
            let mut y = rect.top() + ((rect.height() - plan.height) / 2.0).max(0.0);
            let mut put = |size: Vec2| {
                let size = Vec2::new(size.x.min(rect.width()), size.y);
                let band = Rect::from_min_size(Pos2::new(rect.left(), y), Vec2::new(rect.width(), size.y));
                y += size.y + gap;
                Flow::new(band, rtl).align(size, Align::Center, Align::Min)
            };
            placed.icon = plan.icon.map(|side| put(Vec2::splat(side)));
            placed.title = plan.title.map(&mut put);
            placed.message = Some(put(plan.message));
            placed.actions = Vec::new();
            if !plan.actions.is_empty() {
                let total = plan.actions.iter().map(|size| size.x).sum::<f32>() + gap * (plan.actions.len() - 1) as f32;
                let height = plan.actions.iter().map(|size| size.y).fold(0.0, f32::max);
                let band = Rect::from_min_size(Pos2::new(rect.left(), y), Vec2::new(rect.width(), height));
                let aligned = Flow::new(band, rtl).align(Vec2::new(total.min(rect.width()), height), Align::Center, Align::Min);
                let mut line = Flow::new(aligned, rtl);
                for size in &plan.actions {
                    let (slot, rest) = line.take_start(size.x);
                    placed.actions.push(Rect::from_min_size(slot.min, *size));
                    line = rest.take_start(gap).1;
                }
            }
        }
        if self.layout == NoticeLayout::Centred
            && let Some(title) = placed.title
        {
            cx.part(TITLE, title).sense(Sense::HOVER);
        }
        for (at, rect) in placed.actions.iter().enumerate() {
            cx.part(ACTION.at(at as u64), *rect).sense(Sense::CLICK | Sense::FOCUS).focusable();
        }
        if let Some(rect) = placed.dismiss {
            cx.part(DISMISS, rect).sense(Sense::CLICK | Sense::FOCUS).focusable();
        }
        if !placed.actions.is_empty() || placed.dismiss.is_some() {
            cx.roving(Roving::new(NavShape::Flat));
        }
        self.placed = Some(placed);
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let by_key = cx.activation().filter(|activation| activation.by_key).and(cx.active_part());
        let hit = |cx: &Cx<'_, Self>, part: PartRef| {
            cx.part_if_declared(part).is_some_and(|result| result.press.phase.is_click())
                || (by_key == Some(part) && cx.part_if_declared(part).is_some())
        };
        for (at, action) in self.actions.iter().enumerate() {
            if hit(cx, ACTION.at(at as u64)) {
                self.taken = Some(at);
                if let Some(command) = &action.command {
                    commands::fire(&cx.ctx, command.clone(), FireSource::Pointer);
                }
            }
        }
        if hit(cx, DISMISS.into()) {
            self.dismissed = true;
        }
        if let Some(politeness) = self.announce {
            let first = {
                let announced = cx.memory::<Announced>();
                !std::mem::replace(&mut announced.0, true)
            };
            if first {
                let message = cx.resolved.label;
                let text = match self.title.as_ref().map(|title| cx.env.resolve(title)) {
                    Some(title) => format!("{}, {message}", title.as_str()),
                    None => message.to_owned(),
                };
                cx.announce(text, politeness);
            }
        }
    }

    fn access(&self, access: &mut Access, cx: &Cx<'_, Self>) {
        match (self.announce, self.tone) {
            (None, _) => {
                access.role(Role::LABEL);
            }
            (Some(_), Tone::Bad) => {
                access.role(Role::ALERT);
            }
            (Some(_), _) => {}
        }
        if let Some(title) = self.title.as_ref().map(|title| cx.env.resolve(title).as_str().to_owned()) {
            access.name(Name::Joined(vec![title.clone(), cx.resolved.label.to_owned()]));
            if self.layout == NoticeLayout::Centred {
                let node = access.node(TITLE, Role::HEADING);
                node.level(TITLE_LEVEL);
                node.name = Name::Text(title);
            }
        }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let Some(placed) = self.placed.clone() else { return };
        let words = cx.words::<NoticeWords>();
        for (at, label) in self.action_labels(&cx.ctx).into_iter().enumerate() {
            let part = ACTION.at(at as u64);
            if self.row() {
                ui.face(part, Link::new(label).role(cx.style.message.role));
            } else {
                ui.face(part, Button::new(label));
            }
        }
        if placed.dismiss.is_some() {
            ui.face(DISMISS, IconButton::new(IconId::new(DISMISS_ICON)).label(words.dismiss.clone()));
        }
        let draw = ui.draw();
        if let (Some(rect), Some(mut spec)) = (placed.icon, self.icon_spec()) {
            if spec.color.is_none() {
                spec.color = Some(cx.resolve(&cx.style.icon.color));
            }
            draw.icon(rect, &spec);
        }
        if let (Some(rect), Some(title)) = (placed.title, self.title.as_ref()) {
            let run = TextRun::new(cx.env.resolve(title).as_str().to_owned()).role(cx.style.title.role).wrap(Wrap::Wrap);
            draw.text(rect, &run.color(cx.resolve(&cx.style.title.color)));
        }
        let run = TextRun::new(cx.resolved.label).role(cx.style.message.role).wrap(Wrap::Wrap);
        if let Some(message) = placed.message {
            draw.text(message, &run.color(cx.resolve(&cx.style.message.color)));
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> NoticeOutput {
        NoticeOutput { response: cx.response.clone(), action: self.taken, dismissed: self.dismissed }
    }
}

/// The icon that marks `tone`; the neutral tone has none.
fn tone_icon(tone: Tone) -> Option<&'static str> {
    match tone {
        Tone::Info | Tone::Accent => Some("info"),
        Tone::Good => Some("success"),
        Tone::Warn => Some("warning"),
        Tone::Bad => Some("error"),
        _ => None,
    }
}
