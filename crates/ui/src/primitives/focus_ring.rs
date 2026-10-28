//! [`FocusRing`]: the keyboard focus outline, shown only while the user works with the keyboard.

use egui::accesskit::Role;
use egui::epaint::RectShape;
use egui::plugin::Plugin;
use egui::{
    Color32, Context, CornerRadius, Event, LayerId, Painter, RawInput, Rect, Response, Shape, Stroke, StrokeKind, TouchPhase, Ui, Vec2, vec2,
};

use super::Corner;
use crate::story::{Args, Level, Story};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens};

/// The ring's stroke and its distance from the element, from [`FocusRing::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct FocusRingStyle {
    /// One point of `line.focus`.
    pub stroke: Stroke,
    /// Gap between the element's rect and the ring's inner edge, in points.
    pub offset: f32,
}

impl FocusRingStyle {
    /// How far the ring reaches outside the element's rect, in points.
    pub fn outset(&self) -> f32 {
        self.offset + self.stroke.width
    }
}

/// Paints the keyboard focus ring outside an element's rect.
///
/// The ring shows only while the element has keyboard focus and the last input was from the
/// keyboard (Tab, arrows, shortcuts, a screen reader's focus request), not after a pointer press,
/// as browsers do for `:focus-visible`. [`theme::install`](crate::theme::install) tracks the
/// input; without it no ring shows. [`Interactive`](super::Interactive) paints the ring for its
/// controls.
///
/// The ring goes on a sublayer just above the element's layer, so it draws over the neighbours
/// that follow the element and outside the element's own clip rect, but stays under popups and
/// windows above that layer.
pub struct FocusRing;

impl FocusRing {
    pub fn style(tokens: &UiTokens) -> FocusRingStyle {
        FocusRingStyle { stroke: Stroke::new(craft_theme::Stroke::W1.width(1.0), tokens.color.line.focus.to_egui()), offset: 1.0 }
    }

    /// The ring around `rect` with `corner`'s corners, whatever the focus.
    pub fn shape(tokens: &UiTokens, corner: Corner, rect: Rect) -> Shape {
        FocusRing::ring(tokens, corner.radius(tokens), rect)
    }

    /// The ring around `rect`, whose corners are `radius`.
    pub(crate) fn ring(tokens: &UiTokens, radius: CornerRadius, rect: Rect) -> Shape {
        let style = FocusRing::style(tokens);
        let ring = rect.expand(style.offset);
        // The ring follows the element's corners at its distance.
        let grow = |r: u8| r.saturating_add(style.offset.round() as u8);
        let radius = CornerRadius { nw: grow(radius.nw), ne: grow(radius.ne), sw: grow(radius.sw), se: grow(radius.se) };
        Shape::Rect(RectShape::stroke(ring, radius, style.stroke, StrokeKind::Outside))
    }

    /// Paints the ring around `rect` (with `corner`'s corners) when `response` has keyboard focus
    /// and [`FocusRing::visible`].
    pub fn paint(ui: &Ui, rect: Rect, corner: Corner, response: &Response) {
        FocusRing::paint_radius(ui, rect, corner.radius(&Tokens::of(ui)), response);
    }

    /// [`FocusRing::paint`] with resolved corners.
    pub(crate) fn paint_radius(ui: &Ui, rect: Rect, radius: CornerRadius, response: &Response) {
        if !(response.has_focus() && ui.is_visible() && FocusRing::visible(ui.ctx())) {
            return;
        }
        let tokens = Tokens::of(ui);
        let painter = ring_painter(ui, FocusRing::style(&tokens).outset());
        painter.add(FocusRing::ring(&tokens, radius, rect));
    }

    /// Whether the last input in `ctx` came from the keyboard, so focus rings show. False until
    /// [`theme::install`](crate::theme::install) has run in `ctx`.
    pub fn visible(ctx: &Context) -> bool {
        ctx.with_plugin(|m: &mut Modality| m.keyboard).unwrap_or(false)
    }
}

