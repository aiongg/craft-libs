//! Tests for installing, updating and removing apps.

use super::*;

#[test]
fn swap_keeps_the_old_copy_until_the_new_one_is_in_place() {
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("app");
    let staged = dir.path().join("app.new");
    std::fs::create_dir(&dest).unwrap();
    std::fs::write(dest.join("v"), "old").unwrap();
    std::fs::create_dir(&staged).unwrap();
    std::fs::write(staged.join("v"), "new").unwrap();
    swap_into_place(&staged, &dest).unwrap();
    assert_eq!(std::fs::read_to_string(dest.join("v")).unwrap(), "new");
    assert!(!staged.exists() && !dir.path().join("app.old").exists());
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    // A missing staged copy leaves the installed one alone.
    assert!(swap_into_place(&staged, &dest).is_err());
    assert_eq!(std::fs::read_to_string(dest.join("v")).unwrap(), "new");
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn a_release_without_a_checksum_is_refused_before_anything_is_fetched() {
    use crate::http::tests::{Reply, body, serve};
    let base = App::by_id("wordcraft").unwrap();
    let wordcraft = App { icon_png: Some(b"\x89PNG\r\n\x1a\nfake-png-bytes"), ..*base };
    let app = &wordcraft;
    let (body, _) = body();
    let (url, requests) = serve(body.clone(), vec![Reply::Full]);
    // Neither a per-file digest nor a SHA256SUMS.txt in the release.
    let asset = Asset { name: "wordcraft-0.3.1-test.pkg".into(), size: body.len() as u64, url, sha256: None };
    let release = Release { assets: vec![asset.clone()], ..Release::default() };
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::under(dir.path());
    let http = Http::new(crate::TEST_AGENT).unwrap();
    let kind = if cfg!(windows) { PackageKind::Dmg } else { PackageKind::Msi };
    let mut phases = Vec::new();
    let mut phase = |p: JobPhase| phases.push(p);
    let mut job = Job {
        http: &http,
        paths: &paths,
        stop: &Stop::default(),
        phase: &mut phase,
        install_locations: &InstallLocations::default(),
        replace: false,
    };
    let err = install(app, &release, &asset, kind, None, None, &mut job).unwrap_err();
    assert!(err.contains("publishes no checksum"), "{err}");
    assert!(!download_path(&paths, &asset).exists());
    assert!(requests.lock().unwrap().is_empty(), "nothing was fetched");
    assert!(!phases.contains(&JobPhase::Installing));
}

#[cfg(unix)]
fn tarball(path: &Path, entries: &[&str]) {
    let f = std::fs::File::create(path).unwrap();
    let mut b = tar::Builder::new(flate2::write::GzEncoder::new(f, flate2::Compression::fast()));
    for name in entries {
        let mut h = tar::Header::new_gnu();
        h.set_size(2);
        h.set_mode(0o755);
        h.set_cksum();
        b.append_data(&mut h, name, &b"hi"[..]).unwrap();
    }
    b.into_inner().unwrap().finish().unwrap();
}

#[cfg(target_os = "freebsd")]
#[test]
fn remove_files_refuses_escaping_paths() {
    assert!(freebsd::remove_files(&[PathBuf::from("/etc/shadow")], false).is_err());
    if let Some(home) = crate::folders::home_dir() {
        let prefix = home.join(".local");
        assert!(freebsd::remove_files(&[prefix.join("../escaped")], false).is_err());
    }
}

#[cfg(target_os = "linux")]
#[test]
fn a_tarball_binary_that_is_a_symlink_out_of_the_tree_is_refused() {
    use std::os::unix::fs::PermissionsExt;
    let base = App::by_id("wordcraft").unwrap();
    let wordcraft = App { icon_png: Some(b"\x89PNG\r\n\x1a\nfake-png-bytes"), ..*base };
    let app = &wordcraft;
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::under(dir.path());

    // A working install the rejected update must leave untouched.
    let good = dir.path().join("wordcraft-0.3.0-linux-x86_64.tar.gz");
    tarball(&good, &["wordcraft-0.3.0-linux-x86_64/bin/wordcraft"]);
    let bin = linux::install_tree(app, &good, &paths).unwrap();
    assert_eq!(std::fs::read_to_string(&bin).unwrap(), "hi");

    // A file outside the install tree that the archive tries to chmod through a symlink.
    let outside = dir.path().join("victim");
    std::fs::write(&outside, b"secret").unwrap();
    std::fs::set_permissions(&outside, std::fs::Permissions::from_mode(0o600)).unwrap();

    // A tarball whose bin/wordcraft is a symlink to that external file.
    let file = dir.path().join("evil.tar.gz");
    let f = std::fs::File::create(&file).unwrap();
    let mut b = tar::Builder::new(flate2::write::GzEncoder::new(f, flate2::Compression::fast()));
    let mut link = tar::Header::new_gnu();
    link.set_entry_type(tar::EntryType::Symlink);
    link.set_size(0);
    link.set_mode(0o777);
    link.set_cksum();
    b.append_link(&mut link, "wordcraft-0.3.0-linux-x86_64/bin/wordcraft", &outside).unwrap();
    // A second real file so the top folder is detected.
    let mut reg = tar::Header::new_gnu();
    reg.set_size(2);
    reg.set_mode(0o644);
    reg.set_cksum();
    b.append_data(&mut reg, "wordcraft-0.3.0-linux-x86_64/share/applications/ai.storyteller.wordcraft.desktop", &b"hi"[..]).unwrap();
    b.into_inner().unwrap().finish().unwrap();

    let result = linux::install_tree(app, &file, &paths);
    assert!(result.is_err(), "a symlink binary must be rejected: {result:?}");
    assert!(result.unwrap_err().contains("symlink"));
    // The external file keeps its original mode: the chmod never reached it.
    let mode = std::fs::metadata(&outside).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "the external file's permissions must be untouched");
    // The rejected update left the previously installed, working binary fully intact.
    assert!(bin.is_file(), "the rejected update must not remove the working binary");
    assert_eq!(std::fs::read_to_string(&bin).unwrap(), "hi");
}

