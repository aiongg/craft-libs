//! Faces made from a catalog's fonts, for an app that shapes and draws with its own face objects
//! (skrifa, harfrust): each face of a font file is made once, kept while the catalog lists its
//! file unchanged, and let go when a rescan finds the file changed or removed, or when the scope
//! or the added data it came from is removed.
//!
//! Nothing here knows the app's face type `F`.

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::scan::{FileStamp, file_stamp};
use crate::{Catalog, FaceInfo, FontData, FontFileError};

/// Which face of which font data a cached face was made from.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum FaceKey {
    /// A face of a font file: the file and the face's number in it ([`FaceInfo::index`]).
    File(PathBuf, u32),
    /// A face of font data added to the catalog as bytes: the catalog's number for the data,
    /// never given to other data, and the face's number in it.
    Added(u64, u32),
}

impl FaceKey {
    fn of(info: &FaceInfo) -> Option<Self> {
        match (info.blob, &info.path) {
            (Some(blob), _) => Some(Self::Added(blob, info.index)),
            (None, Some(path)) => Some(Self::File(path.clone(), info.index)),
            (None, None) => None,
        }
    }
}

struct Cached<F> {
    face: Arc<F>,
    /// The file's stamp in the listing the face was made for ([`FaceInfo::file`]); `None` for
    /// added data.
    stamp: Option<FileStamp>,
}

struct State<F> {
    faces: HashMap<FaceKey, Cached<F>>,
    /// The catalog's [`Catalog::face_changes`] the faces were last checked at.
    checked: Option<(u64, u64)>,
    /// Faces [`FaceCache::get_or_make`] made again because the catalog lists their file changed,
    /// for the next [`FaceCache::refresh`] to return.
    replaced: Vec<(FaceKey, Arc<F>)>,
}

/// The app's faces (`F`) made from a [`Catalog`]'s fonts, by font file and face index, or by
/// added data and face index.
///
/// One cache holds faces of the installed fonts, of fonts added without a scope and of every
/// scope: [`FaceCache::refresh`] keeps a face while any of them lists its file unchanged, so a
/// file that two documents' scopes (or a scope and the installed fonts) share has one face. The
/// key is the file and the face index alone, so an app whose faces differ by where they came
/// from (a document face tagged with its document) keeps one cache for each kind.
pub struct FaceCache<F> {
    state: Mutex<State<F>>,
}

impl<F> Default for FaceCache<F> {
    fn default() -> Self {
        Self { state: Mutex::new(State { faces: HashMap::new(), checked: None, replaced: Vec::new() }) }
    }
}

impl<F> FaceCache<F> {
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> MutexGuard<'_, State<F>> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The face made from `info`'s data: the one made before for the same file and face index,
    /// else `make` (called without locks held) from the data `catalog` loads. `make` gets the
    /// data's own index ([`FontData::index`], which differs from [`FaceInfo::index`] for a Mac
    /// suitcase font); `None` from it is [`FontFileError::NotAFont`]. Two threads making the same
    /// face at once both make it, and both get the first one cached.
    ///
    /// When `catalog` lists the file changed since its face was made (a rescan, or the file
    /// added to a scope again), the face is made again, and the next [`FaceCache::refresh`]
    /// returns the old one. An `info` looked up before that change gets the face made for the
    /// file as it is now.
    pub fn get_or_make(
        &self,
        catalog: &Catalog,
        info: &FaceInfo,
        make: impl FnOnce(&FaceInfo, &FontData) -> Option<F>,
    ) -> Result<Arc<F>, FontFileError> {
        let key = FaceKey::of(info).ok_or(FontFileError::Gone)?;
        if let Some(face) = self.cached(&key, info) {
            return Ok(face);
        }
        // The catalog reads a file replaced since it last read it again.
        let data = catalog.load(info)?;
        if let Some(face) = self.cached(&key, info) {
            return Ok(face);
        }
        let face = Arc::new(make(info, &data).ok_or(FontFileError::NotAFont)?);
        let mut state = self.lock();
        let State { faces, replaced, .. } = &mut *state;
        match faces.entry(key) {
            Entry::Occupied(c) if c.get().stamp == info.file => Ok(Arc::clone(&c.get().face)),
            Entry::Occupied(mut c) => {
                let old = c.insert(Cached { face: Arc::clone(&face), stamp: info.file });
                replaced.push((c.key().clone(), old.face));
                Ok(face)
            }
            Entry::Vacant(c) => Ok(Arc::clone(&c.insert(Cached { face, stamp: info.file }).face)),
        }
    }

