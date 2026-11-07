//! `FooterBar` (16.4): the container that fills a container's footer region.

use super::words::SectionWords;
use crate::buttons::{Button, IconButton};
use crate::contract::StopProps as _;
use crate::contract::{
    Access, BodyLayout, Common, Component, Container, ContainerProps, Cx, DropMark, DropOperation, DropTarget, NoValue, Outcome, Pad, Pointer as _, Props as _,
    Role, RowAlign, Show as _, ShowContainer as _, Sizing, WordSetCx as _,
};
use crate::primitives::IconSpec;
use crate::runtime::access::Name;
use crate::runtime::env::Label;
use crate::runtime::focus;
use crate::runtime::keys;
use crate::style::{ComponentStyle, StyleCx};
use craft_behavior::{Metrics, NavShape, OwnerClass, Roving, Sense};
use craft_host::target::Os;
use craft_keymap::{Chord, Key};
use craft_theme::Variant;
use egui::{Context, Pos2, Rect, Response, Ui};
use std::any::Any;
use std::fmt;
use std::sync::Arc;

#[cfg(feature = "testing")]
mod stories;

/// The style of a footer bar: no look of its own.
#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct FooterBarStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// A footer region's content: its body is the caller's closure, and its members are one roving group.
#[derive(crate::contract::Component)]
#[component(container)]
pub struct FooterBar {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    content: Content,
}

/// The payload a dropped-on action reports, type-erased so actions of different payload types share one footer.
type Dropped = Arc<dyn Any + Send + Sync>;

/// Shows an action's drop slot around its button and returns the button's response and the payload dropped on it.
type DropShow = dyn Fn(&mut Ui, &FooterAction) -> (Response, Option<Dropped>) + Send + Sync;

/// An icon action of a panel footer: an `IconButton` named by its label.
#[derive(Clone)]
pub struct FooterAction {
    icon: IconSpec<'static>,
    label: Label,
    enabled: bool,
    drops: Option<Arc<DropShow>>,
}

impl fmt::Debug for FooterAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FooterAction")
            .field("icon", &self.icon)
            .field("label", &self.label)
            .field("enabled", &self.enabled)
            .field("accepts_drops", &self.drops.is_some())
            .finish()
    }
}

impl FooterAction {
    /// An action showing `icon`, named `label`.
    pub fn new(icon: impl Into<IconSpec<'static>>, label: impl Into<Label>) -> Self {
        FooterAction { icon: icon.into(), label: label.into(), enabled: true, drops: None }
    }

    /// Whether a click or the keyboard can activate the action. A disabled action that accepts drops still takes an
    /// accepted drop: Delete with nothing selected still deletes a dragged row.
    #[must_use]
    pub fn enabled(mut self, on: bool) -> Self {
        self.enabled = on;
        self
    }

    /// Lets drags carrying a `P` that `accepts` takes drop on the action. The action shows the drop outline while
    /// such a drag is over it, and the drop comes back in [`FooterOutput::dropped`].
    #[must_use]
    pub fn accepts_drops<P: Clone + Send + Sync + 'static>(mut self, accepts: fn(&P) -> bool) -> Self {
        self.drops = Some(Arc::new(move |ui, action| {
            let shown = DropAction::new(accepts).show(ui, |ui, _| action.button().show(ui));
            let response = shown.inner.unwrap_or_else(|| shown.output.response.clone());
            (response, shown.output.dropped.map(|payload| Arc::new(payload) as Dropped))
        }));
        self
    }

    fn button(&self) -> IconButton<'static> {
        IconButton::new(self.icon.clone()).label(self.label.clone()).enabled(self.enabled)
    }
}

/// A drop slot around one footer action: it takes the drags its predicate accepts over the action's rect.
#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct DropActionStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// What a drop slot reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct DropActionOutput<P> {
    /// The slot's response.
    pub response: Response,
    /// The payload dropped on the slot this pass.
    pub dropped: Option<P>,
}

impl<P> Outcome for DropActionOutput<P> {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// A container that is as large as its one child and takes drops of payload type `P` over that child.
#[derive(crate::contract::Component)]
#[component(container, probe(DropAction<u8>))]
pub struct DropAction<P: Clone + Send + Sync + 'static> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    accepts: fn(&P) -> bool,
    dropped: Option<P>,
}

impl<P: Clone + Send + Sync + 'static> DropAction<P> {
    /// A slot that takes the payloads `accepts` takes.
    pub fn new(accepts: fn(&P) -> bool) -> Self {
        let props = ContainerProps::new().pad(Pad::None).sizing(Sizing::Content);
        DropAction { common: Common::new(), props, accepts, dropped: None }
    }
}

