//! Where layers may go: the window's layer room and the regions that contain popups and shell layers. ui-rt places
//! each layer child after layout (`craft_ui_rt::LayerSpec::place`) from the placement `LayerShow::of` gives it.

use super::place::{OverlayFit, ViewportShare};
use super::props::Overlay;
use crate::contract::Context;
use crate::runtime::env::Env;
use craft_ui_rt::Rect;

/// The screen rect the popups opened in an env are contained in, and the popups opened from those: the menus of a menu
/// bar stay below the bar ([`OverlayUi::contain_popups_below`](super::OverlayUi::contain_popups_below)). `None`, the
/// root's, lets popups take the whole safe rect.
#[derive(Clone, Copy, Debug, Default, PartialEq, crate::contract::MemoProps)]
#[non_exhaustive]
pub struct PopupRegion(pub Option<Rect>);

impl PopupRegion {
    /// Popups contained in `region`; `None` lets them take the whole safe rect.
    pub fn new(region: Option<Rect>) -> Self {
        PopupRegion(region)
    }
}

impl crate::runtime::env::EnvValue for PopupRegion {
    fn root() -> Self {
        PopupRegion(None)
    }
}

/// The region a layer opened in `env` is contained in: for a modal or floating window layer the region of the
/// embedded shell it is opened in (`AppShell::embedded`), for a popup the env's [`PopupRegion`]; `None` for a window's
/// own layers and for toasts and tooltips, which may reach past it.
pub(crate) fn region_for(env: &Env, overlay: &Overlay) -> Option<Rect> {
    match overlay.rank {
        crate::runtime::overlay::Rank::Popup => env.get::<PopupRegion>().0,
        rank if rank.is_modal() || matches!(rank, crate::runtime::overlay::Rank::Window) => {
            env.get::<crate::runtime::env::ShellEntry>().0.and_then(|shell| shell.region())
        }
        _ => None,
    }
}

/// The room a layer opened in the window of `ctx` may take, in its points: the window's content rect. A tracked
/// read: a reader builds again when the window resizes.
pub fn layer_room(ctx: &Context) -> Rect {
    ctx.content_rect()
}

/// The screen rect a layer of `overlay` opened in `env` keeps inside: the layer room ([`layer_room`]), or the part
/// of it a contained layer's region leaves.
pub(crate) fn screen_for(ctx: &Context, env: &Env, overlay: &Overlay) -> Rect {
    let safe = layer_room(ctx);
    region_for(env, overlay).map_or(safe, |region| region.intersect(safe))
}

impl Overlay {
    /// The overlay as a layer opened in `env` shows: a modal or window layer contained in an embedded shell's region
    /// may take the whole region, since the region is what the host has to give; a popup kept in a [`PopupRegion`]
    /// keeps its cap.
    pub(crate) fn opened_in(&self, env: &Env) -> Overlay {
        let shell = self.rank.is_modal() || matches!(self.rank, crate::runtime::overlay::Rank::Window);
        if shell && region_for(env, self).is_some() { self.in_region() } else { self.clone() }
    }

    /// The overlay as a layer contained in a region shows: its content may take the whole region.
    fn in_region(&self) -> Overlay {
        let mut overlay = self.clone();
        if let OverlayFit::ContentCapped(_) = overlay.fit {
            overlay.fit = OverlayFit::ContentCapped(ViewportShare::new(1.0, 1.0));
        }
        overlay
    }
}
