#![allow(clippy::unwrap_used, clippy::expect_used)]

use serde_json::{Value, json};

use super::format::{self, ErrorKind, FormatErrors, ThemeFile};
use super::semantic_tests::{app, appearances, grey_blue, hex};
use super::*;
use crate::AppId;
use ColorToken as T;

/// A valid file that uses every section, with unknown keys at several levels.
fn full() -> Value {
    json!({
        "schema": "craft-design-theme", "schema_version": 1,
        "x_top": { "list": [1, 2, { "k": null }] },
        "id": "test.full", "name": "Full", "version": 3,
        "variants": "both", "contrast_intent": "low",
        "seeds": {
            "neutral": "#535353", "accent": "#78ADF6", "x_seed": "kept",
            "status": { "good": "#85DC6A", "warn": "#E8B046", "bad": "#FF8080", "info": "#78ADF6", "x_status": 1 }
        },
        "shape": {
            "radius": { "control": 3, "popup": 4, "window": 6, "panel": 0, "button": "full", "x_radius": 1 },
            "edge": { "bevel": { "width": 1 } }, "x_shape": true
        },
        "light": { "surface.panel": "#F0F0F0", "button.primary.fill.rest": "#1062C4" },
        "dark": { "surface.panel": "#535353" },
        "high_contrast": { "light": { "text.normal": "#000000" }, "dark": {}, "x_hc": "kept" },
        "components": { "button.radius": 4.5 },
        "apps": {
            "photocraft": {
                "light": {}, "dark": { "surface.canvas": "#282828" },
                "high_contrast": { "dark": { "text.normal": "#FFFFFF" }, "x_app_hc": 1 },
                "components": { "button.radius": "full" }, "x_app": [true]
            },
            "x_app_entry": {}
        }
    })
}

fn bytes(v: &Value) -> Vec<u8> {
    serde_json::to_vec(v).unwrap()
}

fn parse_value(v: &Value) -> Result<ThemeFile, FormatErrors> {
    format::parse(&bytes(v))
}

fn errors(v: &Value) -> Vec<String> {
    parse_value(v).unwrap_err().0.iter().map(ToString::to_string).collect()
}

#[test]
fn round_trip_keeps_values_and_unknown_keys() {
    let file = parse_value(&full()).unwrap();
    let t = &file.theme;
    assert_eq!((t.id.as_str(), t.name.as_str(), t.version), ("test.full", "Full", 3));
    assert_eq!((t.variants, t.contrast_intent), (Variants::Both, ContrastIntent::Low));
    assert_eq!(t.seeds.accent, hex("#78adf6"));
    let shape = t.shape.unwrap();
    assert_eq!(shape.radius.unwrap().button, 999.0);
    assert_eq!(shape.edge, Some(EdgeSeed::Bevel { width: 1.0 }));
    assert_eq!(t.pins.light.get(T::SurfacePanel), Some(hex("#f0f0f0")));
    assert_eq!(t.high_contrast.light.get(T::TextNormal), Some(hex("#000000")));
    assert_eq!(t.components.len(), 1);
    let photo = &t.apps[&AppId::parse("photocraft").unwrap()];
    assert_eq!(photo.pins.dark.get(T::SurfaceCanvas), Some(hex("#282828")));
    assert_eq!(photo.high_contrast.dark.get(T::TextNormal), Some(hex("#ffffff")));
    assert_eq!(photo.components.values().next(), Some(&OverrideValue::Length(999.0)));

    let text = format::write(&file).unwrap();
    let written: Value = serde_json::from_str(&text).unwrap();
    let orig = full();
    for (path, expected) in [
        ("/x_top", &orig["x_top"]),
        ("/seeds/x_seed", &orig["seeds"]["x_seed"]),
        ("/seeds/status/x_status", &orig["seeds"]["status"]["x_status"]),
        ("/shape/x_shape", &orig["shape"]["x_shape"]),
        ("/shape/radius/x_radius", &orig["shape"]["radius"]["x_radius"]),
        ("/high_contrast/x_hc", &orig["high_contrast"]["x_hc"]),
        ("/apps/photocraft/x_app", &orig["apps"]["photocraft"]["x_app"]),
        ("/apps/photocraft/high_contrast/x_app_hc", &orig["apps"]["photocraft"]["high_contrast"]["x_app_hc"]),
        ("/apps/photocraft/high_contrast/dark/text.normal", &json!("#ffffff")),
        ("/apps/x_app_entry", &orig["apps"]["x_app_entry"]),
        ("/version", &json!(3)),
        ("/components/button.radius", &json!(4.5)),
        ("/shape/radius/button", &json!("full")),
        ("/dark/surface.panel", &json!("#535353")),
    ] {
        assert_eq!(written.pointer(path), Some(expected), "{path}");
    }
    assert!(text.starts_with("{\n  \"schema\": \"craft-design-theme\",\n  \"schema_version\": 1,\n  \"id\""), "{text}");
    let again = format::parse(text.as_bytes()).unwrap();
    assert_eq!(again.theme, file.theme);
    assert_eq!(format::write(&again).unwrap(), text, "writing is stable");
}

