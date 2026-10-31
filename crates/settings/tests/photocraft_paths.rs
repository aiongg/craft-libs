//! Gap 61: PhotoCraft's folder rules (`apps/photocraft/src/app_dirs.rs:457 resolve`) against
//! `craft_settings::paths::resolve_named`, as settled by Q-S4F-12. PhotoCraft passes its marker
//! names (`Portable { markers: &["portable.txt", "PhotoCraft.portable"], dir: "PhotoCraftData" }`),
//! the app id `photocraft` and the folder name `Photocraft` on macOS and Windows. The library
//! keeps its own rule in three places, and the tests below assert the library's rule there: an
//! empty `PHOTOCRAFT_CONFIG_DIR` isolates the store, the macOS marker sits beside the `.app`, and
//! without a user folder the store uses `./photocraft`.
mod common;

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use craft_settings::paths::{Location, Portable, Probe, Rule, resolve_named};

const PHOTOCRAFT: Portable = Portable { markers: &["portable.txt", "PhotoCraft.portable"], dir: "PhotoCraftData" };

fn probe(env: Option<&'static str>, exe_dir: Option<&Path>, platform: Option<&Path>) -> Probe {
    Probe {
        env: Box::new(move |name| (name == "PHOTOCRAFT_CONFIG_DIR").then(|| env.map(OsString::from)).flatten()),
        exe_dir: exe_dir.map(Path::to_path_buf),
        platform_config_dir: platform.map(Path::to_path_buf),
    }
}

const FOLDER: &str = if cfg!(any(target_os = "macos", windows)) { "Photocraft" } else { "photocraft" };

fn resolved(probe: &Probe) -> craft_settings::Resolved {
    resolve_named("photocraft", FOLDER, &PHOTOCRAFT, &Location::Auto, probe)
}

// PHOTOCRAFT_CONFIG_DIR, non-empty, wins (PhotoCraft: Override; library: EnvVar).
#[test]
fn env_override_matches() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("portable.txt"), "").unwrap();
    let r = resolved(&probe(Some("/custom"), Some(tmp.path()), Some(Path::new("/platform"))));
    assert_eq!(r.dir, Some(PathBuf::from("/custom")));
    assert_eq!(r.rule, Rule::EnvVar("PHOTOCRAFT_CONFIG_DIR".into()));
}

// An empty PHOTOCRAFT_CONFIG_DIR: the library isolates the store (no folder), so tests and agents
// never touch the real profile. PhotoCraft's own rule (fall through to the platform folder) is not
// adopted.
#[test]
fn empty_env_var() {
    let r = resolved(&probe(Some(""), None, Some(Path::new("/platform"))));
    assert_eq!(r.rule, Rule::Isolated("PHOTOCRAFT_CONFIG_DIR".into()));
    assert_eq!(r.dir, None);
}

// portable.txt or PhotoCraft.portable beside the executable gives <exe dir>/PhotoCraftData.
#[test]
fn portable_markers_beside_the_executable() {
    for marker in ["portable.txt", "PhotoCraft.portable"] {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(marker), "").unwrap();
        let r = resolved(&probe(None, Some(tmp.path()), Some(Path::new("/platform"))));
        assert_eq!(r.rule, Rule::Portable, "{marker}");
        assert_eq!(r.dir, Some(tmp.path().join("PhotoCraftData")), "{marker}");
    }
    let tmp = tempfile::tempdir().unwrap();
    let r = resolved(&probe(None, Some(tmp.path()), Some(Path::new("/platform"))));
    assert_eq!(r.rule, Rule::Platform, "no marker, no portable mode");
}

// macOS: the marker is looked for beside the .app bundle, which keeps the bundle's signature
// intact; a marker inside X.app/Contents/MacOS is not seen. The check runs on macOS only.
#[test]
#[cfg(target_os = "macos")]
fn macos_marker_location() {
    let tmp = tempfile::tempdir().unwrap();
    let exe_dir = tmp.path().join("Photo.app/Contents/MacOS");
    std::fs::create_dir_all(&exe_dir).unwrap();
    std::fs::write(exe_dir.join("portable.txt"), "").unwrap();
    let r = resolved(&probe(None, Some(&exe_dir), Some(Path::new("/platform"))));
    assert_eq!(r.rule, Rule::Platform, "a marker inside the bundle is ignored");
    std::fs::write(tmp.path().join("portable.txt"), "").unwrap();
    let r = resolved(&probe(None, Some(&exe_dir), Some(Path::new("/platform"))));
    assert_eq!(r.rule, Rule::Portable);
    assert_eq!(r.dir, Some(tmp.path().join("PhotoCraftData")));
}

// A portable folder that isn't writable: the platform folder is used and the warning says why.
#[test]
#[cfg(unix)]
fn unwritable_portable_folder() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("portable.txt"), "").unwrap();
    let data = tmp.path().join("PhotoCraftData");
    std::fs::create_dir(&data).unwrap();
    std::fs::set_permissions(&data, std::fs::Permissions::from_mode(0o555)).unwrap();
    let r = resolved(&probe(None, Some(tmp.path()), Some(Path::new("/platform"))));
    std::fs::set_permissions(&data, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(r.rule, Rule::Platform);
    assert_eq!(r.dir, Some(Path::new("/platform").join(FOLDER)));
    let warning = r.warning.expect("the fallback is reported");
    assert!(warning.contains("PhotoCraftData") && warning.contains("portable.txt"), "{warning}");
}

// A portable folder that can't even be created (a file is in the way) falls back too.
#[test]
fn uncreatable_portable_folder() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("portable.txt"), "").unwrap();
    std::fs::write(tmp.path().join("PhotoCraftData"), "a file").unwrap();
    let r = resolved(&probe(None, Some(tmp.path()), Some(Path::new("/platform"))));
    assert_eq!(r.rule, Rule::Platform);
    assert!(r.warning.is_some());
}

// Platform folder: the app passes "Photocraft" on macOS and Windows and "photocraft" elsewhere;
// without an override the library names it after the app id.
#[test]
fn platform_folder_names() {
    let r = resolved(&probe(None, None, Some(Path::new("/platform"))));
    assert_eq!(r.rule, Rule::Platform);
    assert_eq!(r.dir, Some(Path::new("/platform").join(FOLDER)));
    assert_eq!(r.warning, None);

    let by_id = craft_settings::paths::resolve("photocraft", &PHOTOCRAFT, &Location::Auto, &probe(None, None, Some(Path::new("/platform"))));
    assert_eq!(by_id.dir, Some(PathBuf::from("/platform/photocraft")), "no override: the app id");
}

// No HOME and no platform folder: the library falls back to ./<folder> (rule Fallback) where
// PhotoCraft's own code keeps nothing on disk. Q-S4F-12 lists no change here.
#[test]
fn no_platform_folder() {
    let r = resolved(&probe(None, None, None));
    assert_eq!(r.rule, Rule::Fallback);
    assert_eq!(r.dir, Some(Path::new(".").join(FOLDER)));
}
