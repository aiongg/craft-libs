//! The Windows update helper, its arguments, the failure file and the pre-update checks.

use super::detect::marker;
#[cfg(windows)]
use super::detect::{clear_pretend_version, exe_after_msi, msi_step, restarts};
use super::*;

/// The first argument of the Windows update helper: a copy of this program that finishes an
/// update once the launcher has exited (see [`finish`]).
pub const FINISH_FLAG: &str = "--finish-update";

/// What the update helper does once the old launcher has exited.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(not(windows), allow(dead_code))]
pub(super) enum Finish {
    /// Run the new MSI, which reinstalls into the same folder.
    Msi(PathBuf),
    /// Unpack the new portable zip into `tmp` and copy it over the launcher's folder.
    Portable { zip: PathBuf, tmp: PathBuf },
}

/// The helper's arguments: [`FINISH_FLAG`], the process id to wait for, the exe to start
/// afterwards, then the step.
#[cfg_attr(not(windows), allow(dead_code))]
pub(super) fn helper_args(pid: u32, exe: &Path, finish: &Finish) -> Vec<OsString> {
    let mut args: Vec<OsString> = vec![FINISH_FLAG.into(), pid.to_string().into(), exe.into()];
    match finish {
        Finish::Msi(msi) => args.extend(["msi".into(), msi.into()]),
        Finish::Portable { zip, tmp } => args.extend(["portable".into(), zip.into(), tmp.into()]),
    }
    args
}

/// Reads [`helper_args`] back.
#[cfg_attr(not(windows), allow(dead_code))]
pub(super) fn parse_helper_args(args: &[OsString]) -> Option<(u32, PathBuf, Finish)> {
    let [flag, pid, exe, step @ ..] = args else { return None };
    if flag != FINISH_FLAG {
        return None;
    }
    let pid = pid.to_str()?.parse().ok()?;
    let finish = match step {
        [kind, msi] if kind == "msi" => Finish::Msi(msi.into()),
        [kind, zip, tmp] if kind == "portable" => Finish::Portable { zip: zip.into(), tmp: tmp.into() },
        _ => return None,
    };
    Some((pid, exe.into(), finish))
}

#[cfg_attr(not(windows), allow(dead_code))]
pub(super) fn msi_finish(file: &Path) -> Result<Finish, String> {
    install::msi_path(file).map(Finish::Msi)
}

/// The cache folder the update helper is copied into.
#[cfg_attr(not(windows), allow(dead_code))]
pub(super) const HELPER_DIR: &str = "launcher-update-helper";
/// The cache folder a portable update is unpacked into.
#[cfg_attr(not(windows), allow(dead_code))]
pub(super) const PORTABLE_STAGING: &str = "launcher-update";

/// Copies this program into the cache and starts the copy as the update helper, which waits for
/// `pid` to exit, does `finish` and starts `exe`. Run from the cache, the helper doesn't hold the
/// installed exe that the step replaces.
#[cfg(windows)]
pub(super) fn windows_helper(app: &App, paths: &Paths, pid: u32, exe: &Path, finish: &Finish) -> Result<(), String> {
    let program = std::env::current_exe().map_err(|e| format!("couldn't find this program ({e})"))?;
    let dir = paths.cache.join(HELPER_DIR);
    // The previous update's copy, when it's still there.
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(|e| format!("couldn't schedule the update ({e})"))?;
    let helper = dir.join(program.file_name().ok_or("this program has no file name")?);
    std::fs::copy(&program, &helper).map_err(|e| format!("couldn't schedule the update ({e})"))?;
    let mut cmd = process::command(&helper);
    cmd.args(helper_args(pid, exe, finish));
    clear_pretend_version(&mut cmd, app);
    process::spawn_detached(cmd).map_err(|e| format!("couldn't schedule the update ({e})"))
}

