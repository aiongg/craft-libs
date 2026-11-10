//! Tests for the atomic save engine.

use super::*;
use std::cell::Cell;

struct TempDir(PathBuf);
impl TempDir {
    fn new(tag: &str) -> Self {
        static N: AtomicU64 = AtomicU64::new(0);
        let d = std::env::temp_dir().join(format!("craft-atomic-{tag}-{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).ok();
        Self(d)
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o755));
        }
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn assert_no_temp(dir: &Path) {
    let leftovers: Vec<_> = std::fs::read_dir(dir)
        .ok()
        .map(|entries| entries.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).filter(|n| n.ends_with(".craft_tmp")).collect())
        .unwrap_or_default();
    assert!(leftovers.is_empty(), "temporary files left behind: {leftovers:?}");
}

const ORIGINAL: &[u8] = b"the user's precious original document";

#[derive(Default)]
struct Faulty {
    events: std::cell::RefCell<Vec<&'static str>>,
    #[cfg(unix)]
    created_modes: std::cell::RefCell<Vec<u32>>,
    #[cfg(unix)]
    permission_lengths: std::cell::RefCell<Vec<u64>>,
    fail_sync: bool,
    /// Syncs fail once this many have succeeded.
    syncs_before_failure: Option<u32>,
    syncs: Cell<u32>,
    fail_rename: Option<io::ErrorKind>,
    locked_renames: Cell<u32>,
    renames: Cell<u32>,
    no_hard_links: bool,
    fail_permissions: bool,
    permission_error: Option<io::ErrorKind>,
    /// Simulate a network filesystem exposing the same fixed mode on every new file.
    #[cfg(unix)]
    fixed_mode: Option<u32>,
    fail_sync_dir: bool,
    fail_normal_create: bool,
    normal_collisions: Cell<u32>,
    /// Another process creates the target just before the hard link.
    created_meanwhile: Option<&'static [u8]>,
}

impl AtomicFs for Faulty {
    /// Create a real stage with injected failures or a simulated fixed network mode,
    /// recording the creation policy and the Unix mode exposed by the filesystem.
    fn create_new(&self, path: &Path, permissions: NewFilePermissions) -> io::Result<File> {
        let normal = permissions == NewFilePermissions::PlatformDefault;
        self.events.borrow_mut().push(if normal { "create_new_normal" } else { "create_new" });
        if normal && self.fail_normal_create {
            return Err(io::Error::new(io::ErrorKind::PermissionDenied, "injected: normal creation failed"));
        }
        if normal && self.normal_collisions.get() > 0 {
            self.normal_collisions.set(self.normal_collisions.get() - 1);
            return Err(io::Error::from(io::ErrorKind::AlreadyExists));
        }
        let file = RealFs.create_new(path, permissions)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = self.fixed_mode {
                file.set_permissions(std::fs::Permissions::from_mode(mode))?;
            }
            self.created_modes.borrow_mut().push(file.metadata()?.permissions().mode() & 0o777);
        }
        Ok(file)
    }
    fn sync(&self, file: &File) -> io::Result<()> {
        self.events.borrow_mut().push("sync_file");
        let done = self.syncs.replace(self.syncs.get() + 1);
        if self.fail_sync || self.syncs_before_failure.is_some_and(|n| done >= n) {
            return Err(io::Error::new(io::ErrorKind::StorageFull, "injected: no space left on device"));
        }
        RealFs.sync(file)
    }
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.events.borrow_mut().push("rename");
        self.renames.set(self.renames.get() + 1);
        if let Some(kind) = self.fail_rename {
            return Err(io::Error::new(kind, "injected rename failure"));
        }
        if self.locked_renames.get() > 0 {
            self.locked_renames.set(self.locked_renames.get() - 1);
            return Err(io::Error::new(io::ErrorKind::PermissionDenied, "injected: sharing violation"));
        }
        RealFs.rename(from, to)
    }
    fn hard_link(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.events.borrow_mut().push("hard_link");
        if let Some(bytes) = self.created_meanwhile {
            std::fs::write(to, bytes)?;
        }
        if self.no_hard_links {
            return Err(io::Error::new(io::ErrorKind::PermissionDenied, "injected: hard links not supported"));
        }
        RealFs.hard_link(from, to)
    }
    fn remove(&self, path: &Path) -> io::Result<()> {
        self.events.borrow_mut().push("remove");
        RealFs.remove(path)
    }
    fn sync_dir(&self, dir: &Path) -> io::Result<()> {
        self.events.borrow_mut().push("sync_dir");
        if self.fail_sync_dir {
            return Err(io::Error::new(io::ErrorKind::PermissionDenied, "injected: directory sync failed"));
        }
        RealFs.sync_dir(dir)
    }
    /// Record permission restoration and payload length, then inject a configured error
    /// or delegate the mode change to the real filesystem.
    fn set_permissions(&self, file: &File, perms: std::fs::Permissions) -> io::Result<()> {
        self.events.borrow_mut().push("set_permissions");
        #[cfg(unix)]
        self.permission_lengths.borrow_mut().push(file.metadata()?.len());
        if let Some(kind) = self.permission_error {
            return Err(io::Error::new(kind, "injected permission failure"));
        }
        if self.fail_permissions {
            return Err(io::Error::new(io::ErrorKind::PermissionDenied, "injected: operation not permitted"));
        }
        RealFs.set_permissions(file, perms)
    }
}

