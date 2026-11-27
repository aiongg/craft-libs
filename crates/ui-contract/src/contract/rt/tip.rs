//! Tooltips on craft-ui-rt: the delay and the hide run on ui-rt's timers, so a pointer resting on a
//! component runs no frame until the delay ends.
//!
//! A component with a tooltip asks ui-rt for hover events. The pointer entering it starts the
//! delay with a build wake at its end ([`EventCx::request_build_at`]); leaving cancels the wake.
//! At the wake the component runs and shows the tooltip as a layer child on the tooltip layer,
//! below it; the pointer leaving or a press runs it again without the tooltip, and a press keeps
//! it hidden until the pointer leaves.

use std::sync::Arc;

use crate::contract::Id;
use craft_theme::Tokens;
use craft_ui_rt::{Align, BuildCx, EventCx, Layer, LayerSpec, Placement, Rect, Side, Vec2};

use super::overlay::{LayerShow, Surface, show_layer};
use super::ui::Ui;
use crate::contract::{Access, Common, Component, Cx, MeasureCx, PaintUi, Response, Role, Show};
use crate::contract::{MemoKey, MemoProps, POPUP_PAD};
use crate::primitives::TextRun;
use crate::style::{ComponentStyle, StyleCx};
use craft_behavior::{Metrics, Sense};

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

/// A part's tooltip: the ui-rt part id it shows over, its text, and the part's rect in node coordinates, which the
/// tooltip stands under. A collection's members' parts carry their own (or the member's) this way.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PartTip {
    pub(crate) id: u64,
    pub(crate) text: String,
    pub(crate) rect: Rect,
}

/// A component's tooltip, kept on its node: its own text and its parts' tooltips.
#[derive(Clone, Debug, Default)]
pub(crate) struct TipKept {
    pub(crate) text: Option<String>,
    pub(crate) parts: Vec<PartTip>,
    /// The node's part under the pointer, as of the last hover event (0 for its own shape, `None` over a node under it).
    pub(crate) part: Option<u64>,
    pub(crate) phase: TipPhase,
}

impl TipKept {
    /// What shows while the pointer is over `part`: the part's tooltip and its rect, else the component's own text.
    fn text_for(&self, part: Option<u64>) -> Option<(&str, Option<Rect>)> {
        let own = || self.text.as_deref().map(|text| (text, None));
        part.and_then(|id| self.parts.iter().find(|tip| tip.id == id)).map(|tip| (tip.text.as_str(), Some(tip.rect))).or_else(own)
    }

    /// Starts the delay at this frame time.
    fn wait(&mut self, cx: &mut EventCx<'_>) {
        let since = cx.time();
        self.phase = TipPhase::Waiting { since };
        cx.request_build_at(Some(since + DELAY));
    }

    /// The pointer entered (`true`) or left the component.
    pub(crate) fn hover(&mut self, on: bool, cx: &mut EventCx<'_>) {
        self.part = if on { cx.runtime().hovered_part(cx.node()) } else { None };
        match (on, self.phase) {
            (true, TipPhase::Idle) if self.text_for(self.part).is_some() => self.wait(cx),
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

    /// The pointer moved to another part of the component (`Event::HoverPart`): a part with a tooltip of its own
    /// starts the delay, or changes the shown tooltip at once; leaving every tooltip ends it.
    pub(crate) fn hover_part(&mut self, part: Option<u64>, cx: &mut EventCx<'_>) {
        let before = self.text_for(self.part).map(|(text, rect)| (text.to_owned(), rect));
        self.part = part;
        let after = self.text_for(part).map(|(text, rect)| (text.to_owned(), rect));
        match self.phase {
            TipPhase::Idle if after.is_some() => self.wait(cx),
            TipPhase::Waiting { .. } if after.is_none() => {
                self.phase = TipPhase::Idle;
                cx.request_build_at(None);
            }
            TipPhase::Shown if after != before => {
                if after.is_none() {
                    self.phase = TipPhase::Idle;
                }
                cx.request_build();
            }
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

/// Takes the component's tooltip text and its parts' tooltips from its run, and shows the tooltip while it is due:
/// the hovered part's, under the part, else the component's own, under the component. Runs in the component's run
/// (`ran`) and in passes that reach a kept component's layer children.
#[allow(clippy::too_many_arguments)]
pub(crate) fn show(
    tip: &mut Option<TipKept>,
    text: Option<String>,
    parts: Vec<PartTip>,
    ran: bool,
    cx: &mut BuildCx<'_>,
    owner: Id,
    rect: Rect,
    tokens: &Arc<Tokens>,
) {
    if ran {
        if text.is_none() && parts.is_empty() {
            *tip = None;
        } else {
            let kept = tip.get_or_insert_with(TipKept::default);
            kept.text = text;
            kept.parts = parts;
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
    let Some((text, part_rect)) = kept.text_for(kept.part) else { return };
    let gap = tokens.space.gap.tight;
    let (placement, anchor) = match part_rect {
        Some(part) => {
            let part = part.translate(rect.min.to_vec2());
            (Placement::Beside { rect: part, side: Side::Below, align: Align::Min, gap }, part)
        }
        None => (Placement::Anchor { node: cx.node(), side: Side::Below, align: Align::Min, gap }, rect),
    };
    let show = LayerShow {
        layer: Layer::Tooltip,
        spec: LayerSpec::new(placement),
        surface: Surface::Tip,
        pad: POPUP_PAD.points(tokens),
        share: Vec2::new(0.5, 0.5),
        anchor: Some(anchor),
        role: accesskit::Role::Tooltip,
        hover: false,
        sweep: None,
        focus: crate::runtime::focus::FocusPolicy::NONE,
        entry: None,
        host: None,
        on_activate: false,
        enter: false,
    };
    let text = text.to_owned();
    show_layer(cx, owner.with("craft-tip"), TIP_KEY, show, ran, &mut |ui| {
        TipText::new(&text).show(ui);
    });
}

/// The look of TipText: its metrics only; the text takes the ink and the layer's frame paints the surface.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
struct TipTextStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// One line of text in a layer child, sized by the shaped text.
#[derive(crate::contract::Component)]
struct TipText<'a> {
    #[common]
    common: Common,
    text: &'a str,
}

impl<'a> TipText<'a> {
    fn new(text: &'a str) -> Self {
        TipText { common: Common::new(), text }
    }

    fn show(self, ui: &mut Ui<'_, '_>) {
        let _ = <Self as Show>::show(self, ui);
    }
}

impl Component for TipText<'_> {
    const ROLE: Role = Role::Node(accesskit::Role::Label);
    const PARTS: bool = false;
    const STOPS: bool = false;
    type Style = TipTextStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: TipTextStyleKey) -> TipTextStyle {
        TipTextStyle { metrics: Metrics::new() }
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    /// The shaped text, wrapped at the room: the size the layer child is placed by.
    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        let run = TextRun::new(self.text);
        let text = match cx.room {
            Some(width) => cx.layout_at(&run, width).size(),
            None => cx.layout(&run).size(),
        };
        Vec2::new(text.x.min(cx.room.unwrap_or(f32::INFINITY)), text.y)
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        ui.draw().text(cx.rect, &TextRun::new(self.text));
    }

    fn access(&self, access: &mut Access, _: &Cx<'_, Self>) {
        access.name(crate::runtime::access::Name::Text(self.text.to_owned()));
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}
