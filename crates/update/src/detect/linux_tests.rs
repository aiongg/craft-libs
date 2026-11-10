//! Tests for Linux detection.

use super::*;

#[test]
fn ours_filter_matches_rpm_packages() {
    let ours = |name: &str| {
        crate::suite::SUITE.iter().any(|a| {
            [InstallSource::Deb, InstallSource::Rpm, InstallSource::Pacman, InstallSource::Flatpak]
                .iter()
                .any(|m| crate::packages::candidate_names(a, *m).iter().any(|n| n == name))
        })
    };
    assert!(ours("photocraft"));
    assert!(ours("ai.storyteller.photocraft"));
    assert!(!ours("unrelated-package"));
}

#[test]
fn pacman_is_only_queried_where_its_database_exists() {
    let root = tempfile::tempdir().unwrap();
    assert!(!linux::pacman_db_present(&root.path().join("local")));
    std::fs::create_dir(root.path().join("local")).unwrap();
    assert!(linux::pacman_db_present(&root.path().join("local")));
}

/// `linux::binary_version` on a script the test just wrote. Another test thread that forks
/// while the script is still open for writing holds that descriptor until it execs, and running
/// the script in that window fails with ETXTBSY ("Text file busy"), so a miss is retried.
fn binary_version_of_new_script(bin: &Path, app: &App) -> Option<version::Version> {
    for _ in 0..20 {
        if let Some(v) = linux::binary_version(bin, app) {
            return Some(v);
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    None
}

#[test]
fn reads_binary_version_from_cli() {
    let photo = App::by_id("photocraft").unwrap();
    let temp_dir = tempfile::tempdir().unwrap();
    let bin = temp_dir.path().join("photocraft");
    std::fs::write(&bin, "#!/bin/sh\necho 'photocraft 0.6.0 (0c72d9542, 2026-10-10)'\n").unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    let ver = binary_version_of_new_script(&bin, photo);
    assert_eq!(ver.unwrap().to_string(), "0.6.0");
}

#[test]
fn reads_binary_version_when_the_cli_uses_the_display_name_or_a_former_id() {
    let temp_dir = tempfile::tempdir().unwrap();
    use std::os::unix::fs::PermissionsExt;
    let write = |name: &str, script: &str| {
        let bin = temp_dir.path().join(name);
        std::fs::write(&bin, script).unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        bin
    };
    let sound = write("soundcraft", "#!/bin/sh\necho 'SoundCraft 0.5.0'\n");
    assert_eq!(binary_version_of_new_script(&sound, App::by_id("soundcraft").unwrap()).unwrap().to_string(), "0.5.0");
    let pdf = write("pdfcraft", "#!/bin/sh\necho 'printcraft 0.2.1'\n");
    assert_eq!(binary_version_of_new_script(&pdf, App::by_id("pdfcraft").unwrap()).unwrap().to_string(), "0.2.1");
}
