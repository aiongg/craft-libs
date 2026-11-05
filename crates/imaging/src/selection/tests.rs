//! Tests of the preview and Quick Mask colour rules.

use super::*;

// Every `PreviewMode` at coverage 0, 0.5 and 1 gives PhotoCraft's `overlay_pixel` values
// (grayscale 0/128/255 opaque, black matte alpha 255/128/0, white matte likewise, quick mask red at
// 128/64/0), and coverage outside 0..=1 is clamped.
#[test]
fn preview_pixel_rules() {
    let look = QuickMaskLook::default();
    let at = |mode: PreviewMode, c: f32| mode.pixel(c, &look).0;
    for c in [0.0, 0.5, 1.0] {
        assert_eq!(at(PreviewMode::None, c), [0, 0, 0, 0]);
    }
    assert_eq!([0.0, 0.5, 1.0].map(|c| at(PreviewMode::Grayscale, c)), [[0, 0, 0, 255], [128, 128, 128, 255], [255, 255, 255, 255]]);
    assert_eq!([0.0, 0.5, 1.0].map(|c| at(PreviewMode::BlackMatte, c)[3]), [255, 128, 0]);
    assert_eq!(at(PreviewMode::WhiteMatte, 0.5), [255, 255, 255, 128]);
    assert_eq!([0.0, 0.5, 1.0].map(|c| at(PreviewMode::QuickMask, c)), [[255, 0, 0, 128], [255, 0, 0, 64], [255, 0, 0, 0]]);
    assert_eq!(at(PreviewMode::Grayscale, 2.0), at(PreviewMode::Grayscale, 1.0));
    assert_eq!(at(PreviewMode::BlackMatte, -1.0), at(PreviewMode::BlackMatte, 0.0));
}

// `QuickMaskLook::pixel`: masked areas tint the unselected pixels, selected areas tint the
// selected ones; the colour and the opacity carry through.
#[test]
fn quick_mask_color_indicates_flips_coverage() {
    let masked = QuickMaskLook { color: Srgba::rgb(0.0, 0.0, 1.0), opacity: 0.8, indicates: ColorIndicates::MaskedAreas };
    let selected = QuickMaskLook { indicates: ColorIndicates::SelectedAreas, ..masked };
    assert_eq!(masked.pixel(0.0).0, [0, 0, 255, 204]);
    assert_eq!(masked.pixel(1.0).0[3], 0);
    assert_eq!(selected.pixel(0.0).0[3], 0);
    assert_eq!(selected.pixel(1.0).0, [0, 0, 255, 204]);
}
