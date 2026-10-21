//! The font store folder: consent, installs, the manifest, removal and checks.
//!
//! Layout of the folder ([`craft_font_catalog::font_store_dir`]):
//!
//! ```text
//! settings.json        the consent setting
//! manifest.json        what is installed
//! index.json           the last index fetched
//! <family dir>/        google/fonts' folder name: the font files and the licence
//! .staging/            downloads being installed; nothing a font scan picks up
//! .lock                held while the store changes
//! ```

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

use craft_font_catalog::{Catalog, FontEnv, font_store_dir};
use serde::{Deserialize, Serialize};

use crate::http::{Cancel, Client, FetchError};
use crate::index::{Face, Family, FontFile, Index, IndexError, MAX_INDEX_BYTES, Unavailable, safe_dir, safe_file_name};
use crate::pin::IndexPin;
use crate::verify::{VerifyError, is_font, sha256_hex, verify_font};

const SETTINGS: &str = "settings.json";
const MANIFEST: &str = "manifest.json";
const INDEX_CACHE: &str = "index.json";
const STAGING: &str = ".staging";
const LOCK: &str = ".lock";
const MAX_MANIFEST_BYTES: u64 = 16 << 20;
const MAX_LICENSE_BYTES: u64 = 1 << 20;
/// Staging folders older than this belong to an install that crashed.
const STALE: Duration = Duration::from_secs(3600);

/// Whether the store may download.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    /// No requests. The default.
    #[default]
    Off,
    /// The user allowed downloads from jsDelivr and GitHub.
    On,
}

/// Who drives the store. Only a person in the app can turn the network on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Caller {
    /// The app's own interface, acting on the user's click.
    User,
    /// An agent (MCP, the CLI, scripting): it can install when the user has turned the network
    /// on, and can never turn it on.
    Agent,
}

/// Why a store operation failed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Downloading is off; the user turns it on in the app.
    #[error("downloading fonts is turned off")]
    NetworkOff,
    /// An agent tried to turn the network on.
    #[error("only the user can turn on downloading fonts")]
    NotAllowed,
    /// No exact match in the index.
    #[error(transparent)]
    Unavailable(#[from] Unavailable),
    /// The index can't be used.
    #[error(transparent)]
    Index(#[from] IndexError),
    /// Every URL failed.
    #[error("downloading {what} failed: {}", reasons.join("; "))]
    Download {
        /// The file.
        what: String,
        /// Each URL's failure.
        reasons: Vec<String>,
    },
    /// The downloaded file isn't the one the index describes.
    #[error("{file} was rejected: {error}")]
    Verify {
        /// The file.
        file: String,
        /// What was wrong.
        error: VerifyError,
    },
    /// The family isn't installed.
    #[error("{0} is not installed")]
    NotInstalled(String),
    /// The operation was cancelled.
    #[error("cancelled")]
    Cancelled,
    /// Reading or writing the store failed.
    #[error("{what}: {source}")]
    Io {
        /// What was being done.
        what: String,
        /// The error.
        source: std::io::Error,
    },
}

fn io(what: impl Into<String>) -> impl FnOnce(std::io::Error) -> Error {
    let what = what.into();
    move |source| Error::Io { what, source }
}

/// Download progress of an install.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Progress {
    /// The file being downloaded, from 0.
    pub file: usize,
    /// Files to download.
    pub files: usize,
    /// Bytes downloaded, of all files.
    pub done: u64,
    /// Bytes of all files.
    pub total: u64,
}

/// An installed font file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledFile {
    /// The file's name in the family folder.
    pub name: String,
    /// Its path in google/fonts.
    pub path: String,
    /// Bytes.
    pub size: u64,
    /// sha256, lowercase hex.
    pub sha256: String,
    /// The google/fonts commit of the index it was installed from.
    pub commit: String,
    /// Its PostScript names.
    #[serde(default)]
    pub postscript: Vec<String>,
}