const NO_RETRY: RenameRetry = RenameRetry { attempts: 0, first_delay_ms: 0 };

#[test]
fn replaces_and_creates() {
    let d = TempDir::new("ok");
    let p = d.0.join("doc.craft");
    atomic_write(&p, b"first").ok();
    assert_eq!(std::fs::read(&p).ok(), Some(b"first".to_vec()));
    atomic_write(&p, b"second, longer").ok();
    assert_eq!(std::fs::read(&p).ok(), Some(b"second, longer".to_vec()));
    assert_no_temp(&d.0);
}

#[test]
fn a_missing_folder_is_an_error_naming_it() {
    let d = TempDir::new("parent");
    let folder = d.0.join("a");
    let p = folder.join("b/out.png");
    for r in [atomic_write(&p, b"x"), write_new(&p, b"x")] {
        let msg = r.err().map(|e| e.to_string()).unwrap_or_default();
        assert!(msg.contains(&folder.join("b").display().to_string()), "{msg}");
    }
    assert!(!folder.exists(), "no folder is created");
}

#[test]
fn failed_flush_keeps_original() {
    let d = TempDir::new("write");
    let p = d.0.join("doc.craft");
    std::fs::write(&p, ORIGINAL).ok();
    let fs = Faulty { fail_sync: true, ..Default::default() };
    let e = atomic_write_custom(&fs, NO_RETRY, &p, &[7u8; 4096]);
    assert_eq!(e.map_err(|e| e.kind()), Err(io::ErrorKind::StorageFull));
    assert_eq!(std::fs::read(&p).ok(), Some(ORIGINAL.to_vec()));
    assert_eq!(fs.renames.get(), 0);
    assert_no_temp(&d.0);
}

#[test]
fn streaming_writes_go_through_the_injected_fs() {
    let d = TempDir::new("stream");
    let p = d.0.join("doc.craft");
    std::fs::write(&p, ORIGINAL).ok();
    let chunks = |f: &mut File| {
        f.write_all(b"chunk1 ")?;
        f.write_all(b"chunk2")
    };
    let fs = Faulty { fail_sync: true, ..Default::default() };
    assert!(replace(&fs, NO_RETRY, &p, chunks).is_err());
    let fs = Faulty { fail_rename: Some(io::ErrorKind::Other), ..Default::default() };
    assert!(replace(&fs, NO_RETRY, &p, chunks).is_err());
    assert_eq!(fs.renames.get(), 1);
    assert_eq!(std::fs::read(&p).ok(), Some(ORIGINAL.to_vec()));
    assert_no_temp(&d.0);
    let fs = Faulty::default();
    assert!(replace(&fs, NO_RETRY, &p, chunks).is_ok());
    assert_eq!(std::fs::read(&p).ok(), Some(b"chunk1 chunk2".to_vec()));
}

