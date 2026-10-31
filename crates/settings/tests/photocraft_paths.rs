//! Gap 61: PhotoCraft's folder rules (`apps/photocraft/src/app_dirs.rs:457 resolve`) against
//! `craft_settings::paths::resolve`. Each test states PhotoCraft's behaviour and checks what the
//! library gives with PhotoCraft's names (`Portable { markers: &["portable.txt",
//! "PhotoCraft.portable"], dir: "PhotoCraftData" }`, app id `photocraft`). A difference fails the
//! test; the spec's Q-S4F-12 decides each one before PhotoCraft adopts the library's rules.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// PHOTOCRAFT_CONFIG_DIR, non-empty, wins (PhotoCraft: Override; library: EnvVar).
#[test]
#[ignore = "skeleton"]
fn env_override_matches() {
    todo!()
}

// An empty PHOTOCRAFT_CONFIG_DIR: PhotoCraft ignores it and falls through; the library isolates
// (no folder).
#[test]
#[ignore = "skeleton"]
fn empty_env_var() {
    todo!()
}

// portable.txt or PhotoCraft.portable beside the executable gives <exe dir>/PhotoCraftData.
#[test]
#[ignore = "skeleton"]
fn portable_markers_beside_the_executable() {
    todo!()
}

// macOS: PhotoCraft looks for the marker beside the executable inside the bundle; the library
// beside the .app.
#[test]
#[ignore = "skeleton"]
fn macos_marker_location() {
    todo!()
}

// A portable folder that isn't writable: PhotoCraft falls back to the platform folder with a
// warning; the library doesn't probe.
#[test]
#[ignore = "skeleton"]
fn unwritable_portable_folder() {
    todo!()
}

// Platform folder: PhotoCraft uses "Photocraft" on macOS (Application Support) and Windows
// (%APPDATA%), "photocraft" under XDG_CONFIG_HOME or ~/.config; the library joins the app id to
// dirs::config_dir().
#[test]
#[ignore = "skeleton"]
fn platform_folder_names() {
    todo!()
}

// No HOME and no platform folder: PhotoCraft has no folder; the library falls back to ./photocraft.
#[test]
#[ignore = "skeleton"]
fn no_platform_folder() {
    todo!()
}
