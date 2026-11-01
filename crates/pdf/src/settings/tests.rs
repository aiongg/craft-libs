use super::*;

struct Profiles;

impl ProfileCatalog for Profiles {
    fn kind(&self, name: &str) -> Option<ProfileKind> {
        match name {
            "sRGB" => Some(ProfileKind::Rgb),
            "Coated FOGRA39" => Some(ProfileKind::Cmyk),
            "Dot Gain 20%" => Some(ProfileKind::Gray),
            _ => None,
        }
    }
    fn rgb(&self) -> &str {
        "sRGB"
    }
    fn cmyk(&self) -> &str {
        "Coated FOGRA39"
    }
}

fn refused(s: &PdfSettings) -> SettingsError {
    s.check(&Profiles).unwrap_err()
}

#[test]
fn defaults_pass_every_check_and_warn_about_nothing() {
    let d = PdfSettings::default();
    assert_eq!(d.check(&Profiles), Ok(()));
    assert!(d.warnings(&Profiles).is_empty());
    assert_eq!((d.compatibility, d.compression.color.above_ppi, d.compression.mono.ppi), (Compatibility::Pdf17, 450.0, 1200.0));
}

#[test]
fn choice_ids_round_trip_and_match_the_lists() {
    fn round<C: Choice + PartialEq + std::fmt::Debug>(from: fn(&str) -> Option<C>, id: fn(C) -> &'static str, label: fn(C) -> &'static str) {
        assert_eq!(C::ALL.len(), C::IDS.len());
        for (i, c) in C::ALL.iter().enumerate() {
            assert_eq!((id(*c), label(*c)), (C::IDS[i], C::LABELS[i]));
            assert_eq!(from(id(*c)), Some(*c));
        }
        assert_eq!(from("nope"), None);
    }
    round(Standard::from_id, Standard::id, Standard::label);
    round(Compatibility::from_id, Compatibility::id, Compatibility::label);
    round(MonoCodec::from_id, MonoCodec::id, MonoCodec::label);
    round(Changes::from_id, Changes::id, Changes::label);
    assert_eq!(Compatibility::from_id("1.6"), Some(Compatibility::Pdf16));
    assert!(Compatibility::ALL.iter().all(|c| Standard::None.allows(*c)));
}

#[test]
fn a_standard_with_the_wrong_pdf_version_is_refused() {
    let x4 = PdfSettings { standard: Standard::PdfX4, ..Default::default() };
    let e = refused(&x4);
    assert_eq!(e.id, "standardVersion");
    assert!(e.message.contains("PDF/X-4:2010 files can't be PDF 1.7"), "{e}");
    assert_eq!((e.value("standard"), e.value("version")), (Some("PDF/X-4:2010"), Some("PDF 1.7")));
    assert_eq!(PdfSettings { compatibility: Compatibility::Pdf16, ..x4 }.check(&Profiles), Ok(()));
    let a = PdfSettings { standard: Standard::PdfA2b, compatibility: Compatibility::Pdf20, ..Default::default() };
    assert!(refused(&a).message.contains("PDF/A-2b"));
    assert_eq!(PdfSettings { compatibility: Compatibility::Pdf14, ..a }.check(&Profiles), Ok(()));
}

#[test]
fn a_standard_refuses_editing_data_and_pdf13_standards_refuse_layers() {
    let s = PdfSettings { standard: Standard::PdfA2b, preserve_editing: true, ..Default::default() };
    assert_eq!(refused(&s).id, "standardEditing");
    let x = PdfSettings { standard: Standard::PdfX3, compatibility: Compatibility::Pdf13, create_layers: true, ..Default::default() };
    let e = refused(&x);
    assert_eq!(e.id, "standardLayers");
    assert!(e.message.contains("turn off createLayers"));
}

