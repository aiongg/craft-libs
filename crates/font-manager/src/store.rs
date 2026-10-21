//! What Craft Font Manager needs from the suite's font store: the download consent, installing a
//! family from the index, removing one, and downloading a family's files for a preview without
//! installing them.
//!
//! [`SuiteStore`] is craft-font-store's store; [`FakeStore`] stands in for it in tests.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use craft_font_catalog::Catalog;
pub use craft_font_store::UserSource;
use craft_font_store::{Cancel, Client, FontStore, Network, PINNED_INDEX, verify_font};

use crate::index::{Family, Index};

/// A store family to remove: the files the catalog found for it in the font store.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoveRequest {
    pub family: String,
    pub files: Vec<PathBuf>,
}

/// Bytes downloaded so far, of `total`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Progress {
    pub done: u64,
    pub total: u64,
}

/// A downloaded and verified file.
#[derive(Clone, Debug)]
pub struct Downloaded {
    pub path: String,
    pub bytes: Vec<u8>,
}

/// An error to show the user, in English.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreError(pub String);

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for StoreError {}

fn err(e: impl std::fmt::Display) -> StoreError {
    StoreError(e.to_string())
}

/// The font store. Calls block; the app makes the downloading ones on a worker thread.
pub trait Installer: Send + Sync {
    /// Has the user allowed downloads? The setting is the whole suite's.
    fn downloads_allowed(&self) -> bool;

    /// Turns downloads on or off, as the user's own action.
    fn allow_downloads(&self, allowed: bool) -> Result<(), StoreError>;

    /// Downloads, verifies and installs every file of `family`, with its licence.
    fn install(&self, index: &Index, family: &Family, progress: &mut dyn FnMut(Progress)) -> Result<(), StoreError>;

    /// Deletes a family's files from the store.
    fn remove(&self, request: &RemoveRequest) -> Result<(), StoreError>;

    /// Downloads and verifies `family`'s files without installing them.
    fn download(&self, index: &Index, family: &Family, progress: &mut dyn FnMut(Progress)) -> Result<Vec<Downloaded>, StoreError>;

    /// The indexes without a request: the built-in ones compiled in, and the sources the user
    /// added as last fetched.
    fn indexes(&self) -> Result<Index, StoreError> {
        crate::index::bundled().map_err(err)
    }

    /// Downloads the indexes: the built-in ones at the pinned commit and the user's sources. A
    /// source that fails is in [`Index::problems`]; the others load.
    fn fetch_indexes(&self) -> Result<Index, StoreError> {
        Err(StoreError(NO_FOLDER.into()))
    }

    /// The index URLs the user added as sources. Their families are never curated.
    fn user_sources(&self) -> Vec<UserSource> {
        Vec::new()
    }

    /// Adds an index URL (https, `.json` or `.json.gz`) as a source, as the user's own action.
    /// The next [`Installer::fetch_indexes`] downloads it.
    fn add_user_source(&self, _url: &str) -> Result<(), StoreError> {
        Err(StoreError(NO_FOLDER.into()))
    }

    /// Removes a source the user added. Families installed from it stay installed.
    fn remove_user_source(&self, _url: &str) -> Result<(), StoreError> {
        Err(StoreError(NO_FOLDER.into()))
    }
}

const NO_FOLDER: &str = "there is no folder for the font store";

/// craft-font-store's store, in the user's font store folder.
pub struct SuiteStore {
    store: FontStore,
    client: Client,
}

impl SuiteStore {
    /// The store the catalog's environment names, rescanning `catalog` after each change.
    pub fn for_user(catalog: Arc<Catalog>) -> Option<SuiteStore> {
        let store = FontStore::for_user(catalog.env())?.with_catalog(catalog);
        Some(SuiteStore { store, client: Client::default() })
    }

    /// The store's source and name for the family holding `files`, which can differ from the
    /// name in the fonts themselves.
    fn installed_name(&self, request: &RemoveRequest) -> Option<(String, String)> {
        let dir = self.store.dir();
        let installed = self.store.installed();
        let lower = request.family.to_lowercase();
        installed
            .iter()
            .find(|f| request.files.iter().any(|path| path.starts_with(dir.join(&f.dir))))
            .or_else(|| installed.iter().find(|f| f.family.to_lowercase() == lower))
            .map(|f| (f.source.clone(), f.family.clone()))
    }
}

impl Installer for SuiteStore {
    fn downloads_allowed(&self) -> bool {
        self.store.network() == Network::On
    }

    fn allow_downloads(&self, allowed: bool) -> Result<(), StoreError> {
        if allowed { self.store.enable_network() } else { self.store.disable_network() }.map_err(err)
    }

    fn install(&self, index: &Index, family: &Family, progress: &mut dyn FnMut(Progress)) -> Result<(), StoreError> {
        let mut report = |p: craft_font_store::Progress| progress(Progress { done: p.done, total: p.total });
        self.store.install_family(index, family, &Cancel::new(), &mut report).map(drop).map_err(err)
    }

    fn remove(&self, request: &RemoveRequest) -> Result<(), StoreError> {
        let (source, name) = self.installed_name(request).ok_or_else(|| err(craft_font_store::Error::NotInstalled(request.family.clone())))?;
        self.store.remove_family(&source, &name).map_err(err)
    }

