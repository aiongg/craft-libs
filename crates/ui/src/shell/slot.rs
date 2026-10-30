//! [`Slot`]: one bar or rail of the skeleton.

use egui::Frame;

/// The side a slot lies against.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Edge {
    Top,
    Bottom,
    /// The reading start: left, or right right to left.
    Start,
    /// The reading end.
    End,
}

/// The stable key of a slot.
pub type SlotKey = &'static str;

/// One bar or rail of the skeleton: its key, whether it shows, whether it shows in a reveal, its
/// exact size and its AccessKit name. The slot's closure draws its own surface and edge.
#[derive(Clone, Debug)]
pub struct Slot {
    pub(super) key: SlotKey,
    pub(super) size: Option<f32>,
    pub(super) narrow: bool,
    pub(super) columns: u8,
    pub(super) visible: bool,
    pub(super) reveal: bool,
    pub(super) label: Option<String>,
    pub(super) frame: Option<Frame>,
}

impl Slot {
    /// A slot with a key that is stable and unique in the window. It is visible and does not
    /// show in a reveal. Without a size it takes the token default of its edge: the title bar,
    /// options bar, status bar or rail width.
    pub fn new(key: SlotKey) -> Self {
        Self { key, size: None, narrow: false, columns: 1, visible: true, reveal: false, label: None, frame: None }
    }

    /// The exact height of a top or bottom slot, the exact width of a start or end slot.
    pub fn size(mut self, size: f32) -> Self {
        self.size = Some(size);
        self
    }

    /// The exact height of a top or bottom slot.
    pub fn height(self, height: f32) -> Self {
        self.size(height)
    }

    /// The exact width of a start or end slot.
    pub fn width(self, width: f32) -> Self {
        self.size(width)
    }

    /// A top slot takes the narrow options bar height (`bar.options_narrow`) instead of the
    /// options bar height. Serves Preferences > Workspace > "Enable Narrow Options Bar". Has no
    /// effect on the title bar or on a slot with an exact [`size`](Self::size).
    pub fn narrow(mut self, narrow: bool) -> Self {
        self.narrow = narrow;
        self
    }

    /// The number of button columns of a start or end rail: 1 (the default) is `rail.width`,
    /// 2 adds one more button column and its gap, so the toolbar's two-column mode is
    /// `rail.width + rail.button + 2` wide. Values above 2 add a column each; 0 counts as 1.
    /// Has no effect on a slot with an exact [`size`](Self::size).
    pub fn columns(mut self, columns: u8) -> Self {
        self.columns = columns.max(1);
        self
    }

    /// Whether the slot shows in normal mode: the host's flag, passed every frame. A slot that
    /// is not visible is not created: no panel, no space, no closure call.
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// The slot shows during a reveal in body-only mode, whatever its `visible` flag.
    pub fn reveal(self) -> Self {
        self.reveal_if(true)
    }

    /// [`reveal`](Self::reveal) when `reveal` is true. A title slot passes the host's menu bar
    /// flag, so it reveals only when the menu bar is on.
    pub fn reveal_if(mut self, reveal: bool) -> Self {
        self.reveal = reveal;
        self
    }

    /// The translated name of the slot's AccessKit region ("Options", "Tools", "Status").
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// The egui frame of the slot's panel. By default none: the slot paints its own surface.
    pub fn frame(mut self, frame: Frame) -> Self {
        self.frame = Some(frame);
        self
    }

    pub fn key(&self) -> SlotKey {
        self.key
    }
}
