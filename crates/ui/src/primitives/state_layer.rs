//! [`StateLayer`]: the hover and pressed wash of interactive elements.

use egui::{Color32, CornerRadius, Rect, Response, Ui, vec2};

use crate::story::{Args, Level, Story};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens};

/// The wash colours and their fade, from [`StateLayer::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StateLayerStyle {
    /// Under the pointer: `state.hover`.
    pub hover: Color32,
    /// While pressed: `state.pressed`, drawn over the hover wash.
    pub pressed: Color32,
    /// Fade in and out, in seconds: `motion.hover`, 0 with reduced motion.
    pub fade: f32,
}

/// Paints the hover and pressed wash behind an element's content; nothing when disabled.
///
/// [`Interactive`](super::Interactive) paints it for its controls. Call it before drawing the
/// content: the state fills are opaque.
pub struct StateLayer;

impl StateLayer {
    pub fn style(tokens: &UiTokens) -> StateLayerStyle {
        let c = &tokens.color;
        StateLayerStyle { hover: c.state.hover.to_egui(), pressed: c.state.pressed.to_egui(), fade: tokens.motion.hover.millis as f32 / 1000.0 }
    }

    /// Paints the wash for `response` over `rect`.
    pub fn paint(ui: &Ui, rect: Rect, radius: CornerRadius, response: &Response) {
        if !response.enabled() {
            return;
        }
        let style = StateLayer::style(&Tokens::of(ui));
        let ctx = ui.ctx();
        let hover = ctx.animate_bool_with_time(response.id.with("craft_ui::state_layer::hover"), response.hovered(), style.fade);
        let pressed =
            ctx.animate_bool_with_time(response.id.with("craft_ui::state_layer::pressed"), response.is_pointer_button_down_on(), style.fade);
        let painter = ui.painter();
        for (amount, color) in [(hover, style.hover), (pressed, style.pressed)] {
            if amount > 0.0 {
                painter.rect_filled(rect, radius, color.gamma_multiply(amount));
            }
        }
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["State layer"];

pub(crate) fn stories() -> Vec<Story> {
    vec![Story {
        id: "primitives/state-layer/states",
        level: Level::Primitives,
        component: "State layer",
        name: "States",
        args: || Args::new().with_bool("disabled", false),
        show,
    }]
}

/// The rest, hover and pressed washes side by side, then a live one.
fn show(ui: &mut Ui, args: &mut Args) {
    let tokens = Tokens::of(ui);
    let style = StateLayer::style(&tokens);
    let radius = tokens.radius.control;
    let size = vec2(72.0, tokens.size.control.md);
    super::Row::new(super::Gap::Inline).show(ui, |ui| {
        for (name, fill) in [("rest", Color32::TRANSPARENT), ("hover", style.hover), ("pressed", style.pressed)] {
            let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
            ui.painter().rect_filled(rect, radius, fill);
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                name,
                tokens.font(craft_theme::TypeRole::Label),
                tokens.color.text.normal.to_egui(),
            );
        }
    });
    ui.add_enabled_ui(!args.bool("disabled"), |ui| {
        let response = super::Interactive::new("state-layer-live", egui::accesskit::Role::Button, "Hover or press")
            .focus_ring(false)
            .show(ui, size * vec2(2.0, 1.0));
        let color = super::Content::Normal.color(&tokens).to_egui();
        ui.painter().text(response.rect.center(), egui::Align2::CENTER_CENTER, "Hover or press", tokens.font(craft_theme::TypeRole::Label), color);
    });
}