#[test]
fn rename_failure_keeps_original() {
    let d = TempDir::new("rename");
    let p = d.0.join("doc.craft");
    std::fs::write(&p, ORIGINAL).ok();
    let fs = Faulty { fail_rename: Some(io::ErrorKind::Other), ..Default::default() };
    assert!(atomic_write_custom(&fs, NO_RETRY, &p, b"new").is_err());
    assert_eq!(std::fs::read(&p).ok(), Some(ORIGINAL.to_vec()));
    assert_no_temp(&d.0);
}

#[test]
fn locked_target_rename_is_retried() {
    let d = TempDir::new("locked");
    let p = d.0.join("doc.craft");
    std::fs::write(&p, ORIGINAL).ok();
    let fs = Faulty { locked_renames: Cell::new(2), ..Default::default() };
    atomic_write_custom(&fs, RenameRetry { attempts: 3, first_delay_ms: 1 }, &p, b"new").ok();
    assert_eq!(std::fs::read(&p).ok(), Some(b"new".to_vec()));
    assert_eq!(fs.renames.get(), 3);
    assert_no_temp(&d.0);
}

#[test]
fn options_inject_policy_failures_and_retry_budget() -> io::Result<()> {
    let d = TempDir::new("options-injection");
    for policy in [NewFilePermissions::Private, NewFilePermissions::PlatformDefault] {
        let options = WriteOptions::default().with_new_file_permissions(policy);
        for streaming in [false, true] {
            let path = d.0.join(format!("{policy:?}-{streaming}"));
            let fs = Faulty { locked_renames: Cell::new(1), ..Default::default() };
            let retry = RenameRetry { attempts: 1, first_delay_ms: 0 };
            if streaming {
                options.atomic_write_with_custom(&fs, retry, &path, |f| f.write_all(b"payload"))?;
            } else {
                options.atomic_write_custom(&fs, retry, &path, b"payload")?;
            }
            assert_eq!(fs.renames.get(), 2);
            assert_eq!(std::fs::read(&path)?, b"payload");
            let event = if policy == NewFilePermissions::Private { "create_new" } else { "create_new_normal" };
            assert_eq!(fs.events.borrow()[0], event);
            std::fs::remove_file(&path)?;
            let fs = Faulty { fail_sync: true, ..Default::default() };
            let result = if streaming {
                options.write_new_with_custom(&fs, &path, |f| f.write_all(b"payload"))
            } else {
                options.write_new_custom(&fs, &path, b"payload")
            };
            assert_eq!(result.err().map(|e| e.kind()), Some(io::ErrorKind::StorageFull));
            assert_eq!(fs.events.borrow()[0], event);
            assert!(!path.exists());
            assert_no_temp(&d.0);
        }
    }
    Ok(())
}

#[test]
fn write_new_never_overwrites_existing() {
    let d = TempDir::new("new");
    let p = d.0.join("file.craft");
    write_new(&p, b"first").ok();
    assert_eq!(std::fs::read(&p).ok(), Some(b"first".to_vec()));
    let res = write_new(&p, b"second");
    assert!(res.is_err());
    assert_eq!(std::fs::read(&p).ok(), Some(b"first".to_vec()));
    assert_no_temp(&d.0);
}

