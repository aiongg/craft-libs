//! `contract::tooltip`: the one step-16 function that shows every tooltip (3.4, 15.5).
//!
//! A tooltip shows after one delay while the pointer rests on the component, or on keyboard focus after the same
//! delay, and at once while a neighbour's tooltip was hiding a moment ago. A press, a menu that owns keys and a
//! running drag hide it.

use super::overlay::{self, Anchor, Layer, Overlay, Place};
use super::{Common, Component, Cx, Props as _, Show, TipFn, TooltipAt, TooltipWhen};
use crate::primitives::{TextRun, Wrap};
use crate::runtime::egui::memory::{self as store, Service};
use crate::runtime::egui::{areas, pass, repaint};
use crate::runtime::env::Env;
use crate::runtime::memory::{self, sub};
use crate::runtime::motion;
use crate::runtime::{frame, pointer};
use crate::style::{ComponentStyle, Flags, StyleColor, StyleCx, SurfaceKind, SurfaceLook};
use craft_behavior::overlay::{Align, Side};
use craft_behavior::{Metrics, OwnerClass};
use craft_theme::Stateful;
use egui::{Rect, Ui, Vec2};

/// Seconds the pointer rests on a component before its tooltip shows.
pub const DELAY: f64 = 0.6;
/// Seconds after a tooltip hides during which the next one shows at once.
pub const GRACE: f64 = 0.3;

#[derive(Clone, Copy, Default)]
struct TipState {
    since: Option<f64>,
    shown: bool,
}

/// When the last tooltip of the context hid.
#[derive(Default)]
struct Grace(Option<f64>);

/// What step 16 hands the tooltip: the component's facts and the pass's `Ui`.
pub(crate) struct TipIn<'a> {
    pub ui: &'a Ui,
    pub env: &'a Env,
    /// The rich content closure of `show_with_tip`.
    pub rich: Option<TipFn<'a>>,
}

/// The text a tooltip shows, or `None` when the component has none: the tooltip's text, or its disabled text while
/// disabled, with the bound command's chord; else the full label when the paint cut it.
fn text_of<C: Component>(cx: &Cx<'_, C>) -> Option<String> {
    let core = &cx.core;
    let resolved = core.resolved;
    let spec = resolved.tooltip.map(|tooltip| tooltip.spec());
    let base = match spec {
        Some(spec) => {
            let label = match (&spec.disabled_text, resolved.enabled) {
                (Some(disabled), false) => disabled,
                _ => &spec.text,
            };
            Some(core.env.resolve(label).as_str().to_owned())
        }
        None => core.paint.borrow().elided.clone(),
    }?;
    let platform = craft_keymap::egui::platform(&core.ctx);
    Some(match core.command.as_deref().and_then(|command| command.chord.as_ref()) {
        Some(chord) => format!("{base} ({})", chord.display(platform)),
        None => base,
    })
}