/// An installed family.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledFamily {
    /// The family name.
    pub family: String,
    /// The folder in the store holding its files.
    pub dir: String,
    /// `OFL`, `APACHE2` or `UFL`.
    pub license: String,
    /// The licence file's name in the folder.
    pub license_file: Option<String>,
    /// The installed files: all of the family's, or some faces.
    pub files: Vec<InstalledFile>,
}

#[derive(Default, Serialize, Deserialize)]
struct Manifest {
    version: u32,
    families: Vec<InstalledFamily>,
}

#[derive(Default, Serialize, Deserialize)]
struct Settings {
    #[serde(default)]
    network: Network,
}

/// Something [`FontStore::verify`] found wrong with an installed file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem {
    /// The family.
    pub family: String,
    /// The file.
    pub file: String,
    /// What is wrong; `None` when the file is missing.
    pub error: Option<VerifyError>,
}

/// The suite's font store.
///
/// Nothing here touches the network on its own: only [`FontStore::fetch_index`] and the installs
/// do, and only while the user has turned the network on.
pub struct FontStore {
    dir: PathBuf,
    caller: Caller,
    client: Client,
    catalog: Option<Arc<Catalog>>,
}

impl std::fmt::Debug for FontStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FontStore").field("dir", &self.dir).field("caller", &self.caller).finish_non_exhaustive()
    }
}

impl FontStore {
    /// The store for the app's own interface, in the folder every app reads.
    /// `None` where there is no per-user folder.
    pub fn for_user(env: &FontEnv) -> Option<Self> {
        Some(Self::new(font_store_dir(env)?, Caller::User))
    }

    /// The store for an agent (MCP, CLI): it can never turn the network on.
    pub fn for_agent(env: &FontEnv) -> Option<Self> {
        Some(Self::new(font_store_dir(env)?, Caller::Agent))
    }

    /// A store in `dir`.
    pub fn new(dir: PathBuf, caller: Caller) -> Self {
        Self { dir, caller, client: Client::default(), catalog: None }
    }

    /// Uses `client` for downloads (other timeouts).
    pub fn with_client(mut self, client: Client) -> Self {
        self.client = client;
        self
    }

    /// Rescans `catalog` after each install and removal, so the app sees the change at once.
    /// Apps pass `Catalog::global().clone()`.
    pub fn with_catalog(mut self, catalog: Arc<Catalog>) -> Self {
        self.catalog = Some(catalog);
        self
    }

    /// The store's folder.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Who drives this store.
    pub fn caller(&self) -> Caller {
        self.caller
    }

    // ----- consent

    /// The user's setting; [`Network::Off`] when it was never set or can't be read.
    pub fn network(&self) -> Network {
        read_limited(&self.dir.join(SETTINGS), 64 << 10)
            .and_then(|b| serde_json::from_slice::<Settings>(&b).ok())
            .map(|s| s.network)
            .unwrap_or_default()
    }

    /// Turns downloading on: call it only from the user's own action (a setting, an Install
    /// click). [`Error::NotAllowed`] for an agent.
    pub fn enable_network(&self) -> Result<(), Error> {
        if self.caller != Caller::User {
            return Err(Error::NotAllowed);
        }
        self.write_settings(Network::On)
    }

    /// Turns downloading off. Anyone may.
    pub fn disable_network(&self) -> Result<(), Error> {
        self.write_settings(Network::Off)
    }

    fn write_settings(&self, network: Network) -> Result<(), Error> {
        let bytes = serde_json::to_vec_pretty(&Settings { network }).map_err(|e| io("saving the setting")(std::io::Error::other(e)))?;
        self.write_file(SETTINGS, &bytes)
    }

    fn require_network(&self) -> Result<(), Error> {
        if self.network() == Network::On { Ok(()) } else { Err(Error::NetworkOff) }
    }

    // ----- index

    /// The index saved by the last [`FontStore::fetch_index`], when it is `pin`'s.
    pub fn cached_index(&self, pin: &IndexPin) -> Option<Index> {
        let bytes = read_limited(&self.dir.join(INDEX_CACHE), MAX_INDEX_BYTES as u64)?;
        (sha256_hex(&bytes) == pin.sha256).then(|| Index::from_json(&bytes).ok()).flatten()
    }

