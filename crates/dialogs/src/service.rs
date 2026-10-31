//! [`FileService`]: craft-host's [`Files`] over the native panels of [`Dialogs`](crate::Dialogs).
//!
//! The app builds the pair at startup, installs the service in the Host
//! (`Host::builder(..).files(Arc::new(service))`) and keeps the [`FilePanels`] on the main thread,
//! calling [`FilePanels::pump`] once per frame after its UI (the panel shows at the end of the
//! frame). The service is `Send + Sync`; the panels are not, because the native dialogs are not. Features never see the panels: they ask `host.files().pick(..)` and
//! read the answer from `host.files().take_pick()` on a later pass.
//!
//! What the user sees: Open and Folder as free-standing panels on macOS, Save as a sheet; the panel
//! starts in the last folder chosen for the same purpose; a Save that names an existing file the
//! OS panel did not ask about gets the Replace question (PhotoCraft `file_dialog.rs:193
//! replaces_unasked`); a typed name without an extension gets the suggested one and an uppercase
//! extension is lowercased (`file_dialog.rs:210 lowercased_extension`, `:365 saved_path`).

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};

use craft_host::{FileError, Files, PickAnswer, PickId, PickRequest};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use craft_host::{PickKind, Picked};

use crate::{Answer, Dialogs, FileDialog, Filter, Kind, Message, MessageButtons, MessageLevel, MessageResult, Wake};

const REPLACE: &str = "Replace";
const CANCEL: &str = "Cancel";

fn lock(inner: &Mutex<Inner>) -> std::sync::MutexGuard<'_, Inner> {
    inner.lock().unwrap_or_else(|e| e.into_inner())
}

fn file_error(e: std::io::Error, path: &Path) -> FileError {
    match e.kind() {
        std::io::ErrorKind::NotFound => FileError::NotFound(path.to_owned()),
        std::io::ErrorKind::PermissionDenied => FileError::PermissionDenied(path.to_owned()),
        _ => FileError::Other(e.to_string()),
    }
}

/// The name a Save panel starts with. Where the panel adds the filter's extension itself
/// (`adds_extension`, Windows), a lowercase extension is left off so the name does not end up
/// doubled; an uppercase extension is the user's spelling and stays.
// pc: apps/photocraft/src/services.rs:192 dialog_file_name
fn dialog_file_name(suggested: &str, adds_extension: bool) -> Option<String> {
    let path = Path::new(suggested);
    let name = path.file_name()?.to_string_lossy().into_owned();
    let (Some(stem), Some(ext)) = (path.file_stem(), path.extension().and_then(|e| e.to_str())) else { return Some(name) };
    let bare = adds_extension && Path::new(stem).extension().is_none() && !ext.is_empty() && ext == ext.to_ascii_lowercase();
    Some(if bare { stem.to_string_lossy().into_owned() } else { name })
}

/// Whether `a` and `b` reach the same file (device and inode on Unix, final path elsewhere).
// pc: apps/photocraft/src/services.rs:370 same_file
fn same_file(a: &Path, b: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        match (std::fs::metadata(a), std::fs::metadata(b)) {
            (Ok(a), Ok(b)) => a.dev() == b.dev() && a.ino() == b.ino(),
            _ => false,
        }
    }
    #[cfg(not(unix))]
    {
        match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
            (Ok(a), Ok(b)) => a == b,
            _ => false,
        }
    }
}

/// Whether `a` and `b` name one entry in one folder: another spelling of a name on a file system
/// that ignores letter case is the same entry; a hard link is another entry.
// pc: apps/photocraft/src/services.rs:392 same_entry
fn same_entry(a: &Path, b: &Path) -> bool {
    let folder = |p: &Path| p.parent().filter(|d| !d.as_os_str().is_empty()).unwrap_or(Path::new(".")).to_path_buf();
    if !same_file(a, b) || !same_file(&folder(a), &folder(b)) {
        return false;
    }
    let (Some(x), Some(y)) = (a.file_name(), b.file_name()) else { return false };
    if x == y {
        return true;
    }
    let Ok(entries) = std::fs::read_dir(folder(a)) else { return false };
    let listed: Vec<_> = entries.filter_map(|e| e.ok().map(|e| e.file_name())).collect();
    !(listed.iter().any(|n| n == x) && listed.iter().any(|n| n == y))
}

/// How a Save answer's extension is settled before the app sees it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ExtensionPolicy {
    /// Lowercase the extension; add the request's suggested one when the name has none.
    #[default]
    Suggest,
    /// Keep the typed name as the panel answered it.
    AsTyped,
}

