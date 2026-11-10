//! Which install this launcher is, its version and whether it can update itself.

#[cfg(windows)]
use super::helper::is_own_executable;
use super::*;

/// Makes an installed copy believe it's this version, to try the self-update against the current
/// release. The restart helper drops it, so it lasts one run.
pub const PRETEND_VAR: &str = "ARTCRAFT_LAUNCHER_PRETEND_VERSION";

/// Makes a development build believe it's an installed copy, to try the self-update.
pub const DEV_VAR: &str = "CRAFT_UPDATE_DEV";

/// The version this app counts as (see [`PRETEND_VAR`]).
pub fn current_version(app: &App, fallback_pkg_version: &str) -> Option<Version> {
    let app_pretend = pretend_var(app);
    std::env::var(&app_pretend)
        .or_else(|_| std::env::var(PRETEND_VAR))
        .ok()
        .and_then(|v| version::parse(&v))
        .or_else(|| version::parse(fallback_pkg_version))
}

pub(super) fn pretend_var(app: &App) -> String {
    format!("{}_PRETEND_VERSION", app.id.replace('-', "_").to_uppercase())
}

/// A pretend version lasts one run, including when an update schedules a restart.
pub(super) fn clear_pretend_version(cmd: &mut std::process::Command, app: &App) {
    cmd.env_remove(PRETEND_VAR).env_remove(pretend_var(app));
}

/// How long a just-installed version stays exempt from being offered again.
pub(super) const JUST_UPDATED_FOR: i64 = 24 * 3600;

/// Whether `release` is the version this app just updated itself to. Offering it again would
/// loop: update, restart, "update available", … That happens if a release's binary reports a
/// different version from its tag, or while a pretend version is set.
pub fn just_updated_to(paths: &Paths, release: &Release) -> bool {
    let Ok(text) = std::fs::read_to_string(marker(paths)) else { return false };
    let mut lines = text.lines();
    let (Some(tag), Some(at)) = (lines.next(), lines.next().and_then(|a| a.parse::<i64>().ok())) else { return false };
    tag == release.tag && crate::now().checked_sub(at).is_some_and(|age| (0..JUST_UPDATED_FOR).contains(&age))
}

pub(super) fn marker(paths: &Paths) -> PathBuf {
    paths.data.join("self-update.txt")
}

/// How this copy is installed, if it's one the app knows how to replace.
pub fn current(app: &App) -> Option<Install> {
    let exe = std::env::current_exe().ok()?;
    current_at(app, &exe, std::env::var_os(DEV_VAR).is_some())
}

/// [`current`] for a known executable and opt-in setting, without reading the ambient
/// environment.
pub(super) fn current_at(app: &App, exe: &Path, opted_in: bool) -> Option<Install> {
    let exe = exe.canonicalize().unwrap_or_else(|_| exe.to_path_buf());
    if is_development_build(&exe, opted_in) {
        return None;
    }
    detect(app, &exe)
}

/// Cargo's cache-directory marker at the root of a target directory.
pub(super) const CARGO_CACHE_MARKER: &str = "CACHEDIR.TAG";

/// Whether `exe` runs from a Cargo build tree, wherever `CARGO_TARGET_DIR` or
/// `build.target-dir` put it. Normally Cargo caches rustc information in `.rustc_info.json` at
/// the target root, and that marker alone identifies the tree; a folder that installers put the
/// app in doesn't carry it, so this doesn't depend on the directory being called `target`. But
/// the cache can be disabled (`CARGO_CACHE_RUSTC_INFO=0`) or a tree copied without the hidden
/// file, so a profile output directory is also accepted when it has Cargo's `.fingerprint`,
/// `deps` and `build` subdirectories and its target root carries `CACHEDIR.TAG`.
/// `CACHEDIR.TAG` alone isn't Cargo-specific (the same signature marks any cache directory), so
/// it only corroborates that profile layout.
pub(super) fn is_development_build(exe: &Path, opted_in: bool) -> bool {
    !opted_in && exe.ancestors().skip(1).any(is_cargo_build_dir)
}

/// Whether `dir` is a Cargo target root (by its rustc-info cache marker) or a profile output
/// directory inside one (by the cache-disabled fallback).
pub(super) fn is_cargo_build_dir(dir: &Path) -> bool {
    if dir.join(".rustc_info.json").is_file() {
        return true;
    }
    let Some(profile_parent) = dir.parent() else { return false };
    let is_profile = [".fingerprint", "deps", "build"].into_iter().all(|name| dir.join(name).is_dir());
    if !is_profile {
        return false;
    }
    // The target root is the profile's parent, or its grandparent for cross-compiled output
    // (`<target>/<triple>/<profile>`). It must carry Cargo's marker, as a file rather than a
    // directory of that name.
    if profile_parent.join(CARGO_CACHE_MARKER).is_file() {
        return true;
    }
    profile_parent.parent().is_some_and(|root| root.join(CARGO_CACHE_MARKER).is_file())
}

