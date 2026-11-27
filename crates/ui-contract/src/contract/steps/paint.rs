//! Step 13, paint, the pipeline's part around `Component::paint`: the root surface, the part state
//! layers and a faded component's veil. All of it records into the look's canvas, so it is part of
//! every recorded look.

use crate::contract::cx::CxCore;
use crate::primitives::{Canvas, StateLayer, StateLayerKind, surface_look_items};
use crate::runtime::env::{Env, EnvValue};
use crate::style::{Ink, StateSet, StyleView, surface_style};
use craft_theme::Opacity;

/// The ink the surface reads in: the surface kind of the state, else `inherited`.
pub(crate) fn surface_ink(look: Option<&crate::style::SurfaceLook>, state: &StateSet, inherited: Ink) -> Ink {
    look.and_then(|look| look.kind_in(state)).map_or(inherited, |kind| kind.ink())
}

/// Where a faded component's paint starts in its look's canvas, and the ground its veil takes.
#[derive(Clone, Copy)]
pub(crate) struct Fade {
    from: crate::primitives::CanvasMark,
    ground: craft_ui_rt::Rgba,
    /// The component lies under the veil of a faded owner that shows it from its paint: it paints its
    /// enabled look and adds no veil of its own.
    covered: bool,
}

/// Set for the children a faded component shows from its paint, which its veil covers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, crate::contract::MemoProps)]
pub(crate) struct UnderVeil(pub bool);

impl EnvValue for UnderVeil {
    fn root() -> Self {
        UnderVeil(false)
    }
}

/// Step 13 for a component that is faded (`DisabledLook::Faded`, not enabled): notes where its paint
/// starts and the ground colour, the surface it sits on, and stores the fade in `core.fade` for
/// `PaintUi`. `None` for a component that is not faded.
pub(crate) fn fade(canvas: &Canvas<'_>, env: &Env, core: &mut CxCore<'_>) -> Option<Fade> {
    if !core.faded {
        core.fade = None;
        return None;
    }
    let covered = env.values.get::<UnderVeil>().0;
    let fade = Fade { from: canvas.mark(), ground: surface_style(&env.tokens, env.surface).fill.into(), covered };
    core.fade = Some(fade);
    Some(fade)
}

/// Lays the veil over what the component painted since `fade`: one rect of the ground at
/// `1 - Opacity::DISABLED` over the component's rect with its surface's corners, so each colour under
/// it shows mixed toward the ground by the disabled share. A component that painted nothing adds no
/// veil.
pub(crate) fn unfade(canvas: &Canvas<'_>, core: &mut CxCore<'_>, fade: Option<Fade>) {
    core.fade = None;
    let Some(fade) = fade.filter(|fade| !fade.covered) else { return };
    if !canvas.painted_since(fade.from) {
        return;
    }
    let radius = core.paint.borrow().surface_radius;
    canvas.rect_filled(core.rect, radius, fade.ground.multiply(1.0 - Opacity::DISABLED));
}

/// The items of the root surface; `None` for a component with no surface.
pub(crate) fn surface_items(env: &Env, core: &CxCore<'_>, style: &dyn StyleView, scale: f32) -> Option<Vec<craft_ui_rt::Item>> {
    let look = style.surface()?;
    if core.faded {
        core.paint.borrow_mut().surface_radius = look.radius;
    }
    let mut items = surface_look_items(&env.tokens, look, &core.state, core.rect, env.direction, scale);
    if let Some(frame) = core.env.frame() {
        crate::primitives::fit_to_layer_frame(&frame, &mut items);
    }
    Some(items)
}

/// Step 13, after the root surface: the state layer of every part declared `state_layer`, with
/// control corners, under the component's own paint.
pub(crate) fn paint_part_layers(canvas: &Canvas<'_>, env: &Env, core: &CxCore<'_>) {
    if core.state_layer_parts.is_empty() {
        return;
    }
    let look = crate::style::state_layer_look(&env.tokens);
    let radius = crate::style::UiTokens::corner(env.tokens.shape.radius.control);
    for &(key, index) in &core.state_layer_parts {
        let part = craft_behavior::PartRef { key, index };
        let fill = core.part_state(part).resolve(&look);
        if fill.egui().a() > 0 {
            canvas.extend(StateLayer::items(StateLayerKind::Fill, fill, core.part(part).rect, radius, 0.0));
        }
    }
}