#[test]
fn write_new_copies_where_hard_links_fail_and_never_replaces() {
    let d = TempDir::new("new-copy");
    let p = d.0.join("export.png");
    let fs = Faulty { no_hard_links: true, ..Default::default() };
    assert!(create(&fs, &p, |f| f.write_all(b"copied")).is_ok());
    assert_eq!(std::fs::read(&p).ok(), Some(b"copied".to_vec()));
    assert_no_temp(&d.0);

    // A file that appears after the existence check is kept, whether the copy or the hard
    // link finds it.
    for no_hard_links in [true, false] {
        let p = d.0.join(format!("raced-{no_hard_links}.png"));
        let fs = Faulty { no_hard_links, created_meanwhile: Some(ORIGINAL), ..Default::default() };
        let e = create(&fs, &p, |f| f.write_all(b"mine"));
        assert_eq!(e.map_err(|e| e.kind()), Err(io::ErrorKind::AlreadyExists));
        assert_eq!(std::fs::read(&p).ok(), Some(ORIGINAL.to_vec()));
        assert_eq!(fs.renames.get(), 0);
    }
    assert_no_temp(&d.0);

    // A failed new file has no original to speak of.
    let p = d.0.join("full.png");
    let fs = Faulty { fail_sync: true, ..Default::default() };
    let msg = create(&fs, &p, |f| f.write_all(b"x")).err().map(|e| e.to_string()).unwrap_or_default();
    assert!(msg.contains("disk is full") && !msg.contains("original"), "{msg}");
    assert!(!p.exists());
    assert_no_temp(&d.0);
}

#[test]
fn a_failed_copy_leaves_no_partial_file() {
    let d = TempDir::new("new-copy-fails");
    let p = d.0.join("export.png");
    // The temporary file flushes; the copy made in its place does not.
    let fs = Faulty { no_hard_links: true, syncs_before_failure: Some(1), ..Default::default() };
    let e = write_new_custom(&fs, &p, b"copied");
    assert_eq!(e.map_err(|e| e.kind()), Err(io::ErrorKind::StorageFull));
    assert_eq!(fs.syncs.get(), 2);
    assert!(!p.exists());
    assert_no_temp(&d.0);
}

#[cfg(unix)]
#[test]
fn exclusive_copy_starts_private_and_keeps_callback_mode() -> io::Result<()> {
    use std::fs::Permissions;
    use std::os::unix::fs::PermissionsExt;

    let d = TempDir::new("private-copy");
    for mode in [0o600, 0o640] {
        let p = d.0.join(format!("export-{mode:o}.craft"));
        let fs = Faulty { no_hard_links: true, ..Default::default() };
        create_with_options(&fs, &p, NewFilePermissions::Private, |f| {
            f.write_all(b"private export")?;
            f.set_permissions(Permissions::from_mode(mode))
        })?;
        // Check both the staging file and the exclusive-copy destination at creation.
        assert_eq!(*fs.created_modes.borrow(), [0o600, 0o600]);
        assert_eq!(std::fs::metadata(&p)?.permissions().mode() & 0o777, mode);
        assert_eq!(std::fs::read(&p)?, b"private export");
    }
    assert_no_temp(&d.0);
    Ok(())
}

/// Matching fixed modes permit byte and streaming replacements under both creation
/// policies without calling a permission operation that the mount rejects.
#[cfg(unix)]
#[test]
fn unchanged_replacement_modes_need_no_chmod() -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let d = TempDir::new("fixed-replacement-mode");
    // GVfs SMB reports 0700 regardless of the requested creation mode; also cover a
    // private stage whose normal 0600 mode already matches the original.
    for mode in [0o600, 0o700] {
        for policy in [NewFilePermissions::Private, NewFilePermissions::PlatformDefault] {
            for streaming in [false, true] {
                let p = d.0.join(format!("{mode:o}-{policy:?}-{streaming}"));
                std::fs::write(&p, ORIGINAL)?;
                std::fs::set_permissions(&p, std::fs::Permissions::from_mode(mode))?;
                let fs = Faulty { fixed_mode: Some(mode), permission_error: Some(io::ErrorKind::Unsupported), ..Default::default() };
                let options = WriteOptions::default().with_new_file_permissions(policy);
                if streaming {
                    options.atomic_write_with_custom(&fs, NO_RETRY, &p, |f| f.write_all(b"replacement"))?;
                } else {
                    options.atomic_write_custom(&fs, NO_RETRY, &p, b"replacement")?;
                }
                assert_eq!(std::fs::read(&p)?, b"replacement");
                assert_eq!(std::fs::metadata(&p)?.permissions().mode() & 0o7777, mode);
                assert!(!fs.events.borrow().contains(&"set_permissions"));
                assert_eq!(fs.renames.get(), 1);
                assert_no_temp(&d.0);
            }
        }
    }
    Ok(())
}

