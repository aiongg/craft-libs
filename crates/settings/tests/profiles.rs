//! `Profiles`.
mod common;

use common::Prefs;
use craft_settings::profiles::PROFILES_DIR;
use craft_settings::{ProfileError, Profiles};

fn prefs(language: &str, minutes: u32) -> Prefs {
    let mut p = Prefs::default();
    p.general.language = language.into();
    p.general.autosave_minutes = minutes;
    p
}

// Save, list (sorted ignoring case), load and delete round-trip a settings value.
#[test]
fn save_list_load_delete() {
    let tmp = tempfile::tempdir().unwrap();
    let profiles = Profiles::<Prefs>::new(Some(tmp.path()));
    assert!(profiles.list().is_empty());

    profiles.save("beta", &prefs("fr", 1)).unwrap();
    profiles.save("Alpha", &prefs("de", 2)).unwrap();
    profiles.save("gamma", &prefs("nan", 3)).unwrap();
    assert_eq!(profiles.list(), ["Alpha", "beta", "gamma"]);
    assert!(tmp.path().join(PROFILES_DIR).join("beta.toml").is_file());

    assert_eq!(profiles.load("beta").unwrap(), prefs("fr", 1));
    assert_eq!(profiles.load("ALPHA").unwrap(), prefs("de", 2), "names compare without case");

    // Saving under a name that differs only in case replaces that profile.
    profiles.save("BETA", &prefs("es", 9)).unwrap();
    assert_eq!(profiles.list(), ["Alpha", "BETA", "gamma"]);
    assert_eq!(profiles.load("beta").unwrap(), prefs("es", 9));

    profiles.delete("Gamma").unwrap();
    assert_eq!(profiles.list(), ["Alpha", "BETA"]);
    assert!(matches!(profiles.load("gamma"), Err(ProfileError::NotFound)));
    assert!(matches!(profiles.delete("gamma"), Err(ProfileError::NotFound)));
    for bad in ["", "  ", "a/b", "a\\b", "..", "x:y", "con"] {
        assert!(matches!(profiles.save(bad, &Prefs::default()), Err(ProfileError::BadName)), "{bad:?}");
    }
}

// Import takes the file stem and adds " 2" when the name is taken; a newer schema is Unreadable.
#[test]
fn import_names_and_refuses_newer_files() {
    let tmp = tempfile::tempdir().unwrap();
    let profiles = Profiles::<Prefs>::new(Some(&tmp.path().join("config")));
    profiles.save("Work", &prefs("fr", 4)).unwrap();

    let out = tmp.path().join("Work.toml");
    profiles.export("work", &out).unwrap();
    assert_eq!(profiles.import(&out).unwrap(), "Work 2");
    assert_eq!(profiles.import(&out).unwrap(), "Work 3");
    assert_eq!(profiles.list(), ["Work", "Work 2", "Work 3"]);
    assert_eq!(profiles.load("Work 3").unwrap(), prefs("fr", 4));

    let newer = tmp.path().join("newer.toml");
    std::fs::write(&newer, "schema_version = 99\n[general]\nlanguage = \"fr\"\n").unwrap();
    assert!(matches!(profiles.import(&newer), Err(ProfileError::Unreadable(_))));
    let garbage = tmp.path().join("garbage.toml");
    std::fs::write(&garbage, "this is [not toml").unwrap();
    assert!(matches!(profiles.import(&garbage), Err(ProfileError::Unreadable(_))));
    assert_eq!(profiles.list().len(), 3, "a refused file adds no profile");

    // An older schema is migrated on import; a section that doesn't fit falls back to its defaults.
    let old = tmp.path().join("old.toml");
    std::fs::write(&old, "schema_version = 1\n[appearance]\ndark = true\n[general]\nlanguage = 5\n").unwrap();
    assert_eq!(profiles.import(&old).unwrap(), "old");
    let loaded = profiles.load("old").unwrap();
    assert_eq!(loaded.appearance.theme, common::Theme::Dark);
    assert_eq!(loaded.general, Prefs::default().general);

    let text = Profiles::<Prefs>::to_text(&prefs("fr", 4)).unwrap();
    assert!(text.starts_with("schema_version = 2"), "{text}");
}
