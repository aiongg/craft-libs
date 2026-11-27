//! Step 16's fill of the nodes step 8 created, and of the sub-nodes below them (12.2).

use super::{Numeric, PopupKind, SubNode};
use crate::primitives::id::Id;
use craft_theme::Flags;
use craft_ui_rt::Rect;

/// A popup an anchor shows this pass.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PopupShown {
    /// The popup's layer id (its root node is [`layer_root`]'s).
    pub id: Id,
    pub kind: PopupKind,
    pub open: bool,
}

/// A sub-node and what the pipeline knows about its part.
#[doc(hidden)]
pub struct SubDecl<'a> {
    pub owner: Id,
    pub component: &'static str,
    pub node: &'a SubNode,
    pub rect: Rect,
    pub enabled: bool,
    pub focusable: bool,
    pub clickable: bool,
    /// The part is the component's active part: AccessKit focus goes to it.
    pub active: bool,
    pub face: Option<FaceFacts<'a>>,
    /// The component's label, the name of a sub-node with `Name::FromLabel` and no face.
    pub label: &'a str,
    /// The node the sub-node attaches to.
    pub parent: Id,
}

/// What the face painted for a part adds to the part's sub-node.
#[doc(hidden)]
pub struct FaceFacts<'a> {
    pub name: &'a str,
    pub flags: Flags,
    pub numeric: Option<&'a Numeric>,
    pub expanded: Option<bool>,
    pub value: Option<&'a str>,
    pub color: Option<craft_theme::Srgba>,
}
