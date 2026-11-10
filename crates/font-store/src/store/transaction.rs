//! The font store's on-disk transaction: the lock, the manifest, staging and committing files.

use super::*;

impl FontStore {
    pub(super) fn rescan(&self) {
        if let Some(catalog) = &self.catalog {
            catalog.rescan();
        }
    }

    pub(crate) fn lock(&self) -> Result<File, Error> {
        std::fs::create_dir_all(&self.dir).map_err(io("creating the font store folder"))?;
        let file = File::options().create(true).truncate(false).write(true).open(self.dir.join(LOCK)).map_err(io("locking the font store"))?;
        file.lock().map_err(io("locking the font store"))?;
        Ok(file)
    }

    /// The manifest, without entries that aren't safe to act on. A manifest that can't be read
    /// is kept aside as `manifest.json.bad` and counts as empty.
    pub(super) fn read_manifest(&self) -> Manifest {
        let path = self.dir.join(MANIFEST);
        let Some(bytes) = read_limited(&path, MAX_MANIFEST_BYTES) else { return Manifest::default() };
        let Ok(mut manifest) = serde_json::from_slice::<Manifest>(&bytes) else {
            let _ = std::fs::rename(&path, self.dir.join("manifest.json.bad"));
            return Manifest::default();
        };
        manifest
            .families
            .retain(|f| safe_dir(&f.dir) && f.files.iter().all(|x| safe_file_name(&x.name)) && f.license_file.as_deref().is_none_or(safe_file_name));
        manifest
    }

    pub(super) fn write_manifest(&self, manifest: &Manifest) -> Result<(), Error> {
        let manifest = Manifest { version: 1, families: manifest.families.clone() };
        let bytes = serde_json::to_vec_pretty(&manifest).map_err(|e| io("saving the manifest")(std::io::Error::other(e)))?;
        self.write_file(MANIFEST, &bytes)
    }

    pub(crate) fn write_file(&self, name: &str, bytes: &[u8]) -> Result<(), Error> {
        std::fs::create_dir_all(&self.dir).map_err(io("creating the font store folder"))?;
        craft_atomic::atomic_write(&self.dir.join(name), bytes).map_err(io(format!("saving {name}")))
    }

    pub(crate) fn sweep_staging(&self) {
        for e in std::fs::read_dir(self.dir.join(STAGING)).into_iter().flatten().flatten() {
            let old = e.metadata().and_then(|m| m.modified()).is_ok_and(|t| SystemTime::now().duration_since(t).is_ok_and(|d| d > STALE));
            if old {
                let _ = std::fs::remove_dir_all(e.path());
            }
        }
    }
}

/// The family's manifest entry after installing `files`, merged into `manifest`.
pub(super) fn merge_entry(manifest: &mut Manifest, index: &Index, family: &Family, files: &[&FontFile]) -> InstalledFamily {
    let pos = match manifest.families.iter().position(|f| f.is(&family.source, &family.family)) {
        Some(p) => p,
        None => {
            manifest.families.push(InstalledFamily {
                family: family.family.clone(),
                source: family.source.clone(),
                dir: family.dir().to_string(),
                license: family.license.clone(),
                license_file: None,
                files: Vec::new(),
            });
            manifest.families.len() - 1
        }
    };
    let Some(entry) = manifest.families.get_mut(pos) else { return InstalledFamily::empty() };
    entry.license.clone_from(&family.license);
    let commit = index.source_of(family).map(|s| s.commit.clone()).unwrap_or_default();
    if let Some(name) = family.license_file_name() {
        entry.license_file = Some(name.to_string());
    }
    for f in files {
        entry.files.retain(|x| x.name != f.file_name());
        entry.files.push(InstalledFile {
            name: f.file_name().to_string(),
            path: f.path.clone(),
            size: f.size,
            sha256: f.sha256.clone(),
            commit: commit.clone(),
            postscript: f.postscript.clone(),
        });
    }
    entry.files.sort_by(|a, b| a.name.cmp(&b.name));
    let entry = entry.clone();
    manifest.families.sort_by_key(|f| (f.family.to_lowercase(), f.source.clone()));
    entry
}

/// A folder of downloads under `.staging`, deleted when dropped.
pub(crate) struct Stage {
    pub(crate) dir: PathBuf,
}

impl Stage {
    pub(crate) fn new(store: &Path) -> Result<Self, Error> {
        static N: AtomicU64 = AtomicU64::new(0);
        let staging = store.join(STAGING);
        std::fs::create_dir_all(&staging).map_err(io("creating a download folder"))?;
        // A folder of that name may be another process's (PID namespaces in Flatpak or
        // containers repeat ids) or a stale one: never touched here, the next name is taken
        // (stale ones go in `sweep_staging`).
        let mut last = None;
        for _ in 0..1000 {
            let dir = staging.join(format!("{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
            match std::fs::create_dir(&dir) {
                Ok(()) => return Ok(Self { dir }),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => last = Some(e),
                Err(e) => return Err(io("creating a download folder")(e)),
            }
        }
        Err(io("creating a download folder")(last.unwrap_or_else(|| std::io::Error::other("no free name"))))
    }

    /// Writes a verified download as `<name>.part`, which no font scan picks up.
    pub(super) fn write(&self, name: &str, bytes: &[u8]) -> Result<PathBuf, Error> {
        let path = self.dir.join(format!("{name}.part"));
        let write = || -> std::io::Result<()> {
            let mut f = File::options().write(true).create_new(true).open(&path)?;
            f.write_all(bytes)?;
            f.sync_all()
        };
        write().map_err(io(format!("saving {name}")))?;
        Ok(path)
    }
}

impl Drop for Stage {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Staged files moved into a family folder, undone unless kept.
pub(super) struct Commit {
    folder: PathBuf,
    created_folder: bool,
    /// Each file moved in, and where the file it replaced went.
    moved: Vec<(PathBuf, Option<PathBuf>)>,
    kept: bool,
}

impl Commit {
    pub(super) fn apply(folder: &Path, stage: &Path, staged: &[(String, PathBuf)]) -> Result<Self, Error> {
        let created_folder = !folder.is_dir();
        let mut commit = Self { folder: folder.to_path_buf(), created_folder, moved: Vec::new(), kept: false };
        std::fs::create_dir_all(folder).map_err(io("creating the family folder"))?;
        for (name, from) in staged {
            let dest = folder.join(name);
            let backup = if dest.exists() {
                let b = stage.join(format!("{name}.old"));
                std::fs::rename(&dest, &b).map_err(io(format!("replacing {name}")))?;
                Some(b)
            } else {
                None
            };
            if let Err(e) = std::fs::rename(from, &dest) {
                if let Some(b) = &backup {
                    let _ = std::fs::rename(b, &dest);
                }
                return Err(io(format!("installing {name}"))(e));
            }
            commit.moved.push((dest, backup));
        }
        Ok(commit)
    }

    pub(super) fn keep(mut self) {
        self.kept = true;
    }

    pub(super) fn undo(self) {}
}

impl Drop for Commit {
    fn drop(&mut self) {
        if self.kept {
            return;
        }
        for (dest, backup) in self.moved.iter().rev() {
            let _ = std::fs::remove_file(dest);
            if let Some(b) = backup {
                let _ = std::fs::rename(b, dest);
            }
        }
        if self.created_folder {
            let _ = std::fs::remove_dir(&self.folder);
        }
    }
}