/// A required mode change that fails must preserve the original bytes and mode,
/// report the error, and remove the stage without publishing it.
#[cfg(unix)]
#[test]
fn failing_to_keep_permissions_fails_the_save() -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let d = TempDir::new("perms");
    for kind in [io::ErrorKind::Unsupported, io::ErrorKind::PermissionDenied, io::ErrorKind::Other] {
        let p = d.0.join(format!("{kind:?}.craft"));
        std::fs::write(&p, ORIGINAL)?;
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o640))?;
        let fs = Faulty { permission_error: Some(kind), ..Default::default() };
        let error = atomic_write_custom(&fs, NO_RETRY, &p, b"new").err().ok_or_else(|| io::Error::other("a required mode change must succeed"))?;
        assert_eq!(error.kind(), kind);
        assert!(error.to_string().contains("permissions"), "{error}");
        assert_eq!(std::fs::read(&p)?, ORIGINAL);
        assert_eq!(std::fs::metadata(&p)?.permissions().mode() & 0o7777, 0o640);
        assert_eq!(fs.renames.get(), 0);
        assert_no_temp(&d.0);
    }
    Ok(())
}

/// A streaming callback that changes permissions or setuid/setgid/sticky bits must not
/// bypass restoration of the original mode, even when the modes matched at creation.
#[cfg(unix)]
#[test]
fn replacement_checks_mode_after_streaming_callback() -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let d = TempDir::new("callback-mode-change");
    for callback_mode in [0o640, 0o1600, 0o2600, 0o4600] {
        let p = d.0.join(format!("doc-{callback_mode:o}.craft"));
        std::fs::write(&p, ORIGINAL)?;
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o600))?;
        let fs = Faulty::default();
        WriteOptions::default().atomic_write_with_custom(&fs, NO_RETRY, &p, |f| {
            f.write_all(b"replacement")?;
            f.set_permissions(std::fs::Permissions::from_mode(callback_mode))?;
            assert_eq!(f.metadata()?.permissions().mode() & 0o7777, callback_mode);
            Ok(())
        })?;
        assert_eq!(std::fs::read(&p)?, b"replacement");
        assert_eq!(std::fs::metadata(&p)?.permissions().mode() & 0o7777, 0o600);
        assert!(fs.events.borrow().contains(&"set_permissions"));
        assert_no_temp(&d.0);
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn normal_creation_needs_no_chmod_even_when_hard_links_fail() -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    use std::process::Command;
    if std::env::var_os("CRAFT_ATOMIC_COPY_CHILD").is_none() {
        // Test both publication paths under real umasks without changing the test runner's
        // process-global mask. In particular, 002 must produce a shared 0664 final file.
        for mask in ["000", "002", "022", "027", "077"] {
            let output = Command::new("sh")
                .args([
                    "-c",
                    "umask \"$1\"; exec \"$2\" --exact tests::normal_creation_needs_no_chmod_even_when_hard_links_fail --nocapture",
                    "copy-permissions",
                    mask,
                ])
                .arg(std::env::current_exe()?)
                .env("CRAFT_ATOMIC_COPY_CHILD", "1")
                .output()?;
            assert!(output.status.success(), "umask {mask}: {}", String::from_utf8_lossy(&output.stdout));
            assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed"));
        }
        return Ok(());
    }
    let d = TempDir::new("normal-copy");
    let reference = d.0.join("reference");
    std::fs::write(&reference, b"normal creation")?;
    let expected = std::fs::metadata(reference)?.permissions().mode() & 0o777;
    for no_hard_links in [false, true] {
        let p = d.0.join(format!("export-{no_hard_links}"));
        let fs = Faulty { no_hard_links, normal_collisions: Cell::new(2), fail_permissions: true, ..Default::default() };
        create_with_options(&fs, &p, NewFilePermissions::PlatformDefault, |f| {
            assert_eq!(f.metadata()?.permissions().mode() & 0o777, expected);
            f.write_all(b"document payload")
        })?;
        assert_eq!(std::fs::metadata(&p)?.permissions().mode() & 0o777, expected);
        assert_eq!(std::fs::read(&p)?, b"document payload");
        assert!(fs.created_modes.borrow().iter().all(|m| *m == expected), "stage and copy match ordinary creation");
        assert!(!fs.events.borrow().contains(&"set_permissions"));
    }
    let p = d.0.join("new-document");
    let fs = Faulty { fail_permissions: true, ..Default::default() };
    atomic_write_custom(&fs, NO_RETRY, &p, b"document")?;
    assert_eq!(std::fs::metadata(&p)?.permissions().mode() & 0o777, expected);
    assert!(!fs.events.borrow().contains(&"set_permissions"));
    assert_no_temp(&d.0);
    Ok(())
}

