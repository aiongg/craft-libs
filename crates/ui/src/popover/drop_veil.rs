//! `DropVeil` (8.4, 12.4): a container over its body that takes files dragged in from the OS. While files hover it
//! dims the body, frames it and shows a card with the label; a drop is reported in the output.

use crate::contract::{
    Accessible as _, Common, Component, Container, ContainerProps, Cx, DropMark, DropOperation, NoValue, OsFiles, Outcome, Pad, PaintUi, PartsCx,
    Pointer as _, Politeness, Props as _, Remember as _, Role, Sizing, WindowDropTarget, WordSetCx as _,
};
use crate::display::Text;
use crate::runtime::drag;
use crate::runtime::env::Label;
use crate::style::{ComponentStyle, Corners, StyleCx, SurfaceKind, TypeRole};
use craft_behavior::{Metrics, Proposal, Sense};
use egui::{Rect, Response, Vec2};

#[cfg(feature = "testing")]
mod stories;

/// The style of a drop veil: the card's padding. The scrim, the frame and the card's surface come from the surface
/// kinds; the card's text is a `Text` child.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct DropVeilStyle {
    #[style(metrics)]
    metrics: Metrics,
    /// Between the card's edge and its text.
    pad: f32,
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

impl WindowDropTarget<OsFiles> for WholeWindow {
    type Drop = OsFiles;

    fn accept(&self, payload: &OsFiles, _op: DropOperation) -> Option<OsFiles> {
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
    /// The card and its text rect, from the text child's measured size.
    card: Option<(Rect, Rect)>,
}

impl DropVeil {
    /// A veil saying `label` ("Import 3 photos") while files hover.
    pub fn new(label: impl Into<Label>) -> Self {
        let props = ContainerProps::new().sizing(Sizing::Fill).pad(Pad::None);
        DropVeil { common: Common::new(), props, hovering: false, dropped: None, card: None }.label(label)
    }
}

/// The card's text.
fn card_text(label: Label) -> Text {
    Text::new(label).role(TypeRole::Title)
}

impl Component for DropVeil {
    const ROLE: Role = Role::GROUP;
    const OVERLAYS: bool = false;
    type Style = DropVeilStyle;
    type Output = DropVeilOutput;

    fn style(cx: &StyleCx<'_>, _: DropVeilStyleKey) -> DropVeilStyle {
        DropVeilStyle { metrics: Metrics::new(), pad: cx.tokens.space.pad.dialog }
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        let held = drag::active::<OsFiles>(cx.ui.ctx()).is_some_and(|files| !files.files.is_empty());
        self.card = held.then(|| {
            let pad = cx.style.pad;
            let room = (rect.width() - 2.0 * pad).max(0.0);
            let size = cx.child("card-text", &mut card_text(cx.label()), Proposal::Width(room));
            let card = Rect::from_center_size(rect.center(), size + Vec2::splat(2.0 * pad));
            (card, Rect::from_min_size(card.min + Vec2::splat(pad), size))
        });
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let held = drag::active::<OsFiles>(&cx.ctx).is_some_and(|files| !files.files.is_empty());
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
        if let Some((card, text)) = self.card {
            draw.surface(card, SurfaceKind::Popup, Corners::All);
            ui.child("card-text", text, card_text(cx.label()));
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> DropVeilOutput {
        DropVeilOutput { response: cx.response.clone(), dropped: self.dropped }
    }
}

impl Container for DropVeil {}
