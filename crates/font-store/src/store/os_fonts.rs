//! "Available to all apps": copies of a store family in the OS user font folder, recorded in a
//! ledger (`os-fonts.json`) so that withdrawing one removes exactly what the store added.
//!
//! Crash safety: the ledger lists a file as `adding` before it is copied, and as `active` once
//! the copy is in place. Copies go to a dot-file (`.craft-<name>.part`, which neither fontconfig
//! nor macOS reads) and are renamed into place. The next call that changes the ledger settles
//! every `adding` entry: the temporary file goes; a file that has the new sha256 (or, for a
//! replacement, the old one) stays listed as `active`; any other is dropped from the ledger and
//! left alone. A withdrawal deletes files first and rewrites the ledger after, so a crash in
//! between leaves entries whose files are gone, which the next withdrawal drops.

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::path::{Component, Path, PathBuf};

use craft_font_catalog::{Catalog, FontEnv, Os, Source};
use serde::{Deserialize, Serialize};
use skrifa::MetadataProvider;
use skrifa::raw::FileRef;
use skrifa::string::StringId;

use super::{Caller, Error, FontStore, InstalledFamily, io, read_limited};
use crate::verify::{VerifyError, sha256_hex};

const LEDGER: &str = "os-fonts.json";
const MAX_LEDGER_BYTES: u64 = 16 << 20;

/// How the OS user font folder is laid out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OsFontLayout {
    /// One folder of files, each named `Craft-<file>`: `~/Library/Fonts`.
    Flat,
    /// A folder per family, `<folder>/<store family folder>/<file>`: `~/.local/share/fonts/Craft`,
    /// which fontconfig reads with its subfolders.
    Folders,
}

/// The OS user font folder that "available to all apps" copies families into.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OsFontFolder {
    dir: PathBuf,
    layout: OsFontLayout,
    fc_cache: bool,
}

impl OsFontFolder {
    /// The current user's font folder on the system `env` describes, or why the store can't add
    /// fonts to it:
    ///
    /// - macOS: `~/Library/Fonts`. Not in a sandboxed build (`APP_SANDBOX_CONTAINER_ID` is set),
    ///   such as the Mac App Store's.
    /// - Linux and the BSDs: `$XDG_DATA_HOME/fonts/Craft` (`~/.local/share/fonts/Craft`), and
    ///   `fc-cache` refreshes the user's font cache after each change when it is on `PATH`. In a
    ///   Flatpak, the host's folder (`$HOST_XDG_DATA_HOME`, else `~/.local/share`), only with the
    ///   `xdg-data/fonts` permission, and without `fc-cache`. Not in a Snap (`SNAP` is set).
    /// - Windows (for now), Haiku and the web: not offered.
    pub fn for_env(env: &FontEnv) -> Result<Self, String> {
        const NO_HOME: &str = "there is no home folder";
        match env.os {
            Os::MacOs => {
                if std::env::var_os("APP_SANDBOX_CONTAINER_ID").is_some() {
                    return Err("a sandboxed app (such as the Mac App Store build) can't add fonts to ~/Library/Fonts".into());
                }
                let home = env.home.as_ref().ok_or(NO_HOME)?;
                Ok(Self { dir: home.join("Library/Fonts"), layout: OsFontLayout::Flat, fc_cache: false })
            }
            Os::Unix => {
                if std::env::var_os("SNAP").is_some() {
                    return Err("the Snap build can't write the user font folder".into());
                }
                let absolute = |p: &PathBuf| p.is_absolute();
                let local_share = env.home.as_ref().map(|h| h.join(".local/share"));
                let (data, fc_cache) = if env.flatpak {
                    if !flatpak_can_write_fonts(&env.root.join(".flatpak-info")) {
                        return Err("the Flatpak has no write access to the user font folder (--filesystem=xdg-data/fonts:create)".into());
                    }
                    (std::env::var_os("HOST_XDG_DATA_HOME").map(PathBuf::from).filter(absolute).or(local_share), false)
                } else {
                    (env.xdg_data_home.clone().filter(absolute).or(local_share), true)
                };
                Ok(Self { dir: data.ok_or(NO_HOME)?.join("fonts/Craft"), layout: OsFontLayout::Folders, fc_cache })
            }
            Os::Windows => Err("making fonts available to all apps isn't supported on Windows yet".into()),
            Os::Haiku | Os::None => Err("this system has no user font folder the store can add to".into()),
        }
    }

