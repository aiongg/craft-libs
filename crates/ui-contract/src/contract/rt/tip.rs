//! Tooltips on craft-ui-rt: the delay and the hide run on ui-rt's timers, so a pointer resting on a
//! component runs no frame until the delay ends.
//!
//! A component with a tooltip asks ui-rt for hover events. The pointer entering it starts the
//! delay with a build wake at its end ([`EventCx::request_build_at`]); leaving cancels the wake.
//! At the wake the component runs and shows the tooltip as a layer child on the tooltip layer,
//! below it; the pointer leaving or a press runs it again without the tooltip, and a press keeps
//! it hidden until the pointer leaves.

use std::sync::Arc;

use craft_theme::Tokens;
use craft_ui_rt::{Align, BuildCx, EventCx, Layer, LayerSpec, Placement, Rect, Side, Vec2};
use egui::Id;

use super::key::ComponentKey;
use super::overlay::{LayerShow, Surface, show_layer};
use super::run::{Component, Cx, MeasureCx, show_leaf};
use super::ui::Ui;
use crate::contract::{MemoKey, MemoProps, POPUP_PAD};

pub use crate::contract::tooltip::DELAY;

/// Where a component's tooltip stands.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) enum TipPhase {
    /// The pointer is not on the component.
    #[default]
    Idle,
    /// The pointer rests on it since this frame time; the tooltip shows at `since + DELAY`.
    Waiting { since: f64 },
    /// The tooltip shows.
    Shown,
    /// A press on the component hid it; it stays hidden until the pointer leaves.
    Pressed,
}

/// A component's tooltip, kept on its node.
#[derive(Clone, Debug, Default)]
pub(crate) struct TipKept {
    pub(crate) text: String,
    pub(crate) phase: TipPhase,
}

impl TipKept {
    /// The pointer entered (`true`) or left the component.
    pub(crate) fn hover(&mut self, on: bool, cx: &mut EventCx<'_>) {
        match (on, self.phase) {
            (true, TipPhase::Idle) => {
                let since = cx.time();
                self.phase = TipPhase::Waiting { since };
                cx.request_build_at(Some(since + DELAY));
            }
            (false, TipPhase::Waiting { .. }) => {
                self.phase = TipPhase::Idle;
                cx.request_build_at(None);
            }
            (false, TipPhase::Shown) => {
                self.phase = TipPhase::Idle;
                cx.request_build();
            }
            (false, TipPhase::Pressed) => self.phase = TipPhase::Idle,
            _ => {}
        }
    }

    /// A press went down on the component.
    pub(crate) fn pressed(&mut self, cx: &mut EventCx<'_>) {
        match self.phase {
            TipPhase::Waiting { .. } => cx.request_build_at(None),
            TipPhase::Shown => cx.request_build(),
            _ => {}
        }
        if self.phase != TipPhase::Idle {
            self.phase = TipPhase::Pressed;
        }
    }
}

/// The tooltip key among a node's layer children.
const TIP_KEY: u64 = 0x7469_70;

/// Takes the component's tooltip text from its run, and shows the tooltip while it is due. Runs in
/// the component's run and in passes that reach a kept component's layer children.
pub(crate) fn show(tip: &mut Option<TipKept>, text: Option<String>, ran: bool, cx: &mut BuildCx<'_>, owner: Id, rect: Rect, tokens: &Arc<Tokens>) {
    if ran {
        match (tip.as_mut(), text) {
            (Some(kept), Some(text)) => kept.text = text,
            (None, Some(text)) => *tip = Some(TipKept { text, phase: TipPhase::Idle }),
            (_, None) => *tip = None,
        }
        cx.hover_events(tip.is_some());
    }
    let Some(kept) = tip.as_mut() else { return };
    if let TipPhase::Waiting { since } = kept.phase
        && cx.now() >= since + DELAY
    {
        kept.phase = TipPhase::Shown;
    }
    if kept.phase != TipPhase::Shown {
        return;
    }
    let gap = tokens.space.gap.tight;
    let spec = LayerSpec::new(Placement::Anchor { node: cx.node(), side: Side::Below, align: Align::Min, gap });
    let show = LayerShow {
        layer: Layer::Tooltip,
        spec,
        surface: Surface::Tip,
        pad: POPUP_PAD.points(tokens),
        share: Vec2::new(0.5, 0.5),
        anchor: Some(rect),
        role: accesskit::Role::Tooltip,
        hover: false,
    };
    let text = kept.text.clone();
    show_layer(cx, owner.with("craft-tip"), TIP_KEY, show, ran, &mut |ui| {
        TipText { text: &text }.show(ui);
    });
}

/// The text of a tooltip. Its size is estimated from the text until craft text runs on ui-rt.
struct TipText<'a> {
    text: &'a str,
}

impl TipText<'_> {
    fn show(self, ui: &mut Ui<'_, '_>) {
        show_leaf(self, ui);
    }
}

impl ComponentKey for TipText<'_> {
    fn key(&self, key: &mut MemoKey) {
        self.text.key(key);
    }
}

impl Component for TipText<'_> {
    const ROLE: accesskit::Role = accesskit::Role::Label;
    type Output = ();

    fn measure(&mut self, cx: &MeasureCx<'_>) -> Vec2 {
        let size = cx.tokens.size.icon.md;
        let chars = self.text.chars().count() as f32;
        Vec2::new((chars * size * 0.55).min(cx.room.x), size * 1.3)
    }

    fn access(&self, node: &mut accesskit::Node, _: &Cx<'_>) {
        node.set_label(self.text);
    }

    fn finish(self, _: &Cx<'_>) {}
}