/// Whether this installed copy can update itself.
pub fn is_self_updating(app: &App) -> bool {
    current(app).is_some()
}

/// Whether `exe` sits in a folder a macOS package manager owns (Homebrew, MacPorts, Nix).
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(super) fn macos_package_manager_owns(exe: &Path) -> bool {
    // Homebrew (both prefixes), MacPorts and the Nix store; component-wise, so `/opt/homebrewed` is not one
    ["/opt/homebrew", "/usr/local/Cellar", "/usr/local/Homebrew", "/usr/local/opt", "/opt/local", "/nix"].iter().any(|root| exe.starts_with(root))
}

/// Whether `exe` sits in a folder a Windows package manager owns (scoop, winget, Chocolatey, the
/// Store).
#[cfg_attr(not(windows), allow(dead_code))]
pub(super) fn windows_package_manager_owns(exe: &Path) -> bool {
    // Compared as text so the rule reads the same wherever it is tested: either separator, any case.
    let text = exe.to_string_lossy().to_lowercase();
    let parts: Vec<&str> = text.split(['\\', '/']).collect();
    parts.contains(&"windowsapps")
        || parts.windows(2).any(|w| matches!((w[0], w[1]), ("scoop", "apps" | "shims") | ("winget", "packages") | ("chocolatey", "lib" | "bin")))
}

#[cfg(target_os = "macos")]
pub(super) fn detect(app: &App, exe: &Path) -> Option<Install> {
    if let Some(bundle) = exe.ancestors().find(|p| p.extension().is_some_and(|e| e == "app"))
        && let Ok(plist) = plist::Value::from_file(bundle.join("Contents/Info.plist"))
        && let Some(id) = plist.as_dictionary().and_then(|d| d.get("CFBundleIdentifier")).and_then(|v| v.as_string())
        && id == app.bundle_id()
    {
        return Some(Install::MacApp(bundle.to_path_buf()));
    }
    // A package manager's copy is its to update (replacing it in place leaves its record stale), and
    // a program inside another app's bundle is that app's file.
    if macos_package_manager_owns(exe) || exe.ancestors().any(|p| p.extension().is_some_and(|e| e == "app")) {
        return None;
    }
    if exe.is_file() {
        return Some(Install::Standalone(exe.to_path_buf()));
    }
    None
}

/// How the Windows copy at `exe` was installed: portable (a `portable.txt` beside it), by its MSI
/// (registered in either context, or the launcher's own per-user package), else a loose folder.
#[cfg(windows)]
pub(super) fn detect(app: &App, exe: &Path) -> Option<Install> {
    let dir = exe.parent()?;
    if dir.join("portable.txt").is_file() {
        return Some(Install::Portable(exe.to_path_buf()));
    }
    // Any folder, either context: the MSI's own registration says so (see `msi_copy`). A copy
    // installed for everyone must never be unpacked over as if it were a loose folder.
    if crate::msi_copy::windows::find(app, exe).is_some() {
        return Some(Install::Msi(exe.to_path_buf()));
    }
    // scoop, winget, Chocolatey and the Store own their copies; an in-place replace leaves their record stale
    if windows_package_manager_owns(exe) {
        return None;
    }
    if exe.is_file() {
        return Some(Install::Standalone(exe.to_path_buf()));
    }
    None
}

/// What `exe`'s MSI update does (update in place, or move a per-user copy to everyone), or why it
/// stops. Asked while the app still runs (so it can show the error) and again by the helper.
#[cfg(windows)]
pub(super) fn msi_step(app: &App, exe: &Path) -> Result<crate::msi_migrate::Step, String> {
    crate::msi_migrate::windows::step(app, exe)
}

/// Whether the helper starts the app after the update step: always after a successful one, and
/// after a failed one only while its exe is still there.
#[cfg_attr(not(windows), allow(dead_code))]
pub(super) fn restarts(step_succeeded: bool, exe_exists: bool) -> bool {
    step_succeeded || exe_exists
}