    /// Downloads the index `pin` names, checks its sha256 and saves it for
    /// [`FontStore::cached_index`]. Needs the network on.
    pub fn fetch_index(&self, pin: &IndexPin, cancel: &Cancel) -> Result<Index, Error> {
        self.require_network()?;
        let check = |bytes: &[u8]| if sha256_hex(bytes) == pin.sha256 { Ok(()) } else { Err(VerifyError::Sha256) };
        let bytes = self.fetch(&pin.urls(), MAX_INDEX_BYTES as u64, "the font index", cancel, &mut |_| {}, &check)?;
        let index = Index::from_json(&bytes)?;
        self.write_file(INDEX_CACHE, &bytes)?;
        Ok(index)
    }

    // ----- install

    /// Installs every file of `family`. Files already installed with the same sha256 aren't
    /// downloaded again.
    pub fn install_family(
        &self,
        index: &Index,
        family: &Family,
        cancel: &Cancel,
        progress: &mut dyn FnMut(Progress),
    ) -> Result<InstalledFamily, Error> {
        let files: Vec<&FontFile> = family.files.iter().collect();
        self.install(index, family, &files, cancel, progress)
    }

    /// Installs the file of one face (from [`Index::find_face`] or [`Index::find_postscript`]),
    /// with the family's licence.
    pub fn install_face(&self, index: &Index, face: Face<'_>, cancel: &Cancel, progress: &mut dyn FnMut(Progress)) -> Result<InstalledFamily, Error> {
        self.install(index, face.family, &[face.file], cancel, progress)
    }

    fn install(
        &self,
        index: &Index,
        family: &Family,
        files: &[&FontFile],
        cancel: &Cancel,
        progress: &mut dyn FnMut(Progress),
    ) -> Result<InstalledFamily, Error> {
        let installed = self.installed_family(&family.family);
        let folder = self.dir.join(family.dir());
        let have = |f: &FontFile| installed.as_ref().is_some_and(|i| i.files.iter().any(|x| x.name == f.file_name() && x.sha256 == f.sha256));
        let wanted: Vec<&FontFile> = files.iter().copied().filter(|f| !have(f) || !folder.join(f.file_name()).is_file()).collect();
        let license_missing = family.license_file_name().is_some_and(|name| !folder.join(name).is_file());
        if wanted.is_empty() && !license_missing {
            return installed.ok_or_else(|| Error::NotInstalled(family.family.clone()));
        }
        self.require_network()?;
        std::fs::create_dir_all(&self.dir).map_err(io("creating the font store folder"))?;
        self.sweep_staging();
        let stage = Stage::new(&self.dir)?;

        let total: u64 = wanted.iter().map(|f| f.size).sum();
        let mut done_before = 0u64;
        let mut staged: Vec<(String, PathBuf)> = Vec::new();
        for (i, file) in wanted.iter().enumerate() {
            let mut on_bytes = |n: u64| progress(Progress { file: i, files: wanted.len(), done: done_before + n, total });
            let check = |bytes: &[u8]| verify_font(file, bytes);
            let bytes = self.fetch(&index.urls(&file.path), file.size, file.file_name(), cancel, &mut on_bytes, &check)?;
            done_before += file.size;
            staged.push((file.file_name().to_string(), stage.write(file.file_name(), &bytes)?));
        }
        if license_missing && let (Some(path), Some(name)) = (&family.license_path, family.license_file_name()) {
            // The index has no sha256 for licences: the size cap is the only check.
            let check = |_: &[u8]| Ok(());
            let bytes = self.fetch(&index.urls(path), MAX_LICENSE_BYTES, name, cancel, &mut |_| {}, &check)?;
            staged.push((name.to_string(), stage.write(name, &bytes)?));
        }
        if cancel.is_cancelled() {
            return Err(Error::Cancelled);
        }

        let result = {
            let _lock = self.lock()?;
            let mut manifest = self.read_manifest();
            let commit = Commit::apply(&folder, &stage.dir, &staged)?;
            let entry = merge_entry(&mut manifest, index, family, &wanted);
            match self.write_manifest(&manifest) {
                Ok(()) => {
                    commit.keep();
                    Ok(entry)
                }
                Err(e) => {
                    commit.undo();
                    Err(e)
                }
            }
        };
        drop(stage);
        self.rescan();
        result
    }

