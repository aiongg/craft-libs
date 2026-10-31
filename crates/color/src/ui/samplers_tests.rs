//! Behaviour tests of `samplers`.

#![expect(clippy::todo, reason = "S4C-F13: the tool tests below are stubs until the canvas tools land")]

use craft_cms::convert::{CmykConverter, srgb_to_cmyk_naive};
use craft_theme::Srgba;

use super::{ColorSamplers, MAX_SAMPLERS, SamplerError, SamplerReadout};

/// A click places points up to ten and the eleventh reports Full.
#[test]
fn points_stop_at_ten() {
    let mut samplers = ColorSamplers::new();
    for i in 0..MAX_SAMPLERS {
        assert_eq!(samplers.add([i as f64 + 0.7, 3.2], [100, 100]), Ok(i));
    }
    assert_eq!(samplers.add([50.0, 50.0], [100, 100]), Err(SamplerError::Full));
    assert_eq!(samplers.points().len(), 10);
    // A position floors to its pixel; outside the canvas is refused before the limit applies.
    assert_eq!(samplers.points()[0], [0.0, 3.0]);
    assert_eq!(samplers.add([100.0, 5.0], [100, 100]), Err(SamplerError::Outside));
    assert_eq!(samplers.add([-0.5, 5.0], [100, 100]), Err(SamplerError::Outside));
}

/// Moving onto the canvas floors; dropping off it removes; removing renumbers; a hit picks the
/// nearest point within the tolerance.
#[test]
fn points_move_remove_and_hit() {
    let mut samplers = ColorSamplers::new();
    for x in [10.0, 20.0, 30.0] {
        samplers.add([x, 10.0], [100, 100]).unwrap();
    }
    samplers.move_point(1, [22.9, 11.1], [100, 100]).unwrap();
    assert_eq!(samplers.points()[1], [22.0, 11.0]);
    assert_eq!(samplers.hit([21.0, 11.0], 7.0), Some(1));
    assert_eq!(samplers.hit([60.0, 60.0], 7.0), None);
    samplers.move_point(1, [-3.0, 11.0], [100, 100]).unwrap();
    assert_eq!(samplers.points(), [[10.0, 10.0], [30.0, 10.0]]);
    samplers.remove(0).unwrap();
    assert_eq!(samplers.points(), [[30.0, 10.0]]);
    assert_eq!(samplers.remove(3), Err(SamplerError::NoSuchPoint));
    assert_eq!(samplers.move_point(3, [1.0, 1.0], [100, 100]), Err(SamplerError::NoSuchPoint));
}

/// The readout's CMYK comes from the given converter, as the picker's.
#[test]
fn readouts_use_the_documents_converter() {
    let converter = CmykConverter::coated();
    let blue = Srgba { r: 0.0, g: 0.0, b: 1.0, a: 1.0 };
    let reading = SamplerReadout::new(2, [4.0, 9.0], blue, converter);
    assert_eq!((reading.number, reading.position, reading.rgb), (3, [4, 9], [0, 0, 255]));
    let managed = converter.from_srgb([0.0, 0.0, 1.0]).map(|v| (v * 100.0).round() as u8);
    assert_eq!(reading.cmyk, managed);
    let naive = srgb_to_cmyk_naive([0.0, 0.0, 1.0]).map(|v| (v * 100.0).round() as u8);
    assert_ne!(reading.cmyk, naive);
    // Channels that are not finite read as 0 and the rest is clamped.
    let odd = SamplerReadout::new(0, [0.0, 0.0], Srgba { r: f32::NAN, g: 2.0, b: -1.0, a: 1.0 }, converter);
    assert_eq!(odd.rgb, [0, 255, 0]);
}

/// A point dragged off the canvas is removed; a release within half a pixel is a click.
#[test]
#[ignore = "S4C-F13: needs ColorSamplerTool"]
fn a_point_dragged_off_the_canvas_is_removed() {
    todo!()
}

/// Alt-click on a point removes it; the later points move up one number.
#[test]
#[ignore = "S4C-F13: needs ColorSamplerTool"]
fn alt_click_removes_and_renumbers() {
    todo!()
}
