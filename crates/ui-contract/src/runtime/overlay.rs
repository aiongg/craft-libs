//! The layer stack (15.2): the record of the egui layers shown, their order and the open-modal fact.
//!
//! The stack owns no open state. The overlay host records an entry for each layer it shows; entries recorded in a
//! pass are read in the next one, and an entry not recorded for a frame is gone.

use crate::runtime::egui::areas::{self, Layer};

use crate::runtime::shift::Shift;
use craft_behavior::OwnerClass;
use craft_behavior::overlay::Side;

use egui::{Id, IdMap, IdSet, Pos2, Rect};

/// A modal's backdrop.
#[derive(Clone, Copy, Debug, PartialEq, crate::contract::MemoProps)]
#[non_exhaustive]
pub enum Backdrop {
    /// No scrim; the layer still blocks input below it.
    None,
    /// One scrim over the viewport.
    Scrim,
}

/// An overlay's place in the order. `Rank` is the only place a layer's egui order is chosen.
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

    /// Families of one egui order sort by this class: toast below popup below modal below modal chrome.
    const fn class(self) -> u8 {
        match self {
            Rank::Window => 0,
            Rank::Toast => 1,
            Rank::Popup => 2,
            Rank::Modal { .. } => 3,
            Rank::ModalChrome => 4,
            Rank::Tooltip => 5,
        }
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

/// One shown egui layer.
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
    /// The layer's egui id and the cover under it, listed in the order.
    pub(crate) layer: Layer,
    pub(crate) cover: Option<Layer>,
    /// The layer has a focus scope.
    pub(crate) scoped: bool,
    /// A press outside its family closes it and goes no further.
    pub(crate) absorbs: bool,
    /// A command run from outside it closes it (`Dismiss::commands`).
    pub(crate) closes_on_commands: bool,
    /// The entry is an OS window of the integration: it takes its place in the stack's order and facts, and no egui
    /// layer is shown for it.
    pub(crate) window: bool,
    /// The entry stands in its opener's stack for a layer shown in a popup window of its own (with `window`): the
    /// family, the anchor and the presses in the opener's window are judged here, and the popup's own stack holds the
    /// layer, so the entry blocks nothing here.
    pub(crate) popup: bool,
    /// The screen rect the layer is contained in: its scrim, placement, clip and input blocking stop at it.
    pub(crate) region: Option<Rect>,
    /// The layer's place on its dialog stack (`stack_place`): the modal families of one stack stack in its order.
    pub(crate) place: Option<usize>,
    /// The dialog of a dialog host the layer shows for (`stack_place`).
    dialog: Option<Id>,
}

/// A replay moves the entry's screen rects with its record; the rest stays.
impl Shift for OpenEntry {
    fn shifted(&self, by: &egui::RecordShift) -> Self {
        OpenEntry {
            anchor: self.anchor.shifted(by),
            peers: self.peers.shifted(by),
            rect: self.rect.shifted(by),
            region: self.region.shifted(by),
            ..self.clone()
        }
    }

    fn snapped(&self, to: &Self, within: f32) -> Self {
        OpenEntry {
            anchor: self.anchor.snapped(&to.anchor, within),
            peers: self.peers.snapped(&to.peers, within),
            rect: self.rect.snapped(&to.rect, within),
            region: self.region.snapped(&to.region, within),
            ..self.clone()
        }
    }
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
            cover: None,
            scoped: false,
            absorbs: false,
            closes_on_commands: true,
            window: false,
            popup: false,
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

#[derive(Default)]
pub(crate) struct Persist {
    raised: IdMap<u64>,
    counter: u64,
    /// The viewport's place in the app's modal stack: the root frame its top live modal layer over the whole window
    /// first showed in, as of its last finished pass, or `None` without one. A modal viewport blocks the viewports whose
    /// top modal opened in an earlier frame and those with none (`modal_viewport`). Modals that open in the same frame
    /// form one level and block none of each other: the screen picker opens a capture window over each monitor in one
    /// pass, and each must take its click. When the top modal closes, the one under it keeps its own first frame.
    modal: Option<u64>,
    /// The layers of the last pass, bottom to top, as `end_pass` handed them to egui.
    stack: Vec<Id>,
    /// A press the modal drop removed, for the window host to raise the modal's window.
    dropped_press: bool,
    /// The screen rects a press may land in without closing the top menu family: the family and its anchors. `None`
    /// when the top layer does not absorb outside presses.
    absorb_zone: Option<Vec<Rect>>,
    /// An outside press the input hook removed for the running pass.
    outside_press: Option<Pos2>,
    /// The top live modal layer of the last finished pass in this viewport.
    top_modal: Option<TopModal>,
    /// A popup stands over a modal layer here, as of the last finished pass.
    popup_over_modal: bool,
    /// The layers that first showed while the primary button was held, until that press ends.
    held_open: IdSet,
    /// The root frame and pass each shown layer first ran its body in, and for a layer of this window that shows in a
    /// popup window of its own, the pass it first asked for it in; gone with the layer's last entry.
    appeared: IdMap<(u64, u64)>,
    /// The roots whose asked raise ran in a pass where a floating window (re)appeared: egui raises every layer that
    /// shows again, and its area order keeps the old order among the layers raised in one pass, so the raise is asked
    /// again in the next pass.
    raise_again: Vec<Id>,
}