    /// A folder of `layout` at `dir`, with no font cache refresh: tests, and setups the platform
    /// rules don't cover.
    pub fn at(dir: impl Into<PathBuf>, layout: OsFontLayout) -> Self {
        Self { dir: dir.into(), layout, fc_cache: false }
    }

    /// The folder.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn path_for(&self, family: &InstalledFamily, file: &str) -> PathBuf {
        match self.layout {
            OsFontLayout::Flat => self.dir.join(format!("Craft-{file}")),
            OsFontLayout::Folders => self.dir.join(&family.dir).join(file),
        }
    }

    /// Whether `path` is a file inside the folder, with no `..` or other odd component: the only
    /// paths from the ledger the store touches.
    fn holds(&self, path: &Path) -> bool {
        path.strip_prefix(&self.dir).is_ok_and(|rest| rest.components().count() > 0 && rest.components().all(|c| matches!(c, Component::Normal(_))))
    }

    /// Tells fontconfig, off the calling thread. It isn't needed for correctness: fontconfig
    /// notices changed folders by their mtime.
    fn refresh(&self) {
        if !self.fc_cache {
            return;
        }
        let fonts = self.dir.parent().unwrap_or(&self.dir).to_path_buf();
        let _ = std::thread::Builder::new().name("fc-cache".into()).spawn(move || {
            let _ = std::process::Command::new("fc-cache")
                .arg(fonts)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
        });
    }
}

/// Whether `/.flatpak-info` grants write access to `~/.local/share/fonts`.
fn flatpak_can_write_fonts(info: &Path) -> bool {
    let Some(bytes) = read_limited(info, 1 << 20) else { return false };
    let text = String::from_utf8_lossy(&bytes);
    let granted = ["host", "home", "xdg-data", "xdg-data/fonts", "~/.local", "~/.local/share", "~/.local/share/fonts"];
    text.lines().filter_map(|l| l.trim().strip_prefix("filesystems=")).flat_map(|l| l.split(';')).any(|entry| {
        let entry = entry.trim();
        let (name, mode) = entry.rsplit_once(':').unwrap_or((entry, "rw"));
        mode != "ro" && granted.contains(&name.trim_end_matches('/'))
    })
}

