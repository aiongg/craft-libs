#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::report::contrast_report;

fn graphite() -> ThemeDef {
    builtins().iter().find(|t| t.id.as_str() == "craft.graphite").unwrap().clone()
}

fn keys(theme: &ThemeDef) -> Vec<(Option<String>, Mode, Contrast, Size)> {
    contrast_report(theme)
        .iter()
        .map(|r| (r.app.as_ref().map(ToString::to_string), r.appearance.mode, r.appearance.contrast, r.appearance.size))
        .collect()
}

#[test]
fn appearances_are_ordered_app_mode_contrast_size() {
    let mut theme = graphite();
    theme.variants = Variants::Both;
    theme.apps.clear();
    theme.apps.insert(AppId::parse("photocraft").unwrap(), AppOverrides::default());
    let got = keys(&theme);
    assert_eq!(got.len(), 2 * 2 * 2 * 2);
    let mut expected = Vec::new();
    for app in [None, Some("photocraft".to_owned())] {
        for mode in [Mode::Light, Mode::Dark] {
            for contrast in [Contrast::Standard, Contrast::High] {
                for size in [Size::Compact, Size::Comfortable] {
                    expected.push((app.clone(), mode, contrast, size));
                }
            }
        }
    }
    assert_eq!(got, expected);

    theme.variants = Variants::LightOnly;
    theme.apps.clear();
    let got = keys(&theme);
    assert_eq!(got.len(), 4);
    assert!(got.iter().all(|k| k.1 == Mode::Light));
}

#[test]
fn appearance_warnings_equal_resolve_warnings() {
    let theme = graphite();
    let report = contrast_report(&theme);
    for (mode, contrast, size) in [(Mode::Light, Contrast::Standard, Size::Compact), (Mode::Dark, Contrast::High, Size::Comfortable)] {
        let entry = report.for_appearance(None, mode, contrast, size).unwrap();
        let appearance = Appearance { mode, contrast, size, text_size: TextSize::Default, reduced_motion: false };
        assert_eq!(format!("{:?}", entry.warnings), format!("{:?}", resolve(&theme, appearance, None).warnings));
    }
    assert!(report.for_appearance(Some(&AppId::parse("nope").unwrap()), Mode::Light, Contrast::Standard, Size::Compact).is_none());
}
