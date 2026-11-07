//! `Area` with its left-top pivot, `Order`, sublayers, transform layers and `set_layer_order`.

#[cfg(feature = "testing")]
use egui::Ui;
use egui::emath::TSTransform;
use egui::{Color32, Context, Id, LayerId, Order, Painter, Pos2, Rect, Sense, Vec2};

/// Shows an `egui::Area` named `name` and runs `body` on its `Ui`.
#[cfg(feature = "testing")]
pub(crate) fn show_area<R>(ctx: &Context, name: &'static str, body: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Area::new(crate::runtime::egui::memory::unique(name)).show(ctx, body).inner
}

/// Shows an interactable `egui::Area` at the screen's top left and runs `body` on its `Ui`.
#[cfg(feature = "testing")]
pub(crate) fn show_cover<R>(ctx: &Context, name: &'static str, body: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Area::new(crate::runtime::egui::memory::unique(name)).fixed_pos(Pos2::ZERO).interactable(true).show(ctx, body).inner
}

/// An egui layer: an `Area`, a panel or a transformed body.
pub(crate) type Layer = LayerId;

/// A layer's mapping from its own space to the screen's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LayerMap(TSTransform);

impl LayerMap {
    /// The position in the layer of a screen position.
    pub(crate) fn to_layer(self, point: Pos2) -> Pos2 {
        self.0.inverse() * point
    }

    /// The screen position of a position in the layer.
    pub(crate) fn to_screen(self, point: Pos2) -> Pos2 {
        self.0 * point
    }

    /// A screen-space movement as the layer sees it.
    pub(crate) fn delta_to_layer(self, delta: Vec2) -> Vec2 {
        delta / self.0.scaling
    }
}

/// The mapping of `layer` to the screen, `None` for a layer with no transform.
pub(crate) fn layer_map(ctx: &Context, layer: LayerId) -> Option<LayerMap> {
    ctx.layer_transform_to_global(layer).map(LayerMap)
}

/// How high `layer` stands in egui's layer order, which the overlay host sets each pass: a larger value lies over a
/// smaller one. A layer egui does not list stands lowest.
pub(crate) fn layer_position(ctx: &Context, layer: LayerId) -> usize {
    ctx.memory(|memory| memory.layer_ids().position(|listed| listed == layer).map_or(0, |at| at + 1))
}

/// The topmost layer under a screen position.
pub(crate) fn layer_at(ctx: &Context, point: Pos2) -> Option<LayerId> {
    ctx.layer_id_at(point)
}

/// The layer of a transformed body: a sublayer of `parent` in the parent's order, named by `root`.
pub(crate) fn body_layer(parent: LayerId, root: Id) -> LayerId {
    LayerId::new(parent.order, crate::runtime::memory::sub(root, "body_layer"))
}

/// What a transformed body's forwarding widget carries: the root's sense and interact rect, in screen space.
pub(crate) struct Forward {
    pub id: Id,
    pub sense: egui::Sense,
    pub rect: Rect,
}

/// A transformed body's layer: `layer` is shown through `transform`, a sublayer of `parent`, with its top left at
/// `at` in its own space and clipped to `clip` in screen space.
pub(crate) struct BodyLayer {
    pub parent: LayerId,
    pub layer: LayerId,
    pub transform: TSTransform,
    pub at: Pos2,
    pub clip: Rect,
    /// Registered on the layer first, so an allocating `Ui` on the layer cannot hide the root under egui's
    /// layer-cover rule.
    pub forward: Option<Forward>,
}

/// Shows a transformed body in its layer.
pub(crate) fn show_body_layer<R>(ctx: &Context, body_layer: BodyLayer, body: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let BodyLayer { parent, layer, transform, at, clip, forward } = body_layer;
    ctx.set_transform_layer(layer, transform);
    ctx.set_sublayer(parent, layer);
    egui::Area::new(layer.id)
        .order(layer.order)
        .pivot(egui::Align2::LEFT_TOP)
        .fixed_pos(at)
        .constrain(false)
        .sense(egui::Sense::hover())
        .show(ctx, |ui| {
            if let Some(forward) = forward {
                ui.interact(transform.inverse() * forward.rect, forward.id, forward.sense);
            }
            ui.set_clip_rect(transform.inverse() * clip);
            body(ui)
        })
        .inner
}