/// A font the system already has under a PostScript name of the family being made available.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conflict {
    /// The PostScript name.
    pub postscript: String,
    /// The file that has it. For a file the store didn't add that sits where a copy would go,
    /// that file.
    pub path: PathBuf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum State {
    /// Listed before the copy; settled by the next change.
    Adding,
    Active,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Mechanism {
    Copy,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct LedgerFile {
    /// The file's name in the store family's folder.
    store_file: String,
    /// The copy in the OS folder.
    path: PathBuf,
    /// The copy's sha256: the store file's.
    sha256: String,
    postscript: Vec<String>,
    mechanism: Mechanism,
    state: State,
    /// While `adding` over an earlier copy: that copy's sha256.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    replacing: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct LedgerFamily {
    family: String,
    source: String,
    files: Vec<LedgerFile>,
}

impl LedgerFamily {
    fn is(&self, source: &str, name: &str) -> bool {
        self.source == source && self.family.to_lowercase() == name.to_lowercase()
    }
}

#[derive(Default, Serialize, Deserialize)]
struct Ledger {
    version: u32,
    families: Vec<LedgerFamily>,
}

impl Ledger {
    fn family(&self, source: &str, name: &str) -> Option<&LedgerFamily> {
        self.families.iter().find(|f| f.is(source, name))
    }

    /// Settles every `adding` entry (see the module comment).
    fn settle(&mut self, folder: &OsFontFolder) {
        for family in &mut self.families {
            family.files.retain_mut(|f| {
                if f.state == State::Active {
                    return true;
                }
                if !folder.holds(&f.path) {
                    return false;
                }
                let _ = std::fs::remove_file(part_path(&f.path));
                match file_sha256(&f.path) {
                    Some(sha) if sha == f.sha256 => {}
                    Some(sha) if f.replacing.as_ref() == Some(&sha) => f.sha256 = sha,
                    _ => return false,
                }
                f.state = State::Active;
                f.replacing = None;
                true
            });
        }
        self.families.retain(|f| !f.files.is_empty());
    }
}

/// Where a copy is written before it is renamed into place.
fn part_path(path: &Path) -> PathBuf {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    path.with_file_name(format!(".craft-{name}.part"))
}

/// The sha256 of the file at `path`, read in pieces; `None` when it can't be read.
fn file_sha256(path: &Path) -> Option<String> {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    std::io::copy(&mut File::open(path).ok()?, &mut hasher).ok()?;
    Some(hasher.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// The PostScript name of each face in `bytes`.
fn postscript_names(bytes: &[u8]) -> Vec<String> {
    let Ok(file) = FileRef::new(bytes) else { return Vec::new() };
    file.fonts()
        .flatten()
        .filter_map(|f| f.localized_strings(StringId::POSTSCRIPT_NAME).english_or_first().map(|s| s.to_string()))
        .filter(|s| !s.is_empty())
        .collect()
}

/// A store file to copy.
struct Planned {
    store_file: String,
    source: PathBuf,
    path: PathBuf,
    sha256: String,
    postscript: Vec<String>,
}

impl FontStore {
    /// Uses `folder` as the OS user font folder. [`FontStore::for_user`] and
    /// [`FontStore::for_agent`] use [`OsFontFolder::for_env`]; [`FontStore::new`] has none.
    pub fn with_os_fonts(mut self, folder: OsFontFolder) -> Self {
        self.os_fonts = Ok(folder);
        self
    }

    pub(super) fn set_os_fonts(&mut self, folder: Result<OsFontFolder, String>) {
        self.os_fonts = folder;
    }

    /// Whether families can be made available to all apps here. The interface hides the action
    /// when it is false.
    pub fn available_to_all_supported(&self) -> bool {
        self.os_fonts.is_ok()
    }

    /// Why families can't be made available to all apps here; `None` when they can.
    pub fn available_to_all_unsupported(&self) -> Option<&str> {
        self.os_fonts.as_ref().err().map(String::as_str)
    }

    /// The OS user font folder, when there is one.
    pub fn os_fonts(&self) -> Option<&OsFontFolder> {
        self.os_fonts.as_ref().ok()
    }

    /// Whether the family of this name installed from `source` is available to all apps.
    pub fn is_available(&self, source: &str, name: &str) -> bool {
        !self.available_files(source, name).is_empty()
    }

    /// The copies of the family's files in the OS user font folder.
    pub fn available_files(&self, source: &str, name: &str) -> Vec<PathBuf> {
        self.read_ledger().family(source, name).map(|f| f.files.iter().map(|x| x.path.clone()).collect()).unwrap_or_default()
    }

    /// Makes the installed family available to all apps, from the user's own action: copies its
    /// font files into the OS user font folder and records them in the ledger. Calling it again
    /// brings the copies up to date with the store. Returns the copies.
    ///
    /// Refuses with [`Error::Conflict`] when a font the store didn't add has a PostScript name of
    /// the family, or sits where a copy would go; a file identical to the store's is left as it
    /// is and not recorded. [`Error::NotAllowed`] for an agent, [`Error::Unsupported`] where
    /// [`FontStore::available_to_all_supported`] is false.
    ///
    /// Running apps see the fonts at once or after a restart, depending on the app.
    pub fn make_available(&self, source: &str, name: &str) -> Result<Vec<PathBuf>, Error> {
        if self.caller != Caller::User {
            return Err(Error::NotAllowed);
        }
        self.share(source, name)
    }

    /// Takes the family's copies out of the OS user font folder, from the user's own action. Only
    /// files the ledger lists go, and only while unchanged: returns the ones left in place because
    /// they changed since (or lie outside the folder). [`Error::NotAllowed`] for an agent.
    pub fn withdraw(&self, source: &str, name: &str) -> Result<Vec<PathBuf>, Error> {
        if self.caller != Caller::User {
            return Err(Error::NotAllowed);
        }
        let folder = self.os_fonts.as_ref().map_err(|reason| Error::Unsupported(reason.clone()))?;
        let kept = {
            let _lock = self.lock()?;
            self.unshare_locked(folder, source, name, None)?
        };
        folder.refresh();
        self.rescan();
        Ok(kept)
    }

    /// After an install: brings the copies of an available family up to date. A failure leaves
    /// the earlier copies listed.
    pub(super) fn reshare(&self, source: &str, name: &str) {
        if self.is_available(source, name) {
            let _ = self.share(source, name);
        }
    }

    /// While the store is locked: withdraws the family's copies (`files` only, by their store
    /// names, when given) before it leaves the store. Nothing to do without an OS folder.
    pub(super) fn withdraw_for_removal(&self, source: &str, name: &str, files: Option<&[String]>) -> Result<(), Error> {
        let Ok(folder) = &self.os_fonts else { return Ok(()) };
        if self.read_ledger().family(source, name).is_none() {
            return Ok(());
        }
        self.unshare_locked(folder, source, name, files)?;
        folder.refresh();
        Ok(())
    }

    fn share(&self, source: &str, name: &str) -> Result<Vec<PathBuf>, Error> {
        let folder = self.os_fonts.as_ref().map_err(|reason| Error::Unsupported(reason.clone()))?;
        let family = self.installed_family(source, name).ok_or_else(|| Error::NotInstalled(name.to_string()))?;
        let store_dir = self.dir.join(&family.dir);
        let mut planned = Vec::new();
        for f in &family.files {
            let from = store_dir.join(&f.name);
            let bytes = std::fs::read(&from).map_err(io(format!("reading {}", f.name)))?;
            if sha256_hex(&bytes) != f.sha256 {
                return Err(Error::Verify { file: f.name.clone(), error: VerifyError::Sha256 });
            }
            let mut postscript = postscript_names(&bytes);
            postscript.extend(f.postscript.iter().cloned());
            postscript.sort();
            postscript.dedup();
            planned.push(Planned {
                store_file: f.name.clone(),
                source: from,
                path: folder.path_for(&family, &f.name),
                sha256: f.sha256.clone(),
                postscript,
            });
        }
        let ours: HashSet<PathBuf> = self.read_ledger().families.iter().flat_map(|f| f.files.iter().map(|x| x.path.clone())).collect();
        let (mut conflicts, present) = self.system_conflicts(&planned, &ours);
        planned.retain(|p| !present.contains(&p.store_file));

        let copies = {
            let _lock = self.lock()?;
            let mut ledger = self.read_ledger();
            ledger.settle(folder);
            let earlier: Vec<LedgerFile> = ledger.family(source, name).map(|f| f.files.clone()).unwrap_or_default();
            let on_disk: Vec<Option<String>> = planned.iter().map(|p| file_sha256(&p.path)).collect();
            // A file where a copy goes is ours when the ledger lists it for this family unchanged,
            // or when it is the store's file already (a copy the ledger lost), unless another
            // family's copy is there.
            for (p, disk) in planned.iter().zip(&on_disk) {
                let others = ledger.families.iter().any(|f| !f.is(source, name) && f.files.iter().any(|x| x.path == p.path));
                let blocked = others
                    || match disk {
                        Some(sha) => *sha != p.sha256 && !earlier.iter().any(|e| e.path == p.path && e.sha256 == *sha),
                        None => p.path.exists(),
                    };
                if blocked {
                    conflicts.push(Conflict { postscript: p.postscript.first().cloned().unwrap_or_default(), path: p.path.clone() });
                }
            }
            if !conflicts.is_empty() {
                conflicts.sort_by(|a, b| (&a.postscript, &a.path).cmp(&(&b.postscript, &b.path)));
                conflicts.dedup();
                return Err(Error::Conflict(conflicts));
            }

            let mut files = Vec::new();
            let mut to_copy = Vec::new();
            for (p, disk) in planned.iter().zip(&on_disk) {
                let earlier_copy = earlier.iter().find(|e| e.path == p.path);
                let state = if disk.as_ref() == Some(&p.sha256) {
                    State::Active
                } else {
                    to_copy.push(p);
                    State::Adding
                };
                files.push(LedgerFile {
                    store_file: p.store_file.clone(),
                    path: p.path.clone(),
                    sha256: p.sha256.clone(),
                    postscript: p.postscript.clone(),
                    mechanism: Mechanism::Copy,
                    state,
                    replacing: earlier_copy.filter(|_| state == State::Adding).map(|e| e.sha256.clone()),
                });
            }
            // Earlier copies the store no longer has (or the system has itself) stay listed until
            // they are deleted below.
            let stale: Vec<LedgerFile> = earlier.into_iter().filter(|e| !files.iter().any(|f| f.path == e.path)).collect();
            files.extend(stale.iter().cloned());
            ledger.families.retain(|f| !f.is(source, name));
            ledger.families.push(LedgerFamily { family: family.family.clone(), source: family.source.clone(), files });
            ledger.families.retain(|f| !f.files.is_empty());
            self.write_ledger(&ledger)?;

            if let Err(e) = to_copy.into_iter().try_for_each(copy_in) {
                ledger.settle(folder);
                let _ = self.write_ledger(&ledger);
                return Err(e);
            }
            if let Some(entry) = ledger.families.iter_mut().find(|f| f.is(source, name)) {
                for f in &mut entry.files {
                    f.state = State::Active;
                    f.replacing = None;
                }
                // A stale copy leaves the ledger once deleted, or once changed and so not ours.
                entry.files.retain(|f| !stale.iter().any(|s| s.path == f.path) || remove_if_unchanged(folder, f).is_err());
            }
            ledger.families.retain(|f| !f.files.is_empty());
            ledger.families.sort_by_key(|f| (f.family.to_lowercase(), f.source.clone()));
            self.write_ledger(&ledger)?;
            prune_dirs(folder, stale.iter().map(|s| s.path.as_path()));
            ledger.family(source, name).map(|f| f.files.iter().map(|x| x.path.clone()).collect()).unwrap_or_default()
        };
        folder.refresh();
        self.rescan();
        Ok(copies)
    }

    /// Faces of the system (not the store's, nor the copies the ledger lists) with a PostScript
    /// name of `planned`: conflicts, and the store files the system already has identical copies of.
    fn system_conflicts(&self, planned: &[Planned], ours: &HashSet<PathBuf>) -> (Vec<Conflict>, HashSet<String>) {
        // Not `faces()`: it keeps one of two faces alike, which may be the store's.
        let faces = match &self.catalog {
            Some(catalog) => {
                catalog.rescan_if_changed();
                catalog.scanned_faces()
            }
            None => Catalog::new(FontEnv::from_process()).scanned_faces(),
        };
        let mut wanted: HashMap<String, Vec<&Planned>> = HashMap::new();
        for p in planned {
            for ps in &p.postscript {
                wanted.entry(ps.to_ascii_lowercase()).or_default().push(p);
            }
        }
        let mut hashes: HashMap<PathBuf, Option<String>> = HashMap::new();
        let mut conflicts = Vec::new();
        let mut present = HashSet::new();
        for face in faces.iter().filter(|f| f.source == Source::System) {
            let (Some(ps), Some(path)) = (&face.postscript_name, &face.path) else { continue };
            if ours.contains(path) {
                continue;
            }
            for p in wanted.get(&ps.to_ascii_lowercase()).into_iter().flatten() {
                let sha = hashes.entry(path.clone()).or_insert_with(|| file_sha256(path));
                if sha.as_ref() == Some(&p.sha256) {
                    present.insert(p.store_file.clone());
                } else {
                    conflicts.push(Conflict { postscript: ps.clone(), path: path.clone() });
                }
            }
        }
        // A file identical to the store's doesn't excuse another face's conflict.
        (conflicts, present)
    }

    fn unshare_locked(&self, folder: &OsFontFolder, source: &str, name: &str, only: Option<&[String]>) -> Result<Vec<PathBuf>, Error> {
        let mut ledger = self.read_ledger();
        ledger.settle(folder);
        let mut kept = Vec::new();
        let mut removed = Vec::new();
        let mut failed = None;
        if let Some(entry) = ledger.families.iter_mut().find(|f| f.is(source, name)) {
            entry.files.retain(|f| {
                if only.is_some_and(|o| !o.contains(&f.store_file)) {
                    return true;
                }
                match remove_if_unchanged(folder, f) {
                    Ok(true) => removed.push(f.path.clone()),
                    Ok(false) => kept.push(f.path.clone()),
                    Err(e) => {
                        failed.get_or_insert(e);
                        return true;
                    }
                }
                false
            });
        }
        ledger.families.retain(|f| !f.files.is_empty());
        self.write_ledger(&ledger)?;
        prune_dirs(folder, removed.iter().map(PathBuf::as_path));
        match failed {
            Some(e) => Err(e),
            None => Ok(kept),
        }
    }

    /// The ledger. One that can't be read is kept aside as `os-fonts.json.bad` and counts as
    /// empty: the copies it listed are then the system's, which the conflict check reports.
    fn read_ledger(&self) -> Ledger {
        let path = self.dir.join(LEDGER);
        let Some(bytes) = read_limited(&path, MAX_LEDGER_BYTES) else { return Ledger::default() };
        serde_json::from_slice(&bytes).unwrap_or_else(|_| {
            let _ = std::fs::rename(&path, self.dir.join("os-fonts.json.bad"));
            Ledger::default()
        })
    }

    fn write_ledger(&self, ledger: &Ledger) -> Result<(), Error> {
        let ledger = Ledger { version: 1, families: ledger.families.clone() };
        let bytes = serde_json::to_vec_pretty(&ledger).map_err(|e| io("saving the OS font ledger")(std::io::Error::other(e)))?;
        self.write_file(LEDGER, &bytes)
    }
}

/// Copies a store file to its place through a dot-file, checking the copy's sha256.
fn copy_in(p: &Planned) -> Result<(), Error> {
    let what = || format!("copying {} to {}", p.store_file, p.path.display());
    if let Some(parent) = p.path.parent() {
        std::fs::create_dir_all(parent).map_err(io(what()))?;
    }
    let part = part_path(&p.path);
    std::fs::copy(&p.source, &part).map_err(io(what()))?;
    let checked = File::open(&part).and_then(|f| f.sync_all()).map_err(io(what())).and_then(|()| {
        if file_sha256(&part).as_ref() == Some(&p.sha256) {
            Ok(())
        } else {
            Err(Error::Verify { file: p.store_file.clone(), error: VerifyError::Sha256 })
        }
    });
    if let Err(e) = checked.and_then(|()| std::fs::rename(&part, &p.path).map_err(io(what()))) {
        let _ = std::fs::remove_file(&part);
        return Err(e);
    }
    Ok(())
}

/// Deletes a listed copy when it is unchanged: `Ok(true)` when it is gone (or already was),
/// `Ok(false)` when it is left in place.
fn remove_if_unchanged(folder: &OsFontFolder, f: &LedgerFile) -> Result<bool, Error> {
    if !folder.holds(&f.path) {
        return Ok(false);
    }
    let _ = std::fs::remove_file(part_path(&f.path));
    match file_sha256(&f.path) {
        None if !f.path.exists() => Ok(true),
        Some(sha) if sha == f.sha256 => std::fs::remove_file(&f.path).map(|()| true).map_err(io(format!("removing {}", f.path.display()))),
        _ => Ok(false),
    }
}

/// Removes the folders `removed` files leave empty, up to the OS folder itself.
fn prune_dirs<'a>(folder: &OsFontFolder, removed: impl Iterator<Item = &'a Path>) {
    for path in removed {
        let mut dir = path.parent();
        while let Some(d) = dir.filter(|d| *d != folder.dir && d.starts_with(&folder.dir)) {
            if std::fs::remove_dir(d).is_err() {
                break;
            }
            dir = d.parent();
        }
    }
}
