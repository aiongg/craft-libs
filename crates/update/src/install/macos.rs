//! macOS installs: the signed `.app` from the release DMG.

use std::time::Duration;

use crate::version::Version;

use super::{App, InstallSource, Installed, Path, PathBuf, Paths, Release, process, swap_into_place};

/// `/Applications` when we can write there, else `~/Applications`.
fn applications_dir(existing: Option<&Installed>, destination: Option<&Path>) -> Result<PathBuf, String> {
    if let Some(parent) = existing.filter(|i| i.source == InstallSource::MacApp).and_then(|i| Path::new(&i.location).parent().map(Path::to_path_buf))
    {
        return Ok(parent);
    }
    if let Some(dir) = destination {
        return Ok(dir.to_path_buf());
    }
    let system = PathBuf::from("/Applications");
    let probe = system.join(".artcraft-launcher-write-test");
    if std::fs::write(&probe, b"").is_ok() {
        let _ = std::fs::remove_file(&probe);
        return Ok(system);
    }
    let user = crate::folders::home_dir().ok_or("no home folder")?.join("Applications");
    std::fs::create_dir_all(&user).map_err(|e| format!("couldn't create {} ({e})", user.display()))?;
    Ok(user)
}

/// The Apple Developer team that signs every Craft App and ArtCraft Launcher (Learning
/// Machines LLC): the macOS side of the Windows publisher check.
pub const TEAM_ID: &str = "DJ6XS33FX8";

/// What a new copy must prove before it replaces anything. The checksum shows the download is
/// the file the release lists; these show the app inside is the one the release promises.
pub struct Expect<'a> {
    /// The team whose Developer ID signature the bundle must carry. `None` only in tests,
    /// which build unsigned bundles.
    pub team: Option<&'a str>,
    /// The release's version, which the bundle must report. `None` when the tag isn't one.
    pub version: Option<&'a Version>,
}

impl<'a> Expect<'a> {
    pub fn release(release: &'a Release) -> Self {
        Self { team: Some(TEAM_ID), version: release.version.as_ref() }
    }
}

/// Held while this process asks diskimages-helper for something (an attach here; the tests'
/// `hdiutil create` too), since it turns concurrent requests away.
pub(crate) static DISK_IMAGES: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Apple's tools, by full path: a launcher started from Finder or at login gets a minimal
/// `PATH`, and nothing earlier on it should stand in for these.
const HDIUTIL: &str = "/usr/bin/hdiutil";
const DITTO: &str = "/usr/bin/ditto";
const CODESIGN: &str = "/usr/bin/codesign";

pub fn install_dmg(
    app: &App,
    dmg: &Path,
    existing: Option<&Installed>,
    destination: Option<&Path>,
    replace: bool,
    paths: &Paths,
    expect: &Expect<'_>,
) -> Result<PathBuf, String> {
    // A new, empty mount point for every install. A fixed one can still hold an image from an
    // attach that was interrupted or timed out, and hdiutil won't mount another over it.
    let attempt = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or_default();
    let mount = paths.cache.join(format!("mount-{}-{}-{attempt}", app.id, std::process::id()));
    std::fs::create_dir_all(&mount).map_err(|e| format!("couldn't create a folder to open the disk image in ({e})"))?;
    let (Some(dmg_s), Some(mount_s)) = (dmg.to_str(), mount.to_str()) else {
        let _ = std::fs::remove_dir(&mount);
        return Err("unusual characters in the cache folder's path".into());
    };
    // diskimages-helper refuses an attach while it is busy with another image ("Resource
    // temporarily unavailable"): attach one image at a time (several apps may be updating at
    // once), and wait a little and try again when something outside the launcher holds it.
    // The download's SHA-256 is already checked, so hdiutil's own checksum pass (-noverify)
    // would only add minutes on a large image.
    let attached = {
        let _one_at_a_time = DISK_IMAGES.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let attach = || {
            process::run(
                HDIUTIL,
                &["attach", "-nobrowse", "-readonly", "-noautoopen", "-noverify", "-mountpoint", mount_s, dmg_s],
                Duration::from_secs(300),
            )
        };
        // diskimages-helper can stay busy for a while on a shared runner; a minute of retries is still quick.
        let mut attached = attach();
        for wait in [1, 2, 4, 8, 16, 32] {
            if attached.is_ok() {
                break;
            }
            let _ = detach(&mount);
            std::thread::sleep(Duration::from_secs(wait));
            attached = attach();
        }
        attached
    };
    if let Err(reason) = attached {
        let _ = std::fs::remove_dir(&mount);
        return Err(format!("couldn't open the disk image: {reason}"));
    }
    let result = copy_app(app, &mount, existing, destination, replace, expect);
    let _ = detach(&mount);
    let _ = std::fs::remove_dir(&mount);
    result
}

