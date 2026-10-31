//! The export dialogs' pure rules.

use std::path::{Path, PathBuf};

use super::*;

// Reserved characters become `_`, the name is cut to 80 characters and trimmed, and an empty name
// becomes the fallback.
#[test]
fn file_name_replaces_reserved_characters_and_trims() {
    assert_eq!(export_file_name("a<b>:c\"d/e\\f|g?h*i\u{7}j", "layer"), "a_b__c_d_e_f_g_h_i_j");
    assert_eq!(export_file_name("  ..name.. ", "layer"), "name");
    assert_eq!(export_file_name("...", "layer"), "layer");
    assert_eq!(export_file_name("", "Untitled"), "Untitled");
    assert_eq!(export_file_name(&"x".repeat(100), "layer"), "x".repeat(80));
}

// Later files of a multi-file export are numbered with their layer names and never collide.
#[test]
fn numbered_paths_add_layer_names_and_avoid_collisions() {
    let first = Path::new("out/sheet.png");
    let taken = [PathBuf::from("out/sheet-2-sky.png"), PathBuf::from("out/sheet-2-sky-1.png")];
    let paths = numbered_paths(first, &["base", "sky", "a/b"], |path| taken.iter().any(|t| t == path));
    assert_eq!(paths, [PathBuf::from("out/sheet.png"), PathBuf::from("out/sheet-2-sky-2.png"), PathBuf::from("out/sheet-3-a_b.png")]);

    let bare = numbered_paths(Path::new(""), &["one", "two"], |_| false);
    assert_eq!(bare, [PathBuf::from(""), PathBuf::from("layers-2-two.png")]);
}

// Scale rounds each side to at least 1 px and refuses 0 % or 1001 %; a locked typed side keeps the
// source's aspect and 300,001 px is refused.
#[test]
fn resize_rules_round_keep_aspect_and_refuse_out_of_range() {
    assert_eq!(resize_from_scale((101, 3), 50.0), Ok((51, 2)));
    assert_eq!(resize_from_scale((10, 1), 1.0), Ok((1, 1)));
    assert_eq!(resize_from_scale((0, 5), 100.0), Ok((1, 5)));
    assert_eq!(resize_from_scale((100, 100), 0.0), Err(SizeError::Scale));
    assert_eq!(resize_from_scale((100, 100), 1001.0), Err(SizeError::Scale));
    assert_eq!(resize_from_scale((100, 100), f64::NAN), Err(SizeError::Scale));
    assert_eq!(resize_from_scale((100_000, 10), 400.0), Err(SizeError::Pixels));

    assert_eq!(resize_from_side((1200, 800), 600.0, 0.0, true, true), Ok(((600, 400), 50.0)));
    assert_eq!(resize_from_side((1200, 800), 0.0, 400.0, true, false), Ok(((600, 400), 50.0)));
    assert_eq!(resize_from_side((1200, 800), 600.0, 123.0, false, true), Ok(((600, 123), 50.0)));
    let ((width, height), scale) = resize_from_side((1200, 1), 1.0, 0.0, true, true).unwrap();
    assert_eq!((width, height), (1, 1));
    assert!((scale - 100.0 / 1200.0).abs() < 1e-12);
    assert_eq!(resize_from_side((1200, 800), 300_001.0, 10.0, false, true), Err(SizeError::Pixels));
    assert_eq!(resize_from_side((1200, 800), 0.4, 10.0, false, true), Err(SizeError::Pixels));
    assert_eq!(resize_from_side((800, 1200), 300_000.0, 0.0, true, true), Err(SizeError::Pixels));
}

// 4-Up halves and quarters the quality of a JPEG and the colours of a GIF.
#[test]
fn four_up_variants_step_quality_or_colours() {
    let jpeg = WebSettings { format: WebFormat::Jpeg, quality: 60, ..WebSettings::default() };
    let [half, quarter] = four_up_variants(&jpeg);
    assert_eq!((half.quality, quarter.quality), (30, 15));
    assert_eq!((half.format, quarter.format), (WebFormat::Jpeg, WebFormat::Jpeg));

    let gif = WebSettings { format: WebFormat::Gif, colors: 128, ..WebSettings::default() };
    let [half, quarter] = four_up_variants(&gif);
    assert_eq!((half.colors, quarter.colors), (64, 32));

    let png = WebSettings { format: WebFormat::Png24, ..WebSettings::default() };
    let [half, quarter] = four_up_variants(&png);
    assert_eq!((half.format, half.colors, quarter.format, quarter.colors), (WebFormat::Png8, 128, WebFormat::Gif, 64));

    let lossless = WebSettings { format: WebFormat::WebP, lossless: true, ..WebSettings::default() };
    assert_eq!(four_up_variants(&lossless)[0].format, WebFormat::Png8);
    let lossy = WebSettings { format: WebFormat::WebP, quality: 1, ..WebSettings::default() };
    assert_eq!(four_up_variants(&lossy)[1].quality, 1, "quality never drops below 1");
}

// The download time is Photoshop's 56.6 Kbps estimate, rounded up to whole seconds.
#[test]
fn download_time_rounds_up_at_56k() {
    assert_eq!(download_seconds(0, 56.6), 0);
    assert_eq!(download_seconds(1, 56.6), 1);
    assert_eq!(download_seconds(7075, 56.6), 1);
    assert_eq!(download_seconds(7076, 56.6), 2);
    assert_eq!(download_seconds(50_000, 56.6), 8);
}

// JPEG quality 0–12 maps onto 1–100 and its label is Low, Medium, High or Maximum.
#[test]
fn jpeg_quality_maps_to_percent_and_label() {
    assert_eq!([0, 12, 13].map(jpeg_quality_percent), [1, 100, 100]);
    assert_eq!([3, 5, 8].map(jpeg_quality_percent), [26, 42, 67]);
    let labels = FormatOptionsLabels::default();
    let names: Vec<&str> = (0..=12).map(|q| jpeg_quality_label(q, &labels)).collect();
    assert_eq!(names, ["Low", "Low", "Low", "Low", "Low", "Medium", "Medium", "Medium", "High", "High", "Maximum", "Maximum", "Maximum"]);
}
