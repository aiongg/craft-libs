//! [`Files`]: the file service of a [`Host`](crate::Host). Features pick, read and write files
//! through it; craft-dialogs implements it over the native panels, apps on wasm over the browser.
//!
//! Pickers never block the frame: [`Files::pick`] returns a [`PickId`] at once and the answer
//! arrives on a later pass through [`Files::take_answer`]. One picker shows at a time; later
//! requests wait in order. The service remembers the last folder per [`PickRequest::purpose`]
//! and applies the extension policy to a saved name before the app sees it.

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// What a picker asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PickKind {
    /// One existing file.
    Open,
    /// Several existing files.
    OpenMany,
    /// A folder.
    Folder,
    /// A new or existing file to write.
    Save,
}

/// One entry of a picker's type list: a display name and its extensions, lowercase, without
/// the dot (`("JPEG", ["jpg", "jpeg"])`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileFilter {
    pub name: String,
    pub extensions: Vec<String>,
}

impl FileFilter {
    pub fn new(name: impl Into<String>, extensions: &[&str]) -> Self {
        Self { name: name.into(), extensions: extensions.iter().map(|e| e.trim_start_matches('.').to_ascii_lowercase()).collect() }
    }
}

/// One picker request. Built with the builder methods and handed to [`Files::pick`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PickRequest {
    pub kind: PickKind,
    /// Groups requests that share a remembered folder (`"document"`, `"export"`, `"swatches"`).
    /// The picker opens in the folder the last answered request of this purpose chose.
    pub purpose: String,
    pub title: Option<String>,
    pub filters: Vec<FileFilter>,
    /// Starting folder when the purpose has no remembered folder.
    pub directory: Option<PathBuf>,
    /// Suggested name for [`PickKind::Save`].
    pub file_name: Option<String>,
    /// The extension a Save answer gets when the user typed none (`"png"`), lowercase.
    pub suggested_extension: Option<String>,
}

impl PickRequest {
    pub fn new(kind: PickKind, purpose: impl Into<String>) -> Self {
        Self { kind, purpose: purpose.into(), title: None, filters: Vec::new(), directory: None, file_name: None, suggested_extension: None }
    }

    pub fn open(purpose: impl Into<String>) -> Self {
        Self::new(PickKind::Open, purpose)
    }

    pub fn open_many(purpose: impl Into<String>) -> Self {
        Self::new(PickKind::OpenMany, purpose)
    }

    pub fn folder(purpose: impl Into<String>) -> Self {
        Self::new(PickKind::Folder, purpose)
    }

    pub fn save(purpose: impl Into<String>) -> Self {
        Self::new(PickKind::Save, purpose)
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn filter(mut self, name: impl Into<String>, extensions: &[&str]) -> Self {
        self.filters.push(FileFilter::new(name, extensions));
        self
    }

    pub fn directory(mut self, directory: impl Into<PathBuf>) -> Self {
        self.directory = Some(directory.into());
        self
    }

    pub fn file_name(mut self, file_name: impl Into<String>) -> Self {
        self.file_name = Some(file_name.into());
        self
    }

    pub fn suggested_extension(mut self, extension: impl Into<String>) -> Self {
        self.suggested_extension = Some(extension.into().trim_start_matches('.').to_ascii_lowercase());
        self
    }
}

/// The ticket of a pending pick; the answer carries it back.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PickId(pub u64);

/// What the user chose. On wasm a chosen file has no path the app could reopen, so an Open
/// answer carries the bytes too and the path is the browser's file name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Picked {
    /// The user cancelled.
    Cancelled,
    /// One or more paths. For [`PickKind::Save`] the extension policy has been applied, and the
    /// service has already asked about replacing an existing file the OS panel did not check.
    Paths(Vec<PathBuf>),
    /// Files read by the browser (wasm): name and contents.
    Contents(Vec<(String, Vec<u8>)>),
}

impl Picked {
    /// The first chosen path; on wasm the first file's name. `None` when cancelled.
    pub fn first_path(&self) -> Option<PathBuf> {
        match self {
            Picked::Paths(paths) => paths.first().cloned(),
            Picked::Contents(files) => files.first().map(|(name, _)| PathBuf::from(name)),
            Picked::Cancelled => None,
        }
    }

    /// Every chosen path; on wasm the files' names. Empty when cancelled.
    pub fn paths(&self) -> Vec<PathBuf> {
        match self {
            Picked::Paths(paths) => paths.clone(),
            Picked::Contents(files) => files.iter().map(|(name, _)| PathBuf::from(name)).collect(),
            Picked::Cancelled => Vec::new(),
        }
    }
}

/// One answered pick.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PickAnswer {
    pub id: PickId,
    pub request: PickRequest,
    pub picked: Picked,
}

/// Why a read or write failed. The message reads the same in every app (F15.06).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileError {
    NotFound(PathBuf),
    PermissionDenied(PathBuf),
    /// The service cannot do this on the platform (write a path on wasm).
    Unsupported,
    Other(String),
}

