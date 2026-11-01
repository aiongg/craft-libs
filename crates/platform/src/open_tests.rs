//! `craft_platform::open`.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use craft_host::Wake;

use crate::open::{DropTarget, DroppedFile, FileIntake, Intake, path_text, same_file};

// Equal paths match without touching the disk; a symlink matches its target through the
// canonical path.
#[test]
fn same_file_resolves_only_when_needed() {
    let missing = Path::new("/definitely/not/here.psd");
    assert!(same_file(missing, missing, None));
    assert!(!same_file(missing, Path::new("/other.psd"), None));

    let dir = std::env::temp_dir().join(format!("craft-open-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let target = dir.join("target.txt");
    std::fs::write(&target, b"x").unwrap();
    let canonical = std::fs::canonicalize(&target).unwrap();
    #[cfg(unix)]
    {
        let link = dir.join("link.txt");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert!(same_file(&link, &target, Some(&canonical)));
    }
    assert!(!same_file(&dir.join("gone.txt"), &target, Some(&canonical)));
    std::fs::remove_dir_all(&dir).unwrap();
}

// OS open requests from another thread and drops arrive in order, Open batches sorted naturally,
// and the wake callback runs once per request.
#[test]
fn intake_keeps_arrival_order_and_wakes() {
    let wakes = Arc::new(AtomicUsize::new(0));
    let counter = wakes.clone();
    let (intake, sender) = FileIntake::new(Wake::new(move || {
        counter.fetch_add(1, Ordering::SeqCst);
    }));
    let drop = DroppedFile { path: None, name: "a.png".into(), bytes: None };
    intake.dropped(Vec::new(), DropTarget::Canvas);
    intake.dropped(vec![drop.clone()], DropTarget::Tabs(2));
    std::thread::spawn(move || sender.open(vec![PathBuf::from("/x/File_10.psd"), PathBuf::from("/x/File_2.psd")])).join().unwrap();
    assert_eq!(wakes.load(Ordering::SeqCst), 1);

    let batches = intake.take();
    assert_eq!(
        batches,
        [
            Intake::Dropped { files: vec![drop], target: DropTarget::Tabs(2) },
            Intake::Open(vec![PathBuf::from("/x/File_2.psd"), PathBuf::from("/x/File_10.psd")]),
        ]
    );
    assert!(intake.take().is_empty());
}

#[cfg(unix)]
#[test]
fn path_text_names_a_non_unicode_path() {
    use std::os::unix::ffi::OsStrExt as _;
    let path = Path::new(std::ffi::OsStr::from_bytes(b"/x/\xff.psd"));
    assert!(path_text(path, false).unwrap_err().contains("open it again"));
    assert!(path_text(path, true).unwrap_err().contains("save under another name"));
    assert_eq!(path_text(Path::new("/x/a.psd"), false).unwrap(), "/x/a.psd");
}

// The target is one argument after `--` (or the Windows handler), so no shell reads a `&` and no
// tool reads a leading `-` as an option.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn open_command_keeps_the_target_one_inert_argument() {
    let target = "https://x.test/?a=1&calc";
    let command = crate::open::open_command(target);
    let args: Vec<_> = command.get_args().map(|a| a.to_string_lossy().into_owned()).collect();
    assert_ne!(command.get_program(), "cmd");
    assert_eq!(args.last().map(String::as_str), Some(target));
    if !cfg!(windows) {
        assert_eq!(args, ["--", target]);
    }
}
