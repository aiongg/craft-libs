//! Web-safe colours: each channel on 0, 0.2, ... 1 (`00`, `33`, `66`, `99`, `CC`, `FF`).

/// `color` with each channel (clamped to `0..=1`) on the nearest web-safe level.
pub fn web_safe(color: [f32; 3]) -> [f32; 3] {
    color.map(|v| (v.clamp(0.0, 1.0) * 5.0).round() / 5.0)
}

/// The nearest web colour of `color`, when `color` is not one; `None` for a web colour. It is
/// what a "not a web colour" warning offers.
pub fn not_web_safe(color: [f32; 3]) -> Option<[f32; 3]> {
    let near = web_safe(color);
    (crate::format_hex(near) != crate::format_hex(color)).then_some(near)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_nearest_web_colour_is_offered_only_for_a_colour_that_is_not_one() {
        assert_eq!(not_web_safe([0.2, 0.4, 0.6]), None);
        assert_eq!(not_web_safe([0.18, 0.42, 1.2]), Some([0.2, 0.4, 1.0]));
        assert_eq!(web_safe([2.0, -1.0, 0.5]), [1.0, 0.0, 0.6]);
    }
}
