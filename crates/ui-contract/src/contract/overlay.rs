//! Chapter 15: the `Overlay` data, its presets, popup handles and `OverlayUi`.
//!
//! Every overlay is a craft-ui-rt layer child of the node that shows it (`contract::rt::overlay`): `OverlayUi::show`
//! and `show_container` for a component's popups, `show_dialog`, the tooltip and the toast stack. ui-rt places it,
//! orders it on its window's layers and dismisses it; the rules that depend on the contract are in `dismissal`.

pub use crate::runtime::overlay::{LayerRole, OpenFilter};

pub(crate) mod container_open;
mod dismissal;
mod opener;
mod place;
mod placement;
pub(crate) mod popups;
mod props;
mod toggles;
mod ui;

pub use crate::contract::rt::dialog::show_dialog;
pub use container_open::{Popup, opened_from};
pub(crate) use container_open::{hung_from, opener_open, set_opener_open, slot_open};
pub(crate) use dismissal::{ActivationChain, claim_activation, closed_after_show, closes_before_show, note_activation};
pub(crate) use opener::{LayerFloor, anchor_floor, offers_context_menu};
pub use opener::{Opener, Trigger};
pub use place::{Anchor, FloatPlace, OverlayFit, Place, Resize, ResizeEdges, SizeLimits, ViewportShare};
pub use placement::{PopupRegion, layer_room};
pub(crate) use placement::{region_for, screen_for};
pub(crate) use popups::dwell;
pub use popups::{HOLD, Overlays, PopupHandle};
pub use props::{Dismiss, Outside, Overlay, PlacementChange, PlacementKind};
pub(crate) use toggles::{ToggleDecl, drive_toggles, has_toggle_nodes};
pub use ui::OverlayUi;
pub(crate) use ui::{OverlayOwner, run_overlay};
