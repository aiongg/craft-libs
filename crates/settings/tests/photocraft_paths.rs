//! Gap 61: PhotoCraft's folder rules (`apps/photocraft/src/app_dirs.rs:457 resolve`) against
//! `craft_settings::paths::resolve`. Each test states PhotoCraft's behaviour and checks what the
//! library gives with PhotoCraft's names (`Portable { markers: &["portable.txt",
//! "PhotoCraft.portable"], dir: "PhotoCraftData" }`, app id `photocraft`). A difference fails the
//! test; the spec's Q-S4F-12 decides each one before PhotoCraft adopts the library's rules.
mod common;

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use craft_settings::paths::{Location, Portable, Probe, Rule, resolve};

const PHOTOCRAFT: Portable = Portable { markers: &["portable.txt", "PhotoCraft.portable"], dir: "PhotoCraftData" };

fn probe(env: Option<&'static str>, exe_dir: Option<&Path>, platform: Option<&Path>) -> Probe {
    Probe {
        env: Box::new(move |name| (name == "PHOTOCRAFT_CONFIG_DIR").then(|| env.map(OsString::from)).flatten()),
        exe_dir: exe_dir.map(Path::to_path_buf),
        platform_config_dir: platform.map(Path::to_path_buf),
    }
}

fn resolved(probe: &Probe) -> craft_settings::Resolved {
    resolve("photocraft", &PHOTOCRAFT, &Location::Auto, probe)
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

// An empty PHOTOCRAFT_CONFIG_DIR: PhotoCraft ignores it and falls through; the library isolates
// (no folder).
#[test]
#[ignore = "Q-S4F-12: PhotoCraft ignores an empty PHOTOCRAFT_CONFIG_DIR and uses the platform folder; the library returns Rule::Isolated with no folder"]
fn empty_env_var() {
    let r = resolved(&probe(Some(""), None, Some(Path::new("/platform"))));
    assert_eq!(r.rule, Rule::Platform, "PhotoCraft falls through to the platform folder; the library gave {:?}", r.rule);
    assert_eq!(r.dir, Some(PathBuf::from("/platform/photocraft")));
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

// macOS: PhotoCraft looks for the marker beside the executable inside the bundle; the library
// beside the .app. The check runs on macOS only; other platforms have no bundle rule.
#[test]
#[ignore = "Q-S4F-12: on macOS PhotoCraft finds the marker beside the executable inside X.app/Contents/MacOS; the library looks beside X.app"]
fn macos_marker_location() {
    if !cfg!(target_os = "macos") {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let exe_dir = tmp.path().join("Photo.app/Contents/MacOS");
    std::fs::create_dir_all(&exe_dir).unwrap();
    std::fs::write(exe_dir.join("portable.txt"), "").unwrap();
    let r = resolved(&probe(None, Some(&exe_dir), Some(Path::new("/platform"))));
    assert_eq!(r.dir, Some(exe_dir.join("PhotoCraftData")), "the library gave {:?}", r.dir);
}

// A portable folder that isn't writable: PhotoCraft falls back to the platform folder with a
// warning; the library doesn't probe.
#[test]
#[ignore = "Q-S4F-12: PhotoCraft falls back to the platform folder when the portable folder isn't writable; the library returns the portable folder without probing"]
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
    assert_eq!(r.rule, Rule::Platform, "the library gave {:?}", r.rule);
    assert_eq!(r.dir, Some(PathBuf::from("/platform/photocraft")));
}

// Platform folder: PhotoCraft uses "Photocraft" on macOS (Application Support) and Windows
// (%APPDATA%), "photocraft" under XDG_CONFIG_HOME or ~/.config; the library joins the app id to
// dirs::config_dir().
#[test]
#[cfg_attr(
    any(target_os = "macos", windows),
    ignore = "Q-S4F-12: PhotoCraft names the folder Photocraft on macOS and Windows; the library uses the app id, photocraft"
)]
fn platform_folder_names() {
    let name = if cfg!(any(target_os = "macos", windows)) { "Photocraft" } else { "photocraft" };
    let r = resolved(&probe(None, None, Some(Path::new("/platform"))));
    assert_eq!(r.rule, Rule::Platform);
    assert_eq!(r.dir, Some(Path::new("/platform").join(name)));
}

// No HOME and no platform folder: PhotoCraft has no folder; the library falls back to ./photocraft.
#[test]
#[ignore = "Q-S4F-12: PhotoCraft has no folder without a user folder; the library falls back to ./photocraft"]
fn no_platform_folder() {
    let r = resolved(&probe(None, None, None));
    assert_eq!(r.dir, None, "the library gave {:?} ({})", r.dir, r.rule);
}
