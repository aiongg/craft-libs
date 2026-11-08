//! The canvas reach under a modal layer (15.5, 8.3): the one exception to "a modal blocks the input below it".
//!
//! A modal dialog that shows a canvas keeps it usable, as Photoshop's Color Picker, Levels and Curves do. The
//! dialog registers the canvas every pass with one of two reaches. `Reach::View` lets the view navigation through:
//! the wheel (scroll and zoom) and the middle press that pans. `Reach::Sample` adds the primary press, for the
//! eyedropper a control of the dialog armed. Focus, keys and Escape stay with the dialog. One registration holds per
//! app (the latest), for the root frame it is made in and the next, so a dialog that closes or stops calling lets it
//! lapse.
//!
//! The registration is a fact of the input hook, the layer test and the scrim: the hook keeps the events the reach
//! admits ([`Hole::admits`]), `areas::layer_at` answers the canvas's layer inside the [`Hole`], and the scrim leaves
//! the [`Hole`] clear and unabsorbed.

use crate::runtime::egui::input::RawInput;
use crate::runtime::egui::memory::Service;
use crate::runtime::egui::pass;
use crate::runtime::pointer;
use egui::{Context, Event, Id, PointerButton, Pos2, Rect, ViewportId};

/// What a modal leaves of the canvas below it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Reach {
    /// View navigation: the wheel and the middle press.
    View,
    /// View navigation and the primary press, for the sampler an armed control runs on the canvas.
    Sample,
}

/// The canvas a dialog keeps open, and how far.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Armed {
    /// The canvas component's id.
    pub canvas: Id,
    /// How far the modal lets input through.
    pub reach: Reach,
}

/// The screen rect of the armed canvas in one viewport, and its reach.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Hole {
    pub rect: Rect,
    pub reach: Reach,
}

impl Hole {
    /// Whether the modal lets `event` through: a primary press only under `Reach::Sample`, a middle press and the
    /// wheel under either, each only inside the canvas. `pointer` is where the pointer stood before the event.
    pub(crate) fn admits(&self, event: &Event, pointer: Option<Pos2>) -> bool {
        match event {
            Event::PointerButton { pos, button: PointerButton::Primary, pressed: true, .. } => {
                self.reach == Reach::Sample && self.rect.contains(*pos)
            }
            Event::PointerButton { pos, button: PointerButton::Middle, pressed: true, .. } => self.rect.contains(*pos),
            Event::MouseWheel { .. } => pointer.is_some_and(|at| self.rect.contains(at)),
            _ => false,
        }
    }
}

/// The registration and the root frame it was made in. `Cx::arm_sampler` and `Cx::keep_view_open` (contract layer) make it.
#[derive(Default)]
struct Registry {
    made: Option<(u64, Armed)>,
}

pub(crate) fn register(ctx: &Context, armed: Armed) {
    let now = pass::root_frame_nr(ctx);
    Service::<Registry>::run(ctx, |registry| {
        let reach = match registry.made {
            Some((made, held)) if made == now && held.canvas == armed.canvas => held.reach.max(armed.reach),
            _ => armed.reach,
        };
        registry.made = Some((now, Armed { reach, ..armed }));
    });
}

/// The registration made in this root frame or the last: one per app, the latest wins.
pub(crate) fn armed(ctx: &Context) -> Option<Armed> {
    let now = pass::root_frame_nr(ctx);
    Service::<Registry>::run(ctx, |registry| registry.made.filter(|(made, _)| made + 1 >= now).map(|(_, armed)| armed))
}

/// The armed canvas's screen rect in `viewport`, when the canvas is shown there.
pub(crate) fn hole(ctx: &Context, viewport: ViewportId) -> Option<Hole> {
    let armed = armed(ctx)?;
    let (shown_in, _, rect) = pointer::root_bounds(ctx, armed.canvas)?;
    (shown_in == viewport).then_some(Hole { rect, reach: armed.reach })
}

/// Whether the armed canvas in `viewport` covers the screen position `at`.
pub(crate) fn passes(ctx: &Context, viewport: ViewportId, at: Pos2) -> bool {
    hole(ctx, viewport).is_some_and(|hole| hole.rect.contains(at))
}

/// The input hook for a modal in the same viewport: removes the button presses on the canvas that its reach does not
/// admit. egui's modal gate leaves a canvas with no press target, which lets any press through that the scrim's hole
/// uncovers. Returns whether it removed one.
pub(crate) fn bar_presses(ctx: &Context, input: &mut RawInput) -> bool {
    let Some(hole) = hole(ctx, input.viewport_id) else { return false };
    let before = input.events.len();
    input
        .events
        .retain(|event| !matches!(event, Event::PointerButton { pos, pressed: true, .. } if hole.rect.contains(*pos) && !hole.admits(event, None)));
    input.events.len() != before
}