/// Where a drop on the slot lands: the slot itself.
struct OnSlot<P> {
    rect: Rect,
    accepts: fn(&P) -> bool,
}

impl<P: Clone + Send + Sync + 'static> DropTarget<P> for OnSlot<P> {
    type Drop = P;

    fn accept(&self, payload: &P, _: Pos2, _: DropOperation) -> Option<P> {
        (self.accepts)(payload).then(|| payload.clone())
    }

    fn mark(&self, _: &P) -> DropMark {
        DropMark::Into(self.rect)
    }
}

impl<P: Clone + Send + Sync + 'static> Component for DropAction<P> {
    const ROLE: Role = Role::GROUP;
    type Style = DropActionStyle;
    type Output = DropActionOutput<P>;

    fn style(_: &StyleCx<'_>, _: DropActionStyleKey<P>) -> DropActionStyle {
        DropActionStyle::default()
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let target = OnSlot { rect: cx.rect, accepts: self.accepts };
        self.dropped = cx.drop_target(&target);
    }

    fn finish(self, cx: Cx<'_, Self>) -> DropActionOutput<P> {
        DropActionOutput { response: cx.response.clone(), dropped: self.dropped }
    }
}

impl<P: Clone + Send + Sync + 'static> Container for DropAction<P> {}

/// What a footer reports: the action or button clicked, the drop an action took, and each action's response.
#[derive(Debug)]
#[non_exhaustive]
pub struct FooterOutput {
    /// The footer's response.
    pub response: Response,
    /// The index, in the order given, of the action or button activated this pass.
    pub picked: Option<usize>,
    /// The drop an action took this pass.
    pub dropped: Option<FooterDrop>,
    /// The response of each action, in the order given; a caller opens a menu from one with `Menu::opened_by`.
    pub responses: Vec<Response>,
}

impl Outcome for FooterOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// A payload dropped on a footer action.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct FooterDrop {
    /// The index of the action it was dropped on, in the order given.
    pub index: usize,
    payload: Dropped,
}

impl FooterDrop {
    /// The payload, when it is a `P`.
    pub fn payload<P: 'static>(&self) -> Option<&P> {
        self.payload.downcast_ref::<P>()
    }
}

/// What a dialog button means; it decides the button's look and its place in the row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DialogRole {
    /// The default outcome: the primary look, last in the row (first on Windows).
    Accept,
    /// An outcome that destroys something: the danger look.
    Destructive,
    /// Leaves without an outcome.
    Cancel,
    /// Any other outcome.
    Other,
}

impl DialogRole {
    /// The position in the row: later roles stand nearer the end. Windows puts the accepting button first and the
    /// others after it; macOS and Linux put it last.
    fn rank(self, windows: bool) -> u8 {
        let rank = match self {
            DialogRole::Other => 0,
            DialogRole::Destructive => 1,
            DialogRole::Cancel => 2,
            DialogRole::Accept => 3,
        };
        if windows { 3 - rank } else { rank }
    }
}

/// A button of a dialog footer.
#[derive(Clone, Debug)]
pub struct FooterButton {
    label: Label,
    role: DialogRole,
    letter: Option<char>,
    enabled: bool,
}

impl FooterButton {
    /// The meaning of the button.
    pub(crate) fn role(&self) -> DialogRole {
        self.role
    }

    /// A button named `label` with the meaning `role`.
    pub fn new(label: impl Into<Label>, role: DialogRole) -> Self {
        FooterButton { label: label.into(), role, letter: None, enabled: true }
    }

    /// The letter that activates the button while its dialog holds focus and no text part owns the keys. The
    /// button reports it as its `access_key`.
    #[must_use]
    pub const fn letter(mut self, letter: char) -> Self {
        self.letter = Some(letter);
        self
    }

    /// Whether the button can be activated; a disabled button ignores its letter too.
    #[must_use]
    pub const fn enabled(mut self, on: bool) -> Self {
        self.enabled = on;
        self
    }
}

/// Whether a bare letter reaches the footer being shown: focus is inside the focus scope the footer sits in (the
/// dialog's, whether it is a layer or a full window), on a component that does not own text keys.
fn letters_reach_footer(ctx: &Context) -> bool {
    let Some(scope) = focus::open_scope(ctx) else { return false };
    focus::focus_in_scope(ctx, scope) == Some(true) && focus::focused_rec(ctx).is_some_and(|rec| rec.stop.keys.owner == OwnerClass::Widget)
}

#[derive(Default)]
enum Content {
    #[default]
    Body,
    Actions(Vec<FooterAction>),
    Buttons(Vec<FooterButton>),
}