#[cfg(target_os = "linux")]
const FLATPAK_ID: &str = "ai.storyteller.soundcraft";
#[cfg(target_os = "linux")]
const ALREADY_THERE: &str = "ai.storyteller.soundcraft already installed";
#[cfg(target_os = "linux")]
const INSTALL_CALL: &str = "install --user -y --noninteractive --reinstall --bundle /x.flatpak";

/// Runs `install` against a stand-in `flatpak` that logs its calls and fails its first `failures`
/// installs with `why`. Returns the result and the calls.
#[cfg(target_os = "linux")]
fn with_fake_flatpak(
    flatpak_version: &str,
    failures: u32,
    why: &str,
    uninstall_exit: u32,
    install: impl FnOnce(&linux::Root<'_>) -> Result<(), String>,
) -> (Result<(), String>, Vec<String>) {
    let dir = tempfile::tempdir().unwrap();
    let (script, calls) = (dir.path().join("flatpak"), dir.path().join("calls"));
    let log = calls.display();
    std::fs::write(
        &script,
        format!(
            r#"echo "$*" >> "{log}"
case "$1" in
--version) echo "Flatpak {flatpak_version}" ;;
uninstall) exit {uninstall_exit} ;;
install)
[ "$(grep -c '^install' "{log}")" -gt {failures} ] && exit 0
echo "error: Failed to install bundle: {why}" >&2
exit 1 ;;
esac
"#
        ),
    )
    .unwrap();
    let flatpak = || {
        let mut cmd = std::process::Command::new("/bin/sh");
        cmd.arg(&script);
        cmd
    };
    let queue = crate::queue::Queue::new();
    let root = linux::Root { queue: &queue, run: &|_| Ok(()), flatpak: &flatpak };
    let result = install(&root);
    (result, std::fs::read_to_string(calls).unwrap().lines().map(str::to_string).collect())
}

#[cfg(target_os = "linux")]
fn update_bundle(root: &linux::Root<'_>, system: bool) -> Result<(), String> {
    linux::update_flatpak_bundle_via(root, Path::new("/x.flatpak"), FLATPAK_ID, system, &Stop::default(), &mut |_| {})
}

#[cfg(target_os = "linux")]
#[test]
fn a_self_update_never_uninstalls_the_app_it_runs_from() {
    // The update landed earlier and never restarted, so only the restart is left.
    let (result, calls) = with_fake_flatpak("1.14.6", 1, ALREADY_THERE, 0, |root| update_bundle(root, false));
    assert_eq!((result, calls), (Ok(()), vec![INSTALL_CALL.to_string()]));

    let (result, calls) = with_fake_flatpak("1.14.6", 0, ALREADY_THERE, 0, |root| update_bundle(root, false));
    assert_eq!((result, calls), (Ok(()), vec![INSTALL_CALL.to_string()]));

    // Anything else is still a failure: a broken bundle, or one holding another app.
    for why in ["Invalid checksum", "ai.storyteller.photocraft already installed"] {
        let (result, calls) = with_fake_flatpak("1.14.6", 1, why, 0, |root| update_bundle(root, false));
        assert_eq!(result, Err(format!("Flatpak couldn't install the bundle: Failed to install bundle: {why}")));
        assert_eq!(calls, [INSTALL_CALL]);
    }
}

