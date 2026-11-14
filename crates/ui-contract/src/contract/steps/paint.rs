//! Step 13, paint: the root surface, state fill and focus ring around `C::paint`.

use crate::contract::Pointer as _;
use crate::contract::cx::CxCore;
use crate::primitives::{FocusKind, FocusRing, PaintShape, StateLayer, StateLayerKind, surface_look_shapes};
use crate::runtime::env::Env;
use crate::style::{Ink, StateSet, StyleView};
use egui::{Color32, LayerId, Ui};

/// The ink the surface reads in: the surface kind of the state, else `inherited`.
pub(crate) fn surface_ink(look: Option<&crate::style::SurfaceLook>, state: &StateSet, inherited: Ink) -> Ink {
    look.and_then(|look| look.kind_in(state)).map_or(inherited, |kind| kind.ink())
}

/// Step 13 for a component that is `faded` (`DisabledLook::Faded`, not enabled): multiplies `ui`'s opacity by
/// `Opacity::DISABLED` until the caller sets it back. Returns the opacity before, which the focus ring, step 14 and the
/// children `PaintUi` shows paint at. egui fades a shape as it is added, so a run record keeps the faded shapes.
// P3(framework-01-w5): fx.F29: the opacity form, replaced by `fade` and `unfade` with their callers.
pub(crate) fn fade_opacity(ui: &mut Ui, core: &mut CxCore<'_>) -> f32 {
    let before = core.unfaded.unwrap_or_else(|| ui.opacity());
    if core.faded {
        ui.set_opacity(before * craft_theme::Opacity::DISABLED);
        core.unfaded = Some(before);
    }
    before
}

/// Where a faded component's shapes start in its layer's paint list, and the ground they fade toward.
#[derive(Clone, Copy)]
#[allow(dead_code)] // P3(framework-01-w5): fx.F29
pub(crate) struct Fade {
    layer: LayerId,
    from: usize,
    ground: Color32,
}

/// Step 13 for a component that is `faded`: notes where its shapes start in its layer's paint list and the ground
/// colour, and stores the fade in `core.fade` for `PaintUi`.
#[allow(dead_code)] // P3(framework-01-w5): fx.F29
pub(crate) fn fade(ui: &Ui, core: &mut CxCore<'_>) -> Option<Fade> {
    let _ = (ui, core);
    todo!("fx.F29")
}

/// Mixes the colour of every shape added since `fade` toward its ground by `Opacity::DISABLED`; textured meshes and
/// brushed rects fade by alpha, callbacks and placeholder colours are left.
#[allow(dead_code)] // P3(framework-01-w5): fx.F29
pub(crate) fn unfade(ui: &Ui, core: &mut CxCore<'_>, fade: Option<Fade>) {
    let _ = (ui, core, fade);
    todo!("fx.F29")
}

/// The index the next shape `ui` paints gets in its layer's paint list.
pub(crate) fn next_shape(ui: &Ui) -> usize {
    ui.ctx().graphics(|layers| layers.get(ui.layer_id()).map_or(0, |list| list.next_idx().0))
}

/// Step 13, paint, the pipeline's part before `C::paint`: the root surface and the state fill.
pub(crate) fn paint_surface(ui: &Ui, env: &Env, core: &CxCore<'_>, style: &dyn StyleView) {
    let Some(look) = style.surface() else { return };
    let shapes = surface_look_shapes(&env.tokens, look, &core.state, core.rect, env.direction, ui.ctx().pixels_per_point());
    match core.surface_slot {
        Some(slot) => ui.painter().set(slot, egui::Shape::Vec(shapes)),
        None => {
            ui.painter().extend(shapes);
        }
    }
}

/// Step 13, paint, after the root surface: the state layer of every part declared `state_layer`, with control
/// corners, under the component's own paint.
pub(crate) fn paint_part_layers(ui: &Ui, env: &Env, core: &CxCore<'_>) {
    if core.state_layer_parts.is_empty() {
        return;
    }
    let look = crate::style::state_layer_look(&env.tokens);
    let radius = crate::style::UiTokens::corner(env.tokens.shape.radius.control);
    for &(key, index) in &core.state_layer_parts {
        let part = craft_behavior::PartRef { key, index };
        let fill = core.part_state(part).resolve(&look);
        if fill.egui().a() > 0 {
            ui.painter().extend(StateLayer::shapes(StateLayerKind::Fill, fill, core.part(part).rect, radius, 0.0));
        }
    }
}

/// Step 13, paint, after `C::paint`: the focus ring of the focused component or its active part, when focus is
/// keyboard-visible. Never on `Well` surfaces.
pub(crate) fn paint_focus(ui: &Ui, core: &CxCore<'_>, style: &dyn StyleView) {
    let Some(look) = style.focus() else { return };
    let ppp = ui.ctx().pixels_per_point();
    let flags = core.state.flags;
    let well = style.surface().is_some_and(|surface| surface.kind == Some(crate::style::SurfaceKind::Well));
    if let Some(part) = core.active.filter(|_| FocusRing::visible(flags)) {
        let rect = core.part(part).rect;
        ui.painter().extend(FocusRing::shapes(look, FocusKind::Ring, &PaintShape::Rect(rect, egui::CornerRadius::ZERO), ppp));
    } else if FocusRing::visible(flags) && !well {
        let radius = style.surface().map_or(egui::CornerRadius::ZERO, |surface| surface.radius);
        ui.painter().extend(FocusRing::shapes(look, FocusKind::Ring, &PaintShape::Rect(core.rect, radius), ppp));
    }
}
