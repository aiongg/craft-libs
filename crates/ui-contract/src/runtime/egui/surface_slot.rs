//! The egui runtime's surface slots and layer frames: a surface painted under content whose rect comes later, and the
//! rounded frame a floating window's surfaces keep to. Both go with the egui runtime; on craft-ui-rt a node's surface is
//! part of its display list and the layer frame is an env value (B9).

use crate::primitives::Surface;
use crate::style::{Corners, SurfaceKind, UiTokens, surface_style};
use craft_i18n::Direction;
use craft_ui_rt::Item;
use egui::layers::ShapeIdx;
use egui::{CornerRadius, Painter, Rect, Shape, Ui};

/// The rounded frames surfaces keep to, at most one per layer: a floating window's body ([`frame_surfaces`]).
#[derive(Default)]
pub(crate) struct Frames(Vec<(crate::runtime::egui::areas::Layer, Rect, CornerRadius)>);

/// The most layers that keep a frame at once: the frame a closed window left gives way to newer ones.
const MOST_FRAMES: usize = 32;

/// How far a surface's edge may lie from the frame's and still meet it, in points: the rounding to the pixel grid.
const FRAME_SLACK: f32 = 0.75;

/// Rounds the surfaces painted from here on in `ui`'s layer that meet a corner of `rect`: such a surface takes
/// `radius` at that corner where its own is smaller. A floating window sets its body's rect and its own lower corners,
/// so the square surfaces of the panels and rails inside it (a dock column, a tool rail's chrome) keep to its rounded
/// corners instead of covering them.
pub fn frame_surfaces(ui: &Ui, rect: Rect, radius: CornerRadius) {
    // A measuring run paints nothing.
    if crate::runtime::measure::active(ui.ctx()) {
        return;
    }
    let layer = ui.layer_id();
    crate::runtime::pass_state::with::<Frames, _>(ui.ctx(), |frames| {
        frames.0.retain(|(at, ..)| *at != layer);
        if frames.0.len() >= MOST_FRAMES {
            frames.0.remove(0);
        }
        frames.0.push((layer, rect, radius));
    });
}

/// Fits `items`, painted in `layer`, to the layer's frame ([`frame_surfaces`]); a layer without one keeps them.
pub(crate) fn fit_to_frame(ctx: &egui::Context, layer: crate::runtime::egui::areas::Layer, items: &mut [Item]) {
    if crate::runtime::measure::active(ctx) {
        return;
    }
    let frame = crate::runtime::pass_state::with::<Frames, _>(ctx, |frames| {
        frames.0.iter().find(|(at, ..)| *at == layer).map(|(_, rect, radius)| (*rect, *radius))
    });
    let Some((rect, radius)) = frame else { return };
    for item in items {
        fit_item(item, rect, radius);
    }
}

/// Rounds each corner of a rect `item` that meets the same corner of `frame` to at least the frame's radius there.
fn fit_item(item: &mut Item, frame: Rect, radius: CornerRadius) {
    let (Item::Rect { rect: at, radii, .. } | Item::Shadow { rect: at, radii, .. }) = item else { return };
    let near = |a: f32, b: f32| (a - b).abs() <= FRAME_SLACK;
    let (left, right) = (near(at.left(), frame.left()), near(at.right(), frame.right()));
    let (top, bottom) = (near(at.top(), frame.top()), near(at.bottom(), frame.bottom()));
    if top && left {
        radii.top_left = radii.top_left.max(f32::from(radius.nw));
    }
    if top && right {
        radii.top_right = radii.top_right.max(f32::from(radius.ne));
    }
    if bottom && left {
        radii.bottom_left = radii.bottom_left.max(f32::from(radius.sw));
    }
    if bottom && right {
        radii.bottom_right = radii.bottom_right.max(f32::from(radius.se));
    }
}

/// The surface under content that runs before its rect is known: [`reserve`](SurfaceSlot::reserve) takes its place in
/// the paint order, [`paint`](SurfaceSlot::paint) fills it at the rect the content took.
pub(crate) struct SurfaceSlot {
    painter: Painter,
    slot: ShapeIdx,
}

impl SurfaceSlot {
    /// The slot under what `ui` paints next. A layer's surface casts its shadow past the layer's own clip, so on a
    /// `layer` the slot paints under the whole window.
    pub(crate) fn reserve(ui: &Ui, layer: bool) -> Self {
        let mut painter = ui.painter().clone();
        if layer {
            painter.set_clip_rect(ui.ctx().content_rect());
        }
        let slot = painter.add(Shape::Noop);
        SurfaceSlot { painter, slot }
    }

    /// Paints the surface `kind` over `rect`, kept to its layer's frame ([`frame_surfaces`]).
    pub(crate) fn paint(&self, tokens: &UiTokens, kind: SurfaceKind, rect: Rect, direction: Direction) {
        let mut items = Surface::items(&surface_style(tokens, kind), None, rect, Corners::All, direction, self.painter.pixels_per_point());
        fit_to_frame(self.painter.ctx(), self.painter.layer_id(), &mut items);
        self.painter.set(self.slot, Shape::Vec(super::text_layout::shapes_of(&items)));
    }
}
