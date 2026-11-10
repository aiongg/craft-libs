//! Tests for installing, updating and removing apps.

use super::*;

#[cfg(target_os = "linux")]
#[test]
fn a_system_flatpak_bundle_is_reinstalled_with_pkexec_on_that_installation() {
    assert_eq!(super::linux::flatpak_bundle_flags(false), ["install", "--user", "-y", "--noninteractive", "--reinstall", "--bundle"]);
    let system = super::linux::flatpak_bundle_flags(true);
    assert_eq!(system, ["install", "--system", "-y", "--noninteractive", "--reinstall", "--bundle"]);
    assert!(!system.contains(&"--user"));
    assert_eq!(super::linux::system_bundle_prefix(true), ["flatpak-spawn", "--host", "pkexec", "/usr/bin/flatpak"]);
    assert_eq!(super::linux::system_bundle_prefix(false), ["pkexec", "/usr/bin/flatpak"]);
}

#[cfg(target_os = "linux")]
#[test]
fn a_dismissed_system_bundle_password_cancels_and_a_flatpak_error_is_kept() {
    use std::os::unix::process::ExitStatusExt;
    use std::process::Output;
    let dismissed = Output { status: std::process::ExitStatus::from_raw(126 << 8), stdout: Vec::new(), stderr: Vec::new() };
    assert_eq!(super::linux::flatpak_bundle_error(&dismissed).unwrap_err(), "cancelled");
    let failed = Output { status: std::process::ExitStatus::from_raw(1 << 8), stdout: Vec::new(), stderr: b"error: something went wrong\n".to_vec() };
    assert_eq!(super::linux::flatpak_bundle_error(&failed).unwrap_err(), "Flatpak couldn't install the bundle: something went wrong");
    let ok = Output { status: std::process::ExitStatus::from_raw(0), stdout: Vec::new(), stderr: Vec::new() };
    assert!(super::linux::flatpak_bundle_error(&ok).is_ok());
}

#[cfg(unix)]
#[test]
fn msi_install_uses_the_package_target_instead_of_a_symlinked_cache() {
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join("persistent cache");
    std::fs::create_dir(&cache).unwrap();
    let file = cache.join("PhotoCraft.msi");
    std::fs::write(&file, b"test package").unwrap();
    let link = dir.path().join("current cache");
    std::os::unix::fs::symlink(&cache, &link).unwrap();
    let command = msi_command(Path::new("msiexec.exe"), &link.join("PhotoCraft.msi")).unwrap();
    let args: Vec<_> = command.get_args().collect();
    assert_eq!(
        args,
        [
            std::ffi::OsStr::new("/i"),
            file.canonicalize().unwrap().as_os_str(),
            std::ffi::OsStr::new("/passive"),
            std::ffi::OsStr::new("/norestart"),
            std::ffi::OsStr::new("/l*v"),
            file.canonicalize().unwrap().with_extension("log").as_os_str()
        ]
    );
}

#[test]
fn a_self_update_keeps_the_installed_copys_context_after_the_package_options() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("GridCraft.msi");
    std::fs::write(&file, b"test package").unwrap();
    let resolved = msi_path(&file).unwrap();
    for context in [&["ALLUSERS=1"][..], &["ALLUSERS=2", "MSIINSTALLPERUSER=1"], &[]] {
        let command = reinstall_command(Path::new("msiexec.exe"), &file, context).unwrap();
        let args: Vec<_> = command.get_args().map(|a| a.to_string_lossy().into_owned()).collect();
        let mut expected: Vec<String> = ["/i", &resolved.to_string_lossy(), "/passive", "/norestart", "/l*v"].map(String::from).to_vec();
        expected.push(resolved.with_extension("log").to_string_lossy().into_owned());
        expected.extend(context.iter().map(|p| p.to_string()));
        assert_eq!(args, expected);
    }
}

#[test]
fn an_unavailable_msi_is_refused_before_starting_the_installer() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing.msi");
    let error = msi_command(Path::new("msiexec.exe"), &missing).unwrap_err();
    assert!(error.contains("couldn't resolve the Windows Installer package"), "{error}");
}

#[cfg(windows)]
#[test]
fn a_short_local_msi_path_keeps_ordinary_windows_syntax() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("package.msi");
    std::fs::write(&file, b"test package").unwrap();
    let resolved = msi_path(&file).unwrap();
    assert!(!resolved.as_os_str().to_string_lossy().starts_with(r"\\?\"), "{resolved:?}");
    assert_eq!(std::fs::read(resolved).unwrap(), b"test package");
}

#[test]
fn selected_folders_must_exist_and_leave_no_probe_files() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(selected_folder(dir.path().to_str().unwrap(), true).unwrap(), dir.path());
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    #[cfg(unix)]
    {
        let quoted = dir.path().join("Craft \"Apps\"");
        std::fs::create_dir(&quoted).unwrap();
        assert_eq!(selected_folder(quoted.to_str().unwrap(), true).unwrap(), quoted);
    }
    let missing = dir.path().join("unavailable/Apps");
    assert!(selected_folder(missing.to_str().unwrap(), true).is_err());
    assert!(!missing.exists());
    assert!(selected_folder("relative/folder", true).is_err());
    assert!(selected_folder("bad\nfolder", true).is_err());
    let app = App::by_id("photocraft").unwrap();
    let locations = InstallLocations { root: Some(dir.path().display().to_string()), ..Default::default() };
    assert_eq!(destination(app, PackageKind::Msi, None, None, &locations).unwrap(), Some(dir.path().join("PhotoCraft")));
    assert_eq!(destination(app, PackageKind::PortableZip, None, None, &locations).unwrap(), Some(dir.path().to_path_buf()));
    let old = record("0.5.0", PackageKind::PortableZip, dir.path().join("PhotoCraft/photocraft.exe"), vec![], false);
    let unavailable = InstallLocations { root: Some(dir.path().join("missing").display().to_string()), ..Default::default() };
    assert_eq!(destination(app, PackageKind::PortableZip, None, Some(&old), &unavailable).unwrap(), None);
    let msi = Installed {
        version: None,
        source: InstallSource::Msi,
        location: dir.path().join("MSI/PhotoCraft/photocraft.exe").display().to_string(),
        package: None,
    };
    assert_eq!(destination(app, PackageKind::Msi, Some(&msi), Some(&old), &unavailable).unwrap(), Some(dir.path().join("MSI/PhotoCraft")));
    assert!(destination(app, PackageKind::TarGz, None, None, &locations).unwrap_err().contains("aren't supported"));
}