/// Finishes an update when this program was started as the update helper; `args` are its
/// arguments without the program name. `None` when they don't start with [`FINISH_FLAG`]. The
/// app is started again even when the step fails: the old copy is then still in place.
#[cfg(windows)]
pub fn finish(app: &App, args: &[OsString]) -> Option<Result<(), String>> {
    if args.first().is_none_or(|a| a != FINISH_FLAG) {
        return None;
    }
    let Some((pid, exe, step)) = parse_helper_args(args) else { return Some(Err("the update helper's arguments are incomplete".into())) };
    let Ok(me) = std::env::current_exe() else { return Some(Err("the update helper couldn't find itself".into())) };
    let paths = match Paths::for_exe(app, &exe) {
        Ok(paths) => paths,
        Err(e) => return Some(Err(e)),
    };
    // Nothing runs unless this is the app's own update, signed like this helper.
    if let Err(e) = check_helper(app, &me, &paths, &exe, &step).and_then(|()| signed_like(app, &me, &exe)) {
        record_failure(&paths, &e);
        return Some(Err(e));
    }
    // Read before the step: the update may move or replace the old exe.
    let registered =
        matches!(step, Finish::Msi(_)) && matches!(crate::msi_copy::windows::find(app, &exe), Some(crate::msi_copy::MsiCopy::Registered { .. }));
    let done = run_step(app, &me, pid, &exe, &step, std::time::Duration::from_secs(60));
    if let Err(e) = &done {
        record_failure(&paths, e);
    }
    // The new app, or the old one when the step failed.
    let exe = if done.is_ok() { exe_after_msi(app, registered, &exe) } else { exe };
    // A failed move can leave no program to start; its recorded error already says how to get
    // the app back, and a restart error must not replace it.
    if !restarts(done.is_ok(), exe.is_file()) {
        return Some(done);
    }
    let started = signed_like(app, &me, &exe)
        .and_then(|()| process::spawn_detached(std::process::Command::new(&exe)).map_err(|e| format!("couldn't start {} ({e})", exe.display())));
    if let Err(e) = &started {
        record_failure(&paths, e);
    }
    Some(done.and(started))
}

/// Finishes an update on non-Windows platforms (no-op helper).
#[cfg(not(windows))]
pub fn finish(_app: &App, _args: &[OsString]) -> Option<Result<(), String>> {
    None
}

/// Where a failed update's error waits for the next start.
pub(super) fn failure_file(paths: &Paths) -> PathBuf {
    paths.logs().join("self-update-error.txt")
}

/// Records a failed update: the next check offers the update again, and the next start logs `error`.
#[cfg_attr(not(windows), allow(dead_code))]
pub(super) fn record_failure(paths: &Paths, error: &str) {
    let _ = std::fs::remove_file(marker(paths));
    let _ = std::fs::create_dir_all(paths.logs());
    let _ = crate::paths::write_atomic(&failure_file(paths), error.as_bytes());
}

/// The error of the last update, if it failed, once.
pub fn take_failure(paths: &Paths) -> Option<String> {
    let file = failure_file(paths);
    let error = std::fs::read_to_string(&file).ok()?;
    let _ = std::fs::remove_file(&file);
    Some(error)
}

/// Whether `exe` is named like this app's own executable (case-insensitive, any of its ids).
#[cfg(windows)]
pub(super) fn is_own_executable(app: &App, exe: &Path) -> bool {
    exe.file_stem().and_then(|stem| stem.to_str()).is_some_and(|stem| app.stems().any(|id| id.eq_ignore_ascii_case(stem)))
}

/// Refuses an executable that isn't this app's own, as the helper would.
#[cfg(windows)]
pub(super) fn own_executable(app: &App, exe: &Path) -> Result<(), String> {
    if is_own_executable(app, exe) { Ok(()) } else { Err(format!("the update helper only finishes {}'s own update", app.name)) }
}

