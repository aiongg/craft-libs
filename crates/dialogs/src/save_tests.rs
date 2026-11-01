//! `craft_dialogs::save`, with a scripted `Files` and an inline `FileWatch`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use craft_host::{FileError, FileFilter, Files, PickId, PickRequest, Picked};
use craft_platform::watch::FileWatch;

use crate::save::{SaveDocument, SaveFlow, SaveStep, SaveTarget, is_mail_temp_copy, save_dialog_folder};

/// Records every request; the test answers tickets by hand.
#[derive(Default)]
struct ScriptedFiles {
    requests: Mutex<Vec<(PickId, PickRequest)>>,
    answers: Mutex<HashMap<PickId, Picked>>,
}

impl ScriptedFiles {
    fn answer(&self, id: PickId, picked: Picked) {
        self.answers.lock().unwrap().insert(id, picked);
    }

    fn last_ticket(&self) -> PickId {
        self.requests.lock().unwrap().last().unwrap().0
    }

    fn last_request(&self) -> PickRequest {
        self.requests.lock().unwrap().last().unwrap().1.clone()
    }
}

impl Files for ScriptedFiles {
    fn pick(&self, request: PickRequest) -> PickId {
        let mut requests = self.requests.lock().unwrap();
        let id = PickId(requests.len() as u64 + 1);
        requests.push((id, request));
        id
    }

    fn take_answer(&self, id: PickId) -> Option<Picked> {
        self.answers.lock().unwrap().remove(&id)
    }

    fn forget(&self, id: PickId) {
        self.answers.lock().unwrap().remove(&id);
    }

    fn pick_pending(&self) -> bool {
        false
    }

    fn read(&self, _path: &Path) -> Result<Vec<u8>, FileError> {
        Err(FileError::Unsupported)
    }

    fn write(&self, _path: &Path, _bytes: &[u8]) -> Result<(), FileError> {
        Err(FileError::Unsupported)
    }

    fn last_folder(&self, _purpose: &str) -> Option<PathBuf> {
        None
    }
}

fn document(path: Option<&Path>, name: &str) -> SaveDocument<u32> {
    SaveDocument {
        key: 7,
        name: name.into(),
        path: path.map(Path::to_path_buf),
        extension: "pdf".into(),
        filters: vec![FileFilter::new("PDF", &["pdf"])],
        purpose: "document".into(),
        saved: None,
    }
}

