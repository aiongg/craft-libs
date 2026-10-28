//! [`StateLayer`]: the hover and pressed fills of interactive elements.

use craft_theme::Easing;
use egui::{Color32, Rect, Response, Ui, vec2};

use super::{Corner, Surface, SurfaceKind};
use crate::story::{Args, Level, Story};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens};

/// The state fills and their animation, from [`StateLayer::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct StateLayerStyle {
    /// Under the pointer: `state.hover`.
    pub hover: Color32,
    /// While pressed: `state.pressed`, over the hover fill.
    pub pressed: Color32,
    /// Time from one fill to the next, in seconds: `motion.hover`, 0 with reduced motion.
    pub fade: f32,
    /// The curve of that change: `motion.hover`'s easing.
    pub easing: Easing,
}

impl StateLayerStyle {
    /// The fill `hover` and `pressed` of the way (0 to 1) from `rest` to the hover and pressed
    /// fills.
    pub(crate) fn mix(&self, rest: Color32, hover: f32, pressed: f32) -> Color32 {
        rest.lerp_to_gamma(self.hover, hover).lerp_to_gamma(self.pressed, pressed)
    }
}

/// The fill of an interactive element in its hover and pressed states.
///
/// `state.hover` and `state.pressed` are opaque fills that take the place of the element's rest
/// fill, never a wash over it: [`fill`](StateLayer::fill) gives the fill to paint, and the
/// element's edge and content go on top. An element on a [`Surface`] rests at the surface's fill;
/// one without (a toolbar item) rests transparent. [`Interactive`](super::Interactive) paints it
/// for its controls. A disabled element keeps its rest fill.
pub struct StateLayer;

impl StateLayer {
    pub fn style(tokens: &UiTokens) -> StateLayerStyle {
        let c = &tokens.color;
        let motion = tokens.motion.hover;
        StateLayerStyle {
            hover: c.state.hover.to_egui(),
            pressed: c.state.pressed.to_egui(),
            fade: motion.millis as f32 / 1000.0,
            easing: motion.easing,
        }
    }

    /// The fill of `response`'s element now: the rest fill of `surface` (transparent without
    /// one), animated to the hover and pressed fills.
    pub fn fill(ui: &Ui, surface: Option<SurfaceKind>, response: &Response) -> Color32 {
        let tokens = Tokens::of(ui);
        let rest = surface.map_or(Color32::TRANSPARENT, |kind| Surface::style(&tokens, kind).fill);
        if !response.enabled() {
            return rest;
        }
        let style = StateLayer::style(&tokens);
        let (ctx, easing) = (ui.ctx(), style.easing.to_egui());
        let animate = |salt: &str, on: bool| ctx.animate_bool_with_time_and_easing(response.id.with(salt), on, style.fade, easing);
        let hover = animate("craft_ui::state_layer::hover", response.hovered());
        let pressed = animate("craft_ui::state_layer::pressed", response.is_pointer_button_down_on());
        style.mix(rest, hover, pressed)
    }

    /// Paints the state fill of an element without a surface over `rect`, with `corner`'s
    /// corners. Call it before drawing the content.
    pub fn paint(ui: &Ui, rect: Rect, corner: Corner, response: &Response) {
        let fill = StateLayer::fill(ui, None, response);
        if fill != Color32::TRANSPARENT {
            ui.painter().rect_filled(rect, corner.radius(&Tokens::of(ui)), fill);
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

/// The rest, hover and pressed fills side by side, then live ones without and with a surface.
fn show(ui: &mut Ui, args: &mut Args) {
    let tokens = Tokens::of(ui);
    let style = StateLayer::style(&tokens);
    let radius = Corner::Control.radius(&tokens);
    let size = vec2(72.0, tokens.size.control.md);
    let label = |ui: &Ui, rect: Rect, text: &str| {
        let color = super::Content::Normal.color(&tokens).to_egui();
        ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, text, tokens.font(craft_theme::TypeRole::Label), color);
    };
    super::Row::new(super::Gap::Inline).show(ui, |ui| {
        for (name, fill) in [("rest", Color32::TRANSPARENT), ("hover", style.hover), ("pressed", style.pressed)] {
            let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
            ui.painter().rect_filled(rect, radius, fill);
            label(ui, rect, name);
        }
    });
    ui.add_enabled_ui(!args.bool("disabled"), |ui| {
        super::Row::new(super::Gap::Inline).show(ui, |ui| {
            let bare = super::Interactive::new("state-layer-live", egui::accesskit::Role::Button, "Hover or press")
                .focus_ring(false)
                .show(ui, size * vec2(2.0, 1.0));
            label(ui, bare.rect, "Hover or press");
            let raised = super::Interactive::new("state-layer-raised", egui::accesskit::Role::Button, "On a raised surface")
                .surface(SurfaceKind::Raised)
                .focus_ring(false)
                .show(ui, size * vec2(2.0, 1.0));
            label(ui, raised.rect, "On a surface");
        });
    });
}