/// Where a Save answer that would overwrite goes next: the panel already asked (macOS, Windows,
/// GTK and the portal all do), so only answers that still name an existing file on a platform
/// that did not ask get the question.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ReplaceGuard {
    /// Ask with a native message box when the panel did not.
    #[default]
    AskWhenUnasked,
    /// Never ask (the app asks with `StandardPrompt::replace_file` itself).
    Never,
}

/// One pending request: the ticket, what was asked, and whether the service is still settling the
/// answer (the replace question).
struct Pending {
    request: PickRequest,
    /// The Save path waiting on the replace question's answer.
    awaiting_replace: Option<PathBuf>,
}

/// The native file service: the Host's side. Requests go to the [`FilePanels`] over a channel;
/// answers come back through the shared state.
pub struct FileService {
    inner: Arc<Mutex<Inner>>,
    wake: Wake,
    to_panels: Sender<(PickId, PickRequest)>,
}

/// The main-thread side: owns the native panels and settles answers. Built with the service by
/// [`FileService::new`].
pub struct FilePanels {
    inner: Arc<Mutex<Inner>>,
    requests: Receiver<(PickId, PickRequest)>,
    dialogs: Dialogs<PickId>,
    extension_policy: ExtensionPolicy,
    replace_guard: ReplaceGuard,
}

struct Inner {
    next: u64,
    pending: HashMap<PickId, Pending>,
    answered: VecDeque<PickAnswer>,
    last_folder: HashMap<String, PathBuf>,
    /// A panel is on screen or queued (mirrored from the panels each pump).
    panel_busy: bool,
}

impl FileService {
    /// `wake` requests a repaint from any thread (`ctx.request_repaint()`).
    // pc: apps/photocraft/src/services.rs:203 show_file_dialog; crates/ui-egui/src/file_dialog.rs:310 poll_file_dialog
    pub fn new(wake: Wake) -> (Self, FilePanels) {
        let inner = Arc::new(Mutex::new(Inner {
            next: 0,
            pending: HashMap::new(),
            answered: VecDeque::new(),
            last_folder: HashMap::new(),
            panel_busy: false,
        }));
        let (to_panels, requests) = std::sync::mpsc::channel();
        let panels = FilePanels {
            inner: inner.clone(),
            requests,
            dialogs: Dialogs::new(wake.clone()),
            extension_policy: ExtensionPolicy::default(),
            replace_guard: ReplaceGuard::default(),
        };
        (Self { inner, wake, to_panels }, panels)
    }

    /// Seeds the remembered folders (from the app's settings) before the first pick; the app reads
    /// them back with [`last_folders`](FileService::last_folders) to store them.
    pub fn with_last_folders(self, folders: HashMap<String, PathBuf>) -> Self {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).last_folder = folders;
        self
    }

    /// Every remembered folder by purpose, for the app's settings.
    pub fn last_folders(&self) -> HashMap<String, PathBuf> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).last_folder.clone()
    }
}

impl FilePanels {
    pub fn extension_policy(mut self, policy: ExtensionPolicy) -> Self {
        self.extension_policy = policy;
        self
    }

    pub fn replace_guard(mut self, guard: ReplaceGuard) -> Self {
        self.replace_guard = guard;
        self
    }

    /// Shows the next queued panel and collects answers. Called once per frame on the main thread
    /// with the app window (`Some(frame)` in eframe) as the parent; `None` shows free-standing
    /// panels. Save answers go through the extension policy, then the replace guard (which may
    /// queue a message box and finish on a later pump), then the last folder is remembered and
    /// the answer becomes a [`PickAnswer`].
    // pc: crates/ui-egui/src/file_dialog.rs:310 poll_file_dialog, :349 finish_file_dialog, :365 saved_path
    pub fn pump<W: HasWindowHandle + HasDisplayHandle>(&mut self, parent: Option<&W>) {
        while let Ok((id, request)) = self.requests.try_recv() {
            let folder = lock(&self.inner).last_folder.get(&request.purpose).cloned();
            let dialog = self.file_dialog(&request, folder.as_deref());
            self.dialogs.ask(dialog, id);
        }
        // A replace question queued by `finish` shows in the same pass, so the loop runs until no
        // answer is left to settle.
        loop {
            self.dialogs.show_pending(parent);
            let answers = self.dialogs.take_answers();
            if answers.is_empty() {
                break;
            }
            for (id, answer) in answers {
                self.finish(id, answer);
            }
        }
        lock(&self.inner).panel_busy = self.dialogs.is_open() || self.dialogs.queued() > 0;
    }

