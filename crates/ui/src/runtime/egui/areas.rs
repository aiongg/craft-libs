//! `Area` with its left-top pivot, `Order`, sublayers, transform layers and `set_layer_order`.

#[cfg(feature = "testing")]
use egui::Ui;
use egui::emath::TSTransform;
use egui::{Context, Id, LayerId, Pos2, Rect, Vec2};

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

    /// A screen-space movement as the layer sees it.
    pub(crate) fn delta_to_layer(self, delta: Vec2) -> Vec2 {
        delta / self.0.scaling
    }
}

/// The mapping of `layer` to the screen, `None` for a layer with no transform.
pub(crate) fn layer_map(ctx: &Context, layer: LayerId) -> Option<LayerMap> {
    ctx.layer_transform_to_global(layer).map(LayerMap)
}

/// The topmost layer under a screen position.
pub(crate) fn layer_at(ctx: &Context, point: Pos2) -> Option<LayerId> {
    ctx.layer_id_at(point)
}

#[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "used by the F5 mount host"))]
/// The layer of a transformed body: a sublayer of `parent` in the parent's order, named by `root`.
pub(crate) fn body_layer(parent: LayerId, root: Id) -> LayerId {
    LayerId::new(parent.order, crate::runtime::memory::sub(root, "body_layer"))
}

#[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "used by the F5 mount host"))]
/// What a transformed body's forwarding widget carries: the root's sense and interact rect, in screen space.
pub(crate) struct Forward {
    pub id: Id,
    pub sense: egui::Sense,
    pub rect: Rect,
}

#[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "used by the F5 mount host"))]
/// Shows a transformed body: `layer` is shown through `transform`, a sublayer of `parent`, with its top left
/// at `at` in its own space. The forwarding widget, when given, is registered on the layer first, so an
/// allocating `Ui` on the layer cannot hide the root under egui's layer-cover rule.
pub(crate) fn show_body_layer<R>(
    ctx: &Context,
    parent: LayerId,
    layer: LayerId,
    transform: TSTransform,
    at: Pos2,
    forward: Option<Forward>,
    body: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
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
            body(ui)
        })
        .inner
}