#[test]
fn out_of_range_values_are_refused_by_name() {
    let mut s = PdfSettings::default();
    s.compression.color.ppi = 5.0;
    assert!(refused(&s).message.contains("compression.color.ppi must be 9–2400 ppi (got 5)"));
    let mut s = PdfSettings::default();
    s.compression.mono.above_ppi = 3000.0;
    assert!(refused(&s).message.contains("compression.mono.abovePpi"));
    let mut s = PdfSettings::default();
    s.bleed.left = 100.0;
    assert!(refused(&s).message.contains("bleed.left"));
    let mut s = PdfSettings::default();
    s.marks.weight = 0.0;
    assert!(refused(&s).message.contains("marks.weight must be 0.05–2 pt"));
    let mut s = PdfSettings::default();
    s.advanced.font_subset_percent = 120.0;
    assert!(refused(&s).message.contains("advanced.fontSubsetPercent"));
    s.advanced.font_subset_percent = f64::NAN;
    assert_eq!(refused(&s).id, "fontSubsetPercent");
}

#[test]
fn passwords_are_checked_against_the_standard_and_the_algorithm() {
    let pw = |open: &str, owner: &str| SecuritySettings { open_password: open.into(), permissions_password: owner.into(), ..Default::default() };
    let with = |security, standard, compatibility| PdfSettings { security, standard, compatibility, ..Default::default() };
    let ok = with(pw("x", ""), Standard::None, Compatibility::Pdf17);
    assert_eq!(ok.check(&Profiles), Ok(()));
    assert_eq!(ok.encryption(), Some(Encryption::Aes256));
    assert_eq!(PdfSettings::default().encryption(), None);
    let e = refused(&with(pw("x", ""), Standard::PdfA2b, Compatibility::Pdf17));
    assert_eq!(e.id, "standardPassword");
    assert!(e.message.contains("can't be password-protected"));
    assert!(refused(&with(pw("same", "same"), Standard::None, Compatibility::Pdf17)).message.contains("must differ"));
    let low = SecuritySettings { printing: Printing::Low, ..pw("a", "b") };
    assert_eq!(refused(&with(low, Standard::None, Compatibility::Pdf13)).id, "lowPrintingVersion");
    let long = "x".repeat(33);
    assert!(refused(&with(pw(&long, ""), Standard::None, Compatibility::Pdf16)).message.contains("up to 32 ASCII"));
    assert_eq!(with(pw(&long, ""), Standard::None, Compatibility::Pdf17).check(&Profiles), Ok(()));
    assert!(refused(&with(pw(&"x".repeat(128), ""), Standard::None, Compatibility::Pdf17)).message.contains("up to 127 bytes"));
}

#[test]
fn a_missing_or_wrong_destination_profile_is_refused() {
    let dest = |name: &str, standard| {
        let mut s = PdfSettings { standard, compatibility: standard.version(), ..Default::default() };
        s.output.conversion = ColorConversion::Destination;
        s.output.destination = name.into();
        s
    };
    assert_eq!(dest("Coated FOGRA39", Standard::None).check(&Profiles), Ok(()));
    assert_eq!(dest(DOCUMENT_CMYK, Standard::None).check(&Profiles), Ok(()));
    assert_eq!(dest(DOCUMENT_CMYK, Standard::None).check(&()).unwrap_err().id, "destinationUnknown");
    let e = refused(&dest("Nope", Standard::None));
    assert_eq!(e.message, "output.destination: no RGB or CMYK profile is called “Nope”");
    assert_eq!(e.value("name"), Some("Nope"));
    assert_eq!(refused(&dest("Dot Gain 20%", Standard::None)).id, "destinationUnknown");
    let e = refused(&dest("sRGB", Standard::PdfX1a));
    assert_eq!(e.id, "destinationRgb");
    assert!(e.message.contains("PDF/X-1a:2001 files are CMYK"));
    // Without conversion a named destination is not looked up.
    let mut s = dest("Nope", Standard::None);
    s.output.conversion = ColorConversion::None;
    assert_eq!(s.check(&Profiles), Ok(()));
}