#[test]
fn write_follows_the_theme_not_the_old_file() {
    let mut file = parse_value(&full()).unwrap();
    file.theme.shape = None;
    file.theme.pins.dark = Pins::default();
    file.theme.apps.clear();
    file.theme.name = "Renamed".to_owned();
    let written: Value = serde_json::from_str(&format::write(&file).unwrap()).unwrap();
    assert_eq!(written["shape"], Value::Null);
    assert_eq!(written["dark"], json!({}));
    assert_eq!(written["name"], "Renamed");
    assert_eq!(written["apps"], json!({}));
    assert_eq!(written["x_top"], full()["x_top"]);
}

#[test]
fn defaults_and_exported_themes() {
    let min = json!({
        "schema": "craft-design-theme", "schema_version": 1, "id": "test.min",
        "seeds": { "neutral": "#535353", "accent": "#78ADF6",
                   "status": { "good": "#85DC6A", "warn": "#E8B046", "bad": "#FF8080", "info": "#78ADF6" } }
    });
    let t = parse_value(&min).unwrap().theme;
    assert_eq!((t.name.as_str(), t.version, t.variants, t.contrast_intent), ("test.min", 1, Variants::Both, ContrastIntent::Standard));
    assert!(t.shape.is_none() && t.apps.is_empty() && t.components.is_empty());

    let mut def = grey_blue();
    def.pins.light = Pins::from_iter([(T::TextFaint, hex("#c8c8c8"))]);
    def.shape = Some(ShapeSeed { radius: None, radius_offset: None, edge: Some(EdgeSeed::Flat) });
    let file = format::to_file(&def).unwrap();
    let text = format::write(&file).unwrap();
    assert_eq!(format::parse(text.as_bytes()).unwrap().theme, def);
    assert!(!text.contains("\"high_contrast\"") && !text.contains("\"apps\""), "empty sections are left out:\n{text}");
}