/// The indexes of `roles` in display order: by role rank, and in the given order within a rank.
pub(crate) fn display_order(roles: &[DialogRole], windows: bool) -> Vec<usize> {
    let mut order: Vec<usize> = (0..roles.len()).collect();
    order.sort_by_key(|&at| roles[at].rank(windows));
    order
}

impl FooterBar {
    /// A footer bar whose content is its body.
    pub fn body() -> Self {
        FooterBar {
            common: Common::new(),
            props: ContainerProps::new().roving(Roving::new(NavShape::Flat)).layout(BodyLayout::row().align(RowAlign::End)),
            content: Content::Body,
        }
    }

    /// A footer bar of icon actions, in the given order. Show it with `show_content`.
    pub fn actions(actions: impl IntoIterator<Item = FooterAction>) -> Self {
        FooterBar { content: Content::Actions(actions.into_iter().collect()), ..FooterBar::body() }
    }

    /// A footer bar of dialog buttons, in the order of the platform's dialog convention. Show it with
    /// `show_content`.
    pub fn buttons(buttons: impl IntoIterator<Item = FooterButton>) -> Self {
        FooterBar { content: Content::Buttons(buttons.into_iter().collect()), ..FooterBar::body() }
    }

    /// Shows the bar with the actions or buttons it was made with and returns the index, in the order they were
    /// given, of the one clicked this pass.
    pub fn show_content(self, ui: &mut Ui) -> Option<usize> {
        self.show_footer(ui).picked
    }

    /// Shows the bar with the actions or buttons it was made with and reports what happened to them.
    pub fn show_footer(mut self, ui: &mut Ui) -> FooterOutput {
        let content = std::mem::take(&mut self.content);
        let shown = self.show(ui, |ui, _| match &content {
            Content::Body => Shown::default(),
            Content::Actions(actions) => {
                let mut shown = Shown::default();
                for (at, action) in actions.iter().enumerate() {
                    let response = match &action.drops {
                        Some(drops) => {
                            let (response, dropped) = drops(ui, action);
                            if let Some(payload) = dropped {
                                shown.dropped = Some(FooterDrop { index: at, payload });
                            }
                            response
                        }
                        None => action.button().show(ui),
                    };
                    if crate::contract::clicked(&response) {
                        shown.picked = Some(at);
                    }
                    shown.responses.push(response);
                }
                shown
            }
            Content::Buttons(buttons) => {
                let roles: Vec<DialogRole> = buttons.iter().map(|button| button.role).collect();
                let mut shown = Shown::default();
                let (ctx, owner) = (ui.ctx().clone(), ui.scope_id());
                let lettered = letters_reach_footer(&ctx);
                // Initial focus lands on the first enabled Accept button on every OS.
                let default_at = buttons.iter().position(|button| button.role == DialogRole::Accept && button.enabled);
                for at in display_order(&roles, crate::runtime::windows::os_of(ui.ctx()) == Os::Windows) {
                    let button = &buttons[at];
                    let variant = match button.role {
                        DialogRole::Accept => Variant::PRIMARY,
                        DialogRole::Destructive => Variant::DANGER,
                        DialogRole::Cancel | DialogRole::Other => Variant::SECONDARY,
                    };
                    let mut widget = Button::new(button.label.clone()).variant(variant).enabled(button.enabled);
                    if let Some(letter) = button.letter {
                        widget = widget.access_key(letter.to_ascii_uppercase());
                    }
                    if default_at == Some(at) {
                        widget = widget.default_stop();
                    }
                    if crate::contract::clicked(&widget.show(ui)) {
                        shown.picked = Some(at);
                    }
                    let chord = button.letter.and_then(Key::from_char).map(Chord::plain);
                    if let Some(chord) = chord.filter(|_| lettered && button.enabled) {
                        keys::capture(&ctx, owner, chord);
                        if keys::take_claimed(&ctx, owner, chord) {
                            shown.picked = Some(at);
                        }
                    }
                }
                shown
            }
        });
        let inner = shown.inner.unwrap_or_default();
        FooterOutput { response: shown.output, picked: inner.picked, dropped: inner.dropped, responses: inner.responses }
    }
}

/// What the body of a footer collected in one pass.
#[derive(Default)]
struct Shown {
    picked: Option<usize>,
    dropped: Option<FooterDrop>,
    responses: Vec<Response>,
}

impl Component for FooterBar {
    const ROLE: Role = Role::GROUP;
    type Style = FooterBarStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: FooterBarStyleKey) -> FooterBarStyle {
        FooterBarStyle::default()
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    /// A footer bar the caller did not label is named by the words of the family.
    fn access(&self, access: &mut Access, cx: &Cx<'_, Self>) {
        if cx.resolved.label.is_empty() {
            access.name(Name::Text(cx.words::<SectionWords>().footer.as_str().to_owned()));
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

impl Container for FooterBar {}