    fn download(&self, index: &Index, family: &Family, progress: &mut dyn FnMut(Progress)) -> Result<Vec<Downloaded>, StoreError> {
        if !self.downloads_allowed() {
            return Err(err(craft_font_store::Error::NetworkOff));
        }
        let total = family.size();
        let cancel = Cancel::new();
        let mut before = 0u64;
        let mut out = Vec::new();
        for file in &family.files {
            let mut reasons = Vec::new();
            for url in index.file_urls(family, file) {
                let mut on_bytes = |n: u64| progress(Progress { done: before.saturating_add(n), total });
                match self.client.get(&url, file.size, &cancel, &mut on_bytes) {
                    Ok(bytes) => match verify_font(file, &bytes) {
                        Ok(()) => {
                            out.push(Downloaded { path: file.path.clone(), bytes });
                            reasons.clear();
                            break;
                        }
                        Err(e) => reasons.push(format!("{url}: {e}")),
                    },
                    Err(e) => reasons.push(format!("{url}: {e}")),
                }
            }
            if !reasons.is_empty() {
                return Err(StoreError(format!("downloading {} failed: {}", file.file_name(), reasons.join("; "))));
            }
            before = before.saturating_add(file.size);
        }
        Ok(out)
    }

    fn indexes(&self) -> Result<Index, StoreError> {
        Ok(self.store.offline_indexes())
    }

    fn fetch_indexes(&self) -> Result<Index, StoreError> {
        self.store.fetch_indexes(&PINNED_INDEX, &Cancel::new()).map_err(err)
    }

    fn user_sources(&self) -> Vec<UserSource> {
        self.store.user_sources()
    }

    fn add_user_source(&self, url: &str) -> Result<(), StoreError> {
        self.store.add_user_source(url).map_err(err)
    }

    fn remove_user_source(&self, url: &str) -> Result<(), StoreError> {
        self.store.remove_user_source(url).map_err(err)
    }
}

/// The store when the system names no folder for it (no home folder): nothing installs.
pub struct NoStore;

impl Installer for NoStore {
    fn downloads_allowed(&self) -> bool {
        false
    }

    fn allow_downloads(&self, _: bool) -> Result<(), StoreError> {
        Err(StoreError(NO_FOLDER.into()))
    }

    fn install(&self, _: &Index, _: &Family, _: &mut dyn FnMut(Progress)) -> Result<(), StoreError> {
        Err(StoreError(NO_FOLDER.into()))
    }

    fn remove(&self, _: &RemoveRequest) -> Result<(), StoreError> {
        Err(StoreError(NO_FOLDER.into()))
    }

    fn download(&self, _: &Index, _: &Family, _: &mut dyn FnMut(Progress)) -> Result<Vec<Downloaded>, StoreError> {
        Err(StoreError(NO_FOLDER.into()))
    }
}

/// What a [`FakeStore`] was asked to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Call {
    Install(String),
    Remove(String),
    Download(String),
}

/// A font store without a network: "downloads" come from a table of file bytes by index path,
/// and installs write them into a folder as the real store does, `<dir>/<family>/<file>`. It
/// refuses to download while downloads are off, like the real one.
pub struct FakeStore {
    dir: PathBuf,
    files: HashMap<String, Vec<u8>>,
    allowed: AtomicBool,
    calls: Mutex<Vec<Call>>,
}

impl FakeStore {
    pub fn new(dir: impl Into<PathBuf>, files: HashMap<String, Vec<u8>>) -> FakeStore {
        FakeStore { dir: dir.into(), files, allowed: AtomicBool::new(false), calls: Mutex::new(Vec::new()) }
    }

    /// Everything asked of it, downloads refused for lack of consent included.
    pub fn calls(&self) -> Vec<Call> {
        self.calls.lock().map(|c| c.clone()).unwrap_or_default()
    }

    fn record(&self, call: Call) {
        if let Ok(mut calls) = self.calls.lock() {
            calls.push(call);
        }
    }

    fn fetch(&self, family: &Family, progress: &mut dyn FnMut(Progress)) -> Result<Vec<Downloaded>, StoreError> {
        if !self.downloads_allowed() {
            return Err(StoreError("downloading fonts is turned off".into()));
        }
        let total = family.size();
        let mut done = 0u64;
        let mut out = Vec::new();
        for file in &family.files {
            let bytes = self.files.get(&file.path).ok_or_else(|| StoreError(format!("{} isn't available", file.path)))?;
            done = done.saturating_add(file.size);
            progress(Progress { done, total });
            out.push(Downloaded { path: file.path.clone(), bytes: bytes.clone() });
        }
        Ok(out)
    }

    fn in_store(&self, path: &Path) -> bool {
        path.starts_with(&self.dir)
    }
}

impl Installer for FakeStore {
    fn downloads_allowed(&self) -> bool {
        self.allowed.load(Ordering::SeqCst)
    }

    fn allow_downloads(&self, allowed: bool) -> Result<(), StoreError> {
        self.allowed.store(allowed, Ordering::SeqCst);
        Ok(())
    }

    fn install(&self, _: &Index, family: &Family, progress: &mut dyn FnMut(Progress)) -> Result<(), StoreError> {
        self.record(Call::Install(family.family.clone()));
        let files = self.fetch(family, progress)?;
        let dir = self.dir.join(&family.family);
        std::fs::create_dir_all(&dir).map_err(err)?;
        for file in files {
            let name = file.path.rsplit('/').next().unwrap_or(&file.path);
            std::fs::write(dir.join(name), &file.bytes).map_err(err)?;
        }
        Ok(())
    }

    fn remove(&self, request: &RemoveRequest) -> Result<(), StoreError> {
        self.record(Call::Remove(request.family.clone()));
        if let Some(outside) = request.files.iter().find(|f| !self.in_store(f)) {
            return Err(StoreError(format!("{} isn't in the font store", outside.display())));
        }
        for file in &request.files {
            std::fs::remove_file(file).map_err(err)?;
        }
        Ok(())
    }

    fn download(&self, _: &Index, family: &Family, progress: &mut dyn FnMut(Progress)) -> Result<Vec<Downloaded>, StoreError> {
        self.record(Call::Download(family.family.clone()));
        self.fetch(family, progress)
    }
}