/// The transform of a body zoomed by `scale` about its top left `at` and moved by `offset`.
pub(crate) fn body_transform(at: Pos2, scale: f32, offset: Vec2) -> TSTransform {
    TSTransform::new(at.to_vec2() * (1.0 - scale) + offset, scale)
}

/// An egui layer, as the runtime's shape records name it.
pub type LayerKey = egui::LayerId;

/// How an overlay layer ranks among egui's orders: the one place the runtime chooses an `Order`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tier {
    /// Floating windows and panels.
    Window,
    /// Toasts, popups, modals and their chrome.
    Foreground,
    /// Tooltips and the ghost layer.
    Tooltip,
}

impl Tier {
    fn order(self) -> Order {
        match self {
            Tier::Window => Order::Middle,
            Tier::Foreground => Order::Foreground,
            Tier::Tooltip => Order::Tooltip,
        }
    }
}

/// What the overlay host asks of an `Area`.
pub(crate) struct LayerSpec {
    pub id: Id,
    pub tier: Tier,
    /// The physical left-top of the layer.
    pub at: Pos2,
    /// Whether the layer takes input; a tooltip and the ghost layer never do.
    pub interactable: bool,
}

/// What `show_layer` reports.
pub(crate) struct LayerShown {
    /// The layer's rect in screen space.
    pub rect: Rect,
    /// The layer's egui id, as `Context::layer_id_at` names it.
    pub layer: LayerId,
}

/// Shows an overlay layer: an `Area` with its left-top pivot at `spec.at`, never egui's default pivot.
pub(crate) fn show_layer(ctx: &Context, spec: &LayerSpec, body: impl FnOnce(&mut egui::Ui)) -> LayerShown {
    let shown = egui::Area::new(spec.id)
        .order(spec.tier.order())
        .pivot(egui::Align2::LEFT_TOP)
        .fixed_pos(spec.at)
        .constrain(false)
        .interactable(spec.interactable)
        .show(ctx, body);
    LayerShown { rect: shown.response.rect, layer: shown.response.layer_id }
}

/// Shows a layer that covers `screen` and paints `scrim` on it, around `hole` when there is one. While `absorbs` it
/// takes every pointer hit on it, so nothing below it is hit.
pub(crate) fn show_cover_layer(ctx: &Context, id: Id, tier: Tier, screen: Rect, scrim: Color32, hole: Option<Rect>, absorbs: bool) {
    let scrim = Some(scrim);
    egui::Area::new(id).order(tier.order()).pivot(egui::Align2::LEFT_TOP).fixed_pos(screen.min).constrain(false).interactable(absorbs).show(
        ctx,
        |ui| {
            if let Some(scrim) = scrim {
                let painter = ui.painter();
                match hole.map(|hole| hole.intersect(screen)).filter(|hole| hole.is_positive()) {
                    Some(hole) => {
                        let bands = [
                            Rect::from_min_max(screen.min, Pos2::new(screen.right(), hole.top())),
                            Rect::from_min_max(Pos2::new(screen.left(), hole.bottom()), screen.max),
                            Rect::from_min_max(Pos2::new(screen.left(), hole.top()), Pos2::new(hole.left(), hole.bottom())),
                            Rect::from_min_max(Pos2::new(hole.right(), hole.top()), Pos2::new(screen.right(), hole.bottom())),
                        ];
                        for band in bands {
                            painter.rect_filled(band, 0.0, scrim);
                        }
                    }
                    None => {
                        painter.rect_filled(screen, 0.0, scrim);
                    }
                }
            }
            ui.allocate_rect(screen, Sense::click_and_drag());
        },
    );
}

/// The rect an overlay layer had when it was last shown, in screen space.
pub(crate) fn layer_rect(ctx: &Context, id: Id) -> Option<Rect> {
    ctx.memory(|memory| memory.area_rect(id))
}

/// The viewport's safe rect: the room overlays must stay inside.
pub(crate) fn safe_rect(ctx: &Context) -> Rect {
    ctx.content_rect()
}

/// Hands egui the whole layer order of the running pass, bottom first. egui consumes it by the end of the pass.
pub(crate) fn set_layer_order(ctx: &Context, order: &[LayerId]) {
    ctx.set_layer_order(order);
}

/// Makes `layer` the modal layer of the running pass: layers below it take no input.
pub(crate) fn set_modal_layer(ctx: &Context, layer: LayerId) {
    ctx.memory_mut(|memory| memory.set_modal_layer(layer));
}

