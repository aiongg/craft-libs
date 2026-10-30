//! [`Slot`]: one bar or rail of the skeleton.

use egui::Frame;

/// The side a slot lies against.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Edge {
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
        Self { key, size: None, visible: true, reveal: false, label: None, frame: None }
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