#[cfg(unix)]
#[test]
fn normal_policy_preserves_existing_modes_with_private_staging() -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let d = TempDir::new("normal-existing");
    for mode in [0o600, 0o640, 0o664] {
        let p = d.0.join(format!("existing-{mode:o}"));
        std::fs::write(&p, ORIGINAL)?;
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(mode))?;
        let fs = Faulty { fail_normal_create: true, ..Default::default() };
        replace_with_options(&fs, NO_RETRY, &p, NewFilePermissions::PlatformDefault, |f| f.write_all(b"replacement"))?;
        assert_eq!(std::fs::metadata(&p)?.permissions().mode() & 0o777, mode);
        assert!(!fs.events.borrow().contains(&"create_new_normal"));
        assert_eq!(*fs.created_modes.borrow(), [0o600]);
    }
    assert_no_temp(&d.0);
    Ok(())
}

#[cfg(unix)]
#[test]
fn normal_permission_failures_abort_before_publication() -> io::Result<()> {
    let d = TempDir::new("normal-failures");
    for exclusive in [false, true] {
        for (tag, fs, kind, callback_runs) in [
            ("create", Faulty { fail_normal_create: true, ..Default::default() }, io::ErrorKind::PermissionDenied, false),
            ("collisions", Faulty { normal_collisions: Cell::new(8), ..Default::default() }, io::ErrorKind::AlreadyExists, false),
            ("sync", Faulty { fail_sync: true, ..Default::default() }, io::ErrorKind::StorageFull, true),
        ] {
            let p = d.0.join(format!("{tag}-{exclusive}"));
            let called = Cell::new(false);
            let fill = |f: &mut File| {
                called.set(true);
                f.write_all(b"payload")
            };
            let result = if exclusive {
                create_with_options(&fs, &p, NewFilePermissions::PlatformDefault, fill)
            } else {
                replace_with_options(&fs, NO_RETRY, &p, NewFilePermissions::PlatformDefault, fill)
            };
            assert_eq!(result.map_err(|e| e.kind()), Err(kind), "{tag}");
            assert_eq!(called.get(), callback_runs, "{tag}");
            assert!(!p.exists(), "{tag}");
            assert_no_temp(&d.0);
        }
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn normal_copy_preserves_callback_modes_and_refuses_permission_failures() -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let d = TempDir::new("normal-callback-copy");
    for mode in [0o600, 0o640, 0o660] {
        for no_hard_links in [false, true] {
            let p = d.0.join(format!("{mode:o}-{no_hard_links}"));
            let fs = Faulty { no_hard_links, ..Default::default() };
            create(&fs, &p, |f| {
                f.set_permissions(std::fs::Permissions::from_mode(mode))?;
                f.write_all(b"callback payload")
            })?;
            assert_eq!(std::fs::metadata(&p)?.permissions().mode() & 0o777, mode);
            assert_eq!(std::fs::read(&p)?, b"callback payload");
            assert!(fs.permission_lengths.borrow().iter().all(|len| *len == 0), "adjust normal copies before exposing any payload");
        }
    }
    let p = d.0.join("permission-failure");
    let fs = Faulty { no_hard_links: true, fail_permissions: true, ..Default::default() };
    let result = create(&fs, &p, |f| {
        // Toggle a bit from ordinary creation so the fallback really needs a mode change,
        // independently of the test runner's umask.
        let mode = f.metadata()?.permissions().mode() ^ 0o040;
        f.set_permissions(std::fs::Permissions::from_mode(mode))?;
        f.write_all(b"must not survive")
    });
    assert_eq!(result.map_err(|e| e.kind()), Err(io::ErrorKind::PermissionDenied));
    assert_eq!(*fs.permission_lengths.borrow(), [0], "refuse before copying payload");
    assert!(!p.exists());
    assert_no_temp(&d.0);
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
#[ignore = "requires setfacl/getfacl and a filesystem with POSIX ACL support"]
fn normal_permissions_follow_inherited_acl_with_hard_links_and_copy() -> io::Result<()> {
    use std::process::Command;
    let d = TempDir::new("normal-acl");
    let output = Command::new("setfacl").args(["-m", "d:u::rwx,d:u:12345:r--,d:g::r-x,d:m::r-x,d:o::---"]).arg(&d.0).output()?;
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let acl = |path: &Path| -> io::Result<Vec<u8>> {
        let output = Command::new("getfacl").args(["-c", "-n"]).arg(path).output()?;
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        Ok(output.stdout)
    };
    let reference = d.0.join("reference");
    std::fs::write(&reference, b"normal creation")?;
    let expected = acl(&reference)?;
    for no_hard_links in [false, true] {
        let fs = Faulty { no_hard_links, fail_permissions: true, ..Default::default() };
        let p = d.0.join(format!("export-{no_hard_links}"));
        create_with_options(&fs, &p, NewFilePermissions::PlatformDefault, |f| f.write_all(b"export"))?;
        assert_eq!(acl(&p)?, expected, "same inherited ACL as ordinary creation");
        assert_eq!(std::fs::read(&p)?, b"export");
    }
    assert_no_temp(&d.0);
    Ok(())
}

#[test]
fn retry_delays_are_capped() {
    let d = TempDir::new("cap");
    let p = d.0.join("doc.craft");
    let fs = Faulty { locked_renames: Cell::new(2), ..Default::default() };
    let started = std::time::Instant::now();
    assert!(atomic_write_custom(&fs, RenameRetry { attempts: 2, first_delay_ms: 60_000 }, &p, b"new").is_ok());
    assert!(started.elapsed() < std::time::Duration::from_secs(5));
    assert_eq!(fs.renames.get(), 3);
}

#[test]
fn distinguishes_atomicity_and_durability_order() {
    let d = TempDir::new("durability_order");
    let p = d.0.join("doc.craft");
    std::fs::write(&p, ORIGINAL).ok();

    let fs = Faulty::default();
    atomic_write_custom(&fs, NO_RETRY, &p, b"durable payload").expect("successful atomic write");

    let events = fs.events.into_inner();
    let sync_file_idx = events.iter().position(|&e| e == "sync_file").expect("file content must be flushed");
    let rename_idx = events.iter().position(|&e| e == "rename").expect("atomic rename must be performed");

    // 1. Content Durability: staging file must be synced to stable media BEFORE pointer switch (rename)
    assert!(
        sync_file_idx < rename_idx,
        "Durability invariant violated: file content must be flushed to stable storage before rename (sync={sync_file_idx}, rename={rename_idx})"
    );

    // 2. Directory Durability: on Unix, parent directory must be synced AFTER rename to commit directory entry
    #[cfg(unix)]
    {
        let sync_dir_idx = events.iter().position(|&e| e == "sync_dir").expect("directory must be flushed on Unix");
        assert!(
            rename_idx < sync_dir_idx,
            "Durability invariant violated: parent directory must be flushed after rename (rename={rename_idx}, sync_dir={sync_dir_idx})"
        );
    }
}

#[test]
fn sync_dir_failure_is_best_effort() {
    let d = TempDir::new("best_effort_sync_dir");
    let p = d.0.join("doc.craft");
    let fs = Faulty { fail_sync_dir: true, ..Default::default() };
    assert!(atomic_write_custom(&fs, NO_RETRY, &p, b"best effort data").is_ok());
    assert_eq!(std::fs::read(&p).ok(), Some(b"best effort data".to_vec()));
    assert_no_temp(&d.0);
}
