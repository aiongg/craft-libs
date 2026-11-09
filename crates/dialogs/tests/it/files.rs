//! The file list, the path field and the picker tickets they keep, against a file service that records its requests.

use craft_dialogs::ui::{FileList, FileListPicker, PathField};
use craft_host::{AppInfo, FileError, Files, Host, PickAnswer, PickId, PickKind, PickRequest, Picked};
use craft_ui::contract::Bind;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};
use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

/// Records every request; answers are put in by the test.
#[derive(Default)]
struct StubFiles {
    state: Mutex<StubState>,
}

#[derive(Default)]
struct StubState {
    requests: Vec<PickRequest>,
    answered: Vec<PickAnswer>,
}

impl StubFiles {
    /// Answers ticket `id`; a ticket this service never issued is answered as a request of another feature.
    fn answer(&self, id: PickId, picked: Picked) {
        let mut state = self.state.lock().expect("the stub is not poisoned");
        let request = usize::try_from(id.0 - 1).ok().and_then(|at| state.requests.get(at).cloned()).unwrap_or_else(|| PickRequest::open("other"));
        state.answered.push(PickAnswer { id, request, picked });
    }

    fn requests(&self) -> Vec<PickRequest> {
        self.state.lock().expect("the stub is not poisoned").requests.clone()
    }

    fn unclaimed(&self) -> usize {
        self.state.lock().expect("the stub is not poisoned").answered.len()
    }
}

impl Files for StubFiles {
    fn pick(&self, request: PickRequest) -> PickId {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.requests.push(request);
        PickId(state.requests.len() as u64)
    }

    fn take_answer(&self, id: PickId) -> Option<Picked> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let at = state.answered.iter().position(|a| a.id == id)?;
        Some(state.answered.remove(at).picked)
    }

    fn forget(&self, id: PickId) {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).answered.retain(|a| a.id != id);
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

fn host_with(files: &Arc<StubFiles>) -> Arc<Host> {
    Arc::new(Host::builder(AppInfo { id: "test", name: "Test", version: "1.0" }).files(files.clone()).build())
}

fn paths(names: &[&str]) -> Vec<PathBuf> {
    names.iter().map(|n| PathBuf::from(format!("/photos/{n}"))).collect()
}

fn names(list: &FileList) -> Vec<String> {
    list.files.iter().map(|p| p.file_name().expect("a file name").to_string_lossy().into_owned()).collect()
}

/// Browse queues an open-many picker and the answer's files are added on a later pass; Delete removes the selected
/// row; Alt+Down moves it.
#[trial]
fn browse_adds_and_keys_remove_and_move() {
    let files = Arc::new(StubFiles::default());
    let list = Rc::new(RefCell::new(FileList::default()));
    let ticket = Rc::new(Cell::new(None::<PickId>));
    let changed = Rc::new(Cell::new(false));
    let (held, issued, flagged) = (Rc::clone(&list), Rc::clone(&ticket), Rc::clone(&changed));
    let mut fixture = Fixture::new(move |ui| {
        let mut list = held.borrow_mut();
        let out = FileListPicker::new("Files to load", Bind::bound(&mut *list)).show(ui);
        if out.browsing.is_some() {
            issued.set(out.browsing);
        }
        flagged.set(out.changed);
    });
    fixture.host(host_with(&files));
    fixture.settle();
    let browse = fixture.by_label("Browse…").expect("the Browse button");
    fixture.click(&browse);
    fixture.settle();
    assert_eq!(ticket.get(), Some(PickId(1)));
    assert_eq!(files.requests(), vec![PickRequest::open_many("stack")]);
    assert_eq!(files.requests()[0].kind, PickKind::OpenMany);

    files.answer(PickId(1), Picked::Paths(paths(&["a.png", "b.png", "c.png"])));
    fixture.idle(2);
    assert_eq!(names(&list.borrow()), ["a.png", "b.png", "c.png"]);

    let first = fixture.by_label("a.png").expect("the first row");
    fixture.click(&first);
    fixture.settle();
    assert_eq!(list.borrow().selected, paths(&["a.png"]).into_iter().next());
    fixture.key(Key::ArrowDown, Modifiers::ALT);
    fixture.settle();
    assert_eq!(names(&list.borrow()), ["b.png", "a.png", "c.png"]);
    assert_eq!(list.borrow().selected, paths(&["a.png"]).into_iter().next(), "the selection follows the moved file");
    fixture.key(Key::Delete, Modifiers::NONE);
    fixture.settle();
    assert_eq!(names(&list.borrow()), ["b.png", "c.png"]);
    assert_eq!(list.borrow().selected, None);
}