/// Checks, while the app still runs and can show the error, what the helper checks before
/// it installs: this program, the installed exe and the new MSI are signed by one publisher, and
/// the helper finds the cache in `paths` that holds its copy, the download and the staging folder.
#[cfg(windows)]
pub(super) fn windows_precheck(app: &App, install: &Install, paths: &Paths, download: &Path) -> Result<(), String> {
    let me = std::env::current_exe().map_err(|e| format!("couldn't find this program ({e})"))?;
    match install {
        Install::Msi(exe) => signed_like(app, &me, exe)
            .and_then(|()| msi_step(app, exe).map(drop))
            .and_then(|()| signed_like(app, &me, download))
            .and_then(|()| own_executable(app, exe))
            .and_then(|()| same_cache_as_helper(app, paths, exe)),
        Install::Portable(exe) | Install::Standalone(exe) => {
            signed_like(app, &me, exe).and_then(|()| own_executable(app, exe)).and_then(|()| same_cache_as_helper(app, paths, exe))
        }
        _ => Ok(()),
    }
}

/// Accepts `paths` when the update helper, which finds the app's folders again from `exe`
/// ([`Paths::for_exe`]), finds the same cache. Refuses an app that keeps its files in the
/// temporary folder because its own folders couldn't be found.
#[cfg(windows)]
pub(super) fn same_cache_as_helper(app: &App, paths: &Paths, exe: &Path) -> Result<(), String> {
    let canonical = |p: &Path| p.canonicalize().ok();
    let theirs = Paths::for_exe(app, exe).ok().and_then(|helper| canonical(&helper.cache));
    if theirs.is_some() && theirs == canonical(&paths.cache) {
        Ok(())
    } else {
        Err(format!("{} can't update itself while it keeps its files in {}; download the new version instead", app.name, paths.cache.display()))
    }
}

/// Accepts `file` when it carries a valid Authenticode signature from the same publisher as
/// `reference`, this program. Refuses an unsigned file, another publisher's, and every file when
/// `reference` itself isn't signed.
#[cfg(windows)]
pub(super) fn signed_like(app: &App, reference: &Path, file: &Path) -> Result<(), String> {
    let publisher = |path: &Path| verifysign::verify_file(path).map(|signature| signature.subject_name());
    let ours = publisher(reference).map_err(|e| format!("this copy of {} isn't signed ({e}), so it doesn't install updates", app.name))?;
    let theirs = publisher(file).map_err(|e| format!("{} isn't signed ({e})", file.display()))?;
    if ours == theirs { Ok(()) } else { Err(format!("{} isn't signed by {}'s publisher", file.display(), app.name)) }
}

/// Whether `file` is a Windows program or library (it starts with the `MZ` header), whatever its
/// name.
#[cfg(windows)]
pub(super) fn is_program(file: &Path) -> bool {
    use std::io::Read;
    let mut magic = [0u8; 2];
    std::fs::File::open(file).and_then(|mut f| f.read_exact(&mut magic)).is_ok() && magic == *b"MZ"
}

/// Allows only the app's own update: `exe` is this app's MSI or portable installation,
/// of the kind `step` updates, and the helper (`me`), the download and the staging folder are in
/// that installation's cache. Anything else is refused before the helper runs or starts a file.
#[cfg(windows)]
pub(super) fn check_helper(app: &App, me: &Path, paths: &Paths, exe: &Path, step: &Finish) -> Result<(), String> {
    let refused = || format!("the update helper only finishes {}'s own update", app.name);
    let canonical = |p: &Path| p.canonicalize().map_err(|_| refused());
    let cache = canonical(&paths.cache)?;
    if canonical(me.parent().ok_or_else(refused)?)? != cache.join(HELPER_DIR) {
        return Err(refused());
    }
    let exe = canonical(exe)?;
    // Whatever the folder, the program to restart is this app's own, never another executable.
    if !is_own_executable(app, &exe) {
        return Err(refused());
    }
    let downloads = canonical(&paths.downloads())?;
    let in_downloads = |file: &Path| canonical(file).map(|f| f.parent() == Some(downloads.as_path()));
    let allowed = match (detect(app, &exe), step) {
        (Some(Install::Msi(installed)), Finish::Msi(msi)) => installed == exe && in_downloads(msi)?,
        (Some(Install::Portable(installed) | Install::Standalone(installed)), Finish::Portable { zip, tmp }) => {
            installed == exe
                && in_downloads(zip)?
                && tmp.file_name().is_some_and(|n| n == PORTABLE_STAGING)
                && canonical(tmp.parent().ok_or_else(refused)?)? == cache
        }
        _ => false,
    };
    if allowed { Ok(()) } else { Err(refused()) }
}

