//! Tests for launcher self-update.

use super::detect::*;
use super::helper::*;
#[cfg(unix)]
use super::*;
use craft_ui::testing::trial;

/// The helper reads back exactly what the launcher passed it, paths with spaces and
/// apostrophes included, and nothing that is incomplete or unknown.
#[trial]
fn the_update_helper_reads_back_its_arguments() {
    let exe = Path::new(r"C:\Users\O'Brien\AppData\Local\Programs\ArtCraft Launcher\artcraft-launcher.exe");
    let msi = Finish::Msi(PathBuf::from(r"C:\Users\O'Brien\AppData\Local\ArtCraft Launcher\cache\artcraft-launcher-0.7.0-windows-x64.msi"));
    let portable =
        Finish::Portable { zip: PathBuf::from(r"D:\Tools\x 1.zip"), tmp: PathBuf::from(r"D:\Tools\ArtCraftLauncherData\cache\launcher-update") };
    for finish in [msi, portable] {
        let args = helper_args(4242, exe, &finish);
        assert_eq!(args.first().map(OsString::as_os_str), Some(std::ffi::OsStr::new(FINISH_FLAG)));
        assert_eq!(parse_helper_args(&args), Some((4242, exe.to_path_buf(), finish)));
    }
    let os = |args: &[&str]| args.iter().map(OsString::from).collect::<Vec<_>>();
    for bad in [
        os(&[]),
        os(&["--hidden"]),
        os(&[FINISH_FLAG, "4242", "launcher.exe"]),
        os(&[FINISH_FLAG, "4242", "launcher.exe", "msi"]),
        os(&[FINISH_FLAG, "4242", "launcher.exe", "msi", "a.msi", "extra"]),
        os(&[FINISH_FLAG, "4242", "launcher.exe", "portable", "a.zip"]),
        os(&[FINISH_FLAG, "not a pid", "launcher.exe", "msi", "a.msi"]),
        os(&[FINISH_FLAG, "4242", "launcher.exe", "script", "Remove-Item C:\\"]),
        os(&["--finish", "4242", "launcher.exe", "msi", "a.msi"]),
    ] {
        assert_eq!(parse_helper_args(&bad), None, "{bad:?}");
    }
}

/// The helper finishes only the launcher's own update: its own installation, with the
/// download from its own cache. A signed launcher must not run or start anything else.
#[cfg(windows)]
#[trial]
fn the_update_helper_refuses_anything_but_the_launchers_own_update() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("ArtCraft Launcher");
    std::fs::create_dir_all(&folder).unwrap();
    let exe = folder.join("artcraft-launcher.exe");
    std::fs::write(&exe, "0.1.0").unwrap();
    std::fs::write(folder.join("portable.txt"), "").unwrap();
    let paths = Paths::under(dir.path());
    let cache = paths.cache.clone();
    std::fs::create_dir_all(paths.downloads()).unwrap();
    std::fs::create_dir_all(cache.join(HELPER_DIR)).unwrap();
    let me = cache.join(HELPER_DIR).join("artcraft-launcher.exe");
    std::fs::write(&me, "helper").unwrap();
    let zip = paths.downloads().join("artcraft-launcher-0.2.0-windows-x64-portable.zip");
    std::fs::write(&zip, "zip").unwrap();
    let elsewhere = dir.path().join("elsewhere.zip");
    std::fs::write(&elsewhere, "zip").unwrap();
    let other = dir.path().join("other.exe");
    std::fs::write(&other, "other").unwrap();
    let other_in_folder = folder.join("other.exe");
    std::fs::write(&other_in_folder, "other").unwrap();
    let other_app = folder.join(format!("{}.exe", crate::suite::PHOTOCRAFT.id));
    std::fs::write(&other_app, "another Craft app").unwrap();
    let staging = cache.join(PORTABLE_STAGING);
    let portable = |zip: &Path, tmp: &Path| Finish::Portable { zip: zip.to_path_buf(), tmp: tmp.to_path_buf() };

    assert_eq!(check_helper(&crate::suite::LAUNCHER, &me, &paths, &exe, &portable(&zip, &staging)), Ok(()));
    // A helper that isn't in the installation's cache.
    assert!(check_helper(&crate::suite::LAUNCHER, &other, &paths, &exe, &portable(&zip, &staging)).is_err());
    // Not the launcher's installation.
    assert!(check_helper(&crate::suite::LAUNCHER, &me, &paths, &other, &portable(&zip, &staging)).is_err());
    // A portable marker doesn't make a foreign executable the launcher.
    assert!(check_helper(&crate::suite::LAUNCHER, &me, &paths, &other_in_folder, &portable(&zip, &staging)).is_err());
    // Another Craft App's executable is not the launcher, even in the same folder.
    assert!(check_helper(&crate::suite::LAUNCHER, &me, &paths, &other_app, &portable(&zip, &staging)).is_err());
    // A download from outside the cache.
    assert!(check_helper(&crate::suite::LAUNCHER, &me, &paths, &exe, &portable(&elsewhere, &staging)).is_err());
    // Unpacking anywhere but the cache's staging folder.
    assert!(check_helper(&crate::suite::LAUNCHER, &me, &paths, &exe, &portable(&zip, &dir.path().join(PORTABLE_STAGING))).is_err());
    assert!(check_helper(&crate::suite::LAUNCHER, &me, &paths, &exe, &portable(&zip, &paths.downloads())).is_err());
    // An MSI for a portable installation.
    assert!(check_helper(&crate::suite::LAUNCHER, &me, &paths, &exe, &Finish::Msi(zip.clone())).is_err());
    // Without portable.txt it is a standalone executable, which updates from the same zip.
    std::fs::remove_file(folder.join("portable.txt")).unwrap();
    assert_eq!(check_helper(&crate::suite::LAUNCHER, &me, &paths, &exe, &portable(&zip, &staging)), Ok(()));
    // A foreign executable is not a standalone launcher either.
    assert!(check_helper(&crate::suite::LAUNCHER, &me, &paths, &other_in_folder, &portable(&zip, &staging)).is_err());
    // An MSI is for an MSI installation only.
    assert!(check_helper(&crate::suite::LAUNCHER, &me, &paths, &exe, &Finish::Msi(zip.clone())).is_err());
}