fn copy_app(
    app: &App,
    mount: &Path,
    existing: Option<&Installed>,
    destination: Option<&Path>,
    replace: bool,
    expect: &Expect<'_>,
) -> Result<PathBuf, String> {
    let bundle = format!("{}.app", app.name);
    let src = mount.join(&bundle);
    crate::detect::macos::read_bundle(app, &src).ok_or("the disk image doesn't contain the app")?;
    let dir = applications_dir(existing, destination)?;
    let dest = existing.filter(|i| i.source == InstallSource::MacApp).map_or_else(|| dir.join(&bundle), |i| PathBuf::from(&i.location));
    if !replace
        && std::fs::symlink_metadata(&dest).is_ok()
        && (std::fs::symlink_metadata(&dest).is_ok_and(|m| m.file_type().is_symlink()) || crate::detect::macos::read_bundle(app, &dest).is_none())
    {
        return Err(format!("{} already exists and wasn't installed by ArtCraft Launcher", dest.display()));
    }
    let staging = super::staging_dir(&dir)?;
    let staged = staging.path().join(&bundle);
    let (Some(src_s), Some(staged_s)) = (src.to_str(), staged.to_str()) else { return Err("unusual characters in a path".into()) };
    // ditto keeps the code signature and symlinks intact. The disk image's volume adds Finder
    // metadata (com.apple.FinderInfo) to the files; leave it and the extended attributes
    // behind, or `codesign --verify --strict` rejects the bundle as carrying "detritus". A
    // bundle's signature lives in its files, not in extended attributes.
    process::run(DITTO, &["--norsrc", "--noextattr", "--noacl", src_s, staged_s], Duration::from_secs(600))
        .map_err(|reason| format!("couldn't copy the app: {reason}"))?;
    let checked = crate::detect::macos::read_bundle(app, &staged)
        .ok_or_else(|| "the copied app is incomplete".to_string())
        .and_then(|found| check(&staged, &found, expect));
    checked?;
    swap_into_place(&staged, &dest)?;
    Ok(dest)
}

/// Checks a copied bundle against what its release promises, before it replaces anything.
fn check(bundle: &Path, found: &Installed, expect: &Expect<'_>) -> Result<(), String> {
    if let Some(team) = expect.team
        && !signed_by(bundle, team)
    {
        return Err("the app isn't signed by its developer, so it wasn't installed".into());
    }
    if let Some(want) = expect.version
        && !found.version.as_ref().is_some_and(|v| same_release(v, want))
    {
        return Err(format!("the app inside isn't version {want}, so it wasn't installed"));
    }
    Ok(())
}

/// Whether `bundle` has a valid Developer ID signature from `team`, nested code included. The
/// requirement pins the signer, so a valid ad-hoc or other developer's signature fails it.
/// Unlike a Gatekeeper assessment it needs no network.
pub fn signed_by(bundle: &Path, team: &str) -> bool {
    let Some(path) = bundle.to_str() else { return false };
    if team.is_empty() || !team.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return false;
    }
    let requirement = format!("=anchor apple generic and certificate leaf[subject.OU] = \"{team}\"");
    process::capture(CODESIGN, &["--verify", "--deep", "-R", &requirement, path], Duration::from_secs(120)).is_some()
}

/// Whether a bundle's version is the release's. A bundle that reports only the numbers
/// (`CFBundleShortVersionString`) matches a pre-release with the same numbers.
fn same_release(found: &Version, release: &Version) -> bool {
    (found.major, found.minor, found.patch) == (release.major, release.minor, release.patch) && (found.pre.is_empty() || found.pre == release.pre)
}

fn detach(mount: &Path) -> Option<String> {
    let m = mount.to_str()?;
    if !mount.join(".").exists() {
        return None;
    }
    process::capture(HDIUTIL, &["detach", m, "-quiet"], Duration::from_secs(60))
        .or_else(|| process::capture(HDIUTIL, &["detach", m, "-force", "-quiet"], Duration::from_secs(60)))
}

/// Moves an app to the user's Trash (so it can be put back).
pub fn move_to_trash(path: &Path) -> Result<(), String> {
    let trash = crate::folders::home_dir().ok_or("no home folder")?.join(".Trash");
    let name = path.file_name().ok_or("not an app")?;
    let mut dest = trash.join(name);
    if dest.exists() {
        dest = trash.join(format!("{} {}", name.to_string_lossy(), crate::now()));
    }
    std::fs::rename(path, &dest).map_err(|e| format!("couldn't move {} to Trash ({e}). Remove it in Finder instead.", path.display()))
}
