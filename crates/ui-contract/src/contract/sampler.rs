//! The canvas reach under a modal layer, as a control asks for it (15.5, 8.3); the runtime side is
//! `runtime::sampler`, and ui-rt's `ModalReach` lets the input through.

use super::{BodyCx, Component, Cx, Id};
pub use crate::runtime::sampler::CanvasGesture;
use crate::runtime::sampler::{self, Reach};

mod sealed {
    use super::{BodyCx, Component, Cx};
    use crate::contract::Context;

    /// What the chapter needs of a context: the window context the registration is made in.
    pub trait Host {
        fn window_context(&self) -> &Context;
    }

    impl<C: Component> Host for Cx<'_, C> {
        fn window_context(&self) -> &Context {
            self.core.context()
        }
    }

    impl Host for BodyCx<'_> {
        fn window_context(&self) -> &Context {
            self.memory.ctx()
        }
    }
}

/// The canvas reach under a modal layer (15.5, 8.3): a control of a dialog keeps the canvas below it usable. It is
/// implemented for `Cx`, `BodyCx` and so for `FormCx`, which derefs to `BodyCx`. A registration belongs to the modal
/// layer whose body calls it and holds while that body is kept: called outside every modal body it holds nothing, and
/// a modal opened over that layer blocks the canvas. The registrations of one run add up. The canvas's host adds its
/// running tool's gestures to a canvas a modal keeps with `Ui::pass_tool_gesture`.
pub trait Sampler: sealed::Host {
    /// Lets `gesture` on `canvas` (the canvas component's id) through the modal layer, and the view navigation with it
    /// (`keep_view_open`): an armed eyedropper's press ([`CanvasGesture::PRIMARY`]), a dialog's drag on the image, a
    /// ⌘-click (`CanvasGesture::PRIMARY.with(Mods::CMD)`). A press that starts no named gesture stays blocked. The
    /// app runs the dialog's own tool on the canvas meanwhile. The control calls it in every run while it takes the
    /// gesture.
    fn pass_canvas_gesture(&mut self, canvas: Id, gesture: CanvasGesture) {
        sampler::register(self.window_context(), canvas, Reach::gesture(gesture));
    }

    /// Keeps the view of `canvas` (the canvas component's id) usable under the modal layer: the wheel scrolls and
    /// zooms it, and a middle press pans it, as in Photoshop's Color Picker and Levels dialogs. Its other input stays
    /// blocked. A dialog body calls it in every run while the dialog is open.
    fn keep_view_open(&mut self, canvas: Id) {
        sampler::register(self.window_context(), canvas, Reach::view());
    }
}

impl<C: Component> Sampler for Cx<'_, C> {}

impl Sampler for BodyCx<'_> {}
