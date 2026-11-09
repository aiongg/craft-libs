//! Tests of the preview and Quick Mask colour rules.

use super::*;

// Coverage drives each `PreviewMode` monotonically (grayscale brightens with it, mattes and the
// quick mask clear as it rises), `None` is transparent, and coverage outside 0..=1 is clamped.
#[test]
fn preview_pixel_rules() {
    let look = QuickMaskLook::default();
    let at = |mode: PreviewMode, c: f32| mode.pixel(c, &look).0;
    for c in [0.0, 0.5, 1.0] {
        assert_eq!(at(PreviewMode::None, c)[3], 0);
        assert_eq!(at(PreviewMode::Grayscale, c)[3], 255);
    }
    assert!(at(PreviewMode::Grayscale, 0.0)[0] < at(PreviewMode::Grayscale, 0.5)[0]);
    assert!(at(PreviewMode::Grayscale, 0.5)[0] < at(PreviewMode::Grayscale, 1.0)[0]);
    for mode in [PreviewMode::BlackMatte, PreviewMode::WhiteMatte, PreviewMode::QuickMask] {
        assert!(at(mode, 0.0)[3] > at(mode, 0.5)[3], "{mode:?}");
        assert!(at(mode, 0.5)[3] > at(mode, 1.0)[3], "{mode:?}");
    }
    assert_eq!(at(PreviewMode::BlackMatte, 0.5)[3], at(PreviewMode::WhiteMatte, 0.5)[3]);
    assert_eq!(at(PreviewMode::Grayscale, 2.0), at(PreviewMode::Grayscale, 1.0));
    assert_eq!(at(PreviewMode::BlackMatte, -1.0), at(PreviewMode::BlackMatte, 0.0));
}

// `QuickMaskLook::pixel`: masked areas tint the unselected pixels, selected areas tint the
// selected ones; the colour and the opacity carry through.
#[test]
fn quick_mask_color_indicates_flips_coverage() {
    let masked = QuickMaskLook { color: Srgba::rgb(0.0, 0.0, 1.0), opacity: 0.8, indicates: ColorIndicates::MaskedAreas };
    let selected = QuickMaskLook { indicates: ColorIndicates::SelectedAreas, ..masked };
    assert!(masked.pixel(0.0).0[3] > 0);
    assert_eq!(masked.pixel(1.0).0[3], 0);
    assert_eq!(selected.pixel(0.0).0[3], 0);
    assert_eq!(masked.pixel(0.0), selected.pixel(1.0));
}
