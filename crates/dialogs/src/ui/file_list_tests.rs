//! `FileList` and `FileListPicker` in a harness with a `Files` stub installed in the Host.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use craft_host::{FileError, Files, Host, PickAnswer, PickId, PickKind, PickRequest, Picked};
use craft_ui::localized::Labels;
use egui::{Context, Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, vec2};

use super::{FileList, FileListLabels, FileListMode, FileListPicker, FileListPickerResponse};

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
    fn answer(&self, id: PickId, picked: Picked) {
        let mut state = self.state.lock().unwrap();
        let request = state.requests[(id.0 - 1) as usize].clone();
        state.answered.push(PickAnswer { id, request, picked });
    }

    fn requests(&self) -> Vec<PickRequest> {
        self.state.lock().unwrap().requests.clone()
    }
}

impl Files for StubFiles {
    fn pick(&self, request: PickRequest) -> PickId {
        let mut state = self.state.lock().unwrap();
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

struct Rig {
    ctx: Context,
    files: Arc<StubFiles>,
    list: FileList,
    /// The words drawn in the last frame, with where.
    texts: Vec<(String, Pos2)>,
    response: FileListPickerResponse,
    time: f64,
}

impl Rig {
    fn new(list: FileList) -> Self {
        let ctx = Context::default();
        let files = Arc::new(StubFiles::default());
        Host::builder(craft_host::AppInfo { id: "test", name: "Test", version: "1.0" }).files(files.clone()).install(&ctx);
        let mut rig = Self { ctx, files, list, texts: Vec::new(), response: FileListPickerResponse::default(), time: 0.0 };
        rig.frame(vec![]);
        rig
    }

    fn frame(&mut self, mut events: Vec<Event>) {
        let releases: Vec<Event> = events
            .iter()
            .filter_map(|e| match e {
                Event::Key { key, physical_key, pressed: true, modifiers, .. } => {
                    Some(Event::Key { key: *key, physical_key: *physical_key, pressed: false, repeat: false, modifiers: *modifiers })
                }
                _ => None,
            })
            .collect();
        events.extend(releases);
        let input =
            RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))), time: Some(self.time), events, ..Default::default() };
        self.time += 1.0 / 60.0;
        let ctx = self.ctx.clone();
        let output = ctx.run_ui(input, |ui| {
            self.response = FileListPicker::new("stack", &mut self.list).show(ui);
        });
        self.texts = output.shapes.iter().flat_map(|clipped| painted_text(&clipped.shape)).collect();
        output.drop_without_applying_deltas();
    }

    /// Where the word `text` was drawn in the last frame.
    fn at(&self, text: &str) -> Pos2 {
        let (_, pos) = self.texts.iter().find(|(t, _)| t == text).unwrap_or_else(|| panic!("{text:?} is not drawn: {:?}", self.texts));
        *pos + vec2(4.0, 4.0)
    }

    fn click(&mut self, at: Pos2) {
        let button = |pressed| Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
        self.frame(vec![Event::PointerMoved(at)]);
        self.frame(vec![button(true)]);
        self.frame(vec![button(false)]);
    }

    fn key(&mut self, key: Key, modifiers: Modifiers) {
        self.frame(vec![Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }]);
    }

    fn names(&self) -> Vec<String> {
        self.list.files.iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect()
    }
}

fn painted_text(shape: &egui::Shape) -> Vec<(String, Pos2)> {
    match shape {
        egui::Shape::Text(text) => vec![(text.galley.text().to_owned(), text.pos)],
        egui::Shape::Vec(shapes) => shapes.iter().flat_map(painted_text).collect(),
        _ => Vec::new(),
    }
}

fn paths(names: &[&str]) -> Vec<PathBuf> {
    names.iter().map(|n| PathBuf::from(format!("/photos/{n}"))).collect()
}

// Adding keeps order and skips paths already listed.
#[test]
fn add_keeps_order_and_skips_duplicates() {
    let mut list = FileList::default();
    list.add(paths(&["b.png", "a.png", "b.png"]));
    list.add(paths(&["c.png", "a.png"]));
    assert_eq!(list.files, paths(&["b.png", "a.png", "c.png"]));
}

// OK needs a file in Files mode and a folder in Folder mode, with the matching message.
#[test]
fn validity_follows_the_mode() {
    let labels = FileListLabels::english();
    let mut list = FileList::default();
    assert!(!list.is_valid());
    assert_eq!(list.problem(&labels), Some("Add the files to load"));
    list.add(paths(&["a.png"]));
    assert!(list.is_valid());
    assert_eq!(list.problem(&labels), None);

    list.mode = FileListMode::Folder;
    assert!(!list.is_valid(), "files don't count in Folder mode");
    assert_eq!(list.problem(&labels), Some("Choose a folder to load"));
    list.folder = Some(PathBuf::from("/photos"));
    assert!(list.is_valid());
    assert_eq!(list.problem(&labels), None);
}

// Browse queues an open-many picker and the answer's files are added on a later pass; Delete
// removes the selected row; Alt+↓ moves it down.
#[test]
fn browse_adds_and_keys_remove_and_move() {
    let mut rig = Rig::new(FileList::default());
    let browse = rig.at("Browse…");
    rig.click(browse);
    let ticket = rig.response.browsing;
    assert_eq!(ticket, Some(PickId(1)));
    assert_eq!(rig.files.requests(), vec![PickRequest::open_many("stack")]);
    assert_eq!(rig.files.requests()[0].kind, PickKind::OpenMany);

    rig.files.answer(PickId(1), Picked::Paths(paths(&["a.png", "b.png", "c.png"])));
    rig.frame(vec![]);
    assert_eq!(rig.names(), ["a.png", "b.png", "c.png"]);
    assert!(rig.response.changed);

    let first = rig.at("a.png");
    rig.click(first);
    assert_eq!(rig.list.selected, Some(0));
    rig.key(Key::ArrowDown, Modifiers::ALT);
    assert_eq!(rig.names(), ["b.png", "a.png", "c.png"]);
    assert_eq!(rig.list.selected, Some(1), "the selection follows the moved file");
    rig.key(Key::Delete, Modifiers::NONE);
    assert_eq!(rig.names(), ["b.png", "c.png"]);
    assert_eq!(rig.list.selected, None);
}
