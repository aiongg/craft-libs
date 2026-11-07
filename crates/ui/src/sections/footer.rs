//! `FooterBar` (16.4): the container that fills a container's footer region.

use super::words::SectionWords;
use crate::buttons::{Button, IconButton};
use crate::contract::{
    Access, BodyLayout, Common, Component, Container, ContainerProps, Cx, Props as _, Role, RowAlign, Show as _, ShowContainer as _, WordSetCx as _,
};
use crate::primitives::IconSpec;
use crate::runtime::access::Name;
use crate::runtime::env::Label;
use crate::runtime::focus;
use crate::runtime::{keys, overlay};
use crate::style::{ComponentStyle, StyleCx};
use craft_behavior::{Metrics, NavShape, OwnerClass, Roving, Sense};
use craft_host::target::Os;
use craft_keymap::{Chord, Key};
use craft_theme::Variant;
use egui::{Context, Response, Ui};

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

/// An icon action of a panel footer: an `IconButton` named by its label.
#[derive(Clone, Debug)]
pub struct FooterAction {
    icon: IconSpec<'static>,
    label: Label,
}

impl FooterAction {
    /// An action showing `icon`, named `label`.
    pub fn new(icon: impl Into<IconSpec<'static>>, label: impl Into<Label>) -> Self {
        FooterAction { icon: icon.into(), label: label.into() }
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

/// Whether a bare letter reaches the footer being shown: focus is inside the layer whose body shows it, on a
/// component that does not own text keys.
fn letters_reach_footer(ctx: &Context) -> bool {
    let Some(layer) = overlay::drawing_parent(ctx) else { return false };
    focus::focus_in_scope(ctx, layer) == Some(true) && focus::focused_rec(ctx).is_some_and(|rec| rec.stop.keys.owner == OwnerClass::Widget)
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
    pub fn show_content(mut self, ui: &mut Ui) -> Option<usize> {
        let content = std::mem::take(&mut self.content);
        let picked = self.show(ui, |ui, _| match &content {
            Content::Body => None,
            Content::Actions(actions) => {
                actions.iter().position(|action| crate::contract::clicked(&IconButton::new(action.icon.clone()).label(action.label.clone()).show(ui)))
            }
            Content::Buttons(buttons) => {
                let roles: Vec<DialogRole> = buttons.iter().map(|button| button.role).collect();
                let mut picked = None;
                let (ctx, owner) = (ui.ctx().clone(), ui.id());
                let lettered = letters_reach_footer(&ctx);
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
                    if crate::contract::clicked(&widget.show(ui)) {
                        picked = Some(at);
                    }
                    let chord = button.letter.and_then(Key::from_char).map(Chord::plain);
                    if let Some(chord) = chord.filter(|_| lettered && button.enabled) {
                        keys::capture(&ctx, owner, chord);
                        if keys::take_claimed(&ctx, owner, chord) {
                            picked = Some(at);
                        }
                    }
                }
                picked
            }
        });
        picked.inner.flatten()
    }
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