/// Lets a Windows test observe both sides of `run_step`'s wait for the old launcher. Compiled only
/// for tests, so production behavior is unchanged.
#[cfg(all(test, windows))]
pub(super) mod wait_watch {
    use std::sync::mpsc::{self, Receiver, Sender};
    use std::sync::{Mutex, PoisonError};

    /// A test's two observations of one wait, keyed by the launcher's pid.
    pub(crate) struct Channels {
        /// Sent just before `run_step` calls the wait.
        pub(crate) entered: Receiver<()>,
        /// Sent right after the wait returns, before any extraction, and carries the wait's result.
        pub(crate) returned: Receiver<bool>,
    }

    struct Watch {
        pid: u32,
        entered: Option<Sender<()>>,
        returned: Option<Sender<bool>>,
    }

    static WATCH: Mutex<Option<Watch>> = Mutex::new(None);

    pub(crate) fn watch(pid: u32) -> Channels {
        let (entered, entered_rx) = mpsc::channel();
        let (returned, returned_rx) = mpsc::channel();
        *WATCH.lock().unwrap_or_else(PoisonError::into_inner) = Some(Watch { pid, entered: Some(entered), returned: Some(returned) });
        Channels { entered: entered_rx, returned: returned_rx }
    }

    /// Notes that `run_step` is about to wait. Only the first call for a watched pid is kept.
    pub(crate) fn entered(pid: u32) {
        let mut watch = WATCH.lock().unwrap_or_else(PoisonError::into_inner);
        let sender = watch.as_mut().filter(|watch| watch.pid == pid).and_then(|watch| watch.entered.take());
        if let Some(sender) = sender {
            let _ = sender.send(());
        }
    }

    /// Notes the wait's result as soon as it returns; the step hasn't run yet.
    pub(crate) fn returned(pid: u32, waited: bool) {
        let mut watch = WATCH.lock().unwrap_or_else(PoisonError::into_inner);
        let sender = watch.as_mut().filter(|watch| watch.pid == pid).and_then(|watch| watch.returned.take());
        if let Some(sender) = sender {
            let _ = sender.send(waited);
        }
    }
}

/// Waits up to `wait` for `pid` to exit, does `step` once it has, and removes the download either
/// way. The MSI, and every program in the portable zip, must be signed like `reference`.
#[cfg(windows)]
pub(super) fn run_step(app: &App, reference: &Path, pid: u32, exe: &Path, step: &Finish, wait: std::time::Duration) -> Result<(), String> {
    #[cfg(test)]
    wait_watch::entered(pid);
    // The test hook needs the result, so it is stored rather than consumed by the branch.
    let waited = process::wait_for_exit(pid, wait);
    #[cfg(test)]
    wait_watch::returned(pid, waited);
    let result = if waited {
        match step {
            Finish::Msi(msi) => {
                signed_like(app, reference, msi).and_then(|()| msi_step(app, exe)).and_then(|step| crate::msi_migrate::windows::run(msi, &step))
            }
            Finish::Portable { zip, tmp } => match (exe.parent(), exe.file_name()) {
                (Some(dir), Some(executable)) => crate::portable::unpack_over(zip, tmp, dir, executable, &|file| {
                    // The expected executable must be signed even when its contents aren't a PE file.
                    if file.file_name() == Some(executable) || is_program(file) { signed_like(app, reference, file) } else { Ok(()) }
                }),
                _ => Err(format!("{} isn't in a folder", app.name)),
            },
        }
    } else {
        Err(format!("{} was still running after a minute, so the update wasn't installed", app.name))
    };
    match step {
        Finish::Msi(msi) => {
            let _ = std::fs::remove_file(msi);
        }
        // `unpack_over` removes the staging folder, unless it holds files it couldn't put back.
        Finish::Portable { zip, .. } => {
            let _ = std::fs::remove_file(zip);
        }
    }
    result
}
