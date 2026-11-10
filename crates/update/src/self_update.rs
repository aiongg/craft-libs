//! ArtCraft Launcher updating itself.
//!
//! The new version is downloaded and verified like any Craft App, then swapped in the way this
//! copy was installed. A small helper process waits for this launcher to exit, finishes anything
//! that can't happen while it runs (Windows locks a running exe), and starts the new one. The
//! launcher has to exit first anyway: it holds the single-instance lock, and a new copy started
//! while it runs would only ask it to show its window.
//!
//! | Installed as | Swap |
//! |---|---|
//! | macOS `.app` | copy the new bundle from the DMG over this one (fine while running), reopen it |
//! | Windows MSI (any folder, for one account or everyone) | after exit, run the new MSI in the copy's own context (everyone asks for permission), then start the exe it registers |
//! | Windows portable zip | after exit, unzip over this folder (settings in `ArtCraftLauncherData` stay) |
//! | Linux AppImage | rename the new AppImage over this one (fine while running), start it |
//! | Linux `.deb` / `.rpm` | the system package manager, with the password prompt; then restart |
//! | Linux Flatpak | reinstall the bundle on the same installation (per-user, or system-wide through `pkexec`); then restart |
//!
//! Anything else (FreeBSD, a development build) gets the download link instead.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::PackageKind;
use crate::app::App;
use crate::platform::{Platform, pick_asset};
use crate::protocol::JobPhase;
use crate::release::Release;
use crate::version::{self, Version};
#[cfg(target_os = "macos")]
use crate::{InstallSource, Installed};

use crate::http::{Http, Stop};
use crate::install::{self, Job};
use crate::paths::Paths;
use crate::process;

/// How this running launcher was installed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Install {
    /// The `ArtCraft Launcher.app` bundle.
    MacApp(PathBuf),
    /// The per-user Windows Installer package; the exe.
    Msi(PathBuf),
    /// The Windows portable zip; the exe (with `portable.txt` beside it).
    Portable(PathBuf),
    /// The AppImage file.
    AppImage(PathBuf),
    /// A Debian or RPM package; the binary.
    Package(PackageKind, PathBuf),
    /// A Flatpak application.
    Flatpak,
    /// A standalone executable or unpacked tree.
    Standalone(PathBuf),
}

impl Install {
    fn kind(&self) -> PackageKind {
        match self {
            Self::MacApp(_) => PackageKind::Dmg,
            Self::Msi(_) => PackageKind::Msi,
            Self::Portable(_) => PackageKind::PortableZip,
            Self::AppImage(_) => PackageKind::AppImage,
            Self::Package(kind, _) => *kind,
            Self::Flatpak => PackageKind::Flatpak,
            Self::Standalone(_) => {
                #[cfg(windows)]
                {
                    PackageKind::PortableZip
                }
                #[cfg(not(windows))]
                {
                    PackageKind::TarGz
                }
            }
        }
    }
}

mod detect;
mod helper;
#[cfg(unix)]
mod relaunch;
mod replace;

#[cfg(target_os = "linux")]
use detect::{FlatpakLevel, flatpak_level};
#[cfg(windows)]
use helper::{Finish, PORTABLE_STAGING, msi_finish, windows_helper, windows_precheck};
#[cfg(unix)]
use relaunch::relaunch_after_exit;
#[cfg(target_os = "linux")]
use relaunch::relaunch_flatpak_after_exit;
#[cfg(target_os = "linux")]
use replace::replace_appimage;
#[cfg(unix)]
use replace::replace_standalone;
use replace::{mark_installed, unmark_unless_scheduled};

pub use detect::{DEV_VAR, PRETEND_VAR, current, current_version, is_self_updating, just_updated_to};
pub use helper::{FINISH_FLAG, finish, take_failure};

