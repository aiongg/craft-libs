//! The canvas reach under a modal layer: the one exception to "a modal blocks the input below it". A modal dialog that
//! shows a canvas keeps it usable, as Photoshop's Color Picker, Levels and Curves do; the dialog names the gestures
//! ([`CanvasGesture`]) that reach the canvas besides the view navigation.

use craft_behavior::PointerButton;
use craft_keymap::Mods;
use smallvec::SmallVec;

/// A gesture on the canvas that a dialog takes under its modal layer: a press of `button` with at least `mods` held,
/// and the drag that follows it. Other modifiers may be held too: an eyedropper's Shift and Alt add and subtract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct CanvasGesture {
    /// The button that starts it.
    pub button: PointerButton,
    /// The modifiers it needs held at the press.
    pub mods: Mods,
}

impl CanvasGesture {
    /// A press of the primary button, with any modifiers: an eyedropper, a shadow or pattern drag.
    pub const PRIMARY: CanvasGesture = CanvasGesture::press(PointerButton::Primary);

    /// A press of `button`, with any modifiers.
    pub const fn press(button: PointerButton) -> CanvasGesture {
        CanvasGesture { button, mods: Mods::NONE }
    }

    /// The same gesture, needing `mods` held at the press.
    #[must_use]
    pub const fn with(mut self, mods: Mods) -> CanvasGesture {
        self.mods = mods;
        self
    }
}

/// What a modal leaves of the canvas below it: the view navigation, and the gestures it names.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Reach {
    gestures: SmallVec<[CanvasGesture; 2]>,
}

impl Reach {
    /// The view navigation alone.
    pub(crate) fn view() -> Reach {
        Reach::default()
    }

    /// The view navigation and `gesture`.
    pub(crate) fn gesture(gesture: CanvasGesture) -> Reach {
        Reach { gestures: SmallVec::from_slice(&[gesture]) }
    }
}