impl fmt::Display for FileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileError::NotFound(p) => write!(f, "“{}” was not found", p.display()),
            FileError::PermissionDenied(p) => write!(f, "“{}” cannot be accessed", p.display()),
            FileError::Unsupported => f.write_str("not available on this platform"),
            FileError::Other(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for FileError {}

impl From<std::io::Error> for FileError {
    fn from(e: std::io::Error) -> Self {
        match e.kind() {
            std::io::ErrorKind::NotFound => FileError::NotFound(PathBuf::new()),
            std::io::ErrorKind::PermissionDenied => FileError::PermissionDenied(PathBuf::new()),
            _ => FileError::Other(e.to_string()),
        }
    }
}

/// The file service. Pickers are queued and answered later; reads and writes are synchronous
/// (features run large ones through [`Jobs`](crate::Jobs)).
pub trait Files: Send + Sync {
    /// Queues a picker; the answer comes back from [`take_answer`](Files::take_answer) on a
    /// later pass. The ticket lets a feature match the answer to what it asked.
    fn pick(&self, request: PickRequest) -> PickId;
    /// The answer of ticket `id`, once it has come; it is removed. Other tickets' answers stay
    /// where they are, so a feature that never asks for its answer blocks no other feature.
    fn take_answer(&self, id: PickId) -> Option<Picked>;
    /// Drops ticket `id`: its answer, if it has come, and the answer that has yet to come. An
    /// owner that closes with a pick outstanding calls it.
    fn forget(&self, id: PickId);
    /// Whether a picker is on screen or waiting.
    fn pick_pending(&self) -> bool;
    fn read(&self, path: &Path) -> Result<Vec<u8>, FileError>;
    /// Writes atomically where the platform allows (temp file and rename).
    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), FileError>;
    /// The folder the last answered pick of `purpose` chose.
    fn last_folder(&self, purpose: &str) -> Option<PathBuf>;
}

/// No pickers (every pick answers `Cancelled`); reads and writes go to the file system. The
/// default on native when the app installs no service; tests use it too.
#[derive(Default)]
pub struct NoPickFiles {
    next: Mutex<u64>,
    answered: Mutex<Vec<PickAnswer>>,
}

impl Files for NoPickFiles {
    fn pick(&self, request: PickRequest) -> PickId {
        let mut next = self.next.lock().unwrap_or_else(|e| e.into_inner());
        *next += 1;
        let id = PickId(*next);
        self.answered.lock().unwrap_or_else(|e| e.into_inner()).push(PickAnswer { id, request, picked: Picked::Cancelled });
        id
    }

    fn take_answer(&self, id: PickId) -> Option<Picked> {
        let mut answered = self.answered.lock().unwrap_or_else(|e| e.into_inner());
        let at = answered.iter().position(|a| a.id == id)?;
        Some(answered.remove(at).picked)
    }

    fn forget(&self, id: PickId) {
        self.answered.lock().unwrap_or_else(|e| e.into_inner()).retain(|a| a.id != id);
    }

    fn pick_pending(&self) -> bool {
        false
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn read(&self, path: &Path) -> Result<Vec<u8>, FileError> {
        std::fs::read(path).map_err(|e| with_path(e, path))
    }

    #[cfg(target_arch = "wasm32")]
    fn read(&self, _path: &Path) -> Result<Vec<u8>, FileError> {
        Err(FileError::Unsupported)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), FileError> {
        let tmp = path.with_extension(format!("{}.tmp", path.extension().and_then(|e| e.to_str()).unwrap_or("")));
        std::fs::write(&tmp, bytes).map_err(|e| with_path(e, path))?;
        std::fs::rename(&tmp, path).map_err(|e| with_path(e, path))
    }

    #[cfg(target_arch = "wasm32")]
    fn write(&self, _path: &Path, _bytes: &[u8]) -> Result<(), FileError> {
        Err(FileError::Unsupported)
    }

    fn last_folder(&self, _purpose: &str) -> Option<PathBuf> {
        None
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn with_path(e: std::io::Error, path: &Path) -> FileError {
    match FileError::from(e) {
        FileError::NotFound(_) => FileError::NotFound(path.to_owned()),
        FileError::PermissionDenied(_) => FileError::PermissionDenied(path.to_owned()),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // An answer nobody asks for does not hide another ticket's answer.
    #[test]
    fn an_orphaned_answer_blocks_no_other_pick() {
        let files = NoPickFiles::default();
        let orphan = files.pick(PickRequest::open("a"));
        let wanted = files.pick(PickRequest::open("b"));
        assert_eq!(files.take_answer(wanted), Some(Picked::Cancelled));
        assert_eq!(files.take_answer(wanted), None);
        files.forget(orphan);
        assert_eq!(files.take_answer(orphan), None);
    }

    #[test]
    fn picked_reads_paths_and_names() {
        let paths = Picked::Paths(vec!["/a/b.png".into(), "/c.png".into()]);
        assert_eq!(paths.first_path(), Some(PathBuf::from("/a/b.png")));
        assert_eq!(paths.paths().len(), 2);
        let contents = Picked::Contents(vec![("x.png".into(), vec![1])]);
        assert_eq!(contents.first_path(), Some(PathBuf::from("x.png")));
        assert_eq!(Picked::Cancelled.first_path(), None);
        assert!(Picked::Cancelled.paths().is_empty());
    }
}