    /// Translates a request into the panel builder: kind, title, filters, the remembered folder
    /// (else `request.directory`) and the suggested name with its extension.
    // pc: apps/photocraft/src/services.rs:192 dialog_file_name, :203 show_file_dialog
    fn file_dialog(&self, request: &PickRequest, last_folder: Option<&Path>) -> FileDialog {
        let mut dialog = FileDialog::new(match request.kind {
            PickKind::Open => Kind::Open { multiple: false },
            PickKind::OpenMany => Kind::Open { multiple: true },
            PickKind::Folder => Kind::Folder,
            PickKind::Save => Kind::Save,
        });
        dialog.title = request.title.clone();
        dialog.filters = request.filters.iter().map(|f| Filter { name: f.name.clone(), extensions: f.extensions.clone() }).collect();
        dialog.directory = last_folder.map(Path::to_path_buf).or_else(|| request.directory.clone());
        if request.kind == PickKind::Save {
            let adds_extension = cfg!(windows) && !request.filters.is_empty();
            dialog.file_name = request.file_name.as_deref().and_then(|name| dialog_file_name(name, adds_extension));
        }
        dialog
    }

    /// The saved path after the extension policy: `"Photo"` becomes `"Photo.png"` with a
    /// suggested `png`; `"Photo.PNG"` becomes `"Photo.png"`; a path with another known extension
    /// is kept.
    // pc: crates/ui-egui/src/file_dialog.rs:210 lowercased_extension, :365 saved_path
    pub fn settled_save_path(policy: ExtensionPolicy, answered: &Path, suggested: Option<&str>) -> PathBuf {
        if policy == ExtensionPolicy::AsTyped {
            return answered.to_path_buf();
        }
        match answered.extension().and_then(|e| e.to_str()) {
            Some(ext) if ext.bytes().any(|b| b.is_ascii_uppercase()) => answered.with_extension(ext.to_ascii_lowercase()),
            Some(_) => answered.to_path_buf(),
            None => match suggested {
                Some(ext) => answered.with_extension(ext),
                None => answered.to_path_buf(),
            },
        }
    }

    /// Whether the settled path names an existing file the panel did not ask about: the panel
    /// asked when the settled path equals the answered path, so only an extension change can
    /// make a new collision.
    // pc: crates/ui-egui/src/file_dialog.rs:193 replaces_unasked
    pub fn replaces_unasked(answered: &Path, settled: &Path) -> bool {
        answered != settled && settled.exists() && !same_entry(answered, settled)
    }

    /// The replace question as a native message box: "“{name}” already exists. Replace it?" with
    /// Replace (default) and Cancel.
    // pc: crates/ui-egui/src/dialogs/replace_prompt.rs (wording); shown natively here so it sits above the sheet
    fn replace_message(settled: &Path) -> Message {
        let name = settled.file_name().map_or_else(|| settled.to_string_lossy(), |n| n.to_string_lossy());
        Message {
            level: MessageLevel::Warning,
            title: REPLACE.into(),
            description: format!("“{name}” already exists. Replace it?"),
            buttons: MessageButtons::OkCancelCustom(REPLACE.into(), CANCEL.into()),
        }
    }