    /// Downloads from each URL in turn until one gives bytes that pass `check`.
    fn fetch(
        &self,
        urls: &[String],
        max: u64,
        what: &str,
        cancel: &Cancel,
        on_bytes: &mut dyn FnMut(u64),
        check: &dyn Fn(&[u8]) -> Result<(), VerifyError>,
    ) -> Result<Vec<u8>, Error> {
        let mut reasons = Vec::new();
        let mut rejected = None;
        for url in urls {
            match self.client.get(url, max, cancel, on_bytes) {
                Ok(bytes) => match check(&bytes) {
                    Ok(()) => return Ok(bytes),
                    Err(e) => {
                        reasons.push(format!("{}: {e}", host(url)));
                        rejected.get_or_insert(e);
                    }
                },
                Err(FetchError::Cancelled) => return Err(Error::Cancelled),
                Err(e) => reasons.push(format!("{}: {e}", host(url))),
            }
        }
        match rejected {
            Some(error) => Err(Error::Verify { file: what.to_string(), error }),
            None => Err(Error::Download { what: what.to_string(), reasons }),
        }
    }

    // ----- installed fonts

    /// The installed families, sorted by name.
    pub fn installed(&self) -> Vec<InstalledFamily> {
        self.read_manifest().families
    }

    /// The installed family of this name, ignoring case.
    pub fn installed_family(&self, name: &str) -> Option<InstalledFamily> {
        let lower = name.to_lowercase();
        self.installed().into_iter().find(|f| f.family.to_lowercase() == lower)
    }

    /// Whether `file` is installed, with the index's sha256.
    pub fn is_installed(&self, family: &Family, file: &FontFile) -> bool {
        self.installed_family(&family.family).is_some_and(|i| i.files.iter().any(|x| x.name == file.file_name() && x.sha256 == file.sha256))
    }

    /// Removes a family: its files, its licence and its manifest entry. Its folder disappears at
    /// once and is deleted after.
    pub fn remove_family(&self, name: &str) -> Result<(), Error> {
        let result = {
            let _lock = self.lock()?;
            let mut manifest = self.read_manifest();
            let lower = name.to_lowercase();
            let pos = manifest.families.iter().position(|f| f.family.to_lowercase() == lower).ok_or_else(|| Error::NotInstalled(name.to_string()))?;
            let entry = manifest.families.remove(pos);
            let folder = self.dir.join(&entry.dir);
            let stage = Stage::new(&self.dir)?;
            let moved = stage.dir.join("removed");
            let had_folder = folder.exists();
            if had_folder {
                std::fs::rename(&folder, &moved).map_err(io(format!("removing {}", entry.family)))?;
            }
            match self.write_manifest(&manifest) {
                Ok(()) => Ok(()),
                Err(e) => {
                    if had_folder {
                        let _ = std::fs::rename(&moved, &folder);
                    }
                    Err(e)
                }
            }
        };
        self.rescan();
        result
    }

    /// Checks every installed file against the manifest: present, the right size and sha256,
    /// and a font.
    pub fn verify(&self) -> Vec<Problem> {
        let mut problems = Vec::new();
        for fam in self.installed() {
            for f in &fam.files {
                let path = self.dir.join(&fam.dir).join(&f.name);
                let problem = |error| Problem { family: fam.family.clone(), file: f.name.clone(), error };
                let Ok(bytes) = std::fs::read(&path) else {
                    problems.push(problem(None));
                    continue;
                };
                let error = if bytes.len() as u64 != f.size {
                    Some(VerifyError::Size { expected: f.size, actual: bytes.len() as u64 })
                } else if sha256_hex(&bytes) != f.sha256 {
                    Some(VerifyError::Sha256)
                } else if !is_font(&bytes) {
                    Some(VerifyError::NotAFont)
                } else {
                    None
                };
                if error.is_some() {
                    problems.push(problem(error));
                }
            }
        }
        problems
    }