/// A painter for rings around the widgets of `ui`: on a sublayer of `ui`'s layer, with its
/// transform and opacity, clipped to `ui`'s clip rect grown by `outset`.
fn ring_painter(ui: &Ui, outset: f32) -> Painter {
    let ctx = ui.ctx();
    let layer = ui.layer_id();
    // egui supports one level of sublayers: inside a sublayer (a scene), the ring joins the
    // same parent, after the scene's layer.
    let parent = ctx.memory(|m| m.areas().parent_layer(layer)).unwrap_or(layer);
    let ring_layer = LayerId::new(layer.order, layer.id.with("craft_ui::focus_ring"));
    ctx.set_sublayer(parent, ring_layer);
    ctx.set_transform_layer(ring_layer, ctx.layer_transform_to_global(layer).unwrap_or_default());
    let mut painter = ctx.layer_painter(ring_layer).with_clip_rect(ui.clip_rect().expand(outset));
    painter.set_opacity(ui.painter().opacity());
    painter
}

/// Registers the input-modality tracking in `ctx`, once. Called by
/// [`theme::install`](crate::theme::install).
pub(crate) fn track_modality(ctx: &Context) {
    if ctx.plugin_opt::<Modality>().is_none() {
        ctx.add_plugin(Modality::default());
        // The plugin sees input from the next pass on; this pass's input is already in.
        let now = ctx.input(|i| last_modality(&i.events));
        ctx.with_plugin(|m: &mut Modality| m.keyboard = now.unwrap_or(m.keyboard));
    }
}

/// Tracks whether the last input of a context was from the keyboard.
#[derive(Default)]
struct Modality {
    keyboard: bool,
}

impl Plugin for Modality {
    fn debug_name(&self) -> &'static str {
        "craft_ui::primitives::focus_ring"
    }

    fn input_hook(&mut self, _: &Context, input: &mut RawInput) {
        if let Some(keyboard) = last_modality(&input.events) {
            self.keyboard = keyboard;
        }
    }
}

/// `Some(true)` when the last event that sets the modality is from the keyboard, `Some(false)`
/// when it is a pointer press or a touch, `None` when no event sets it.
fn last_modality(events: &[Event]) -> Option<bool> {
    events.iter().rev().find_map(|event| match event {
        Event::Key { pressed: true, .. } | Event::AccessKitActionRequest(_) => Some(true),
        Event::PointerButton { pressed: true, .. } | Event::Touch { phase: TouchPhase::Start, .. } => Some(false),
        _ => None,
    })
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Focus ring"];

pub(crate) fn stories() -> Vec<Story> {
    vec![
        Story {
            id: "primitives/focus-ring/focus",
            level: Level::Primitives,
            component: "Focus ring",
            name: "Focus",
            args: || Args::new().with_number("items", 4.0, 1.0, 8.0),
            show: show_focus,
        },
        Story { id: "primitives/focus-ring/ring", level: Level::Primitives, component: "Focus ring", name: "Ring", args: Args::new, show: show_ring },
    ]
}

/// A row of items to Tab through: rings show after Tab, not after a click.
fn show_focus(ui: &mut Ui, args: &mut Args) {
    let tokens = Tokens::of(ui);
    let count = args.number("items").round().clamp(1.0, 8.0) as usize;
    super::Row::new(super::Gap::Inline).show(ui, |ui| {
        for i in 1..=count {
            let label = format!("Item {i}");
            let response = super::Interactive::new(("focus-ring-item", i), Role::Button, &label).show(ui, item_size(&tokens));
            paint_label(ui, &tokens, response.rect, &label);
        }
    });
    ui.add(super::Text::new("Press Tab to move focus; click to hide the ring.").content(super::Content::Muted));
}

/// The ring drawn without focus, to review its colour and offset.
fn show_ring(ui: &mut Ui, _: &mut Args) {
    let tokens = Tokens::of(ui);
    super::Row::new(super::Gap::Section).show(ui, |ui| {
        for corner in [Corner::Control, Corner::Button] {
            let (rect, _) = ui.allocate_exact_size(item_size(&tokens), egui::Sense::hover());
            ui.painter().rect_filled(rect, corner.radius(&tokens), tokens.color.state.hover.to_egui());
            ui.painter().add(FocusRing::shape(&tokens, corner, rect));
            paint_label(ui, &tokens, rect, corner.name());
        }
    });
}

fn item_size(tokens: &UiTokens) -> Vec2 {
    vec2(64.0, tokens.size.control.md)
}

fn paint_label(ui: &Ui, tokens: &UiTokens, rect: Rect, text: &str) {
    let color: Color32 = tokens.color.text.normal.to_egui();
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, text, tokens.font(craft_theme::TypeRole::Label), color);
}
