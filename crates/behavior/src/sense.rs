//! The library's sense set, for a root or a part.

use bitflags::bitflags;

bitflags! {
    /// What a root or a part senses. The egui adapter maps the root's sense to egui's.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct Sense: u16 {
        /// Hover only.
        const HOVER = 1;
        /// A press released within slop activates.
        const CLICK = 2;
        /// A press that moves past slop drags.
        const DRAG = 4;
        /// A Tab stop.
        const FOCUS = 8;
        /// The wheel goes here while the component is focused or armed.
        const WHEEL = 16;
        /// A press on this shape leaves key focus and any draft where they are.
        const KEEP_FOCUS = 32;
        /// A press on this shape moves key focus to its component, and to the part when the part is focusable.
        const TAKE_FOCUS = 64;
        /// The horizontal wheel share, taken without focus or arming; the split rule passes the vertical share on.
        const WHEEL_X = 128;
        /// A middle press that ends within slop is a click the component reads from its press; it never activates.
        /// Without this bit a middle press only captures for a drag.
        const MIDDLE_CLICK = 256;
    }
}

impl Sense {
    /// The bits that make a shape a press candidate.
    pub const PRESS: Sense = Sense::CLICK.union(Sense::DRAG);

    /// Whether a press can start on a shape with this sense.
    pub const fn takes_press(self) -> bool {
        self.intersects(Sense::PRESS)
    }
}
