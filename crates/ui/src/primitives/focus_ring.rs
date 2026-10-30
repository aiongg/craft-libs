//! [`FocusRing`]: the keyboard focus outline, shown only while the user works with the keyboard.

use egui::accesskit::Role;
use egui::epaint::{CircleShape, PathShape, RectShape};
use egui::plugin::Plugin;
use egui::{
    Color32, Context, CornerRadius, Event, LayerId, Painter, Pos2, RawInput, Rect, Response, Shape, Stroke, StrokeKind, TouchPhase, Ui, Vec2, vec2,
};

use super::Corner;
use crate::story::NoArgs;
use crate::story::{Level, Story, StoryArgs};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens};
use crate::typed_story;

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
        FocusRing::paint_if(ui, rect, radius, response.has_focus());
    }

    /// The ring around `rect` when `focused` and [`FocusRing::visible`], for an element that shows
    /// the focus of a widget it is part of (a list's active row).
    pub(crate) fn paint_if(ui: &Ui, rect: Rect, radius: CornerRadius, focused: bool) {
        if !(focused && ui.is_visible() && FocusRing::visible(ui.ctx())) {
            return;
        }
        let tokens = Tokens::of(ui);
        let painter = ring_painter(ui, FocusRing::style(&tokens).outset());
        painter.add(FocusRing::ring(&tokens, radius, rect));
    }

    /// The ring round a circle of `radius` at `center`: one circle outside it at the ring's
    /// distance. With `inner` (the inner radius of an annulus such as a hue ring), a second circle
    /// inside the inner edge, at the same distance.
    pub fn circle_shapes(tokens: &UiTokens, center: Pos2, radius: f32, inner: Option<f32>) -> Vec<Shape> {
        let style = FocusRing::style(tokens);
        let half = style.stroke.width / 2.0;
        let mut shapes = vec![Shape::Circle(CircleShape::stroke(center, radius + style.offset + half, style.stroke))];
        if let Some(inner) = inner {
            let r = inner - style.offset - half;
            if r > 0.0 {
                shapes.push(Shape::Circle(CircleShape::stroke(center, r, style.stroke)));
            }
        }
        shapes
    }

    /// The ring round the convex polygon `points` (in either winding): its outline moved out by
    /// the ring's distance, with mitred corners.
    pub fn polygon_shape(tokens: &UiTokens, points: &[Pos2]) -> Shape {
        let style = FocusRing::style(tokens);
        let outline = offset_convex(points, style.offset + style.stroke.width / 2.0);
        Shape::Path(PathShape::closed_line(outline, style.stroke))
    }

    /// Paints [`FocusRing::circle_shapes`] when `focused` and [`FocusRing::visible`].
    pub fn paint_circle(ui: &Ui, center: Pos2, radius: f32, inner: Option<f32>, focused: bool) {
        if !(focused && ui.is_visible() && FocusRing::visible(ui.ctx())) {
            return;
        }
        let tokens = Tokens::of(ui);
        let painter = ring_painter(ui, FocusRing::style(&tokens).outset());
        painter.extend(FocusRing::circle_shapes(&tokens, center, radius, inner));
    }

    /// Paints [`FocusRing::polygon_shape`] when `focused` and [`FocusRing::visible`].
    pub fn paint_polygon(ui: &Ui, points: &[Pos2], focused: bool) {
        if !(focused && ui.is_visible() && FocusRing::visible(ui.ctx())) {
            return;
        }
        let tokens = Tokens::of(ui);
        let painter = ring_painter(ui, FocusRing::style(&tokens).outset());
        painter.add(FocusRing::polygon_shape(&tokens, points));
    }

    /// Whether the last input in `ctx` came from the keyboard, so focus rings show. False until
    /// [`theme::install`](crate::theme::install) has run in `ctx`.
    pub fn visible(ctx: &Context) -> bool {
        ctx.with_plugin(|m: &mut Modality| m.keyboard).unwrap_or(false)
    }
}

/// `points` (a convex polygon, either winding) moved outwards by `distance`: every vertex moves
/// along the bisector of its two edges, far enough that each edge moves by `distance`. A vertex
/// of a near-degenerate spike moves at most four times `distance`.
fn offset_convex(points: &[Pos2], distance: f32) -> Vec<Pos2> {
    let n = points.len();
    if n < 3 {
        return points.to_vec();
    }
    let area: f32 = (0..n).map(|i| points[i].to_vec2().x * points[(i + 1) % n].y - points[(i + 1) % n].x * points[i].y).sum();
    // Rotating a counter-clockwise (positive area, y up) edge direction a quarter turn clockwise
    // points outwards; the signs flip with the winding.
    let sign = if area >= 0.0 { 1.0 } else { -1.0 };
    let normal = |a: Pos2, b: Pos2| {
        let d = (b - a).normalized();
        vec2(d.y, -d.x) * sign
    };
    (0..n)
        .map(|i| {
            let prev = points[(i + n - 1) % n];
            let next = points[(i + 1) % n];
            let (n1, n2) = (normal(prev, points[i]), normal(points[i], next));
            let scale = (1.0 / (1.0 + n1.dot(n2)).max(0.0625)).min(4.0);
            points[i] + (n1 + n2) * scale * distance
        })
        .collect()
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
        typed_story!("primitives/focus-ring/focus", Level::Primitives, "Focus ring", "Focus", FocusArgs { items: 4 }, show_focus),
        typed_story!("primitives/focus-ring/ring", Level::Primitives, "Focus ring", "Ring", NoArgs {}, show_ring),
    ]
}

#[derive(StoryArgs, Debug, PartialEq)]
struct FocusArgs {
    #[arg(range = 1.0..=8.0)]
    items: u32,
}

/// A row of items to Tab through: rings show after Tab, not after a click.
fn show_focus(ui: &mut Ui, args: &mut FocusArgs) {
    let tokens = Tokens::of(ui);
    let count = args.items.clamp(1, 8) as usize;
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
fn show_ring(ui: &mut Ui, _: &mut NoArgs) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use egui::pos2;

    /// Each edge of the offset polygon lies `distance` outside the original, whichever way the
    /// polygon winds.
    #[test]
    fn offset_convex_moves_every_edge_out_by_the_distance() {
        let square = [pos2(0.0, 0.0), pos2(10.0, 0.0), pos2(10.0, 10.0), pos2(0.0, 10.0)];
        for points in [square.to_vec(), square.iter().rev().copied().collect()] {
            let out = offset_convex(&points, 2.0);
            let xs: Vec<f32> = out.iter().map(|p| p.x).collect();
            let ys: Vec<f32> = out.iter().map(|p| p.y).collect();
            assert_eq!(xs.iter().copied().fold(f32::MAX, f32::min), -2.0);
            assert_eq!(xs.iter().copied().fold(f32::MIN, f32::max), 12.0);
            assert_eq!(ys.iter().copied().fold(f32::MAX, f32::min), -2.0);
            assert_eq!(ys.iter().copied().fold(f32::MIN, f32::max), 12.0);
        }
    }
}
