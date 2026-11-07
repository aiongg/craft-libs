//! What may happen to a panel, and why an edit was refused.

use std::fmt;

use super::action::Placement;

/// What may happen to a panel. [`Layout::apply_with_permissions`] enforces these for every
/// action, whoever issues it. They also decide which gestures and menu items the renderer
/// offers; a renderer hides the built-in close controls of a panel that stays closable by command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Permissions {
    /// The panel may be moved.
    pub movable: bool,
    /// The panel may be floated.
    pub floatable: bool,
    /// The panel may be closed.
    pub closable: bool,
    /// Other panels may join this panel's group as tabs.
    pub accepts_tabs: bool,
    /// Other panels may split this panel's frame (`Placement::Split` with a side zone).
    /// Outer-edge frames ([`Placement::Edge`]) are not splits of the anchor.
    pub accepts_splits: bool,
}

impl Default for Permissions {
    fn default() -> Self {
        Self { movable: true, floatable: true, closable: true, accepts_tabs: true, accepts_splits: true }
    }
}

impl Permissions {
    /// Permissions with each right given in field order: movable, floatable, closable, accepts tabs, accepts splits.
    pub const fn new(movable: bool, floatable: bool, closable: bool, accepts_tabs: bool, accepts_splits: bool) -> Self {
        Self { movable, floatable, closable, accepts_tabs, accepts_splits }
    }

    /// A document canvas: it stays docked and open, and takes neither tabs nor side splits.
    /// A group dropped on it floats there. An application that opens panels beside its canvas
    /// uses `Permissions::new(false, false, false, false, true)`.
    pub const PROTECTED: Self = Self { movable: false, floatable: false, closable: false, accepts_tabs: false, accepts_splits: false };

    /// A canvas: it can neither move, close nor take tabs. `accepts_splits` and `floatable`
    /// do not matter, so a canvas that opens panels beside it still counts.
    pub fn is_protected(self) -> bool {
        !self.movable && !self.closable && !self.accepts_tabs
    }

    /// Lock Workspace: nothing moves, floats or closes; tab and split drops keep their policy.
    /// A workspace applies this when it is locked; an application with a global lock preference
    /// applies it in its own permissions callback.
    pub fn locked(self) -> Self {
        Self { movable: false, floatable: false, closable: false, ..self }
    }
}

/// Why a layout edit or load was refused. The layout is unchanged when an edit fails.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DockError {
    /// The layout is deeper or larger than [`MAX_DEPTH`] or [`MAX_PANELS`] allow.
    LimitExceeded,
    /// A panel occurs in more than one place.
    DuplicatePanel,
    /// The panel to act on is not in the layout.
    MissingPanel,
    /// The anchor or destination is not in the layout.
    MissingTarget,
    /// An active tab index is out of range.
    InvalidSelection,
    /// A size or rect is not finite or is out of bounds.
    InvalidGeometry,
    /// The destination does not support the placement.
    InvalidPlacement,
    /// The panel's [`Permissions`] forbid the action.
    NotAllowed,
    /// Saved by a newer version of the docking format.
    UnsupportedVersion,
}

impl fmt::Display for DockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::LimitExceeded => "docking layout exceeds its size or depth limit",
            Self::DuplicatePanel => "panel occurs in more than one docking location",
            Self::MissingPanel => "source panel is not present in the docking layout",
            Self::MissingTarget => "docking destination is no longer present",
            Self::InvalidSelection => "active docking tab is out of bounds",
            Self::InvalidGeometry => "docking geometry is not finite and bounded",
            Self::InvalidPlacement => "docking destination does not support this placement",
            Self::NotAllowed => "this panel does not allow the requested docking operation",
            Self::UnsupportedVersion => "docking data was saved by a newer version",
        })
    }
}

impl std::error::Error for DockError {}
