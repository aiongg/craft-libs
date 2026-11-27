//! How a window opens: its level, its spec and the frame it gets.

use craft_host::target::{WindowChrome, WindowKind};
use craft_window::policy::{Captions, Frame};

/// Where a window stands in the stack of windows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum WindowLevel {
    /// An ordinary window.
    #[default]
    Normal,
    /// A palette: above the app's other windows while the app is active, an ordinary window while an app-modal layer
    /// in another window blocks it, and an ordinary window (behind other apps' windows on Windows) once none of the
    /// app's windows has focus, so it stays shown without covering other apps or the modal. Hidden while the main
    /// window is minimized, except on macOS, where the app stays active and its palettes stay.
    Palette,
    /// Above every other window.
    AlwaysOnTop,
}

/// How an OS window opens (21.2): what it is for, who draws its frame, where it stands and whether it has a taskbar
/// entry. Its size limits, position and resizability are the container's fallback `Overlay`'s (`size_limits`,
/// `placed`, `resizable`).
///
/// A chrome or caption layout the spec leaves unset is the installed decision's for the window's kind
/// (`InstallSpec::frame`); with no decision installed the window has the OS's frame.
#[derive(Clone, Debug, PartialEq, crate::contract::MemoProps)]
#[memo_props(every_pass = "its captions and chrome hold platform types the key cannot read")]
#[non_exhaustive]
pub struct WindowSpec {
    /// What the window is for.
    pub kind: WindowKind,
    /// Who draws the frame; unset takes the installed decision's.
    pub chrome: Option<WindowChrome>,
    /// The caption buttons the app draws when the chrome is `Integrated`. Unset takes the installed decision's when
    /// `chrome` is unset too, and none otherwise.
    pub captions: Option<Captions>,
    /// Where the window stands among the windows.
    pub level: WindowLevel,
    /// Whether the window has a taskbar or Dock entry.
    pub taskbar: bool,
    /// Whether the window opens hidden and appears once its first pass has been laid out and painted.
    pub shown_after_first_paint: bool,
    /// Whether the window takes OS focus when it opens.
    pub takes_focus: bool,
    /// Whether pointer input passes through the window to the window below it.
    pub passes_pointer: bool,
    /// Whether an Escape nothing in the window takes is the Cancel of the window it was opened from (a dock window's):
    /// no component, layer, capture or app widget of the window, and no reader of its intents.
    pub hands_on_cancel: bool,
}

impl WindowSpec {
    /// A tool window (a native dock window): its own title row in place of the OS title bar
    /// (`craft_window::policy::tool_frame`), no caption buttons, a palette level, no taskbar entry.
    pub fn tool() -> Self {
        WindowSpec { kind: WindowKind::Tool, level: WindowLevel::Palette, taskbar: false, ..Self::dialog() }
    }

    /// A popup window: a layer opened from a palette window (a flyout, menu or tooltip) in an OS window of its own. It
    /// has no frame, no taskbar entry and a palette level, and appears once its first pass ran, at the size and place
    /// its layer took in that pass.
    pub fn popup() -> Self {
        WindowSpec { chrome: Some(WindowChrome::Integrated), shown_after_first_paint: true, ..Self::tool() }
    }

    /// A document window with the installed decision's frame.
    pub fn document() -> Self {
        WindowSpec { kind: WindowKind::Document, ..Self::dialog() }
    }

    /// A dialog window with the installed decision's frame.
    pub fn dialog() -> Self {
        WindowSpec {
            kind: WindowKind::Dialog,
            chrome: None,
            captions: None,
            level: WindowLevel::Normal,
            taskbar: true,
            shown_after_first_paint: false,
            takes_focus: true,
            passes_pointer: false,
            hands_on_cancel: false,
        }
    }

    /// Sets what the window is for.
    #[must_use]
    pub fn kind(mut self, kind: WindowKind) -> Self {
        self.kind = kind;
        self
    }

    /// Sets who draws the frame, whatever the installed decision says.
    #[must_use]
    pub fn chrome(mut self, chrome: WindowChrome) -> Self {
        self.chrome = Some(chrome);
        self
    }

    /// Sets the caption buttons.
    #[must_use]
    pub fn captions(mut self, captions: Option<Captions>) -> Self {
        self.captions = captions;
        self
    }

    /// Takes the chrome and the captions from a decided `frame`; a frame without buttons has no captions.
    #[must_use]
    pub fn frame(mut self, frame: &Frame) -> Self {
        self.chrome = Some(chrome_of(frame));
        self.captions = match frame {
            Frame::Integrated(captions) if !captions.is_empty() => Some(captions.clone()),
            _ => None,
        };
        self
    }

    /// Sets the level.
    #[must_use]
    pub fn level(mut self, level: WindowLevel) -> Self {
        self.level = level;
        self
    }

    /// Sets whether the window has a taskbar or Dock entry.
    #[must_use]
    pub fn taskbar(mut self, taskbar: bool) -> Self {
        self.taskbar = taskbar;
        self
    }

    /// Sets whether the window takes OS focus when it opens.
    #[must_use]
    pub fn takes_focus(mut self, takes_focus: bool) -> Self {
        self.takes_focus = takes_focus;
        self
    }

    /// Sets whether pointer input passes through the window to the window below it (a tooltip's).
    #[must_use]
    pub fn passes_pointer(mut self, passes_pointer: bool) -> Self {
        self.passes_pointer = passes_pointer;
        self
    }

    /// Sets whether an Escape nothing in the window takes is the Cancel of the window it was opened from.
    #[must_use]
    pub fn hands_on_cancel(mut self, hands_on_cancel: bool) -> Self {
        self.hands_on_cancel = hands_on_cancel;
        self
    }

    /// Sets whether the window opens hidden and appears after its first paint, so a window that shows a capture or
    /// other content that must be ready does not flash an empty frame.
    #[must_use]
    pub fn shown_after_first_paint(mut self, hidden_until_painted: bool) -> Self {
        self.shown_after_first_paint = hidden_until_painted;
        self
    }
}

/// Who draws the frame `frame` describes.
pub(crate) fn chrome_of(frame: &Frame) -> WindowChrome {
    match frame {
        Frame::Native => WindowChrome::Native,
        Frame::Inset(_) => WindowChrome::Inset,
        Frame::Integrated(_) => WindowChrome::Integrated,
    }
}