/// The exe to start after an MSI update of a registered copy: the one its registrations name now
/// (the package may have moved an exact old default folder), else `exe`.
#[cfg(windows)]
pub(super) fn exe_after_msi(app: &App, registered: bool, exe: &Path) -> PathBuf {
    registered
        .then(|| crate::msi_copy::registered_exe(&crate::msi_copy::windows::registered_exes(app)))
        .flatten()
        .filter(|moved| is_own_executable(app, moved))
        .unwrap_or_else(|| exe.to_path_buf())
}

#[cfg(target_os = "linux")]
pub(super) fn detect(app: &App, exe: &Path) -> Option<Install> {
    if process::in_sandbox() {
        // A release bundle updates itself in place. Per-user uses `flatpak install --user`.
        // System-wide uses `pkexec flatpak install --system`: `--user` would leave a second copy
        // beside the system one, and Flatpak's own install-bundle polkit action is denied on
        // current polkit when its message cannot fill in `$(path)`. A copy from a repository is
        // that repository's to update, and an unsigned bundle can't go into a signed one.
        let level = flatpak_level()?;
        return from_bundle(app, level).then_some(Install::Flatpak);
    }
    if let Some(appimage) = std::env::var_os("APPIMAGE") {
        return Some(Install::AppImage(PathBuf::from(appimage)));
    }
    let t = std::time::Duration::from_secs(10);
    if process::capture("dpkg-query", &["-W", "-f", "${db:Status-Status}", app.id], t).is_some_and(|s| s.trim() == "installed") {
        return Some(Install::Package(PackageKind::Deb, exe.to_path_buf()));
    }
    if process::capture("rpm", &["-q", app.id], t).is_some() {
        return Some(Install::Package(PackageKind::Rpm, exe.to_path_buf()));
    }
    // A pacman install of the app. We detect the release's own Arch package (by bundle id).
    // AUR builds (`<id>`, `<id>-bin`, `<id>-git`) are left to pacman and the AUR helper.
    if crate::detect::linux::has_pacman_db() && process::capture("pacman", &["-Q", &app.bundle_id()], t).is_some() {
        return Some(Install::Package(PackageKind::ArchPkg, exe.to_path_buf()));
    }
    if exe.is_file() {
        return Some(Install::Standalone(exe.to_path_buf()));
    }
    None
}

/// Where this sandbox's Flatpak copy is installed, from `app-path` in `/.flatpak-info`.
#[cfg(target_os = "linux")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FlatpakLevel {
    User,
    System,
}

#[cfg(target_os = "linux")]
pub(super) fn flatpak_level() -> Option<FlatpakLevel> {
    let info = std::fs::read_to_string("/.flatpak-info").ok()?;
    parse_flatpak_level(&info)
}

#[cfg(target_os = "linux")]
pub(super) fn parse_flatpak_level(info: &str) -> Option<FlatpakLevel> {
    let mut in_instance = false;
    for line in info.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_instance = line == "[Instance]";
        } else if in_instance && let Some(path) = line.strip_prefix("app-path=") {
            // The host's path. The per-user installation is ~/.local/share/flatpak there; it can't
            // be looked up from in here, where XDG_DATA_HOME points into ~/.var/app instead.
            let path = path.trim();
            if path.contains("/.local/share/flatpak/") {
                return Some(FlatpakLevel::User);
            }
            if path.contains("/var/lib/flatpak/") {
                return Some(FlatpakLevel::System);
            }
            return None;
        }
    }
    None
}

/// Whether this copy was installed from a `.flatpak` bundle. `/.flatpak-info` doesn't say where
/// it came from, so the host's Flatpak is asked on the same installation the copy runs from.
#[cfg(target_os = "linux")]
pub(super) fn from_bundle(app: &App, level: FlatpakLevel) -> bool {
    let scope = match level {
        FlatpakLevel::User => "--user",
        FlatpakLevel::System => "--system",
    };
    process::capture_flatpak(&["info", scope, "--show-origin", &app.bundle_id()], std::time::Duration::from_secs(3))
        .is_some_and(|origin| is_bundle_origin(app, origin.trim()))
}

/// Flatpak names a bundle's origin after the last part of the app id, numbered when that name is
/// taken: `<id>-origin`, `<id>1-origin`, …
#[cfg(target_os = "linux")]
pub(super) fn is_bundle_origin(app: &App, origin: &str) -> bool {
    origin.strip_prefix(app.id).and_then(|rest| rest.strip_suffix("-origin")).is_some_and(|n| n.bytes().all(|b| b.is_ascii_digit()))
}

#[cfg(not(any(target_os = "macos", windows, target_os = "linux")))]
pub(super) fn detect(_app: &App, _exe: &Path) -> Option<Install> {
    None
}
