//! Tests for the portable install.

use super::*;
use std::io::Write;

/// A folder on a network share is the same folder after `canonicalize`.
#[cfg(windows)]
#[test]
fn verbatim_paths_match_their_plain_form() {
    assert!(same_windows_path(Path::new(r"C:\Apps\ArtCraft"), Path::new(r"\\?\C:\apps\artcraft\")));
    assert!(same_windows_path(Path::new(r"\\server\share\Apps\ArtCraft"), Path::new(r"\\?\UNC\server\share\Apps\ArtCraft")));
    assert!(same_windows_path(Path::new(r"\\?\unc\server\share\Apps\ArtCraft"), Path::new(r"\\Server\Share\Apps\ArtCraft")));
    assert!(!same_windows_path(Path::new(r"C:\Apps\ArtCraft"), Path::new(r"\\?\D:\Apps\ArtCraft")));
}

/// Previous files left in the staging folder by a failed update are never cleared by the next one.
#[test]
fn a_self_update_keeps_the_previous_files_of_a_failed_one() {
    let dir = tempfile::tempdir().unwrap();
    let staging = dir.path().join("launcher-update");
    std::fs::create_dir_all(staging.join("backup")).unwrap();
    std::fs::write(staging.join("backup").join("artcraft-launcher.exe"), "0.1.0").unwrap();
    let zip = archive(dir.path(), "update.zip", &[("artcraft-launcher.exe", b"0.2.0")]);
    let folder = dir.path().join("ArtCraft Launcher");
    std::fs::create_dir_all(&folder).unwrap();
    let error = unpack_over(&zip, &staging, &folder, std::ffi::OsStr::new("artcraft-launcher.exe"), &|_| Ok(())).unwrap_err();
    assert!(error.contains("previous files"), "{error}");
    assert_eq!(std::fs::read_to_string(staging.join("backup").join("artcraft-launcher.exe")).unwrap(), "0.1.0");
    assert!(!folder.join("artcraft-launcher.exe").exists());
}

/// Validation of the installed copy is part of the transaction, while a backup still exists.
#[test]
fn a_self_update_that_fails_validation_puts_the_previous_files_back() {
    let dir = tempfile::tempdir().unwrap();
    let staging = dir.path().join("launcher-update");
    let folder = dir.path().join("ArtCraft Launcher");
    std::fs::create_dir_all(&folder).unwrap();
    let exe = folder.join("artcraft-launcher.exe");
    std::fs::write(&exe, "old launcher").unwrap();
    let zip = archive(dir.path(), "update.zip", &[("launcher/artcraft-launcher.exe", b"new launcher"), ("launcher/README.md", b"new readme")]);
    let checked = std::cell::Cell::new(false);
    let error = unpack_over(&zip, &staging, &folder, std::ffi::OsStr::new("artcraft-launcher.exe"), &|file| {
        if file == exe {
            assert_eq!(std::fs::read(file).unwrap(), b"new launcher");
            assert!(folder.join("README.md").is_file());
            checked.set(true);
            Err("installed launcher failed validation".into())
        } else {
            Ok(())
        }
    })
    .unwrap_err();
    assert_eq!(error, "installed launcher failed validation");
    assert!(checked.get());
    assert_eq!(std::fs::read(&exe).unwrap(), b"old launcher");
    assert!(!folder.join("README.md").exists());
    assert!(!staging.exists());
}

fn archive(root: &Path, name: &str, files: &[(&str, &[u8])]) -> PathBuf {
    let path = root.join(name);
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
    for (name, content) in files {
        if name.ends_with('/') {
            zip.add_directory(*name, zip::write::SimpleFileOptions::default()).unwrap();
        } else {
            zip.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
            zip.write_all(content).unwrap();
        }
    }
    zip.finish().unwrap();
    path
}

#[test]
fn wrapped_layout_validates_explicit_directory_entries() {
    let dir = tempfile::tempdir().unwrap();
    let app = App::by_id("photocraft").unwrap();
    let paths = Paths::under(dir.path());
    let invalid = archive(dir.path(), "invalid-dirs.zip", &[("wrapper/", b""), ("wrapper/photocraft.exe", b"program"), ("outside/", b"")]);
    let error = install(app, &invalid, "0.12.0", None, &paths, None, false, &mut |_| Ok(())).unwrap_err();
    assert!(error.contains("unsupported directory layout"), "{error}");
    assert!(!paths.apps().join(app.id).exists());

    let valid = archive(dir.path(), "valid-dirs.zip", &[("wrapper/", b""), ("wrapper/photocraft.exe", b"program")]);
    let record = install(app, &valid, "0.12.0", None, &paths, None, false, &mut |_| Ok(())).unwrap();
    assert!(record.path.exists());
}

#[test]
fn wrapped_packages_install_update_and_remove_at_a_stable_location() {
    let dir = tempfile::tempdir().unwrap();
    let app = App::by_id("photocraft").unwrap();
    let paths = Paths::under(dir.path());
    let parent = dir.path().join("My Tools");
    let mut previous = None;
    // The published ZIPs wrap the app, CLI and resources in a versioned directory.
    // Also exercise transitions to and from the older, flat layout.
    for (version, prefix) in [
        ("0.12.0", "photocraft-0.12.0-windows-x64-portable/"),
        ("0.13.0", "photocraft-0.13.0-windows-x64-portable/"),
        ("0.11.0", ""),
        ("0.12.0", "photocraft-0.12.0-windows-x64-portable/"),
    ] {
        let zip = archive(
            dir.path(),
            "app.zip",
            &[
                (&format!("{prefix}photocraft-cli.exe"), b"cli"),
                (&format!("{prefix}PhotoCraft.exe"), version.as_bytes()),
                (&format!("{prefix}lib/runtime.dll"), b"library"),
                (&format!("{prefix}LICENSE-MIT"), b"license"),
            ],
        );
        let record = install(app, &zip, version, previous.as_ref(), &paths, Some(&parent), false, &mut |_| Ok(())).unwrap();
        let root = std::fs::canonicalize(parent.join(app.id)).unwrap();
        assert_eq!(record.path, root.join("PhotoCraft.exe"));
        assert_eq!(std::fs::read(&record.path).unwrap(), version.as_bytes());
        assert_eq!(std::fs::read(root.join("lib/runtime.dll")).unwrap(), b"library");
        assert_eq!(record.files, ["photocraft-cli.exe", "PhotoCraft.exe", "lib/runtime.dll", "LICENSE-MIT"].map(|p| root.join(p)));
        let personal = root.join("settings.json");
        if previous.is_some() {
            assert_eq!(std::fs::read(&personal).unwrap(), b"personal");
        } else {
            std::fs::write(&personal, b"personal").unwrap();
        }
        previous = Some(record);
    }
    let record = previous.unwrap();
    remove(app, &record).unwrap();
    assert!(record.files.iter().all(|p| !p.exists()));
    assert_eq!(std::fs::read(parent.join("photocraft/settings.json")).unwrap(), b"personal");
}

#[test]
fn rejects_missing_executables_and_ambiguous_or_deep_wrappers() {
    let dir = tempfile::tempdir().unwrap();
    let app = App::by_id("photocraft").unwrap();
    let paths = Paths::under(dir.path());
    for names in [
        vec![],
        vec!["package/photocraft-cli.exe"],
        vec!["package/bin/photocraft.exe"],
        vec!["one/photocraft.exe", "two/photocraft.exe"],
        vec!["package/photocraft.exe", "outside.txt"],
        vec!["package/photocraft.exe", "other/runtime.dll"],
    ] {
        let files: Vec<_> = names.iter().map(|name| (*name, b"content".as_slice())).collect();
        let zip = archive(dir.path(), "invalid.zip", &files);
        assert!(install(app, &zip, "0.12.0", None, &paths, None, false, &mut |_| Ok(())).is_err(), "{names:?}");
        assert!(!paths.apps().join(app.id).exists());
    }
}

#[test]
fn wrapped_update_keeps_collision_checks_and_rolls_back_failed_commits() {
    let dir = tempfile::tempdir().unwrap();
    let app = App::by_id("photocraft").unwrap();
    let paths = Paths::under(dir.path());
    let old = archive(dir.path(), "old.zip", &[("photocraft.exe", b"old"), ("lib.dll", b"library")]);
    let record = install(app, &old, "0.11.0", None, &paths, None, false, &mut |_| Ok(())).unwrap();
    let personal = paths.apps().join("photocraft/settings.json");
    std::fs::write(&personal, b"keep").unwrap();
    let collision = archive(dir.path(), "collision.zip", &[("package/photocraft.exe", b"new"), ("package/settings.json", b"overwrite")]);
    let error = install(app, &collision, "0.12.0", Some(&record), &paths, None, false, &mut |_| Ok(())).unwrap_err();
    assert_eq!(error, "The installation would overwrite an unrecorded file.");
    let next = archive(dir.path(), "next.zip", &[("package/photocraft.exe", b"new"), ("package/new.dll", b"new")]);
    let error = install(app, &next, "0.12.0", Some(&record), &paths, None, false, &mut |_| Err("manifest disk full".into())).unwrap_err();
    assert_eq!(error, "manifest disk full");
    assert_eq!(std::fs::read(record.path).unwrap(), b"old");
    assert_eq!(std::fs::read(paths.apps().join("photocraft/lib.dll")).unwrap(), b"library");
    assert!(!paths.apps().join("photocraft/new.dll").exists());
    assert_eq!(std::fs::read(personal).unwrap(), b"keep");
}

#[cfg(unix)]
#[test]
fn host_symlinks_are_allowed_but_owned_links_are_rejected() {
    use std::os::unix::fs::symlink;

    let dir = tempfile::tempdir().unwrap();
    let host = dir.path().join("var-home");
    std::fs::create_dir(&host).unwrap();
    let home = dir.path().join("home");
    symlink(&host, &home).unwrap();
    let root = home.join("photocraft");
    assert!(validate_directory(&root).is_ok());
    assert!(validate_owned_directory(&root, &root).is_ok());

    let app = App::by_id("photocraft").unwrap();
    let paths = Paths::under(&home);
    let zip = archive(dir.path(), "app.zip", &[("photocraft.exe", b"program"), ("lib/runtime.dll", b"library")]);
    let record = install(app, &zip, "0.5.0", None, &paths, Some(&home), false, &mut |_| Ok(())).unwrap();
    assert_eq!(std::fs::read(&record.path).unwrap(), b"program");

    std::fs::remove_file(root.join("lib/runtime.dll")).unwrap();
    std::fs::remove_dir(root.join("lib")).unwrap();
    symlink(&host, root.join("lib")).unwrap();
    assert!(validate_owned_directory(&root, &root.join("lib/nested")).is_err());
    assert!(remove(app, &record).is_err());
    assert!(install(app, &zip, "0.6.0", Some(&record), &paths, None, false, &mut |_| Ok(())).is_err());
    assert_eq!(std::fs::read(&record.path).unwrap(), b"program");

    let linked_root = home.join("linked-app");
    symlink(&root, &linked_root).unwrap();
    assert!(validate_owned_directory(&linked_root, &linked_root).is_err());
    assert!(validate_owned_directory(&root, &home).is_err());
    let dangling = home.join("dangling");
    symlink(home.join("missing"), &dangling).unwrap();
    assert!(validate_directory(&dangling.join("nested")).is_err());
}

#[test]
fn stale_junction_record_cannot_remove_files_after_retargeting() {
    use std::process::Command;

    fn junction(link: &Path, target: &Path) {
        let status = Command::new("cmd.exe").args(["/C", "mklink", "/J"]).arg(link).arg(target).status().unwrap();
        assert!(status.success());
    }

    let dir = tempfile::tempdir().unwrap();
    let first = dir.path().join("first");
    let second = dir.path().join("second");
    let link = dir.path().join("Apps junction");
    std::fs::create_dir(&first).unwrap();
    std::fs::create_dir(&second).unwrap();
    junction(&link, &first);
    let alias_root = link.join("photocraft");
    std::fs::create_dir(&alias_root).unwrap();
    std::fs::write(alias_root.join("photocraft.exe"), b"old program").unwrap();
    let record = Record {
        version: "0.5.0".into(),
        kind: PackageKind::PortableZip,
        path: alias_root.join("photocraft.exe"),
        installed_at: 0,
        files: vec![alias_root.join("photocraft.exe")],
        needs_root: false,
    };
    std::process::Command::new("cmd.exe").args(["/C", "rmdir"]).arg(&link).status().unwrap();
    junction(&link, &second);
    let foreign_root = link.join("photocraft");
    std::fs::create_dir(&foreign_root).unwrap();
    std::fs::write(foreign_root.join("photocraft.exe"), b"foreign program").unwrap();
    assert!(remove(App::by_id("photocraft").unwrap(), &record).is_err());
    assert_eq!(std::fs::read(foreign_root.join("photocraft.exe")).unwrap(), b"foreign program");
}

#[test]
fn upgrade_downgrade_and_uninstall_preserve_personal_files_and_location() {
    let dir = tempfile::tempdir().unwrap();
    let app = App::by_id("photocraft").unwrap();
    let paths = Paths::under(dir.path());
    let parent = dir.path().join("My Tools");
    let first = archive(dir.path(), "first.zip", &[("photocraft.exe", b"first"), ("obsolete.dll", b"old")]);
    let mut record = install(app, &first, "0.5.0", None, &paths, Some(&parent), false, &mut |_| Ok(())).unwrap();
    let personal = parent.join("photocraft").join("settings.json");
    std::fs::write(&personal, b"personal").unwrap();
    for (version, exe) in [("0.6.0", b"new".as_slice()), ("0.4.0", b"older".as_slice())] {
        let executable = if version == "0.4.0" { "PhotoCraft.exe" } else { "photocraft.exe" };
        let package = archive(dir.path(), "next.zip", &[(executable, exe), ("lib/runtime.dll", b"dll")]);
        record = install(app, &package, version, Some(&record), &paths, Some(&dir.path().join("Ignored")), false, &mut |_| Ok(())).unwrap();
        assert_eq!(std::fs::read(&record.path).unwrap(), exe);
        assert_eq!(record.path.parent().unwrap(), std::fs::canonicalize(personal.parent().unwrap()).unwrap());
        assert!(!parent.join("photocraft/obsolete.dll").exists());
        assert_eq!(std::fs::read(&personal).unwrap(), b"personal");
        let mut manifest = crate::manifest::Manifest::default();
        manifest.apps.insert(app.id.into(), record.clone());
        manifest.save(&paths.data.join("installed.json")).unwrap();
        let restored = crate::manifest::Manifest::load(&paths.data.join("installed.json"));
        let detected = crate::detect::detect(app, &restored, &crate::detect::Snapshot::take()).unwrap();
        assert_eq!(detected.version, crate::version::parse(version));
        assert_eq!(detected.location, record.path.to_string_lossy());
    }
    remove(app, &record).unwrap();
    assert!(!record.path.exists());
    assert_eq!(std::fs::read(personal).unwrap(), b"personal");
}

#[test]
fn a_failed_manifest_commit_restores_the_previous_program() {
    let dir = tempfile::tempdir().unwrap();
    let app = App::by_id("photocraft").unwrap();
    let paths = Paths::under(dir.path());
    let old = archive(dir.path(), "old.zip", &[("photocraft.exe", b"old"), ("lib.dll", b"library")]);
    let record = install(app, &old, "0.5.0", None, &paths, None, false, &mut |_| Ok(())).unwrap();
    let next = archive(dir.path(), "next.zip", &[("photocraft.exe", b"new"), ("new.dll", b"new")]);
    let result = install(app, &next, "0.6.0", Some(&record), &paths, None, false, &mut |_| Err("manifest disk full".into()));
    assert_eq!(result.unwrap_err(), "manifest disk full");
    assert_eq!(std::fs::read(record.path).unwrap(), b"old");
    assert_eq!(std::fs::read(paths.apps().join("photocraft/lib.dll")).unwrap(), b"library");
    assert!(!paths.apps().join("photocraft/new.dll").exists());
}

#[test]
fn collision_is_rejected_before_replacing_any_old_file() {
    let dir = tempfile::tempdir().unwrap();
    let app = App::by_id("photocraft").unwrap();
    let paths = Paths::under(dir.path());
    let old = archive(dir.path(), "old.zip", &[("photocraft.exe", b"old")]);
    let record = install(app, &old, "0.5.0", None, &paths, None, false, &mut |_| Ok(())).unwrap();
    let personal = paths.apps().join("photocraft/settings.json");
    std::fs::write(&personal, b"keep").unwrap();
    let next = archive(dir.path(), "next.zip", &[("photocraft.exe", b"new"), ("settings.json", b"overwrite")]);
    assert!(install(app, &next, "0.6.0", Some(&record), &paths, None, false, &mut |_| Ok(())).is_err());
    assert_eq!(std::fs::read(record.path).unwrap(), b"old");
    assert_eq!(std::fs::read(personal).unwrap(), b"keep");
}

#[test]
fn a_new_install_writes_over_unrecorded_files_only_when_asked_to() {
    let dir = tempfile::tempdir().unwrap();
    let app = App::by_id("photocraft").unwrap();
    let paths = Paths::under(dir.path());
    let folder = paths.apps().join("photocraft");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("photocraft.exe"), b"theirs").unwrap();
    std::fs::write(folder.join("notes.txt"), b"keep").unwrap();
    let zip = archive(dir.path(), "new.zip", &[("photocraft.exe", b"new")]);
    assert!(install(app, &zip, "0.5.0", None, &paths, None, false, &mut |_| Ok(())).is_err());
    assert_eq!(std::fs::read(folder.join("photocraft.exe")).unwrap(), b"theirs");
    install(app, &zip, "0.5.0", None, &paths, None, true, &mut |_| Ok(())).unwrap();
    assert_eq!(std::fs::read(folder.join("photocraft.exe")).unwrap(), b"new");
    assert_eq!(std::fs::read(folder.join("notes.txt")).unwrap(), b"keep");
}

#[test]
fn installs_official_single_directory_portable_zip() {
    let dir = tempfile::tempdir().unwrap();
    let app = App::by_id("photocraft").unwrap();
    let paths = Paths::under(dir.path());
    let zip = archive(
        dir.path(),
        "release.zip",
        &[
            ("photocraft-0.6.1-windows-x64-portable/photocraft.exe", b"program"),
            ("photocraft-0.6.1-windows-x64-portable/portable.txt", b""),
            ("photocraft-0.6.1-windows-x64-portable/lib/runtime.dll", b"library"),
        ],
    );
    let record = install(app, &zip, "0.6.1", None, &paths, None, false, &mut |_| Ok(())).unwrap();
    assert_eq!(record.path.file_name().unwrap(), "photocraft.exe");
    assert_eq!(record.path.parent().unwrap().file_name().unwrap(), "photocraft");
    assert_eq!(std::fs::read(&record.path).unwrap(), b"program");
    assert_eq!(std::fs::read(paths.apps().join("photocraft/lib/runtime.dll")).unwrap(), b"library");
    assert_eq!(record.files.len(), 3);
    remove(app, &record).unwrap();
    assert!(!record.path.exists());
}

#[test]
fn rejects_mixed_root_and_multiple_directory_archives() {
    let dir = tempfile::tempdir().unwrap();
    let app = App::by_id("photocraft").unwrap();
    let paths = Paths::under(dir.path());
    for (name, entries) in [
        ("mixed.zip", vec![("package/photocraft.exe", b"program".as_slice()), ("another/readme.txt", b"other".as_slice())]),
        ("nested.zip", vec![("wrapper/inner/photocraft.exe", b"program".as_slice())]),
    ] {
        let zip = archive(dir.path(), name, &entries);
        assert!(install(app, &zip, "0.6.1", None, &paths, None, false, &mut |_| Ok(())).is_err(), "{name}");
        assert!(!paths.apps().join("photocraft/photocraft.exe").exists());
    }
}

/// ZIP 9 strips a drive prefix from enclosed paths; the original name must still be rejected.
#[test]
fn rejects_drive_prefixed_names_before_writing_files() {
    let dir = tempfile::tempdir().unwrap();
    let zip = archive(dir.path(), "drive.zip", &[("c:escape.exe", b"bad".as_slice())]);
    let output = dir.path().join("output");
    std::fs::create_dir(&output).unwrap();
    assert!(extract(&zip, &output).is_err());
    assert_eq!(std::fs::read_dir(&output).unwrap().count(), 0);
}

#[test]
fn rejects_traversal_and_case_collisions() {
    let dir = tempfile::tempdir().unwrap();
    for (name, files) in [
        ("traversal.zip", vec![("../escape.exe", b"bad".as_slice())]),
        ("colon.zip", vec![("data:stream", b"bad".as_slice())]),
        ("duplicate.zip", vec![("PhotoCraft.exe", b"one".as_slice()), ("photocraft.exe", b"two".as_slice())]),
    ] {
        let zip = archive(dir.path(), name, &files);
        let output = dir.path().join(name.replace('.', "-"));
        std::fs::create_dir(&output).unwrap();
        assert!(extract(&zip, &output).is_err(), "{name}");
    }
    assert!(!dir.path().join("escape.exe").exists());
    assert!(validate_directory(Path::new("relative/folder")).is_err());
    assert!(validate_directory(&dir.path().join("../escape")).is_err());
}