/// An answer arriving while the list shows lands in the list once and is written to the binding once: the footer's
/// sizing pass takes and writes nothing.
#[trial]
fn browse_answer_is_taken_and_written_once() {
    let files = Arc::new(StubFiles::default());
    let list = Rc::new(RefCell::new(FileList::default()));
    let changes = Rc::new(Cell::new(0usize));
    let ticket = Rc::new(Cell::new(None::<PickId>));
    let (held, counted, issued) = (Rc::clone(&list), Rc::clone(&changes), Rc::clone(&ticket));
    let mut fixture = Fixture::new(move |ui| {
        let mut list = held.borrow_mut();
        let out = FileListPicker::new("Files to load", Bind::bound(&mut *list)).show(ui);
        if out.browsing.is_some() {
            issued.set(out.browsing);
        }
        if out.changed {
            counted.set(counted.get() + 1);
        }
    });
    fixture.host(host_with(&files));
    fixture.settle();
    let browse = fixture.by_label("Browse…").expect("the Browse button");
    fixture.click(&browse);
    fixture.settle();
    changes.set(0);

    files.answer(ticket.get().expect("Browse issued a ticket"), Picked::Paths(paths(&["a.png", "b.png"])));
    fixture.idle(3);
    assert_eq!(names(&list.borrow()), ["a.png", "b.png"]);
    assert_eq!(changes.get(), 1, "the answer is written to the binding once");
}

/// A path field over `value` with a file service that records its requests.
struct Field {
    fixture: Fixture,
    files: Arc<StubFiles>,
    value: Rc<RefCell<String>>,
    edits: Rc<Cell<usize>>,
}

impl Field {
    fn new(value: &str, request: PickRequest, must_exist: bool) -> Self {
        let files = Arc::new(StubFiles::default());
        let value = Rc::new(RefCell::new(value.to_owned()));
        let edits = Rc::new(Cell::new(0));
        let (held, counted) = (Rc::clone(&value), Rc::clone(&edits));
        let mut fixture = Fixture::new(move |ui| {
            let mut value = held.borrow_mut();
            let out = PathField::new("Save to", Bind::bound(&mut *value), request.clone()).must_exist(must_exist).show(ui);
            if out.edit.is_some() {
                counted.set(counted.get() + 1);
            }
        });
        fixture.host(host_with(&files));
        fixture.settle();
        Field { fixture, files, value, edits }
    }

    fn browse(&mut self) {
        let button = self.fixture.by_label("Choose File…").expect("the Browse button");
        self.fixture.click(&button);
        self.fixture.settle();
    }
}

/// Browse queues the field's request; the answer's path fills the field on a later pass and edits once. An unclaimed
/// answer of another ticket stays in the service and does not hold the field's own answer back; a cancelled pick changes
/// nothing.
#[trial]
fn browse_answer_fills_the_field_and_an_orphaned_answer_blocks_nothing() {
    let request = PickRequest::save("export").suggested_extension("png");
    let mut field = Field::new("/old.png", request.clone(), false);
    field.browse();
    assert_eq!(field.files.requests(), vec![request]);

    field.files.answer(PickId(99), Picked::Paths(vec![PathBuf::from("/theirs.png")]));
    field.fixture.idle(2);
    assert_eq!(*field.value.borrow(), "/old.png", "another ticket's answer is not ours");
    assert_eq!(field.files.unclaimed(), 1, "and stays in the service");

    field.files.answer(PickId(1), Picked::Paths(vec![PathBuf::from("/new.png"), PathBuf::from("/second.png")]));
    field.fixture.settle();
    assert_eq!(*field.value.borrow(), "/new.png", "the first path fills the field");
    assert_eq!(field.edits.get(), 1, "and edits once");

    field.browse();
    field.files.answer(PickId(2), Picked::Cancelled);
    field.fixture.settle();
    assert_eq!(*field.value.borrow(), "/new.png");
    assert_eq!(field.edits.get(), 1);
}