#[test]
fn updates_stay_beside_the_launchers_copy_and_new_installs_use_the_chosen_folder() {
    let dir = tempfile::tempdir().unwrap();
    let app = App::by_id("photocraft").unwrap();
    let path = dir.path().join("old.AppImage");
    let rec = record("0.5.0", PackageKind::AppImage, path.clone(), vec![], false);
    let launcher = Installed { version: None, source: InstallSource::Launcher, location: path.display().to_string(), package: None };
    let users = Installed { version: None, source: InstallSource::AppImage, location: "/home/me/AppImages/p.AppImage".into(), package: None };
    let chosen = dir.path().join("New apps");
    std::fs::create_dir(&chosen).unwrap();
    let mut locations = InstallLocations { root: Some(chosen.display().to_string()), ..Default::default() };
    assert_eq!(destination(app, PackageKind::AppImage, Some(&launcher), Some(&rec), &locations).unwrap(), Some(dir.path().into()));
    assert_eq!(destination(app, PackageKind::AppImage, Some(&users), None, &locations).unwrap(), None);
    if !process::in_sandbox() {
        assert_eq!(destination(app, PackageKind::AppImage, None, None, &locations).unwrap(), Some(chosen.clone()));
        locations.per_app.insert(app.id.into(), dir.path().display().to_string());
        assert_eq!(destination(app, PackageKind::Dmg, None, None, &locations).unwrap(), Some(dir.path().into()));
    }
}

