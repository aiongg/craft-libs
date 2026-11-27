//! The canvas reach under a modal layer: the one exception to "a modal blocks the input below it". A modal dialog that
//! shows a canvas keeps it usable, as Photoshop's Color Picker, Levels and Curves do; the dialog names the gestures
//! ([`CanvasGesture`]) that reach the canvas besides the view navigation.

use crate::contract::Context;
use crate::contract::context::log_build_read;
use crate::primitives::Id;
use craft_behavior::PointerButton;
use craft_keymap::Mods;
use craft_ui_rt::{BuildCx, ModalReach};
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

/// What a modal leaves of a canvas below it: the view navigation (the wheel, middle presses, Space and the primary
/// press while Space is held), and the gestures it names.
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

    fn join(&mut self, other: &Reach) {
        for gesture in &other.gestures {
            if !self.gestures.contains(gesture) {
                self.gestures.push(*gesture);
            }
        }
    }
}

/// The window's registrations: the canvas reach the running modal body asks for, and the gestures
/// the canvas's host adds for its running tool.
#[derive(Default)]
struct Registry {
    /// One frame per modal body running now, innermost last: its canvas and reach so far.
    running: Vec<Option<(Id, Reach)>>,
    /// The tool gestures a canvas's host passes (`pass_tool_gesture`), by canvas, with the frame time of the host's
    /// build that passed them: a later build's passes start the set over.
    tool: Option<(f64, Id, Reach)>,
}

/// A body of a control in a modal layer asks for `reach` on `canvas`. Outside every modal body it
/// holds nothing. The registrations of one run add up.
pub(crate) fn register(ctx: &Context, canvas: Id, reach: Reach) {
    ctx.service::<Registry, _>(|registry| {
        let Some(frame) = registry.running.last_mut() else { return };
        match frame {
            Some((held, gestures)) if *held == canvas => gestures.join(&reach),
            _ => *frame = Some((canvas, reach)),
        }
    });
}

/// The canvas's host adds its running tool's `gesture` to what a modal lets through to `canvas`. The gestures one
/// build of the host passes are the set; a modal over the canvas lets a changed set through in the same frame, with no
/// run of its body. A host whose tool takes no gesture any more passes none and leaves the last set until its next pass.
pub fn pass_tool_gesture(ctx: &Context, canvas: Id, gesture: CanvasGesture) {
    let now = ctx.time();
    let changed = ctx.service::<Registry, _>(|registry| {
        let mut reach = Reach::gesture(gesture);
        if let Some((at, held, passed)) = &registry.tool
            && *at == now
            && *held == canvas
        {
            reach.join(passed);
        }
        let changed = registry.tool.as_ref().is_none_or(|(_, held, passed)| *held != canvas || *passed != reach);
        registry.tool = Some((now, canvas, reach));
        changed
    });
    if changed {
        log_build_read(move |cx| crate::contract::rt::overlay::refresh_reach(cx, canvas));
    }
}

/// A modal layer's body starts to run: what it registers until [`end_modal`] is its reach.
pub(crate) fn begin_modal(ctx: &Context) {
    ctx.service::<Registry, _>(|registry| registry.running.push(None));
}

/// The modal layer's body ran: the canvas it keeps usable and what of it, `None` when it asked for none.
pub(crate) fn end_modal(ctx: &Context) -> Option<(Id, Reach)> {
    ctx.service::<Registry, _>(|registry| registry.running.pop().flatten())
}

/// The ui-rt reach of a modal whose body keeps `body` (its canvas and what of it): the canvas's node, the view
/// navigation, the presses of the body's gestures and of the gestures the canvas's host passes for its tool. `None`
/// when the body keeps nothing or the canvas is not shown in this window.
pub(crate) fn reach_of(ctx: &Context, cx: &BuildCx<'_>, body: Option<&(Id, Reach)>) -> Option<ModalReach> {
    let (canvas, body) = body?;
    let mut reach = body.clone();
    if let Some((_, _, tool)) = ctx.service::<Registry, _>(|registry| registry.tool.clone()).filter(|(_, held, _)| held == canvas) {
        reach.join(&tool);
    }
    let node = crate::contract::rt::ids::node_of(cx, *canvas)?;
    Some(reach.gestures.iter().fold(ModalReach::view(node), |modal, g| modal.with_press(g.button, g.mods)))
}
