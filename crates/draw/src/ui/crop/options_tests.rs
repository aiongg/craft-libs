//! Tests of the crop settings model (task TO-C1).

use kurbo::Size;

use super::bar::CropBarLabels;
use super::crop_guides::{CropGuide, GuideShow};
use super::options::*;
use super::session::{CropDocKey, CropDocument};

fn document() -> CropDocument {
    CropDocument::new(CropDocKey(1), Size::new(2400.0, 1500.0)).resolution(72.0)
}

fn size_mode(width: &str, height: &str, resolution: &str) -> CropSettings {
    CropSettings {
        ratio: CropRatio::Size,
        size: CropSizeFields { width: width.into(), height: height.into(), resolution: resolution.into(), ..CropSizeFields::default() },
        ..CropSettings::default()
    }
}

// Choosing each of `CROP_RATIOS` sets the ratio and its fields; each of the eight `CROP_SIZES` fills W, H and the resolution in px/in and turns W x H x Resolution on; `choice` shows them back.
#[test]
fn ratio_presets_and_size_presets_fill_the_fields() {
    let doc = document();
    let mut settings = CropSettings::default();
    for (i, ratio) in CROP_RATIOS.iter().enumerate() {
        settings.choose(CropPresetChoice::Free, &doc, &[]);
        settings.choose(CropPresetChoice::Ratio(i), &doc, &[]);
        assert_eq!(settings.ratio, *ratio);
        if let CropRatio::Sides { width, height } = ratio {
            assert_eq!(settings.ratio_fields, [*width, *height]);
        }
        assert_eq!(settings.choice(&[]), CropPresetChoice::Ratio(i));
    }
    for (i, preset) in CROP_SIZES.iter().enumerate() {
        settings.size.resolution_unit = ResolutionUnit::PixelsPerCentimetre;
        assert!(settings.choose(CropPresetChoice::SizePreset(i), &doc, &[]));
        assert_eq!(settings.ratio, CropRatio::Size);
        assert_eq!(
            (settings.size.width.as_str(), settings.size.height.as_str(), settings.size.resolution.as_str(), settings.size.resolution_unit),
            (preset.width, preset.height, preset.resolution, ResolutionUnit::PixelsPerInch)
        );
        assert_eq!(settings.choice(&[]), CropPresetChoice::SizePreset(i));
    }
    assert!(!settings.choose(CropPresetChoice::NewPreset, &doc, &[]));
    assert!(settings.choose(CropPresetChoice::Free, &doc, &[]));
    assert_eq!(settings.choice(&[]), CropPresetChoice::Free);

    let user = [CropUserPreset { name: "Cinema".into(), ratio: CropRatio::Sides { width: 21.0, height: 9.0 }, size: CropSizeFields::default() }];
    assert!(settings.choose(CropPresetChoice::User(0), &doc, &user));
    assert_eq!(settings.ratio_fields, [21.0, 9.0]);
    assert_eq!(settings.choice(&user), CropPresetChoice::User(0));
}

// "4 in", "4in", "10,5 cm" and "1024" parse; a bare number keeps the field's unit (pixels when empty); zero, negative, percent and unknown units do not; normalising keeps the previous text for junk.
#[test]
fn lengths_parse_with_units_and_a_bare_number_keeps_the_field_unit() {
    let norm = CropSizeFields::normalize_length;
    assert_eq!(norm("4 in", ""), "4 in");
    assert_eq!(norm("4in", ""), "4 in");
    assert_eq!(norm("10,5 cm", ""), "10.5 cm");
    assert_eq!(norm("1024", ""), "1024 px");
    assert_eq!(norm("6", "4 in"), "6 in");
    assert_eq!(norm("  ", "4 in"), "");
    for junk in ["0", "-3", "50%", "4 furlongs", "abc"] {
        assert_eq!(norm(junk, "4 in"), "4 in", "{junk}");
    }
    assert_eq!(CropSizeFields::normalize_resolution("300,5", "72"), "300.5");
    assert_eq!(CropSizeFields::normalize_resolution("0", "72"), "72");
    assert_eq!(CropSizeFields::normalize_resolution("", "72"), "");
}

// `target` converts physical lengths at the typed resolution (px/cm x 2.54), else at the document's; points at the document's points per inch.
#[test]
fn target_uses_the_typed_resolution_else_the_documents() {
    let doc = document().resolution(100.0);
    let at_typed = size_mode("4 in", "6 in", "300").target(&doc).unwrap();
    assert_eq!((at_typed.width, at_typed.height, at_typed.resolution), (1200.0, 1800.0, Some(300.0)));
    let at_document = size_mode("4 in", "1024", "").target(&doc).unwrap();
    assert_eq!((at_document.width, at_document.height, at_document.resolution), (400.0, 1024.0, None));
    let mut per_cm = size_mode("2.54 cm", "1 in", "100");
    per_cm.size.resolution_unit = ResolutionUnit::PixelsPerCentimetre;
    let t = per_cm.target(&doc).unwrap();
    assert!((t.width - 254.0).abs() < 1e-9 && (t.height - 254.0).abs() < 1e-9 && t.resolution == Some(254.0));
    let points = size_mode("72 pt", "1", "").target(&doc.points_per_inch(72.27)).unwrap();
    assert!((points.width - 100.0 * 72.0 / 72.27).abs() < 1e-9);
    assert!((size_mode("4 in", "6 in", "").ratio_value(&doc).unwrap() - 4.0 / 6.0).abs() < 1e-12);
}