    /// Turns a panel answer into the pick's answer: `Cancelled` stays, `Paths` for Save go through
    /// the policy and guard, `Unavailable` becomes `Cancelled` plus a log line (the service has no
    /// way to show an error; the feature sees nothing chosen).
    fn finish(&mut self, id: PickId, answer: Answer) {
        let Some(pending) = lock(&self.inner).pending.remove(&id) else { return };
        let picked = match (answer, pending.awaiting_replace) {
            (Answer::Message(result), Some(settled)) => {
                let replace = matches!(&result, MessageResult::Ok) || matches!(&result, MessageResult::Custom(label) if label == REPLACE);
                if replace { Picked::Paths(vec![settled]) } else { Picked::Cancelled }
            }
            (Answer::Paths(paths), _) if paths.is_empty() => Picked::Cancelled,
            (Answer::Paths(mut paths), _) => {
                if pending.request.kind == PickKind::Save {
                    let answered = paths.swap_remove(0);
                    let settled = Self::settled_save_path(self.extension_policy, &answered, pending.request.suggested_extension.as_deref());
                    if self.replace_guard == ReplaceGuard::AskWhenUnasked && Self::replaces_unasked(&answered, &settled) {
                        self.dialogs.ask_message(Self::replace_message(&settled), id);
                        lock(&self.inner).pending.insert(id, Pending { request: pending.request, awaiting_replace: Some(settled) });
                        return;
                    }
                    paths = vec![settled];
                }
                Picked::Paths(paths)
            }
            (Answer::Unavailable(why), _) => {
                eprintln!("craft-dialogs: no file dialog could be shown: {why}");
                Picked::Cancelled
            }
            (Answer::Cancelled | Answer::Message(_), _) => Picked::Cancelled,
        };
        let mut inner = lock(&self.inner);
        if let Picked::Paths(paths) = &picked
            && let Some(first) = paths.first()
        {
            let folder = if pending.request.kind == PickKind::Folder { Some(first.as_path()) } else { first.parent() };
            if let Some(folder) = folder.filter(|f| !f.as_os_str().is_empty()) {
                inner.last_folder.insert(pending.request.purpose.clone(), folder.to_path_buf());
            }
        }
        inner.answered.push_back(PickAnswer { id, request: pending.request, picked });
    }
}

impl Files for FileService {
    // new: ticket allocation and queueing on `Dialogs::ask`
    fn pick(&self, request: PickRequest) -> PickId {
        let id = {
            let mut inner = lock(&self.inner);
            inner.next += 1;
            let id = PickId(inner.next);
            inner.pending.insert(id, Pending { request: request.clone(), awaiting_replace: None });
            id
        };
        // The panels gone (the app is shutting down) leaves the ticket unanswered.
        let _ = self.to_panels.send((id, request));
        (self.wake)();
        id
    }

    fn take_pick(&self) -> Option<PickAnswer> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).answered.pop_front()
    }

    fn return_pick(&self, answer: PickAnswer) {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).answered.push_front(answer);
    }

    fn pick_pending(&self) -> bool {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.panel_busy || !inner.pending.is_empty()
    }

    // pc: apps/photocraft/src/services.rs:359 write_atomic (read is plain `std::fs::read`)
    fn read(&self, path: &Path) -> Result<Vec<u8>, FileError> {
        std::fs::read(path).map_err(|e| file_error(e, path))
    }

    // pc: apps/photocraft/src/services.rs:359 write_atomic
    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), FileError> {
        craft_atomic::atomic_write(path, bytes).map_err(|e| file_error(e, path))
    }

    fn last_folder(&self, purpose: &str) -> Option<PathBuf> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).last_folder.get(purpose).cloned()
    }
}

#[cfg(test)]
mod service_tests {
    //! Behaviour tests with `Dialogs::set_override` standing in for the panels.

    use std::cell::RefCell;
    use std::rc::Rc;

    use craft_host::Picked;
    use raw_window_handle::{DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, WindowHandle};

    use super::*;
    use crate::{MessageResult, Request};

    struct NoWindow;

