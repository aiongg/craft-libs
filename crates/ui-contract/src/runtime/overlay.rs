//! The overlay values (15.2): ranks, close reasons, layer roles and the entry each shown layer keeps for the layer
//! queries. The stack itself is ui-rt's: each window's layers and their children, in order (`craft_ui_rt::Layer`).
//! A layer child keeps its entry while it shows (`contract::rt::overlay::LayerKept`), so an entry goes with its layer.

use crate::primitives::Id;
use craft_behavior::OwnerClass;
use craft_behavior::overlay::Side;
use craft_ui_rt::{Layer, Rect};

/// A modal's backdrop.
#[derive(Clone, Copy, Debug, PartialEq, crate::contract::MemoProps)]
#[non_exhaustive]
pub enum Backdrop {
    /// No scrim; the layer still blocks input below it.
    None,
    /// One scrim over the viewport.
    Scrim,
}

/// An overlay's place in the order. `Rank` is the only place a layer's ui-rt layer is chosen.
#[derive(Clone, Copy, Debug, PartialEq, crate::contract::MemoProps)]
#[non_exhaustive]
pub enum Rank {
    /// A floating window or panel.
    Window,
    /// A toast.
    Toast,
    /// A popup, menu or dropdown.
    Popup,
    /// A modal layer; it blocks input below it.
    #[non_exhaustive]
    Modal {
        /// The scrim behind it.
        backdrop: Backdrop,
    },
    /// The chrome of a modal that stays above its popups.
    ModalChrome,
    /// A tooltip.
    Tooltip,
}

impl Rank {
    /// A modal rank with `backdrop`.
    pub const fn modal(backdrop: Backdrop) -> Rank {
        Rank::Modal { backdrop }
    }

    /// Whether a layer of this rank takes keys without a focus scope. Modal chrome takes none: Cancel and Enter stay
    /// with the modal under it.
    pub(crate) const fn takes_keys(self) -> bool {
        matches!(self, Rank::Popup | Rank::Modal { .. })
    }

    #[doc(hidden)]
    pub const fn is_modal(self) -> bool {
        matches!(self, Rank::Modal { .. })
    }
}

/// Why a layer closes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, crate::contract::MemoProps)]
#[non_exhaustive]
pub enum CloseReason {
    /// Code asked for it.
    Requested,
    /// Escape reached the top layer.
    Escape,
    /// A press landed outside the layer and its anchor.
    Outside,
    /// Focus left the layer, or the anchor of a layer that keeps the focus on its anchor; the move says what moved it.
    FocusLeft(crate::runtime::focus::FocusMove),
    /// A component inside was activated.
    Activated,
    /// Enter was pressed inside.
    Enter,
    /// The timeout ran out.
    Timeout,
    /// The anchor is no longer shown.
    AnchorGone,
}

/// What a layer is for, beyond its rank.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, crate::contract::MemoProps)]
#[non_exhaustive]
pub enum LayerRole {
    /// An ordinary layer.
    #[default]
    Plain,
    /// A progress window: `any_open` with `OpenFilter::Asks` leaves it out, as it asks the user nothing. The other
    /// filters count it, since it still covers the window.
    Progress,
    /// A field over its owner's own content that edits it in place (a page's rename field): it is part of the owner,
    /// so the owner reports no open or expanded state and no popup it controls while it shows.
    InPlaceEditor,
}

/// What `any_open` counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum OpenFilter {
    /// Every layer.
    Any,
    /// Modal layers.
    Modal,
    /// Layers that take keys: popups and modals.
    Keys,
    /// Modal layers that wait for the user's answer: every modal layer except a progress window.
    Asks,
}

/// One shown layer, as the layer queries read it.
#[derive(Clone, Debug, PartialEq)]
#[doc(hidden)]
pub struct OpenEntry {
    /// The layer's id.
    pub id: Id,
    /// The layer it was opened from inside.
    pub parent: Option<Id>,
    /// Its rank.
    pub rank: Rank,
    /// Its key class.
    pub keys: OwnerClass,
    /// What the layer is for.
    pub role: LayerRole,
    /// The anchor in screen space.
    pub anchor: Option<Rect>,
    /// The rects in screen space of the other parts of the owner that open popups: a press on one is not outside,
    /// so the press reaches it and it switches the open popup.
    pub peers: Vec<Rect>,
    /// The layer's rect in screen space.
    pub rect: Rect,
    /// The side of the anchor it took by its size; `None` for a float and while its size is unknown.
    pub side: Option<Side>,
    /// The side its host asked for when it took `side`: a later pass keeps `side` only while the host asks for the same.
    pub(crate) asked: Option<Side>,
    /// It is fading out: not `top`, not counted by `any_open`, taking no input.
    pub exiting: bool,
    /// The ui-rt layer it shows on.
    pub(crate) layer: Layer,
    /// The layer has a focus scope.
    pub(crate) scoped: bool,
    /// A press outside its family closes it and goes no further.
    pub(crate) absorbs: bool,
    /// A command run from outside it closes it (`Dismiss::commands`).
    pub(crate) closes_on_commands: bool,
    /// The screen rect the layer is contained in: its scrim, placement, clip and input blocking stop at it.
    pub(crate) region: Option<Rect>,
    /// The layer's place on its dialog stack (`stack_place`): the modal families of one stack stack in its order.
    pub(crate) place: Option<usize>,
    /// The dialog of a dialog host the layer shows for (`stack_place`).
    dialog: Option<Id>,
}

impl OpenEntry {
    /// An entry for the layer `id` of `rank`.
    pub(crate) fn new(id: Id, rank: Rank, keys: OwnerClass, rect: Rect, layer: Layer) -> Self {
        OpenEntry {
            id,
            parent: None,
            rank,
            keys,
            role: LayerRole::default(),
            anchor: None,
            peers: Vec::new(),
            rect,
            side: None,
            asked: None,
            exiting: false,
            layer,
            scoped: false,
            absorbs: false,
            closes_on_commands: true,
            region: None,
            place: None,
            dialog: None,
        }
    }

    /// Whether the layer takes keys: the layers `top` can name. A layer with a focus scope takes keys whatever its rank.
    pub(crate) fn takes_keys(&self) -> bool {
        self.scoped || self.rank.takes_keys()
    }
}