#[test]
fn file_level_errors() {
    let pilot = format::parse(br#"{"schema":"craft-theme","schema_version":1}"#).unwrap_err();
    assert_eq!(pilot.file_level(), Some(&ErrorKind::OtherSchema { schema: "craft-theme".to_owned() }));
    let big = vec![b' '; format::MAX_THEME_BYTES + 1];
    assert_eq!(format::parse(&big).unwrap_err().file_level(), Some(&ErrorKind::TooLarge { len: format::MAX_THEME_BYTES + 1 }));
    assert!(matches!(format::parse(b"{").unwrap_err().file_level(), Some(ErrorKind::Json(_))));
    assert_eq!(format::parse(b"[]").unwrap_err().file_level(), Some(&ErrorKind::NotObject));
    let mut v = full();
    v["schema_version"] = json!(2);
    assert_eq!(parse_value(&v).unwrap_err().file_level(), Some(&ErrorKind::TooNew { schema_version: 2 }));
    for bad in [json!(0), json!("1"), json!(1.5)] {
        v["schema_version"] = bad;
        assert_eq!(parse_value(&v).unwrap_err().file_level(), Some(&ErrorKind::BadSchemaVersion));
    }
    assert_eq!(errors(&json!({ "schema_version": 1 })), ["schema: missing"]);
    let mut with_bom = b"\xEF\xBB\xBF".to_vec();
    with_bom.extend(bytes(&full()));
    assert!(format::parse(&with_bom).is_ok());
}

#[test]
fn every_field_error_has_its_path() {
    let cases: [(&str, Value, &str); 23] = [
        ("/dark/surface.panel", json!("nope"), "dark.\"surface.panel\": not a colour"),
        ("/dark/surface.panel", json!("#53535380"), "dark.\"surface.panel\": surfaces must be opaque"),
        ("/high_contrast/light/surface.canvas", json!("#00000000"), "high_contrast.light.\"surface.canvas\": surfaces must be opaque"),
        ("/light/surface.nothing", json!("#ffffff"), "light.\"surface.nothing\": unknown token"),
        ("/light/button.radius", json!("#ffffff"), "light.\"button.radius\": not a colour field; set it in components"),
        (
            "/components/button.primary.fill.rest",
            json!("#ffffff"),
            "components.\"button.primary.fill.rest\": colours are set per mode, in light or dark",
        ),
        ("/components/surface.panel", json!("#ffffff"), "components.\"surface.panel\": colours are set per mode, in light or dark"),
        ("/components/button.radius", json!({ "level": 1 }), "components.\"button.radius\": expected a Length value, found Elevation"),
        ("/components/button.edge", json!("flat"), "components.\"button.edge\": unknown token"),
        ("/components/button.nope", json!(4), "components.\"button.nope\": unknown token"),
        ("/components/button.radius", json!(-1), "components.\"button.radius\": expected a length: a number of 0 or more, or \"full\""),
        ("/components/button.radius", json!({ "x": 1 }), "components.\"button.radius\": expected a length: a number of 0 or more, or \"full\""),
        ("/id", json!("Bad Id"), "id: a theme id starts with a-z or 0-9"),
        ("/id", json!(4), "id: expected a string"),
        ("/seeds", json!(4), "seeds: expected an object"),
        ("/seeds/status/good", json!("red"), "seeds.status.good: not a colour"),
        ("/shape/radius/panel", Value::Null, "shape.radius.panel: missing"),
        ("/shape/edge", json!({ "bevel": { "width": 0 } }), "shape.edge: expected \"flat\" or {\"bevel\": {\"width\": n}} with n above 0"),
        ("/variants", json!("blue"), "variants: expected one of the listed names"),
        ("/version", json!(-1), "version: expected a non-negative integer"),
        ("/apps/photocraft/dark/surface.canvas", json!(1), "apps.photocraft.dark.\"surface.canvas\": not a colour"),
        ("/apps/photocraft/high_contrast/dark/text.normal", json!("x"), "apps.photocraft.high_contrast.dark.\"text.normal\": not a colour"),
        (
            "/apps/photocraft/components/button.radius",
            json!({ "level": 1 }),
            "apps.photocraft.components.\"button.radius\": expected a Length value, found Elevation",
        ),
    ];
    for (pointer, value, expected) in cases {
        let mut v = full();
        let (parent, key) = pointer.rsplit_once('/').unwrap();
        v.pointer_mut(parent).unwrap().as_object_mut().unwrap().insert(key.to_owned(), value);
        assert_eq!(errors(&v), [expected], "{pointer}");
    }

    let mut v = full();
    v["seeds"].as_object_mut().unwrap().remove("accent");
    v["apps"]["Bad App"] = json!({});
    v["variants"] = json!("light");
    v["dark"] = json!({ "surface.panel": "#535353" });
    let got = errors(&v);
    assert_eq!(got.len(), 5, "{got:?}");
    assert_eq!(got[0], "seeds.accent: missing");
    assert_eq!(got[1], "dark: the theme's variants leave this mode out");
    assert!(got[2].starts_with("apps.Bad App: not an app id"), "{got:?}");
    assert_eq!(got[3], "apps.photocraft.dark: the theme's variants leave this mode out");
    assert_eq!(got[4], "apps.photocraft.high_contrast.dark: the theme's variants leave this mode out");
}

#[test]
fn errors_are_reported_together() {
    let mut v = full();
    v["id"] = json!("Bad");
    v["seeds"]["neutral"] = json!("grey");
    v["dark"]["surface.panel"] = json!("nope");
    v["components"]["button.radius"] = json!({ "level": 1 });
    assert_eq!(
        errors(&v),
        [
            "id: a theme id starts with a-z or 0-9",
            "seeds.neutral: not a colour",
            "dark.\"surface.panel\": not a colour",
            "components.\"button.radius\": expected a Length value, found Elevation",
        ]
    );
}

#[test]
fn empty_map_for_a_missing_mode_is_accepted() {
    let mut v = full();
    v["variants"] = json!("dark");
    v["light"] = json!({});
    v["high_contrast"]["light"] = json!({});
    v["apps"]["photocraft"]["light"] = json!({});
    v["apps"]["photocraft"]["high_contrast"]["light"] = json!({});
    assert!(parse_value(&v).is_ok());
}

#[test]
fn app_overrides_apply_only_to_that_app() {
    let photo = AppId::parse("photocraft").unwrap();
    let other = AppId::parse("filmcraft").unwrap();
    let mut def = grey_blue();
    def.pins.dark = Pins::from_iter([(T::SurfaceRaised, hex("#303030"))]);
    def.components.insert(ComponentField::from_path("button.radius").unwrap(), OverrideValue::Length(2.0));
    def.apps.insert(
        photo.clone(),
        AppOverrides {
            pins: ModePins { light: Pins::default(), dark: Pins::from_iter([(T::SurfaceRaised, hex("#404040"))]) },
            high_contrast: ModePins { light: Pins::default(), dark: Pins::from_iter([(T::LineDivider, hex("#606060"))]) },
            components: [(ComponentField::from_path("button.radius").unwrap(), OverrideValue::Length(9.0))].into_iter().collect(),
        },
    );
    assert!(def.override_errors().is_empty());
    let dark = app(Mode::Dark, Contrast::Standard);
    let raised = |a: Option<&AppId>| resolve(&def, dark, a).tokens.color(T::SurfaceRaised);
    assert_eq!(raised(None), hex("#303030"));
    assert_eq!(raised(Some(&photo)), hex("#404040"));
    assert_eq!(raised(Some(&other)), hex("#303030"));
    let radius = |a: Option<&AppId>| resolve(&def, dark, a).tokens.component.button.radius;
    assert_eq!((radius(None), radius(Some(&photo)), radius(Some(&other))), (2.0, 9.0, 2.0));
    // The app's high-contrast pins apply after the theme's, which apply after the app's pins.
    def.high_contrast.dark = Pins::from_iter([(T::LineDivider, hex("#505050")), (T::SurfaceRaised, hex("#000000"))]);
    let at = |a: Option<&AppId>, contrast| resolve(&def, app(Mode::Dark, contrast), a).tokens;
    assert_eq!(at(Some(&photo), Contrast::High).color(T::LineDivider), hex("#606060"));
    assert_eq!(at(Some(&photo), Contrast::High).color(T::SurfaceRaised), Rgba::BLACK);
    assert_eq!(at(Some(&other), Contrast::High).color(T::LineDivider), hex("#505050"));
    assert_ne!(at(Some(&photo), Contrast::Standard).color(T::LineDivider), hex("#606060"));
    // Light mode has no app pin.
    let light = app(Mode::Light, Contrast::Standard);
    assert_eq!(resolve(&def, light, Some(&photo)).tokens.color, resolve(&def, light, None).tokens.color);
}

#[test]
fn version_is_part_of_the_theme() {
    let mut file = parse_value(&full()).unwrap();
    file.theme.version = 7;
    let text = format::write(&file).unwrap();
    assert_eq!(format::parse(text.as_bytes()).unwrap().theme.version, 7);
    let mut def = grey_blue();
    def.version = 4;
    let written: Value = serde_json::from_str(&format::write(&format::to_file(&def).unwrap()).unwrap()).unwrap();
    assert_eq!(written["version"], 4);
}

#[test]
fn write_refuses_what_parse_rejects() {
    for theme in builtins() {
        let text = format::write(&format::to_file(theme).unwrap()).unwrap();
        assert_eq!(&format::parse(text.as_bytes()).unwrap().theme, theme, "{}", theme.id);
    }

    // A theme narrowed to light after parsing still has its dark pins.
    let mut file = parse_value(&full()).unwrap();
    file.theme.variants = Variants::LightOnly;
    let paths = |e: FormatErrors| e.0.iter().map(ToString::to_string).collect::<Vec<_>>();
    let expected = [
        "dark: the theme's variants leave this mode out",
        "apps.photocraft.dark: the theme's variants leave this mode out",
        "apps.photocraft.high_contrast.dark: the theme's variants leave this mode out",
    ];
    assert_eq!(paths(format::write(&file).unwrap_err()), expected);
    assert_eq!(paths(format::to_file(&file.theme).unwrap_err()), expected);
    file.theme.pins.dark = Pins::default();
    file.theme.apps.clear();
    assert!(format::parse(format::write(&file).unwrap().as_bytes()).is_ok());

    let mut def = grey_blue();
    let radius = ComponentField::from_path("button.radius").unwrap();
    let fill = ComponentField::from_path("button.primary.fill.rest").unwrap();
    def.components = [(radius, OverrideValue::Length(f32::NAN)), (fill, OverrideValue::Color(Rgba::WHITE))].into_iter().collect();
    def.pins.light = Pins::from_iter([(TokenRef::Color(T::SurfacePanel), hex("#ffffff80")), (TokenRef::Component(radius), Rgba::WHITE)]);
    def.shape = Some(ShapeSeed {
        radius: Some(RadiusSet { control: -1.0, popup: 0.0, window: 0.0, panel: 0.0, button: 0.0, pill: 0.0 }),
        radius_offset: None,
        edge: Some(EdgeSeed::Bevel { width: 0.0 }),
    });
    let got = paths(def.validate().unwrap_err());
    assert_eq!(
        got,
        [
            "shape.radius.control: expected a length: a number of 0 or more, or \"full\"",
            "shape.edge: expected \"flat\" or {\"bevel\": {\"width\": n}} with n above 0",
            "light.\"surface.panel\": surfaces must be opaque",
            "light.\"button.radius\": not a colour field; set it in components",
            "components.\"button.radius\": expected a length: a number of 0 or more, or \"full\"",
            "components.\"button.primary.fill.rest\": colours are set per mode, in light or dark",
        ]
    );
}

#[test]
fn a_mode_map_overrides_seeds_field_by_field() {
    let mut v = json!({
        "schema": "craft-design-theme", "schema_version": 1, "id": "test.modeseeds",
        "seeds": { "neutral": "#323232", "accent": "#378ef0",
                   "status": { "good": "#85dc6a", "warn": "#e8b046", "bad": "#ec5b62", "info": "#378ef0" } },
        "light": { "seeds": { "neutral": "#f0f0f0", "status": { "good": "#4e712e" } } }
    });
    let t = parse_value(&v).unwrap().theme;
    let light = t.seeds_for(Mode::Light);
    assert_eq!((light.neutral, light.accent, light.status.good, light.status.warn), (hex("#f0f0f0"), hex("#378ef0"), hex("#4e712e"), hex("#e8b046")));
    assert_eq!(t.seeds_for(Mode::Dark), t.seeds);
    let neutral = |mode| {
        let a = appearance(mode);
        resolve(&t, a, None).tokens.color(ColorToken::SurfacePanel)
    };
    assert_ne!(neutral(Mode::Light), neutral(Mode::Dark));
    let plain = ThemeDef { mode_seeds: ModeSeeds::default(), ..t.clone() };
    let dark = appearance(Mode::Dark);
    assert_eq!(resolve(&t, dark, None), resolve(&plain, dark, None));

    let file = format::to_file(&t).unwrap();
    assert_eq!(format::parse(format::write(&file).unwrap().as_bytes()).unwrap().theme, t);

    v["high_contrast"] = json!({ "light": { "seeds": { "accent": "#000000" } } });
    assert_eq!(errors(&v), ["high_contrast.light.seeds: seeds are set in the theme's own light and dark maps only"]);
}

fn appearance(mode: Mode) -> Appearance {
    Appearance { mode, contrast: Contrast::Standard, size: Size::Compact, text_size: TextSize::Default, reduced_motion: false }
}

/// `float.shadow` is written as `{"level": n}` and read back as the same elevation.
#[test]
fn a_float_shadow_override_round_trips_through_the_file() {
    let mut def = grey_blue();
    let shadow = ComponentField::from_path("float.shadow").unwrap();
    for level in 0..Elevation::LEVELS.len() {
        def.components.insert(shadow, OverrideValue::Elevation(Elevation::level(level)));
        let text = format::write(&format::to_file(&def).unwrap()).unwrap();
        let written: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(written["components"]["float.shadow"], json!({ "level": level }));
        let back = format::parse(text.as_bytes()).unwrap().theme;
        assert_eq!(back.components[&shadow], OverrideValue::Elevation(Elevation::level(level)));
        assert_eq!(resolve(&back, appearance(Mode::Light), None).tokens.component.float.shadow, Elevation::level(level));
    }
    let bad = json!({ "level": 4 });
    let mut v = full();
    v["components"] = json!({ "float.shadow": bad });
    assert_eq!(errors(&v), ["components.\"float.shadow\": expected {\"level\": 0 to 3}"]);
}

#[test]
fn a_default_pill_radius_is_not_written_and_a_set_one_is() {
    let mut def = grey_blue();
    let radius = |pill| ShapeSeed {
        radius: Some(RadiusSet { control: 1.0, popup: 1.0, window: 1.0, panel: 1.0, button: 1.0, pill }),
        radius_offset: None,
        edge: None,
    };
    let written = |def: &ThemeDef| -> Value { serde_json::from_str(&format::write(&format::to_file(def).unwrap()).unwrap()).unwrap() };
    def.shape = Some(radius(Radius::Full.px()));
    assert!(written(&def)["shape"]["radius"].get("pill").is_none());
    def.shape = Some(radius(0.0));
    let w = written(&def);
    assert_eq!(w["shape"]["radius"]["pill"], 0.0);
    let back = format::parse(serde_json::to_string(&w).unwrap().as_bytes()).unwrap().theme;
    assert_eq!(back.shape.unwrap().radius.unwrap().pill, 0.0);
}

/// A theme file with the given light-mode pins.
fn with_light(pins: Value) -> Value {
    let mut v = full();
    v["light"] = pins;
    v
}

#[test]
fn references_resolve_to_the_final_value_in_every_appearance() {
    let v = with_light(json!({
        "slider.fill.rest": "{accent.solid}", "slider.fill.hover": "{accent.hover}",
        "text.muted": "{slider.fill.rest}", "accent.solid": "#204080"
    }));
    let mut v = v;
    v["dark"] = json!({ "slider.fill.rest": "{accent.solid}", "slider.fill.hover": "{accent.hover}" });
    v["high_contrast"] = json!({ "light": { "slider.fill.rest": "{text.normal}" }, "dark": {} });
    let file = parse_value(&v).unwrap();
    for a in appearances() {
        let t = resolve(&file.theme, a, None).tokens;
        let (s, accent) = (&t.component.slider.fill, |c| t.color(c));
        let high_light = a.contrast == Contrast::High && a.mode == Mode::Light;
        assert_eq!(s.rest, if high_light { accent(T::TextNormal) } else { accent(T::AccentSolid) }, "{a:?}");
        if !high_light {
            assert_eq!(s.hover, accent(T::AccentHover), "{a:?}");
        }
        if a.mode == Mode::Light {
            // A reference to a reference reads the pinned token's value, and a pin is read as pinned.
            assert_eq!(accent(T::TextMuted), s.rest, "{a:?}");
            if a.contrast == Contrast::Standard {
                assert_eq!(accent(T::AccentSolid), hex("#204080"));
            }
        }
    }
}

#[test]
fn references_survive_a_write_and_parse() {
    let file = parse_value(&with_light(json!({ "slider.fill.rest": "{accent.solid}" }))).unwrap();
    let again = format::parse(format::write(&file).unwrap().as_bytes()).unwrap();
    assert_eq!(again.theme.pins.light.refs, file.theme.pins.light.refs);
    assert!(again.theme.pins.light.colors.is_empty());
}

#[test]
fn unknown_reference_names_the_key_and_the_reference() {
    let e = errors(&with_light(json!({ "slider.fill.rest": "{accent.nope}" })));
    assert_eq!(e, ["light.\"slider.fill.rest\": reference {accent.nope} names no token"]);
    let e = errors(&with_light(json!({ "text.muted": "{button.radius}" })));
    assert_eq!(e, ["light.\"text.muted\": reference {button.radius} is not a colour field"]);
}

#[test]
fn reference_cycles_are_errors() {
    let e = errors(&with_light(json!({ "text.muted": "{text.faint}", "text.faint": "{text.muted}" })));
    assert_eq!(e.len(), 1, "{e:?}");
    assert!(
        e[0].contains("references form a cycle: text.faint -> text.muted -> text.faint") || e[0].contains("text.muted -> text.faint -> text.muted"),
        "{e:?}"
    );
    let e = errors(&with_light(json!({ "text.muted": "{text.muted}" })));
    assert_eq!(e.len(), 1, "{e:?}");
}

#[test]
fn classic_slider_fill_follows_its_accent_states() {
    let classic = builtins().iter().find(|t| t.id.as_str() == "craft.classic").unwrap();
    for a in appearances() {
        let t = resolve(classic, a, None).tokens;
        let s = &t.component.slider.fill;
        assert_eq!((s.rest, s.hover, s.pressed), (t.color(T::AccentSolid), t.color(T::AccentHover), t.color(T::AccentPressed)), "{a:?}");
    }
}
