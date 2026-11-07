//! The toast host (15.5): one layer per viewport that shows each queued toast as a card.

use super::overlay::{self, Anchor, Layer, Overlay, Place};
use super::{Common, Component, Cx, MeasureCx, PaintUi, Props as _, Role, Show};
use crate::primitives::{TextRun, Wrap};
use crate::runtime::egui::memory as store;
use crate::runtime::env::Env;
use crate::runtime::memory::sub;
use crate::runtime::motion;
use crate::runtime::toast;
use crate::style::{ComponentStyle, StyleColor, StyleCx, SurfaceKind, SurfaceLook};
use craft_behavior::Metrics;
use craft_behavior::overlay::{Align, Side};
use craft_theme::Stateful;
use egui::{Align2, Rect, Ui, Vec2};

/// The texts of the cards shown last, so the layer's exit fade still shows them after the queue emptied.
#[derive(Clone, Default)]
struct Shown(Vec<String>);

/// The layer that shows the toast queue of a viewport. The shell calls `show` once per pass.
#[derive(Clone, Copy, Debug, Default)]
pub struct ToastHost;

impl ToastHost {
    /// Shows every queued toast of the running viewport, oldest at the top, in the viewport's end corner. A card the
    /// pointer rests on keeps its timer paused; the first show of a toast announces it. When the queue empties the
    /// layer fades out with the last cards.
    pub fn show(ui: &Ui) {
        let ctx = ui.ctx();
        let cards = toast::cards(ctx);
        let env = Env::of(ui);
        let id = sub(store::unique("craft_ui.toast_host"), crate::runtime::egui::pass::current_viewport(ctx));
        let open = !cards.is_empty();
        let presence = motion::layer_presence(ctx, id, id, env.motion, open, motion::spec::OPEN_FADE);
        if !presence.visible {
            overlay::forget_layer(ctx, id, id);
            let _ = store::take::<Shown>(ctx, sub(id, "shown"));
            return;
        }
        let texts: Vec<String> = if open {
            cards.iter().map(|card| env.view().resolve(&card.toast.message).as_str().to_owned()).collect()
        } else {
            store::peek_clone::<Shown>(ctx, sub(id, "shown")).unwrap_or_default().0
        };
        if open {
            store::put(ctx, sub(id, "shown"), Shown(texts.clone()));
        }
        let overlay = Overlay::toast().place(Place::anchored(Anchor::Viewport(corner(&env)), Side::Above, Align::End));
        let layer = Layer { id, owner: id, overlay: &overlay, anchor: Rect::NOTHING, env: &env, exiting: !open, fade: presence.t };
        overlay::host(ctx, &layer, &mut |ui| {
            for (at, text) in texts.iter().enumerate() {
                let card = cards.get(at);
                let salt = ("toast", card.map(|card| card.id), card.is_none().then_some(at));
                let hovered = ToastCard { common: Common::default(), text }.id_salt(salt).show(ui).hovered();
                if let Some(card) = cards.get(at) {
                    toast::shown(ctx, card.id, text, hovered);
                }
            }
        });
    }
}

/// The viewport's end-bottom corner: right in left-to-right, left in right-to-left.
fn corner(env: &Env) -> Align2 {
    if env.direction.is_rtl() { Align2::LEFT_BOTTOM } else { Align2::RIGHT_BOTTOM }
}

/// The look of a toast card.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct CardStyle {
    #[style(surface)]
    surface: SurfaceLook,
    #[style(metrics)]
    metrics: Metrics,
}

/// One toast's text on a floating surface.
// F8-LATER: `Notice` shows the card (tone, icon, actions).
#[derive(crate::contract::Component)]
#[component(exempt(case = "face-only", reason = "The toast card is shown by the toast host, never stand-alone."))]
struct ToastCard<'a> {
    #[common]
    common: Common,
    text: &'a str,
}

impl Component for ToastCard<'_> {
    const ROLE: Role = Role::STATUS;
    const INPUT: craft_behavior::Input = craft_behavior::Input::Display;
    type Style = CardStyle;
    type Output = egui::Response;

    fn style(cx: &StyleCx<'_>, _: ToastCardStyleKey) -> CardStyle {
        let fill = StyleColor(cx.surface(SurfaceKind::Floating).fill);
        CardStyle { surface: SurfaceLook::new(Stateful::new(fill)).kind(SurfaceKind::Floating), metrics: cx.control().metrics() }
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        cx.layout(&TextRun::new(self.text).wrap(Wrap::Wrap)).size()
    }

    fn sense(&self) -> craft_behavior::Sense {
        craft_behavior::Sense::HOVER
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        ui.draw().text(cx.content_rect, &TextRun::new(self.text).wrap(Wrap::Wrap));
    }

    fn finish(self, cx: Cx<'_, Self>) -> egui::Response {
        cx.response.clone()
    }
}