#[test]
fn a_pdfx_output_intent_must_be_a_usable_profile_or_a_registered_condition() {
    let intent = |name: &str, registry: &str, standard| {
        let mut s = PdfSettings { standard, compatibility: standard.version(), ..Default::default() };
        s.output.output_intent = name.into();
        s.output.registry = registry.into();
        s
    };
    assert_eq!(intent("Coated FOGRA39", "", Standard::PdfX1a).check(&Profiles), Ok(()));
    assert_eq!(intent("sRGB", "", Standard::PdfX3).check(&Profiles), Ok(()));
    assert_eq!(refused(&intent("sRGB", "", Standard::PdfX1a)).id, "intentRgb");
    assert!(refused(&intent("Dot Gain 20%", "", Standard::PdfX4)).message.contains("is a grey profile"));
    assert_eq!(refused(&intent("Nope", "", Standard::PdfX1a)).id, "intentUnknown");
    assert_eq!(refused(&intent("Nope", "http://example.com", Standard::PdfX4)).id, "intentUnknownX4");
    assert_eq!(intent("Nope", "http://example.com", Standard::PdfX3).check(&Profiles), Ok(()));
    // Other standards don't look at the output intent.
    assert_eq!(intent("Nope", "", Standard::None).check(&Profiles), Ok(()));
}

#[test]
fn options_not_applied_come_back_as_warnings_with_ids() {
    let warns = |s: &PdfSettings| s.warnings(&Profiles);
    let one = |s: PdfSettings, id: &str, word: &str| {
        let w = warns(&s);
        assert!(w.len() == 1 && w[0].id == id && w[0].message.contains(word), "{w:?}");
    };
    one(PdfSettings { create_layers: true, compatibility: Compatibility::Pdf14, ..Default::default() }, "layersVersion", "PDF 1.5");
    let mut s = PdfSettings::default();
    s.advanced.font_subset_percent = 35.0;
    one(s, "fontSubsetThreshold", "subset");
    let mut s = PdfSettings::default();
    s.security.printing = Printing::Low;
    one(s, "permissionsNeedPassword", "permissions need a password");
    let mut s = PdfSettings::default();
    s.output.output_intent = "No Such Press".into();
    one(s, "intentNotEmbedded", "without embedding");
    let mut s = PdfSettings::default();
    s.output.registry = "http://example.com".into();
    one(s, "conditionNeedsIntent", "condition identifier");
    let mut s = PdfSettings { standard: Standard::PdfA2b, ..Default::default() };
    s.output.trapped = true;
    one(s, "pdfaOutputIntent", "PDF/A");
    let mut s = PdfSettings { compatibility: Compatibility::Pdf14, ..Default::default() };
    s.security.open_password = "x".into();
    one(s, "plaintextMetadataVersion", "needs PDF 1.5");
    let mut s = PdfSettings { compatibility: Compatibility::Pdf13, ..Default::default() };
    s.security.open_password = "x".into();
    s.security.plaintext_metadata = false;
    one(s, "weakKey", "40-bit");
    // Applied options warn about nothing.
    let mut s = PdfSettings { create_layers: true, ..Default::default() };
    s.output.output_intent = "Coated FOGRA39".into();
    s.output.output_condition = "Press".into();
    s.output.trapped = true;
    s.advanced.outline_text = true;
    s.advanced.font_subset_percent = 35.0;
    assert!(warns(&s).is_empty());
}

#[test]
fn permission_bits_allow_everything_without_a_permissions_password() {
    assert_eq!(SecuritySettings::default().permission_bits(), !3);
    let locked = SecuritySettings {
        permissions_password: "p".into(),
        printing: Printing::None,
        changes: Changes::None,
        copy: false,
        screen_reader: false,
        ..Default::default()
    };
    assert!(locked.restricts());
    assert_eq!(locked.permission_bits() as u32, 0xFFFF_F0C0);
}