#[test]
fn install_rejects_an_artifact_whose_version_is_not_the_advertised_one() {
    // The release advertises 99.0.0, but the selected artifact is an older official build
    // (filename and URL tag say 0.4.0). The guard must refuse before any download.
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::under(dir.path());
    let http = Http::new(crate::TEST_AGENT).unwrap();
    let phase = &mut |_| ();
    let locations = InstallLocations::default();
    let app = App::by_id("photocraft").unwrap();
    let release = Release { version: version::parse("99.0.0"), tag: "v99.0.0".into(), ..Release::default() };
    let asset = Asset {
        name: "photocraft-0.4.0-linux-x86_64.AppImage".into(),
        url: "https://github.com/storytold/photocraft/releases/download/v0.4.0/photocraft-0.4.0-linux-x86_64.AppImage".into(),
        ..Asset::default()
    };
    let stop = Stop::default();
    let mut job = Job { http: &http, paths: &paths, stop: &stop, phase, install_locations: &locations, replace: false };
    let result = install_at(app, &release, &asset, PackageKind::AppImage, None, None, &mut job, None);
    assert!(result.is_err(), "mismatched artifact version must be rejected");
    assert!(result.unwrap_err().contains("does not match the advertised version"));

    // A matching artifact passes the guard (it fails later for unrelated reasons, not the bind).
    let good = Asset {
        name: "photocraft-99.0.0-linux-x86_64.AppImage".into(),
        url: "https://github.com/storytold/photocraft/releases/download/v99.0.0/photocraft-99.0.0-linux-x86_64.AppImage".into(),
        ..Asset::default()
    };
    assert!(bind_artifact_version(app, &release, &good).is_ok());

    // URL filename mismatch is rejected (CWE-494).
    let bad_url_name = Asset {
        name: "photocraft-99.0.0-linux-x86_64.AppImage".into(),
        url: "https://github.com/storytold/photocraft/releases/download/v99.0.0/other.AppImage".into(),
        ..Asset::default()
    };
    assert!(bind_artifact_version(app, &release, &bad_url_name).is_err());

    // URL tag mismatch is rejected.
    let bad_tag = Asset {
        name: "photocraft-99.0.0-linux-x86_64.AppImage".into(),
        url: "https://github.com/storytold/photocraft/releases/download/unparsed_tag/photocraft-99.0.0-linux-x86_64.AppImage".into(),
        ..Asset::default()
    };
    assert!(bind_artifact_version(app, &release, &bad_tag).is_err());
}

#[cfg(target_os = "macos")]
mod macos_checks {
    use crate::version;

    use super::macos::{TEAM_ID, signed_by};
    use super::*;

    /// A minimal `PhotoCraft.app` at `dir`, reporting `version`.
    fn bundle(dir: &Path, version: &str) -> PathBuf {
        let app = dir.join("PhotoCraft.app");
        let contents = app.join("Contents");
        std::fs::create_dir_all(contents.join("MacOS")).unwrap();
        std::fs::write(
            contents.join("Info.plist"),
            format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?><plist version=\"1.0\"><dict>\
                 <key>CFBundleIdentifier</key><string>ai.storyteller.photocraft</string>\
                 <key>CFBundleExecutable</key><string>PhotoCraft</string>\
                 <key>CFBundleShortVersionString</key><string>{version}</string></dict></plist>"
            ),
        )
        .unwrap();
        std::fs::write(contents.join("MacOS/PhotoCraft"), version).unwrap();
        app
    }

    /// A release-style disk image holding a `PhotoCraft.app` that reports `version`. The
    /// disk-image helper turns requests away while it is busy (`attach` retries for the same
    /// reason), so `hdiutil create` is tried a few times, and what it said is kept for the
    /// failure message.
    fn dmg(dir: &Path, version: &str) -> PathBuf {
        let src = dir.join(format!("src-{version}"));
        bundle(&src, version);
        let dmg = dir.join(format!("photocraft-{version}-macos-universal.dmg"));
        let _one_at_a_time = super::macos::DISK_IMAGES.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut said = String::new();
        for attempt in 0..5u64 {
            if attempt > 0 {
                std::thread::sleep(std::time::Duration::from_millis(400 * attempt));
                let _ = std::fs::remove_file(&dmg);
            }
            let output = std::process::Command::new("hdiutil")
                .args(["create", "-quiet", "-format", "UDZO", "-volname", "PhotoCraft", "-srcfolder"])
                .arg(&src)
                .arg(&dmg)
                .output()
                .unwrap();
            if output.status.success() {
                return dmg;
            }
            said = String::from_utf8_lossy(&output.stderr).trim().to_string();
        }
        panic!("hdiutil create failed after 5 tries: {said}");
    }

    #[test]
    fn only_the_teams_signature_passes() {
        let dir = tempfile::tempdir().unwrap();
        let app = bundle(dir.path(), "0.6.0");
        assert!(!signed_by(&app, TEAM_ID), "an unsigned bundle passed");

        // A valid signature, but not from the team.
        let ok = std::process::Command::new("codesign").args(["--force", "--sign", "-"]).arg(&app).status().unwrap().success();
        assert!(ok, "codesign failed");
        assert!(!signed_by(&app, TEAM_ID), "an ad-hoc signature passed");

        // A team id can't smuggle anything into the requirement.
        assert!(!signed_by(&app, "X\" or anchor apple generic or \"Y"));
        assert!(!signed_by(&app, ""));
    }
}