/// The layer id an overlay layer named `id` has in `tier`.
pub(crate) fn layer_of(tier: Tier, id: Id) -> LayerId {
    LayerId::new(tier.order(), id)
}

/// The mapping from `layer`'s space to the screen's; the identity for a layer with no transform.
pub(crate) fn layer_to_screen(ctx: &Context, layer: LayerId) -> TSTransform {
    ctx.layer_transform_to_global(layer).unwrap_or(TSTransform::IDENTITY)
}

/// Limits `ui` to `max` and at least `min_width` across: the room the placement leaves the layer's body.
pub(crate) fn bound(ui: &mut egui::Ui, max: Vec2, min_width: f32) {
    ui.set_max_size(max);
    ui.set_min_width(min_width);
}

/// Fades everything painted in `ui` to `opacity`.
pub(crate) fn fade(ui: &mut egui::Ui, opacity: f32) {
    ui.set_opacity(opacity);
}

/// How a painted cursor is drawn: from the theme's tokens, so the ring reads on any canvas.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CursorInk {
    pub color: Color32,
    pub width: f32,
    /// The half-length of a crosshair's arms.
    pub arm: f32,
}

/// The painter of the ghost layer, over the whole screen and above every popup. The layer takes no input.
pub(crate) fn ghost_painter(ctx: &Context) -> Painter {
    ctx.layer_painter(layer_of(Tier::Tooltip, crate::runtime::egui::memory::unique("craft_ui.ghost")))
}

/// Paints a cursor shape at `at` on the ghost layer, above every other layer.
pub(crate) fn paint_cursor(ctx: &Context, at: Pos2, cursor: &craft_behavior::PaintedCursor, ink: CursorInk) {
    use craft_behavior::PaintedCursor;
    let painter = ghost_painter(ctx);
    let stroke = egui::Stroke::new(ink.width, ink.color);
    match cursor {
        PaintedCursor::Circle { radius, .. } => {
            painter.circle_stroke(at, *radius, stroke);
        }
        PaintedCursor::Crosshair => {
            painter.line_segment([at - Vec2::new(ink.arm, 0.0), at + Vec2::new(ink.arm, 0.0)], stroke);
            painter.line_segment([at - Vec2::new(0.0, ink.arm), at + Vec2::new(0.0, ink.arm)], stroke);
        }
        _ => {}
    }
}

/// The pointer position of the running pass, in screen space.
pub(crate) fn pointer_pos(ctx: &Context) -> Option<Pos2> {
    ctx.input(|input| input.pointer.latest_pos())
}

/// The layer order after `Context::set_layer_order`.
#[cfg(feature = "testing")]
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LayerOrder {
    /// The listed layer is the topmost of the layers shown.
    pub listed_on_top: bool,
    /// The unlisted layers keep their relative order.
    pub unlisted_keep_order: bool,
    /// A listed id that is no layer does not become one.
    pub unknown_ignored: bool,
}

/// Two frames over three areas; the second lists the first area and an unknown layer.
#[cfg(feature = "testing")]
pub fn layer_order() -> LayerOrder {
    let ctx = super::fixture::bare_context();
    let ids = [Id::unique("craft_ui.surface.la"), Id::unique("craft_ui.surface.lb"), Id::unique("craft_ui.surface.lc")];
    let layers = ids.map(|id| LayerId::new(Order::Middle, id));
    let ghost = LayerId::new(Order::Middle, Id::unique("craft_ui.surface.ghost"));
    let frame = |time: f64, list: &[LayerId]| {
        let mut output = ctx.run_ui(super::surface::raw(time, Vec::new()), |ui| {
            let ctx = ui.ctx().clone();
            for id in ids {
                egui::Area::new(id).show(&ctx, |ui| {
                    ui.label("layer");
                });
            }
            if !list.is_empty() {
                ctx.set_layer_order(list);
            }
        });
        output.textures_delta.clear();
        ctx.memory(|memory| memory.layer_ids().filter(|layer| layers.contains(layer) || *layer == ghost).collect::<Vec<_>>())
    };
    let before = frame(1.0, &[]);
    let after = frame(2.0, &[layers[0], ghost]);
    let rest = |order: &[LayerId]| order.iter().copied().filter(|layer| *layer != layers[0]).collect::<Vec<_>>();
    LayerOrder {
        listed_on_top: after.last() == Some(&layers[0]),
        unlisted_keep_order: rest(&before) == rest(&after),
        unknown_ignored: !after.contains(&ghost),
    }
}
