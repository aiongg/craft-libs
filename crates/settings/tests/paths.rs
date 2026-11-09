//! Integration tests of craft-settings: paths.

mod common;

use std::path::PathBuf;

use common::{open_in, probe_with_env};
use craft_settings::paths::{Location, Portable, Probe, Rule, resolve};

#[test]
fn env_var_wins_over_portable_and_platform() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("portable"), "").unwrap();
    let mut probe = probe_with_env("TESTCRAFT_CONFIG_DIR", "/from/env");
    probe.exe_dir = Some(tmp.path().to_path_buf());
    probe.platform_config_dir = Some(PathBuf::from("/platform"));
    let r = resolve("testcraft", &Portable::DEFAULT, &Location::Auto, &probe);
    assert_eq!(r.dir, Some(PathBuf::from("/from/env")));
    assert_eq!(r.rule, Rule::EnvVar("TESTCRAFT_CONFIG_DIR".into()));
}

#[test]
fn portable_marker_next_to_executable_uses_config_folder_beside_it() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("portable"), "").unwrap();
    let probe = Probe { env: Box::new(|_| None), exe_dir: Some(tmp.path().to_path_buf()), platform_config_dir: Some(PathBuf::from("/platform")) };
    let r = resolve("testcraft", &Portable::DEFAULT, &Location::Auto, &probe);
    assert_eq!(r.rule, Rule::Portable);
    assert_eq!(r.dir, Some(tmp.path().join("config")));
}

#[cfg(target_os = "macos")]
#[test]
fn portable_marker_is_looked_for_next_to_the_app_bundle() {
    let tmp = tempfile::tempdir().unwrap();
    let exe_dir = tmp.path().join("Test.app/Contents/MacOS");
    std::fs::create_dir_all(&exe_dir).unwrap();
    std::fs::write(tmp.path().join("portable"), "").unwrap();
    let probe = Probe { env: Box::new(|_| None), exe_dir: Some(exe_dir), platform_config_dir: None };
    let r = resolve("testcraft", &Portable::DEFAULT, &Location::Auto, &probe);
    assert_eq!(r.rule, Rule::Portable);
    assert_eq!(r.dir, Some(tmp.path().join("config")));
}

#[cfg(unix)]
#[test]
fn unwritable_folder_is_reported_at_open_not_first_save() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("ro");
    std::fs::create_dir(&dir).unwrap();
    if !common::restrict_directory_writes(&dir) {
        return;
    }
    let (mut store, report) = open_in(&dir);
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert!(store.is_read_only());
    assert!(matches!(report.read_only, Some(craft_settings::ReadOnlyReason::DirNotWritable(_))), "{report:?}");
    assert!(matches!(store.flush(), Err(craft_settings::SettingsError::ReadOnly(_))));
}