#[cfg(windows)]
fn system_program(name: &str) -> PathBuf {
    PathBuf::from(std::env::var_os("SystemRoot").unwrap()).join("System32").join(name)
}

/// The process id of a process that has already exited.
#[cfg(windows)]
fn exited_pid() -> u32 {
    let mut gone = std::process::Command::new("cmd").args(["/C", "exit"]).spawn().unwrap();
    gone.wait().unwrap();
    gone.id()
}

/// A missing or malformed executable must be rejected before any installed file is changed.
#[cfg(windows)]
#[trial]
fn a_portable_update_requires_a_signed_launcher_even_without_an_mz_header() {
    use std::io::Write;
    for launcher in [None, Some(b"not a Windows program".as_slice()), Some(b"".as_slice())] {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join("ArtCraft Launcher");
        std::fs::create_dir_all(folder.join("ArtCraftLauncherData")).unwrap();
        let exe = folder.join("artcraft-launcher.exe");
        std::fs::write(&exe, "old launcher").unwrap();
        let settings = folder.join("ArtCraftLauncherData/settings.json");
        std::fs::write(&settings, "settings").unwrap();
        let zip = dir.path().join("update.zip");
        let mut writer = zip::ZipWriter::new(std::fs::File::create(&zip).unwrap());
        writer.start_file("launcher/README.md", zip::write::SimpleFileOptions::default()).unwrap();
        writer.write_all(b"new readme").unwrap();
        if let Some(body) = launcher {
            writer.start_file("launcher/artcraft-launcher.exe", zip::write::SimpleFileOptions::default()).unwrap();
            writer.write_all(body).unwrap();
        }
        writer.finish().unwrap();
        let tmp = dir.path().join(PORTABLE_STAGING);
        let step = Finish::Portable { zip: zip.clone(), tmp: tmp.clone() };
        let error =
            run_step(&crate::suite::LAUNCHER, &system_program("cmd.exe"), exited_pid(), &exe, &step, std::time::Duration::from_secs(5)).unwrap_err();
        assert!(error.contains(if launcher.is_some() { "isn't signed" } else { "doesn't contain" }), "{error}");
        assert_eq!(std::fs::read(&exe).unwrap(), b"old launcher");
        assert_eq!(std::fs::read(&settings).unwrap(), b"settings");
        assert!(!folder.join("README.md").exists());
        assert!(!zip.exists() && !tmp.exists());
    }
}

/// A copy that fails part way through puts the previous files back and removes the new ones,
/// so the launcher that starts again is the old one, whole.
#[cfg(windows)]
#[trial]
fn a_portable_update_that_fails_part_way_puts_the_old_files_back() {
    use std::io::Write;
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("ArtCraft Launcher");
    // A folder where the zip has a file: copying it there fails after the exe was replaced.
    std::fs::create_dir_all(folder.join("NOTICE")).unwrap();
    let exe = folder.join("artcraft-launcher.exe");
    std::fs::write(&exe, "0.1.0").unwrap();
    let zip = dir.path().join("artcraft-launcher-0.2.0-windows-x64-portable.zip");
    let mut writer = zip::ZipWriter::new(std::fs::File::create(&zip).unwrap());
    let signed_exe = std::fs::read(system_program("cmd.exe")).unwrap();
    for (name, body) in [
        ("artcraft-launcher-0.2.0-windows-x64-portable/artcraft-launcher.exe", signed_exe.as_slice()),
        ("artcraft-launcher-0.2.0-windows-x64-portable/README.md", b"# ArtCraft Launcher".as_slice()),
        ("artcraft-launcher-0.2.0-windows-x64-portable/NOTICE", b"notice".as_slice()),
    ] {
        writer.start_file(name, zip::write::SimpleFileOptions::default()).unwrap();
        writer.write_all(body).unwrap();
    }
    writer.finish().unwrap();
    let tmp = dir.path().join("cache").join(PORTABLE_STAGING);
    let step = Finish::Portable { zip: zip.clone(), tmp: tmp.clone() };
    let error =
        run_step(&crate::suite::LAUNCHER, &system_program("cmd.exe"), exited_pid(), &exe, &step, std::time::Duration::from_secs(5)).unwrap_err();
    assert!(error.contains("NOTICE"), "{error}");
    assert_eq!(std::fs::read_to_string(&exe).unwrap(), "0.1.0", "the previous exe is put back");
    assert!(!folder.join("README.md").exists(), "a file the update added is removed");
    assert!(!zip.exists() && !tmp.exists());
}

/// A Cargo build tree is not recognised by names: an installed copy in a folder literally
/// called `target` stays an installed copy.
#[trial]
fn an_installed_copy_in_a_folder_called_target_is_not_a_development_build() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("target").join("ArtCraft Launcher");
    std::fs::create_dir_all(&folder).unwrap();
    let exe = folder.join("artcraft-launcher.exe");
    std::fs::write(&exe, "MZ").unwrap();
    assert!(!is_development_build(&exe, false), "a folder named target is not a build tree");
}