    impl HasWindowHandle for NoWindow {
        fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
            Err(HandleError::Unavailable)
        }
    }

    impl HasDisplayHandle for NoWindow {
        fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
            Err(HandleError::Unavailable)
        }
    }

    const NONE: Option<&NoWindow> = None;

    type Log = Rc<RefCell<Vec<Request>>>;

    /// A service whose panels answer each request with the next of `answers`, recording the requests.
    fn service_with(answers: Vec<Answer>) -> (FileService, FilePanels, Log) {
        let (service, mut panels) = FileService::new(Arc::new(|| {}));
        let log: Log = Rc::default();
        let seen = log.clone();
        let mut answers = answers.into_iter();
        panels.dialogs.set_override(Some(Box::new(move |request| {
            seen.borrow_mut().push(request.clone());
            answers.next().unwrap_or(Answer::Cancelled)
        })));
        (service, panels, log)
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("craft-dialogs-service-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch folder");
        dir
    }

    fn save_with(answer: &str, suggested: &str) -> Picked {
        let (service, mut panels, _) = service_with(vec![Answer::Paths(vec![PathBuf::from(answer)])]);
        service.pick(PickRequest::save("export").suggested_extension(suggested));
        panels.pump(NONE);
        service.take_pick().expect("answered").picked
    }

    // A Save answer "Photo" with suggested "png" reaches the app as "Photo.png".
    #[test]
    fn save_without_extension_gets_the_suggested_one() {
        let dir = scratch("suggest");
        let picked = save_with(dir.join("Photo").to_str().expect("utf-8"), "png");
        assert_eq!(picked, Picked::Paths(vec![dir.join("Photo.png")]));
    }

    // "Photo.PNG" is lowercased to "Photo.png"; "photo.jpeg" is kept.
    #[test]
    fn save_extension_is_lowercased_and_known_ones_are_kept() {
        let dir = scratch("lowercase");
        assert_eq!(save_with(dir.join("Photo.PNG").to_str().expect("utf-8"), "png"), Picked::Paths(vec![dir.join("Photo.png")]));
        assert_eq!(save_with(dir.join("photo.jpeg").to_str().expect("utf-8"), "png"), Picked::Paths(vec![dir.join("photo.jpeg")]));
    }

    // A settled path that newly collides with an existing file asks Replace; Cancel answers
    // Cancelled, Replace hands the path on.
    #[test]
    fn replace_is_asked_only_when_the_panel_did_not() {
        let dir = scratch("replace");
        std::fs::write(dir.join("Photo.png"), b"old").expect("fixture");
        let typed = Answer::Paths(vec![dir.join("Photo")]);
        let ask = |reply: MessageResult| {
            let (service, mut panels, log) = service_with(vec![typed.clone(), Answer::Message(reply)]);
            service.pick(PickRequest::save("export").suggested_extension("png"));
            panels.pump(NONE);
            let questions = log.borrow().iter().filter(|r| matches!(r, Request::Message(_))).count();
            (service.take_pick().expect("answered").picked, questions)
        };
        assert_eq!(ask(MessageResult::Cancel), (Picked::Cancelled, 1));
        assert_eq!(ask(MessageResult::Custom("Replace".into())), (Picked::Paths(vec![dir.join("Photo.png")]), 1));

        // The panel asked about "Photo.png" itself: no second question.
        let (service, mut panels, log) = service_with(vec![Answer::Paths(vec![dir.join("Photo.png")])]);
        service.pick(PickRequest::save("export").suggested_extension("png"));
        panels.pump(NONE);
        assert_eq!(service.take_pick().expect("answered").picked, Picked::Paths(vec![dir.join("Photo.png")]));
        assert_eq!(log.borrow().len(), 1);

        // The guard off: the app asks itself.
        let (service, panels, log) = service_with(vec![typed.clone()]);
        let mut panels = panels.replace_guard(ReplaceGuard::Never);
        service.pick(PickRequest::save("export").suggested_extension("png"));
        panels.pump(NONE);
        assert_eq!(service.take_pick().expect("answered").picked, Picked::Paths(vec![dir.join("Photo.png")]));
        assert_eq!(log.borrow().len(), 1);
    }

    // The second Open of purpose "document" starts in the folder the first chose; another
    // purpose does not.
    #[test]
    fn last_folder_is_remembered_per_purpose() {
        let dir = scratch("folders");
        let (service, mut panels, log) = service_with(vec![Answer::Paths(vec![dir.join("a.png")])]);
        service.pick(PickRequest::open("document"));
        panels.pump(NONE);
        assert_eq!(service.last_folder("document"), Some(dir.clone()));
        service.pick(PickRequest::open("document"));
        service.pick(PickRequest::open("swatches").directory("/elsewhere"));
        panels.pump(NONE);
        let directories: Vec<_> = log
            .borrow()
            .iter()
            .map(|r| match r {
                Request::File(d) => d.directory.clone(),
                Request::Message(_) => None,
            })
            .collect();
        assert_eq!(directories, vec![None, Some(dir), Some(PathBuf::from("/elsewhere"))]);
        assert_eq!(service.last_folder("swatches"), None);
    }

    // Two picks queue in order; `take_pick` returns them oldest first with their tickets.
    #[test]
    fn picks_answer_in_order_with_their_tickets() {
        let (service, mut panels, _) = service_with(vec![Answer::Paths(vec![PathBuf::from("/x/first.png")]), Answer::Cancelled]);
        let first = service.pick(PickRequest::open("document"));
        let second = service.pick(PickRequest::open_many("document"));
        assert!(service.pick_pending());
        panels.pump(NONE);
        assert!(!service.pick_pending());
        let a = service.take_pick().expect("first");
        let b = service.take_pick().expect("second");
        assert_eq!((a.id, a.picked), (first, Picked::Paths(vec![PathBuf::from("/x/first.png")])));
        assert_eq!((b.id, b.picked), (second, Picked::Cancelled));
        assert!(service.take_pick().is_none());
    }
}
