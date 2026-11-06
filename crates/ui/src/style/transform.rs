//! Style of the `transform` family.

const HIT: f32 = 12.0;
const ROTATE_REACH: f32 = 36.0;
const BAND_INNER: f32 = 5.0;
const BAND_OUTER: f32 = 18.0;
const CLICK_SLOP: f32 = 2.0;
const PIVOT_ARM: f32 = 8.0;
const TURN_RADIUS: f32 = 12.0;
const WARP_MINOR: f32 = 0.75;
const WARP_REVEAL: f32 = 56.0;
const PIN: f32 = 11.0;
const PIN_CORE: f32 = 4.0;
const PIN_TURN: f32 = 18.0;
const PLANE_CORNER: f32 = 9.0;

/// Transform values that are not semantic tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TransformStyle {
    /// Distance from a handle or the reference point within which the pointer takes it.
    pub hit: f32,
    /// Distance from a corner, outside the box, within which the rotate cursor shows.
    pub rotate_reach: f32,
    /// Inner edge of the rotate band outside a corner, from the corner.
    pub band_inner: f32,
    /// Outer edge of the rotate band outside a corner, from the corner.
    pub band_outer: f32,
    /// Farthest a press outside the box may move and still be a click.
    pub click_slop: f32,
    /// Radius of the reference point's circle.
    pub pivot_radius: f32,
    /// Length of each arm of the reference point's cross, from its centre.
    pub pivot_arm: f32,
    /// Radius of the rotate cursor's arc.
    pub turn_radius: f32,
    /// Width of the rule-of-thirds lines inside a one-patch warp mesh.
    pub warp_minor: f32,
    /// Side of a warp mesh's anchor squares.
    pub warp_anchor: f32,
    /// Diameter of a warp mesh's handle ends.
    pub warp_control: f32,
    /// Distance from the pointer within which a warp handle and its arm show.
    pub warp_reveal: f32,
    /// Diameter of a Puppet Warp pin's disc.
    pub pin: f32,
    /// Diameter of the selected pin's core.
    pub pin_core: f32,
    /// Radius of the fixed-rotation ring, and the reach of the Option turn.
    pub pin_turn: f32,
    /// Diameter of a Perspective Warp plane's corner disc.
    pub plane_corner: f32,
}

impl TransformStyle {
    /// The transform style for `tokens`.
    pub fn new(tokens: &craft_theme::Tokens) -> Self {
        let half_handle = tokens.size.handle.sm / 2.0;
        Self {
            hit: HIT,
            rotate_reach: ROTATE_REACH,
            band_inner: BAND_INNER,
            band_outer: BAND_OUTER,
            click_slop: CLICK_SLOP,
            pivot_radius: half_handle,
            pivot_arm: PIVOT_ARM,
            turn_radius: TURN_RADIUS,
            warp_minor: WARP_MINOR,
            warp_anchor: half_handle + tokens.stroke.thin,
            warp_control: half_handle + tokens.stroke.thin,
            warp_reveal: WARP_REVEAL,
            pin: PIN,
            pin_core: PIN_CORE,
            pin_turn: PIN_TURN,
            plane_corner: PLANE_CORNER,
        }
    }
}
