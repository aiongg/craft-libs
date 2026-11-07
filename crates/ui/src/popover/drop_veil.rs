//! `DropVeil` (8.4, 12.4): a container over its body that takes files dragged in from the OS. While files hover it
//! dims the body, frames it and shows a card with the label; a drop is reported in the output.

use crate::contract::{
    Accessible as _, Common, Component, Container, ContainerProps, Cx, DropMark, DropOperation, DropTarget, NoValue, OsFiles, Outcome, Pad, PaintUi,
    Pointer as _, Politeness, Props as _, Remember as _, Role, Sizing, Styled as _, WordSetCx as _,
};
use crate::primitives::TextRun;
use crate::runtime::env::Label;
use crate::style::{ComponentStyle, Corners, Ink, StyleCx, SurfaceKind, TextLook, TypeRole};
use craft_behavior::{Metrics, Sense};
use craft_theme::Stateful;
use egui::{Pos2, Rect, Response, Vec2};

#[cfg(feature = "testing")]
mod stories;

/// The style of a drop veil: the card's padding and text. The scrim, the frame and the card's surface come from the
/// surface kinds.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct DropVeilStyle {
    #[style(metrics)]
    metrics: Metrics,
    /// Between the card's edge and its text.
    pad: f32,
    /// The card's text.
    #[style(text)]
    text: TextLook,
}

/// What a drop veil reports: its response and the files dropped on it in this pass.
#[derive(Debug)]
#[non_exhaustive]
pub struct DropVeilOutput {
    /// The veil's response.
    pub response: Response,
    /// The files dropped anywhere in the window in this pass.
    pub dropped: Option<OsFiles>,
}

impl Outcome for DropVeilOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// The accepting side of a veil: any files, anywhere in the window, by any operation.
struct WholeWindow {
    rect: Rect,
}

impl DropTarget<OsFiles> for WholeWindow {
    type Drop = OsFiles;

    fn accept(&self, payload: &OsFiles, _at: Pos2, _op: DropOperation) -> Option<OsFiles> {
        (!payload.files.is_empty()).then(|| payload.clone())
    }

    fn mark(&self, _drop: &OsFiles) -> DropMark {
        DropMark::Into(self.rect)
    }
}

/// Whether the hover of the current files has been announced.
#[derive(Clone, Default)]
struct Announced(bool);

/// A body that dims and shows `label` on a card while the OS drags files over the window, and reports the files
/// dropped on the window.
#[derive(crate::contract::Component)]
#[component(container)]
pub struct DropVeil {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    hovering: bool,
    dropped: Option<OsFiles>,
}

impl DropVeil {
    /// A veil saying `label` ("Import 3 photos") while files hover.
    pub fn new(label: impl Into<Label>) -> Self {
        let props = ContainerProps::new().sizing(Sizing::Fill).pad(Pad::None);
        DropVeil { common: Common::new(), props, hovering: false, dropped: None }.label(label)
    }
}

impl Component for DropVeil {
    const ROLE: Role = Role::GROUP;
    const OVERLAYS: bool = false;
    type Style = DropVeilStyle;
    type Output = DropVeilOutput;

    fn style(cx: &StyleCx<'_>, _: DropVeilStyleKey) -> DropVeilStyle {
        DropVeilStyle {
            metrics: Metrics::new(),
            pad: cx.tokens.space.pad.dialog,
            text: TextLook::new(TypeRole::Title, Stateful::new(cx.ink(Ink::OnPanel))),
        }
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let held = cx.dragging::<OsFiles>().is_some_and(|(files, _)| !files.files.is_empty());
        let target = WholeWindow { rect: cx.rect };
        self.dropped = cx.window_drop_target::<OsFiles, _>(&target);
        self.hovering = held && self.dropped.is_none();
        let announced = cx.memory::<Announced>();
        let first = self.hovering && !announced.0;
        announced.0 = self.hovering;
        if first {
            let text = cx.env.resolve(&cx.label()).as_str().to_owned();
            cx.announce(text, Politeness::Polite);
        }
    }

    fn paint_over(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        if !self.hovering {
            return;
        }
        let draw = ui.draw();
        draw.surface(cx.rect, SurfaceKind::Scrim, Corners::None);
        let pad = cx.style.pad;
        let text = cx.env.resolve(&cx.label());
        let run = TextRun::new(text.as_str()).role(cx.style.text.role).color(cx.resolve(&cx.style.text.color));
        let layout = draw.layout(&run, (cx.rect.width() - 2.0 * pad).max(0.0));
        let card = Rect::from_center_size(cx.rect.center(), layout.size() + Vec2::splat(2.0 * pad));
        draw.surface(card, SurfaceKind::Popup, Corners::All);
        draw.galley(card.min + Vec2::splat(pad), &layout);
    }

    fn finish(self, cx: Cx<'_, Self>) -> DropVeilOutput {
        DropVeilOutput { response: cx.response.clone(), dropped: self.dropped }
    }
}

impl Container for DropVeil {}