    // ----- folder

    fn rescan(&self) {
        if let Some(catalog) = &self.catalog {
            catalog.rescan();
        }
    }

    fn lock(&self) -> Result<File, Error> {
        std::fs::create_dir_all(&self.dir).map_err(io("creating the font store folder"))?;
        let file = File::options().create(true).truncate(false).write(true).open(self.dir.join(LOCK)).map_err(io("locking the font store"))?;
        file.lock().map_err(io("locking the font store"))?;
        Ok(file)
    }

    /// The manifest, without entries that aren't safe to act on. A manifest that can't be read
    /// is kept aside as `manifest.json.bad` and counts as empty.
    fn read_manifest(&self) -> Manifest {
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

    fn write_manifest(&self, manifest: &Manifest) -> Result<(), Error> {
        let manifest = Manifest { version: 1, families: manifest.families.clone() };
        let bytes = serde_json::to_vec_pretty(&manifest).map_err(|e| io("saving the manifest")(std::io::Error::other(e)))?;
        self.write_file(MANIFEST, &bytes)
    }

    fn write_file(&self, name: &str, bytes: &[u8]) -> Result<(), Error> {
        std::fs::create_dir_all(&self.dir).map_err(io("creating the font store folder"))?;
        craft_atomic::atomic_write(&self.dir.join(name), bytes).map_err(io(format!("saving {name}")))
    }

    fn sweep_staging(&self) {
        for e in std::fs::read_dir(self.dir.join(STAGING)).into_iter().flatten().flatten() {
            let old = e.metadata().and_then(|m| m.modified()).is_ok_and(|t| SystemTime::now().duration_since(t).is_ok_and(|d| d > STALE));
            if old {
                let _ = std::fs::remove_dir_all(e.path());
            }
        }
    }
}

/// The family's manifest entry after installing `files`, merged into `manifest`.
fn merge_entry(manifest: &mut Manifest, index: &Index, family: &Family, files: &[&FontFile]) -> InstalledFamily {
    let lower = family.family.to_lowercase();
    let pos = match manifest.families.iter().position(|f| f.family.to_lowercase() == lower) {
        Some(p) => p,
        None => {
            manifest.families.push(InstalledFamily {
                family: family.family.clone(),
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
            commit: index.commit().to_string(),
            postscript: f.postscript.clone(),
        });
    }
    entry.files.sort_by(|a, b| a.name.cmp(&b.name));
    let entry = entry.clone();
    manifest.families.sort_by_key(|f| f.family.to_lowercase());
    entry
}

impl InstalledFamily {
    fn empty() -> Self {
        Self { family: String::new(), dir: String::new(), license: String::new(), license_file: None, files: Vec::new() }
    }
}

fn read_limited(path: &Path, max: u64) -> Option<Vec<u8>> {
    if std::fs::metadata(path).ok()?.len() > max {
        return None;
    }
    std::fs::read(path).ok()
}

fn host(url: &str) -> &str {
    url.split("://").nth(1).and_then(|r| r.split('/').next()).unwrap_or(url)
}

/// A folder of downloads under `.staging`, deleted when dropped.
struct Stage {
    dir: PathBuf,
}

impl Stage {
    fn new(store: &Path) -> Result<Self, Error> {
        static N: AtomicU64 = AtomicU64::new(0);
        let dir = store.join(STAGING).join(format!("{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
        std::fs::create_dir_all(&dir).map_err(io("creating a download folder"))?;
        Ok(Self { dir })
    }

    /// Writes a verified download as `<name>.part`, which no font scan picks up.
    fn write(&self, name: &str, bytes: &[u8]) -> Result<PathBuf, Error> {
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
struct Commit {
    folder: PathBuf,
    created_folder: bool,
    /// Each file moved in, and where the file it replaced went.
    moved: Vec<(PathBuf, Option<PathBuf>)>,
    kept: bool,
}

impl Commit {
    fn apply(folder: &Path, stage: &Path, staged: &[(String, PathBuf)]) -> Result<Self, Error> {
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

    fn keep(mut self) {
        self.kept = true;
    }

    fn undo(self) {}
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
