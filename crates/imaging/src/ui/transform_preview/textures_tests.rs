//! Tests of the preview textures (task TR-T4).

use std::sync::Arc;

use egui::{Color32, ColorImage, Context};

use super::{PreviewDisplay, PreviewMask, PreviewPixels, PreviewTextures, TextureChoice};

fn choice(level: u32, nearest: bool) -> TextureChoice {
    TextureChoice { level, nearest }
}

fn solid(size: [usize; 2], color: Color32) -> ColorImage {
    ColorImage::new(size, vec![color; size[0] * size[1]])
}

// Below ½ screen pixel per source pixel a mip level (zoom 0.25 → level 2, at most 12); around 1:1
// level 0 linear; at zoom ≥ 2 with scale ≤ 1.05 level 0 nearest; a scale over 1.05 stays linear.
#[test]
fn the_texture_level_follows_the_on_screen_size() {
    assert_eq!(PreviewTextures::choose(1.0, 1.0, false), choice(0, false));
    assert_eq!(PreviewTextures::choose(3.0, 1.0, false), choice(0, true));
    assert_eq!(PreviewTextures::choose(3.0, 2.0, false), choice(0, false));
    assert_eq!(PreviewTextures::choose(0.25, 1.0, false), choice(2, false));
    assert_eq!(PreviewTextures::choose(0.3, 1.0, false), choice(1, false));
    assert_eq!(PreviewTextures::choose(1e-9, 1.0, false).level, 12);
    assert_eq!(PreviewTextures::choose(f32::NAN, 1.0, false), choice(0, false));
    assert_eq!(PreviewTextures::choose(1.0, 0.0, false), choice(0, false));
}

// With nearest interpolation every zoom and scale gives level 0 with nearest filtering.
#[test]
fn nearest_interpolation_keeps_level_zero_and_nearest_filtering() {
    for zoom in [1.0 / 4096.0, 0.25, 1.0, 8.0] {
        for scale in [0.1, 1.0, 4.0] {
            assert_eq!(PreviewTextures::choose(zoom, scale, true), choice(0, true));
        }
    }
}

// A display conversion changes only the displayed upload: `level_image` stays raw, a new key
// replaces the uploads of the old one.
#[test]
fn the_display_conversion_changes_only_the_displayed_upload() {
    let ctx = Context::default();
    let raw = solid([2, 2], Color32::from_rgb(100, 50, 20));
    let textures = PreviewTextures::new(&ctx, "display", PreviewPixels::new(raw.clone(), [1.0, 1.0]));
    let display = |key: u64| PreviewDisplay { key, convert: Arc::new(|image: &ColorImage| solid(image.size, Color32::from_rgb(1, 2, 3))) };
    let level0 = choice(0, false);
    let plain = textures.pick(&ctx, level0, None).0;
    let first = textures.pick(&ctx, level0, Some(&display(1))).0;
    assert_ne!(plain, first);
    assert_eq!(textures.pick(&ctx, level0, Some(&display(1))).0, first);
    let second = textures.pick(&ctx, level0, Some(&display(2))).0;
    assert_ne!(first, second);
    assert_eq!(textures.uploaded.lock().unwrap().len(), 1, "the old key's uploads are dropped");
    assert_eq!(textures.level_image(0).unwrap().0.pixels, raw.pixels);
}

// A mask value of 0 at density 1 clears a texel; 128 at density 0.5 keeps three quarters of its
// alpha and colour.
#[test]
fn the_mask_scales_the_texel_alpha() {
    let texel = Color32::from_rgba_premultiplied(200, 100, 40, 200);
    let pixels = |density: f32, value: u8| {
        PreviewPixels::new(solid([1, 1], texel), [1.0, 1.0]).masked(&PreviewMask { values: vec![value], density }).image.pixels[0]
    };
    assert_eq!(pixels(1.0, 0), Color32::TRANSPARENT);
    assert_eq!(pixels(1.0, 255), texel);
    let [r, g, b, a] = pixels(0.5, 128).to_array();
    assert_eq!([r, g, b, a], [150, 75, 30, 150]);
}

// Mip levels are built only when asked for, halve down to one texel, and keep the uv extent of the
// source.
#[test]
fn mip_levels_are_built_on_demand_and_keep_the_source_extent() {
    let ctx = Context::default();
    let textures = PreviewTextures::new(&ctx, "mips", PreviewPixels::new(solid([5, 3], Color32::WHITE), [0.8, 0.6]));
    assert_eq!(textures.size(), [5, 3]);
    assert_eq!(textures.images.lock().unwrap().len(), 1);
    let (level1, uv1) = textures.level_image(1).unwrap();
    assert_eq!(level1.size, [3, 2]);
    assert_eq!(textures.images.lock().unwrap().len(), 2);
    assert!((uv1[0] - 0.8 * 5.0 / 6.0).abs() < 1e-6 && (uv1[1] - 0.6 * 3.0 / 4.0).abs() < 1e-6);
    let (last, _) = textures.level_image(12).unwrap();
    assert_eq!(last.size, [1, 1]);
    assert_eq!(textures.images.lock().unwrap().len(), 4);
}
