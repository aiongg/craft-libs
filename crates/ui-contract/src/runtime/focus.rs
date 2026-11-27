//! The focus service: stops, scopes and roving groups registered per pass, the focused stop per arena root,
//! focus on press, focus after removal and programmatic focus (spec 8.3, 9.4, 9.5).
//!
//! egui's focus sits on an arena root. The focused member or part is runtime data keyed by that root, and
//! every focus read of component code goes through `is_focused`.

use crate::primitives::Id;
use craft_behavior::PartRef;
pub use craft_behavior::Roving;
pub use policy::{ActivateKeys, EditSet, EndKey, EnterRule, FocusPolicy, FocusRequest, FocusTarget, Initial, KeepKeys, KeyUse, NudgeMods, TabReach};
pub(crate) use registration::{Group, Stop};

use std::sync::Arc;

mod policy;

#[doc(hidden)]
pub mod registration;

/// The shape the focus ring takes around a part.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum FocusShape {
    /// The part's rect.
    Rect,
    /// The circle inside the part's rect.
    Circle,
    /// A closed polygon, its points in screen points.
    Polygon(Vec<craft_ui_rt::Pos2>),
}

/// A focus stop: a component, or one of its parts.
pub(crate) type StopKey = (Id, Option<PartRef>);

/// A registered stop with its place in the scope tree.
#[derive(Clone, Debug, PartialEq, Eq)]
#[doc(hidden)]
pub struct StopRec {
    #[doc(hidden)]
    pub stop: Stop,
    pub(crate) scope: Option<Id>,
    /// The component takes its place in its set from its roving group's members of this pass, which the group's owner
    /// writes into its node at its own report (`group_positions`). Marked at the component's report.
    pub(crate) group_position: bool,
}

impl StopRec {
    /// The record of the focused stop of component `id` with `keys`, from its node's kept facts.
    pub(crate) fn of_kept(id: Id, keys: KeyUse, press_only: bool) -> StopRec {
        let mut stop = Stop::new(id, keys);
        stop.text = keys.owner == craft_behavior::OwnerClass::Text;
        stop.press_only = press_only;
        StopRec { stop, scope: None, group_position: false }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ScopeRec {
    pub(crate) policy: FocusPolicy,
    pub(crate) parent: Option<Id>,
    pub(crate) label: Option<Arc<str>>,
    /// The stop that held focus when the scope was first pushed, or for a scope that does not trap the stop focus last
    /// entered it from, which `restore` returns to.
    pub(crate) opener: Option<StopKey>,
    /// The scope's layer is playing its exit: focus inside it leaves at once, to the opener or nowhere.
    pub(crate) closing: bool,
    /// For a layer's scope: whether this is its first pass since the layer appeared, as the layer's memory says;
    /// none for other scopes, which are new after a gap of a frame.
    pub(crate) entering: Option<bool>,
}

/// What a pick closes before its command runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[doc(hidden)]
pub enum PickedIn {
    /// The layer the pick ran in and the menus the walk out of it crosses (`menus_closed_from`).
    Menus(Id),
    /// The modal layer the pick was made in (the command palette).
    Layer(Id),
}

/// What moved focus onto a stop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FocusOrigin {
    /// A pointer press on the stop.
    Press,
    /// The keyboard (Tab, arrows, Page keys) or a focus request.
    Request,
    /// A scope shown for the first time moved focus to its initial stop (a dialog's first field): a request too, so
    /// it counts as an arrival by key, and a text part selects its whole text so typing replaces it.
    Open,
    /// The focused child of a collection row went away and handed focus back to its row: neither the keyboard nor a
    /// request chose it, so it is no arrival by key and no scroll owner reveals it.
    Removal,
}

/// What a component reads of focus in one pass.
#[derive(Clone, Copy, Default)]
#[doc(hidden)]
pub struct Facts {
    /// `is_focused`.
    #[doc(hidden)]
    pub focused: bool,
    /// `focus_visible`.
    pub(crate) visible: bool,
    /// `active_part`.
    pub(crate) part: Option<PartRef>,
    /// The part focus moved onto in this frame, when the component holds focus on it.
    pub(crate) arrived: Option<PartRef>,
    /// What moved focus onto `arrived`.
    pub(crate) arrived_origin: Option<FocusOrigin>,
    /// The count of focus writes when the facts were read; see `writes`.
    pub(crate) at: u64,
}

/// What moved the key focus.
#[derive(Clone, Copy, Debug, PartialEq, Eq, crate::contract::MemoProps)]
#[non_exhaustive]
pub enum FocusMove {
    /// Keyboard navigation: Tab, Shift+Tab or F6.
    Keys,
    /// A press elsewhere, or code.
    Other,
}