/// Downloads `release` for `platform`, puts it in place and starts the helper that relaunches
/// it once this process has exited. On success the caller must quit promptly. `last` is called
/// once the download is ready and returns when the app may replace itself and restart.
#[allow(clippy::too_many_arguments)]
pub fn update(
    app: &App,
    install: &Install,
    release: &Release,
    platform: Platform,
    http: &Http,
    paths: &Paths,
    stop: &Stop,
    phase: &mut dyn FnMut(JobPhase),
    last: &dyn Fn() -> Result<(), String>,
    running_version: Option<&str>,
) -> Result<(), String> {
    let cur_v = running_version.and_then(|v| current_version(app, v));
    if let (Some(rel_v), Some(cur_v)) = (release.version.as_ref(), cur_v.as_ref())
        && !version::offers_update(rel_v, cur_v)
    {
        // Compare the effective version: an explicit pretend version also exercises installation.
        // Without that opt-in, a build from source requires a strictly higher release.
        return Err(format!("{} {} is not newer than currently running version {}", app.name, release.version_label(), cur_v));
    }
    let (asset, _) = pick_asset(release, &[app.id], platform, &[install.kind()])
        .ok_or_else(|| format!("{} {} has no {} for {}", app.name, release.version_label(), install.kind().label(), platform.label()))?;
    let asset = asset.clone();
    crate::install::bind_artifact_version(app, release, &asset)?;
    let mut job = Job { http, paths, stop, phase, install_locations: &Default::default(), replace: false };
    let file = install::download(release, &asset, &mut job)?;
    #[cfg(windows)]
    windows_precheck(app, install, paths, &file)?;
    // The restart ends this process: nothing else may be under way.
    last()?;
    phase(JobPhase::Installing);
    // Written before the swap starts: the Windows helper runs alongside this process and clears
    // it when it fails, which a write after scheduling could undo.
    mark_installed(paths, release);
    let pid = std::process::id();
    let scheduled = (|| -> Result<(), String> {
        match install {
            #[cfg(target_os = "macos")]
            Install::MacApp(bundle) => {
                let existing = Installed { version: None, source: InstallSource::MacApp, location: bundle.display().to_string(), package: None };
                let placed = install::macos::install_dmg(app, &file, Some(&existing), None, false, paths, &install::macos::Expect::release(release));
                let _ = std::fs::remove_file(&file);
                let placed = placed?;
                relaunch_after_exit(app, pid, &[Path::new("/usr/bin/open").as_os_str(), placed.as_os_str()])
            }
            #[cfg(target_os = "linux")]
            Install::AppImage(path) => {
                replace_appimage(&file, path)?;
                let _ = std::fs::remove_file(&file);
                relaunch_after_exit(app, pid, &[path.as_os_str()])
            }
            #[cfg(target_os = "linux")]
            Install::Package(kind, exe) => {
                let result = install::linux::install_package(*kind, &file, None, stop, phase);
                let _ = std::fs::remove_file(&file);
                result?;
                relaunch_after_exit(app, pid, &[exe.as_os_str()])
            }
            #[cfg(target_os = "linux")]
            Install::Flatpak => {
                let system = match flatpak_level() {
                    Some(FlatpakLevel::System) => true,
                    Some(FlatpakLevel::User) => false,
                    // Re-read rather than trusting a stale guess: `--user` on a system copy
                    // would install a second one beside it.
                    None => return Err(format!("this copy of {} can't update itself; download the new version instead", app.name)),
                };
                let result = install::linux::update_flatpak_bundle(&file, &app.bundle_id(), system, stop, phase);
                let _ = std::fs::remove_file(&file);
                result?;
                relaunch_flatpak_after_exit(app)
            }
            #[cfg(windows)]
            Install::Msi(exe) => windows_helper(app, paths, pid, exe, &msi_finish(&file)?),
            #[cfg(windows)]
            Install::Portable(exe) => {
                exe.parent().ok_or_else(|| format!("{} isn't in a folder", app.name))?;
                windows_helper(app, paths, pid, exe, &Finish::Portable { zip: file, tmp: paths.cache.join(PORTABLE_STAGING) })
            }
            #[cfg(windows)]
            Install::Standalone(exe) => {
                exe.parent().ok_or_else(|| format!("{} isn't in a folder", app.name))?;
                windows_helper(app, paths, pid, exe, &Finish::Portable { zip: file, tmp: paths.cache.join(PORTABLE_STAGING) })
            }
            #[cfg(unix)]
            Install::Standalone(exe) => {
                let result = replace_standalone(app, paths, &file, exe);
                let _ = std::fs::remove_file(&file);
                result?;
                relaunch_after_exit(app, pid, &[exe.as_os_str()])
            }
            #[allow(unreachable_patterns)]
            _ => {
                let _ = (pid, &file);
                Err(format!("this copy of {} can't update itself; download the new version instead", app.name))
            }
        }
    })();
    unmark_unless_scheduled(paths, scheduled)
}

#[cfg(test)]
mod tests;