/// Step 16, after the component's report: advances the component's tooltip timer and shows its tooltip.
pub(crate) fn run<C: Component>(cx: &Cx<'_, C>, input: TipIn<'_>) {
    let core = &cx.core;
    let ctx = &core.ctx;
    let (id, now) = (core.id, frame::clock(ctx));
    let state_id = sub(id, "tip_state");
    let text = text_of(cx);
    let has_content = text.is_some() || input.rich.is_some();
    let mut state = store::take::<TipState>(ctx, state_id).unwrap_or_else(|| {
        memory::adopt::<TipState>(ctx, core.memory.owner(), state_id);
        TipState::default()
    });
    let spec = core.resolved.tooltip.map(|tooltip| tooltip.spec());
    let when_now = spec.is_some_and(|spec| spec.when == TooltipWhen::Now);
    let keyboard = core.state.flags.contains(Flags::FOCUS_VISIBLE);
    let wants = has_content && (core.response.hovered() || keyboard);
    let suppressed = core.press.phase.is_press() || overlay_blocks(ctx) || pointer::press_captured(ctx);
    if wants {
        state.since.get_or_insert(now);
    } else {
        state.since = None;
    }
    let due = state.since.map(|since| since + if when_now { 0.0 } else { DELAY });
    let in_grace = Service::<Grace>::of(ctx).with(|grace| grace.0.is_some_and(|hid| now - hid < GRACE));
    let show = wants && !suppressed && due.is_some_and(|due| now >= due || in_grace);
    if let (Some(due), true, false) = (due, wants, show)
        && !suppressed
        && due > now
    {
        repaint::request_after(ctx, pass::current_viewport(ctx), due - now);
    }
    if state.shown && !show {
        Service::<Grace>::of(ctx).with(|grace| grace.0 = Some(now));
    }
    state.shown = show;
    store::put(ctx, state_id, state);
    let tip_id = sub(id, "tip");
    let presence = motion::layer_presence(ctx, tip_id, core.memory.owner(), input.env.motion, show, overlay_motion());
    if !presence.visible {
        return;
    }
    let at_pointer = spec.is_some_and(|spec| spec.at == TooltipAt::Pointer);
    let place = if at_pointer {
        Place::anchored(Anchor::Pointer, Side::Below, Align::Start)
    } else {
        Place::anchored(Anchor::Rect(core.rect), Side::Below, Align::Start)
    };
    let tooltip = Overlay::tooltip().place(place);
    let to_screen = areas::layer_to_screen(ctx, input.ui.layer_id());
    let layer = Layer { id: tip_id, overlay: &tooltip, anchor: to_screen * core.rect, env: input.env, exiting: !show, fade: presence.t };
    let mut rich = input.rich;
    overlay::host(ctx, &layer, &mut |ui| match rich.take() {
        Some(content) => content(ui),
        None => {
            if let Some(text) = &text {
                TipText { common: Common::default(), text }.id_salt(tip_id).show(ui);
            }
        }
    });
}

fn overlay_motion() -> motion::MotionSpec {
    motion::spec::OPEN_FADE
}

/// Whether a menu owns the keys: tooltips stay hidden then.
fn overlay_blocks(ctx: &egui::Context) -> bool {
    crate::runtime::overlay::class_open(ctx, OwnerClass::Menu)
}

/// The look of a tooltip card.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct TipStyle {
    #[style(surface)]
    surface: SurfaceLook,
    #[style(metrics)]
    metrics: Metrics,
}

/// The text of a tooltip, on a popup surface.
#[derive(crate::contract::Component)]
pub(crate) struct TipText<'a> {
    #[common]
    common: Common,
    text: &'a str,
}

impl Component for TipText<'_> {
    const ROLE: super::Role = super::Role::TOOLTIP;
    const INPUT: craft_behavior::Input = craft_behavior::Input::Display;
    type Style = TipStyle;
    type Output = egui::Response;

    fn style(cx: &StyleCx<'_>, _: TipTextStyleKey) -> TipStyle {
        let fill = StyleColor(cx.surface(SurfaceKind::Popup).fill);
        TipStyle { surface: SurfaceLook::new(Stateful::new(fill)).kind(SurfaceKind::Popup), metrics: cx.control().metrics() }
    }

    fn measure(&mut self, cx: &super::MeasureCx<'_, Self>) -> Vec2 {
        cx.layout(&TextRun::new(self.text).wrap(Wrap::Line)).size()
    }

    fn sense(&self) -> craft_behavior::Sense {
        craft_behavior::Sense::empty()
    }

    fn paint(&mut self, ui: &mut super::PaintUi<'_>, cx: &Cx<'_, Self>) {
        let rect: Rect = cx.content_rect;
        ui.draw().text(rect, &TextRun::new(self.text));
    }

    fn finish(self, cx: Cx<'_, Self>) -> egui::Response {
        cx.response.clone()
    }
}