#[test]
fn built_in_presets_are_valid_described_and_named_for_the_app() {
    let all = builtin_presets("VectorCraft");
    assert_eq!(all.len(), 7);
    assert_eq!(all[0].name, "VectorCraft Default");
    assert!(all[0].settings.preserve_editing, "the app default keeps the document editable");
    assert!(all[0].description.starts_with("Keeps the document editable: VectorCraft reopens"));
    let names: Vec<_> = all.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names[1..], ["High Quality Print", "Press Quality", "Smallest File Size", "PDF/X-1a:2001", "PDF/X-3:2002", "PDF/X-4:2010"]);
    for p in &all {
        assert_eq!(p.settings.check_values(), Ok(()), "{}", p.name);
        assert!(!p.description.is_empty());
    }
    let press = &all[2].settings;
    assert_eq!((press.output.conversion, press.output.destination.as_str()), (ColorConversion::PreserveNumbers, DOCUMENT_CMYK));
    assert_eq!((press.compatibility, press.compression.color.compression), (Compatibility::Pdf16, ImageCodec::Zip));
    assert_eq!(all[3].settings.compression.color.ppi, 100.0);
    assert_eq!(all[3].settings.compression.mono.above_ppi, 450.0);
    for name in ["pdf/x-1a:2001", "PDF/X-3:2002", "pdf/x-4:2010"] {
        let x = builtin_preset(name, "VectorCraft").unwrap();
        assert!(x.settings.standard.is_pdfx() && x.settings.check(&Profiles).is_ok(), "{name}");
    }
    assert_eq!(builtin_preset("default", "VectorCraft").unwrap().name, "VectorCraft Default");
    assert!(builtin_preset("Nope", "VectorCraft").is_none());
}

mod json {
    use serde_json::json;

    use super::*;

    fn settings(v: serde_json::Value) -> PdfSettings {
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn json_is_camel_case_and_partial_objects_keep_defaults() {
        let d = PdfSettings::default();
        let v = serde_json::to_value(&d).unwrap();
        assert_eq!(v["compatibility"], "1.7");
        assert_eq!(v["standard"], "none");
        assert_eq!(v["compression"]["compressText"], true);
        assert_eq!(v["compression"]["color"]["abovePpi"], 450.0);
        assert_eq!(v["advanced"]["overprint"], "preserve");
        assert_eq!(serde_json::from_value::<PdfSettings>(v).unwrap(), d, "round-trips");
        let s = settings(json!({"compatibility": "1.5", "marks": {"trim": true}, "compression": {"gray": {"downsample": "bicubic"}}}));
        assert_eq!(s.compatibility, Compatibility::Pdf15);
        assert!(s.marks.trim && s.marks.weight == 0.25);
        assert_eq!(s.compression.gray.downsample, Downsample::Bicubic);
        assert_eq!(s.compression.gray.ppi, 300.0);
        assert!(s.compression.compress_text);
        for bad in [json!({"compatibility": "1.2"}), json!({"standard": "pdfX9"}), json!({"marks": {"trim": "yes"}})] {
            assert!(serde_json::from_value::<PdfSettings>(bad.clone()).is_err(), "{bad}");
        }
    }

    #[test]
    fn passwords_are_never_serialized() {
        let s = settings(json!({"security": {"openPassword": "secret", "permissionsPassword": "owner"}}));
        assert_eq!(s.security.open_password, "secret");
        let text = serde_json::to_string(&s).unwrap();
        assert!(!text.contains("secret") && !text.contains("owner") && !text.contains("Password"), "{text}");
    }

    #[test]
    fn checks_see_settings_read_from_json() {
        assert_eq!(settings(json!({"standard": "pdfX4"})).check(&Profiles).unwrap_err().id, "standardVersion");
        assert_eq!(settings(json!({"standard": "pdfX4", "compatibility": "1.6"})).check(&Profiles), Ok(()));
        let ppi = settings(json!({"compression": {"color": {"ppi": 5}}})).check(&Profiles).unwrap_err();
        assert_eq!(ppi.id, "compressionPpi");
        assert_eq!((ppi.value("field"), ppi.value("range"), ppi.value("value")), (Some("compression.color.ppi"), Some("9–2400 ppi"), Some("5")));
    }

    #[test]
    fn presets_round_trip_as_json() {
        for p in builtin_presets("VectorCraft") {
            let back: PdfPreset = serde_json::from_value(serde_json::to_value(&p).unwrap()).unwrap();
            assert_eq!(back, p, "{}", p.name);
        }
        let bare: PdfPreset = serde_json::from_value(json!({"name": "Mine"})).unwrap();
        assert_eq!((bare.name.as_str(), bare.description.as_str(), bare.settings.clone()), ("Mine", "", PdfSettings::default()));
        assert!(serde_json::to_value(&bare).unwrap().get("description").is_none());
    }
}