    /// The cached face for `key` when it was made for the file `info` lists, or for the file as
    /// it is now (`info` came from a listing older than the face's).
    fn cached(&self, key: &FaceKey, info: &FaceInfo) -> Option<Arc<F>> {
        let made_for = {
            let state = self.lock();
            let c = state.faces.get(key)?;
            if c.stamp == info.file {
                return Some(Arc::clone(&c.face));
            }
            c.stamp
        };
        let now = std::fs::metadata(info.path.as_deref()?).ok().map(|m| file_stamp(&m));
        if made_for.is_none() || now != made_for {
            return None;
        }
        self.lock().faces.get(key).filter(|c| c.stamp == made_for).map(|c| Arc::clone(&c.face))
    }

    /// Let go of the faces whose file `catalog` no longer lists, or lists changed, and of those
    /// whose added data or scope was removed, and return them with the faces
    /// [`FaceCache::get_or_make`] made again (for the app to drop what it made from them). A
    /// face stays while the installed fonts, the fonts added without a scope or any scope list
    /// its file as it was. Nothing to do while nothing was added to or removed from `catalog`
    /// and no rescan changed it; it never waits for a scan and reads no files.
    pub fn refresh(&self, catalog: &Catalog) -> Vec<(FaceKey, Arc<F>)> {
        let changes = catalog.face_changes();
        let judged: Vec<(FaceKey, Option<FileStamp>, Arc<F>)> = {
            let mut state = self.lock();
            if state.checked == Some(changes) {
                return std::mem::take(&mut state.replaced);
            }
            state.faces.iter().map(|(k, c)| (k.clone(), c.stamp, Arc::clone(&c.face))).collect()
        };
        let mut by_path: HashMap<&Path, Vec<usize>> = HashMap::new();
        let mut by_blob: HashMap<u64, Vec<usize>> = HashMap::new();
        for (n, (key, ..)) in judged.iter().enumerate() {
            match key {
                FaceKey::File(path, _) => by_path.entry(path).or_default().push(n),
                FaceKey::Added(blob, _) => by_blob.entry(*blob).or_default().push(n),
            }
        }
        let mut keep = vec![false; judged.len()];
        let index = |key: &FaceKey| match key {
            FaceKey::File(_, i) | FaceKey::Added(_, i) => *i,
        };
        let changes = catalog.visit_faces(|info| {
            let found = match (info.blob, &info.path) {
                (Some(blob), _) => by_blob.get(&blob),
                (None, Some(path)) => by_path.get(path.as_path()),
                (None, None) => None,
            };
            for &n in found.into_iter().flatten() {
                let (key, stamp, _) = &judged[n];
                if index(key) == info.index && (info.blob.is_some() || *stamp == info.file) {
                    keep[n] = true;
                }
            }
        });
        let mut state = self.lock();
        state.checked = Some(changes);
        let mut gone = std::mem::take(&mut state.replaced);
        for ((key, _, face), keep) in judged.into_iter().zip(keep) {
            // A face made again meanwhile was made for the listing as it is now.
            if !keep && state.faces.get(&key).is_some_and(|c| Arc::ptr_eq(&c.face, &face)) {
                state.faces.remove(&key);
                gone.push((key, face));
            }
        }
        gone
    }

    /// Let go of the faces made from the font file at `path` if it changed (its length or
    /// modification time) since the listing they were made for, and return them: the next
    /// [`FaceCache::get_or_make`] reads the file as it is now. For an app that learns of a
    /// change from its own file watcher, before the catalog lists the file again.
    pub fn forget_changed(&self, path: &Path) -> Vec<(FaceKey, Arc<F>)> {
        let now = std::fs::metadata(path).ok().map(|m| file_stamp(&m));
        let mut state = self.lock();
        let changed: Vec<FaceKey> =
            state.faces.iter().filter(|(k, c)| matches!(k, FaceKey::File(p, _) if p == path) && c.stamp != now).map(|(k, _)| k.clone()).collect();
        changed.into_iter().filter_map(|key| state.faces.remove(&key).map(|c| (key, c.face))).collect()
    }
}
