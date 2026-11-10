//! Swapping the running launcher for a new copy: AppImage, standalone tarball and release tree.

use super::detect::marker;
use super::*;

/// Records `release` as just installed, so it isn't offered again for [`super::detect::JUST_UPDATED_FOR`].
pub(super) fn mark_installed(paths: &Paths, release: &Release) {
    if let Err(e) = crate::paths::write_atomic(&marker(paths), format!("{}\n{}\n", release.tag, crate::now()).as_bytes()) {
        log::warn!("couldn't record the self-update: {e}");
    }
}

/// Removes the record of [`mark_installed`] when the swap couldn't be scheduled, so the update
/// stays on offer.
pub(super) fn unmark_unless_scheduled(paths: &Paths, scheduled: Result<(), String>) -> Result<(), String> {
    if scheduled.is_err() {
        let _ = std::fs::remove_file(marker(paths));
    }
    scheduled
}

/// Copies the new AppImage beside the running one and renames it over it. The running copy is
/// mounted from its own (now unlinked) file, so it keeps working until it exits.
#[cfg(target_os = "linux")]
pub(super) fn replace_appimage(new: &Path, path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let dir = path.parent().ok_or("the AppImage isn't in a folder")?;
    let staged = dir.join(format!(".{}.update", path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()));
    std::fs::copy(new, &staged).map_err(|e| format!("couldn't write next to {} ({e})", path.display()))?;
    // The replacement AppImage must be executable; 0o755 is not group/world-writable.
    std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?; // NOSONAR
    std::fs::rename(&staged, path).map_err(|e| {
        let _ = std::fs::remove_file(&staged);
        format!("couldn't replace {} ({e})", path.display())
    })
}

/// Replaces a standalone copy from a release tarball, along with what the tarball install put
/// beside it: the release's other programs that are already in the executable's folder, and,
/// when that folder is a prefix's `bin` with the app's `share/<id>` folder next to it, everything
/// under the tarball's `share` (data, desktop entry, icons). A copy on its own in some folder gets
/// only its executable. Files the release doesn't ship are never touched. Every file is copied
/// next to its destination before any is renamed over it, so a copy that fails (full disk,
/// unwritable folder) changes nothing; the executable is renamed last.
#[cfg(unix)]
pub(super) fn replace_standalone(app: &App, paths: &Paths, tarball: &Path, exe: &Path) -> Result<(), String> {
    // Per-process names, so two updates running at once can't remove each other's files.
    let staging = paths.cache.join(format!("{}.standalone.{}", app.id, std::process::id()));
    let result = install::tree::unpack(app, tarball, &staging).and_then(|tree| replace_from_tree(app, &tree, exe));
    let _ = std::fs::remove_dir_all(&staging);
    result
}

#[cfg(unix)]
pub(super) fn replace_from_tree(app: &App, tree: &Path, exe: &Path) -> Result<(), String> {
    let new_exe = tree.join("bin").join(app.id);
    if !is_plain_file(&new_exe) {
        return Err(format!("release archive did not contain binary for {}", app.id));
    }
    let bin = exe.parent().ok_or_else(|| "executable isn't in a folder".to_string())?;
    let mut moves = Vec::new();
    for entry in std::fs::read_dir(tree.join("bin")).map_err(|e| format!("couldn't read the release's programs ({e})"))?.flatten() {
        let name = entry.file_name();
        if name != app.id && is_plain_file(&entry.path()) && bin.join(&name).is_file() {
            moves.push((entry.path(), bin.join(&name)));
        }
    }
    let prefix = bin.parent().filter(|_| bin.file_name().is_some_and(|n| n == "bin"));
    if let Some(prefix) = prefix
        && prefix.join("share").join(app.id).is_dir()
    {
        for file in plain_files(&tree.join("share"))? {
            let relative = file.strip_prefix(tree).map_err(|_| "staged file escapes the release".to_string())?;
            moves.push((file.clone(), prefix.join(relative)));
        }
    }
    moves.push((new_exe, exe.to_path_buf()));
    // A folder where a file goes would fail its rename after earlier files were replaced.
    if let Some((_, to)) = moves.iter().find(|(_, to)| to.symlink_metadata().is_ok_and(|m| m.is_dir())) {
        return Err(format!("couldn't replace {} (it's a folder)", to.display()));
    }
    // Copy everything next to its destination first, so a full disk or an unwritable folder
    // stops the update before any installed file changes. Renames within a folder don't fail
    // for those reasons, and the executable is renamed last.
    let mut staged = Vec::new();
    for (i, (from, to)) in moves.iter().enumerate() {
        match stage(from, to, i == moves.len() - 1) {
            Ok(s) => staged.push((s, to)),
            Err(e) => {
                for (s, _) in &staged {
                    let _ = std::fs::remove_file(s);
                }
                return Err(e);
            }
        }
    }
    let mut result = Ok(());
    for (s, to) in &staged {
        if result.is_ok() {
            result = std::fs::rename(s, to).map_err(|e| format!("couldn't replace {} ({e})", to.display()));
        }
        if result.is_err() {
            let _ = std::fs::remove_file(s);
        }
    }
    result
}

/// Copies `from` to a hidden file next to `to` and returns its path. Group and world write and
/// the set-id bits are never carried over from the archive; the executable is always 0755.
#[cfg(unix)]
pub(super) fn stage(from: &Path, to: &Path, executable: bool) -> Result<PathBuf, String> {
    use std::os::unix::fs::PermissionsExt;
    let dir = to.parent().ok_or_else(|| format!("{} isn't in a folder", to.display()))?;
    std::fs::create_dir_all(dir).map_err(|e| format!("couldn't create {} ({e})", dir.display()))?;
    let name = to.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let staged = dir.join(format!(".{name}.{}.update", std::process::id()));
    let done = std::fs::copy(from, &staged).map_err(|e| format!("couldn't write next to {} ({e})", to.display())).and_then(|_| {
        let mode = std::fs::metadata(from).map(|m| m.permissions().mode()).unwrap_or(0o644);
        let mode = if executable { 0o755 } else { mode & 0o755 };
        std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(mode)).map_err(|e| e.to_string())
    });
    match done {
        Ok(()) => Ok(staged),
        Err(e) => {
            let _ = std::fs::remove_file(&staged);
            Err(e)
        }
    }
}

/// A regular file, not a link to one.
#[cfg(unix)]
pub(super) fn is_plain_file(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_file())
}

/// The regular files under `dir`, without following links; none when `dir` doesn't exist.
#[cfg(unix)]
pub(super) fn plain_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries {
            let entry = entry.map_err(|e| format!("couldn't read {} ({e})", dir.display()))?;
            let kind = entry.file_type().map_err(|e| e.to_string())?;
            if kind.is_dir() {
                pending.push(entry.path());
            } else if kind.is_file() {
                files.push(entry.path());
            }
        }
    }
    Ok(files)
}
