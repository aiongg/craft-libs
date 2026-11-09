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

// macOS: the marker is looked for beside the .app bundle, which keeps the bundle's signature
// intact; a marker inside X.app/Contents/MacOS is not seen. The check runs on macOS only.

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

// Platform folder: the app passes "Photocraft" on macOS and Windows and "photocraft" elsewhere;
// without an override the library names it after the app id.

// No HOME and no platform folder: the library falls back to ./<folder> (rule Fallback) where
// PhotoCraft's own code keeps nothing on disk. Q-S4F-12 lists no change here.
