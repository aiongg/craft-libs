//! The release `/usr/local`-style tree: placement and removal.

use super::{App, Path, PathBuf, single_top_folder};

/// Unpacks `file` and returns a staging folder laid out like `/usr/local`.
pub fn unpack(app: &App, file: &Path, staging: &Path) -> Result<PathBuf, String> {
    let _ = std::fs::remove_dir_all(staging);
    std::fs::create_dir_all(staging).map_err(|e| format!("couldn't create {} ({e})", staging.display()))?;
    let gz = std::fs::File::open(file).map_err(|e| e.to_string())?;
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(gz));
    archive.set_preserve_permissions(true);
    // Bound entry count and cumulative unpacked bytes to prevent decompression bombs (CWE-409).
    const MAX_ENTRIES: usize = 50_000;
    const MAX_UNPACKED_BYTES: u64 = 2 * 1024 * 1024 * 1024;
    let mut entry_count = 0;
    let mut total_unpacked = 0u64;
    let mut directories = Vec::new();
    let entries = archive.entries().map_err(|e| format!("couldn't read archive entries ({e})"))?;
    for entry in entries {
        let mut entry = entry.map_err(|e| format!("invalid tar entry ({e})"))?;
        entry_count += 1;
        if entry_count > MAX_ENTRIES {
            return Err("archive has too many entries".into());
        }
        let size = entry.size();
        total_unpacked = total_unpacked.saturating_add(size);
        if total_unpacked > MAX_UNPACKED_BYTES {
            return Err("archive exceeds maximum unpacked size".into());
        }
        if entry.header().entry_type().is_dir() {
            directories.push(entry);
        } else {
            entry.unpack_in(staging).map_err(|e| format!("couldn't unpack archive entry ({e})"))?;
        }
    }
    // Delay directory entries until child entries are written so restrictive modes do not break child extraction.
    directories.sort_by(|a, b| b.path_bytes().cmp(&a.path_bytes()));
    for mut dir in directories {
        dir.unpack_in(staging).map_err(|e| format!("couldn't unpack archive directory ({e})"))?;
    }
    let top = single_top_folder(staging)?;
    if top.join("bin").join(app.id).is_file() {
        return Ok(top);
    }
    // Flat layout: put each file where /usr/local keeps it.
    // Ensure tree is a fresh directory and not an archive-created symlink or directory (CWE-59).
    let tree = staging.join("tree");
    if let Ok(m) = std::fs::symlink_metadata(&tree) {
        if m.file_type().is_symlink() || !m.is_dir() {
            let _ = std::fs::remove_file(&tree);
        } else {
            let _ = std::fs::remove_dir_all(&tree);
        }
    }
    std::fs::create_dir_all(&tree).map_err(|e| e.to_string())?;
    let id = app.bundle_id();
    for (from, to) in [
        (top.join(app.id), tree.join("bin").join(app.id)),
        (top.join(format!("{}-cli", app.id)), tree.join("bin").join(format!("{}-cli", app.id))),
        (top.join(format!("{id}.desktop")), tree.join("share/applications").join(format!("{id}.desktop"))),
        (top.join(format!("{id}.png")), tree.join("share/pixmaps").join(format!("{id}.png"))),
    ] {
        if std::fs::symlink_metadata(&from).is_ok_and(|m| m.file_type().is_file()) {
            if let Some(dir) = to.parent() {
                let rel = dir.strip_prefix(&tree).map_err(|_| "destination escapes staging tree".to_string())?;
                let mut check = tree.clone();
                for part in rel.components() {
                    check.push(part);
                    if let Ok(m) = std::fs::symlink_metadata(&check)
                        && m.file_type().is_symlink()
                    {
                        return Err("destination parent directory is a symlink".into());
                    }
                }
                std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
            }
            std::fs::rename(&from, &to).map_err(|e| e.to_string())?;
        }
    }
    if !tree.join("bin").join(app.id).is_file() {
        return Err("the download doesn't contain the app".into());
    }
    Ok(tree)
}

/// Every file under `root`, relative to it. Does not descend through symlinks (CWE-59, CWE-400).
pub fn files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    let mut visited = 0;
    const MAX_FILES: usize = 50_000;
    while let Some(dir) = stack.pop() {
        let entries = std::fs::read_dir(&dir).map_err(|e| format!("couldn't read directory {}: {e}", dir.display()))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("couldn't read directory entry: {e}"))?;
            visited += 1;
            if visited > MAX_FILES {
                return Err("directory tree exceeds maximum file limit of 50,000".into());
            }
            let ft = entry.file_type().map_err(|e| format!("couldn't read file type: {e}"))?;
            let p = entry.path();
            if ft.is_dir() && !ft.is_symlink() {
                stack.push(p);
            } else {
                let rel = p.strip_prefix(root).map_err(|e| e.to_string())?;
                out.push(rel.to_path_buf());
            }
        }
    }
    out.sort();
    Ok(out)
}
