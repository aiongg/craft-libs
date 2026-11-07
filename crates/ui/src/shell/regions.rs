//! The data types of the window skeleton: slots, window keys, the body mode and the rects a shell pass fills.

use crate::contract::PaneEdge;
use crate::runtime::env::Label;
use craft_theme::SizeTokens;
use egui::Rect;
use std::fmt;

/// The stable key of a slot.
pub type SlotKey = &'static str;

/// The key of a window. It equals the placement `SaveKey` of the window, so two windows of one app need different keys.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WindowKey(String);

impl WindowKey {
    /// The first window, `"main"`.
    pub fn main() -> Self {
        Self::new("main")
    }

    /// A key from any string.
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }

    /// The key as a string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for WindowKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for WindowKey {
    fn from(key: &str) -> Self {
        Self::new(key)
    }
}

/// What the window shows besides its body.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum BodyMode {
    /// The slots show as their `visible` flags say.
    #[default]
    Normal,
    /// No slot shows, floating windows and document tabs hide, and the backdrop is black. A reveal shows the slots built
    /// with [`Slot::reveal`].
    Only,
}

/// The size of a slot along its edge: the height of a top or bottom slot, the width of a start or end slot. Each names a
/// size of the theme.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SlotSize {
    /// `size.bar.title`.
    TitleBar,
    /// `size.bar.tab`.
    TabBar,
    /// `size.bar.status`.
    StatusBar,
    /// `size.bar.options`.
    OptionsBar,
    /// `size.rail.width`.
    RailWidth,
}

impl SlotSize {
    /// The size in logical pixels under `sizes`.
    pub fn px(self, sizes: &SizeTokens) -> f32 {
        match self {
            SlotSize::TitleBar => sizes.bar.title,
            SlotSize::TabBar => sizes.bar.tab,
            SlotSize::StatusBar => sizes.bar.status,
            SlotSize::OptionsBar => sizes.bar.options,
            SlotSize::RailWidth => sizes.rail.width,
        }
    }
}

/// One bar or rail of the skeleton: its key, whether it shows, whether it shows in a reveal, its size and its AccessKit
/// name. The slot's body draws its own surface and edge.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Slot {
    key: SlotKey,
    size: Option<SlotSize>,
    narrow: bool,
    columns: usize,
    visible: bool,
    reveal: bool,
    label: Option<Label>,
}

impl Slot {
    /// A slot with a key that is stable and unique in the window. It is visible and does not show in a reveal. Without a
    /// [`size`](Self::size) it takes the default of its edge: the title bar, options bar, status bar or rail width.
    pub fn new(key: SlotKey) -> Self {
        Self { key, size: None, narrow: false, columns: 1, visible: true, reveal: false, label: None }
    }

    /// The size along the slot's edge, in place of the default of the edge.
    pub fn size(mut self, size: SlotSize) -> Self {
        self.size = Some(size);
        self
    }

    /// A top slot takes the narrow options bar height (`size.control.md` plus the tight gap on both sides) instead of
    /// `size.bar.options`. Has no effect on the title bar or on a slot with a [`size`](Self::size). The slot only sizes
    /// itself: the host passes the same choice to the options bar it draws in it.
    pub fn narrow(mut self, narrow: bool) -> Self {
        self.narrow = narrow;
        self
    }

    /// The number of button columns of a start or end rail, clamped to 1 and 2: 1 is `size.rail.width`. Has no effect on
    /// a slot with a [`size`](Self::size). The slot only sizes itself: the host passes the same count to the rail it draws
    /// in it.
    pub fn columns(mut self, columns: usize) -> Self {
        self.columns = columns.clamp(1, 2);
        self
    }

    /// Whether the slot shows in normal mode: the host's flag, passed every frame. A slot that is not visible is not
    /// built.
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// The slot shows during a reveal in body-only mode, whatever its `visible` flag.
    pub fn reveal(self) -> Self {
        self.reveal_if(true)
    }

    /// [`reveal`](Self::reveal) when `reveal` is true. A title slot passes the host's menu bar flag, so it reveals only
    /// when the menu bar is on.
    pub fn reveal_if(mut self, reveal: bool) -> Self {
        self.reveal = reveal;
        self
    }

    /// The name of the slot's AccessKit region ("Options", "Tools", "Status").
    pub fn label(mut self, label: impl Into<Label>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// The slot's key, as visibility flags and [`ShellCx::slot_visible`](super::ShellCx::slot_visible) name it.
    pub fn key(&self) -> SlotKey {
        self.key
    }

    /// The size set by [`size`](Self::size), if any.
    pub fn slot_size(&self) -> Option<SlotSize> {
        self.size
    }

    /// Whether the slot takes the narrow options bar height.
    pub fn is_narrow(&self) -> bool {
        self.narrow
    }

    /// The number of rail columns, 1 or 2.
    pub fn column_count(&self) -> usize {
        self.columns
    }

    /// The host's visibility flag for normal mode.
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Whether the slot shows during a reveal in body-only mode.
    pub fn reveals(&self) -> bool {
        self.reveal
    }

    /// The name of the slot's AccessKit region, if set.
    pub fn name(&self) -> Option<&Label> {
        self.label.as_ref()
    }
}

/// What one shell pass laid out.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ShellRegions {
    /// The body's rect: the dock host's rect.
    pub body: Rect,
    /// The rects of the shown slots, in the order drawn.
    pub slots: Vec<(SlotKey, Rect)>,
    /// Slots show because of a reveal (Tab or edge) on this pass.
    pub revealed: bool,
}

/// The index of `edge` among the edges with an edge reveal (top, start, end), `None` for the bottom edge and the centre,
/// which never reveal.
pub(crate) fn reveal_index(edge: PaneEdge) -> Option<usize> {
    match edge {
        PaneEdge::Top => Some(0),
        PaneEdge::Start => Some(1),
        PaneEdge::End => Some(2),
        _ => None,
    }
}

impl ShellRegions {
    /// A layout with `body` and no slots, and no reveal.
    pub fn new(body: Rect) -> Self {
        ShellRegions { body, slots: Vec::new(), revealed: false }
    }

    /// The layout with the shown slot `key` at `rect`, after the slots already listed.
    pub fn with_slot(mut self, key: SlotKey, rect: Rect) -> Self {
        self.slots.push((key, rect));
        self
    }
}
