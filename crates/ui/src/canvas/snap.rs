//! `Snap`: the host's snapping of a coordinate the pointer sets on a canvas.

use craft_behavior::Axis;
use craft_keymap::Mods;

/// The host's snapping of a coordinate the pointer sets (View > Snap To): the axis a guide at the coordinate would
/// run along (an x coordinate is across `Axis::Y`), the document coordinate under the pointer and the modifiers held,
/// to the coordinate to use. The rulers snap the zero point with it and the guides snap a guide.
pub struct Snap<'a>(Box<dyn Fn(Axis, f64, Mods) -> f64 + 'a>);

impl<'a> Snap<'a> {
    /// The snapping that runs `snap`.
    pub fn new(snap: impl Fn(Axis, f64, Mods) -> f64 + 'a) -> Self {
        Snap(Box::new(snap))
    }

    /// `position` across a guide running `along`, snapped.
    pub(crate) fn apply(&self, along: Axis, position: f64, mods: Mods) -> f64 {
        (self.0)(along, position, mods)
    }
}