// W or H empty: no target and no ratio; one ratio side typed: a free ratio that keeps the side.
#[test]
fn empty_w_or_h_is_a_free_crop() {
    let doc = document();
    let half = size_mode("4 in", "", "300");
    assert_eq!((half.target(&doc), half.ratio_value(&doc)), (None, None));
    let mut settings = CropSettings::default();
    settings.set_ratio_field(0, 3.0);
    assert_eq!((settings.ratio, settings.ratio_fields, settings.ratio_value(&doc)), (CropRatio::Free, [3.0, 0.0], None));
    assert_eq!(settings.shown_ratio_fields(&doc), [3.0, 0.0]);
    settings.set_ratio_field(1, 1.41437);
    assert_eq!(settings.ratio, CropRatio::Sides { width: 3.0, height: 1.414 });
    settings.set_ratio_field(0, 0.0);
    assert_eq!((settings.ratio, settings.ratio_fields), (CropRatio::Free, [0.0, 1.414]));
}

// X swaps W and H in W x H x Resolution mode and the ratio's sides otherwise (Original turns the document's size); Front Image fills the document's size in px and its resolution in the field's unit.
#[test]
fn swap_and_front_image() {
    let doc = document();
    let mut settings = size_mode("4 in", "6 px", "300");
    assert!(settings.swap(&doc));
    assert_eq!((settings.size.width.as_str(), settings.size.height.as_str()), ("6 px", "4 in"));

    let mut ratio = CropSettings { ratio: CropRatio::Sides { width: 16.0, height: 9.0 }, ..CropSettings::default() };
    assert!(ratio.swap(&doc));
    assert_eq!(ratio.ratio, CropRatio::Sides { width: 9.0, height: 16.0 });
    let mut original = CropSettings { ratio: CropRatio::Original, ..CropSettings::default() };
    assert!(original.swap(&doc));
    assert_eq!(original.ratio, CropRatio::Sides { width: 1500.0, height: 2400.0 });
    assert!(!CropSettings::default().swap(&doc));

    let mut front = CropSettings::default();
    front.size.resolution_unit = ResolutionUnit::PixelsPerCentimetre;
    front.front_image(&document().resolution(254.0));
    assert_eq!(front.ratio, CropRatio::Size);
    assert_eq!((front.size.width.as_str(), front.size.height.as_str(), front.size.resolution.as_str()), ("2400 px", "1500 px", "100"));
}

// "4 x 5 in 300 ppi", "2 : 3", mixed units, a px/cm resolution; `None` for Original and a free ratio.
#[test]
fn preset_name_is_made_from_the_values() {
    let labels = CropBarLabels::default();
    assert_eq!(size_mode("4 in", "5 in", "300").preset_name(&labels).as_deref(), Some("4 x 5 in 300 ppi"));
    assert_eq!(size_mode("8.5 in", "11 in", "").preset_name(&labels).as_deref(), Some("8.5 x 11 in"));
    assert_eq!(size_mode("4 in", "300", "").preset_name(&labels).as_deref(), Some("4 in x 300 px"));
    let mut per_cm = size_mode("1024", "768", "35.5");
    per_cm.size.resolution_unit = ResolutionUnit::PixelsPerCentimetre;
    assert_eq!(per_cm.preset_name(&labels).as_deref(), Some("1024 x 768 px 35.5 px/cm"));
    assert_eq!(size_mode("4 in", "", "300").preset_name(&labels), None);
    let ratio = CropSettings { ratio: CropRatio::Sides { width: 2.0, height: 3.0 }, ..CropSettings::default() };
    assert_eq!(ratio.preset_name(&labels).as_deref(), Some("2 : 3"));
    let original = CropSettings { ratio: CropRatio::Original, ..CropSettings::default() };
    assert_eq!((original.preset_name(&labels), CropSettings::default().preset_name(&labels)), (None, None));
}

// An empty or partial serialised `CropSettings` loads with every other field at its default (Delete Cropped Pixels on, Rule of Thirds, Auto).
#[test]
fn older_settings_load_with_defaults() {
    let empty: CropSettings = serde_json::from_str("{}").unwrap();
    assert_eq!(empty, CropSettings::default());
    assert!(empty.delete_pixels);
    assert_eq!((empty.guide, empty.guide_show), (CropGuide::Thirds, GuideShow::Auto));
    let partial: CropSettings = serde_json::from_str(r#"{"fill":"contentAware","ratio":{"sides":{"width":4.0,"height":5.0}}}"#).unwrap();
    assert_eq!((partial.fill, partial.ratio), (CropFill::ContentAware, CropRatio::Sides { width: 4.0, height: 5.0 }));
    assert!(partial.delete_pixels);
    let round_trip: CropSettings = serde_json::from_str(&serde_json::to_string(&partial).unwrap()).unwrap();
    assert_eq!(round_trip, partial);
}
