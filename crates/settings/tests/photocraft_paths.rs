//! PhotoCraft's folder rules against `craft_settings::paths::resolve_named`: the app passes its
//! marker names, its app id and a folder name. The cases that follow the library's own rules
//! (precedence, markers, fallback) are in paths.rs.
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

const FOLDER: &str = "Photocraft";

fn resolved(probe: &Probe) -> craft_settings::Resolved {
    resolve_named("photocraft", FOLDER, &PHOTOCRAFT, &Location::Auto, probe)
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

// The platform folder is named by the folder the app passes, and by the app id when none is passed.
#[test]
fn platform_folder_uses_the_passed_name_or_the_app_id() {
    let platform = Path::new("/platform");
    let r = resolved(&probe(None, None, Some(platform)));
    assert_eq!(r.rule, Rule::Platform);
    assert_eq!(r.dir, Some(platform.join(FOLDER)));
    assert_eq!(r.warning, None);

    let by_id = craft_settings::paths::resolve("photocraft", &PHOTOCRAFT, &Location::Auto, &probe(None, None, Some(platform)));
    assert_eq!(by_id.dir, Some(platform.join("photocraft")), "no override: the app id");
}
