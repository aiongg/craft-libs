//! When a layer closes by itself (15.4). ui-rt decides the presses outside, an Escape no focused node handled, focus
//! leaving and the user turning away (`LayerSpec`, `craft_ui_rt::DismissReason`); the layer child keeps the reason
//! for its owner. The rules here are the ones that depend on the contract: the anchor going disabled, the timeout,
//! an activation inside, an untaken Enter.

use super::props::Overlay;
use crate::contract::rt::overlay::{BodyClose, LayerScope, layer_closed, opened_at};
use crate::contract::{Component, Cx};
use crate::runtime::commands::FireSource;
use crate::runtime::overlay::CloseReason;
use craft_keymap::Mods;
use craft_ui_rt::{BuildCx, Layer};

/// Why the layer child `key` on `layer` of the owner `cx` builds closes before it shows in this build: a ui-rt
/// dismissal or a close its body decided since the owner's last build, its anchor gone disabled, its timeout run out.
/// A layer with a timeout still running asks for an owner build at its deadline.
pub(crate) fn closes_before_show(cx: &mut BuildCx<'_>, overlay: &Overlay, layer: Layer, key: u64, enabled: bool) -> Option<CloseReason> {
    if let Some(reason) = layer_closed(cx, layer, key) {
        return Some(reason);
    }
    // A popup with no recorded opener hangs from its owner: when the owner goes disabled, its anchor is gone.
    if overlay.dismiss.anchor_gone && overlay.opener.is_none() && !enabled {
        return Some(CloseReason::AnchorGone);
    }
    if let Some(timeout) = overlay.dismiss.timeout
        && let Some(since) = opened_at(cx, layer, key)
    {
        let due = since + f64::from(timeout);
        if cx.now() >= due {
            return Some(CloseReason::Timeout);
        }
        cx.request_build_at(Some(due));
    }
    None
}

/// The close a layer's body decided while it ran in this build (a popup body's or container's close, an activation
/// inside, an untaken Enter), taken.
pub(crate) fn closed_after_show(cx: &BuildCx<'_>, layer: Layer, key: u64) -> Option<CloseReason> {
    cx.peek_layer::<LayerScope>(layer, key).and_then(|p| p.local.closed.take())
}

/// The layers an activation closes (`Dismiss::on_activate`): each open layer that closes on one, from the outermost
/// in. A layer's body runs with the chain of its parent plus its own close, provided on its scope.
#[derive(Clone, Default)]
pub(crate) struct ActivationChain(pub(crate) Vec<(BodyClose, craft_ui_rt::NodeId)>);

/// A component under `cx` was activated: every layer of the chain it runs in closes as `Activated`, unless the
/// activation keeps the chain open (`Cx::keep_open`, an opener toggling its own popup). The driver calls it at step
/// 17 for a fired activation; the owners' overlay passes read the closes on the way out of this build.
pub(crate) fn note_activation(cx: &mut BuildCx<'_>, keep_open: bool) {
    if keep_open {
        return;
    }
    if let Some(chain) = cx.provided::<ActivationChain>() {
        for (closed, owner) in &chain.0 {
            if closed.get().is_none() {
                closed.set(Some(CloseReason::Activated));
                // A body that builds without its layer's owner (a region scope of its own) still reaches the owner's
                // overlay pass, which takes the close.
                if !cx.is_building(*owner) {
                    cx.request_build_of(*owner);
                }
            }
        }
    }
}

/// An opener in the build `cx` runs opened its container's layer by its activation
/// ([`Opener::claims_activation`](super::opener::Opener::claims_activation)): the closes that activation set on the
/// layer chain it runs in are taken back, as `Cx::keep_open` would have kept them. Call it from the container's build,
/// which runs in the same chain as its opener, after the opener showed.
pub(crate) fn claim_activation(cx: &BuildCx<'_>) {
    if let Some(chain) = cx.provided::<ActivationChain>() {
        for (closed, _) in &chain.0 {
            if closed.get() == Some(CloseReason::Activated) {
                closed.set(None);
            }
        }
    }
}

impl<C: Component> Cx<'_, C> {
    /// This activation runs but does not close the layer chain it happened in (`Dismiss::on_activate`).
    pub fn keep_open(&mut self) {
        self.core.keep_open = true;
    }

    /// Injects an activation for this component, from `source`: the pipeline runs its bound command with that
    /// source, reports it to `Component::activated` readers and closes the layer chain as for a click.
    pub fn activate(&mut self, source: FireSource) {
        self.core.injected = Some(source);
    }

    /// The modifiers held this frame, Alt alone included. A tracked input: a memoized view that reads them runs when
    /// they change.
    pub fn modifiers(&self) -> Mods {
        self.core.context().modifiers()
    }
}
