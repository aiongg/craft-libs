//! [`StateLayer`]: the hover and pressed fills of interactive elements.

use craft_theme::{ButtonColors, Easing, ItemColors, States};
use egui::{Color32, Id, Rect, Response, Ui, vec2};

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

/// The colours `hover` and `pressed` of the way (0 to 1) from `colors.rest` to the hover and
/// pressed colours.
pub(crate) fn mix(colors: &States<Color32>, hover: f32, pressed: f32) -> Color32 {
    colors.rest.lerp_to_gamma(colors.hover, hover).lerp_to_gamma(colors.pressed, pressed)
}

/// A component's own fill and border colours per state, from its family's tokens, which
/// [`Interactive::colors`](super::Interactive::colors) paints in place of the surface's fill, the
/// `state.*` fills and the surface's border colour. Content colours (`text`) stay with the
/// component, which tints them with [`StateLayer::color`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct StateColors {
    pub fill: States<Color32>,
    /// The border's colour; `None` keeps the surface's. A bevel keeps its own colours.
    pub border: Option<States<Color32>>,
}

impl StateColors {
    /// Fills only: the border, if any, stays the surface's.
    pub fn fill(fill: States<craft_theme::Rgba>) -> Self {
        Self { fill: fill.to_egui(), border: None }
    }

    /// The border colours too.
    pub fn border(mut self, border: States<craft_theme::Rgba>) -> Self {
        self.border = Some(border.to_egui());
        self
    }
}

/// A button variant's fill and border.
impl From<&ButtonColors> for StateColors {
    fn from(c: &ButtonColors) -> Self {
        Self { fill: c.fill.to_egui(), border: Some(c.border.to_egui()) }
    }
}

/// An item's fill; items have no outline.
impl From<&ItemColors> for StateColors {
    fn from(c: &ItemColors) -> Self {
        StateColors::fill(c.fill)
    }
}

/// The fill of an interactive element in its hover and pressed states.
///
/// `state.hover` and `state.pressed` are opaque fills that take the place of the element's rest
/// fill, never a wash over it: [`fill`](StateLayer::fill) gives the fill to paint, and the
/// element's edge and content go on top. An element on a [`Surface`] rests at the surface's fill;
/// one without (a toolbar item) rests transparent. [`Interactive`](super::Interactive) paints it
/// for its controls. A disabled element keeps its rest fill.
///
/// A component with its own colours per state (a button's `fill`, `border` and `text`) runs them
/// through [`color`](StateLayer::color): one hover and one pressed animation per element move
/// every part together.
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

    /// The fills of an element on `surface` (none: transparent), per state: the surface's fill
    /// at rest and when disabled, `state.hover` and `state.pressed`.
    pub fn fills(tokens: &UiTokens, surface: Option<SurfaceKind>) -> States<Color32> {
        let rest = surface.map_or(Color32::TRANSPARENT, |kind| Surface::style(tokens, kind).fill);
        let style = StateLayer::style(tokens);
        States::new(rest, style.hover, style.pressed, rest)
    }

    /// The fill of `response`'s element now: the rest fill of `surface` (transparent without
    /// one), animated to the hover and pressed fills.
    pub fn fill(ui: &Ui, surface: Option<SurfaceKind>, response: &Response) -> Color32 {
        StateLayer::color(ui, &StateLayer::fills(&Tokens::of(ui), surface), response)
    }

    /// `colors` for `response`'s element now: its rest colour animated to the hover and pressed
    /// colours with `motion.hover`, or its disabled colour. Every part of one element (fill,
    /// border, text) shares the same animation.
    pub fn color(ui: &Ui, colors: &States<Color32>, response: &Response) -> Color32 {
        StateLayer::color_of(ui, colors, response.id, response.enabled(), response.hovered(), response.is_pointer_button_down_on())
    }

    /// [`StateLayer::color`] for a part of a control that has no response of its own (a range
    /// slider's thumb), hovered and pressed as the control works out; `id` keys its animation.
    pub(crate) fn color_of(ui: &Ui, colors: &States<Color32>, id: Id, enabled: bool, hovered: bool, pressed: bool) -> Color32 {
        if !enabled {
            return colors.disabled;
        }
        let style = StateLayer::style(&Tokens::of(ui));
        let (ctx, easing) = (ui.ctx(), style.easing.to_egui());
        let animate = |salt: &str, on: bool| ctx.animate_bool_with_time_and_easing(id.with(salt), on, style.fade, easing);
        let hover = animate("craft_ui::state_layer::hover", hovered);
        let pressed = animate("craft_ui::state_layer::pressed", pressed);
        mix(colors, hover, pressed)
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
    vec![Story::new("primitives/state-layer/states", Level::Primitives, "State layer", "States", || Args::new().with_bool("disabled", false), show)]
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
