//! The toast host (15.5): one layer per viewport that shows each queued toast as a card.

use super::overlay::{self, Layer, Overlay};
use super::{Common, Component, Cx, MeasureCx, PaintUi, Props as _, Role, Show};
use crate::primitives::{TextRun, Wrap};
use crate::runtime::egui::memory as store;
use crate::runtime::env::Env;
use crate::runtime::memory::sub;
use crate::runtime::toast;
use crate::style::{ComponentStyle, StyleColor, StyleCx, SurfaceKind, SurfaceLook};
use craft_behavior::Metrics;
use craft_theme::Stateful;
use egui::{Rect, Ui, Vec2};

/// The layer that shows the toast queue of a viewport. The shell calls `show` once per pass.
#[derive(Clone, Copy, Debug, Default)]
pub struct ToastHost;

impl ToastHost {
    /// Shows every queued toast of the running viewport, oldest at the top, in the viewport's end corner. A card the
    /// pointer rests on keeps its timer paused; the first show of a toast announces it.
    pub fn show(ui: &Ui) {
        let ctx = ui.ctx();
        let cards = toast::cards(ctx);
        if cards.is_empty() {
            return;
        }
        let env = Env::of(ui);
        let id = sub(store::unique("craft_ui.toast_host"), crate::runtime::egui::pass::current_viewport(ctx));
        let overlay = Overlay::toast();
        let layer = Layer { id, overlay: &overlay, anchor: Rect::NOTHING, env: &env, exiting: false, fade: 1.0 };
        overlay::host(ctx, &layer, &mut |ui| {
            for card in &cards {
                let text = env.view().resolve(&card.toast.message).as_str().to_owned();
                let hovered = ToastCard { common: Common::default(), text: &text }.id_salt(("toast", card.id)).show(ui).hovered();
                toast::shown(ctx, card.id, &text, hovered);
            }
        });
    }
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
        cx.layout(&TextRun::new(self.text).wrap(Wrap::Line)).size()
    }

    fn sense(&self) -> craft_behavior::Sense {
        craft_behavior::Sense::HOVER
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        ui.draw().text(cx.content_rect, &TextRun::new(self.text));
    }

    fn finish(self, cx: Cx<'_, Self>) -> egui::Response {
        cx.response.clone()
    }
}