fn watch() -> FileWatch<u32> {
    FileWatch::new(craft_host::Wake::none())
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("craft-save-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

// Save of a document with a file writes in place with no picker; Save As and a new document open
// the picker in the document's folder with name and extension suggested, and the answer becomes
// Write with the picked path.
#[test]
fn save_writes_in_place_and_save_as_asks_where() {
    let dir = scratch("place");
    let file = dir.join("poster.pdf");
    std::fs::write(&file, b"one").unwrap();
    let (files, mut watch, mut flow) = (ScriptedFiles::default(), watch(), SaveFlow::new());
    watch.known_now(7, &file);

    let step = flow.start(&files, &watch, document(Some(&file), "poster.pdf"), SaveTarget::InPlace);
    assert_eq!(step, SaveStep::Write { key: 7, path: file.clone(), target: SaveTarget::InPlace });
    assert!(files.requests.lock().unwrap().is_empty());
    flow.written(&mut watch, 7, &file);
    assert!(!flow.is_busy());

    let step = flow.start(&files, &watch, document(Some(&file), "poster.pdf"), SaveTarget::As);
    assert_eq!(step, SaveStep::Picking);
    let request = files.last_request();
    assert_eq!(request.directory.as_deref(), Some(dir.as_path()));
    assert_eq!(request.file_name.as_deref(), Some("poster.pdf"));
    assert_eq!(request.suggested_extension.as_deref(), Some("pdf"));
    assert_eq!(flow.take_pick(&files, &watch), None);
    let other = dir.join("copy.pdf");
    files.answer(files.last_ticket(), Picked::Paths(vec![other.clone()]));
    assert_eq!(flow.take_pick(&files, &watch), Some(SaveStep::Write { key: 7, path: other, target: SaveTarget::As }));

    flow.failed(&7);
    let step = flow.start(&files, &watch, document(None, "Untitled-2"), SaveTarget::InPlace);
    assert_eq!(step, SaveStep::Picking);
    assert_eq!(files.last_request().file_name.as_deref(), Some("Untitled-2.pdf"));
    assert_eq!(files.last_request().directory, None);
    files.answer(files.last_ticket(), Picked::Cancelled);
    assert_eq!(flow.take_pick(&files, &watch), Some(SaveStep::Cancelled { key: 7 }));
    assert!(!flow.is_busy());
    std::fs::remove_dir_all(dir).ok();
}

// After another writer changed the document's own file, Save asks; Replace writes, Save As opens
// the picker, Cancel cancels; a file the app wrote itself (written) never asks.
#[test]
fn a_file_changed_on_disk_is_asked_about_once() {
    let dir = scratch("changed");
    let file = dir.join("a.pdf");
    std::fs::write(&file, b"one").unwrap();
    let (files, mut watch, mut flow) = (ScriptedFiles::default(), watch(), SaveFlow::new());
    watch.known_now(7, &file);
    std::fs::write(&file, b"another app wrote more").unwrap();
    let ask = SaveStep::AskChangedOnDisk { key: 7, path: file.clone() };

    assert_eq!(flow.start(&files, &watch, document(Some(&file), "a.pdf"), SaveTarget::InPlace), ask);
    assert!(flow.is_busy());
    assert_eq!(flow.replace(), SaveStep::Write { key: 7, path: file.clone(), target: SaveTarget::InPlace });
    flow.written(&mut watch, 7, &file);
    assert_eq!(
        flow.start(&files, &watch, document(Some(&file), "a.pdf"), SaveTarget::InPlace),
        SaveStep::Write { key: 7, path: file.clone(), target: SaveTarget::InPlace },
        "the app's own write is no change on disk"
    );
    flow.failed(&7);

    std::fs::write(&file, b"changed again, longer").unwrap();
    assert_eq!(flow.start(&files, &watch, document(Some(&file), "a.pdf"), SaveTarget::InPlace), ask);
    assert_eq!(flow.save_elsewhere(&files), SaveStep::Picking);
    let request = files.last_request();
    assert_eq!(request.directory.as_deref(), Some(dir.as_path()));
    let elsewhere = dir.join("b.pdf");
    files.answer(files.last_ticket(), Picked::Paths(vec![elsewhere.clone()]));
    assert_eq!(flow.take_pick(&files, &watch), Some(SaveStep::Write { key: 7, path: elsewhere, target: SaveTarget::As }));
    flow.failed(&7);

    assert_eq!(flow.start(&files, &watch, document(Some(&file), "a.pdf"), SaveTarget::InPlace), ask);
    assert_eq!(flow.cancel(&files), Some(SaveStep::Cancelled { key: 7 }));
    assert!(!flow.is_busy());
    std::fs::remove_dir_all(dir).ok();
}

// Saving a mail attachment's temporary copy asks where to keep it; the Outlook and Apple Mail
// paths match on either separator and in any case, an ordinary folder named Content.Outlook
// doesn't.
#[test]
fn mail_temporary_copies_are_saved_elsewhere() {
    for path in [
        r"C:\Users\me\AppData\Local\Microsoft\Windows\INetCache\Content.Outlook\AB12CD\invoice.pdf",
        "c:/users/me/appdata/local/microsoft/windows/temporary internet files/content.outlook/x9/invoice.pdf",
        r"C:\Users\me\AppData\Local\Packages\Microsoft.OutlookForWindows_8wekyb3d8bbwe\LocalCache\f.pdf",
        "/Users/me/Library/Containers/com.apple.mail/Data/Library/Mail Downloads/UUID/invoice.pdf",
    ] {
        assert!(is_mail_temp_copy(Path::new(path)), "{path}");
        assert_eq!(save_dialog_folder(Some(Path::new(path))), None, "{path}");
    }
    for path in ["/home/me/Content.Outlook/x/f.pdf", "/home/me/INetCache/Content.Outlook/f.pdf", "/home/me/INetCache/Content.Outlook/a/b/f.pdf"] {
        assert!(!is_mail_temp_copy(Path::new(path)), "{path}");
    }
    assert_eq!(save_dialog_folder(Some(Path::new("/home/me/doc.pdf"))), Some(PathBuf::from("/home/me")));
    assert_eq!(save_dialog_folder(Some(Path::new("doc.pdf"))), None);
    assert_eq!(save_dialog_folder(None), None);

    let temp = Path::new("/Users/me/Library/Containers/com.apple.mail/Data/Library/Mail Downloads/U/invoice.pdf");
    let (files, watch, mut flow) = (ScriptedFiles::default(), watch(), SaveFlow::new());
    assert_eq!(flow.start(&files, &watch, document(Some(temp), "invoice.pdf"), SaveTarget::InPlace), SaveStep::Picking);
    assert_eq!(files.last_request().directory, None);
}

// A second start while a picker, question or write is pending answers Busy; another feature's
// answer stays with the file service; cancel forgets the flow's ticket, so its late answer is
// never taken.
#[test]
fn one_save_at_a_time_and_cancel_forgets_the_ticket() {
    let (files, watch, mut flow) = (ScriptedFiles::default(), watch(), SaveFlow::new());
    let other = files.pick(PickRequest::open("document"));
    files.answer(other, Picked::Cancelled);

    assert_eq!(flow.start(&files, &watch, document(None, "A"), SaveTarget::InPlace), SaveStep::Picking);
    assert_eq!(flow.start(&files, &watch, document(None, "B"), SaveTarget::As), SaveStep::Busy);
    assert_eq!(flow.take_pick(&files, &watch), None);
    assert_eq!(files.take_answer(other), Some(Picked::Cancelled), "the flow left another feature's answer alone");

    let ticket = files.last_ticket();
    assert_eq!(flow.cancel(&files), Some(SaveStep::Cancelled { key: 7 }));
    files.answer(ticket, Picked::Paths(vec![PathBuf::from("/x/late.pdf")]));
    assert_eq!(files.take_answer(ticket), Some(Picked::Paths(vec![PathBuf::from("/x/late.pdf")])), "answered after cancel");
    assert_eq!(flow.take_pick(&files, &watch), None);
    assert_eq!(flow.cancel(&files), None);

    assert_eq!(flow.start(&files, &watch, document(None, "A"), SaveTarget::Copy), SaveStep::Picking);
    files.answer(files.last_ticket(), Picked::Paths(vec![PathBuf::from("/x/c.pdf")]));
    assert!(matches!(flow.take_pick(&files, &watch), Some(SaveStep::Write { target: SaveTarget::Copy, .. })));
    assert_eq!(flow.start(&files, &watch, document(None, "B"), SaveTarget::As), SaveStep::Busy);
}

// A write that finishes for another document (its save was cancelled) leaves a newer save alone.
#[test]
fn a_late_write_does_not_free_another_save() {
    let (files, mut watch, mut flow) = (ScriptedFiles::default(), watch(), SaveFlow::new());
    assert_eq!(
        flow.start(&files, &watch, document(Some(Path::new("/x/a.pdf")), "a.pdf"), SaveTarget::InPlace),
        SaveStep::Write { key: 7, path: PathBuf::from("/x/a.pdf"), target: SaveTarget::InPlace }
    );
    assert_eq!(flow.cancel(&files), Some(SaveStep::Cancelled { key: 7 }));
    let mut other = document(None, "B");
    other.key = 8;
    assert_eq!(flow.start(&files, &watch, other, SaveTarget::As), SaveStep::Picking);
    let ticket = files.last_ticket();

    flow.written(&mut watch, 7, Path::new("/x/a.pdf"));
    assert!(flow.is_busy(), "the pick of the other document is still open");
    files.answer(ticket, Picked::Paths(vec![PathBuf::from("/x/b.pdf")]));
    assert_eq!(flow.take_pick(&files, &watch), Some(SaveStep::Write { key: 8, path: PathBuf::from("/x/b.pdf"), target: SaveTarget::As }));
    flow.written(&mut watch, 8, Path::new("/x/b.pdf"));
    assert!(!flow.is_busy());
}

// Only the change time differs (a permission change stands for Finder tags): with the bytes the
// app saved Save writes without asking; without them, or with other bytes, it asks.
#[cfg(unix)]
#[test]
fn a_change_time_only_difference_with_equal_bytes_does_not_ask() {
    use std::os::unix::fs::PermissionsExt as _;
    let dir = scratch("ctime");
    let file = dir.join("a.pdf");
    std::fs::write(&file, b"content").unwrap();
    let (files, mut watch, mut flow) = (ScriptedFiles::default(), watch(), SaveFlow::new());
    watch.known_now(7, &file);
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o640)).unwrap();

    let ask = SaveStep::AskChangedOnDisk { key: 7, path: file.clone() };
    let write = SaveStep::Write { key: 7, path: file.clone(), target: SaveTarget::InPlace };
    let mut doc = document(Some(&file), "a.pdf");
    assert_eq!(flow.start(&files, &watch, doc.clone(), SaveTarget::InPlace), ask, "no saved bytes");
    flow.cancel(&files);
    doc.saved = Some(Arc::new(b"other".to_vec()));
    assert_eq!(flow.start(&files, &watch, doc.clone(), SaveTarget::InPlace), ask, "other bytes");
    flow.cancel(&files);
    doc.saved = Some(Arc::new(b"content".to_vec()));
    assert_eq!(flow.start(&files, &watch, doc, SaveTarget::InPlace), write);
    std::fs::remove_dir_all(dir).ok();
}
