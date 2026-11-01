//! [`BlurPinsLabels`]: the pins' words.

craft_ui::words! {
    /// The Blur Gallery pins' words, English by default.
    #[non_exhaustive]
    pub struct BlurPinsLabels {
        /// The pins' group on the canvas.
        pins: "blurPins.pins" = "Blur pins";
        /// A pin as a keyboard part: `{n}` its number, `{amount}` its amount with the unit.
        pin_part: "blurPins.pinPart" = "Pin {n}, {amount}";
        blur: "blurPins.blur" = "Blur";
        blur_angle: "blurPins.blurAngle" = "Blur Angle";
        no_pin: "blurPins.noPin" = "No pin selected";
        hint_pins: "blurPins.hintPins" = "Click the image to add a pin; drag a pin to move it, or its ring to change its blur. Delete removes the selected pin.";
        hint_tilt: "blurPins.hintTilt" = "Drag the pin to move the band, or its ring to change its blur. Drag the solid lines to widen the sharp band, the dashed lines to set the transition, and the dots to turn it.";
        hint_iris: "blurPins.hintIris" = "Drag the handles on the outline to resize and turn it, the square to round its corners, and the dots inside to set where the blur starts.";
        hint_spin: "blurPins.hintSpin" = "Drag the handles on the outline to resize and turn it; the ring sets the blur angle.";
    }
}
