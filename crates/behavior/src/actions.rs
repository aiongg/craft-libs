//! Declared actions: what a part's gestures mean, and what a delivered action carries.

use crate::intent::Intent;
use crate::part::PartRef;
use craft_keymap::Mods;

/// A pointer gesture on a part.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Click {
    /// A primary click.
    Primary,
    /// A double click.
    Double,
    /// A secondary click.
    Secondary,
}

impl Click {
    const fn slot(self) -> usize {
        match self {
            Click::Primary => 0,
            Click::Double => 1,
            Click::Secondary => 2,
        }
    }
}

/// What the gestures on one part mean, declared with the part and kept on its shape.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct PartActions {
    clicks: [Option<Intent>; 3],
    drag: Option<Intent>,
    for_item: bool,
}

impl PartActions {
    /// No declared action.
    pub const NONE: PartActions = PartActions { clicks: [None, None, None], drag: None, for_item: false };

    /// `gesture` means `intent`.
    pub fn on(mut self, gesture: Click, intent: Intent) -> Self {
        self.clicks[gesture.slot()] = Some(intent);
        self
    }

    /// A drag on the part moves by `intent` (`Nudge` or `Move`), which fixes its keyboard counterpart.
    pub fn drags(mut self, intent: Intent) -> Self {
        self.drag = Some(intent);
        self
    }

    /// The collection handles the actions, with the item's key.
    pub const fn for_item(mut self) -> Self {
        self.for_item = true;
        self
    }

    /// The meaning of `gesture`, if declared.
    pub fn get(&self, gesture: Click) -> Option<&Intent> {
        self.clicks[gesture.slot()].as_ref()
    }

    /// The meaning of a drag, if declared.
    pub fn drag(&self) -> Option<&Intent> {
        self.drag.as_ref()
    }

    /// Whether the collection handles the actions.
    pub const fn targets_item(&self) -> bool {
        self.for_item
    }

    /// Every declared intent: the clicks, then the drag.
    pub fn intents(&self) -> impl Iterator<Item = &Intent> {
        self.clicks.iter().chain(std::iter::once(&self.drag)).flatten()
    }

    /// Whether nothing is declared.
    pub fn is_empty(&self) -> bool {
        self.intents().next().is_none()
    }
}

/// Where a delivered action came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FireSource {
    /// A key or a shortcut.
    Key,
    /// A menu row.
    Menu,
    /// The command palette.
    Palette,
    /// The pointer.
    Pointer,
    /// An automation script.
    Automation,
    /// An AccessKit action.
    Access,
}

/// Which handler a delivered action is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Target {
    /// The component itself.
    Own,
    /// One of its parts.
    Part(PartRef),
    /// An item of a collection, and the part of the item the action came from.
    #[non_exhaustive]
    Item {
        /// The item's key hash.
        key: u64,
        /// The item's part, `None` for its own area.
        part: Option<PartRef>,
    },
}

impl Target {
    /// An item of a collection by key hash, and its part.
    pub const fn item(key: u64, part: Option<PartRef>) -> Target {
        Target::Item { key, part }
    }
}

/// One action delivered to its handler.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Delivered {
    /// The action.
    pub intent: Intent,
    /// Who handles it.
    pub at: Target,
    /// The input path that produced it.
    pub source: FireSource,
    /// The modifiers the source held; `Mods::NONE` for AccessKit.
    pub mods: Mods,
    /// Clicks in the gesture, 0 for a source that is no click.
    pub clicks: u8,
}

impl Delivered {
    /// `intent` for `at` from `source`, with no modifiers and no clicks.
    pub fn new(intent: Intent, at: Target, source: FireSource) -> Delivered {
        Delivered { intent, at, source, mods: Mods::NONE, clicks: 0 }
    }

    /// The modifiers held.
    pub fn mods(mut self, mods: Mods) -> Self {
        self.mods = mods;
        self
    }

    /// The click count of the gesture.
    pub fn clicks(mut self, clicks: u8) -> Self {
        self.clicks = clicks;
        self
    }
}