#[test]
fn msi_folder_assignment_preserves_spaces_unicode_and_root_separators() {
    for (value, argument) in [
        (r"D:\", r#"INSTALLFOLDER="D:\""#),
        (r"D:\Craft Apps\PhotoCraft", r#"INSTALLFOLDER="D:\Craft Apps\PhotoCraft""#),
        (r"\\server\apps\绘画", r#"INSTALLFOLDER="\\server\apps\绘画""#),
        (r"D:\Craft Apps\", r#"INSTALLFOLDER="D:\Craft Apps\""#),
    ] {
        assert_eq!(msi_folder_argument(Path::new(value)).unwrap(), argument);
    }
    assert!(msi_folder_argument(Path::new("D:\\bad\" TRANSFORMS=evil")).is_err());
    assert!(msi_folder_argument(Path::new("D:\\bad\nfolder")).is_err());
}

#[cfg(target_os = "linux")]
#[test]
fn appimage_updates_replace_only_the_recorded_file_in_the_same_folder() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("Craft Apps");
    std::fs::create_dir(&folder).unwrap();
    let download = dir.path().join("download");
    std::fs::write(&download, "first").unwrap();
    let unrelated = folder.join("photocraft-unrelated.AppImage");
    std::fs::write(&unrelated, "user copy").unwrap();
    let (old, removed) = linux::install_appimage(&download, "photocraft-0.5.0.AppImage", Some(&folder), None, false).unwrap();
    assert!(removed.is_empty());
    assert_eq!(std::fs::read(&old).unwrap(), b"first");
    let rec = record("0.5.0", PackageKind::AppImage, old.clone(), vec![], false);
    std::fs::write(&download, "updated").unwrap();
    let (new, removed) = linux::install_appimage(&download, "photocraft-0.6.0.AppImage", Some(&folder), Some(&rec), false).unwrap();
    assert_eq!(new, folder.join("photocraft-0.6.0.AppImage"));
    assert_eq!(std::fs::read(&new).unwrap(), b"updated");
    assert_eq!(removed.as_slice(), std::slice::from_ref(&old));
    assert!(!old.exists());
    assert_eq!(std::fs::read(&unrelated).unwrap(), b"user copy");
    assert!(linux::install_appimage(&download, "photocraft-unrelated.AppImage", Some(&folder), Some(&rec), false).is_err());
    assert_eq!(std::fs::read(&unrelated).unwrap(), b"user copy");
    let new_rec = record("0.6.0", PackageKind::AppImage, new.clone(), vec![], false);
    assert!(linux::install_appimage(&download, "photocraft-0.6.0.AppImage", Some(&folder), Some(&new_rec), false).is_ok());
    assert_eq!(std::fs::read_dir(&folder).unwrap().count(), 2);
    let link = folder.join("linked.AppImage");
    std::os::unix::fs::symlink(&unrelated, &link).unwrap();
    let link_rec = record("0.6.0", PackageKind::AppImage, link, vec![], false);
    assert!(linux::install_appimage(&download, "linked.AppImage", Some(&folder), Some(&link_rec), false).is_err());
    assert_eq!(std::fs::read(&unrelated).unwrap(), b"user copy");
}

#[cfg(target_os = "linux")]
#[test]
fn an_appimage_writes_over_a_users_file_only_when_asked_to() {
    let dir = tempfile::tempdir().unwrap();
    let download = dir.path().join("download");
    std::fs::write(&download, "new").unwrap();
    let folder = dir.path().join("Apps");
    std::fs::create_dir(&folder).unwrap();
    let theirs = folder.join("photocraft.AppImage");
    let personal = dir.path().join("personal.txt");
    std::fs::write(&theirs, "user copy").unwrap();
    std::fs::write(&personal, "user file").unwrap();
    std::os::unix::fs::symlink(&personal, folder.join(".photocraft.AppImage.partial")).unwrap();
    assert!(linux::install_appimage(&download, "photocraft.AppImage", Some(&folder), None, false).is_err());
    assert_eq!(std::fs::read(&theirs).unwrap(), b"user copy");
    linux::install_appimage(&download, "photocraft.AppImage", Some(&folder), None, true).unwrap();
    assert_eq!(std::fs::read(&theirs).unwrap(), b"new");
    assert_eq!(std::fs::read(&personal).unwrap(), b"user file");
    assert_eq!(std::fs::read_dir(&folder).unwrap().count(), 2, "staging was left behind");
}

#[test]
fn a_new_install_reports_what_it_would_write_over() {
    let dir = tempfile::tempdir().unwrap();
    let app = App::by_id("photocraft").unwrap();
    let paths = Paths::under(dir.path());
    let locations = InstallLocations { root: Some(dir.path().display().to_string()), ..Default::default() };
    let asset = Asset { name: "photocraft.zip".into(), ..Default::default() };
    let occupied = |previous| occupied(app, &asset, PackageKind::PortableZip, None, previous, &locations, &paths).unwrap();
    assert_eq!(occupied(None), None);
    std::fs::create_dir(dir.path().join("photocraft")).unwrap();
    assert_eq!(occupied(None), Some(dir.path().join("photocraft")));
    let old = record("0.5.0", PackageKind::PortableZip, dir.path().join("photocraft/photocraft.exe"), vec![], false);
    assert_eq!(occupied(Some(&old)), None);
}

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
fn swap_preserves_an_unrelated_backup_file() {
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("app");
    let staged = dir.path().join("app.new");
    let unrelated = dest.with_extension("old");
    std::fs::write(&dest, b"installed app").unwrap();
    std::fs::write(&staged, b"updated app").unwrap();
    std::fs::write(&unrelated, b"user backup").unwrap();
    swap_into_place(&staged, &dest).unwrap();
    assert_eq!(std::fs::read(&unrelated).unwrap(), b"user backup");
    assert_eq!(std::fs::read(&dest).unwrap(), b"updated app");
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
}

#[test]
fn swap_preserves_an_unrelated_backup_directory() {
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("app");
    let staged = dir.path().join("app.new");
    let unrelated = dest.with_extension("old");
    std::fs::create_dir(&dest).unwrap();
    std::fs::write(dest.join("v"), b"installed app").unwrap();
    std::fs::create_dir(&staged).unwrap();
    std::fs::write(staged.join("v"), b"updated app").unwrap();
    std::fs::create_dir(&unrelated).unwrap();
    std::fs::write(unrelated.join("keep.txt"), b"user backup").unwrap();
    swap_into_place(&staged, &dest).unwrap();
    assert_eq!(std::fs::read(unrelated.join("keep.txt")).unwrap(), b"user backup");
    assert_eq!(std::fs::read(dest.join("v")).unwrap(), b"updated app");
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
}

#[cfg(unix)]
#[test]
fn swap_preserves_a_dangling_backup_symlink() {
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("app");
    let staged = dir.path().join("app.new");
    let unrelated = dest.with_extension("old");
    let target = dir.path().join("missing");
    std::fs::write(&dest, b"installed app").unwrap();
    std::fs::write(&staged, b"updated app").unwrap();
    std::os::unix::fs::symlink(&target, &unrelated).unwrap();
    swap_into_place(&staged, &dest).unwrap();
    assert_eq!(std::fs::read_link(&unrelated).unwrap(), target);
    assert_eq!(std::fs::read(&dest).unwrap(), b"updated app");
}

#[test]
fn a_failed_rollback_keeps_the_old_app_and_reports_its_backup_path() {
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("app");
    let staged = dir.path().join("app.new");
    std::fs::write(&dest, b"installed app").unwrap();
    std::fs::write(&staged, b"updated app").unwrap();
    let calls = std::cell::Cell::new(0);
    let err = swap_into_place_via(&staged, &dest, |from, to| {
        calls.set(calls.get() + 1);
        match calls.get() {
            1 => std::fs::rename(from, to),
            2 => Err(std::io::Error::other("staged move failed")),
            _ => Err(std::io::Error::other("restore failed")),
        }
    })
    .unwrap_err();
    assert_eq!(calls.get(), 3);
    assert!(err.contains("staged move failed") && err.contains("restore failed"));
    let backup = std::fs::read_dir(dir.path()).unwrap().map(|e| e.unwrap().path()).find(|p| p.is_dir()).unwrap();
    let saved = backup.join("old");
    assert_eq!(std::fs::read(&saved).unwrap(), b"installed app");
    assert!(err.contains(&saved.display().to_string()));
    assert_eq!(std::fs::read(&staged).unwrap(), b"updated app");
}

#[test]
fn a_failed_install_keeps_the_download_for_the_next_try() {
    use crate::http::tests::{Reply, body, serve};
    let base = App::by_id("wordcraft").unwrap();
    let wordcraft = App { icon_png: Some(b"\x89PNG\r\n\x1a\nfake-png-bytes"), ..*base };
    let app = &wordcraft;
    let (body, sha) = body();
    let (url, requests) = serve(body.clone(), vec![Reply::Full, Reply::Full]);
    let asset = Asset { name: "wordcraft-0.3.0-test.pkg".into(), size: body.len() as u64, url, sha256: Some(sha) };
    let release = Release { assets: vec![asset.clone()], ..Release::default() };
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::under(dir.path());
    let http = Http::new(crate::TEST_AGENT).unwrap();
    // A format no build of the launcher installs here stands in for any failing install step.
    let kind = if cfg!(windows) { PackageKind::Dmg } else { PackageKind::Msi };
    let mut phases = Vec::new();
    for _ in 0..2 {
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
        assert!(err.contains("isn't supported"), "{err}");
        assert_eq!(std::fs::read(download_path(&paths, &asset)).unwrap(), body);
    }
    assert_eq!(requests.lock().unwrap().len(), 1, "the second try reuses the kept file");
    assert_eq!(phases.iter().filter(|p| **p == JobPhase::Installing).count(), 2);
}

#[test]
fn an_unavailable_custom_folder_is_rejected_before_downloading() {
    use crate::http::tests::{Reply, body, serve};
    let base = App::by_id("wordcraft").unwrap();
    let wordcraft = App { icon_png: Some(b"\x89PNG\r\n\x1a\nfake-png-bytes"), ..*base };
    let app = &wordcraft;
    let (body, sha) = body();
    let (url, requests) = serve(body.clone(), vec![Reply::Full]);
    let asset = Asset { name: "wordcraft-0.3.0-linux-x86_64.AppImage".into(), size: body.len() as u64, url, sha256: Some(sha) };
    let release = Release { assets: vec![asset.clone()], ..Release::default() };
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("unavailable/Apps");
    let locations = InstallLocations { root: Some(missing.display().to_string()), ..Default::default() };
    let paths = Paths::under(dir.path());
    let http = Http::new(crate::TEST_AGENT).unwrap();
    let mut phases = vec![];
    let mut phase = |p| phases.push(p);
    let mut job = Job { http: &http, paths: &paths, stop: &Stop::default(), phase: &mut phase, install_locations: &locations, replace: false };
    let error = install(app, &release, &asset, PackageKind::AppImage, None, None, &mut job).unwrap_err();
    assert!(error.contains("folder is unavailable") || (process::in_sandbox() && error.contains("aren't supported")), "{error}");
    assert!(requests.lock().unwrap().is_empty());
    assert!(phases.is_empty());
    assert!(!missing.exists());
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

#[cfg(unix)]
#[test]
fn lays_out_both_freebsd_tarball_styles_like_usr_local() {
    let base = App::by_id("wordcraft").unwrap();
    let wordcraft = App { icon_png: Some(b"\x89PNG\r\n\x1a\nfake-png-bytes"), ..*base };
    let app = &wordcraft;
    let dir = tempfile::tempdir().unwrap();
    let tree_style = dir.path().join("tree.tar.gz");
    tarball(
        &tree_style,
        &["wordcraft-0.3.0-freebsd-x86_64/bin/wordcraft", "wordcraft-0.3.0-freebsd-x86_64/share/applications/ai.storyteller.wordcraft.desktop"],
    );
    let flat_style = dir.path().join("flat.tar.gz");
    tarball(
        &flat_style,
        &[
            "wordcraft-0.3.0-freebsd-x86_64/wordcraft",
            "wordcraft-0.3.0-freebsd-x86_64/wordcraft-cli",
            "wordcraft-0.3.0-freebsd-x86_64/README.md",
            "wordcraft-0.3.0-freebsd-x86_64/ai.storyteller.wordcraft.desktop",
            "wordcraft-0.3.0-freebsd-x86_64/ai.storyteller.wordcraft.png",
        ],
    );
    for file in [tree_style, flat_style] {
        let root = tree::unpack(app, &file, &dir.path().join("staging")).unwrap();
        let files = tree::files(&root).unwrap();
        assert!(files.contains(&PathBuf::from("bin/wordcraft")), "{files:?}");
        assert!(files.contains(&PathBuf::from("share/applications/ai.storyteller.wordcraft.desktop")), "{files:?}");
        assert!(!files.iter().any(|f| f.ends_with("README.md")), "{files:?}");
    }
    let bad = dir.path().join("bad.tar.gz");
    tarball(&bad, &["x/README.md"]);
    assert!(tree::unpack(app, &bad, &dir.path().join("staging")).is_err());
}

#[cfg(unix)]
#[test]
fn tree_files_does_not_traverse_directory_symlinks() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root");
    let external = dir.path().join("external");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&external).unwrap();
    std::fs::write(root.join("regular.txt"), b"hi").unwrap();
    std::fs::write(external.join("secret.txt"), b"secret").unwrap();
    std::os::unix::fs::symlink(&external, root.join("link_to_external")).unwrap();

    let list = tree::files(&root).unwrap();
    assert_eq!(list, vec![PathBuf::from("link_to_external"), PathBuf::from("regular.txt")]);
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

#[test]
fn single_top_folder_rejects_symlink() {
    let dir = tempfile::tempdir().unwrap();
    let real_dir = dir.path().join("real");
    std::fs::create_dir_all(&real_dir).unwrap();
    let staging = dir.path().join("staging");
    std::fs::create_dir_all(&staging).unwrap();
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&real_dir, staging.join("symlink_dir")).unwrap();
        let res = single_top_folder(&staging).unwrap();
        assert_eq!(res, staging, "a directory symlink must not be chosen as top folder");
    }
}

#[cfg(target_os = "linux")]
#[test]
fn a_tarball_install_lives_in_the_launchers_folder_and_keeps_the_apps_home() {
    let base = App::by_id("wordcraft").unwrap();
    let wordcraft = App { icon_png: Some(b"\x89PNG\r\n\x1a\nfake-png-bytes"), ..*base };
    let app = &wordcraft;
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::under(dir.path());
    let file = dir.path().join("wordcraft-0.3.0-linux-x86_64.tar.gz");
    tarball(
        &file,
        &[
            "wordcraft-0.3.0-linux-x86_64/bin/wordcraft",
            "wordcraft-0.3.0-linux-x86_64/bin/wordcraft-cli",
            "wordcraft-0.3.0-linux-x86_64/share/applications/ai.storyteller.wordcraft.desktop",
        ],
    );

    let bin = linux::install_tree(app, &file, &paths).unwrap();
    let root = paths.apps().join("wordcraft");
    assert_eq!(bin, root.join("bin").join("wordcraft"));
    assert!(bin.is_file(), "the app's binary wasn't unpacked");
    assert!(root.join("share/applications/ai.storyteller.wordcraft.desktop").is_file(), "share/ wasn't unpacked");
    assert!(!paths.cache.join("wordcraft.new").exists(), "the staging folder was left behind");

    // The app's own home sits beside the tree, and an update keeps it.
    let home = paths.app_home(app.id);
    std::fs::create_dir_all(home.join(".config/wordcraft")).unwrap();
    std::fs::write(home.join(".config/wordcraft/settings.toml"), "keep me").unwrap();
    linux::install_tree(app, &file, &paths).unwrap();
    assert_eq!(std::fs::read_to_string(home.join(".config/wordcraft/settings.toml")).unwrap(), "keep me");

    // Removing the install takes the tree and nothing else.
    linux::remove_tree(app, &paths).unwrap();
    assert!(!bin.exists() && !root.join("share").exists());
    assert!(home.join(".config/wordcraft/settings.toml").is_file(), "the app's settings were removed with it");
    assert!(linux::remove_tree(app, &paths).is_err(), "a second removal says there is nothing there");
}

#[cfg(target_os = "linux")]
#[test]
fn a_tarball_binary_that_is_a_symlink_out_of_the_tree_is_refused() {
    use std::io::Write;
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
    drop(std::io::stdout().flush());

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
#[test]
fn an_app_sees_its_own_home_and_the_xdg_folders_inside_it() {
    let home = PathBuf::from("/data/apps/photocraft/home");
    let env = home_env(&home);
    assert_eq!(env[0], ("HOME", home.clone()));
    for (name, dir) in &env[1..] {
        assert!(dir.starts_with(&home), "{name}: {}", dir.display());
    }
    for wanted in ["XDG_CONFIG_HOME", "XDG_DATA_HOME", "XDG_CACHE_HOME", "XDG_STATE_HOME"] {
        assert!(env.iter().any(|(name, _)| *name == wanted), "{wanted} isn't set");
    }
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
#[test]
fn root_commands_run_programs_by_absolute_path_from_the_given_dirs() {
    use std::ffi::OsStr;
    let root = tempfile::tempdir().unwrap();
    let (usr_bin, sbin) = (root.path().join("usr/bin"), root.path().join("sbin"));
    std::fs::create_dir_all(&usr_bin).unwrap();
    std::fs::create_dir_all(&sbin).unwrap();
    std::fs::write(usr_bin.join("rpm"), b"").unwrap();
    std::fs::write(sbin.join("rpm"), b"").unwrap();
    std::fs::write(sbin.join("zypper"), b"").unwrap();
    let dirs = [&usr_bin, &sbin];
    let run = |args: &[&str]| {
        let args: Vec<&OsStr> = args.iter().map(OsStr::new).collect();
        with_absolute_program(&dirs, &args).map(|a| a.into_iter().map(PathBuf::from).collect::<Vec<_>>())
    };

    // The first directory that has the program wins, and the arguments follow unchanged.
    assert_eq!(run(&["rpm", "-e", "wordcraft"]), Ok(vec![usr_bin.join("rpm"), "-e".into(), "wordcraft".into()]));
    assert_eq!(run(&["zypper"]), Ok(vec![sbin.join("zypper")]));
    // A program that isn't in those directories isn't looked for anywhere else.
    assert_eq!(run(&["dnf", "remove"]), Err("dnf isn't installed".into()));
    // An absolute path is kept; a relative path with folders in it is refused.
    assert_eq!(run(&["/bin/cp", "-R"]), Ok(vec!["/bin/cp".into(), "-R".into()]));
    assert!(run(&["../usr/bin/rpm"]).is_err());
    assert!(run(&["bin/rpm"]).is_err());
    assert!(run(&[]).is_err());
}

#[cfg(target_os = "linux")]
#[test]
fn several_debs_fit_one_apt_call_and_share_its_result() {
    // The files are appended to this, so it must end where apt expects packages.
    assert_eq!(
        linux::install_command(PackageKind::Deb),
        Some(&["apt-get", "-o", "DPkg::Lock::Timeout=60", "install", "-y", "--allow-downgrades"][..])
    );
    assert_eq!(linux::install_command(PackageKind::Flatpak), None);
    let failed = || Err("the system installer stopped with error 100".to_string());
    assert_eq!(linux::shared_outcome(Ok(()), 3), Ok(()));
    assert_eq!(linux::shared_outcome(failed(), 1), failed());
    assert_eq!(
        linux::shared_outcome(failed(), 3),
        Err("the system installer stopped with error 100 while handling 3 apps together. Try them one at a time.".into())
    );
    // A dismissed password prompt stays a cancellation for every app, not an error.
    assert_eq!(linux::shared_outcome(Err("cancelled".into()), 3), Err("cancelled".into()));
}

/// Installs two `.deb` files that reach the installer together (both wait behind a running
/// install), with a stand-in for `pkexec` that exits with `code`. Returns every argument
/// list that reached it, and each job's result.
#[cfg(target_os = "linux")]
fn two_debs_through_pkexec(code: i32) -> (Vec<Vec<String>>, Vec<Result<(), String>>) {
    use std::sync::{Mutex, mpsc};
    let queue = crate::queue::Queue::new();
    let ran = Mutex::new(Vec::new());
    let run = |args: &[&std::ffi::OsStr]| {
        ran.lock().unwrap().push(args.iter().map(|a| a.to_string_lossy().into_owned()).collect::<Vec<_>>());
        root_outcome(Some(code))
    };
    let root = linux::Root { queue: &queue, run: &run, flatpak: &process::flatpak_command_c };
    let (started, has_started) = mpsc::channel();
    let (release, released) = mpsc::channel::<()>();
    let (waiting, waits) = mpsc::channel();
    let results = std::thread::scope(|scope| {
        let (queue, root) = (&queue, &root);
        scope.spawn(move || {
            queue.alone(&|| false, &mut |_| {}, || {
                started.send(()).unwrap();
                released.recv().unwrap();
            });
        });
        has_started.recv().unwrap();
        let jobs = ["/downloads/a.deb", "/downloads/b.deb"].map(|file| {
            let waiting = waiting.clone();
            scope.spawn(move || {
                let mut phase = |p: JobPhase| {
                    if p == JobPhase::Waiting {
                        waiting.send(()).unwrap();
                    }
                };
                linux::install_package_via(root, PackageKind::Deb, Path::new(file), None, &Stop::default(), &mut phase)
            })
        });
        waits.recv().unwrap();
        waits.recv().unwrap();
        release.send(()).unwrap();
        jobs.map(|j| j.join().unwrap()).to_vec()
    });
    (ran.into_inner().unwrap(), results)
}

#[cfg(target_os = "linux")]
#[test]
fn two_debs_reach_root_as_one_apt_call() {
    let (ran, results) = two_debs_through_pkexec(0);
    assert_eq!(ran.len(), 1, "{ran:?}");
    let (command, files) = ran[0].split_at(6);
    assert_eq!(command, ["apt-get", "-o", "DPkg::Lock::Timeout=60", "install", "-y", "--allow-downgrades"]);
    let mut files = files.to_vec();
    files.sort();
    assert_eq!(files, ["/downloads/a.deb", "/downloads/b.deb"]);
    assert_eq!(results, [Ok(()), Ok(())]);

    // The password prompt was dismissed: one call, and both apps are cancelled, not failed.
    let (ran, results) = two_debs_through_pkexec(126);
    assert_eq!(ran.len(), 1, "{ran:?}");
    assert_eq!(results, vec![Err("cancelled".to_string()); 2]);

    // apt failed: both apps say so, and how to find the one at fault.
    let (ran, results) = two_debs_through_pkexec(100);
    assert_eq!(ran.len(), 1, "{ran:?}");
    let error = "the system installer stopped with error 100 while handling 2 apps together. Try them one at a time.";
    assert_eq!(results, vec![Err(error.to_string()); 2]);
}

#[cfg(target_os = "linux")]
#[test]
fn flatpak_bundle_install_failure_reports_clean_error() {
    if !crate::process::is_flatpak_available() {
        return;
    }
    let queue = crate::queue::Queue::new();
    let root = linux::Root { queue: &queue, run: &|_| Ok(()), flatpak: &process::flatpak_command_c };
    let result = linux::install_package_via(&root, PackageKind::Flatpak, Path::new("/nonexistent/file.flatpak"), None, &Stop::default(), &mut |_| ());
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.starts_with("Flatpak couldn't install the bundle: "), "unexpected error: {err}");
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
fn install_bundle(root: &linux::Root<'_>, bundle: Option<&str>) -> Result<(), String> {
    linux::install_package_via(root, PackageKind::Flatpak, Path::new("/x.flatpak"), bundle, &Stop::default(), &mut |_| {})
}

#[cfg(target_os = "linux")]
fn update_bundle(root: &linux::Root<'_>, system: bool) -> Result<(), String> {
    linux::update_flatpak_bundle_via(root, Path::new("/x.flatpak"), FLATPAK_ID, system, &Stop::default(), &mut |_| {})
}

#[cfg(target_os = "linux")]
#[test]
fn a_bundle_goes_in_again_by_hand_only_when_flatpak_needs_it() {
    let uninstall = format!("uninstall --user -y --noninteractive {FLATPAK_ID}");
    let refused = format!("Flatpak couldn't install the bundle: Failed to install bundle: {ALREADY_THERE}");
    let id = Some(FLATPAK_ID);

    // An old Flatpak: the data stays, because nothing else goes on the uninstall's command line.
    let (result, calls) = with_fake_flatpak("1.14.6", 1, ALREADY_THERE, 0, |root| install_bundle(root, id));
    assert_eq!(result, Ok(()));
    assert_eq!(calls, [INSTALL_CALL, "--version", &uninstall, INSTALL_CALL]);

    // A first install that works, or a call with no bundle id, asks nothing more.
    assert_eq!(with_fake_flatpak("1.14.6", 0, ALREADY_THERE, 0, |root| install_bundle(root, id)), (Ok(()), vec![INSTALL_CALL.to_string()]));
    let (result, calls) = with_fake_flatpak("1.14.6", 1, ALREADY_THERE, 0, |root| install_bundle(root, None));
    assert_eq!((result, calls), (Err(refused.clone()), vec![INSTALL_CALL.to_string()]));

    // A Flatpak that can reinstall a bundle meant the refusal, so the installed copy stays.
    let (result, calls) = with_fake_flatpak("1.16.2", 1, ALREADY_THERE, 0, |root| install_bundle(root, id));
    assert_eq!(result, Err(refused.clone()));
    assert_eq!(calls, [INSTALL_CALL, "--version"]);

    // Any other failure leaves it alone too.
    let (result, calls) = with_fake_flatpak("1.14.6", 1, "Invalid checksum", 0, |root| install_bundle(root, id));
    assert_eq!(result, Err("Flatpak couldn't install the bundle: Failed to install bundle: Invalid checksum".to_string()));
    assert_eq!(calls, [INSTALL_CALL]);

    // A failed uninstall reports the first error, and the app is still there.
    let (result, calls) = with_fake_flatpak("1.14.6", 1, ALREADY_THERE, 1, |root| install_bundle(root, id));
    assert_eq!(result, Err(refused.clone()));
    assert_eq!(calls, [INSTALL_CALL, "--version", &uninstall]);

    // A failure after the uninstall says what happened and what to do next.
    let (result, calls) = with_fake_flatpak("1.14.6", 2, ALREADY_THERE, 0, |root| install_bundle(root, id));
    let error = result.unwrap_err();
    assert!(error.starts_with(&refused) && error.contains("uninstalled first") && error.contains("Install tries again"), "{error}");
    assert_eq!(calls, [INSTALL_CALL, "--version", &uninstall, INSTALL_CALL]);
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

#[cfg(target_os = "linux")]
#[test]
fn only_flatpaks_already_installed_error_counts_as_installed() {
    let id = FLATPAK_ID;
    let failed = |why: &str| format!("Flatpak couldn't install the bundle: Failed to install bundle {id}: {why}");
    assert!(linux::already_installed(&failed(ALREADY_THERE), id));
    // A bundle Flatpak can't use, or one for another app, keeps the installed copy.
    assert!(!linux::already_installed(&failed("Can't change remote during bundle install"), id));
    assert!(!linux::already_installed(&failed("Invalid checksum of length 0 expected 32"), id));
    assert!(!linux::already_installed(&failed("ai.storyteller.photocraft already installed"), id));
}

#[cfg(target_os = "linux")]
#[test]
fn only_a_flatpak_before_1_16_2_needs_a_bundle_reinstalled_by_hand() {
    assert!(linux::ignores_bundle_reinstall("Flatpak 1.14.6\n"));
    assert!(linux::ignores_bundle_reinstall("Flatpak 1.16.1"));
    assert!(!linux::ignores_bundle_reinstall("Flatpak 1.16.2"));
    assert!(!linux::ignores_bundle_reinstall("Flatpak 1.18.4"));
    // An unreadable version leaves the failed install as it is.
    assert!(!linux::ignores_bundle_reinstall(""));
}

#[cfg(target_os = "linux")]
#[test]
fn adds_and_removes_only_its_own_menu_entry_and_icon() {
    let base = App::by_id("wordcraft").unwrap();
    let wordcraft = App { icon_png: Some(b"\x89PNG\r\n\x1a\nfake-png-bytes"), ..*base };
    let app = &wordcraft;
    let root = tempfile::tempdir().unwrap();
    let data = root.path().join("share");
    let desktop = data.join("applications/ai.storyteller.wordcraft.desktop");
    let icon = data.join("icons/hicolor/128x128/apps/ai.storyteller.wordcraft.png");
    let old = root.path().join("My Apps/wordcraft-0.2.0-linux-x86_64.AppImage");
    let new = root.path().join("My Apps/wordcraft-0.3.0-linux-x86_64.AppImage");
    std::fs::create_dir_all(new.parent().unwrap()).unwrap();
    std::fs::write(&new, b"").unwrap();

    // Fresh install: entry and icon written and recorded.
    let files = linux::integrate_into(&data, app, &new, &[], None);
    assert_eq!(files, [icon.clone(), desktop.clone()]);
    let text = std::fs::read_to_string(&desktop).unwrap();
    assert!(text.contains("Exec=\"") && text.contains("My Apps/wordcraft-0.3.0-linux-x86_64.AppImage\" %F\n"), "{text}");
    assert!(text.contains("Icon=ai.storyteller.wordcraft\n"));
    assert!(std::fs::read(&icon).unwrap().starts_with(b"\x89PNG"));
    let rec = record("0.3.0", PackageKind::AppImage, new.clone(), files, false);

    // An update rewrites both.
    assert_eq!(linux::integrate_into(&data, app, &new, &[], Some(&rec)).len(), 2);

    // Someone else's entry and icon are left alone (and not recorded).
    std::fs::write(&desktop, "[Desktop Entry]\nExec=/usr/bin/wordcraft\n").unwrap();
    std::fs::write(&icon, b"theirs").unwrap();
    assert!(linux::integrate_into(&data, app, &new, &[], None).is_empty());
    assert_eq!(std::fs::read(&icon).unwrap(), b"theirs");
    // ...unless the entry runs a copy this install just replaced (the app wrote it itself).
    std::fs::write(&desktop, format!("[Desktop Entry]\nExec=\"{}\" %F\n", old.display())).unwrap();
    assert_eq!(linux::integrate_into(&data, app, &new, std::slice::from_ref(&old), None), std::slice::from_ref(&desktop));
    assert!(desktop_entry_is_ours(&desktop));

    // Uninstall: an icon the record doesn't list stays; the entry for the removed copy goes.
    std::fs::remove_file(&new).unwrap();
    linux::unintegrate_from(&data, app, &new, None);
    assert!(!desktop.exists());
    assert_eq!(std::fs::read(&icon).unwrap(), b"theirs");
    // With the record, the launcher's icon goes too.
    std::fs::remove_file(&icon).unwrap();
    std::fs::write(&new, b"").unwrap();
    let files = linux::integrate_into(&data, app, &new, &[], None);
    let rec = record("0.3.0", PackageKind::AppImage, new.clone(), files, false);
    std::fs::remove_file(&new).unwrap();
    linux::unintegrate_from(&data, app, &new, Some(&rec));
    assert!(!desktop.exists() && !icon.exists());
    // Someone else's entry survives an uninstall.
    std::fs::write(&desktop, "[Desktop Entry]\nExec=/usr/bin/wordcraft\n").unwrap();
    linux::unintegrate_from(&data, app, &new, Some(&rec));
    assert!(desktop.exists());
}

#[cfg(target_os = "linux")]
fn desktop_entry_is_ours(path: &Path) -> bool {
    crate::desktop_entry::is_ours(&std::fs::read_to_string(path).unwrap())
}

#[cfg(target_os = "linux")]
#[test]
fn immutable_ostree_offers_no_deb_or_rpm() {
    if !Path::new("/run/ostree-booted").exists() {
        return;
    }
    let kinds = available_kinds();
    assert!(!kinds.contains(&PackageKind::Rpm), "{kinds:?}");
    assert!(!kinds.contains(&PackageKind::Deb), "{kinds:?}");
    assert!(kinds.contains(&PackageKind::AppImage), "{kinds:?}");
}

#[test]
fn updates_keep_the_installed_format() {
    let inst = |source| Installed { version: None, source, location: String::new(), package: None };
    let app = crate::App::by_id("photocraft").unwrap();
    let available = available_kinds();
    if let Some(first) = available.first() {
        assert_eq!(choose_kinds(app, None, None, &[]).first(), Some(first));
    }
    #[cfg(target_os = "macos")]
    assert_eq!(choose_kinds(app, Some(&inst(InstallSource::MacApp)), None, &[]), [PackageKind::Dmg]);
    #[cfg(windows)]
    assert_eq!(choose_kinds(app, Some(&inst(InstallSource::Msi)), None, &[]), [PackageKind::Msi]);
    #[cfg(target_os = "linux")]
    assert_eq!(choose_kinds(app, Some(&inst(InstallSource::AppImage)), None, &[]), [PackageKind::AppImage]);
    let _ = inst;
}

#[test]
fn only_the_release_arch_package_is_updated_by_the_launcher() {
    let pacman = |name: &str| Installed { version: None, source: InstallSource::Pacman, location: String::new(), package: Some(name.into()) };
    let app = crate::App::by_id("wordcraft").unwrap();
    assert!(is_release_arch_package(app, &pacman("ai.storyteller.wordcraft")));
    for aur in ["wordcraft", "wordcraft-bin", "wordcraft-git"] {
        assert!(!is_release_arch_package(app, &pacman(aur)), "{aur}");
    }
    assert_eq!(managed_elsewhere_with(app, &pacman("ai.storyteller.wordcraft"), false, |_| None), None);
    assert_eq!(managed_elsewhere_with(app, &pacman("wordcraft-bin"), true, |_| None), None);
}

#[test]
fn arch_packages_from_a_repository_are_updated_with_the_system() {
    let app = crate::App::by_id("wordcraft").unwrap();
    let hint = pacman_update_hint(app, "wordcraft-bin", Some("omarchy"));
    assert!(hint.starts_with("WordCraft was installed from the omarchy repository or the AUR (wordcraft-bin)."));
    assert!(hint.ends_with("or sudo pacman -Syu"));

    let aur_hint = pacman_update_hint(app, "wordcraft-bin", None);
    assert!(aur_hint.starts_with("WordCraft was installed from the AUR (wordcraft-bin)."));

    let si = "Repository      : omarchy\nName            : wordcraft-bin\nVersion         : 0.6.1-1\n";
    assert_eq!(parse_pacman_repo(si).as_deref(), Some("omarchy"));
    assert_eq!(parse_pacman_repo("Name : wordcraft-bin\n"), None);
    assert_eq!(parse_pacman_repo("Repository :\n"), None);
}

#[test]
fn aur_packages_are_not_updated_as_arch_pkg() {
    let pacman = |name: &str| Installed { version: None, source: InstallSource::Pacman, location: String::new(), package: Some(name.into()) };
    let app = crate::App::by_id("filmcraft").unwrap();
    let installed = pacman("filmcraft-bin");
    assert!(managed_elsewhere_with(app, &installed, false, |_| None).is_some());
    assert_eq!(choose_kinds_with(app, Some(&installed), None, &[], &[], false), Vec::<PackageKind>::new());
    assert_eq!(choose_kinds_with(app, Some(&installed), None, &[PackageKind::ArchPkg], &[PackageKind::ArchPkg], true), Vec::<PackageKind>::new());
}

#[test]
fn release_arch_packages_are_updated_as_arch_pkg() {
    let pacman = |name: &str| Installed { version: None, source: InstallSource::Pacman, location: String::new(), package: Some(name.into()) };
    let app = crate::App::by_id("filmcraft").unwrap();
    let installed = pacman("ai.storyteller.filmcraft");
    assert_eq!(managed_elsewhere_with(app, &installed, false, |_| None), None);
    assert_eq!(choose_kinds_with(app, Some(&installed), None, &[], &[], true), vec![PackageKind::ArchPkg]);
    assert_eq!(choose_kinds_with(app, Some(&installed), None, &[], &[PackageKind::ArchPkg], false), vec![PackageKind::ArchPkg]);
}

#[test]
fn unavailable_arch_package_not_selected_for_new_install() {
    let app = crate::App::by_id("filmcraft").unwrap();
    assert_eq!(choose_kinds_with(app, None, None, &[PackageKind::ArchPkg], &[], true), Vec::<PackageKind>::new());
    assert_eq!(choose_kinds_with(app, None, None, &[PackageKind::ArchPkg], &[], false), Vec::<PackageKind>::new());
    assert_eq!(choose_kinds_with(app, None, None, &[PackageKind::ArchPkg], &[PackageKind::ArchPkg], false), vec![PackageKind::ArchPkg]);
}

#[test]
fn install_refuses_to_downgrade_when_target_release_is_older() {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::under(dir.path());
    let http = Http::new(crate::TEST_AGENT).unwrap();
    let phase = &mut |_| ();
    let locations = InstallLocations::default();
    let app = App::by_id("photocraft").unwrap();
    let release = Release { version: version::parse("0.4.0"), tag: "v0.4.0".into(), ..Release::default() };
    let asset = Asset { name: "photocraft-0.4.0-linux-x86_64.AppImage".into(), ..Asset::default() };
    let installed = Installed {
        version: version::parse("0.5.0"),
        source: InstallSource::Launcher,
        location: dir.path().join("PhotoCraft.AppImage").display().to_string(),
        package: None,
    };
    let stop = Stop::default();
    let mut job = Job { http: &http, paths: &paths, stop: &stop, phase, install_locations: &locations, replace: false };
    let result = install_at(app, &release, &asset, PackageKind::AppImage, Some(&installed), None, &mut job, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("installed version 0.5.0 is newer"));
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

#[cfg(target_os = "linux")]
#[test]
fn package_kinds_order_matches_launcher_format() {
    use super::linux::order_available_kinds;

    // Flatpak launcher defaults to Flatpak if available
    let kinds = order_available_kinds(true, false, true, true, false, false);
    assert_eq!(kinds.first(), Some(&PackageKind::Flatpak));
    assert!(kinds.contains(&PackageKind::Deb));
    assert!(kinds.contains(&PackageKind::AppImage));

    // AppImage launcher defaults to AppImage
    let kinds = order_available_kinds(false, true, true, true, false, false);
    assert_eq!(kinds.first(), Some(&PackageKind::AppImage));
    assert!(kinds.contains(&PackageKind::Deb));
    assert!(kinds.contains(&PackageKind::Flatpak));

    // Base Linux launcher defaults to native package format (deb, rpm, or arch) if available
    let kinds = order_available_kinds(false, false, true, true, false, false);
    assert_eq!(kinds.first(), Some(&PackageKind::Deb));

    let kinds = order_available_kinds(false, false, true, false, true, false);
    assert_eq!(kinds.first(), Some(&PackageKind::Rpm));

    let kinds = order_available_kinds(false, false, true, false, false, true);
    assert_eq!(kinds.first(), Some(&PackageKind::ArchPkg));

    // Base Linux launcher without root/distro packaging falls back to Flatpak then AppImage
    let kinds = order_available_kinds(false, false, true, false, false, false);
    assert_eq!(kinds.first(), Some(&PackageKind::Flatpak));
    assert_eq!(kinds.get(1), Some(&PackageKind::AppImage));

    // The tarball runs inside the sandbox, so it is only offered there: with a usable host
    // Flatpak it stays an alternative, without one it is the default (an AppImage cannot
    // mount in a sandbox, and a distro package needs root).
    let kinds = order_available_kinds(true, false, true, false, false, false);
    assert_eq!(kinds.first(), Some(&PackageKind::Flatpak));
    assert_eq!(kinds.get(1), Some(&PackageKind::TarGz));
    let kinds = order_available_kinds(true, false, false, false, false, false);
    assert_eq!(kinds, [PackageKind::TarGz, PackageKind::AppImage]);
    for (appimage, flatpak) in [(true, true), (false, false), (false, true)] {
        assert!(!order_available_kinds(false, appimage, flatpak, true, true, false).contains(&PackageKind::TarGz));
    }
}

#[cfg(target_os = "macos")]
mod macos_checks {
    use crate::{SUITE, version};

    use super::macos::{Expect, TEAM_ID, install_dmg, signed_by};
    use super::*;

    fn photocraft() -> &'static App {
        SUITE.iter().find(|a| a.id == "photocraft").unwrap()
    }

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
    fn custom_bundles_are_detected_and_updates_stay_in_their_actual_location() {
        let dir = tempfile::tempdir().unwrap();
        let apps = dir.path().join("Craft Apps");
        std::fs::create_dir(&apps).unwrap();
        let paths = Paths::under(dir.path());
        let image = dmg(dir.path(), "0.6.0");
        let expect = Expect { team: None, version: None };
        let placed = install_dmg(photocraft(), &image, None, Some(&apps), false, &paths, &expect).unwrap();
        assert_eq!(placed, apps.join("PhotoCraft.app"));
        let mut manifest = crate::manifest::Manifest::default();
        manifest.apps.insert("photocraft".into(), record("0.6.0", PackageKind::Dmg, placed.clone(), vec![], false));
        let existing = crate::detect::detect(photocraft(), &manifest, &crate::detect::Snapshot::default()).unwrap();
        assert_eq!(existing.source, InstallSource::MacApp);
        assert_eq!(existing.location, placed.display().to_string());
        assert_eq!(existing.version.unwrap().to_string(), "0.6.0");
        let existing = crate::detect::detect(photocraft(), &manifest, &crate::detect::Snapshot::default()).unwrap();
        let new_default = dir.path().join("Other Apps");
        std::fs::create_dir(&new_default).unwrap();
        let update = dmg(dir.path(), "0.7.0");
        assert_eq!(install_dmg(photocraft(), &update, Some(&existing), Some(&new_default), false, &paths, &expect).unwrap(), placed);
        assert!(!new_default.join("PhotoCraft.app").exists());
        let detected = crate::detect::detect(photocraft(), &manifest, &crate::detect::Snapshot::default()).unwrap();
        assert_eq!(detected.version.unwrap().to_string(), "0.7.0");
        std::fs::write(placed.join("Contents/Info.plist"), "invalid").unwrap();
        assert_ne!(
            crate::detect::detect(photocraft(), &manifest, &crate::detect::Snapshot::default()).map(|i| i.source),
            Some(InstallSource::Launcher)
        );
    }

    #[test]
    fn an_unrelated_bundle_at_the_selected_destination_is_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let placed = bundle(dir.path(), "0.5.0");
        let plist = placed.join("Contents/Info.plist");
        let unrelated = std::fs::read_to_string(&plist).unwrap().replace("ai.storyteller.photocraft", "com.example.other");
        std::fs::write(&plist, &unrelated).unwrap();
        let image = dmg(dir.path(), "0.6.0");
        let result =
            install_dmg(photocraft(), &image, None, Some(dir.path()), false, &Paths::under(dir.path()), &Expect { team: None, version: None });
        assert!(result.unwrap_err().contains("already exists"));
        assert_eq!(std::fs::read_to_string(plist).unwrap(), unrelated);
        assert_eq!(std::fs::read(placed.join("Contents/MacOS/PhotoCraft")).unwrap(), b"0.5.0");
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

    #[test]
    fn the_bundle_must_be_the_releases_version() {
        let dir = tempfile::tempdir().unwrap();
        let apps = dir.path().join("Applications");
        let current = bundle(&apps, "0.5.0");
        let existing = Installed { version: None, source: InstallSource::MacApp, location: current.display().to_string(), package: None };
        let paths = Paths::under(dir.path());
        let image = dmg(dir.path(), "0.6.0");

        for (release, ok) in [("0.6.0", true), ("0.7.0", false), ("0.6.0-rc.1", true), ("0.5.0", false)] {
            std::fs::write(current.join("Contents/MacOS/PhotoCraft"), "0.5.0").unwrap();
            let want = version::parse(release).unwrap();
            let expect = Expect { team: None, version: Some(&want) };
            let result = install_dmg(photocraft(), &image, Some(&existing), None, false, &paths, &expect);
            assert_eq!(result.is_ok(), ok, "release {release}: {result:?}");
            let now = std::fs::read_to_string(current.join("Contents/MacOS/PhotoCraft")).unwrap();
            assert_eq!(now, if ok { "0.6.0" } else { "0.5.0" }, "release {release}");
            if !ok {
                assert_eq!(result, Err(format!("the app inside isn't version {release}, so it wasn't installed")));
            }
            assert_eq!(
                std::fs::read_dir(&apps).unwrap().map(|e| e.unwrap().file_name()).collect::<Vec<_>>(),
                [std::ffi::OsString::from("PhotoCraft.app")]
            );
        }

        // A bundle that names a pre-release must name the release's.
        let rc = dmg(dir.path(), "0.8.0-rc.2");
        let want = version::parse("0.8.0-rc.1").unwrap();
        let expect = Expect { team: None, version: Some(&want) };
        assert!(install_dmg(photocraft(), &rc, Some(&existing), None, false, &paths, &expect).is_err());
    }

    #[test]
    fn an_unsigned_update_leaves_the_installed_copy_alone() {
        let dir = tempfile::tempdir().unwrap();
        let apps = dir.path().join("Applications");
        let current = bundle(&apps, "0.5.0");
        let existing = Installed { version: None, source: InstallSource::MacApp, location: current.display().to_string(), package: None };
        let image = dmg(dir.path(), "0.6.0");
        let release = Release { tag: "v0.6.0".into(), version: version::parse("0.6.0"), ..Release::default() };

        let result = install_dmg(photocraft(), &image, Some(&existing), None, false, &Paths::under(dir.path()), &Expect::release(&release));
        assert_eq!(result, Err("the app isn't signed by its developer, so it wasn't installed".into()));
        assert_eq!(std::fs::read_to_string(current.join("Contents/MacOS/PhotoCraft")).unwrap(), "0.5.0");
        assert_eq!(
            std::fs::read_dir(&apps).unwrap().map(|e| e.unwrap().file_name()).collect::<Vec<_>>(),
            [std::ffi::OsString::from("PhotoCraft.app")]
        );
    }

    /// A real release passes. Run with `CRAFT_APP=/path/to/PhotoCraft.app cargo test -p
    /// artcraft-launcher-native -- --ignored a_real_craft_app`.
    #[test]
    #[ignore = "needs a Craft App from a real release"]
    fn a_real_craft_app_carries_the_teams_signature() {
        let app = std::env::var("CRAFT_APP").expect("set CRAFT_APP to a Craft App bundle");
        assert!(signed_by(Path::new(&app), TEAM_ID), "{app} isn't signed by {TEAM_ID}");
    }
}
